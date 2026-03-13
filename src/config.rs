use serde::Deserialize;
use std::time::Duration;

const MIN: u64 = 60;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    #[serde(with = "crate::duration::serde")]
    pub work: Duration,
    #[serde(with = "crate::duration::serde")]
    pub short_break: Duration,
    #[serde(with = "crate::duration::serde")]
    pub long_break: Duration,
    pub long_break_every: u32,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            work: Duration::from_secs(25 * MIN),
            short_break: Duration::from_secs(5 * MIN),
            long_break: Duration::from_secs(15 * MIN),
            long_break_every: 4,
        }
    }
}

impl Config {
    pub fn from_toml(s: &str) -> Result<Config, toml::de::Error> {
        toml::from_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_file_gives_defaults() {
        assert_eq!(Config::from_toml("").unwrap(), Config::default());
    }

    #[test]
    fn overrides_individual_fields() {
        let c = Config::from_toml("work = 50").unwrap();
        assert_eq!(c.work, Duration::from_secs(50 * MIN));
        assert_eq!(c.short_break, Duration::from_secs(5 * MIN));
    }

    #[test]
    fn unknown_keys_are_rejected() {
        assert!(Config::from_toml("wrok = 50").is_err());
    }

    #[test]
    fn durations_accept_strings() {
        let c = Config::from_toml("work = \"1h\"\nshort_break = \"90s\"").unwrap();
        assert_eq!(c.work, Duration::from_secs(3600));
        assert_eq!(c.short_break, Duration::from_secs(90));
    }

    #[test]
    fn bad_duration_string_is_an_error() {
        assert!(Config::from_toml("work = \"soon\"").is_err());
    }
}
