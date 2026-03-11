use std::time::Duration;

pub fn parse(s: &str) -> Result<Duration, String> {
    let s = s.trim();
    let err = || format!("invalid duration: {s:?}");
    if s.is_empty() {
        return Err("empty duration".into());
    }
    if let Ok(n) = s.parse::<u64>() {
        return Ok(Duration::from_secs(n * 60));
    }
    let mut total = 0u64;
    let mut rest = s;
    while !rest.is_empty() {
        let i = rest.find(|c: char| !c.is_ascii_digit()).ok_or_else(err)?;
        let n: u64 = rest[..i].parse().map_err(|_| err())?;
        let unit = rest[i..].chars().next().unwrap();
        total += match unit {
            's' => n,
            'm' => n * 60,
            'h' => n * 3600,
            _ => return Err(err()),
        };
        rest = &rest[i + unit.len_utf8()..];
    }
    Ok(Duration::from_secs(total))
}

pub fn human(d: Duration) -> String {
    let mins = d.as_secs() / 60;
    match (mins / 60, mins % 60) {
        (0, m) => format!("{m}m"),
        (h, 0) => format!("{h}h"),
        (h, m) => format!("{h}h{m}m"),
    }
}

pub fn clock(d: Duration) -> String {
    let secs = d.as_secs();
    if secs >= 3600 {
        return format!("{}:{:02}:{:02}", secs / 3600, secs / 60 % 60, secs % 60);
    }
    format!("{:02}:{:02}", secs / 60, secs % 60)
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

    #[test]
    fn combined_units() {
        assert_eq!(parse("1h30m").unwrap(), Duration::from_secs(5400));
        assert_eq!(parse("2m30s").unwrap(), Duration::from_secs(150));
    }

    #[test]
    fn rejects_garbage() {
        for bad in ["", "abc", "5x", "m", "1h30", "-5m"] {
            assert!(parse(bad).is_err(), "{bad:?} should not parse");
        }
    }

    #[test]
    fn clock_format() {
        assert_eq!(clock(Duration::from_secs(0)), "00:00");
        assert_eq!(clock(Duration::from_secs(25 * 60)), "25:00");
        assert_eq!(clock(Duration::from_secs(61)), "01:01");
        assert_eq!(clock(Duration::from_secs(3661)), "1:01:01");
    }

    #[test]
    fn human_format() {
        assert_eq!(human(Duration::from_secs(0)), "0m");
        assert_eq!(human(Duration::from_secs(25 * 60)), "25m");
        assert_eq!(human(Duration::from_secs(120 * 60)), "2h");
        assert_eq!(human(Duration::from_secs(95 * 60)), "1h35m");
    }
}
