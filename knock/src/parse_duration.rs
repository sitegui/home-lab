use anyhow::bail;
use chrono::TimeDelta;

pub fn parse_duration(s: &str) -> anyhow::Result<TimeDelta> {
    let s = s.trim();

    if let Some(amount) = s.strip_suffix("ms").and_then(|v| v.parse().ok()) {
        Ok(TimeDelta::milliseconds(amount))
    } else if let Some(amount) = s.strip_suffix('s').and_then(|v| v.parse().ok()) {
        Ok(TimeDelta::seconds(amount))
    } else if let Some(amount) = s.strip_suffix('m').and_then(|v| v.parse().ok()) {
        Ok(TimeDelta::minutes(amount))
    } else if let Some(amount) = s.strip_suffix('h').and_then(|v| v.parse().ok()) {
        Ok(TimeDelta::hours(amount))
    } else if let Some(amount) = s.strip_suffix('d').and_then(|v| v.parse().ok()) {
        Ok(TimeDelta::days(amount))
    } else if let Some(amount) = s.strip_suffix('w').and_then(|v| v.parse::<i64>().ok()) {
        Ok(TimeDelta::days(7 * amount))
    } else if let Some(amount) = s.strip_suffix('M').and_then(|v| v.parse::<i64>().ok()) {
        Ok(TimeDelta::days(30 * amount))
    } else if let Some(amount) = s.strip_suffix('y').and_then(|v| v.parse::<i64>().ok()) {
        Ok(TimeDelta::days(365 * amount))
    } else {
        bail!("failed to parse {:?} as a duration", s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration() {
        assert_eq!(
            parse_duration("100ms").unwrap(),
            TimeDelta::milliseconds(100)
        );
        assert_eq!(parse_duration("5s").unwrap(), TimeDelta::seconds(5));
        assert_eq!(parse_duration("10m").unwrap(), TimeDelta::minutes(10));
        assert_eq!(parse_duration("2h").unwrap(), TimeDelta::hours(2));
        assert_eq!(parse_duration("3d").unwrap(), TimeDelta::days(3));
        assert_eq!(parse_duration("4w").unwrap(), TimeDelta::days(28));
        assert_eq!(parse_duration("2M").unwrap(), TimeDelta::days(60));
        assert_eq!(parse_duration("1y").unwrap(), TimeDelta::days(365));
        assert!(parse_duration("invalid").is_err());
    }
}
