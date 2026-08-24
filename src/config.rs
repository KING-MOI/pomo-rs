use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

const MIN: u64 = 60;

/// Commented starter config written by `pomo config init`.
pub const TEMPLATE: &str = include_str!("../examples/config.toml");

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    #[serde(with = "crate::duration::serde")]
    pub work: Duration,
    #[serde(with = "crate::duration::serde")]
    pub short_break: Duration,
    #[serde(with = "crate::duration::serde")]
    pub long_break: Duration,
    pub long_break_every: u32,
    /// Full work sessions to aim for each day; 0 turns the goal off.
    pub daily_goal: u32,
    /// Nudge after the timer has sat paused this long; 0 turns it off.
    #[serde(with = "crate::duration::serde")]
    pub pause_reminder: Duration,
    /// Let work run past zero, counting up, until you move on.
    pub overtime: bool,
    /// Roll straight into the next session instead of waiting for a key.
    pub auto_start: bool,
    /// Overrides `auto_start` for breaks only, e.g. start breaks right away
    /// but wait before going back to work.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_start_breaks: Option<bool>,
    pub display: Display,
    pub bar_width: usize,
    pub bar_style: BarStyle,
    pub color: bool,
    /// Mirror the countdown in the terminal title.
    pub title: bool,
    pub notify: crate::notify::Method,
    /// macOS sound name played by terminal-notifier, e.g. "Glass".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sound: Option<String>,
    /// Shell command run after each session, with POMO_KIND, POMO_OUTCOME,
    /// POMO_SECONDS and POMO_TASK set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hook: Option<String>,
    /// Where finished sessions are recorded; defaults to the xdg data dir.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_file: Option<std::path::PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum BarStyle {
    /// eighth-block characters for a smooth edge
    #[default]
    Smooth,
    /// whole blocks only
    Blocks,
    /// `[###---]`, for fonts without block glyphs
    Ascii,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Display {
    #[default]
    Plain,
    Bar,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            work: Duration::from_secs(25 * MIN),
            short_break: Duration::from_secs(5 * MIN),
            long_break: Duration::from_secs(15 * MIN),
            long_break_every: 4,
            daily_goal: 0,
            pause_reminder: Duration::ZERO,
            overtime: false,
            auto_start: true,
            auto_start_breaks: None,
            display: Display::Plain,
            bar_width: 24,
            bar_style: BarStyle::Smooth,
            color: true,
            title: false,
            notify: Default::default(),
            sound: None,
            hook: None,
            log_file: None,
        }
    }
}

impl Config {
    pub fn from_toml(s: &str) -> Result<Config, String> {
        let c: Config = toml::from_str(s).map_err(|e| e.to_string())?;
        c.validate()?;
        Ok(c)
    }

    pub fn to_toml(&self) -> String {
        toml::to_string(self).expect("config always serializes")
    }

    pub fn load(path: &Path) -> Result<Config, String> {
        match std::fs::read_to_string(path) {
            Ok(s) => Config::from_toml(&s).map_err(|e| format!("{}: {e}", path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
            Err(e) => Err(format!("{}: {e}", path.display())),
        }
    }

    /// Whether `next` should begin without waiting for a key.
    pub fn starts_automatically(&self, next: crate::session::Kind) -> bool {
        match next {
            crate::session::Kind::Work => self.auto_start,
            _ => self.auto_start_breaks.unwrap_or(self.auto_start),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        for (name, d) in [
            ("work", self.work),
            ("short_break", self.short_break),
            ("long_break", self.long_break),
        ] {
            if d.is_zero() {
                return Err(format!("{name} must be longer than zero"));
            }
        }
        if self.bar_width > 200 {
            return Err("bar_width must be 200 or less (0 fits the terminal)".into());
        }
        if self.long_break_every == 0 {
            return Err("long_break_every must be at least 1".into());
        }
        Ok(())
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

    #[test]
    fn zero_lengths_are_rejected() {
        assert!(Config::from_toml("work = 0").is_err());
        assert!(Config::from_toml("short_break = \"0s\"").is_err());
        assert!(Config::from_toml("long_break_every = 0").is_err());
    }

    #[test]
    fn missing_file_gives_defaults() {
        let c = Config::load(Path::new("/definitely/not/here.toml")).unwrap();
        assert_eq!(c, Config::default());
    }

    #[test]
    fn display_mode() {
        assert_eq!(Config::default().display, Display::Plain);
        let c = Config::from_toml("display = \"bar\"\nbar_width = 10").unwrap();
        assert_eq!(c.display, Display::Bar);
        assert_eq!(c.bar_width, 10);
        assert!(Config::from_toml("display = \"fancy\"").is_err());
    }

    #[test]
    fn notify_method() {
        use crate::notify::Method;
        assert_eq!(Config::default().notify, Method::Auto);
        let c = Config::from_toml("notify = \"terminal-notifier\"").unwrap();
        assert_eq!(c.notify, Method::TerminalNotifier);
        let c = Config::from_toml("notify = \"none\"").unwrap();
        assert_eq!(c.notify, Method::None);
    }

    #[test]
    fn toml_round_trip() {
        let c = Config::from_toml("work = \"50m\"\ndisplay = \"bar\"").unwrap();
        let text = c.to_toml();
        assert!(text.contains("work = \"50m\""));
        assert_eq!(Config::from_toml(&text).unwrap(), c);
    }

    #[test]
    fn template_matches_defaults() {
        assert_eq!(Config::from_toml(TEMPLATE).unwrap(), Config::default());
    }

    #[test]
    fn log_file_override() {
        let c = Config::from_toml("log_file = \"/tmp/pomo.log\"").unwrap();
        assert_eq!(c.log_file.as_deref(), Some(Path::new("/tmp/pomo.log")));
    }

    #[test]
    fn bar_width_bounds() {
        assert!(Config::from_toml("bar_width = 0").is_ok());
        assert!(Config::from_toml("bar_width = 200").is_ok());
        assert!(Config::from_toml("bar_width = 201").is_err());
    }

    #[test]
    fn auto_start_can_differ_for_breaks() {
        use crate::session::Kind;
        let c = Config::from_toml("auto_start = false\nauto_start_breaks = true").unwrap();
        assert!(!c.starts_automatically(Kind::Work));
        assert!(c.starts_automatically(Kind::ShortBreak));
        let c = Config::from_toml("auto_start = false").unwrap();
        assert!(!c.starts_automatically(Kind::LongBreak));
    }

    #[test]
    fn pause_reminder_defaults_off() {
        assert!(Config::default().pause_reminder.is_zero());
        let c = Config::from_toml("pause_reminder = \"10m\"").unwrap();
        assert_eq!(c.pause_reminder, Duration::from_secs(600));
    }
}
