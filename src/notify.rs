use std::io::{self, Write};

/// OSC 9 desktop notification, understood by iTerm2, WezTerm, kitty and others.
pub fn osc9(msg: &str) -> String {
    format!("\x1b]9;{msg}\x07")
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
}
