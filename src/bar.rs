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
}
