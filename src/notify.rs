use serde::{Deserialize, Serialize};
use std::io::{self, IsTerminal, Write};
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Method {
    /// terminal-notifier when installed, OSC 9 otherwise
    #[default]
    Auto,
    Osc9,
    TerminalNotifier,
    /// plain terminal bell
    Bell,
    None,
}

/// Strips control characters so a message can never smuggle in its own
/// escape sequence and end the OSC early.
fn clean(msg: &str) -> String {
    msg.chars().filter(|c| !c.is_control()).collect()
}

/// OSC 9 desktop notification, understood by iTerm2, WezTerm, kitty and others.
pub fn osc9(msg: &str) -> String {
    format!("\x1b]9;{}\x07", clean(msg))
}

/// Wraps an escape sequence so tmux forwards it to the outer terminal.
pub fn tmux_wrap(seq: &str) -> String {
    format!("\x1bPtmux;{}\x1b\\", seq.replace('\x1b', "\x1b\x1b"))
}

fn in_tmux() -> bool {
    std::env::var_os("TMUX").is_some()
}

fn terminal_notifier(title: &str, msg: &str) -> io::Result<bool> {
    let status = Command::new("terminal-notifier")
        .args(["-title", title, "-message", msg])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    match status {
        Ok(s) => Ok(s.success()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e),
    }
}

pub fn send(method: Method, msg: &str) -> io::Result<()> {
    match method {
        Method::None => Ok(()),
        Method::Osc9 => send_osc9(msg),
        Method::Bell => {
            let mut out = io::stdout();
            write!(out, "\x07")?;
            out.flush()
        }
        Method::TerminalNotifier => terminal_notifier("pomo", msg).map(drop),
        Method::Auto => {
            if terminal_notifier("pomo", msg)? {
                Ok(())
            } else {
                send_osc9(msg)
            }
        }
    }
}

fn send_osc9(msg: &str) -> io::Result<()> {
    let mut out = io::stdout();
    if !out.is_terminal() {
        return Ok(());
    }
    let seq = osc9(msg);
    let seq = if in_tmux() { tmux_wrap(&seq) } else { seq };
    write!(out, "{seq}")?;
    out.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn osc9_sequence() {
        assert_eq!(osc9("hi"), "\x1b]9;hi\x07");
    }

    #[test]
    fn osc9_strips_control_chars() {
        assert_eq!(osc9("a\x07b\x1b]c\n"), "\x1b]9;ab]c\x07");
    }

    #[test]
    fn tmux_passthrough_doubles_escapes() {
        assert_eq!(
            tmux_wrap("\x1b]9;hi\x07"),
            "\x1bPtmux;\x1b\x1b]9;hi\x07\x1b\\"
        );
    }
}
