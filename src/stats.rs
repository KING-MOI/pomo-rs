use crate::log::Entry;
use crate::session::Kind;
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
}
