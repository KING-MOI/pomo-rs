use crate::session::Kind;
use std::fs;
use std::io;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const STATE: &str = "state";
const CONTROL: &str = "control";

/// Snapshot of the running timer, written whenever it changes so other
/// `pomo` invocations can report on it.
#[derive(Debug, Clone, PartialEq)]
pub struct State {
    pub pid: u32,
    pub kind: Kind,
    pub length: Duration,
    pub elapsed: Duration,
    pub paused: bool,
    /// Unix time the snapshot was taken, in milliseconds.
    pub at: u128,
}

pub fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

impl State {
    pub fn to_text(&self) -> String {
        format!(
            "pid={}\nkind={}\nlength={}\nelapsed={}\npaused={}\nat={}\n",
            self.pid,
            self.kind.key(),
            self.length.as_millis(),
            self.elapsed.as_millis(),
            self.paused,
            self.at
        )
    }

    pub fn parse(text: &str) -> Option<State> {
        let get = |k: &str| {
            text.lines()
                .find_map(|l| l.strip_prefix(k).and_then(|r| r.strip_prefix('=')))
        };
        let ms = |k: &str| get(k)?.parse().ok().map(Duration::from_millis);
        Some(State {
            pid: get("pid")?.parse().ok()?,
            kind: get("kind")?.parse().ok()?,
            length: ms("length")?,
            elapsed: ms("elapsed")?,
            paused: get("paused")?.parse().ok()?,
            at: get("at")?.parse().ok()?,
        })
    }

    /// Time left as of unix time `now` (ms), accounting for time that has
    /// passed since the snapshot if the timer was running.
    pub fn remaining(&self, now: u128) -> Duration {
        let mut elapsed = self.elapsed;
        if !self.paused {
            elapsed += Duration::from_millis(now.saturating_sub(self.at) as u64);
        }
        self.length.saturating_sub(elapsed)
    }
}

/// Writes the snapshot atomically so a reader never sees half a file.
pub fn save(dir: &Path, state: &State) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let tmp = dir.join(format!("{STATE}.{}", std::process::id()));
    fs::write(&tmp, state.to_text())?;
    fs::rename(tmp, dir.join(STATE))
}

/// Loads the snapshot, ignoring it if the process that wrote it is gone
/// (e.g. killed with SIGKILL before it could clean up).
pub fn load(dir: &Path) -> Option<State> {
    let state = State::parse(&fs::read_to_string(dir.join(STATE)).ok()?)?;
    alive(state.pid).then_some(state)
}

fn alive(pid: u32) -> bool {
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

pub fn clear(dir: &Path) {
    let _ = fs::remove_file(dir.join(STATE));
}

/// Leaves a command for the running timer to pick up on its next tick.
pub fn send(dir: &Path, command: &str) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let tmp = dir.join(format!("{CONTROL}.{}", std::process::id()));
    fs::write(&tmp, command)?;
    fs::rename(tmp, dir.join(CONTROL))
}

/// Takes the pending command, if any, so it only runs once.
pub fn take(dir: &Path) -> Option<String> {
    let path = dir.join(CONTROL);
    let text = fs::read_to_string(&path).ok()?;
    fs::remove_file(&path).ok()?;
    Some(text.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> State {
        State {
            pid: std::process::id(),
            kind: Kind::ShortBreak,
            length: Duration::from_secs(300),
            elapsed: Duration::from_secs(60),
            paused: false,
            at: 1_000_000,
        }
    }

    #[test]
    fn text_round_trip() {
        assert_eq!(State::parse(&sample().to_text()), Some(sample()));
        assert_eq!(State::parse("pid=1\n"), None);
    }

    #[test]
    fn remaining_moves_only_while_running() {
        let s = sample();
        assert_eq!(s.remaining(1_010_000), Duration::from_secs(230));
        let paused = State { paused: true, ..s };
        assert_eq!(paused.remaining(9_999_999), Duration::from_secs(240));
    }

    fn tmp_dir(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("pomo-{name}-{}", std::process::id()))
    }

    #[test]
    fn save_load_clear() {
        let dir = tmp_dir("state");
        assert_eq!(load(&dir), None);
        save(&dir, &sample()).unwrap();
        assert_eq!(load(&dir), Some(sample()));
        clear(&dir);
        assert_eq!(load(&dir), None);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn state_from_a_dead_process_is_ignored() {
        let dir = tmp_dir("stale");
        let dead = State {
            pid: u32::MAX - 1,
            ..sample()
        };
        save(&dir, &dead).unwrap();
        assert_eq!(load(&dir), None);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn commands_are_taken_once() {
        let dir = tmp_dir("control");
        assert_eq!(take(&dir), None);
        send(&dir, "skip").unwrap();
        assert_eq!(take(&dir).as_deref(), Some("skip"));
        assert_eq!(take(&dir), None);
        fs::remove_dir_all(dir).unwrap();
    }
}
