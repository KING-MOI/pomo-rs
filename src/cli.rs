use clap::{Args, Parser, Subcommand};
use std::time::Duration;

#[derive(Parser)]
#[command(name = "pomo", version, about = "A minimal pomodoro timer for the terminal")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Start a pomodoro cycle
    Start(StartArgs),
    /// Show focus time, streaks and a per-day summary
    Stats {
        /// How many recent days to list
        #[arg(short, long, default_value_t = 7)]
        days: u32,

        /// Print machine readable JSON
        #[arg(long)]
        json: bool,
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

    /// Show a progress bar instead of a plain countdown
    #[arg(short, long)]
    pub bar: bool,

    /// How to notify when a session ends
    #[arg(long, value_enum)]
    pub notify: Option<crate::notify::Method>,

    /// Stop after this many work sessions
    #[arg(short = 'n', long)]
    pub count: Option<u32>,
}

#[derive(Subcommand)]
pub enum ConfigCommand {
    /// Print where the config file is read from
    Path,
    /// Print the effective config, defaults included
    Show,
}
