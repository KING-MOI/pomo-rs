mod bar;
mod cli;
mod config;
mod duration;
mod keys;
mod log;
mod notify;
mod paths;
mod remote;
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
        Command::Stats {
            days,
            json,
            task,
            since,
        } => stats(days, json, task, since),
        Command::Config(cmd) => config_cmd(cmd),
        Command::Status { format } => status(format),
        Command::Pause => control("pause"),
        Command::Resume => control("resume"),
        Command::Toggle => control("toggle"),
        Command::Skip => control("skip"),
        Command::Stop => control("stop"),
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
    let state_dir = paths::state_dir();
    if let Some(other) = remote::load(&state_dir) {
        return Err(format!("a timer is already running (pid {})", other.pid));
    }
    let log_path = paths::log_file(config.log_file.as_deref());
    run::run(&config, args.task.as_deref(), args.count, &log_path, &state_dir).map_err(|e| e.to_string())
}

fn stats(
    days: u32,
    json: bool,
    task: Option<String>,
    since: Option<chrono::NaiveDate>,
) -> Result<(), String> {
    let config = Config::load(&paths::config_file())?;
    let (entries, warnings) = log::read(&paths::log_file(config.log_file.as_deref()))?;
    for w in warnings {
        eprintln!("pomo: skipping {w}");
    }
    let mut entries = entries;
    if let Some(t) = &task {
        entries.retain(|e| e.task.as_ref() == Some(t));
    }
    if let Some(d) = since {
        entries.retain(|e| e.start.date_naive() >= d);
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

fn status(format: Option<String>) -> Result<(), String> {
    let Some(state) = remote::load(&paths::state_dir()) else {
        if format.is_none() {
            println!("not running");
        }
        return Ok(());
    };
    let now = remote::now_ms();
    match format {
        Some(t) => println!("{}", remote::format(&t, &state, now)),
        None => println!("{}", bar::plain(state.kind, state.remaining(now), state.paused)),
    }
    Ok(())
}

fn control(command: &str) -> Result<(), String> {
    let dir = paths::state_dir();
    if remote::load(&dir).is_none() {
        return Err("no timer is running".into());
    }
    remote::send(&dir, command).map_err(|e| e.to_string())
}
