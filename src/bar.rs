pub const CLEAR_LINE: &str = "\r\x1b[2K";
pub const HIDE_CURSOR: &str = "\x1b[?25l";
pub const SHOW_CURSOR: &str = "\x1b[?25h";

/// Sets the terminal window or tab title (OSC 2).
pub fn title(s: &str) -> String {
    let clean: String = s.chars().filter(|c| !c.is_control()).collect();
    format!("\x1b]2;{clean}\x07")
}

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

use crate::duration;
use crate::session::Kind;
use std::time::Duration;

/// One frame of the plain display: `work 12:34`.
pub fn plain(kind: Kind, remaining: Duration, paused: bool) -> String {
    let mut s = format!(
        "{} {}",
        kind.label(),
        duration::clock(duration::ceil_secs(remaining))
    );
    if paused {
        s.push_str(" (paused)");
    }
    s
}

/// One frame of the bar display: `work ████░░░░ 12:34`.
pub fn frame(
    kind: Kind,
    progress: f64,
    remaining: Duration,
    paused: bool,
    width: usize,
    color: bool,
) -> String {
    let mut b = smooth(progress, width);
    if color {
        b = paint(&b, if paused { 90 } else { kind.color() });
    }
    let mut s = format!(
        "{} {} {}",
        kind.label(),
        b,
        duration::clock(duration::ceil_secs(remaining))
    );
    if paused {
        s.push_str(" (paused)");
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
    fn title_sequence_is_sanitized() {
        assert_eq!(title("work 12:00"), "\x1b]2;work 12:00\x07");
        assert_eq!(title("a\x07b"), "\x1b]2;ab\x07");
    }

    #[test]
    fn paint_wraps_in_sgr() {
        assert_eq!(paint("hi", 31), "\x1b[31mhi\x1b[0m");
    }

    #[test]
    fn frame_layout() {
        let f = frame(Kind::Work, 0.5, Duration::from_secs(90), false, 4, false);
        assert_eq!(f, "work ██   01:30");
    }

    #[test]
    fn frame_colors_the_bar_only() {
        let f = frame(Kind::ShortBreak, 1.0, Duration::ZERO, false, 2, true);
        assert_eq!(f, "short break \x1b[32m██\x1b[0m 00:00");
    }

    #[test]
    fn plain_layout() {
        assert_eq!(
            plain(Kind::LongBreak, Duration::from_millis(1500), false),
            "long break 00:02"
        );
    }

    #[test]
    fn paused_frames_say_so() {
        assert_eq!(
            plain(Kind::Work, Duration::from_secs(60), true),
            "work 01:00 (paused)"
        );
        let f = frame(Kind::Work, 0.0, Duration::from_secs(60), true, 1, true);
        assert_eq!(f, "work \x1b[90m \x1b[0m 01:00 (paused)");
    }
}
