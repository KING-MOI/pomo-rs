use std::io::{self, Write};

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

pub fn send(msg: &str) -> io::Result<()> {
    let mut out = io::stdout();
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
        assert_eq!(tmux_wrap("\x1b]9;hi\x07"), "\x1bPtmux;\x1b\x1b]9;hi\x07\x1b\\");
    }
}
