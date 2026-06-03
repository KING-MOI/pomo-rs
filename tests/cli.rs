use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

/// A throwaway home so tests never touch the real config or log.
struct Sandbox {
    dir: PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Sandbox {
        let dir = std::env::temp_dir().join(format!("pomo-it-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Sandbox { dir }
    }

    fn log(&self) -> PathBuf {
        self.dir.join("sessions.log")
    }

    fn cmd(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_pomo"))
            .args(args)
            .env("HOME", &self.dir)
            .env("POMO_CONFIG", self.dir.join("config.toml"))
            .env("POMO_LOG", self.log())
            .env("POMO_STATE_DIR", self.dir.join("state"))
            .env_remove("XDG_CONFIG_HOME")
            .env_remove("XDG_DATA_HOME")
            .env_remove("XDG_STATE_HOME")
            .output()
            .unwrap()
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

#[test]
fn help_lists_subcommands() {
    let o = Sandbox::new("help").cmd(&["--help"]);
    assert!(o.status.success());
    for sub in ["start", "stats", "status", "config"] {
        assert!(stdout(&o).contains(sub), "missing {sub}");
    }
}

#[test]
fn stats_on_an_empty_log() {
    let o = Sandbox::new("empty").cmd(&["stats", "-d", "0"]);
    assert!(o.status.success());
    assert!(stdout(&o).starts_with("today        0 sessions  0m\n"), "{}", stdout(&o));
}

#[test]
fn stats_reads_the_log() {
    let sb = Sandbox::new("stats");
    fs::write(
        sb.log(),
        "2020-01-01T09:00:00+00:00\twork\t1500\t1500\tcode\n\
         2020-01-01T09:30:00+00:00\tshort_break\t300\t300\n\
         2020-01-02T09:00:00+00:00\twork\t1500\t1500\n",
    )
    .unwrap();
    let out = stdout(&sb.cmd(&["stats", "-d", "0"]));
    assert!(out.contains("all time     2 sessions  50m"), "{out}");
    assert!(out.contains("longest 2"), "{out}");
    assert!(out.contains("code"), "{out}");
}
