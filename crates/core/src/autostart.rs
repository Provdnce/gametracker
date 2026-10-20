//! "Start with the system" for Windows (registry Run key), Linux (XDG
//! autostart file) and macOS (LaunchAgent). The app is started with
//! `--hidden`, so it only tracks in the background until you open it.

use anyhow::Result;
use std::path::Path;

pub const HIDDEN_FLAG: &str = "--hidden";

pub fn is_enabled() -> bool {
    imp::is_enabled()
}

pub fn set_enabled(enabled: bool) -> Result<()> {
    let exe = std::env::current_exe()?;
    imp::set(enabled, &exe)
}

#[cfg(windows)]
mod imp {
    use super::*;
    use anyhow::bail;
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::Foundation::ERROR_FILE_NOT_FOUND;
    use windows_sys::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ, RRF_RT_REG_SZ,
        RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegGetValueW, RegSetValueExW,
    };

    const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const NAME: &str = "GameTracker";

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    pub fn is_enabled() -> bool {
        unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                wide(RUN).as_ptr(),
                wide(NAME).as_ptr(),
                RRF_RT_REG_SZ,
                null_mut(),
                null_mut(),
                null_mut(),
            ) == 0
        }
    }

    pub fn set(enabled: bool, exe: &Path) -> Result<()> {
        unsafe {
            let mut key: HKEY = null_mut();
            let r = RegCreateKeyExW(
                HKEY_CURRENT_USER,
                wide(RUN).as_ptr(),
                0,
                null(),
                REG_OPTION_NON_VOLATILE,
                KEY_SET_VALUE,
                null(),
                &mut key,
                null_mut(),
            );
            if r != 0 {
                bail!("Registry-Schlüssel konnte nicht geöffnet werden (Fehler {r})");
            }
            let name = wide(NAME);
            let r = if enabled {
                let value = wide(&format!("\"{}\" {}", exe.display(), HIDDEN_FLAG));
                RegSetValueExW(
                    key,
                    name.as_ptr(),
                    0,
                    REG_SZ,
                    value.as_ptr() as *const u8,
                    (value.len() * 2) as u32,
                )
            } else {
                match RegDeleteValueW(key, name.as_ptr()) {
                    ERROR_FILE_NOT_FOUND => 0,
                    r => r,
                }
            };
            RegCloseKey(key);
            if r != 0 {
                bail!("Autostart konnte nicht geändert werden (Fehler {r})");
            }
        }
        Ok(())
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use super::*;
    use std::path::PathBuf;

    fn file() -> Option<PathBuf> {
        Some(
            dirs::config_dir()?
                .join("autostart")
                .join("gametracker.desktop"),
        )
    }

    pub fn is_enabled() -> bool {
        file().is_some_and(|f| f.exists())
    }

    pub fn set(enabled: bool, exe: &Path) -> Result<()> {
        let f = file().ok_or_else(|| anyhow::anyhow!("Kein Konfigurationsordner gefunden"))?;
        if enabled {
            std::fs::create_dir_all(f.parent().unwrap())?;
            std::fs::write(
                &f,
                format!(
                    "[Desktop Entry]\nType=Application\nName=GameTracker\nExec=\"{}\" {}\nX-GNOME-Autostart-enabled=true\n",
                    exe.display(),
                    HIDDEN_FLAG
                ),
            )?;
        } else if f.exists() {
            std::fs::remove_file(f)?;
        }
        Ok(())
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use super::*;
    use std::path::PathBuf;

    fn file() -> Option<PathBuf> {
        Some(dirs::home_dir()?.join("Library/LaunchAgents/de.gametracker.app.plist"))
    }

    pub fn is_enabled() -> bool {
        file().is_some_and(|f| f.exists())
    }

    pub fn set(enabled: bool, exe: &Path) -> Result<()> {
        let f = file().ok_or_else(|| anyhow::anyhow!("Kein Home-Ordner gefunden"))?;
        if enabled {
            std::fs::create_dir_all(f.parent().unwrap())?;
            std::fs::write(
                &f,
                format!(
                    r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>Label</key><string>de.gametracker.app</string>
<key>ProgramArguments</key><array><string>{}</string><string>{}</string></array>
<key>RunAtLoad</key><true/>
</dict></plist>
"#,
                    exe.display(),
                    HIDDEN_FLAG
                ),
            )?;
        } else if f.exists() {
            std::fs::remove_file(f)?;
        }
        Ok(())
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
mod imp {
    use super::*;
    pub fn is_enabled() -> bool {
        false
    }
    pub fn set(_enabled: bool, _exe: &Path) -> Result<()> {
        anyhow::bail!("Autostart wird auf diesem System nicht unterstützt")
    }
}
