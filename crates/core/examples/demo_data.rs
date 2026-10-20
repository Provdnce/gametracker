//! Fills a database with made-up data so the UI can be tried out.
//! Usage: GAMETRACKER_DATA_DIR=/tmp/demo cargo run -p gametracker-core --example demo_data
use chrono::{Duration, Local, Utc};
use gametracker_core::db::{Db, Session};

fn main() -> anyhow::Result<()> {
    let dir = gametracker_core::data_dir();
    let mut db = Db::open(&dir.join("gametracker.db"))?;
    let machine = db.machine_id()?;
    let games = [
        ("ELDEN RING", "eldenring.exe", 0.30),
        ("Hades II", "hades2.exe", 0.25),
        ("Counter-Strike 2", "cs2.exe", 0.35),
        ("Stardew Valley", "stardew valley.exe", 0.15),
        ("Baldur's Gate 3", "bg3.exe", 0.2),
        ("Rocket League", "rocketleague.exe", 0.1),
    ];
    let mut seed = 42u64;
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed % 10_000) as f64 / 10_000.0
    };
    let now = Utc::now().timestamp();
    for day in (1..240).rev() {
        let base = now - day * 86_400;
        for (gi, (name, exe, p)) in games.iter().enumerate() {
            // Games go in and out of fashion over the months
            let phase = ((day as f64 / 40.0) + gi as f64).sin().abs();
            if rnd() < p * phase * 1.6 {
                let gid = db.game_id(name, Some(exe))?;
                let start = base + 17 * 3600 + (rnd() * 4.0 * 3600.0) as i64;
                let len = (1200.0 + rnd() * 3.0 * 3600.0) as i64;
                db.insert_session(&Session {
                    id: uuid_like(&mut rnd),
                    game_id: gid,
                    game_name: String::new(),
                    start,
                    end: start + len,
                    machine: machine.clone(),
                })?;
            }
        }
    }
    let keys = [
        ("KeyW", 900.0),
        ("KeyA", 500.0),
        ("KeyS", 420.0),
        ("KeyD", 520.0),
        ("Space", 650.0),
        ("ShiftLeft", 380.0),
        ("KeyE", 300.0),
        ("KeyR", 220.0),
        ("KeyQ", 150.0),
        ("KeyF", 140.0),
        ("Num1", 120.0),
        ("Num2", 110.0),
        ("Num3", 80.0),
        ("Tab", 90.0),
        ("Escape", 70.0),
        ("ControlLeft", 160.0),
        ("KeyN", 120.0),
        ("KeyI", 140.0),
        ("KeyT", 130.0),
        ("Return", 110.0),
        ("Backspace", 160.0),
        ("KeyM", 60.0),
        ("KeyC", 90.0),
        ("KeyG", 50.0),
        ("KeyH", 70.0),
        ("KeyO", 100.0),
        ("KeyU", 80.0),
        ("KeyL", 70.0),
        ("KeyB", 40.0),
        ("KeyY", 30.0),
        ("KeyZ", 30.0),
        ("Alt", 40.0),
        ("UpArrow", 20.0),
        ("DownArrow", 20.0),
        ("Dot", 40.0),
        ("Comma", 50.0),
        ("F5", 10.0),
        ("KeyP", 40.0),
        ("KeyK", 30.0),
        ("KeyJ", 20.0),
        ("KeyX", 30.0),
        ("KeyV", 60.0),
        ("SemiColon", 8.0),
        ("Quote", 12.0),
    ];
    let today = Local::now().date_naive();
    for day in 0..240 {
        let d = today - Duration::days(day);
        let activity = 0.3 + rnd() * 1.4;
        let rows: Vec<(chrono::NaiveDate, &str, u64)> = keys
            .iter()
            .map(|(k, n)| (d, *k, (n * activity * (0.6 + rnd() * 0.8)) as u64))
            .collect();
        db.add_key_counts(&machine, rows.iter().map(|(d, k, n)| (d, *k, *n)))?;
    }
    println!("Demo-Daten geschrieben nach {}", dir.display());
    Ok(())
}

fn uuid_like(rnd: &mut impl FnMut() -> f64) -> String {
    format!(
        "demo-{:016x}",
        (rnd() * 1e15) as u64 ^ ((rnd() * 1e15) as u64) << 8
    )
}
