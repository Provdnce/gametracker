//! Colors and small reusable building blocks.

use gpui::{Div, FontWeight, Rgba, SharedString, Stateful, div, prelude::*, px, rgb};

pub const BG: u32 = 0x0e1016;
pub const SIDEBAR: u32 = 0x141720;
pub const CARD: u32 = 0x1a1e29;
pub const CARD_HOVER: u32 = 0x222736;
pub const BORDER: u32 = 0x2a3041;
pub const TEXT: u32 = 0xe8eaf1;
pub const MUTED: u32 = 0x8a92a6;
pub const FAINT: u32 = 0x5b6275;
pub const ACCENT: u32 = 0x8b6cff;
pub const ACCENT_SOFT: u32 = 0x2c2552;
pub const LIVE: u32 = 0x2dd4a7;
pub const WARN: u32 = 0xf5a524;
pub const DANGER: u32 = 0xf25f5c;

/// Palette for game bars, so different games are easy to tell apart.
const GAME_COLORS: [u32; 8] = [
    0x8b6cff, 0x2dd4a7, 0xf5a524, 0x4fa3ff, 0xf25f9a, 0x9ad94f, 0xff8a4c, 0x5ad1e6,
];

/// Color per game id, so a game has the same color everywhere.
pub fn game_color(game_id: i64) -> Rgba {
    rgb(GAME_COLORS[game_id.rem_euclid(GAME_COLORS.len() as i64) as usize])
}

/// Linear blend between two colors.
pub fn mix(a: u32, b: u32, t: f32) -> Rgba {
    let t = t.clamp(0.0, 1.0);
    let ch = |c: u32, s: u32| ((c >> s) & 0xff) as f32;
    let lerp = |s| (ch(a, s) + (ch(b, s) - ch(a, s)) * t).round() as u32;
    rgb((lerp(16) << 16) | (lerp(8) << 8) | lerp(0))
}

/// Heatmap color for `t` in 0..=1 (cold → hot).
pub fn heat(t: f32) -> Rgba {
    const STOPS: [u32; 5] = [0x232838, 0x2f4fa8, 0x8b6cff, 0xf25f9a, 0xffc24b];
    if t <= 0.0 {
        return rgb(STOPS[0]);
    }
    let t = t.clamp(0.0, 1.0) * (STOPS.len() - 1) as f32;
    let i = (t.floor() as usize).min(STOPS.len() - 2);
    mix(STOPS[i], STOPS[i + 1], t - i as f32)
}

pub fn card() -> Div {
    div()
        .bg(rgb(CARD))
        .border_1()
        .border_color(rgb(BORDER))
        .rounded_lg()
        .p_4()
}

pub fn title(text: impl Into<SharedString>) -> Div {
    div()
        .text_2xl()
        .font_weight(FontWeight::BOLD)
        .text_color(rgb(TEXT))
        .child(text.into())
}

pub fn section_title(text: impl Into<SharedString>) -> Div {
    div()
        .text_sm()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(MUTED))
        .child(text.into())
}

pub fn muted(text: impl Into<SharedString>) -> Div {
    div().text_sm().text_color(rgb(MUTED)).child(text.into())
}

pub fn button(id: impl Into<gpui::ElementId>, label: impl Into<SharedString>) -> Stateful<Div> {
    div()
        .id(id.into())
        .px_3()
        .py_1p5()
        .rounded_md()
        .bg(rgb(CARD_HOVER))
        .border_1()
        .border_color(rgb(BORDER))
        .text_sm()
        .text_color(rgb(TEXT))
        .cursor_pointer()
        .hover_style(|s| s.bg(rgb(BORDER)))
        .child(label.into())
}

pub fn primary_button(
    id: impl Into<gpui::ElementId>,
    label: impl Into<SharedString>,
) -> Stateful<Div> {
    div()
        .id(id.into())
        .px_3()
        .py_1p5()
        .rounded_md()
        .bg(rgb(ACCENT))
        .text_sm()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(0xffffff))
        .cursor_pointer()
        .hover_style(|s| s.bg(rgb(0x9d83ff)))
        .child(label.into())
}

/// Segmented-control item.
pub fn chip(
    id: impl Into<gpui::ElementId>,
    label: impl Into<SharedString>,
    selected: bool,
) -> Stateful<Div> {
    div()
        .id(id.into())
        .px_3()
        .py_1()
        .rounded_md()
        .text_sm()
        .cursor_pointer()
        .when(selected, |d| d.bg(rgb(ACCENT)).text_color(rgb(0xffffff)))
        .when(!selected, |d| {
            d.text_color(rgb(MUTED))
                .hover_style(|s| s.bg(rgb(CARD_HOVER)).text_color(rgb(TEXT)))
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
                .child(div().text_color(rgb(TEXT)).child(label.into()))
                .when(!hint.is_empty(), |d| d.child(muted(hint.to_string()))),
        )
        .child(
            div()
                .flex_none()
                .w(px(40.))
                .h(px(22.))
                .rounded_full()
                .p(px(3.))
                .flex()
                .when(on, |d| d.bg(rgb(ACCENT)).justify_end())
                .when(!on, |d| d.bg(rgb(BORDER)).justify_start())
                .child(div().size(px(16.)).rounded_full().bg(rgb(0xffffff))),
        )
}

/// Horizontal bar, `fraction` of the full width filled.
pub fn bar(fraction: f32, color: Rgba) -> Div {
    div()
        .h(px(8.))
        .w_full()
        .rounded_full()
        .bg(rgb(CARD_HOVER))
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
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(color))
                .child(value.into()),
        )
}
