use std::io::{self, IsTerminal, Read};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Toggle,
    Skip,
    Quit,
    Extend,
    Restart,
}

impl Key {
    pub fn from_byte(b: u8) -> Option<Key> {
        match b {
            b' ' | b'p' => Some(Key::Toggle),
            b's' => Some(Key::Skip),
            b'+' | b'=' => Some(Key::Extend),
            b'r' => Some(Key::Restart),
            b'q' | 3 | 4 => Some(Key::Quit),
            _ => None,
        }
    }
}

/// Puts the terminal into unbuffered, no-echo mode and restores the
/// previous settings on drop.
pub struct RawMode {
    saved: String,
}

impl RawMode {
    pub fn enable() -> Option<RawMode> {
        let out = Command::new("stty")
            .arg("-g")
            .stdin(Stdio::inherit())
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let saved = String::from_utf8(out.stdout).ok()?.trim().to_string();
        let ok = Command::new("stty")
            .args(["-icanon", "-echo", "-isig", "min", "1"])
            .status()
            .is_ok_and(|s| s.success());
        ok.then_some(RawMode { saved })
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        let _ = Command::new("stty").arg(&self.saved).status();
    }
}

/// Reads keys from stdin on a background thread. Returns `None` when stdin
/// is not a terminal, in which case the timer just runs uninterrupted.
pub fn listen() -> Option<(RawMode, Receiver<Key>)> {
    if !io::stdin().is_terminal() {
        return None;
    }
    let raw = RawMode::enable()?;
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        for b in io::stdin().lock().bytes() {
            let Ok(b) = b else { break };
            if let Some(k) = Key::from_byte(b) {
                if tx.send(k).is_err() {
                    break;
                }
            }
        }
    });
    Some((raw, rx))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_bytes_to_keys() {
        assert_eq!(Key::from_byte(b' '), Some(Key::Toggle));
        assert_eq!(Key::from_byte(b'p'), Some(Key::Toggle));
        assert_eq!(Key::from_byte(b's'), Some(Key::Skip));
        assert_eq!(Key::from_byte(b'q'), Some(Key::Quit));
        assert_eq!(Key::from_byte(3), Some(Key::Quit));
        assert_eq!(Key::from_byte(b'+'), Some(Key::Extend));
        assert_eq!(Key::from_byte(b'r'), Some(Key::Restart));
        assert_eq!(Key::from_byte(b'x'), None);
    }
}
