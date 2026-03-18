use crate::config::Config;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Work,
    ShortBreak,
    LongBreak,
}

impl Kind {
    pub fn length(self, c: &Config) -> Duration {
        match self {
            Kind::Work => c.work,
            Kind::ShortBreak => c.short_break,
            Kind::LongBreak => c.long_break,
        }
    }
}

/// Walks through work and break sessions in pomodoro order.
#[derive(Debug, Clone)]
pub struct Cycle {
    every: u32,
    done: u32,
    current: Kind,
}

impl Cycle {
    pub fn new(long_break_every: u32) -> Cycle {
        Cycle {
            every: long_break_every,
            done: 0,
            current: Kind::Work,
        }
    }

    pub fn completed_work(&self) -> u32 {
        self.done
    }

    pub fn current(&self) -> Kind {
        self.current
    }

    pub fn advance(&mut self) -> Kind {
        self.current = match self.current {
            Kind::Work => {
                self.done += 1;
                if self.done % self.every == 0 {
                    Kind::LongBreak
                } else {
                    Kind::ShortBreak
                }
            }
            Kind::ShortBreak | Kind::LongBreak => Kind::Work,
        };
        self.current
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classic_pomodoro_order() {
        let mut c = Cycle::new(4);
        let mut seen = vec![c.current()];
        for _ in 0..8 {
            seen.push(c.advance());
        }
        use Kind::*;
        assert_eq!(
            seen,
            [Work, ShortBreak, Work, ShortBreak, Work, ShortBreak, Work, LongBreak, Work]
        );
    }

    #[test]
    fn long_break_every_one() {
        let mut c = Cycle::new(1);
        assert_eq!(c.advance(), Kind::LongBreak);
        assert_eq!(c.advance(), Kind::Work);
        assert_eq!(c.advance(), Kind::LongBreak);
        assert_eq!(c.completed_work(), 2);
    }
}
