use std::env;
use std::path::PathBuf;

/// Expands a leading `~/` so config values like `~/notes/pomo.log` work.
pub fn expand_tilde(p: &std::path::Path) -> PathBuf {
    match p.strip_prefix("~") {
        Ok(rest) => home().join(rest),
        Err(_) => p.to_path_buf(),
    }
}

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
        return expand_tilde(p);
    }
    xdg("XDG_DATA_HOME", ".local/share").join("pomo").join("sessions.log")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn tilde_expands_to_home() {
        let home = home();
        assert_eq!(expand_tilde(Path::new("~/x.log")), home.join("x.log"));
        assert_eq!(expand_tilde(Path::new("/abs/x.log")), Path::new("/abs/x.log"));
        assert_eq!(expand_tilde(Path::new("~user/x")), Path::new("~user/x"));
    }
}
