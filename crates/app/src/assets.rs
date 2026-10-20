//! Icons and fonts compiled into the executable, so the app is a single file.

use gpui::{AssetSource, Result, SharedString};
use std::borrow::Cow;

macro_rules! files {
    ($($path:literal),* $(,)?) => {
        &[$(($path, include_bytes!(concat!("../assets/", $path)) as &[u8])),*]
    };
}

const ICONS: &[(&str, &[u8])] = files![
    "icons/overview.svg",
    "icons/games.svg",
    "icons/history.svg",
    "icons/keyboard.svg",
    "icons/settings.svg",
    "icons/power.svg",
    "icons/chevron-left.svg",
    "icons/chevron-right.svg",
    "icons/pause.svg",
    "icons/alert.svg",
    "icons/eye-off.svg",
    "icons/check.svg",
];

pub const FONTS: &[(&str, &[u8])] = files![
    "fonts/Inter-Regular.ttf",
    "fonts/Inter-Medium.ttf",
    "fonts/Inter-SemiBold.ttf",
    "fonts/Inter-Bold.ttf",
    "fonts/InterDisplay-SemiBold.ttf",
];

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(ICONS
            .iter()
            .find(|(p, _)| *p == path)
            .map(|(_, bytes)| Cow::Borrowed(*bytes)))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(ICONS
            .iter()
            .filter(|(p, _)| p.starts_with(path))
            .map(|(p, _)| SharedString::from(*p))
            .collect())
    }
}

pub fn fonts() -> Vec<Cow<'static, [u8]>> {
    FONTS.iter().map(|(_, b)| Cow::Borrowed(*b)).collect()
}
