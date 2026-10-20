use crate::theme::*;
use crate::view::{MainView, PeriodTarget};
use gametracker_core::fmt;
use gametracker_core::stats;
use gpui::{Context, FontWeight, PromptLevel, Window, div, prelude::*, px, rgb};
use gpui_elements::editable_text::text_input;

impl MainView {
    pub fn render_games(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let (from, to) = self.games_period.timestamps();
        let totals = stats::game_totals(&self.sessions, from, to);
        let total_secs: i64 = totals.iter().map(|g| g.secs).sum();
        let session_count = self
            .sessions
            .iter()
            .filter(|s| s.end >= from.unwrap_or(i64::MIN) && s.start < to.unwrap_or(i64::MAX))
            .count();
        let max = totals.first().map(|g| g.secs).unwrap_or(1).max(1);
        let live: Vec<i64> = self.active.iter().map(|a| a.game_id).collect();

        let list = card()
            .p_0()
            .flex()
            .flex_col()
            .children(totals.iter().enumerate().map(|(i, g)| {
                let id = g.game_id;
                let selected = self.selected_game == Some(id);
                let is_live = live.contains(&id);
                let row = div()
                    .id(("game-row", i))
                    .flex()
                    .items_center()
                    .gap_4()
                    .px_4()
                    .py_3()
                    .cursor_pointer()
                    .when(i > 0, |d| d.border_t_1().border_color(rgb(c().border)))
                    .when(selected, |d| d.bg(rgb(c().card_hover)))
                    .hover_style(|s| s.bg(rgb(c().card_hover)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.message = None;
                        if this.selected_game == Some(id) {
                            this.selected_game = None;
                        } else {
                            this.selected_game = Some(id);
                            let name = this
                                .games
                                .iter()
                                .find(|g| g.id == id)
                                .map(|g| g.name.clone())
                                .unwrap_or_default();
                            this.rename_input.update(cx, |s, cx| s.emplace(&name, cx));
                        }
                        cx.notify();
                    }))
                    .child(
                        div()
                            .w(px(24.))
                            .text_sm()
                            .text_color(rgb(c().faint))
                            .child(format!("{}.", i + 1)),
                    )
                    .child(
                        div()
                            .flex_none()
                            .size(px(10.))
                            .rounded_full()
                            .bg(game_color(g.game_id)),
                    )
                    .child(
                        div()
                            .w(px(260.))
                            .flex_none()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .flex()
                                    .gap_2()
                                    .items_center()
                                    .child(
                                        div()
                                            .truncate()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child(g.name.clone()),
                                    )
                                    .when(is_live, |d| {
                                        d.child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap_1()
                                                .text_xs()
                                                .text_color(rgb(c().live))
                                                .child(dot(c().live, true))
                                                .child("läuft"),
                                        )
                                    }),
                            )
                            .child(div().text_xs().text_color(rgb(c().muted)).child(format!(
                                "{} Sessions · zuletzt {}",
                                g.sessions,
                                fmt::date(g.last_played)
                            ))),
                    )
                    .child(
                        div()
                            .flex_1()
                            .child(bar(g.secs as f32 / max as f32, game_color(g.game_id))),
                    )
                    .child(
                        div()
                            .w(px(110.))
                            .flex_none()
                            .text_right()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(fmt::duration(g.secs)),
                    );

                let details = selected.then(|| self.game_details(id, from, to, cx));
                div().flex().flex_col().child(row).children(details)
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
                    .child(title("Spiele"))
                    .child(muted(
                        "Klick auf ein Spiel für alle Sessions, zum Umbenennen oder Löschen.",
                    )),
            )
            .child(self.period_bar(PeriodTarget::Games, cx))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_3()
                    .child(stat("Spielzeit", fmt::duration(total_secs), c().accent))
                    .child(stat("Spiele", totals.len().to_string(), c().text))
                    .child(stat("Sessions", session_count.to_string(), c().text)),
            )
            .when(totals.is_empty(), |d| {
                d.child(card().child(muted("In diesem Zeitraum wurde nichts gespielt.")))
            })
            .when(!totals.is_empty(), |d| d.child(list))
    }

    fn game_details(
        &self,
        id: i64,
        from: Option<i64>,
        to: Option<i64>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let sessions: Vec<_> = self
            .sessions
            .iter()
            .filter(|s| {
                s.game_id == id
                    && s.end >= from.unwrap_or(i64::MIN)
                    && s.start < to.unwrap_or(i64::MAX)
            })
            .collect();
        let exe = self
            .games
            .iter()
            .find(|g| g.id == id)
            .and_then(|g| g.exe.clone());
        let longest = sessions.iter().map(|s| s.duration()).max().unwrap_or(0);
        let avg = if sessions.is_empty() {
            0
        } else {
            sessions.iter().map(|s| s.duration()).sum::<i64>() / sessions.len() as i64
        };
        let shown = 60;

        div()
            .flex()
            .flex_col()
            .gap_4()
            .px_6()
            .py_4()
            .bg(rgb(c().bg))
            .border_t_1()
            .border_color(rgb(c().border))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_6()
                    .text_sm()
                    .child(muted(format!("Längste Session: {}", fmt::duration(longest))))
                    .child(muted(format!("Ø pro Session: {}", fmt::duration(avg))))
                    .when_some(exe, |d, exe| d.child(muted(format!("Programm: {exe}")))),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        text_input("rename-input")
                            .state(self.rename_input.downgrade())
                            .placeholder_color(gpui::rgb_to_hsla(rgb(c().faint)))
                            .caret_color(gpui::rgb_to_hsla(rgb(c().accent)))
                            .selection_color(gpui::rgb_to_hsla(gpui::rgba((c().accent << 8) | 0x4d)))
                            .caret_blink_interval_500ms()
                            .text_color(rgb(c().text))
                            .placeholder("Neuer Name")
                            .w(px(300.))
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .bg(rgb(c().card))
                            .border_1()
                            .border_color(rgb(c().border))
                            .text_sm()
                            .whitespace_nowrap(),
                    )
                    .child(primary_button("rename-btn", "Umbenennen").on_click(cx.listener(move |this, _, _, cx| {
                        let name = this.rename_input.read(cx).as_str().trim().to_string();
                        if let Some(db) = &mut this.db {
                            match db.rename_game(id, &name) {
                                Ok(()) => {
                                    this.shared.rules_changed();
                                    this.flash(true, format!("Umbenannt in „{name}“."), cx);
                                }
                                Err(e) => this.flash(false, format!("{e:#}"), cx),
                            }
                        }
                        this.reload(cx);
                    })))
                    .child(button("ignore-btn", "Nicht mehr tracken").on_click(cx.listener(
                        move |this, _, window, cx| {
                            let name = this.games.iter().find(|g| g.id == id).map(|g| g.name.clone()).unwrap_or_default();
                            let answer = window.prompt(
                                PromptLevel::Info,
                                &format!("„{name}“ nicht mehr tracken?"),
                                Some("Rückgängig unter Einstellungen."),
                                &["Ignorieren, bisherige Zeit behalten", "Ignorieren und Verlauf löschen", "Abbrechen"],
                                cx,
                            );
                            cx.spawn(async move |this, cx| {
                                let choice = answer.await.unwrap_or(2);
                                if choice > 1 {
                                    return;
                                }
                                this.update(cx, |this, cx| {
                                    if let Some(db) = &this.db {
                                        let result = db.set_game_ignored(&name, true).and_then(|_| {
                                            if choice == 1 { db.delete_game(id) } else { Ok(()) }
                                        });
                                        match result {
                                            Ok(()) => {
                                                this.shared.rules_changed();
                                                if choice == 1 {
                                                    this.selected_game = None;
                                                }
                                                this.flash(true, format!("„{name}“ wird nicht mehr getrackt."), cx);
                                            }
                                            Err(e) => this.flash(false, format!("{e:#}"), cx),
                                        }
                                    }
                                    this.reload(cx);
                                })
                                .ok();
                            })
                            .detach();
                        },
                    )))
                    .child(button("delete-btn", "Löschen").text_color(rgb(c().danger)).on_click(cx.listener(
                        move |this, _, window, cx| {
                            let name = this.games.iter().find(|g| g.id == id).map(|g| g.name.clone()).unwrap_or_default();
                            let answer = window.prompt(
                                PromptLevel::Warning,
                                &format!("„{name}“ löschen?"),
                                Some("Alle Sessions dieses Spiels werden gelöscht. Das kann nicht rückgängig gemacht werden."),
                                &["Löschen", "Abbrechen"],
                                cx,
                            );
                            cx.spawn(async move |this, cx| {
                                if answer.await == Ok(0) {
                                    this.update(cx, |this, cx| {
                                        if let Some(db) = &this.db
                                            && let Err(e) = db.delete_game(id) {
                                                this.flash(false, format!("{e:#}"), cx);
                                            }
                                        this.selected_game = None;
                                        this.reload(cx);
                                    })
                                    .ok();
                                }
                            })
                            .detach();
                        },
                    ))),
            )
            .when_some(self.message.clone(), |d, (ok, msg)| {
                d.child(div().text_sm().text_color(rgb(if ok { c().live } else { c().danger })).child(msg))
            })
            .child(
                div().flex().flex_col().children(sessions.iter().take(shown).map(|s| {
                    div()
                        .flex()
                        .gap_4()
                        .py_1()
                        .text_sm()
                        .child(div().w(px(110.)).text_color(rgb(c().muted)).child(fmt::date(s.start)))
                        .child(
                            div()
                                .w(px(120.))
                                .text_color(rgb(c().muted))
                                .child(format!("{} – {}", fmt::time(s.start), fmt::time(s.end))),
                        )
                        .child(div().child(fmt::duration(s.duration())))
                })),
            )
            .when(sessions.len() > shown, |d| {
                d.child(div().text_xs().text_color(rgb(c().faint)).child(format!(
                    "… und {} ältere Sessions (Zeitraum eingrenzen, um sie zu sehen)",
                    sessions.len() - shown
                )))
            })
    }
}
