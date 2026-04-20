mod bar;
mod cli;
mod config;
mod duration;
mod log;
mod notify;
mod paths;
mod run;
mod session;
mod stats;
mod timer;

use clap::Parser;
use cli::{Cli, Command};
use config::Config;
use std::io::IsTerminal;
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command.unwrap_or(Command::Start(Default::default())) {
        Command::Start(args) => start(args),
        Command::Stats { days } => stats(days),
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
    if let Some(m) = args.notify {
        config.notify = m;
    }
    if args.bar {
        config.display = config::Display::Bar;
    }
    if !std::io::stdout().is_terminal()
        || std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty())
    {
        config.color = false;
    }
    config.validate()?;
    run::run(&config, args.count, &paths::log_file()).map_err(|e| e.to_string())
}

fn stats(days: u32) -> Result<(), String> {
    let entries = log::read(&paths::log_file())?;
    let today = chrono::Local::now().date_naive();
    print!("{}", stats::report(&entries, today, days));
    Ok(())
}
