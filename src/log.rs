use crate::session::Kind;
use chrono::{DateTime, FixedOffset};
use std::time::Duration;

/// One finished (or skipped) session, stored as a tab separated line.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub start: DateTime<FixedOffset>,
    pub kind: Kind,
    pub planned: Duration,
    pub actual: Duration,
}

impl Entry {
    pub fn to_line(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}",
            self.start.to_rfc3339(),
            self.kind.key(),
            self.planned.as_secs(),
            self.actual.as_secs()
        )
    }

    pub fn parse(line: &str) -> Result<Entry, String> {
        let f: Vec<&str> = line.split('\t').collect();
        let [start, kind, planned, actual] = f[..] else {
            return Err(format!("expected 4 fields, got {}", f.len()));
        };
        let secs = |s: &str| {
            s.parse()
                .map(Duration::from_secs)
                .map_err(|_| format!("bad seconds: {s:?}"))
        };
        Ok(Entry {
            start: DateTime::parse_from_rfc3339(start).map_err(|e| format!("bad time: {e}"))?,
            kind: kind.parse()?,
            planned: secs(planned)?,
            actual: secs(actual)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Entry {
        Entry {
            start: DateTime::parse_from_rfc3339("2026-11-17T09:00:00+05:30").unwrap(),
            kind: Kind::Work,
            planned: Duration::from_secs(1500),
            actual: Duration::from_secs(1500),
        }
    }

    #[test]
    fn line_round_trip() {
        let line = sample().to_line();
        assert_eq!(line, "2026-11-17T09:00:00+05:30\twork\t1500\t1500");
        assert_eq!(Entry::parse(&line).unwrap(), sample());
    }

    #[test]
    fn rejects_malformed_lines() {
        assert!(Entry::parse("").is_err());
        assert!(Entry::parse("2026-11-17T09:00:00+05:30\twork\t1500").is_err());
        assert!(Entry::parse("yesterday\twork\t1500\t1500").is_err());
        assert!(Entry::parse("2026-11-17T09:00:00+05:30\tnap\t1500\t1500").is_err());
        assert!(Entry::parse("2026-11-17T09:00:00+05:30\twork\tlots\t1500").is_err());
    }
}
