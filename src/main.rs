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
    let result = match cli.command.unwrap_or(Command::Start(Default::default())) {
        Command::Start(args) => start(args),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("pomo: {e}");
            ExitCode::FAILURE
        }
    }
}

fn start(args: cli::StartArgs) -> Result<(), String> {
    let mut config = Config::load(&paths::config_file())?;
    if let Some(d) = args.work {
        config.work = d;
    }
    if let Some(d) = args.short {
        config.short_break = d;
    }
    if let Some(d) = args.long {
        config.long_break = d;
    }
    config.validate()?;
    run::run(&config).map_err(|e| e.to_string())
}
