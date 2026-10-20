use crate::theme::*;
use crate::view::{ChartId, MainView, PeriodTarget};
use gametracker_core::fmt;
use gametracker_core::keymap::{self, Layout};
use gametracker_core::stats::PeriodKind;
use gpui::{Context, FontWeight, SharedString, div, prelude::*, px, rgb};
use std::collections::HashMap;

const UNIT: f32 = 34.;
const GAP: f32 = 4.;

impl MainView {
    pub fn render_keyboard(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let layout = self.settings.layout;
        let counts: HashMap<&str, u64> = self
            .key_totals
            .iter()
            .map(|(k, n)| (k.as_str(), *n))
            .collect();
        let total: u64 = counts.values().sum();
        let max = self.key_totals.first().map(|k| k.1).unwrap_or(0).max(1);
        let days_with_data = self.key_chart.iter().filter(|c| c.2 > 0).count();

        let favorite = self
            .key_totals
            .first()
            .map(|(k, _)| keymap::long_label(k, layout))
            .unwrap_or_else(|| "–".into());
        let mut stats_row = div()
            .flex()
            .flex_wrap()
            .gap_3()
            .child(stat("Tastenanschläge", fmt::number(total), ACCENT))
            .child(stat("Lieblingstaste", favorite, LIVE))
            .child(stat(
                "Verschiedene Tasten",
                self.key_totals.len().to_string(),
                TEXT,
            ));
        if matches!(self.key_period.kind, PeriodKind::Week | PeriodKind::Month)
            && days_with_data > 0
        {
            stats_row = stats_row.child(stat(
                "Ø pro aktivem Tag",
                fmt::number(total / days_with_data as u64),
                TEXT,
            ));
        }

        // Hover info for the keyboard
        let rank: HashMap<&str, usize> = self
            .key_totals
            .iter()
            .enumerate()
            .map(|(i, (k, _))| (k.as_str(), i + 1))
            .collect();
        let hover_line = match &self.hovered_key {
            Some(k) => {
                let n = counts.get(k.as_str()).copied().unwrap_or(0);
                let pct = if total > 0 {
                    n as f64 * 100.0 / total as f64
                } else {
                    0.0
                };
                let place = rank
                    .get(k.as_str())
                    .map(|r| format!(" · Platz {r}"))
                    .unwrap_or_default();
                format!(
                    "{}: {}× ({} %){}",
                    keymap::long_label(k, layout),
                    fmt::number(n),
                    format!("{pct:.1}").replace('.', ","),
                    place
                )
            }
            None => "Fahr mit der Maus über eine Taste für Details.".into(),
        };

        let keyboard = card()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .flex()
                    .justify_between()
                    .items_center()
                    .child(section_title("HEATMAP"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(if self.hovered_key.is_some() {
                                TEXT
                            } else {
                                MUTED
                            }))
                            .child(hover_line),
                    ),
            )
            .child(self.keyboard_heatmap(layout, &counts, max, cx))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_xs()
                    .text_color(rgb(MUTED))
                    .child("selten")
                    .children((0..=10).map(|i| {
                        div()
                            .w(px(22.))
                            .h(px(8.))
                            .rounded_sm()
                            .bg(heat(i as f32 / 10.0))
                    }))
                    .child("oft"),
            );

        let top_max = self.key_totals.first().map(|k| k.1).unwrap_or(1).max(1);
        let top = card()
            .flex()
            .flex_col()
            .gap_2()
            .child(section_title("TOP 20 TASTEN"))
            .children(
                self.key_totals
                    .iter()
                    .take(20)
                    .enumerate()
                    .map(|(i, (k, n))| {
                        let pct = if total > 0 {
                            *n as f64 * 100.0 / total as f64
                        } else {
                            0.0
                        };
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .text_sm()
                            .child(
                                div()
                                    .w(px(28.))
                                    .text_color(rgb(FAINT))
                                    .child(format!("{}.", i + 1)),
                            )
                            .child(
                                div()
                                    .w(px(150.))
                                    .flex_none()
                                    .truncate()
                                    .child(keymap::long_label(k, layout)),
                            )
                            .child(div().flex_1().child(bar(
                                *n as f32 / top_max as f32,
                                heat((*n as f32 / top_max as f32).sqrt()),
                            )))
                            .child(div().w(px(100.)).text_right().child(fmt::number(*n)))
                            .child(
                                div()
                                    .w(px(60.))
                                    .text_right()
                                    .text_color(rgb(MUTED))
                                    .child(format!("{pct:.1} %").replace('.', ",")),
                            )
                    }),
            );

        let chart_title = match self.key_period.kind {
            PeriodKind::Week | PeriodKind::Month => "ANSCHLÄGE PRO TAG",
            _ => "ANSCHLÄGE PRO MONAT",
        };
        let chart = (!self.key_chart.is_empty()).then(|| {
            let values = self.key_chart.clone();
            card()
                .flex()
                .flex_col()
                .gap_3()
                .child(section_title(chart_title))
                .child(self.column_chart(
                    ChartId::Keys,
                    &values,
                    |v| format!("{} Anschläge", fmt::number(v)),
                    LIVE,
                    cx,
                ))
        });

        let notice = if let Some(err) = &self.key_error {
            Some(
                card()
                    .border_color(rgb(WARN))
                    .child(div().text_sm().text_color(rgb(WARN)).child(err.clone())),
            )
        } else if !self.settings.track_keys {
            Some(card().child(muted(
                "Tasten-Tracking ist pausiert. Einschalten unter Einstellungen.",
            )))
        } else {
            None
        };

        div()
            .flex()
            .flex_col()
            .gap_5()
            .child(
                div().flex().flex_col().gap_1().child(title("Tastatur")).child(muted(
                    "Welche Tasten drückst du wie oft? Gezählt wird nur die Anzahl pro Taste und Tag – nie, was du tippst.",
                )),
            )
            .children(notice)
            .child(self.period_bar(PeriodTarget::Keys, cx))
            .child(stats_row)
            .when(total == 0, |d| d.child(card().child(muted("Für diesen Zeitraum gibt es noch keine Tastendrücke."))))
            .child(keyboard)
            .children(chart)
            .when(total > 0, |d| d.child(top))
    }

    fn keyboard_heatmap(
        &self,
        layout: Layout,
        counts: &HashMap<&str, u64>,
        max: u64,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let blocks = keymap::keyboard(layout);
        div()
            .flex()
            .flex_wrap()
            .gap(px(14.))
            .children(blocks.into_iter().enumerate().map(|(b, rows)| {
                div()
                    .flex()
                    .flex_col()
                    .gap(px(GAP))
                    .children(rows.into_iter().enumerate().map(|(r, row)| {
                        div()
                            .flex()
                            .gap(px(GAP))
                            .children(row.into_iter().enumerate().map(|(s, slot)| {
                                let width = slot.width * UNIT + (slot.width - 1.0).max(0.0) * GAP;
                                let Some(id) = slot.id else {
                                    return div()
                                        .id(SharedString::from(format!("gap-{b}-{r}-{s}")))
                                        .w(px(width))
                                        .h(px(UNIT));
                                };
                                let n = counts.get(id).copied().unwrap_or(0);
                                let t = if n == 0 {
                                    0.0
                                } else {
                                    (n as f32 / max as f32).sqrt()
                                };
                                let hovered = self.hovered_key.as_deref() == Some(id);
                                let dark_text = t > 0.6;
                                div()
                                    .id(SharedString::from(format!("key-{b}-{r}-{s}")))
                                    .w(px(width))
                                    .h(px(UNIT))
                                    .flex_none()
                                    .rounded(px(5.))
                                    .bg(heat(t))
                                    .border_1()
                                    .border_color(if hovered { rgb(0xffffff) } else { rgb(BORDER) })
                                    .flex()
                                    .flex_col()
                                    .justify_center()
                                    .items_center()
                                    .overflow_hidden()
                                    .on_hover(cx.listener(move |this, h: &bool, _, cx| {
                                        if *h {
                                            this.hovered_key = Some(id.to_string());
                                        } else if this.hovered_key.as_deref() == Some(id) {
                                            this.hovered_key = None;
                                        }
                                        cx.notify();
                                    }))
                                    .when(slot.show_label, |d| {
                                        d.child(
                                            div()
                                                .text_xs()
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(rgb(if dark_text {
                                                    0x15121f
                                                } else {
                                                    TEXT
                                                }))
                                                .child(keymap::label(id, layout)),
                                        )
                                        .when(
                                            n > 0 && slot.width >= 1.0,
                                            |d| {
                                                d.child(
                                                    div()
                                                        .text_size(px(9.))
                                                        .text_color(rgb(if dark_text {
                                                            0x3a3350
                                                        } else {
                                                            MUTED
                                                        }))
                                                        .child(compact(n)),
                                                )
                                            },
                                        )
                                    })
                            }))
                    }))
            }))
    }
}

/// 1234 → 1,2k ; 1234567 → 1,2M
fn compact(n: u64) -> String {
    let s = if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1e6)
    } else if n >= 10_000 {
        format!("{}k", n / 1000)
    } else if n >= 1000 {
        format!("{:.1}k", n as f64 / 1e3)
    } else {
        n.to_string()
    };
    s.replace('.', ",")
}
