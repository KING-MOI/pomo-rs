use std::time::Duration;

pub fn parse(s: &str) -> Result<Duration, String> {
    let s = s.trim();
    let err = || format!("invalid duration: {s:?}");
    let (num, unit) = match s.find(|c: char| !c.is_ascii_digit()) {
        Some(i) => s.split_at(i),
        None => (s, "m"),
    };
    let n: u64 = num.parse().map_err(|_| err())?;
    let secs = match unit {
        "s" => n,
        "m" => n * 60,
        "h" => n * 3600,
        _ => return Err(err()),
    };
    Ok(Duration::from_secs(secs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_number_is_minutes() {
        assert_eq!(parse("25").unwrap(), Duration::from_secs(25 * 60));
    }

    #[test]
    fn unit_suffixes() {
        assert_eq!(parse("90s").unwrap(), Duration::from_secs(90));
        assert_eq!(parse("5m").unwrap(), Duration::from_secs(300));
        assert_eq!(parse("2h").unwrap(), Duration::from_secs(7200));
    }
}
