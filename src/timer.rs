use std::time::{Duration, SystemTime};

/// A countdown that can be paused and resumed. Every method takes `now`
/// so the logic stays testable without sleeping.
///
/// It runs on wall-clock time rather than `Instant`: the monotonic clock
/// stops while the machine is asleep, which would stretch a session by
/// however long the lid was closed.
#[derive(Debug, Clone)]
pub struct Timer {
    length: Duration,
    banked: Duration,
    running_since: Option<SystemTime>,
}

impl Timer {
    pub fn new(length: Duration) -> Timer {
        Timer {
            length,
            banked: Duration::ZERO,
            running_since: None,
        }
    }

    pub fn start(&mut self, now: SystemTime) {
        if self.running_since.is_none() {
            self.running_since = Some(now);
        }
    }

    pub fn pause(&mut self, now: SystemTime) {
        if let Some(t) = self.running_since.take() {
            self.banked += now.duration_since(t).unwrap_or_default();
        }
    }

    pub fn toggle(&mut self, now: SystemTime) {
        if self.is_running() {
            self.pause(now);
        } else {
            self.start(now);
        }
    }

    pub fn is_running(&self) -> bool {
        self.running_since.is_some()
    }

    pub fn elapsed(&self, now: SystemTime) -> Duration {
        let live = self.running_since.map_or(Duration::ZERO, |t| {
            now.duration_since(t).unwrap_or_default()
        });
        self.banked + live
    }

    pub fn remaining(&self, now: SystemTime) -> Duration {
        self.length.saturating_sub(self.elapsed(now))
    }

    pub fn is_done(&self, now: SystemTime) -> bool {
        self.remaining(now).is_zero()
    }

    /// Fraction of the session elapsed, from 0.0 to 1.0.
    pub fn progress(&self, now: SystemTime) -> f64 {
        if self.length.is_zero() {
            return 1.0;
        }
        (self.elapsed(now).as_secs_f64() / self.length.as_secs_f64()).min(1.0)
    }

    /// Throws away elapsed time; keeps running if it was running.
    pub fn restart(&mut self, now: SystemTime) {
        self.banked = Duration::ZERO;
        if self.running_since.is_some() {
            self.running_since = Some(now);
        }
    }

    pub fn extend(&mut self, by: Duration) {
        self.length += by;
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
        let t0 = SystemTime::now();
        let mut t = Timer::new(10 * S);
        assert_eq!(t.remaining(t0), 10 * S);
        t.start(t0);
        assert_eq!(t.remaining(t0 + 3 * S), 7 * S);
        assert_eq!(t.remaining(t0 + 30 * S), Duration::ZERO);
    }

    #[test]
    fn pause_freezes_the_clock() {
        let t0 = SystemTime::now();
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
        let t0 = SystemTime::now();
        let mut t = Timer::new(5 * S);
        t.toggle(t0);
        assert!(t.is_running());
        t.toggle(t0 + S);
        assert!(!t.is_running());
        assert!(!t.is_done(t0 + 100 * S));
        t.toggle(t0 + 100 * S);
        assert!(t.is_done(t0 + 104 * S));
    }

    #[test]
    fn progress_is_clamped() {
        let t0 = SystemTime::now();
        let mut t = Timer::new(4 * S);
        assert_eq!(t.progress(t0), 0.0);
        t.start(t0);
        assert_eq!(t.progress(t0 + S), 0.25);
        assert_eq!(t.progress(t0 + 9 * S), 1.0);
        assert_eq!(Timer::new(Duration::ZERO).progress(t0), 1.0);
    }

    #[test]
    fn extend_adds_time() {
        let t0 = SystemTime::now();
        let mut t = Timer::new(2 * S);
        t.start(t0);
        assert!(t.is_done(t0 + 2 * S));
        t.extend(60 * S);
        assert_eq!(t.remaining(t0 + 2 * S), 60 * S);
    }

    #[test]
    fn restart_resets_elapsed() {
        let t0 = SystemTime::now();
        let mut t = Timer::new(10 * S);
        t.start(t0);
        t.restart(t0 + 4 * S);
        assert_eq!(t.remaining(t0 + 5 * S), 9 * S);
        t.pause(t0 + 5 * S);
        t.restart(t0 + 6 * S);
        assert!(!t.is_running());
        assert_eq!(t.remaining(t0 + 7 * S), 10 * S);
    }

    #[test]
    fn clock_going_backwards_does_not_panic() {
        let t0 = SystemTime::now();
        let mut t = Timer::new(10 * S);
        t.start(t0);
        assert_eq!(t.remaining(t0 - 5 * S), 10 * S);
        t.pause(t0 - 5 * S);
        assert_eq!(t.remaining(t0), 10 * S);
    }
}
