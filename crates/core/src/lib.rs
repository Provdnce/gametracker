//! GameTracker backend: tracks which games you play and how often you press
//! each key, stores everything in a local SQLite database and can export /
//! import it as JSON (e.g. when moving to a new PC).

pub mod autostart;
pub mod db;
pub mod detect;
pub mod fmt;
pub mod keymap;
pub mod keys;
pub mod stats;
pub mod tracker;
pub mod transfer;

pub use db::Db;
pub use tracker::{Settings, Shared, Tracker};

use std::path::PathBuf;

/// Where the database and backups live:
/// Windows `%APPDATA%\GameTracker`, Linux `~/.local/share/GameTracker`,
/// macOS `~/Library/Application Support/GameTracker`.
/// Can be overridden with the `GAMETRACKER_DATA_DIR` environment variable.
pub fn data_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("GAMETRACKER_DATA_DIR") {
        return PathBuf::from(dir);
    }
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("GameTracker")
}
