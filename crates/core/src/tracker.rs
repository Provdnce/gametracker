//! Background tracking: a game thread that polls running processes, and a key
//! listener whose counts are flushed to the database every few seconds.

use crate::db::{Db, Session};
use crate::detect::{Detected, Detector, ProcInfo};
use crate::keymap::Layout;
use anyhow::Result;
use chrono::{Local, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub track_games: bool,
    pub track_keys: bool,
    /// Detect games by install folder (Steam, Epic, ...) and a built-in list.
    pub auto_detect: bool,
    pub layout: Layout,
    /// How often running processes are checked.
    pub poll_secs: u64,
    /// Sessions shorter than this are dropped (e.g. a game that crashed on start).
    pub min_session_secs: i64,
    /// If a game is restarted within this time, the old session continues.
    pub merge_gap_secs: i64,
    /// 0 = no automatic backups
    pub backup_every_days: u32,
    pub backups_to_keep: usize,
    /// Closing the window keeps tracking in the background.
    pub run_in_background: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            track_games: true,
            track_keys: true,
            auto_detect: true,
            layout: Layout::Qwertz,
            poll_secs: 10,
            min_session_secs: 60,
            merge_gap_secs: 180,
            backup_every_days: 7,
            backups_to_keep: 12,
            run_in_background: true,
        }
    }
}

impl Settings {
    pub fn load(db: &Db) -> Settings {
        db.meta("settings")
            .ok()
            .flatten()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, db: &Db) -> Result<()> {
        db.set_meta("settings", &serde_json::to_string(self)?)
    }
}

/// A game that is running right now.
#[derive(Debug, Clone)]
pub struct ActiveGame {
    pub name: String,
    pub exe: String,
    pub game_id: i64,
    pub session_id: String,
    pub start: i64,
}

/// A running process that could be marked as a game in the settings.
#[derive(Debug, Clone)]
pub struct RunningProcess {
    pub exe: String,
    pub path: Option<String>,
    pub detected_as: Option<String>,
}

/// State shared between the tracker threads and the UI.
pub struct Shared {
    pub db_path: PathBuf,
    pub backup_dir: PathBuf,
    pub machine: String,
    pub settings: RwLock<Settings>,
    pub active: Mutex<Vec<ActiveGame>>,
    pub running: Mutex<Vec<RunningProcess>>,
    pending_keys: Mutex<HashMap<(NaiveDate, String), u64>>,
    pub key_error: Mutex<Option<String>>,
    pub last_error: Mutex<Option<String>>,
    rules_changed: AtomicBool,
    wake_games: AtomicBool,
}

impl Shared {
    pub fn settings(&self) -> Settings {
        self.settings.read().unwrap().clone()
    }

    pub fn update_settings(&self, f: impl FnOnce(&mut Settings)) -> Result<()> {
        let mut s = self.settings.write().unwrap();
        f(&mut s);
        let db = Db::open(&self.db_path)?;
        s.save(&db)?;
        self.rules_changed.store(true, Ordering::SeqCst);
        self.wake_games.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Call after changing rules so the game thread reloads them.
    pub fn rules_changed(&self) {
        self.rules_changed.store(true, Ordering::SeqCst);
        self.wake_games.store(true, Ordering::SeqCst);
    }

    pub fn active_games(&self) -> Vec<ActiveGame> {
        self.active.lock().unwrap().clone()
    }

    pub fn running_processes(&self) -> Vec<RunningProcess> {
        self.running.lock().unwrap().clone()
    }

    fn record_key(&self, key: &str) {
        if !self.settings.read().map(|s| s.track_keys).unwrap_or(false) {
            return;
        }
        let day = Local::now().date_naive();
        *self
            .pending_keys
            .lock()
            .unwrap()
            .entry((day, key.to_string()))
            .or_default() += 1;
    }

    /// Key presses not yet written to the database (for live display).
    pub fn pending_key_total(&self) -> u64 {
        self.pending_keys.lock().unwrap().values().sum()
    }

    /// Writes buffered key counts to the database.
    pub fn flush_keys(&self, db: &mut Db) -> Result<()> {
        let pending = std::mem::take(&mut *self.pending_keys.lock().unwrap());
        if pending.is_empty() {
            return Ok(());
        }
        let result = db.add_key_counts(
            &self.machine,
            pending.iter().map(|((d, k), n)| (d, k.as_str(), *n)),
        );
        if result.is_err() {
            // Put them back so nothing is lost.
            let mut p = self.pending_keys.lock().unwrap();
            for (k, n) in pending {
                *p.entry(k).or_default() += n;
            }
        }
        result
    }

    fn set_error(&self, e: impl ToString) {
        let msg = e.to_string();
        log::error!("{msg}");
        *self.last_error.lock().unwrap() = Some(msg);
    }
}

pub struct Tracker;

impl Tracker {
    /// Opens the database, starts all background threads and returns the shared state.
    pub fn start(data_dir: &Path) -> Result<Arc<Shared>> {
        let db_path = data_dir.join("gametracker.db");
        let db = Db::open(&db_path)?;
        let machine = db.machine_id()?;
        let settings = Settings::load(&db);
        // Left-overs from a crash that are too short to count
        db.prune_short_sessions(settings.min_session_secs)?;

        let shared = Arc::new(Shared {
            db_path,
            backup_dir: data_dir.join("backups"),
            machine,
            settings: RwLock::new(settings),
            active: Mutex::default(),
            running: Mutex::default(),
            pending_keys: Mutex::default(),
            key_error: Mutex::default(),
            last_error: Mutex::default(),
            rules_changed: AtomicBool::new(false),
            wake_games: AtomicBool::new(false),
        });

        let s = shared.clone();
        std::thread::Builder::new()
            .name("game-tracker".into())
            .spawn(move || game_loop(s))?;

        let s = shared.clone();
        std::thread::Builder::new()
            .name("key-flusher".into())
            .spawn(move || flush_loop(s))?;

        let on_press = shared.clone();
        let on_error = shared.clone();
        if let Err(e) = crate::keys::spawn_listener(
            move |key| on_press.record_key(key),
            move |err| {
                log::warn!("{err}");
                *on_error.key_error.lock().unwrap() = Some(err);
            },
        ) {
            *shared.key_error.lock().unwrap() = Some(e.to_string());
        }

        Ok(shared)
    }

    /// Flush everything before the app exits.
    pub fn shutdown(shared: &Shared) {
        if let Ok(mut db) = Db::open(&shared.db_path) {
            if let Err(e) = shared.flush_keys(&mut db) {
                log::error!("Konnte Tastendrücke nicht speichern: {e}");
            }
            let now = Utc::now().timestamp();
            for g in shared.active.lock().unwrap().iter() {
                db.set_session_end(&g.session_id, now).ok();
            }
        }
    }
}

fn flush_loop(shared: Arc<Shared>) {
    let mut db = loop {
        match Db::open(&shared.db_path) {
            Ok(db) => break db,
            Err(e) => {
                shared.set_error(e);
                std::thread::sleep(Duration::from_secs(10));
            }
        }
    };
    let mut last_backup_check = Instant::now() - Duration::from_secs(3600);
    loop {
        std::thread::sleep(Duration::from_secs(5));
        if let Err(e) = shared.flush_keys(&mut db) {
            shared.set_error(format!("Tastendrücke speichern fehlgeschlagen: {e}"));
        }
        if last_backup_check.elapsed() > Duration::from_secs(3600) {
            last_backup_check = Instant::now();
            let s = shared.settings();
            match crate::transfer::auto_backup(
                &db,
                &shared.backup_dir,
                s.backup_every_days,
                s.backups_to_keep,
            ) {
                Ok(Some(p)) => log::info!("Backup geschrieben: {}", p.display()),
                Ok(None) => {}
                Err(e) => shared.set_error(format!("Automatisches Backup fehlgeschlagen: {e}")),
            }
        }
    }
}

struct Running {
    session_id: String,
    game_id: i64,
    exe: String,
    start: i64,
    last_seen: i64,
}

fn game_loop(shared: Arc<Shared>) {
    let db = loop {
        match Db::open(&shared.db_path) {
            Ok(db) => break db,
            Err(e) => {
                shared.set_error(e);
                std::thread::sleep(Duration::from_secs(10));
            }
        }
    };
    let mut sys = System::new();
    let mut detector = Detector::new(
        db.rules().unwrap_or_default(),
        shared.settings().auto_detect,
    );
    let mut active: HashMap<String, Running> = HashMap::new();
    let mut last_tick = Utc::now().timestamp();

    loop {
        let settings = shared.settings();
        if shared.rules_changed.swap(false, Ordering::SeqCst) {
            detector = Detector::new(db.rules().unwrap_or_default(), settings.auto_detect);
        }

        let now = Utc::now().timestamp();
        // If the PC was asleep, end sessions at the moment we last saw them.
        let slept = now - last_tick > (settings.poll_secs as i64) * 3 + 30;
        last_tick = now;

        let procs = scan_processes(&mut sys);
        let mut found: HashMap<String, Detected> = HashMap::new();
        let mut running_list = Vec::new();
        for p in &procs {
            let det = detector.detect(p);
            if let Some(d) = &det {
                found.entry(d.name.clone()).or_insert_with(|| d.clone());
            }
            running_list.push(RunningProcess {
                exe: p.exe.clone(),
                path: p.path.clone(),
                detected_as: det.map(|d| d.name),
            });
        }
        *shared.running.lock().unwrap() = running_list;
        if !settings.track_games {
            found.clear();
        }

        // End sessions of games that stopped (or all, after sleep)
        let ended: Vec<String> = active
            .keys()
            .filter(|name| slept || !found.contains_key(*name))
            .cloned()
            .collect();
        for name in ended {
            let r = active.remove(&name).unwrap();
            if r.last_seen - r.start < settings.min_session_secs {
                if let Err(e) = db.delete_session(&r.session_id) {
                    shared.set_error(e);
                }
            } else if let Err(e) = db.set_session_end(&r.session_id, r.last_seen) {
                shared.set_error(e);
            }
        }

        // Start or continue sessions
        for (name, d) in &found {
            if let Some(r) = active.get_mut(name) {
                r.last_seen = now;
                if let Err(e) = db.set_session_end(&r.session_id, now) {
                    shared.set_error(e);
                }
                continue;
            }
            match start_session(&db, &shared.machine, d, now, settings.merge_gap_secs) {
                Ok(r) => {
                    log::info!("Spiel gestartet: {name}");
                    active.insert(name.clone(), r);
                }
                Err(e) => shared.set_error(e),
            }
        }

        let mut list: Vec<ActiveGame> = active
            .iter()
            .map(|(name, r)| ActiveGame {
                name: name.clone(),
                exe: r.exe.clone(),
                game_id: r.game_id,
                session_id: r.session_id.clone(),
                start: r.start,
            })
            .collect();
        list.sort_by_key(|g| g.start);
        *shared.active.lock().unwrap() = list;

        // Sleep, but wake early if settings/rules changed
        let until = Instant::now() + Duration::from_secs(settings.poll_secs.max(2));
        while Instant::now() < until {
            if shared.wake_games.swap(false, Ordering::SeqCst) {
                break;
            }
            std::thread::sleep(Duration::from_millis(250));
        }
    }
}

fn start_session(
    db: &Db,
    machine: &str,
    d: &Detected,
    now: i64,
    merge_gap: i64,
) -> Result<Running> {
    let game_id = db.game_id(&d.name, Some(&d.exe))?;
    if let Some(last) = db.last_session(game_id, machine)?
        && now - last.end <= merge_gap
    {
        db.set_session_end(&last.id, now)?;
        return Ok(Running {
            session_id: last.id,
            game_id,
            exe: d.exe.clone(),
            start: last.start,
            last_seen: now,
        });
    }
    let session = Session {
        id: uuid::Uuid::new_v4().to_string(),
        game_id,
        game_name: d.name.clone(),
        start: now,
        end: now,
        machine: machine.to_string(),
    };
    db.insert_session(&session)?;
    Ok(Running {
        session_id: session.id,
        game_id,
        exe: d.exe.clone(),
        start: now,
        last_seen: now,
    })
}

fn scan_processes(sys: &mut System) -> Vec<ProcInfo> {
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing()
            .with_exe(UpdateKind::OnlyIfNotSet)
            .with_cmd(UpdateKind::OnlyIfNotSet),
    );
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for p in sys.processes().values() {
        let mut path = p.exe().map(|e| e.to_string_lossy().into_owned());
        let mut exe = p
            .exe()
            .and_then(crate::detect::exe_name)
            .unwrap_or_else(|| p.name().to_string_lossy().to_lowercase());
        // Wine/Proton: the real game exe shows up in the command line.
        if !cfg!(windows)
            && let Some(arg) = p
                .cmd()
                .iter()
                .map(|a| a.to_string_lossy())
                .find(|a| a.to_lowercase().ends_with(".exe"))
        {
            let name = arg
                .rsplit(['\\', '/'])
                .next()
                .unwrap_or(&arg)
                .to_lowercase();
            exe = name;
            path = Some(arg.into_owned());
        }
        if is_system_path(path.as_deref()) {
            continue;
        }
        if seen.insert((exe.clone(), path.clone())) {
            out.push(ProcInfo { exe, path });
        }
    }
    out.sort_by(|a, b| a.exe.cmp(&b.exe));
    out
}

fn is_system_path(path: Option<&str>) -> bool {
    let Some(p) = path else { return false };
    let l = p.to_lowercase();
    l.starts_with(r"c:\windows\")
        || l.starts_with("/usr/lib/")
        || l.starts_with("/usr/libexec/")
        || l.starts_with("/usr/sbin/")
        || l.starts_with("/system/")
}
