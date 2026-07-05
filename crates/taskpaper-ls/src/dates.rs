//! Date parsing and relative formatting for @due/@done/@start values.

use chrono::{Local, NaiveDate};

/// Parse the leading date out of a tag value: "2026-07-05", optionally
/// followed by more text ("2026-07-05 17:00"). Anything else is None —
/// free-form values like @due(soon) are legitimate TaskPaper.
pub fn parse(value: &str) -> Option<NaiveDate> {
    let head = value.trim().get(..10)?;
    NaiveDate::parse_from_str(head, "%Y-%m-%d").ok()
}

pub fn today() -> NaiveDate {
    Local::now().date_naive()
}

/// Human phrase for `date` relative to `today`: "today", "tomorrow",
/// "in 3 days", "5 days overdue", ...
pub fn relative(date: NaiveDate, today: NaiveDate) -> String {
    let days = (date - today).num_days();
    match days {
        0 => "today".into(),
        1 => "tomorrow".into(),
        -1 => "1 day overdue".into(),
        d if d > 1 => format!("in {d} days"),
        d => format!("{} days overdue", -d),
    }
}

/// Like `relative`, but for past events ("@done(...)"): "today",
/// "yesterday", "3 days ago"; future dates fall back to "in N days".
pub fn ago(date: NaiveDate, today: NaiveDate) -> String {
    let days = (today - date).num_days();
    match days {
        0 => "today".into(),
        1 => "yesterday".into(),
        d if d > 1 => format!("{d} days ago"),
        d => format!("in {} days", -d),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn parsing() {
        assert_eq!(parse("2026-07-05"), Some(d("2026-07-05")));
        assert_eq!(parse(" 2026-07-05 17:00"), Some(d("2026-07-05")));
        assert_eq!(parse("soon"), None);
        assert_eq!(parse("2026-13-40"), None);
    }

    #[test]
    fn phrasing() {
        let t = d("2026-07-05");
        assert_eq!(relative(d("2026-07-05"), t), "today");
        assert_eq!(relative(d("2026-07-06"), t), "tomorrow");
        assert_eq!(relative(d("2026-07-08"), t), "in 3 days");
        assert_eq!(relative(d("2026-07-01"), t), "4 days overdue");
        assert_eq!(ago(d("2026-07-03"), t), "2 days ago");
    }
}
