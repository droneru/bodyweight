//! Dates of workouts and weigh-ins as the API takes them.

use chrono::{Days, NaiveDate};

/// "Today" is counted in this time zone, see docs/spec.md.
const TIME_ZONE: chrono_tz::Tz = chrono_tz::Europe::Moscow;
/// How far past the server's today a workout or a weigh-in may be dated. The slack covers
/// users whose local date is ahead of the server's.
const MAX_DAYS_AHEAD: u64 = 2;

/// Today's date of the server.
pub fn today() -> NaiveDate {
    chrono::Utc::now().with_timezone(&TIME_ZONE).date_naive()
}

/// Parses a strict `YYYY-MM-DD` date that is at most `MAX_DAYS_AHEAD` days after `today`.
pub fn parse_date(s: &str, today: NaiveDate) -> Result<NaiveDate, &'static str> {
    const INVALID: &str = "date must be in the YYYY-MM-DD format";
    if s.len() != 10 {
        return Err(INVALID);
    }
    let date = NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| INVALID)?;
    if date > today + Days::new(MAX_DAYS_AHEAD) {
        return Err("date must be at most 2 days ahead of today");
    }
    Ok(date)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn accepts_dates_up_to_two_days_ahead() {
        let today = date(2026, 11, 13);
        assert_eq!(parse_date("2020-02-29", today), Ok(date(2020, 2, 29)));
        assert_eq!(parse_date("2026-11-13", today), Ok(today));
        assert_eq!(parse_date("2026-11-15", today), Ok(date(2026, 11, 15)));
        assert!(parse_date("2026-11-16", today).is_err());
    }

    #[test]
    fn rejects_malformed_dates() {
        let today = date(2026, 11, 13);
        for s in ["", "2026-1-5", "2026-02-30", "13.11.2026", "2026-11-13T00"] {
            assert!(parse_date(s, today).is_err(), "{s:?}");
        }
    }
}
