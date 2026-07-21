use clap::{Args, Parser, Subcommand};
use std::time::Duration;

#[derive(Parser)]
#[command(
    name = "pomo",
    version,
    about = "A minimal pomodoro timer for the terminal"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
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
    /// Print a shell completion script
    Completions {
        /// bash, zsh, fish, elvish or powershell
        shell: clap_complete::Shell,
    },
    /// Inspect or create the config file
    #[command(subcommand)]
    Config(ConfigCommand),
}

#[derive(Args, Default)]
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

    /// What you're working on; saved with each session in the log
    #[arg(short, long)]
    pub task: Option<String>,

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

    /// Show a calendar heatmap of the last WEEKS weeks (default 12)
    #[arg(long, value_name = "WEEKS", num_args = 0..=1, default_missing_value = "12",
          value_parser = clap::value_parser!(u32).range(1..=104),
          conflicts_with_all = ["json", "weekdays"])]
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
    fn cli_definition_is_valid() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}
