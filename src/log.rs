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
    /// Optional label for what the session was spent on.
    pub task: Option<String>,
}

impl Entry {
    pub fn to_line(&self) -> String {
        let mut line = format!(
            "{}\t{}\t{}\t{}",
            self.start.to_rfc3339_opts(SecondsFormat::Secs, false),
            self.kind.key(),
            round_secs(self.planned),
            round_secs(self.actual)
        );
        if let Some(t) = self.task.as_deref().filter(|t| !t.is_empty()) {
            line.push('\t');
            line.extend(t.chars().filter(|c| !c.is_control()));
        }
        line
    }

    /// Whether the session ran for its full planned length.
    pub fn completed(&self) -> bool {
        self.actual >= self.planned
    }

    pub fn parse(line: &str) -> Result<Entry, String> {
        let f: Vec<&str> = line.split('\t').collect();
        let (start, kind, planned, actual, task) = match f[..] {
            [s, k, p, a] => (s, k, p, a, None),
            [s, k, p, a, t] => (s, k, p, a, Some(t.to_string()).filter(|t| !t.is_empty())),
            _ => return Err(format!("expected 4 or 5 fields, got {}", f.len())),
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
            task,
        })
    }
}

impl Entry {
    /// One human readable line for `pomo log`.
    pub fn pretty(&self) -> String {
        let mark = if self.completed() { "✓" } else { "·" };
        let mut s = format!(
            "{}  {:<11}  {:>6}  {mark}",
            self.start.format("%Y-%m-%d %H:%M"),
            self.kind.label(),
            crate::duration::human(self.actual),
        );
        if let Some(t) = &self.task {
            s.push_str("  ");
            s.push_str(t);
        }
        s
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

/// Reads every entry it can. Lines that fail to parse are reported back
/// as warnings rather than failing the whole read, so one bad edit to the
/// log doesn't hide months of history.
pub fn read(path: &Path) -> Result<(Vec<Entry>, Vec<String>), String> {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Default::default()),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    let mut entries = Vec::new();
    let mut warnings = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        match Entry::parse(line) {
            Ok(e) => entries.push(e),
            Err(e) => warnings.push(format!("{}:{}: {e}", path.display(), i + 1)),
        }
    }
    Ok((entries, warnings))
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
            task: None,
        }
    }

    #[test]
    fn line_round_trip() {
        let line = sample().to_line();
        assert_eq!(line, "2026-11-17T09:00:00+05:30\twork\t1500\t1500");
        assert_eq!(Entry::parse(&line).unwrap(), sample());
    }

    #[test]
    fn task_is_an_optional_fifth_field() {
        let e = Entry {
            task: Some("write\treport\n".into()),
            ..sample()
        };
        let line = e.to_line();
        assert!(line.ends_with("\t1500\twritereport"));
        assert_eq!(
            Entry::parse(&line).unwrap().task.as_deref(),
            Some("writereport")
        );
    }

    #[test]
    fn empty_task_is_none() {
        let e = Entry {
            task: Some(String::new()),
            ..sample()
        };
        assert_eq!(e.to_line(), sample().to_line());
        let parsed = Entry::parse(&format!("{}\t", sample().to_line())).unwrap();
        assert_eq!(parsed.task, None);
    }

    #[test]
    fn pretty_line() {
        assert_eq!(sample().pretty(), "2026-11-17 09:00  work            25m  ✓");
        let skipped = Entry {
            actual: Duration::from_secs(600),
            task: Some("docs".into()),
            ..sample()
        };
        assert_eq!(skipped.pretty(), "2026-11-17 09:00  work            10m  ·  docs");
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
        assert_eq!(read(&path).unwrap().0, vec![sample(), sample()]);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn missing_log_reads_as_empty() {
        assert!(read(Path::new("/no/such/pomo.log")).unwrap().0.is_empty());
    }

    #[test]
    fn bad_lines_become_warnings() {
        let path = std::env::temp_dir().join(format!("pomo-bad-{}.log", std::process::id()));
        fs::write(
            &path,
            format!(
                "{}\ngarbage\n\n{}\n",
                sample().to_line(),
                sample().to_line()
            ),
        )
        .unwrap();
        let (entries, warnings) = read(&path).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].ends_with(":2: expected 4 or 5 fields, got 1"));
        fs::remove_file(path).unwrap();
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
