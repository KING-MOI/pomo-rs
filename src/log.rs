use crate::session::Kind;
use chrono::{DateTime, FixedOffset, SecondsFormat};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
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
            self.start.to_rfc3339_opts(SecondsFormat::Secs, false),
            self.kind.key(),
            round_secs(self.planned),
            round_secs(self.actual)
        )
    }

    /// Whether the session ran for its full planned length.
    pub fn completed(&self) -> bool {
        self.actual >= self.planned
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

fn round_secs(d: Duration) -> u64 {
    (d.as_millis() as u64 + 500) / 1000
}

pub fn append(path: &Path, entry: &Entry) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut f = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(f, "{}", entry.to_line())
}

pub fn read(path: &Path) -> Result<Vec<Entry>, String> {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    text.lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
        .map(|(i, l)| Entry::parse(l).map_err(|e| format!("{}:{}: {e}", path.display(), i + 1)))
        .collect()
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

    #[test]
    fn append_then_read() {
        let dir = std::env::temp_dir().join(format!("pomo-log-{}", std::process::id()));
        let path = dir.join("nested").join("sessions.log");
        append(&path, &sample()).unwrap();
        append(&path, &sample()).unwrap();
        assert_eq!(read(&path).unwrap(), vec![sample(), sample()]);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn missing_log_reads_as_empty() {
        assert!(read(Path::new("/no/such/pomo.log")).unwrap().is_empty());
    }

    #[test]
    fn timestamps_are_written_without_fractions() {
        let mut e = sample();
        e.start = DateTime::parse_from_rfc3339("2026-11-17T09:00:00.123456-08:00").unwrap();
        assert!(e.to_line().starts_with("2026-11-17T09:00:00-08:00\t"));
    }

    #[test]
    fn durations_round_to_nearest_second() {
        let mut e = sample();
        e.actual = Duration::from_millis(959);
        e.planned = Duration::from_millis(1499);
        assert!(e.to_line().ends_with("\t1\t1"));
    }
}
