use std::time::Duration;

pub fn parse(s: &str) -> Result<Duration, String> {
    let s = s.trim();
    let n: u64 = s.parse().map_err(|_| format!("invalid duration: {s:?}"))?;
    Ok(Duration::from_secs(n * 60))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_number_is_minutes() {
        assert_eq!(parse("25").unwrap(), Duration::from_secs(25 * 60));
    }
}
