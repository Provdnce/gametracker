use crate::theme::*;
use crate::view::{ChartId, MainView, Page, PeriodTarget};
use chrono::{Datelike, Duration, Local, Utc};
use gametracker_core::fmt;
use gametracker_core::stats::{self, Period, PeriodKind};
use gpui::{Context, FontWeight, SharedString, div, prelude::*, px, rgb};

impl MainView {
    pub fn render_overview(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let now = Utc::now().timestamp();
        let today = Local::now().date_naive();
        let range = |kind| Period::today(kind).timestamps();
        let total = |kind| {
            let (a, b) = range(kind);
            stats::total_secs(&self.sessions, a, b)
        };

        // Live banner
        let live = if !self.settings.track_games {
            card().child(muted("Spiele-Tracking ist pausiert. Du kannst es in den Einstellungen wieder einschalten."))
        } else if self.active.is_empty() {
            card().flex().items_center().gap_3().child(div().text_color(rgb(FAINT)).child("○")).child(muted(
                "Gerade läuft kein Spiel. Sobald du eins startest, wird es hier automatisch angezeigt.",
            ))
        } else {
            card()
                .border_color(rgb(LIVE))
                .flex()
                .flex_col()
                .gap_2()
                .children(self.active.iter().map(|g| {
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_3()
                                .child(div().text_color(rgb(LIVE)).child("●"))
                                .child(
                                    div()
                                        .text_lg()
                                        .font_weight(FontWeight::BOLD)
                                        .child(g.name.clone()),
                                )
                                .child(muted(format!("seit {} Uhr", fmt::time(g.start)))),
                        )
                        .child(
                            div()
                                .text_lg()
                                .text_color(rgb(LIVE))
                                .child(fmt::duration(now - g.start)),
                        )
                }))
        };

        let stat_row = div()
            .flex()
            .flex_wrap()
            .gap_3()
            .child(stat("Heute", fmt::duration(total(PeriodKind::Day)), TEXT))
            .child(stat(
                "Diese Woche",
                fmt::duration(total(PeriodKind::Week)),
                TEXT,
            ))
            .child(stat(
                format!("{} {}", fmt::month_name(today.month()), today.year()),
                fmt::duration(total(PeriodKind::Month)),
                TEXT,
            ))
            .child(stat(
                "Insgesamt",
                fmt::duration(total(PeriodKind::All)),
                ACCENT,
            ))
            .child(stat("Tasten heute", fmt::number(self.keys_today), LIVE));

        // Last 30 days chart
        let from = today - Duration::days(29);
        let days: Vec<(String, String, u64)> = stats::daily_totals(&self.sessions, from, today)
            .into_iter()
            .map(|(d, secs)| {
                (
                    d.day().to_string(),
                    format!("{}, {}", fmt::weekday_name(d.weekday()), d.format("%d.%m.")),
                    secs as u64,
                )
            })
            .collect();
        let chart = card()
            .flex()
            .flex_col()
            .gap_3()
            .child(section_title("SPIELZEIT DER LETZTEN 30 TAGE"))
            .child(self.column_chart(
                ChartId::Last30,
                &days,
                |v| fmt::duration(v as i64),
                ACCENT,
                cx,
            ));

        // Recent sessions
        let recent = card()
            .flex_1()
            .min_w(px(320.))
            .flex()
            .flex_col()
            .gap_3()
            .child(section_title("ZULETZT GESPIELT"))
            .when(self.sessions.is_empty(), |d| {
                d.child(muted("Noch keine Sessions aufgezeichnet."))
            })
            .children(self.sessions.iter().take(8).map(|s| {
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .flex_none()
                            .size(px(10.))
                            .rounded_full()
                            .bg(game_color(s.game_id)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_color(rgb(TEXT))
                                    .truncate()
                                    .child(s.game_name.clone()),
                            )
                            .child(div().text_xs().text_color(rgb(MUTED)).child(format!(
                                "{} · {}–{}",
                                fmt::date(s.start),
                                fmt::time(s.start),
                                fmt::time(s.end)
                            ))),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(MUTED))
                            .child(fmt::duration(s.duration())),
                    )
            }));

        // Top games all time
        let top = stats::game_totals(&self.sessions, None, None);
        let max = top.first().map(|g| g.secs).unwrap_or(1).max(1);
        let top_card = card()
            .flex_1()
            .min_w(px(320.))
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .justify_between()
                    .child(section_title("MEISTGESPIELT (GESAMT)"))
                    .child(
                        div()
                            .id("all-games")
                            .text_xs()
                            .text_color(rgb(ACCENT))
                            .cursor_pointer()
                            .child("Alle Spiele →")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.set_period(
                                    PeriodTarget::Games,
                                    Period::today(PeriodKind::All),
                                    cx,
                                );
                                this.go(Page::Games, cx);
                            })),
                    ),
            )
            .when(top.is_empty(), |d| d.child(muted("Noch keine Daten.")))
            .children(top.iter().take(6).map(|g| {
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .gap_2()
                            .child(div().truncate().child(g.name.clone()))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(MUTED))
                                    .flex_none()
                                    .child(fmt::duration(g.secs)),
                            ),
                    )
                    .child(bar(g.secs as f32 / max as f32, game_color(g.game_id)))
            }));

        div()
            .flex()
            .flex_col()
            .gap_5()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(title("Übersicht"))
                    .child(muted(format!(
                        "{}, {}",
                        fmt::weekday_name(today.weekday()),
                        today.format("%d.%m.%Y")
                    ))),
            )
            .child(live)
            .child(stat_row)
            .child(chart)
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_4()
                    .child(recent)
                    .child(top_card),
            )
    }

    pub fn render_history(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let months = stats::months(&self.sessions);
        let id_of = |name: &str| {
            self.games
                .iter()
                .find(|g| g.name == name)
                .map(|g| g.id)
                .unwrap_or(0)
        };

        let mut years: Vec<(i32, i64)> = Vec::new();
        for m in &months {
            match years.last_mut() {
                Some((y, secs)) if *y == m.year => *secs += m.secs,
                _ => years.push((m.year, m.secs)),
            }
        }

        div()
            .flex()
            .flex_col()
            .gap_5()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(title("Verlauf"))
                    .child(muted(
                        "Was hast du wann gespielt? Jeder Monat deines Lebens auf einen Blick.",
                    )),
            )
            .when(months.is_empty(), |d| {
                d.child(card().child(muted(
                    "Noch keine Daten. Starte ein Spiel – GameTracker zeichnet es automatisch auf.",
                )))
            })
            .when(!years.is_empty(), |d| {
                d.child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap_3()
                        .children(years.iter().map(|(y, secs)| {
                            let year = *y;
                            stat(year.to_string(), fmt::duration(*secs), TEXT)
                                .id(SharedString::from(format!("year-{year}")))
                                .cursor_pointer()
                                .hover_style(|s| s.border_color(rgb(ACCENT)))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    let anchor =
                                        chrono::NaiveDate::from_ymd_opt(year, 1, 1).unwrap();
                                    this.set_period(
                                        PeriodTarget::Games,
                                        Period::new(PeriodKind::Year, anchor),
                                        cx,
                                    );
                                    this.go(Page::Games, cx);
                                }))
                        })),
                )
            })
            .children(months.into_iter().map(|m| {
                let top = m.games.first().map(|g| g.1).unwrap_or(1).max(1);
                let shown = 6;
                let more = m.games.len().saturating_sub(shown);
                let (year, month) = (m.year, m.month);
                card()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .id(SharedString::from(format!("month-{year}-{month}")))
                            .flex()
                            .items_baseline()
                            .justify_between()
                            .cursor_pointer()
                            .on_click(cx.listener(move |this, _, _, cx| {
                                let anchor =
                                    chrono::NaiveDate::from_ymd_opt(year, month, 1).unwrap();
                                this.set_period(
                                    PeriodTarget::Games,
                                    Period::new(PeriodKind::Month, anchor),
                                    cx,
                                );
                                this.go(Page::Games, cx);
                            }))
                            .child(
                                div()
                                    .flex()
                                    .items_baseline()
                                    .gap_3()
                                    .child(
                                        div()
                                            .text_lg()
                                            .font_weight(FontWeight::BOLD)
                                            .child(format!("{} {}", fmt::month_name(month), year)),
                                    )
                                    .child(muted(format!(
                                        "{} · {} Spiele · {} Sessions",
                                        fmt::duration(m.secs),
                                        m.games.len(),
                                        m.sessions
                                    ))),
                            )
                            .child(div().text_xs().text_color(rgb(ACCENT)).child("Details →")),
                    )
                    .children(m.games.iter().take(shown).map(|(name, secs)| {
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .w(px(220.))
                                    .flex_none()
                                    .truncate()
                                    .text_sm()
                                    .child(name.clone()),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .child(bar(*secs as f32 / top as f32, game_color(id_of(name)))),
                            )
                            .child(
                                div()
                                    .w(px(90.))
                                    .flex_none()
                                    .text_right()
                                    .text_sm()
                                    .text_color(rgb(MUTED))
                                    .child(fmt::duration(*secs)),
                            )
                    }))
                    .when(more > 0, |d| {
                        d.child(
                            div()
                                .text_xs()
                                .text_color(rgb(FAINT))
                                .child(format!("+ {more} weitere")),
                        )
                    })
            }))
    }
}
