use crate::config::Config;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Work,
    ShortBreak,
    LongBreak,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::Work => "work",
            Kind::ShortBreak => "short break",
            Kind::LongBreak => "long break",
        }
    }

    /// ANSI foreground color used when drawing this session.
    pub fn color(self) -> u8 {
        match self {
            Kind::Work => 31,
            Kind::ShortBreak => 32,
            Kind::LongBreak => 36,
        }
    }

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

    /// Starts the cycle on a break instead of work.
    pub fn starting_with(long_break_every: u32, first: Kind) -> Cycle {
        Cycle {
            current: first,
            ..Cycle::new(long_break_every)
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
                if self.done.is_multiple_of(self.every) {
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

impl std::str::FromStr for Kind {
    type Err = String;

    fn from_str(s: &str) -> Result<Kind, String> {
        match s {
            "work" => Ok(Kind::Work),
            "short" | "short_break" => Ok(Kind::ShortBreak),
            "long" | "long_break" => Ok(Kind::LongBreak),
            _ => Err(format!("unknown session kind: {s:?}")),
        }
    }
}

impl Kind {
    pub fn key(self) -> &'static str {
        match self {
            Kind::Work => "work",
            Kind::ShortBreak => "short_break",
            Kind::LongBreak => "long_break",
        }
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
            [
                Work, ShortBreak, Work, ShortBreak, Work, ShortBreak, Work, LongBreak, Work
            ]
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

    #[test]
    fn key_round_trips() {
        for k in [Kind::Work, Kind::ShortBreak, Kind::LongBreak] {
            assert_eq!(k.key().parse::<Kind>().unwrap(), k);
        }
        assert!("nap".parse::<Kind>().is_err());
    }

    #[test]
    fn can_start_on_a_break() {
        let mut c = Cycle::starting_with(4, Kind::ShortBreak);
        assert_eq!(c.current(), Kind::ShortBreak);
        assert_eq!(c.advance(), Kind::Work);
        assert_eq!(c.completed_work(), 0);
    }
}
