use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "pomo", version, about = "A minimal pomodoro timer for the terminal")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Start a pomodoro cycle
    Start,
}
