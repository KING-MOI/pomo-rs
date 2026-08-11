use clap::{Args, Parser, Subcommand};
use std::time::Duration;

#[derive(Parser)]
#[command(
    name = "pomo",
    version,
    about = "A minimal pomodoro timer for the terminal",
    args_conflicts_with_subcommands = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Flags for `start`, accepted directly so `pomo -w 50m` works
    #[command(flatten)]
    pub start: StartArgs,
}

#[derive(Subcommand)]
pub enum Command {
    /// Start a pomodoro cycle
    Start(StartArgs),
    /// Show focus time, streaks and a per-day summary
    Stats(StatsArgs),
    /// List recent sessions
    Log {
        /// How many sessions to show
        #[arg(short = 'n', long, default_value_t = 10)]
        limit: usize,

        /// Print the log file's path instead
        #[arg(long)]
        path: bool,
    },
    /// Show what the running timer is doing
    Status {
        /// Template with {kind}, {remaining}, {state} and {percent}
        #[arg(short, long)]
        format: Option<String>,
    },
    /// Pause the running timer
    Pause,
    /// Resume a paused timer
    Resume,
    /// Pause or resume the running timer
    Toggle,
    /// Skip to the next session
    Skip,
    /// Stop the running timer
    Stop,
    /// Restart the current session from the top
    Restart,
    /// Add time to the current session
    Extend {
        /// How much to add (default 5m)
        #[arg(default_value = "5m", value_parser = crate::duration::parse)]
        by: Duration,
    },
    /// Open the config file in $EDITOR
    Edit,
    /// Print a shell completion script
    Completions {
        /// bash, zsh, fish, elvish or powershell
        shell: clap_complete::Shell,
    },
    /// Inspect or create the config file
    #[command(subcommand)]
    Config(ConfigCommand),
}

#[derive(Args)]
pub struct StartArgs {
    /// Length of a work session (e.g. 25m, 1h, 90s)
    #[arg(short, long, value_parser = crate::duration::parse)]
    pub work: Option<Duration>,

    /// Length of a short break
    #[arg(short, long, value_parser = crate::duration::parse)]
    pub short: Option<Duration>,

    /// Length of a long break
    #[arg(short, long, value_parser = crate::duration::parse)]
    pub long: Option<Duration>,

    /// Take a long break after this many work sessions
    #[arg(short, long)]
    pub every: Option<u32>,

    /// Show a progress bar instead of a plain countdown
    #[arg(short, long)]
    pub bar: bool,

    /// How to notify when a session ends
    #[arg(long, value_enum)]
    pub notify: Option<crate::notify::Method>,

    /// Session to begin with: work, short or long
    #[arg(short, long, default_value = "work")]
    pub first: crate::session::Kind,

    /// What you're working on; saved with each session in the log
    #[arg(short, long)]
    pub task: Option<String>,

    /// Don't start new sessions after this time of day (HH:MM)
    #[arg(short, long, value_parser = parse_time)]
    pub until: Option<chrono::NaiveTime>,

    /// Stop after this many work sessions
    #[arg(short = 'n', long)]
    pub count: Option<u32>,
}

#[derive(Args)]
pub struct StatsArgs {
    /// How many recent days to list
    #[arg(short, long, default_value_t = 7)]
    pub days: u32,

    /// Print machine readable JSON
    #[arg(long)]
    pub json: bool,

    /// Only count sessions with this task label
    #[arg(short, long)]
    pub task: Option<String>,

    /// Only count sessions on or after this date (YYYY-MM-DD)
    #[arg(long)]
    pub since: Option<chrono::NaiveDate>,

    /// Show average focus per weekday instead
    #[arg(long, conflicts_with = "json")]
    pub weekdays: bool,

    /// Show totals per month instead
    #[arg(long, conflicts_with_all = ["json", "weekdays"])]
    pub months: bool,

    /// Show a calendar heatmap of the last WEEKS weeks (default 12)
    #[arg(long, value_name = "WEEKS", num_args = 0..=1, default_missing_value = "12",
          value_parser = clap::value_parser!(u32).range(1..=104),
          conflicts_with_all = ["json", "weekdays", "months"])]
    pub heatmap: Option<u32>,
}

#[derive(Subcommand)]
pub enum ConfigCommand {
    /// Print where the config file is read from
    Path,
    /// Print the effective config, defaults included
    Show,
    /// Write a commented starter config
    Init {
        /// Overwrite an existing file
        #[arg(short, long)]
        force: bool,
    },
}

fn parse_time(s: &str) -> Result<chrono::NaiveTime, String> {
    chrono::NaiveTime::parse_from_str(s, "%H:%M").map_err(|_| format!("expected HH:MM, got {s:?}"))
}

/// The next moment the clock reads `until`: later today, or tomorrow if
/// that time has already passed.
pub fn deadline(now: chrono::NaiveDateTime, until: chrono::NaiveTime) -> chrono::NaiveDateTime {
    let today = now.date().and_time(until);
    if today > now {
        today
    } else {
        today + chrono::Days::new(1)
    }
}

impl StartArgs {
    /// Layers command line flags over the loaded config.
    pub fn apply(&self, c: &mut crate::config::Config) {
        if let Some(d) = self.work {
            c.work = d;
        }
        if let Some(d) = self.short {
            c.short_break = d;
        }
        if let Some(d) = self.long {
            c.long_break = d;
        }
        if let Some(n) = self.every {
            c.long_break_every = n;
        }
        if let Some(m) = self.notify {
            c.notify = m;
        }
        if self.bar {
            c.display = crate::config::Display::Bar;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, Display};

    #[test]
    fn flags_override_config() {
        let cli = Cli::parse_from(["pomo", "start", "-w", "50m", "-e", "2", "--bar"]);
        let Some(Command::Start(args)) = cli.command else {
            panic!("expected start");
        };
        let mut c = Config::default();
        args.apply(&mut c);
        assert_eq!(c.work, Duration::from_secs(3000));
        assert_eq!(c.long_break_every, 2);
        assert_eq!(c.display, Display::Bar);
        assert_eq!(c.short_break, Config::default().short_break);
    }

    #[test]
    fn start_flags_work_without_the_subcommand() {
        let cli = Cli::parse_from(["pomo", "-w", "50m"]);
        assert!(cli.command.is_none());
        assert_eq!(cli.start.work, Some(Duration::from_secs(3000)));
    }

    #[test]
    fn cli_definition_is_valid() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }

    #[test]
    fn stats_views_are_exclusive() {
        for pair in [
            ["--json", "--months"],
            ["--heatmap", "--months"],
            ["--weekdays", "--json"],
        ] {
            let args = ["pomo", "stats", pair[0], pair[1]];
            assert!(
                Cli::try_parse_from(args).is_err(),
                "{pair:?} should conflict"
            );
        }
    }

    #[test]
    fn deadline_rolls_over_to_tomorrow() {
        let t = |s| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap();
        let five = parse_time("17:00").unwrap();
        assert_eq!(deadline(t("2026-11-17 09:00"), five), t("2026-11-17 17:00"));
        assert_eq!(deadline(t("2026-11-17 18:00"), five), t("2026-11-18 17:00"));
        assert!(parse_time("5pm").is_err());
    }
}
