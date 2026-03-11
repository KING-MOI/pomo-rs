use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct Config {
    pub work: u64,
    pub short_break: u64,
    pub long_break: u64,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            work: 25,
            short_break: 5,
            long_break: 15,
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
        assert_eq!(c.work, 50);
        assert_eq!(c.short_break, 5);
    }
}
