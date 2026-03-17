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
