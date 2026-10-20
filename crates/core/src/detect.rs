//! Decides whether a running process is a game.
//!
//! Order of checks:
//! 1. User rules (by exe name): either "this is game X" or "ignore this".
//! 2. Built-in list of well-known games that don't live in a store folder.
//! 3. Auto-detection by install path (Steam, Epic, GOG, Xbox, Ubisoft, EA, ...).

use crate::db::Rule;
use std::collections::HashMap;
use std::path::{Component, Path};

#[derive(Debug, Clone)]
pub struct ProcInfo {
    /// Lowercase file name, e.g. `eldenring.exe`
    pub exe: String,
    /// Full path of the executable, if the OS gives it to us
    pub path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detected {
    pub name: String,
    pub exe: String,
}

/// Store folders: the folder *after* this sequence of path components is the game folder.
const STORE_ROOTS: &[&[&str]] = &[
    &["steamapps", "common"],
    &["epic games"],
    &["gog galaxy", "games"],
    &["gog games"],
    &["xboxgames"],
    &["modifiablewindowsapps"],
    &["ubisoft game launcher", "games"],
    &["ea games"],
    &["origin games"],
    &["riot games"],
    &["rockstar games"],
    &["battle.net", "games"],
    &["amazon games", "library"],
    &["itch", "apps"],
    &["heroic"],
    &["lutris", "games"],
];

/// Folders inside a store root that are not games.
const NON_GAME_FOLDERS: &[&str] = &[
    "steamworks shared",
    "steamworks common redistributables",
    "steam linux runtime",
    "steamlinuxruntime",
    "steamlinuxruntime_soldier",
    "steamlinuxruntime_sniper",
    "steam controller configs",
    "launcher",
    "epic online services",
    "directxredist",
    "riot client",
    "launcher",
    "battle.net",
];

/// Executables that live in game folders but are not the game itself.
const NON_GAME_EXES: &[&str] = &[
    "unitycrashhandler64.exe",
    "unitycrashhandler32.exe",
    "crashreportclient.exe",
    "crashpad_handler.exe",
    "crashpad_handler",
    "crashhandler.exe",
    "easyanticheat.exe",
    "easyanticheat_setup.exe",
    "easyanticheat_eos.exe",
    "easyanticheat_eos_setup.exe",
    "beservice.exe",
    "beservice_x64.exe",
    "battleye launcher.exe",
    "dxsetup.exe",
    "vc_redist.x64.exe",
    "vc_redist.x86.exe",
    "vcredist_x64.exe",
    "vcredist_x86.exe",
    "dotnetfx.exe",
    "oalinst.exe",
    "uninstall.exe",
    "unins000.exe",
    "cefprocess.exe",
    "cefsharp.browsersubprocess.exe",
    "qtwebengineprocess.exe",
    "steamwebhelper.exe",
    "steamerrorreporter.exe",
    "steamerrorreporter64.exe",
    "epicwebhelper.exe",
    "riotclientservices.exe",
    "riotclientux.exe",
    "riotclientuxrender.exe",
    "riotclientcrashhandler.exe",
    "vgtray.exe",
    "vgc.exe",
    "reaper",
    "pressure-vessel-wrap",
    "pv-bwrap",
    "wineserver",
    "wine64-preloader",
    "wine-preloader",
    "explorer.exe",
    "conhost.exe",
    "services.exe",
    "winedevice.exe",
    "plugplay.exe",
    "svchost.exe",
    "rpcss.exe",
    "tabtip.exe",
];

/// Well-known games outside store folders: (exe, name).
const KNOWN_GAMES: &[(&str, &str)] = &[
    ("league of legends.exe", "League of Legends"),
    ("valorant-win64-shipping.exe", "VALORANT"),
    ("wow.exe", "World of Warcraft"),
    ("wowclassic.exe", "World of Warcraft Classic"),
    ("overwatch.exe", "Overwatch 2"),
    ("diablo iv.exe", "Diablo IV"),
    ("hearthstone.exe", "Hearthstone"),
    ("starcraft ii.exe", "StarCraft II"),
    ("sc2_x64.exe", "StarCraft II"),
    ("cod.exe", "Call of Duty"),
    ("fortniteclient-win64-shipping.exe", "Fortnite"),
    ("minecraft.windows.exe", "Minecraft (Bedrock)"),
    ("osu!.exe", "osu!"),
    ("genshinimpact.exe", "Genshin Impact"),
    ("starrail.exe", "Honkai: Star Rail"),
    ("zenlesszonezero.exe", "Zenless Zone Zero"),
    ("r5apex.exe", "Apex Legends"),
    ("r5apex_dx12.exe", "Apex Legends"),
    ("gta5.exe", "Grand Theft Auto V"),
    ("gta5_enhanced.exe", "Grand Theft Auto V"),
    ("rdr2.exe", "Red Dead Redemption 2"),
    ("rocketleague.exe", "Rocket League"),
    ("pathofexile_x64.exe", "Path of Exile"),
    ("pathofexilesteam.exe", "Path of Exile"),
    ("pathofexile2.exe", "Path of Exile 2"),
    ("eldenring.exe", "ELDEN RING"),
    ("cs2.exe", "Counter-Strike 2"),
    ("dota2.exe", "Dota 2"),
    ("rustclient.exe", "Rust"),
    ("eft.exe", "Escape from Tarkov"),
    ("escapefromtarkov.exe", "Escape from Tarkov"),
    ("ffxiv_dx11.exe", "Final Fantasy XIV"),
    ("lostark.exe", "Lost Ark"),
    ("tslgame.exe", "PUBG: Battlegrounds"),
    ("robloxplayerbeta.exe", "Roblox"),
];

pub struct Detector {
    rules: HashMap<String, Rule>,
    pub auto_detect: bool,
}

impl Detector {
    pub fn new(rules: Vec<Rule>, auto_detect: bool) -> Self {
        Self {
            rules: rules
                .into_iter()
                .map(|r| (r.exe.to_lowercase(), r))
                .collect(),
            auto_detect,
        }
    }

    pub fn detect(&self, p: &ProcInfo) -> Option<Detected> {
        let exe = p.exe.to_lowercase();
        if let Some(rule) = self.rules.get(&exe) {
            return (!rule.ignored).then(|| Detected {
                name: rule.name.clone(),
                exe,
            });
        }
        if !self.auto_detect {
            return None;
        }
        if let Some((_, name)) = KNOWN_GAMES.iter().find(|(e, _)| *e == exe) {
            return Some(Detected {
                name: name.to_string(),
                exe,
            });
        }
        if NON_GAME_EXES.contains(&exe.as_str()) || looks_like_helper(&exe) {
            return None;
        }
        let path = p.path.as_deref()?;
        let name = game_folder_from_path(path)?;
        Some(Detected { name, exe })
    }
}

fn looks_like_helper(exe: &str) -> bool {
    let stem = exe.trim_end_matches(".exe");
    [
        "crash",
        "report",
        "redist",
        "setup",
        "install",
        "uninst",
        "update",
        "patcher",
        "anticheat",
        "webhelper",
        "helper",
        "service",
        "overlay",
    ]
    .iter()
    .any(|w| stem.contains(w))
}

/// Finds the game folder name in a path like `D:\SteamLibrary\steamapps\common\Hades II\Ship\Hades2.exe`.
pub fn game_folder_from_path(path: &str) -> Option<String> {
    // Windows paths may show up on Linux (Proton cmdlines), so split on both.
    let parts: Vec<String> = path
        .split(['\\', '/'])
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    let lower: Vec<String> = parts.iter().map(|s| s.to_lowercase()).collect();

    for root in STORE_ROOTS {
        if root.len() >= lower.len() {
            continue;
        }
        for i in 0..=lower.len() - root.len() {
            if lower[i..i + root.len()]
                .iter()
                .zip(root.iter())
                .all(|(a, b)| a == b)
            {
                let idx = i + root.len();
                // Need a folder AND something inside it (the exe), so idx+1 < len.
                if idx + 1 >= parts.len() {
                    continue;
                }
                let folder = &parts[idx];
                let folder_l = &lower[idx];
                if NON_GAME_FOLDERS.contains(&folder_l.as_str())
                    || folder_l.starts_with("proton")
                    || folder_l.starts_with("steamlinuxruntime")
                    || lower[idx + 1..]
                        .iter()
                        .any(|c| c == "_commonredist" || c == "redist" || c == "__installer")
                {
                    return None;
                }
                return Some(prettify(folder));
            }
        }
    }
    None
}

fn prettify(folder: &str) -> String {
    folder.replace('_', " ").trim().to_string()
}

/// Lowercase file name of a path.
pub fn exe_name(path: &Path) -> Option<String> {
    path.components().next_back().and_then(|c| match c {
        Component::Normal(s) => Some(s.to_string_lossy().to_lowercase()),
        _ => None,
    })
}

/// Picks a nice display name for an exe the user added by hand.
pub fn name_for_exe_path(path: &Path) -> String {
    if let Some(name) = game_folder_from_path(&path.to_string_lossy()) {
        return name;
    }
    path.file_stem()
        .map(|s| prettify(&s.to_string_lossy()))
        .unwrap_or_else(|| "Unbekanntes Spiel".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(exe: &str, path: &str) -> ProcInfo {
        ProcInfo {
            exe: exe.into(),
            path: Some(path.into()),
        }
    }

    #[test]
    fn steam_folder() {
        let d = Detector::new(vec![], true);
        let r = d.detect(&p(
            "hades2.exe",
            r"D:\SteamLibrary\steamapps\common\Hades II\Ship\Hades2.exe",
        ));
        assert_eq!(r.unwrap().name, "Hades II");
        let r = d.detect(&p(
            "unitycrashhandler64.exe",
            r"C:\Steam\steamapps\common\Foo\UnityCrashHandler64.exe",
        ));
        assert!(r.is_none());
        let r = d.detect(&p(
            "proton",
            "/home/u/.steam/steam/steamapps/common/Proton 9.0/proton",
        ));
        assert!(r.is_none());
        let r = d.detect(&p("steam.exe", r"C:\Program Files (x86)\Steam\steam.exe"));
        assert!(r.is_none());
    }

    #[test]
    fn epic_and_rules() {
        let d = Detector::new(
            vec![
                Rule {
                    exe: "Witcher3.exe".into(),
                    name: "The Witcher 3".into(),
                    ignored: false,
                },
                Rule {
                    exe: "tool.exe".into(),
                    name: String::new(),
                    ignored: true,
                },
            ],
            true,
        );
        assert_eq!(
            d.detect(&p("witcher3.exe", r"C:\Games\w3\bin\x64\witcher3.exe"))
                .unwrap()
                .name,
            "The Witcher 3"
        );
        assert!(
            d.detect(&p("tool.exe", r"C:\Program Files\Epic Games\Tool\tool.exe"))
                .is_none()
        );
        assert_eq!(
            d.detect(&p(
                "game.exe",
                r"C:\Program Files\Epic Games\Hogwarts_Legacy\game.exe"
            ))
            .unwrap()
            .name,
            "Hogwarts Legacy"
        );
        let off = Detector::new(vec![], false);
        assert!(
            off.detect(&p("cs2.exe", r"C:\Steam\steamapps\common\cs2\cs2.exe"))
                .is_none()
        );
    }

    #[test]
    fn known_games() {
        let d = Detector::new(vec![], true);
        assert_eq!(
            d.detect(&ProcInfo {
                exe: "league of legends.exe".into(),
                path: None
            })
            .unwrap()
            .name,
            "League of Legends"
        );
    }
}
