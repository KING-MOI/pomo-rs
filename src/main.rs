mod cli;
mod config;
mod duration;
mod paths;
mod run;
mod session;
mod timer;

use clap::Parser;
use cli::{Cli, Command};
use config::Config;
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command.unwrap_or(Command::Start) {
        Command::Start => Config::load(&paths::config_file())
            .and_then(|c| run::run(&c).map_err(|e| e.to_string())),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("pomo: {e}");
            ExitCode::FAILURE
        }
    }
}
