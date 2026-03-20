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

    pub fn pause(&mut self, now: Instant) {
        if let Some(t) = self.running_since.take() {
            self.banked += now.saturating_duration_since(t);
        }
    }

    pub fn toggle(&mut self, now: Instant) {
        if self.is_running() {
            self.pause(now);
        } else {
            self.start(now);
        }
    }

    pub fn is_running(&self) -> bool {
        self.running_since.is_some()
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

    pub fn is_done(&self, now: Instant) -> bool {
        self.remaining(now).is_zero()
    }

    pub fn length(&self) -> Duration {
        self.length
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

    #[test]
    fn pause_freezes_the_clock() {
        let t0 = Instant::now();
        let mut t = Timer::new(10 * S);
        t.start(t0);
        t.pause(t0 + 2 * S);
        assert!(!t.is_running());
        assert_eq!(t.remaining(t0 + 60 * S), 8 * S);
        t.start(t0 + 60 * S);
        assert_eq!(t.remaining(t0 + 61 * S), 7 * S);
    }

    #[test]
    fn toggle_flips_running_state() {
        let t0 = Instant::now();
        let mut t = Timer::new(5 * S);
        t.toggle(t0);
        assert!(t.is_running());
        t.toggle(t0 + S);
        assert!(!t.is_running());
        assert!(!t.is_done(t0 + 100 * S));
        t.toggle(t0 + 100 * S);
        assert!(t.is_done(t0 + 104 * S));
    }
}
