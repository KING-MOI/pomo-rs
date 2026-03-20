use std::time::{Duration, Instant};

/// A countdown that can be paused and resumed. Every method takes `now`
/// so the logic stays testable without sleeping.
#[derive(Debug, Clone)]
pub struct Timer {
    length: Duration,
    banked: Duration,
    running_since: Option<Instant>,
}

impl Timer {
    pub fn new(length: Duration) -> Timer {
        Timer {
            length,
            banked: Duration::ZERO,
            running_since: None,
        }
    }

    pub fn start(&mut self, now: Instant) {
        if self.running_since.is_none() {
            self.running_since = Some(now);
        }
    }

    pub fn elapsed(&self, now: Instant) -> Duration {
        let live = self
            .running_since
            .map_or(Duration::ZERO, |t| now.saturating_duration_since(t));
        self.banked + live
    }

    pub fn remaining(&self, now: Instant) -> Duration {
        self.length.saturating_sub(self.elapsed(now))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: Duration = Duration::from_secs(1);

    #[test]
    fn counts_down_while_running() {
        let t0 = Instant::now();
        let mut t = Timer::new(10 * S);
        assert_eq!(t.remaining(t0), 10 * S);
        t.start(t0);
        assert_eq!(t.remaining(t0 + 3 * S), 7 * S);
        assert_eq!(t.remaining(t0 + 30 * S), Duration::ZERO);
    }
}
