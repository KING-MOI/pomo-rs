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

pub fn send(msg: &str) -> io::Result<()> {
    let mut out = io::stdout();
    write!(out, "{}", osc9(msg))?;
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
}
