//! Talking to a running timer from another process via files in the
//! state directory: a `state` snapshot it publishes and a `control` queue
//! it reads.

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

/// Queues a command for the running timer to pick up on its next tick.
/// Appending keeps two quick commands (say `pause` then `skip`) from
/// overwriting each other.
pub fn send(dir: &Path, command: &str) -> io::Result<()> {
    use std::io::Write;
    fs::create_dir_all(dir)?;
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join(CONTROL))?;
    writeln!(f, "{command}")
}

/// Takes every queued command, oldest first, so each only runs once.
/// The file is renamed before reading so nothing appended in between is lost.
pub fn take(dir: &Path) -> Vec<String> {
    let path = dir.join(CONTROL);
    let taken = dir.join(format!("{CONTROL}.taken"));
    if fs::rename(&path, &taken).is_err() {
        return Vec::new();
    }
    let text = fs::read_to_string(&taken).unwrap_or_default();
    let _ = fs::remove_file(&taken);
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect()
}

/// Fills `{kind}`, `{remaining}`, `{state}` and `{percent}` in a status
/// template, for status bars like tmux or sketchybar.
pub fn format(template: &str, s: &State, now: u128) -> String {
    let left = crate::duration::ceil_secs(s.remaining(now));
    let done = s.length.saturating_sub(left);
    let percent = if s.length.is_zero() {
        100
    } else {
        done.as_millis() * 100 / s.length.as_millis()
    };
    template
        .replace("{kind}", s.kind.label())
        .replace("{remaining}", &crate::duration::clock(left))
        .replace("{state}", if s.paused { "paused" } else { "running" })
        .replace("{percent}", &percent.to_string())
}

impl State {
    pub fn to_json(&self, now: u128) -> String {
        format!(
            "{{\"kind\":\"{}\",\"remaining_secs\":{},\"length_secs\":{},\"paused\":{},\"pid\":{}}}",
            self.kind.key(),
            crate::duration::ceil_secs(self.remaining(now)).as_secs(),
            self.length.as_secs(),
            self.paused,
            self.pid
        )
    }
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
    fn format_fills_placeholders() {
        let s = sample();
        let out = format("{kind} {remaining} {state} {percent}%", &s, s.at);
        assert_eq!(out, "short break 04:00 running 20%");
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
        assert!(take(&dir).is_empty());
        send(&dir, "pause").unwrap();
        send(&dir, "skip").unwrap();
        assert_eq!(take(&dir), ["pause", "skip"]);
        assert!(take(&dir).is_empty());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn json_snapshot() {
        let s = State { pid: 7, ..sample() };
        assert_eq!(
            s.to_json(s.at),
            r#"{"kind":"short_break","remaining_secs":240,"length_secs":300,"paused":false,"pid":7}"#
        );
    }
}
