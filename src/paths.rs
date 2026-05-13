use std::env;
use std::path::PathBuf;

fn home() -> PathBuf {
    env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."))
}

fn xdg(var: &str, fallback: &str) -> PathBuf {
    match env::var_os(var) {
        Some(v) if !v.is_empty() => PathBuf::from(v),
        _ => home().join(fallback),
    }
}

pub fn config_file() -> PathBuf {
    if let Some(p) = env::var_os("POMO_CONFIG") {
        return PathBuf::from(p);
    }
    xdg("XDG_CONFIG_HOME", ".config").join("pomo").join("config.toml")
}

/// `POMO_LOG` wins, then the config's `log_file`, then the xdg data dir.
pub fn log_file(configured: Option<&std::path::Path>) -> PathBuf {
    if let Some(p) = env::var_os("POMO_LOG") {
        return PathBuf::from(p);
    }
    if let Some(p) = configured {
        return p.to_path_buf();
    }
    xdg("XDG_DATA_HOME", ".local/share").join("pomo").join("sessions.log")
}
