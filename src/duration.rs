use std::time::Duration;

pub fn parse(s: &str) -> Result<Duration, String> {
    let s = s.trim();
    let err = || format!("invalid duration: {s:?}");
    if s.is_empty() {
        return Err("empty duration".into());
    }
    if let Ok(n) = s.parse::<u64>() {
        return n.checked_mul(60).map(Duration::from_secs).ok_or_else(err);
    }
    let mut total = 0u64;
    let mut rest = s;
    while !rest.is_empty() {
        let i = rest.find(|c: char| !c.is_ascii_digit()).ok_or_else(err)?;
        let n: u64 = rest[..i].parse().map_err(|_| err())?;
        let unit = rest[i..].chars().next().unwrap();
        let mult = match unit {
            's' => 1,
            'm' => 60,
            'h' => 3600,
            _ => return Err(err()),
        };
        total = n
            .checked_mul(mult)
            .and_then(|v| total.checked_add(v))
            .ok_or_else(err)?;
        rest = &rest[i + unit.len_utf8()..];
    }
    Ok(Duration::from_secs(total))
}

/// Rounds up to a whole second so a countdown never shows 00:00 early.
pub fn ceil_secs(d: Duration) -> Duration {
    let extra = u64::from(d.subsec_nanos() > 0);
    Duration::from_secs(d.as_secs() + extra)
}

pub fn human(d: Duration) -> String {
    let mins = d.as_secs() / 60;
    match (mins / 60, mins % 60) {
        (0, m) => format!("{m}m"),
        (h, 0) => format!("{h}h"),
        (h, m) => format!("{h}h{m}m"),
    }
}

/// Shortest string [`parse`] turns back into the same duration.
pub fn compact(d: Duration) -> String {
    let secs = d.as_secs();
    if secs == 0 {
        return "0s".into();
    }
    [(secs / 3600, 'h'), (secs / 60 % 60, 'm'), (secs % 60, 's')]
        .iter()
        .filter(|(n, _)| *n > 0)
        .map(|(n, unit)| format!("{n}{unit}"))
        .collect()
}

pub fn clock(d: Duration) -> String {
    let secs = d.as_secs();
    if secs >= 3600 {
        return format!("{}:{:02}:{:02}", secs / 3600, secs / 60 % 60, secs % 60);
    }
    format!("{:02}:{:02}", secs / 60, secs % 60)
}

pub mod serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Raw {
        Minutes(u64),
        Text(String),
    }

    pub fn serialize<S: Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&super::compact(*d))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
        match Raw::deserialize(d)? {
            Raw::Minutes(m) => m
                .checked_mul(60)
                .map(Duration::from_secs)
                .ok_or_else(|| serde::de::Error::custom("duration too large")),
            Raw::Text(s) => super::parse(&s).map_err(serde::de::Error::custom),
        }
    }
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
        for bad in ["", "abc", "5x", "m", "1h30", "-5m", "99999999999999999999h"] {
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
    fn overflow_is_an_error() {
        assert!(parse("307445734561825861m").is_err());
        assert!(parse("18446744073709551615").is_err());
    }

    #[test]
    fn human_format() {
        assert_eq!(human(Duration::from_secs(0)), "0m");
        assert_eq!(human(Duration::from_secs(25 * 60)), "25m");
        assert_eq!(human(Duration::from_secs(120 * 60)), "2h");
        assert_eq!(human(Duration::from_secs(95 * 60)), "1h35m");
    }

    #[test]
    fn ceil_rounds_partial_seconds_up() {
        assert_eq!(ceil_secs(Duration::from_millis(1)), Duration::from_secs(1));
        assert_eq!(ceil_secs(Duration::from_millis(2000)), Duration::from_secs(2));
        assert_eq!(ceil_secs(Duration::ZERO), Duration::ZERO);
    }

    #[test]
    fn compact_round_trips() {
        for secs in [0, 1, 59, 60, 90, 1500, 3600, 5400, 3661, 86400] {
            let d = Duration::from_secs(secs);
            assert_eq!(parse(&compact(d)).unwrap_or_default(), d, "{}", compact(d));
        }
        assert_eq!(compact(Duration::from_secs(5400)), "1h30m");
        assert_eq!(compact(Duration::from_secs(90)), "1m30s");
    }
}
