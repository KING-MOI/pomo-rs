use crate::duration::human;
use crate::log::Entry;
use crate::session::Kind;
use chrono::NaiveDate;
use std::collections::BTreeMap;
use std::fmt::Write;
use std::time::Duration;

#[derive(Debug, Default, PartialEq)]
pub struct Totals {
    pub sessions: u32,
    pub focused: Duration,
}

pub fn totals(entries: &[Entry]) -> Totals {
    let mut t = Totals::default();
    for e in entries.iter().filter(|e| e.kind == Kind::Work) {
        t.sessions += 1;
        t.focused += e.actual;
    }
    t
}

/// Work totals grouped by the calendar day each session started on.
pub fn per_day(entries: &[Entry]) -> BTreeMap<NaiveDate, Totals> {
    let mut days: BTreeMap<NaiveDate, Totals> = BTreeMap::new();
    for e in entries.iter().filter(|e| e.kind == Kind::Work) {
        let t = days.entry(e.start.date_naive()).or_default();
        t.sessions += 1;
        t.focused += e.actual;
    }
    days
}

#[derive(Debug, Default, PartialEq)]
pub struct Streaks {
    pub current: u32,
    pub longest: u32,
}

/// Consecutive days with at least one work session. The current streak
/// survives until the end of the day after the last session, so not having
/// started yet today doesn't reset it.
pub fn streaks(days: &BTreeMap<NaiveDate, Totals>, today: NaiveDate) -> Streaks {
    let mut s = Streaks::default();
    let mut run = 0;
    let mut prev: Option<NaiveDate> = None;
    for &d in days.keys() {
        run = match prev {
            Some(p) if p.succ_opt() == Some(d) => run + 1,
            _ => 1,
        };
        s.longest = s.longest.max(run);
        prev = Some(d);
    }
    if let Some(last) = prev {
        if last == today || last.succ_opt() == Some(today) {
            s.current = run;
        }
    }
    s
}

fn plural(n: u32, word: &str) -> String {
    if n == 1 {
        format!("{n} {word}")
    } else {
        format!("{n} {word}s")
    }
}

pub fn report(entries: &[Entry], today: NaiveDate, last: u32) -> String {
    let all = totals(entries);
    let days = per_day(entries);
    let st = streaks(&days, today);
    let none = Totals::default();
    let t = days.get(&today).unwrap_or(&none);
    let mut out = String::new();
    let _ = writeln!(out, "today     {:>13}  {}", plural(t.sessions, "session"), human(t.focused));
    let _ = writeln!(
        out,
        "all time  {:>13}  {}",
        plural(all.sessions, "session"),
        human(all.focused)
    );
    let _ = writeln!(out, "streak    {:>13}  longest {}", plural(st.current, "day"), st.longest);
    if last > 0 {
        let _ = writeln!(out, "\nlast {last} days");
        for i in (0..last).rev() {
            let Some(d) = today.checked_sub_days(chrono::Days::new(i.into())) else {
                continue;
            };
            let t = days.get(&d).unwrap_or(&none);
            let _ = writeln!(out, "{d}  {:>3}  {}", t.sessions, human(t.focused));
        }
    }
    out
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use chrono::DateTime;

    pub fn entry(start: &str, kind: Kind, planned: u64, actual: u64) -> Entry {
        Entry {
            start: DateTime::parse_from_rfc3339(start).unwrap(),
            kind,
            planned: Duration::from_secs(planned),
            actual: Duration::from_secs(actual),
        }
    }

    #[test]
    fn totals_count_only_work() {
        let log = [
            entry("2026-11-17T09:00:00Z", Kind::Work, 1500, 1500),
            entry("2026-11-17T09:25:00Z", Kind::ShortBreak, 300, 300),
            entry("2026-11-17T09:30:00Z", Kind::Work, 1500, 1500),
        ];
        let t = totals(&log);
        assert_eq!(t.sessions, 2);
        assert_eq!(t.focused, Duration::from_secs(3000));
    }

    fn day(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    #[test]
    fn per_day_groups_by_local_date() {
        let log = [
            entry("2026-11-16T23:30:00-08:00", Kind::Work, 1500, 1500),
            entry("2026-11-17T08:00:00-08:00", Kind::Work, 1500, 600),
            entry("2026-11-17T09:00:00-08:00", Kind::Work, 1500, 1500),
        ];
        let days = per_day(&log);
        assert_eq!(days.len(), 2);
        assert_eq!(days[&day("2026-11-16")].sessions, 1);
        assert_eq!(days[&day("2026-11-17")].sessions, 2);
        assert_eq!(days[&day("2026-11-17")].focused, Duration::from_secs(2100));
    }

    fn days(list: &[&str]) -> BTreeMap<NaiveDate, Totals> {
        list.iter().map(|d| (day(d), Totals::default())).collect()
    }

    #[test]
    fn streak_counts_consecutive_days() {
        let d = days(&["2026-11-10", "2026-11-11", "2026-11-12", "2026-11-15", "2026-11-16"]);
        let s = streaks(&d, day("2026-11-16"));
        assert_eq!(s, Streaks { current: 2, longest: 3 });
    }

    #[test]
    fn streak_survives_until_tomorrow_ends() {
        let d = days(&["2026-11-15", "2026-11-16"]);
        assert_eq!(streaks(&d, day("2026-11-17")).current, 2);
        assert_eq!(streaks(&d, day("2026-11-18")).current, 0);
    }

    #[test]
    fn no_days_no_streak() {
        assert_eq!(streaks(&BTreeMap::new(), day("2026-11-17")), Streaks::default());
    }

    #[test]
    fn report_layout() {
        let log = [
            entry("2026-11-16T09:00:00Z", Kind::Work, 1500, 1500),
            entry("2026-11-17T09:00:00Z", Kind::Work, 1500, 1500),
            entry("2026-11-17T10:00:00Z", Kind::Work, 1500, 1500),
        ];
        let r = report(&log, day("2026-11-17"), 3);
        let expected = "\
today        2 sessions  50m
all time     3 sessions  1h15m
streak           2 days  longest 2

last 3 days
2026-11-15    0  0m
2026-11-16    1  25m
2026-11-17    2  50m
";
        assert_eq!(r, expected);
    }

    #[test]
    fn plural_words() {
        assert_eq!(plural(1, "day"), "1 day");
        assert_eq!(plural(0, "day"), "0 days");
        assert_eq!(plural(2, "session"), "2 sessions");
    }
}
