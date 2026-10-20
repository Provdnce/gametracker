//! Formatting helpers (German UI).

use chrono::{Local, TimeZone, Weekday};

pub fn month_name(m: u32) -> &'static str {
    [
        "Januar",
        "Februar",
        "März",
        "April",
        "Mai",
        "Juni",
        "Juli",
        "August",
        "September",
        "Oktober",
        "November",
        "Dezember",
    ]
    .get(m.saturating_sub(1) as usize)
    .copied()
    .unwrap_or("?")
}

pub fn weekday_name(w: Weekday) -> &'static str {
    match w {
        Weekday::Mon => "Montag",
        Weekday::Tue => "Dienstag",
        Weekday::Wed => "Mittwoch",
        Weekday::Thu => "Donnerstag",
        Weekday::Fri => "Freitag",
        Weekday::Sat => "Samstag",
        Weekday::Sun => "Sonntag",
    }
}

pub fn weekday_short(w: Weekday) -> &'static str {
    &weekday_name(w)[..2]
}

/// `3725` → `1 h 2 min`
pub fn duration(secs: i64) -> String {
    let secs = secs.max(0);
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    match (h, m) {
        (0, 0) if secs > 0 => "< 1 min".into(),
        (0, m) => format!("{m} min"),
        (h, 0) => format!("{h} h"),
        (h, m) => format!("{h} h {m} min"),
    }
}

/// Short form for tight spaces: `1,5 h`, `45 min`.
pub fn duration_short(secs: i64) -> String {
    if secs < 3600 {
        format!("{} min", secs / 60)
    } else {
        format!("{:.1} h", secs as f64 / 3600.0).replace('.', ",")
    }
}

/// `1234567` → `1.234.567`
pub fn number(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(c);
    }
    out
}

pub fn date(ts: i64) -> String {
    Local
        .timestamp_opt(ts, 0)
        .single()
        .map(|t| t.format("%d.%m.%Y").to_string())
        .unwrap_or_default()
}

pub fn date_time(ts: i64) -> String {
    Local
        .timestamp_opt(ts, 0)
        .single()
        .map(|t| t.format("%d.%m.%Y %H:%M").to_string())
        .unwrap_or_default()
}

pub fn time(ts: i64) -> String {
    Local
        .timestamp_opt(ts, 0)
        .single()
        .map(|t| t.format("%H:%M").to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats() {
        assert_eq!(duration(3725), "1 h 2 min");
        assert_eq!(duration(7200), "2 h");
        assert_eq!(duration(30), "< 1 min");
        assert_eq!(duration(0), "0 min");
        assert_eq!(number(1234567), "1.234.567");
        assert_eq!(number(12), "12");
        assert_eq!(duration_short(5400), "1,5 h");
    }
}
