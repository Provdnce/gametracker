//! Main window: sidebar + one page at a time. Data is reloaded from the
//! database every two seconds so the numbers stay live.

use crate::theme::*;
use chrono::{Local, NaiveDate};
use gametracker_core::db::{Game, Rule, Session};
use gametracker_core::stats::{Period, PeriodKind};
use gametracker_core::tracker::{ActiveGame, RunningProcess};
use gametracker_core::{Db, Settings, Shared, fmt};
use gpui::{
    AnyElement, Context, Entity, FontWeight, ScrollHandle, SharedString, Window, div, point,
    prelude::*, px, rgb,
};
use gpui_elements::editable_text::{EditableTextState, StringStorage};
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Overview,
    Games,
    History,
    Keyboard,
    Settings,
}

impl Page {
    const ALL: [Page; 5] = [
        Page::Overview,
        Page::Games,
        Page::History,
        Page::Keyboard,
        Page::Settings,
    ];

    fn label(self) -> &'static str {
        match self {
            Page::Overview => "Übersicht",
            Page::Games => "Spiele",
            Page::History => "Verlauf",
            Page::Keyboard => "Tastatur",
            Page::Settings => "Einstellungen",
        }
    }

    fn icon(self) -> Icon {
        match self {
            Page::Overview => Icon::Overview,
            Page::Games => Icon::Games,
            Page::History => Icon::History,
            Page::Keyboard => Icon::Keyboard,
            Page::Settings => Icon::Settings,
        }
    }
}

/// Which period selector a click belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeriodTarget {
    Games,
    Keys,
}

/// Which chart a hovered column belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartId {
    Last30,
    Keys,
}

pub struct MainView {
    pub shared: Arc<Shared>,
    pub db: Option<Db>,
    pub page: Page,
    pub scroll: ScrollHandle,

    // live data
    pub sessions: Vec<Session>,
    pub games: Vec<Game>,
    pub active: Vec<ActiveGame>,
    pub settings: Settings,
    pub rules: Vec<Rule>,
    pub ignored: Vec<String>,
    pub running: Vec<RunningProcess>,
    pub key_error: Option<String>,
    pub last_error: Option<String>,
    pub keys_today: u64,
    pub keys_all: u64,
    pub autostart: bool,

    // games page
    pub games_period: Period,
    pub selected_game: Option<i64>,
    pub rename_input: Entity<EditableTextState>,

    // keyboard page
    pub key_period: Period,
    pub key_totals: Vec<(String, u64)>,
    pub key_chart: Vec<(String, String, u64)>,
    pub hovered_key: Option<String>,

    // charts
    pub hovered_column: Option<(ChartId, usize)>,

    // settings page
    pub proc_filter: Entity<EditableTextState>,
    pub message: Option<(bool, String)>,
}

impl MainView {
    pub fn new(shared: Arc<Shared>, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        let db = match Db::open(&shared.db_path) {
            Ok(db) => Some(db),
            Err(e) => {
                log::error!("{e:#}");
                None
            }
        };
        let rename_input = cx.new(|cx| EditableTextState::new(StringStorage::default(), cx));
        let proc_filter = cx.new(|cx| EditableTextState::new(StringStorage::default(), cx));

        // Refresh loop
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(2)).await;
                if this.update(cx, |this, cx| this.reload(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();

        let settings = shared.settings();
        let mut view = Self {
            shared,
            db,
            page: Page::Overview,
            scroll: ScrollHandle::new(),
            sessions: Vec::new(),
            games: Vec::new(),
            active: Vec::new(),
            settings,
            rules: Vec::new(),
            ignored: Vec::new(),
            running: Vec::new(),
            key_error: None,
            last_error: None,
            keys_today: 0,
            keys_all: 0,
            autostart: gametracker_core::autostart::is_enabled(),
            games_period: Period::today(PeriodKind::All),
            selected_game: None,
            rename_input,
            key_period: Period::today(PeriodKind::All),
            key_totals: Vec::new(),
            key_chart: Vec::new(),
            hovered_key: None,
            hovered_column: None,
            proc_filter,
            message: None,
        };
        view.reload(cx);
        view
    }

    /// Re-reads everything from the database and the tracker.
    pub fn reload(&mut self, cx: &mut Context<Self>) {
        self.active = self.shared.active_games();
        self.settings = self.shared.settings();
        self.running = self.shared.running_processes();
        self.key_error = self.shared.key_error.lock().unwrap().clone();
        self.last_error = self.shared.last_error.lock().unwrap().clone();
        let pending = self.shared.pending_key_total();

        let Some(db) = self.db.take() else {
            cx.notify();
            return;
        };
        let result = (|| -> anyhow::Result<()> {
            self.sessions = db.all_sessions()?;
            self.games = db.games()?;
            self.rules = db.rules()?;
            self.ignored = db.ignored_games()?;
            let today = Local::now().date_naive();
            self.keys_today = db
                .key_totals(Some((today, today)))?
                .iter()
                .map(|(_, n)| n)
                .sum::<u64>()
                + pending;
            self.keys_all = db.key_totals(None)?.iter().map(|(_, n)| n).sum::<u64>() + pending;
            self.key_totals = db.key_totals(self.key_period.days())?;
            self.key_chart = self.build_key_chart(&db)?;
            Ok(())
        })();
        self.db = Some(db);
        if let Err(e) = result {
            self.last_error = Some(format!("{e:#}"));
        }
        cx.notify();
    }

    /// Key presses over time for the keyboard page: per day for week/month,
    /// per month for year/all time.
    fn build_key_chart(&self, db: &Db) -> anyhow::Result<Vec<(String, String, u64)>> {
        use chrono::Datelike;
        let today = Local::now().date_naive();
        let range = match self.key_period.days() {
            Some(r) => r,
            None => match db.first_key_day()? {
                Some(first) => (first, today),
                None => return Ok(Vec::new()),
            },
        };
        let per_day = db.key_presses_per_day(range.0, range.1)?;
        Ok(match self.key_period.kind {
            PeriodKind::Day => Vec::new(),
            PeriodKind::Week | PeriodKind::Month => {
                let mut out = Vec::new();
                let mut d = range.0;
                while d <= range.1 {
                    let n = per_day
                        .iter()
                        .find(|(day, _)| *day == d)
                        .map(|(_, n)| *n)
                        .unwrap_or(0);
                    let label = if self.key_period.kind == PeriodKind::Week {
                        fmt::weekday_short(d.weekday()).to_string()
                    } else {
                        d.day().to_string()
                    };
                    let tip = format!(
                        "{}, {}",
                        fmt::weekday_name(d.weekday()),
                        d.format("%d.%m.%Y")
                    );
                    out.push((label, tip, n));
                    d += chrono::Duration::days(1);
                }
                out
            }
            PeriodKind::Year | PeriodKind::All => {
                let mut out: Vec<(String, String, u64)> = Vec::new();
                let mut month =
                    NaiveDate::from_ymd_opt(range.0.year(), range.0.month(), 1).unwrap();
                while month <= range.1 {
                    let n = per_day
                        .iter()
                        .filter(|(d, _)| d.year() == month.year() && d.month() == month.month())
                        .map(|(_, n)| n)
                        .sum();
                    let label = fmt::month_name(month.month())[..3].to_string();
                    let tip = format!("{} {}", fmt::month_name(month.month()), month.year());
                    out.push((label, tip, n));
                    month = Period::new(PeriodKind::Month, month).next().anchor;
                }
                out
            }
        })
    }

    pub fn go(&mut self, page: Page, cx: &mut Context<Self>) {
        self.page = page;
        self.message = None;
        self.scroll.set_offset(point(px(0.), px(0.)));
        cx.notify();
    }

    pub fn set_period(&mut self, target: PeriodTarget, period: Period, cx: &mut Context<Self>) {
        match target {
            PeriodTarget::Games => self.games_period = period,
            PeriodTarget::Keys => {
                self.key_period = period;
                self.hovered_column = None;
            }
        }
        self.reload(cx);
    }

    pub fn flash(&mut self, ok: bool, msg: impl Into<String>, cx: &mut Context<Self>) {
        self.message = Some((ok, msg.into()));
        cx.notify();
    }

    // ------------------------------------------------------------ widgets

    /// Gesamt / Jahr / Monat / Woche / Tag + < label > navigation.
    pub fn period_bar(&self, target: PeriodTarget, cx: &mut Context<Self>) -> impl IntoElement {
        let period = match target {
            PeriodTarget::Games => self.games_period,
            PeriodTarget::Keys => self.key_period,
        };
        let tid = match target {
            PeriodTarget::Games => "g",
            PeriodTarget::Keys => "k",
        };
        let kinds = div()
            .flex()
            .gap_1()
            .p_1()
            .rounded_lg()
            .bg(rgb(c().card))
            .border_1()
            .border_color(rgb(c().border))
            .children(PeriodKind::ALL.iter().enumerate().map(|(i, kind)| {
                let kind = *kind;
                chip(
                    SharedString::from(format!("{tid}-kind-{i}")),
                    kind.label(),
                    period.kind == kind,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    // Keep the anchor date so switching Monat → Woche stays in the same time.
                    let anchor = match target {
                        PeriodTarget::Games => this.games_period.anchor,
                        PeriodTarget::Keys => this.key_period.anchor,
                    };
                    this.set_period(target, Period::new(kind, anchor), cx);
                }))
            }));

        let nav = (period.kind != PeriodKind::All).then(|| {
            let at_end = period.is_current();
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    icon_button(SharedString::from(format!("{tid}-prev")), Icon::ChevronLeft)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.set_period(target, period.prev(), cx)
                        })),
                )
                .child(
                    div()
                        .min_w(px(220.))
                        .text_center()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(c().text))
                        .child(period.label()),
                )
                .child(
                    icon_button(
                        SharedString::from(format!("{tid}-next")),
                        Icon::ChevronRight,
                    )
                    .when(at_end, |b| b.opacity(0.35))
                    .when(!at_end, |b| {
                        b.on_click(cx.listener(move |this, _, _, cx| {
                            this.set_period(target, period.next(), cx)
                        }))
                    }),
                )
                .when(!at_end, |d| {
                    d.child(
                        button(SharedString::from(format!("{tid}-today")), "Heute").on_click(
                            cx.listener(move |this, _, _, cx| {
                                this.set_period(target, Period::today(period.kind), cx)
                            }),
                        ),
                    )
                })
        });

        div()
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap_3()
            .child(kinds)
            .children(nav)
    }

    /// Vertical column chart. `values` = (axis label, hover text, value).
    pub fn column_chart(
        &self,
        chart: ChartId,
        values: &[(String, String, u64)],
        value_fmt: fn(u64) -> String,
        color: u32,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let max = values.iter().map(|v| v.2).max().unwrap_or(0).max(1);
        let hovered = self
            .hovered_column
            .filter(|(c, _)| *c == chart)
            .map(|(_, i)| i);
        let caption = match hovered.and_then(|i| values.get(i)) {
            Some((_, tip, v)) => format!("{tip}: {}", value_fmt(*v)),
            None => format!("Maximum: {}", value_fmt(max)),
        };
        let n = values.len();
        let label_every = if n > 20 { (n / 10).max(1) } else { 1 };

        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(if hovered.is_some() {
                        c().text
                    } else {
                        c().muted
                    }))
                    .child(caption),
            )
            .child(div().flex().items_end().gap(px(3.)).h(px(140.)).children(
                values.iter().enumerate().map(|(i, (_, _, v))| {
                    let frac = *v as f32 / max as f32;
                    let is_hovered = hovered == Some(i);
                    div()
                        .id(SharedString::from(format!("col-{chart:?}-{i}")))
                        .flex_1()
                        .max_w(px(56.))
                        .h_full()
                        .flex()
                        .items_end()
                        .on_hover(cx.listener(move |this, h: &bool, _, cx| {
                            if *h {
                                this.hovered_column = Some((chart, i));
                            } else if this.hovered_column == Some((chart, i)) {
                                this.hovered_column = None;
                            }
                            cx.notify();
                        }))
                        .child(
                            div()
                                .w_full()
                                .rounded_t(px(3.))
                                .h(gpui::relative(if *v > 0 { frac.max(0.02) } else { 0.0 }))
                                .min_h(px(2.))
                                .bg(if *v == 0 {
                                    rgb(c().card_hover)
                                } else if is_hovered {
                                    mix(color, c().text, 0.3)
                                } else {
                                    rgb(color)
                                }),
                        )
                }),
            ))
            .child(
                div()
                    .flex()
                    .gap(px(3.))
                    .children(values.iter().enumerate().map(|(i, (label, _, _))| {
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .text_xs()
                            .text_center()
                            .text_color(rgb(c().faint))
                            .child(if i % label_every == 0 {
                                label.clone()
                            } else {
                                String::new()
                            })
                    })),
            )
    }

    fn sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = c();
        let nav = Page::ALL.iter().map(|page| {
            let page = *page;
            let selected = self.page == page;
            div()
                .id(SharedString::from(format!("nav-{}", page.label())))
                .flex()
                .items_center()
                .gap_3()
                .px_3()
                .py_2()
                .rounded_md()
                .cursor_pointer()
                .text_sm()
                .text_color(rgb(if selected { p.text } else { p.muted }))
                .when(selected, |d| {
                    d.bg(rgb(p.accent_soft)).font_weight(FontWeight::MEDIUM)
                })
                .when(!selected, |d| {
                    d.hover_style(move |s| s.bg(rgb(p.card)).text_color(rgb(p.text)))
                })
                .child(icon(
                    page.icon(),
                    16.,
                    if selected { p.accent } else { p.faint },
                ))
                .child(page.label())
                .on_click(cx.listener(move |this, _, _, cx| this.go(page, cx)))
        });

        let line = |i: Icon, color: u32, text: String| {
            div()
                .flex()
                .items_center()
                .gap_2()
                .text_color(rgb(color))
                .child(icon(i, 13., color))
                .child(text)
        };

        let game_status = match self.active.first() {
            Some(g) if self.settings.track_games => div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .text_color(rgb(p.live))
                        .child(dot(p.live, true))
                        .child("Spielt gerade"),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(p.text))
                        .truncate()
                        .child(g.name.clone()),
                )
                .child(
                    div()
                        .text_color(rgb(p.muted))
                        .child(fmt::duration(chrono::Utc::now().timestamp() - g.start)),
                ),
            _ if self.settings.track_games => div()
                .flex()
                .items_center()
                .gap_2()
                .text_color(rgb(p.muted))
                .child(dot(p.faint, false))
                .child("Kein Spiel aktiv"),
            _ => line(Icon::Pause, p.muted, "Spiele-Tracking pausiert".into()),
        };

        let key_status = if self.key_error.is_some() {
            line(Icon::Alert, p.warn, "Tastatur-Listener aus".into())
        } else if self.settings.track_keys {
            line(
                Icon::Keyboard,
                p.muted,
                format!("{} Tasten heute", fmt::number(self.keys_today)),
            )
        } else {
            line(Icon::Pause, p.muted, "Tasten-Tracking pausiert".into())
        };

        let status = div()
            .flex()
            .flex_col()
            .gap_3()
            .p_3()
            .rounded_lg()
            .bg(rgb(p.card))
            .border_1()
            .border_color(rgb(p.border))
            .text_xs()
            .child(game_status)
            .child(key_status);

        div()
            .flex_none()
            .w(px(220.))
            .h_full()
            .flex()
            .flex_col()
            .justify_between()
            .bg(rgb(p.sidebar))
            .border_r_1()
            .border_color(rgb(p.border))
            .p_3()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .px_2()
                            .pt_2()
                            .pb_6()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(logo(30.))
                            .child(
                                div()
                                    .text_base()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(p.text))
                                    .child("GameTracker"),
                            ),
                    )
                    .children(nav),
            )
            .child(
                div().flex().flex_col().gap_2().child(status).child(
                    div()
                        .id("quit")
                        .flex()
                        .items_center()
                        .gap_2()
                        .px_3()
                        .py_1()
                        .text_xs()
                        .text_color(rgb(p.faint))
                        .cursor_pointer()
                        .hover_style(move |s| s.text_color(rgb(p.danger)))
                        .child(icon(Icon::Power, 13., p.faint))
                        .child("Beenden")
                        .on_click(|_, _, cx| cx.quit()),
                ),
            )
    }
}

impl Render for MainView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        set_palette(&self.settings.theme_base, &self.settings.theme_accent);
        let p = c();
        let page: AnyElement = match self.page {
            Page::Overview => self.render_overview(cx).into_any_element(),
            Page::Games => self.render_games(window, cx).into_any_element(),
            Page::History => self.render_history(cx).into_any_element(),
            Page::Keyboard => self.render_keyboard(cx).into_any_element(),
            Page::Settings => self.render_settings(window, cx).into_any_element(),
        };

        let error_banner = self
            .last_error
            .clone()
            .map(|e| notice(false, format!("Fehler: {e}")));

        div()
            .size_full()
            .flex()
            .font_family("Inter")
            .bg(rgb(p.bg))
            .text_color(rgb(p.text))
            .child(self.sidebar(cx))
            .child(
                div()
                    .id("content")
                    .flex_1()
                    .h_full()
                    .min_w_0()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_5()
                            .p_8()
                            .max_w(px(1180.))
                            .children(error_banner)
                            .child(page),
                    ),
            )
    }
}
