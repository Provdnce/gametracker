//! Time periods (all time / year / month / week / day) and playtime summaries.

use crate::db::Session;
use chrono::{Datelike, Duration, Local, NaiveDate, TimeZone};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeriodKind {
    All,
    Year,
    Month,
    Week,
    Day,
}

impl PeriodKind {
    pub const ALL: [PeriodKind; 5] = [
        PeriodKind::All,
        PeriodKind::Year,
        PeriodKind::Month,
        PeriodKind::Week,
        PeriodKind::Day,
    ];

    pub fn label(self) -> &'static str {
        match self {
            PeriodKind::All => "Gesamt",
            PeriodKind::Year => "Jahr",
            PeriodKind::Month => "Monat",
            PeriodKind::Week => "Woche",
            PeriodKind::Day => "Tag",
        }
    }
}

/// A period that contains `anchor`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Period {
    pub kind: PeriodKind,
    pub anchor: NaiveDate,
}

impl Period {
    pub fn new(kind: PeriodKind, anchor: NaiveDate) -> Self {
        Self { kind, anchor }
    }

    pub fn today(kind: PeriodKind) -> Self {
        Self::new(kind, Local::now().date_naive())
    }

    /// First and last day (inclusive). `None` for all time.
    pub fn days(&self) -> Option<(NaiveDate, NaiveDate)> {
        let a = self.anchor;
        match self.kind {
            PeriodKind::All => None,
            PeriodKind::Year => Some((
                NaiveDate::from_ymd_opt(a.year(), 1, 1)?,
                NaiveDate::from_ymd_opt(a.year(), 12, 31)?,
            )),
            PeriodKind::Month => {
                let first = NaiveDate::from_ymd_opt(a.year(), a.month(), 1)?;
                Some((first, next_month(first) - Duration::days(1)))
            }
            PeriodKind::Week => {
                let monday = a - Duration::days(a.weekday().num_days_from_monday() as i64);
                Some((monday, monday + Duration::days(6)))
            }
            PeriodKind::Day => Some((a, a)),
        }
    }

    /// Unix timestamps `[from, to)` covering the period in local time.
    pub fn timestamps(&self) -> (Option<i64>, Option<i64>) {
        match self.days() {
            None => (None, None),
            Some((a, b)) => (
                Some(local_midnight(a)),
                Some(local_midnight(b + Duration::days(1))),
            ),
        }
    }

    pub fn prev(&self) -> Self {
        self.shift(-1)
    }

    pub fn next(&self) -> Self {
        self.shift(1)
    }

    fn shift(&self, n: i32) -> Self {
        let a = self.anchor;
        let anchor = match self.kind {
            PeriodKind::All => a,
            PeriodKind::Year => NaiveDate::from_ymd_opt(a.year() + n, 1, 1).unwrap_or(a),
            PeriodKind::Month => {
                let months = a.year() * 12 + a.month0() as i32 + n;
                NaiveDate::from_ymd_opt(months.div_euclid(12), months.rem_euclid(12) as u32 + 1, 1)
                    .unwrap_or(a)
            }
            PeriodKind::Week => a + Duration::days(7 * n as i64),
            PeriodKind::Day => a + Duration::days(n as i64),
        };
        Self {
            kind: self.kind,
            anchor,
        }
    }

    /// True if the period contains today (so "next" makes no sense).
    pub fn is_current(&self) -> bool {
        match self.days() {
            None => true,
            Some((_, b)) => b >= Local::now().date_naive(),
        }
    }

    pub fn label(&self) -> String {
        let a = self.anchor;
        match self.kind {
            PeriodKind::All => "Gesamte Zeit".into(),
            PeriodKind::Year => a.year().to_string(),
            PeriodKind::Month => format!("{} {}", crate::fmt::month_name(a.month()), a.year()),
            PeriodKind::Week => {
                let (from, to) = self.days().unwrap();
                format!(
                    "KW {} · {} – {}",
                    a.iso_week().week(),
                    from.format("%d.%m."),
                    to.format("%d.%m.%Y")
                )
            }
            PeriodKind::Day => format!(
                "{}, {}",
                crate::fmt::weekday_name(a.weekday()),
                a.format("%d.%m.%Y")
            ),
        }
    }
}

fn next_month(first: NaiveDate) -> NaiveDate {
    if first.month() == 12 {
        NaiveDate::from_ymd_opt(first.year() + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(first.year(), first.month() + 1, 1).unwrap()
    }
}

pub fn local_midnight(d: NaiveDate) -> i64 {
    let naive = d.and_hms_opt(0, 0, 0).unwrap();
    Local
        .from_local_datetime(&naive)
        .earliest()
        // DST gap at midnight (rare): fall back to 1am
        .or_else(|| {
            Local
                .from_local_datetime(&(naive + Duration::hours(1)))
                .earliest()
        })
        .map(|t| t.timestamp())
        .unwrap_or_else(|| naive.and_utc().timestamp())
}

/// Seconds of `s` that fall into `[from, to)`.
fn overlap(s: &Session, from: Option<i64>, to: Option<i64>) -> i64 {
    let a = s.start.max(from.unwrap_or(i64::MIN));
    let b = s.end.min(to.unwrap_or(i64::MAX));
    (b - a).max(0)
}

#[derive(Debug, Clone)]
pub struct GameTotal {
    pub game_id: i64,
    pub name: String,
    pub secs: i64,
    pub sessions: usize,
    pub first_played: i64,
    pub last_played: i64,
}

/// Playtime per game inside `[from, to)`, most played first.
pub fn game_totals(sessions: &[Session], from: Option<i64>, to: Option<i64>) -> Vec<GameTotal> {
    let mut map: HashMap<i64, GameTotal> = HashMap::new();
    for s in sessions {
        let secs = overlap(s, from, to);
        if secs == 0 && !(from.is_none() && to.is_none()) {
            continue;
        }
        let e = map.entry(s.game_id).or_insert_with(|| GameTotal {
            game_id: s.game_id,
            name: s.game_name.clone(),
            secs: 0,
            sessions: 0,
            first_played: s.start,
            last_played: s.end,
        });
        e.secs += secs;
        e.sessions += 1;
        e.first_played = e.first_played.min(s.start);
        e.last_played = e.last_played.max(s.end);
    }
    let mut v: Vec<_> = map.into_values().collect();
    v.sort_by(|a, b| b.secs.cmp(&a.secs).then_with(|| a.name.cmp(&b.name)));
    v
}

#[derive(Debug, Clone)]
pub struct MonthSummary {
    pub year: i32,
    pub month: u32,
    pub secs: i64,
    pub sessions: usize,
    /// (game name, seconds), most played first
    pub games: Vec<(String, i64)>,
}

/// One entry per month that has playtime, newest first. Sessions that cross
/// a month boundary are split between the months.
pub fn months(sessions: &[Session]) -> Vec<MonthSummary> {
    // (year, month) -> (seconds, sessions, seconds per game)
    type MonthAcc = (i64, usize, HashMap<String, i64>);
    let mut map: HashMap<(i32, u32), MonthAcc> = HashMap::new();
    for s in sessions {
        let Some(start) = Local.timestamp_opt(s.start, 0).single() else {
            continue;
        };
        let mut month = NaiveDate::from_ymd_opt(start.year(), start.month(), 1).unwrap();
        loop {
            let from = local_midnight(month);
            if from >= s.end && from > s.start {
                break;
            }
            let nm = next_month(month);
            let secs = overlap(s, Some(from), Some(local_midnight(nm)));
            if secs > 0 {
                let e = map.entry((month.year(), month.month())).or_default();
                e.0 += secs;
                e.1 += 1;
                *e.2.entry(s.game_name.clone()).or_default() += secs;
            }
            month = nm;
        }
    }
    let mut v: Vec<_> = map
        .into_iter()
        .map(|((year, month), (secs, sessions, games))| {
            let mut games: Vec<_> = games.into_iter().collect();
            games.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            MonthSummary {
                year,
                month,
                secs,
                sessions,
                games,
            }
        })
        .collect();
    v.sort_by_key(|m| std::cmp::Reverse((m.year, m.month)));
    v
}

/// Playtime per day for each day in `[from, to]`.
pub fn daily_totals(sessions: &[Session], from: NaiveDate, to: NaiveDate) -> Vec<(NaiveDate, i64)> {
    let mut out = Vec::new();
    let mut d = from;
    while d <= to {
        let a = local_midnight(d);
        let b = local_midnight(d + Duration::days(1));
        let secs = sessions.iter().map(|s| overlap(s, Some(a), Some(b))).sum();
        out.push((d, secs));
        d += Duration::days(1);
    }
    out
}

pub fn total_secs(sessions: &[Session], from: Option<i64>, to: Option<i64>) -> i64 {
    sessions.iter().map(|s| overlap(s, from, to)).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn period_ranges() {
        let p = Period::new(PeriodKind::Week, d(2026, 10, 22)); // Thursday
        assert_eq!(p.days(), Some((d(2026, 10, 19), d(2026, 10, 25))));
        assert_eq!(p.prev().days().unwrap().0, d(2026, 10, 12));
        let m = Period::new(PeriodKind::Month, d(2026, 1, 15));
        assert_eq!(m.prev().days(), Some((d(2025, 12, 1), d(2025, 12, 31))));
        assert_eq!(m.next().days(), Some((d(2026, 2, 1), d(2026, 2, 28))));
        assert_eq!(m.label(), "Januar 2026");
        assert_eq!(Period::new(PeriodKind::All, d(2026, 1, 1)).days(), None);
    }

    fn sess(game: &str, start: i64, end: i64) -> Session {
        Session {
            id: format!("{game}{start}"),
            game_id: game.len() as i64,
            game_name: game.into(),
            start,
            end,
            machine: "m".into(),
        }
    }

    #[test]
    fn month_split() {
        // 1 hour before and 2 hours after local midnight on Nov 1st
        let midnight = local_midnight(d(2026, 11, 1));
        let s = vec![
            sess("Hades", midnight - 3600, midnight + 7200),
            sess("Celeste", midnight + 10_000, midnight + 10_600),
        ];
        let m = months(&s);
        assert_eq!(m.len(), 2);
        assert_eq!((m[0].month, m[0].secs), (11, 7200 + 600));
        assert_eq!(m[0].games[0], ("Hades".to_string(), 7200));
        assert_eq!((m[1].month, m[1].secs), (10, 3600));
        let totals = game_totals(&s, Some(midnight), None);
        assert_eq!(totals[0].secs, 7200);
        assert_eq!(total_secs(&s, None, None), 3600 + 7200 + 600);
    }
}
