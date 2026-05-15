use crate::session::Kind;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Snapshot of the running timer, written whenever it changes so other
/// `pomo` invocations can report on it.
#[derive(Debug, Clone, PartialEq)]
pub struct State {
    pub pid: u32,
    pub kind: Kind,
    pub length: Duration,
    pub elapsed: Duration,
    pub paused: bool,
    /// Unix time the snapshot was taken, in milliseconds.
    pub at: u128,
}

pub fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

impl State {
    pub fn to_text(&self) -> String {
        format!(
            "pid={}\nkind={}\nlength={}\nelapsed={}\npaused={}\nat={}\n",
            self.pid,
            self.kind.key(),
            self.length.as_millis(),
            self.elapsed.as_millis(),
            self.paused,
            self.at
        )
    }

    pub fn parse(text: &str) -> Option<State> {
        let get = |k: &str| {
            text.lines()
                .find_map(|l| l.strip_prefix(k).and_then(|r| r.strip_prefix('=')))
        };
        let ms = |k: &str| get(k)?.parse().ok().map(Duration::from_millis);
        Some(State {
            pid: get("pid")?.parse().ok()?,
            kind: get("kind")?.parse().ok()?,
            length: ms("length")?,
            elapsed: ms("elapsed")?,
            paused: get("paused")?.parse().ok()?,
            at: get("at")?.parse().ok()?,
        })
    }

    /// Time left as of unix time `now` (ms), accounting for time that has
    /// passed since the snapshot if the timer was running.
    pub fn remaining(&self, now: u128) -> Duration {
        let mut elapsed = self.elapsed;
        if !self.paused {
            elapsed += Duration::from_millis(now.saturating_sub(self.at) as u64);
        }
        self.length.saturating_sub(elapsed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> State {
        State {
            pid: 42,
            kind: Kind::ShortBreak,
            length: Duration::from_secs(300),
            elapsed: Duration::from_secs(60),
            paused: false,
            at: 1_000_000,
        }
    }

    #[test]
    fn text_round_trip() {
        assert_eq!(State::parse(&sample().to_text()), Some(sample()));
        assert_eq!(State::parse("pid=1\n"), None);
    }

    #[test]
    fn remaining_moves_only_while_running() {
        let s = sample();
        assert_eq!(s.remaining(1_010_000), Duration::from_secs(230));
        let paused = State { paused: true, ..s };
        assert_eq!(paused.remaining(9_999_999), Duration::from_secs(240));
    }
}
