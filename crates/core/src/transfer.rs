//! Export / import as a single JSON file, plus automatic backups.
//!
//! Import is a *merge* and safe to repeat:
//! - sessions are matched by their id (a UUID), so they are never duplicated
//! - key counts are stored per (PC, day, key); on import the larger count wins
//! - games are matched by name
//! - your local rules are kept; rules from the file are only added

#[cfg(test)]
use crate::db::Session;
use crate::db::{Db, Rule};
use anyhow::{Context, Result, bail};
use chrono::{DateTime, Local, NaiveDate, Utc};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const FORMAT: &str = "gametracker-export";
const VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportFile {
    pub format: String,
    pub version: u32,
    pub exported_at: DateTime<Utc>,
    pub machines: Vec<ExportMachine>,
    pub games: Vec<ExportGame>,
    pub sessions: Vec<ExportSession>,
    pub key_counts: Vec<ExportKeyCount>,
    #[serde(default)]
    pub rules: Vec<Rule>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportMachine {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportGame {
    pub name: String,
    pub exe: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportSession {
    pub id: String,
    pub game: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub machine: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportKeyCount {
    pub machine: String,
    pub day: NaiveDate,
    pub key: String,
    pub count: u64,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ImportSummary {
    pub new_games: usize,
    pub new_sessions: usize,
    pub updated_sessions: usize,
    pub key_rows: usize,
}

impl std::fmt::Display for ImportSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} neue Spiele, {} neue Sessions, {} aktualisierte Sessions, {} Tastatur-Einträge",
            self.new_games, self.new_sessions, self.updated_sessions, self.key_rows
        )
    }
}

pub fn build_export(db: &Db) -> Result<ExportFile> {
    let ts = |t: i64| DateTime::<Utc>::from_timestamp(t, 0).unwrap_or_default();
    Ok(ExportFile {
        format: FORMAT.into(),
        version: VERSION,
        exported_at: Utc::now(),
        machines: db
            .machines()?
            .into_iter()
            .map(|(id, name)| ExportMachine { id, name })
            .collect(),
        games: db
            .games()?
            .into_iter()
            .map(|g| ExportGame {
                name: g.name,
                exe: g.exe,
            })
            .collect(),
        sessions: db
            .all_sessions()?
            .into_iter()
            .rev()
            .map(|s| ExportSession {
                id: s.id,
                game: s.game_name,
                start: ts(s.start),
                end: ts(s.end),
                machine: s.machine,
            })
            .collect(),
        key_counts: db
            .all_key_counts()?
            .into_iter()
            .filter_map(|(machine, day, key, count)| {
                Some(ExportKeyCount {
                    machine,
                    day: NaiveDate::parse_from_str(&day, "%Y-%m-%d").ok()?,
                    key,
                    count,
                })
            })
            .collect(),
        rules: db.rules()?,
    })
}

pub fn export_to(db: &Db, path: &Path) -> Result<()> {
    let data = build_export(db)?;
    let json = serde_json::to_string_pretty(&data)?;
    // Write to a temp file first so a crash never leaves a half-written export.
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json)
        .with_context(|| format!("Konnte {} nicht schreiben", tmp.display()))?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

pub fn import_from(db: &mut Db, path: &Path) -> Result<ImportSummary> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("Konnte {} nicht lesen", path.display()))?;
    let data: ExportFile =
        serde_json::from_str(&text).context("Datei ist kein gültiger GameTracker-Export")?;
    import(db, data)
}

pub fn import(db: &mut Db, data: ExportFile) -> Result<ImportSummary> {
    if data.format != FORMAT {
        bail!("Unbekanntes Dateiformat: {}", data.format);
    }
    if data.version > VERSION {
        bail!(
            "Export stammt aus einer neueren Version (v{}). Bitte GameTracker aktualisieren.",
            data.version
        );
    }
    let mut sum = ImportSummary::default();
    let games_before = db.games()?.len();
    let tx = db.conn().transaction()?;
    {
        for m in &data.machines {
            tx.execute(
                "INSERT INTO machines(id, name) VALUES(?1, ?2) ON CONFLICT(id) DO NOTHING",
                params![m.id, m.name],
            )?;
        }
        let game_id = |name: &str, exe: Option<&str>| -> Result<i64> {
            tx.execute(
                "INSERT INTO games(name, exe, created_at) VALUES(?1, ?2, ?3) ON CONFLICT(name) DO NOTHING",
                params![name, exe, Utc::now().timestamp()],
            )?;
            Ok(tx.query_row("SELECT id FROM games WHERE name = ?1", [name], |r| r.get(0))?)
        };
        for g in &data.games {
            game_id(&g.name, g.exe.as_deref())?;
        }
        for s in &data.sessions {
            let gid = game_id(&s.game, None)?;
            let inserted = tx.execute(
                "INSERT INTO sessions(id, game_id, start_ts, end_ts, machine) VALUES(?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(id) DO NOTHING",
                params![s.id, gid, s.start.timestamp(), s.end.timestamp(), s.machine],
            )?;
            if inserted > 0 {
                sum.new_sessions += 1;
            } else {
                sum.updated_sessions += tx.execute(
                    "UPDATE sessions SET end_ts = ?2 WHERE id = ?1 AND end_ts < ?2",
                    params![s.id, s.end.timestamp()],
                )?;
            }
        }
        for k in &data.key_counts {
            sum.key_rows += tx.execute(
                "INSERT INTO key_counts(machine, day, key, count) VALUES(?1, ?2, ?3, ?4)
                 ON CONFLICT(machine, day, key) DO UPDATE SET count = MAX(count, excluded.count)
                 WHERE excluded.count > count",
                params![
                    k.machine,
                    k.day.format("%Y-%m-%d").to_string(),
                    k.key,
                    k.count as i64
                ],
            )?;
        }
        for r in &data.rules {
            tx.execute(
                "INSERT INTO rules(exe, name, ignored) VALUES(?1, ?2, ?3) ON CONFLICT(exe) DO NOTHING",
                params![r.exe.to_lowercase(), r.name, r.ignored as i64],
            )?;
        }
    }
    tx.commit()?;
    sum.new_games = db.games()?.len() - games_before;
    Ok(sum)
}

pub fn default_export_name() -> String {
    format!(
        "gametracker-export-{}.json",
        Local::now().format("%Y-%m-%d")
    )
}

/// Writes a backup into `dir` if the last one is older than `every_days`.
/// Keeps the newest `keep` backups. Returns the path if a backup was written.
pub fn auto_backup(db: &Db, dir: &Path, every_days: u32, keep: usize) -> Result<Option<PathBuf>> {
    if every_days == 0 {
        return Ok(None);
    }
    let now = Utc::now().timestamp();
    let last: i64 = db
        .meta("last_backup")?
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    if now - last < every_days as i64 * 86_400 {
        return Ok(None);
    }
    std::fs::create_dir_all(dir)?;
    let path = dir.join(format!(
        "backup-{}.json",
        Local::now().format("%Y-%m-%d_%H-%M")
    ));
    export_to(db, &path)?;
    db.set_meta("last_backup", &now.to_string())?;

    let mut backups: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("backup-") && n.ends_with(".json"))
        })
        .collect();
    backups.sort();
    while backups.len() > keep {
        let old = backups.remove(0);
        std::fs::remove_file(old).ok();
    }
    Ok(Some(path))
}

#[cfg(test)]
fn session(id: &str, game_id: i64, start: i64, end: i64, machine: &str) -> Session {
    Session {
        id: id.into(),
        game_id,
        game_name: String::new(),
        start,
        end,
        machine: machine.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_repeat_import() {
        let dir = tempfile::tempdir().unwrap();
        let mut a = Db::open(&dir.path().join("a.db")).unwrap();
        let ma = a.machine_id().unwrap();
        let g = a.game_id("Hades II", Some("hades2.exe")).unwrap();
        a.insert_session(&session("s1", g, 1000, 5000, &ma))
            .unwrap();
        let day = NaiveDate::from_ymd_opt(2026, 10, 20).unwrap();
        a.add_key_counts(&ma, [(&day, "KeyW", 10u64)]).unwrap();
        a.set_rule(&Rule {
            exe: "hades2.exe".into(),
            name: "Hades II".into(),
            ignored: false,
        })
        .unwrap();

        let file = dir.path().join("export.json");
        export_to(&a, &file).unwrap();

        // New PC
        let mut b = Db::open(&dir.path().join("b.db")).unwrap();
        let mb = b.machine_id().unwrap();
        b.add_key_counts(&mb, [(&day, "KeyW", 1u64)]).unwrap();
        let s1 = import_from(&mut b, &file).unwrap();
        assert_eq!(s1.new_sessions, 1);
        assert_eq!(s1.new_games, 1);
        assert_eq!(b.key_totals(None).unwrap()[0].1, 11);

        // Importing the same file again changes nothing
        let s2 = import_from(&mut b, &file).unwrap();
        assert_eq!(s2, ImportSummary::default());
        assert_eq!(b.key_totals(None).unwrap()[0].1, 11);
        assert_eq!(b.all_sessions().unwrap().len(), 1);
        assert_eq!(b.rules().unwrap().len(), 1);

        // Newer export from the old PC: counts grew, session got longer
        a.add_key_counts(&ma, [(&day, "KeyW", 5u64)]).unwrap();
        a.set_session_end("s1", 6000).unwrap();
        export_to(&a, &file).unwrap();
        let s3 = import_from(&mut b, &file).unwrap();
        assert_eq!(s3.updated_sessions, 1);
        assert_eq!(s3.key_rows, 1);
        assert_eq!(b.key_totals(None).unwrap()[0].1, 16);
    }

    #[test]
    fn backups_rotate() {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::open(&dir.path().join("a.db")).unwrap();
        let bdir = dir.path().join("backups");
        assert!(auto_backup(&db, &bdir, 7, 3).unwrap().is_some());
        // Second call within the interval does nothing
        assert!(auto_backup(&db, &bdir, 7, 3).unwrap().is_none());
    }
}
