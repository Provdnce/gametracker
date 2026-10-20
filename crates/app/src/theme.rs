//! Color schemes, icons and small reusable building blocks.
//!
//! Colors come from the active [`Palette`], which is built from the user's
//! choice of background ("graphite", "midnight", "light") and accent color.
//! Read it anywhere with [`c()`].

use gpui::{Div, FontWeight, Rgba, SharedString, Stateful, Svg, div, prelude::*, px, rgb, svg};
use std::cell::Cell;

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub dark: bool,
    pub bg: u32,
    pub sidebar: u32,
    pub card: u32,
    pub card_hover: u32,
    pub border: u32,
    pub text: u32,
    pub muted: u32,
    pub faint: u32,
    pub accent: u32,
    pub accent_hover: u32,
    pub accent_soft: u32,
    pub on_accent: u32,
    pub live: u32,
    pub warn: u32,
    pub danger: u32,
    pub success_bg: u32,
    pub warn_bg: u32,
    pub danger_bg: u32,
}

/// (id, label) of the background options.
pub const BASES: [(&str, &str); 3] = [
    ("graphite", "Graphit"),
    ("midnight", "Mitternacht"),
    ("light", "Hell"),
];

/// (id, label, color on dark, color on light) of the accent options.
pub const ACCENTS: [(&str, &str, u32, u32); 6] = [
    ("indigo", "Indigo", 0x7381f0, 0x4f5bd5),
    ("ocean", "Ozean", 0x4a93d6, 0x2f72b3),
    ("sage", "Salbei", 0x68a689, 0x3f7f62),
    ("sand", "Sand", 0xc9a063, 0x9a7339),
    ("rose", "Rosé", 0xc97a92, 0xa8536d),
    ("mono", "Graphit", 0xc4c7ce, 0x3b3e45),
];

thread_local! {
    static PALETTE: Cell<Palette> = Cell::new(palette("graphite", "indigo"));
}

/// The active palette.
pub fn c() -> Palette {
    PALETTE.with(|p| p.get())
}

pub fn set_palette(base: &str, accent: &str) {
    PALETTE.with(|p| p.set(palette(base, accent)));
}

pub fn palette(base: &str, accent: &str) -> Palette {
    // bg, sidebar, card, card_hover, border, text, muted, faint
    let (dark, b) = match base {
        "midnight" => (
            true,
            [
                0x0e1117, 0x12161e, 0x171c26, 0x1e2431, 0x272f3e, 0xe6e9f0, 0x8a93a6, 0x596277,
            ],
        ),
        "light" => (
            false,
            [
                0xf5f5f3, 0xededea, 0xffffff, 0xf0f0ed, 0xe1e1dc, 0x1b1c1f, 0x686b73, 0x9fa2a9,
            ],
        ),
        _ => (
            true,
            [
                0x111214, 0x151618, 0x1a1b1e, 0x222326, 0x2b2c30, 0xe9eaed, 0x8d9098, 0x5c5f66,
            ],
        ),
    };
    let (_, _, on_dark, on_light) = ACCENTS
        .iter()
        .find(|a| a.0 == accent)
        .copied()
        .unwrap_or(ACCENTS[0]);
    let accent = if dark { on_dark } else { on_light };
    let [bg, sidebar, card, card_hover, border, text, muted, faint] = b;
    let (live, warn, danger) = if dark {
        (0x5fb791, 0xd6a24a, 0xdd6461)
    } else {
        (0x2e8a63, 0xa8731b, 0xbf3f3c)
    };
    Palette {
        dark,
        bg,
        sidebar,
        card,
        card_hover,
        border,
        text,
        muted,
        faint,
        accent,
        accent_hover: blend(accent, if dark { 0xffffff } else { 0x000000 }, 0.12),
        accent_soft: blend(sidebar, accent, if dark { 0.16 } else { 0.12 }),
        on_accent: readable_on(accent),
        live,
        warn,
        danger,
        success_bg: blend(card, live, 0.14),
        warn_bg: blend(card, warn, 0.14),
        danger_bg: blend(card, danger, 0.14),
    }
}

/// Muted colors for games that sit well next to each other.
const GAME_COLORS: [u32; 7] = [
    0x68a689, 0xc9a063, 0x4a93d6, 0xc97a92, 0x8f86c9, 0x6aa8ad, 0xa89d72,
];

/// Color per game id, so a game has the same color everywhere.
/// The first game gets the accent color.
pub fn game_color(game_id: i64) -> Rgba {
    let i = game_id.rem_euclid(GAME_COLORS.len() as i64 + 1) as usize;
    rgb(if i == 0 {
        c().accent
    } else {
        GAME_COLORS[i - 1]
    })
}

/// Linear blend between two colors.
pub fn blend(a: u32, b: u32, t: f32) -> u32 {
    let t = t.clamp(0.0, 1.0);
    let ch = |c: u32, s: u32| ((c >> s) & 0xff) as f32;
    let lerp = |s| (ch(a, s) + (ch(b, s) - ch(a, s)) * t).round() as u32;
    (lerp(16) << 16) | (lerp(8) << 8) | lerp(0)
}

pub fn mix(a: u32, b: u32, t: f32) -> Rgba {
    rgb(blend(a, b, t))
}

fn luminance(c: u32) -> f32 {
    let ch = |s: u32| ((c >> s) & 0xff) as f32 / 255.0;
    0.2126 * ch(16) + 0.7152 * ch(8) + 0.0722 * ch(0)
}

/// Black or white text, whichever reads better on `bg`.
pub fn readable_on(bg: u32) -> u32 {
    if luminance(bg) > 0.55 {
        0x16171a
    } else {
        0xffffff
    }
}

/// Heatmap color for `t` in 0..=1: from the card color to the accent.
pub fn heat_u32(t: f32) -> u32 {
    let p = c();
    if t <= 0.0 {
        return p.card_hover;
    }
    let start = blend(p.card_hover, p.accent, 0.12);
    blend(start, p.accent, t.clamp(0.0, 1.0))
}

pub fn heat(t: f32) -> Rgba {
    rgb(heat_u32(t))
}

// ------------------------------------------------------------------ icons

/// Line icons (Lucide, ISC license) bundled with the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Overview,
    Games,
    History,
    Keyboard,
    Settings,
    Power,
    ChevronLeft,
    ChevronRight,
    Pause,
    Alert,
    EyeOff,
    Check,
}

impl Icon {
    pub fn path(self) -> &'static str {
        match self {
            Icon::Overview => "icons/overview.svg",
            Icon::Games => "icons/games.svg",
            Icon::History => "icons/history.svg",
            Icon::Keyboard => "icons/keyboard.svg",
            Icon::Settings => "icons/settings.svg",
            Icon::Power => "icons/power.svg",
            Icon::ChevronLeft => "icons/chevron-left.svg",
            Icon::ChevronRight => "icons/chevron-right.svg",
            Icon::Pause => "icons/pause.svg",
            Icon::Alert => "icons/alert.svg",
            Icon::EyeOff => "icons/eye-off.svg",
            Icon::Check => "icons/check.svg",
        }
    }
}

pub fn icon(icon: Icon, size: f32, color: u32) -> Svg {
    svg()
        .path(icon.path())
        .size(px(size))
        .flex_none()
        .text_color(rgb(color))
}

/// The "GT" monogram.
pub fn logo(size: f32) -> Div {
    let p = c();
    let (bg, fg) = if p.dark {
        (0x18191d, 0xeceef0)
    } else {
        (0x1b1c1f, 0xf5f5f3)
    };
    div()
        .flex_none()
        .size(px(size))
        .rounded(px(size * 0.24))
        .bg(rgb(bg))
        .border(px((size / 22.).max(1.)))
        .border_color(rgb(p.accent))
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .font_family("Inter Display")
                .font_weight(FontWeight::SEMIBOLD)
                .text_size(px(size * 0.44))
                .line_height(px(size * 0.5))
                .text_color(rgb(fg))
                .child("GT"),
        )
}

/// Small filled or hollow dot (status indicator).
pub fn dot(color: u32, filled: bool) -> Div {
    div()
        .flex_none()
        .size(px(8.))
        .rounded_full()
        .when(filled, |d| d.bg(rgb(color)))
        .when(!filled, |d| d.border_1().border_color(rgb(color)))
}

// ---------------------------------------------------------------- widgets

pub fn card() -> Div {
    let p = c();
    div()
        .bg(rgb(p.card))
        .border_1()
        .border_color(rgb(p.border))
        .rounded_lg()
        .p_4()
}

pub fn title(text: impl Into<SharedString>) -> Div {
    div()
        .text_2xl()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(c().text))
        .child(text.into())
}

pub fn section_title(text: impl Into<SharedString>) -> Div {
    div()
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(c().muted))
        .child(text.into())
}

pub fn muted(text: impl Into<SharedString>) -> Div {
    div()
        .text_sm()
        .text_color(rgb(c().muted))
        .child(text.into())
}

pub fn button(id: impl Into<gpui::ElementId>, label: impl Into<SharedString>) -> Stateful<Div> {
    let p = c();
    div()
        .id(id.into())
        .flex()
        .items_center()
        .gap_1p5()
        .px_3()
        .py_1p5()
        .rounded_md()
        .bg(rgb(p.card_hover))
        .border_1()
        .border_color(rgb(p.border))
        .text_sm()
        .text_color(rgb(p.text))
        .cursor_pointer()
        .hover_style(move |s| s.bg(rgb(p.border)))
        .child(label.into())
}

/// Square button with just an icon.
pub fn icon_button(id: impl Into<gpui::ElementId>, i: Icon) -> Stateful<Div> {
    let p = c();
    div()
        .id(id.into())
        .flex()
        .items_center()
        .justify_center()
        .size(px(32.))
        .rounded_md()
        .bg(rgb(p.card_hover))
        .border_1()
        .border_color(rgb(p.border))
        .cursor_pointer()
        .hover_style(move |s| s.bg(rgb(p.border)))
        .child(icon(i, 16., p.text))
}

pub fn primary_button(
    id: impl Into<gpui::ElementId>,
    label: impl Into<SharedString>,
) -> Stateful<Div> {
    let p = c();
    div()
        .id(id.into())
        .px_3()
        .py_1p5()
        .rounded_md()
        .bg(rgb(p.accent))
        .text_sm()
        .font_weight(FontWeight::MEDIUM)
        .text_color(rgb(p.on_accent))
        .cursor_pointer()
        .hover_style(move |s| s.bg(rgb(p.accent_hover)))
        .child(label.into())
}

/// Segmented-control item.
pub fn chip(
    id: impl Into<gpui::ElementId>,
    label: impl Into<SharedString>,
    selected: bool,
) -> Stateful<Div> {
    let p = c();
    div()
        .id(id.into())
        .px_3()
        .py_1()
        .rounded_md()
        .text_sm()
        .cursor_pointer()
        .when(selected, |d| {
            d.bg(rgb(p.accent)).text_color(rgb(p.on_accent))
        })
        .when(!selected, |d| {
            d.text_color(rgb(p.muted))
                .hover_style(move |s| s.bg(rgb(p.card_hover)).text_color(rgb(p.text)))
        })
        .child(label.into())
}

/// On/off switch with a label.
pub fn toggle(
    id: impl Into<gpui::ElementId>,
    label: impl Into<SharedString>,
    hint: &str,
    on: bool,
) -> Stateful<Div> {
    let p = c();
    div()
        .id(id.into())
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .py_2()
        .cursor_pointer()
        .child(
            div()
                .flex()
                .flex_col()
                .child(div().text_color(rgb(p.text)).child(label.into()))
                .when(!hint.is_empty(), |d| d.child(muted(hint.to_string()))),
        )
        .child(
            div()
                .flex_none()
                .w(px(36.))
                .h(px(20.))
                .rounded_full()
                .p(px(3.))
                .flex()
                .when(on, |d| d.bg(rgb(p.accent)).justify_end())
                .when(!on, |d| d.bg(rgb(p.border)).justify_start())
                .child(div().size(px(14.)).rounded_full().bg(rgb(if on {
                    p.on_accent
                } else {
                    p.muted
                }))),
        )
}

/// Horizontal bar, `fraction` of the full width filled.
pub fn bar(fraction: f32, color: Rgba) -> Div {
    div()
        .h(px(6.))
        .w_full()
        .rounded_full()
        .bg(rgb(c().card_hover))
        .child(
            div()
                .h_full()
                .rounded_full()
                .bg(color)
                .w(gpui::relative(fraction.clamp(0.0, 1.0).max(0.005))),
        )
}

/// Big number with a caption.
pub fn stat(label: impl Into<SharedString>, value: impl Into<SharedString>, color: u32) -> Div {
    card()
        .flex_1()
        .min_w(px(150.))
        .flex()
        .flex_col()
        .gap_1()
        .child(muted(label))
        .child(
            div()
                .text_xl()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(color))
                .child(value.into()),
        )
}

/// Colored notice box (success / warning / error).
pub fn notice(ok: bool, text: impl Into<SharedString>) -> Div {
    let p = c();
    div()
        .p_3()
        .rounded_md()
        .text_sm()
        .border_1()
        .text_color(rgb(p.text))
        .when(ok, |d| d.bg(rgb(p.success_bg)).border_color(rgb(p.live)))
        .when(!ok, |d| d.bg(rgb(p.danger_bg)).border_color(rgb(p.danger)))
        .child(text.into())
}
