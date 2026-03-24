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
}
