//! SQLite storage. Every thread opens its own `Db`; WAL mode lets the
//! tracker threads write while the UI reads.

use anyhow::{Context, Result};
use chrono::{NaiveDate, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use std::path::{Path, PathBuf};

const SCHEMA_VERSION: i64 = 1;

pub struct Db {
    conn: Connection,
    path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Game {
    pub id: i64,
    pub name: String,
    pub exe: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub id: String,
    pub game_id: i64,
    pub game_name: String,
    /// Unix seconds (UTC)
    pub start: i64,
    /// Unix seconds (UTC)
    pub end: i64,
    pub machine: String,
}

impl Session {
    pub fn duration(&self) -> i64 {
        (self.end - self.start).max(0)
    }
}

/// A user-defined rule for one executable name (lowercase, e.g. `eldenring.exe`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Rule {
    pub exe: String,
    /// Display name. Ignored when `ignored` is true.
    pub name: String,
    pub ignored: bool,
}

impl Db {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).ok();
        }
        let conn = Connection::open(path).with_context(|| {
            format!("Datenbank {} konnte nicht geöffnet werden", path.display())
        })?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let db = Db {
            conn,
            path: path.to_path_buf(),
        };
        db.migrate()?;
        Ok(db)
    }

    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Db {
            conn,
            path: PathBuf::from(":memory:"),
        };
        db.migrate()?;
        Ok(db)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn migrate(&self) -> Result<()> {
        let version: i64 = self
            .conn
            .pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version < 1 {
            self.conn.execute_batch(
                "
                CREATE TABLE IF NOT EXISTS meta(
                    key   TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS machines(
                    id   TEXT PRIMARY KEY,
                    name TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS games(
                    id         INTEGER PRIMARY KEY,
                    name       TEXT NOT NULL UNIQUE COLLATE NOCASE,
                    exe        TEXT,
                    created_at INTEGER NOT NULL
                );
                CREATE TABLE IF NOT EXISTS sessions(
                    id       TEXT PRIMARY KEY,
                    game_id  INTEGER NOT NULL REFERENCES games(id) ON DELETE CASCADE,
                    start_ts INTEGER NOT NULL,
                    end_ts   INTEGER NOT NULL,
                    machine  TEXT NOT NULL
                );
                CREATE INDEX IF NOT EXISTS sessions_start ON sessions(start_ts);
                CREATE INDEX IF NOT EXISTS sessions_game ON sessions(game_id);
                CREATE TABLE IF NOT EXISTS key_counts(
                    machine TEXT NOT NULL,
                    day     TEXT NOT NULL,
                    key     TEXT NOT NULL,
                    count   INTEGER NOT NULL,
                    PRIMARY KEY(machine, day, key)
                );
                CREATE INDEX IF NOT EXISTS key_counts_day ON key_counts(day);
                CREATE TABLE IF NOT EXISTS rules(
                    exe     TEXT PRIMARY KEY,
                    name    TEXT NOT NULL,
                    ignored INTEGER NOT NULL DEFAULT 0
                );
                ",
            )?;
        }
        self.conn
            .pragma_update(None, "user_version", SCHEMA_VERSION)?;
        Ok(())
    }

    // ---------------------------------------------------------------- meta

    pub fn meta(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| r.get(0))
            .optional()?)
    }

    pub fn set_meta(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO meta(key, value) VALUES(?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    /// Stable id of this installation. Created on first use.
    pub fn machine_id(&self) -> Result<String> {
        if let Some(id) = self.meta("machine_id")? {
            return Ok(id);
        }
        let id = uuid::Uuid::new_v4().to_string();
        self.set_meta("machine_id", &id)?;
        let host = sysinfo::System::host_name().unwrap_or_else(|| "PC".into());
        self.upsert_machine(&id, &host)?;
        Ok(id)
    }

    pub fn upsert_machine(&self, id: &str, name: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO machines(id, name) VALUES(?1, ?2)
             ON CONFLICT(id) DO UPDATE SET name = excluded.name",
            params![id, name],
        )?;
        Ok(())
    }

    pub fn machines(&self) -> Result<Vec<(String, String)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name FROM machines ORDER BY name")?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    // ---------------------------------------------------------------- games

    /// Returns the id of the game with this name, creating it if needed.
    pub fn game_id(&self, name: &str, exe: Option<&str>) -> Result<i64> {
        if let Some(id) = self
            .conn
            .query_row("SELECT id FROM games WHERE name = ?1", [name], |r| r.get(0))
            .optional()?
        {
            return Ok(id);
        }
        self.conn.execute(
            "INSERT INTO games(name, exe, created_at) VALUES(?1, ?2, ?3)",
            params![name, exe, Utc::now().timestamp()],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn games(&self) -> Result<Vec<Game>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, exe FROM games ORDER BY name")?;
        let rows = stmt.query_map([], |r| {
            Ok(Game {
                id: r.get(0)?,
                name: r.get(1)?,
                exe: r.get(2)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Renames a game. If another game already has the new name, the two
    /// are merged (sessions move over to the existing game).
    pub fn rename_game(&mut self, id: i64, new_name: &str) -> Result<()> {
        let new_name = new_name.trim();
        anyhow::ensure!(!new_name.is_empty(), "Name darf nicht leer sein");
        let tx = self.conn.transaction()?;
        let existing: Option<i64> = tx
            .query_row(
                "SELECT id FROM games WHERE name = ?1 AND id != ?2",
                params![new_name, id],
                |r| r.get(0),
            )
            .optional()?;
        let old_exe: Option<String> = tx
            .query_row("SELECT exe FROM games WHERE id = ?1", [id], |r| r.get(0))
            .optional()?
            .flatten();
        match existing {
            Some(target) => {
                tx.execute(
                    "UPDATE sessions SET game_id = ?1 WHERE game_id = ?2",
                    params![target, id],
                )?;
                tx.execute("DELETE FROM games WHERE id = ?1", [id])?;
            }
            None => {
                tx.execute(
                    "UPDATE games SET name = ?1 WHERE id = ?2",
                    params![new_name, id],
                )?;
            }
        }
        // Keep detection in sync: the exe should map to the new name from now on.
        if let Some(exe) = old_exe {
            tx.execute(
                "INSERT INTO rules(exe, name, ignored) VALUES(?1, ?2, 0)
                 ON CONFLICT(exe) DO UPDATE SET name = excluded.name, ignored = 0",
                params![exe, new_name],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn delete_game(&self, id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM sessions WHERE game_id = ?1", [id])?;
        self.conn.execute("DELETE FROM games WHERE id = ?1", [id])?;
        Ok(())
    }

    // ------------------------------------------------------------- sessions

    pub fn insert_session(&self, s: &Session) -> Result<()> {
        self.conn.execute(
            "INSERT INTO sessions(id, game_id, start_ts, end_ts, machine) VALUES(?1, ?2, ?3, ?4, ?5)",
            params![s.id, s.game_id, s.start, s.end, s.machine],
        )?;
        Ok(())
    }

    pub fn set_session_end(&self, id: &str, end: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE sessions SET end_ts = ?2 WHERE id = ?1",
            params![id, end],
        )?;
        Ok(())
    }

    pub fn delete_session(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM sessions WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Most recent session of a game on a machine, if any.
    pub fn last_session(&self, game_id: i64, machine: &str) -> Result<Option<Session>> {
        Ok(self
            .conn
            .query_row(
                "SELECT s.id, s.game_id, g.name, s.start_ts, s.end_ts, s.machine
                 FROM sessions s JOIN games g ON g.id = s.game_id
                 WHERE s.game_id = ?1 AND s.machine = ?2
                 ORDER BY s.end_ts DESC LIMIT 1",
                params![game_id, machine],
                row_to_session,
            )
            .optional()?)
    }

    /// Sessions overlapping `[from, to)`. `None` = unbounded.
    pub fn sessions_between(&self, from: Option<i64>, to: Option<i64>) -> Result<Vec<Session>> {
        let mut stmt = self.conn.prepare(
            "SELECT s.id, s.game_id, g.name, s.start_ts, s.end_ts, s.machine
             FROM sessions s JOIN games g ON g.id = s.game_id
             WHERE s.end_ts >= ?1 AND s.start_ts < ?2
             ORDER BY s.start_ts DESC",
        )?;
        let rows = stmt.query_map(
            params![from.unwrap_or(i64::MIN), to.unwrap_or(i64::MAX)],
            row_to_session,
        )?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn all_sessions(&self) -> Result<Vec<Session>> {
        self.sessions_between(None, None)
    }

    /// Removes sessions shorter than `min_secs` (e.g. left over after a crash).
    pub fn prune_short_sessions(&self, min_secs: i64) -> Result<usize> {
        Ok(self.conn.execute(
            "DELETE FROM sessions WHERE end_ts - start_ts < ?1",
            [min_secs],
        )?)
    }

    // ----------------------------------------------------------- key counts

    /// Adds counts for `(day, key)` on this machine.
    pub fn add_key_counts<'a>(
        &mut self,
        machine: &str,
        counts: impl IntoIterator<Item = (&'a NaiveDate, &'a str, u64)>,
    ) -> Result<()> {
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO key_counts(machine, day, key, count) VALUES(?1, ?2, ?3, ?4)
                 ON CONFLICT(machine, day, key) DO UPDATE SET count = count + excluded.count",
            )?;
            for (day, key, n) in counts {
                stmt.execute(params![
                    machine,
                    day.format("%Y-%m-%d").to_string(),
                    key,
                    n as i64
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Sums per key for days in `[from, to]` (inclusive). `None` = all time.
    pub fn key_totals(&self, range: Option<(NaiveDate, NaiveDate)>) -> Result<Vec<(String, u64)>> {
        let (from, to) = match range {
            Some((a, b)) => (
                a.format("%Y-%m-%d").to_string(),
                b.format("%Y-%m-%d").to_string(),
            ),
            None => ("0000-00-00".to_string(), "9999-99-99".to_string()),
        };
        let mut stmt = self.conn.prepare(
            "SELECT key, SUM(count) AS c FROM key_counts
             WHERE day >= ?1 AND day <= ?2
             GROUP BY key ORDER BY c DESC",
        )?;
        let rows = stmt.query_map(params![from, to], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? as u64))
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Total key presses per day for days in `[from, to]`.
    pub fn key_presses_per_day(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<(NaiveDate, u64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT day, SUM(count) FROM key_counts WHERE day >= ?1 AND day <= ?2
             GROUP BY day ORDER BY day",
        )?;
        let rows = stmt.query_map(
            params![
                from.format("%Y-%m-%d").to_string(),
                to.format("%Y-%m-%d").to_string()
            ],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? as u64)),
        )?;
        let mut out = Vec::new();
        for row in rows {
            let (day, n) = row?;
            if let Ok(d) = NaiveDate::parse_from_str(&day, "%Y-%m-%d") {
                out.push((d, n));
            }
        }
        Ok(out)
    }

    pub fn all_key_counts(&self) -> Result<Vec<(String, String, String, u64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT machine, day, key, count FROM key_counts ORDER BY day, machine, key",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get::<_, i64>(3)? as u64))
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn first_key_day(&self) -> Result<Option<NaiveDate>> {
        let day: Option<String> =
            self.conn
                .query_row("SELECT MIN(day) FROM key_counts", [], |r| r.get(0))?;
        Ok(day.and_then(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok()))
    }

    // ---------------------------------------------------------------- rules

    pub fn rules(&self) -> Result<Vec<Rule>> {
        let mut stmt = self
            .conn
            .prepare("SELECT exe, name, ignored FROM rules ORDER BY ignored, name")?;
        let rows = stmt.query_map([], |r| {
            Ok(Rule {
                exe: r.get(0)?,
                name: r.get(1)?,
                ignored: r.get::<_, i64>(2)? != 0,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn set_rule(&self, rule: &Rule) -> Result<()> {
        self.conn.execute(
            "INSERT INTO rules(exe, name, ignored) VALUES(?1, ?2, ?3)
             ON CONFLICT(exe) DO UPDATE SET name = excluded.name, ignored = excluded.ignored",
            params![rule.exe.to_lowercase(), rule.name, rule.ignored as i64],
        )?;
        Ok(())
    }

    pub fn remove_rule(&self, exe: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM rules WHERE exe = ?1", [exe.to_lowercase()])?;
        Ok(())
    }

    // ------------------------------------------------------------ raw access

    pub(crate) fn conn(&mut self) -> &mut Connection {
        &mut self.conn
    }
}

fn row_to_session(r: &rusqlite::Row) -> rusqlite::Result<Session> {
    Ok(Session {
        id: r.get(0)?,
        game_id: r.get(1)?,
        game_name: r.get(2)?,
        start: r.get(3)?,
        end: r.get(4)?,
        machine: r.get(5)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_counts_accumulate() {
        let mut db = Db::open_in_memory().unwrap();
        let d = NaiveDate::from_ymd_opt(2026, 10, 20).unwrap();
        db.add_key_counts("m", [(&d, "KeyW", 3u64), (&d, "Space", 1)])
            .unwrap();
        db.add_key_counts("m", [(&d, "KeyW", 2u64)]).unwrap();
        let totals = db.key_totals(None).unwrap();
        assert_eq!(totals[0], ("KeyW".to_string(), 5));
        assert_eq!(totals[1], ("Space".to_string(), 1));
        let other = NaiveDate::from_ymd_opt(2026, 10, 21).unwrap();
        assert!(db.key_totals(Some((other, other))).unwrap().is_empty());
    }

    #[test]
    fn rename_merges_games() {
        let mut db = Db::open_in_memory().unwrap();
        let a = db.game_id("Elden Ring", Some("eldenring.exe")).unwrap();
        let b = db.game_id("ELDEN RING NIGHTREIGN", None).unwrap();
        for (i, g) in [a, b].iter().enumerate() {
            db.insert_session(&Session {
                id: format!("s{i}"),
                game_id: *g,
                game_name: String::new(),
                start: 0,
                end: 100,
                machine: "m".into(),
            })
            .unwrap();
        }
        db.rename_game(a, "elden ring nightreign").unwrap();
        assert_eq!(db.games().unwrap().len(), 1);
        assert_eq!(db.all_sessions().unwrap().len(), 2);
        assert_eq!(db.rules().unwrap()[0].name, "elden ring nightreign");
    }
}
