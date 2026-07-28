mod bar;
mod cli;
mod config;
mod duration;
mod hook;
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
    let command = cli.command.unwrap_or(Command::Start(cli.start));
    let result = match command {
        Command::Start(args) => start(args),
        Command::Stats(args) => stats(args),
        Command::Config(cmd) => config_cmd(cmd),
        Command::Status { format } => status(format),
        Command::Log { limit, path } => show_log(limit, path),
        Command::Edit => edit(),
        Command::Completions { shell } => {
            use clap::CommandFactory;
            clap_complete::generate(shell, &mut Cli::command(), "pomo", &mut std::io::stdout());
            Ok(())
        }
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
    run::run(&config, &args, &log_path, &state_dir).map_err(|e| e.to_string())
}

/// Loads config and the session log, reporting bad log lines on stderr.
fn load_entries() -> Result<(Config, Vec<log::Entry>), String> {
    let config = Config::load(&paths::config_file())?;
    let (entries, warnings) = log::read(&paths::log_file(config.log_file.as_deref()))?;
    for w in warnings {
        eprintln!("pomo: skipping {w}");
    }
    Ok((config, entries))
}

fn stats(args: cli::StatsArgs) -> Result<(), String> {
    let (config, mut entries) = load_entries()?;
    if let Some(t) = &args.task {
        entries.retain(|e| e.task.as_ref() == Some(t));
    }
    if let Some(d) = args.since {
        entries.retain(|e| e.start.date_naive() >= d);
    }
    let today = chrono::Local::now().date_naive();
    if let Some(weeks) = args.heatmap {
        print!("{}", stats::heatmap(&entries, today, weeks));
    } else if args.months {
        print!("{}", stats::month_report(&entries));
    } else if args.weekdays {
        print!("{}", stats::weekday_report(&entries));
    } else if args.json {
        println!("{}", stats::json(&entries, today, args.days));
    } else {
        print!(
            "{}",
            stats::report(&entries, today, args.days, config.daily_goal)
        );
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
            write_template(&path)?;
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
        None => println!(
            "{}",
            bar::plain(state.kind, state.remaining(now), state.paused)
        ),
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

fn show_log(limit: usize, path: bool) -> Result<(), String> {
    if path {
        let config = Config::load(&paths::config_file())?;
        println!("{}", paths::log_file(config.log_file.as_deref()).display());
        return Ok(());
    }
    let (_, entries) = load_entries()?;
    for e in &entries[entries.len().saturating_sub(limit)..] {
        println!("{}", e.pretty());
    }
    Ok(())
}

/// Opens the config in the user's editor, creating it from the template
/// first, then checks the result so typos surface right away.
fn edit() -> Result<(), String> {
    let path = paths::config_file();
    if !path.exists() {
        write_template(&path)?;
    }
    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".into());
    let status = std::process::Command::new("sh")
        .args(["-c", &format!("{editor} \"$1\""), "sh"])
        .arg(&path)
        .status()
        .map_err(|e| format!("could not run {editor}: {e}"))?;
    if !status.success() {
        return Err(format!("{editor} exited with {status}"));
    }
    Config::load(&path).map(drop)
}

fn write_template(path: &std::path::Path) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, config::TEMPLATE).map_err(|e| format!("{}: {e}", path.display()))
}
