mod bar;
mod cli;
mod config;
mod duration;
mod keys;
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
        Command::Stats { days, json } => stats(days, json),
        Command::Config(cmd) => config_cmd(cmd),
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
    args.apply(&mut config);
    if !std::io::stdout().is_terminal()
        || std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty())
    {
        config.color = false;
    }
    config.validate()?;
    run::run(&config, args.count, &paths::log_file()).map_err(|e| e.to_string())
}

fn stats(days: u32, json: bool) -> Result<(), String> {
    let (entries, warnings) = log::read(&paths::log_file())?;
    for w in warnings {
        eprintln!("pomo: skipping {w}");
    }
    let today = chrono::Local::now().date_naive();
    if json {
        println!("{}", stats::json(&entries, today, days));
    } else {
        print!("{}", stats::report(&entries, today, days));
    }
    Ok(())
}

fn config_cmd(cmd: cli::ConfigCommand) -> Result<(), String> {
    let path = paths::config_file();
    match cmd {
        cli::ConfigCommand::Path => println!("{}", path.display()),
        cli::ConfigCommand::Show => print!("{}", Config::load(&path)?.to_toml()),
        cli::ConfigCommand::Init { force } => {
            if path.exists() && !force {
                return Err(format!("{} already exists (use --force)", path.display()));
            }
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            }
            std::fs::write(&path, config::TEMPLATE).map_err(|e| e.to_string())?;
            println!("wrote {}", path.display());
        }
    }
    Ok(())
}
