pub const CLEAR_LINE: &str = "\r\x1b[2K";
pub const HIDE_CURSOR: &str = "\x1b[?25l";
pub const SHOW_CURSOR: &str = "\x1b[?25h";

pub fn paint(s: &str, color: u8) -> String {
    format!("\x1b[{color}m{s}\x1b[0m")
}

const PARTIAL: [char; 8] = [' ', '▏', '▎', '▍', '▌', '▋', '▊', '▉'];

/// Like [`bar`] but uses eighth-block characters for a smoother edge.
pub fn smooth(progress: f64, width: usize) -> String {
    let eighths = (progress.clamp(0.0, 1.0) * (width * 8) as f64) as usize;
    let full = eighths / 8;
    let mut s = "█".repeat(full);
    if full < width {
        s.push(PARTIAL[eighths % 8]);
        s.push_str(&" ".repeat(width - full - 1));
    }
    s
}

/// Renders a progress bar `width` cells wide for `progress` in 0.0..=1.0.
pub fn bar(progress: f64, width: usize) -> String {
    let filled = ((progress.clamp(0.0, 1.0) * width as f64).round() as usize).min(width);
    format!("{}{}", "█".repeat(filled), "░".repeat(width - filled))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_half_full() {
        assert_eq!(bar(0.0, 4), "░░░░");
        assert_eq!(bar(0.5, 4), "██░░");
        assert_eq!(bar(1.0, 4), "████");
    }

    #[test]
    fn out_of_range_is_clamped() {
        assert_eq!(bar(-1.0, 3), "░░░");
        assert_eq!(bar(7.0, 3), "███");
    }

    #[test]
    fn smooth_uses_partial_blocks() {
        assert_eq!(smooth(0.0, 2), "  ");
        assert_eq!(smooth(0.25, 2), "▌ ");
        assert_eq!(smooth(1.0, 2), "██");
        assert_eq!(smooth(0.5, 3).chars().count(), 3);
    }

    #[test]
    fn paint_wraps_in_sgr() {
        assert_eq!(paint("hi", 31), "\x1b[31mhi\x1b[0m");
    }
}
