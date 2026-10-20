//! Key ids and keyboard layouts.
//!
//! Keys are stored by *physical position* using US-style names (`KeyY`,
//! `SemiColon`, ...), the same names `rdev` uses. That way the data means the
//! same thing on every OS and every keyboard layout. The layout setting only
//! changes the labels the UI shows (on QWERTZ, `KeyY` is labelled "Z").

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Layout {
    #[default]
    Qwertz,
    Qwerty,
}

impl Layout {
    pub fn label(self) -> &'static str {
        match self {
            Layout::Qwertz => "QWERTZ (Deutsch)",
            Layout::Qwerty => "QWERTY (US)",
        }
    }
}

/// Human readable label of a key id for the given layout.
pub fn label(id: &str, layout: Layout) -> String {
    let de = layout == Layout::Qwertz;
    let s = match id {
        "KeyY" if de => "Z",
        "KeyZ" if de => "Y",
        "BackQuote" => {
            if de {
                "^"
            } else {
                "`"
            }
        }
        "Minus" => {
            if de {
                "ß"
            } else {
                "-"
            }
        }
        "Equal" => {
            if de {
                "´"
            } else {
                "="
            }
        }
        "LeftBracket" => {
            if de {
                "Ü"
            } else {
                "["
            }
        }
        "RightBracket" => {
            if de {
                "+"
            } else {
                "]"
            }
        }
        "SemiColon" => {
            if de {
                "Ö"
            } else {
                ";"
            }
        }
        "Quote" => {
            if de {
                "Ä"
            } else {
                "'"
            }
        }
        "BackSlash" => {
            if de {
                "#"
            } else {
                "\\"
            }
        }
        "IntlBackslash" => {
            if de {
                "<"
            } else {
                "\\"
            }
        }
        "Comma" => ",",
        "Dot" => ".",
        "Slash" => {
            if de {
                "-"
            } else {
                "/"
            }
        }
        "Escape" => "Esc",
        "Backspace" => {
            if de {
                "Rück"
            } else {
                "Back"
            }
        }
        "Tab" => "Tab",
        "CapsLock" => {
            if de {
                "Feststell"
            } else {
                "Caps"
            }
        }
        "Return" => "Enter",
        "ShiftLeft" => "Shift",
        "ShiftRight" => "Shift R",
        "ControlLeft" => {
            if de {
                "Strg"
            } else {
                "Ctrl"
            }
        }
        "ControlRight" => {
            if de {
                "Strg R"
            } else {
                "Ctrl R"
            }
        }
        "MetaLeft" => "Win",
        "MetaRight" => "Win R",
        "Alt" => "Alt",
        "AltGr" => "AltGr",
        "Menu" => "Menü",
        "Space" => {
            if de {
                "Leertaste"
            } else {
                "Space"
            }
        }
        "PrintScreen" => {
            if de {
                "Druck"
            } else {
                "PrtSc"
            }
        }
        "ScrollLock" => {
            if de {
                "Rollen"
            } else {
                "ScrLk"
            }
        }
        "Pause" => "Pause",
        "Insert" => {
            if de {
                "Einfg"
            } else {
                "Ins"
            }
        }
        "Delete" => {
            if de {
                "Entf"
            } else {
                "Del"
            }
        }
        "Home" => {
            if de {
                "Pos1"
            } else {
                "Home"
            }
        }
        "End" => {
            if de {
                "Ende"
            } else {
                "End"
            }
        }
        "PageUp" => {
            if de {
                "Bild↑"
            } else {
                "PgUp"
            }
        }
        "PageDown" => {
            if de {
                "Bild↓"
            } else {
                "PgDn"
            }
        }
        "UpArrow" => "↑",
        "DownArrow" => "↓",
        "LeftArrow" => "←",
        "RightArrow" => "→",
        "NumLock" => "Num",
        "KpDivide" => "/",
        "KpMultiply" => "*",
        "KpMinus" => "-",
        "KpPlus" => "+",
        "KpReturn" => "Enter",
        "KpDelete" => {
            if de {
                ","
            } else {
                "."
            }
        }
        "Function" => "Fn",
        "VolumeMute" => "Stumm",
        "VolumeDown" => "Leiser",
        "VolumeUp" => "Lauter",
        "MediaNext" => "Next",
        "MediaPrev" => "Prev",
        "MediaStop" => "Stop",
        "MediaPlayPause" => "Play/Pause",
        _ => "",
    };
    if !s.is_empty() {
        return s.to_string();
    }
    if let Some(rest) = id.strip_prefix("Key") {
        return rest.to_string();
    }
    if let Some(rest) = id.strip_prefix("Num") {
        return rest.to_string();
    }
    if let Some(rest) = id.strip_prefix("Kp") {
        return rest.to_string();
    }
    id.to_string()
}

/// Longer label for lists (adds context like "Numpad").
pub fn long_label(id: &str, layout: Layout) -> String {
    let base = label(id, layout);
    match id {
        "ShiftRight" => "Shift rechts".into(),
        "ControlRight" => {
            if layout == Layout::Qwertz {
                "Strg rechts".into()
            } else {
                "Ctrl right".into()
            }
        }
        "MetaRight" => "Win rechts".into(),
        _ if id.starts_with("Kp") => format!("Numpad {base}"),
        _ => base,
    }
}

/// One slot on the drawn keyboard.
#[derive(Debug, Clone, Copy)]
pub struct Slot {
    /// `None` = empty gap
    pub id: Option<&'static str>,
    /// Width in key units (1.0 = normal key)
    pub width: f32,
    /// False for the second half of tall keys (ISO Enter, Numpad +/Enter)
    pub show_label: bool,
}

const fn k(id: &'static str) -> Slot {
    Slot {
        id: Some(id),
        width: 1.0,
        show_label: true,
    }
}
const fn w(id: &'static str, width: f32) -> Slot {
    Slot {
        id: Some(id),
        width,
        show_label: true,
    }
}
const fn cont(id: &'static str, width: f32) -> Slot {
    Slot {
        id: Some(id),
        width,
        show_label: false,
    }
}
const fn gap(width: f32) -> Slot {
    Slot {
        id: None,
        width,
        show_label: false,
    }
}

/// The keyboard as blocks (main, navigation, numpad), each a list of 6 rows.
pub fn keyboard(layout: Layout) -> Vec<Vec<Vec<Slot>>> {
    let fn_row = vec![
        k("Escape"),
        gap(1.0),
        k("F1"),
        k("F2"),
        k("F3"),
        k("F4"),
        gap(0.5),
        k("F5"),
        k("F6"),
        k("F7"),
        k("F8"),
        gap(0.5),
        k("F9"),
        k("F10"),
        k("F11"),
        k("F12"),
    ];
    let num_row = vec![
        k("BackQuote"),
        k("Num1"),
        k("Num2"),
        k("Num3"),
        k("Num4"),
        k("Num5"),
        k("Num6"),
        k("Num7"),
        k("Num8"),
        k("Num9"),
        k("Num0"),
        k("Minus"),
        k("Equal"),
        w("Backspace", 2.0),
    ];
    let qwer = [
        "KeyQ", "KeyW", "KeyE", "KeyR", "KeyT", "KeyY", "KeyU", "KeyI", "KeyO", "KeyP",
    ];
    let asdf = [
        "KeyA", "KeyS", "KeyD", "KeyF", "KeyG", "KeyH", "KeyJ", "KeyK", "KeyL",
    ];
    let zxcv = ["KeyZ", "KeyX", "KeyC", "KeyV", "KeyB", "KeyN", "KeyM"];
    let bottom = vec![
        w("ControlLeft", 1.25),
        w("MetaLeft", 1.25),
        w("Alt", 1.25),
        w("Space", 6.25),
        w("AltGr", 1.25),
        w("MetaRight", 1.25),
        w("Menu", 1.25),
        w("ControlRight", 1.25),
    ];

    let main = match layout {
        // ISO
        Layout::Qwertz => {
            let mut r2 = vec![w("Tab", 1.5)];
            r2.extend(qwer.iter().map(|id| k(id)));
            r2.extend([k("LeftBracket"), k("RightBracket"), w("Return", 1.5)]);
            let mut r3 = vec![w("CapsLock", 1.75)];
            r3.extend(asdf.iter().map(|id| k(id)));
            r3.extend([
                k("SemiColon"),
                k("Quote"),
                k("BackSlash"),
                cont("Return", 1.25),
            ]);
            let mut r4 = vec![w("ShiftLeft", 1.25), k("IntlBackslash")];
            r4.extend(zxcv.iter().map(|id| k(id)));
            r4.extend([k("Comma"), k("Dot"), k("Slash"), w("ShiftRight", 2.75)]);
            vec![fn_row, num_row, r2, r3, r4, bottom]
        }
        // ANSI
        Layout::Qwerty => {
            let mut r2 = vec![w("Tab", 1.5)];
            r2.extend(qwer.iter().map(|id| k(id)));
            r2.extend([k("LeftBracket"), k("RightBracket"), w("BackSlash", 1.5)]);
            let mut r3 = vec![w("CapsLock", 1.75)];
            r3.extend(asdf.iter().map(|id| k(id)));
            r3.extend([k("SemiColon"), k("Quote"), w("Return", 2.25)]);
            let mut r4 = vec![w("ShiftLeft", 2.25)];
            r4.extend(zxcv.iter().map(|id| k(id)));
            r4.extend([k("Comma"), k("Dot"), k("Slash"), w("ShiftRight", 2.75)]);
            vec![fn_row, num_row, r2, r3, r4, bottom]
        }
    };

    let nav = vec![
        vec![k("PrintScreen"), k("ScrollLock"), k("Pause")],
        vec![k("Insert"), k("Home"), k("PageUp")],
        vec![k("Delete"), k("End"), k("PageDown")],
        vec![gap(3.0)],
        vec![gap(1.0), k("UpArrow"), gap(1.0)],
        vec![k("LeftArrow"), k("DownArrow"), k("RightArrow")],
    ];

    let numpad = vec![
        vec![gap(4.0)],
        vec![k("NumLock"), k("KpDivide"), k("KpMultiply"), k("KpMinus")],
        vec![k("Kp7"), k("Kp8"), k("Kp9"), k("KpPlus")],
        vec![k("Kp4"), k("Kp5"), k("Kp6"), cont("KpPlus", 1.0)],
        vec![k("Kp1"), k("Kp2"), k("Kp3"), k("KpReturn")],
        vec![w("Kp0", 2.0), k("KpDelete"), cont("KpReturn", 1.0)],
    ];

    vec![main, nav, numpad]
}

/// Windows: maps a hardware scan code to our key id.
/// `vk` is used for keys whose scan code is ambiguous or missing.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn from_windows_scancode(scan: u32, extended: bool, vk: u32) -> Option<&'static str> {
    // Keys better identified by virtual key code
    match vk {
        0x13 => return Some("Pause"),
        0x90 => return Some("NumLock"),
        0x2C => return Some("PrintScreen"),
        0xAD => return Some("VolumeMute"),
        0xAE => return Some("VolumeDown"),
        0xAF => return Some("VolumeUp"),
        0xB0 => return Some("MediaNext"),
        0xB1 => return Some("MediaPrev"),
        0xB2 => return Some("MediaStop"),
        0xB3 => return Some("MediaPlayPause"),
        _ => {}
    }
    let id = match (scan, extended) {
        (0x01, _) => "Escape",
        (0x02, _) => "Num1",
        (0x03, _) => "Num2",
        (0x04, _) => "Num3",
        (0x05, _) => "Num4",
        (0x06, _) => "Num5",
        (0x07, _) => "Num6",
        (0x08, _) => "Num7",
        (0x09, _) => "Num8",
        (0x0A, _) => "Num9",
        (0x0B, _) => "Num0",
        (0x0C, _) => "Minus",
        (0x0D, _) => "Equal",
        (0x0E, _) => "Backspace",
        (0x0F, _) => "Tab",
        (0x10, _) => "KeyQ",
        (0x11, _) => "KeyW",
        (0x12, _) => "KeyE",
        (0x13, _) => "KeyR",
        (0x14, _) => "KeyT",
        (0x15, _) => "KeyY",
        (0x16, _) => "KeyU",
        (0x17, _) => "KeyI",
        (0x18, _) => "KeyO",
        (0x19, _) => "KeyP",
        (0x1A, _) => "LeftBracket",
        (0x1B, _) => "RightBracket",
        (0x1C, false) => "Return",
        (0x1C, true) => "KpReturn",
        (0x1D, false) => "ControlLeft",
        (0x1D, true) => "ControlRight",
        (0x1E, _) => "KeyA",
        (0x1F, _) => "KeyS",
        (0x20, _) => "KeyD",
        (0x21, _) => "KeyF",
        (0x22, _) => "KeyG",
        (0x23, _) => "KeyH",
        (0x24, _) => "KeyJ",
        (0x25, _) => "KeyK",
        (0x26, _) => "KeyL",
        (0x27, _) => "SemiColon",
        (0x28, _) => "Quote",
        (0x29, _) => "BackQuote",
        (0x2A, _) => "ShiftLeft",
        (0x2B, _) => "BackSlash",
        (0x2C, _) => "KeyZ",
        (0x2D, _) => "KeyX",
        (0x2E, _) => "KeyC",
        (0x2F, _) => "KeyV",
        (0x30, _) => "KeyB",
        (0x31, _) => "KeyN",
        (0x32, _) => "KeyM",
        (0x33, _) => "Comma",
        (0x34, _) => "Dot",
        (0x35, false) => "Slash",
        (0x35, true) => "KpDivide",
        (0x36, _) => "ShiftRight",
        (0x37, false) => "KpMultiply",
        (0x38, false) => "Alt",
        (0x38, true) => "AltGr",
        (0x39, _) => "Space",
        (0x3A, _) => "CapsLock",
        (0x3B, _) => "F1",
        (0x3C, _) => "F2",
        (0x3D, _) => "F3",
        (0x3E, _) => "F4",
        (0x3F, _) => "F5",
        (0x40, _) => "F6",
        (0x41, _) => "F7",
        (0x42, _) => "F8",
        (0x43, _) => "F9",
        (0x44, _) => "F10",
        (0x46, _) => "ScrollLock",
        (0x47, false) => "Kp7",
        (0x47, true) => "Home",
        (0x48, false) => "Kp8",
        (0x48, true) => "UpArrow",
        (0x49, false) => "Kp9",
        (0x49, true) => "PageUp",
        (0x4A, _) => "KpMinus",
        (0x4B, false) => "Kp4",
        (0x4B, true) => "LeftArrow",
        (0x4C, _) => "Kp5",
        (0x4D, false) => "Kp6",
        (0x4D, true) => "RightArrow",
        (0x4E, _) => "KpPlus",
        (0x4F, false) => "Kp1",
        (0x4F, true) => "End",
        (0x50, false) => "Kp2",
        (0x50, true) => "DownArrow",
        (0x51, false) => "Kp3",
        (0x51, true) => "PageDown",
        (0x52, false) => "Kp0",
        (0x52, true) => "Insert",
        (0x53, false) => "KpDelete",
        (0x53, true) => "Delete",
        (0x56, _) => "IntlBackslash",
        (0x57, _) => "F11",
        (0x58, _) => "F12",
        (0x5B, true) => "MetaLeft",
        (0x5C, true) => "MetaRight",
        (0x5D, true) => "Menu",
        (0x64, _) => "F13",
        (0x65, _) => "F14",
        (0x66, _) => "F15",
        (0x67, _) => "F16",
        (0x68, _) => "F17",
        (0x69, _) => "F18",
        (0x6A, _) => "F19",
        (0x6B, _) => "F20",
        (0x6C, _) => "F21",
        (0x6D, _) => "F22",
        (0x6E, _) => "F23",
        (0x76, _) => "F24",
        _ => return None,
    };
    Some(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels() {
        assert_eq!(label("KeyY", Layout::Qwertz), "Z");
        assert_eq!(label("KeyY", Layout::Qwerty), "Y");
        assert_eq!(label("SemiColon", Layout::Qwertz), "Ö");
        assert_eq!(label("Num7", Layout::Qwertz), "7");
        assert_eq!(label("Kp7", Layout::Qwertz), "7");
        assert_eq!(long_label("Kp7", Layout::Qwertz), "Numpad 7");
        assert_eq!(label("F5", Layout::Qwertz), "F5");
    }

    #[test]
    fn rows_have_equal_width() {
        for layout in [Layout::Qwertz, Layout::Qwerty] {
            for block in keyboard(layout) {
                let widths: Vec<f32> = block
                    .iter()
                    .map(|r| r.iter().map(|s| s.width).sum())
                    .collect();
                assert!(
                    widths.iter().all(|w| (w - widths[0]).abs() < 0.01),
                    "{layout:?}: {widths:?}"
                );
            }
        }
    }
}
