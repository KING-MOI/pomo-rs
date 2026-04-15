use crate::log::Entry;
use crate::session::Kind;
use chrono::NaiveDate;
use std::collections::BTreeMap;
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
}
