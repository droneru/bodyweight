use std::fmt;
use std::str::FromStr;

use chrono::NaiveDate;

/// The user's bodyweight on a date. There is at most one weigh-in per date.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeighIn {
    pub date: NaiveDate,
    pub bodyweight: Bodyweight,
}

/// Bodyweight in kilograms with 0.1 kg precision, stored as tenths of a kilogram.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Bodyweight(u32);

impl Bodyweight {
    /// Upper bound that catches typos such as a missing decimal point.
    pub const MAX_TENTHS: u32 = 5_000;

    pub fn from_tenths(tenths: u32) -> Result<Self, BodyweightError> {
        if tenths == 0 || tenths > Self::MAX_TENTHS {
            return Err(BodyweightError::OutOfRange);
        }
        Ok(Self(tenths))
    }

    pub fn tenths(self) -> u32 {
        self.0
    }
}

impl FromStr for Bodyweight {
    type Err = BodyweightError;

    /// Parses kilograms like `82`, `82.5` or `82.50`. At most one significant decimal digit.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (int_part, frac_part) = s.split_once('.').unwrap_or((s, ""));
        let is_digits = |p: &str| p.bytes().all(|b| b.is_ascii_digit());
        if int_part.is_empty() || !is_digits(int_part) || !is_digits(frac_part) {
            return Err(BodyweightError::Invalid);
        }
        if s.ends_with('.') {
            return Err(BodyweightError::Invalid);
        }

        let frac = frac_part.trim_end_matches('0');
        if frac.len() > 1 {
            return Err(BodyweightError::TooPrecise);
        }
        let kg: u32 = int_part.parse().map_err(|_| BodyweightError::OutOfRange)?;
        let tenths = frac.parse::<u32>().unwrap_or(0);
        let total = kg
            .checked_mul(10)
            .and_then(|v| v.checked_add(tenths))
            .ok_or(BodyweightError::OutOfRange)?;
        Self::from_tenths(total)
    }
}

impl fmt::Display for Bodyweight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.0 / 10, self.0 % 10)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum BodyweightError {
    #[error("bodyweight must be a number of kilograms, e.g. 82.5")]
    Invalid,
    #[error("bodyweight precision is 0.1 kg")]
    TooPrecise,
    #[error("bodyweight must be greater than 0 and at most 500 kg")]
    OutOfRange,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Result<u32, BodyweightError> {
        s.parse::<Bodyweight>().map(Bodyweight::tenths)
    }

    #[test]
    fn parses_whole_and_tenths() {
        assert_eq!(parse("82"), Ok(820));
        assert_eq!(parse("82.5"), Ok(825));
        assert_eq!(parse("82.50"), Ok(825));
        assert_eq!(parse("0.1"), Ok(1));
        assert_eq!(parse("500"), Ok(5000));
    }

    #[test]
    fn rejects_more_than_one_decimal() {
        assert_eq!(parse("82.25"), Err(BodyweightError::TooPrecise));
    }

    #[test]
    fn rejects_malformed() {
        for s in ["", ".5", "82.", "82,5", "-1", "1e2", " 82", "82.5.1"] {
            assert_eq!(parse(s), Err(BodyweightError::Invalid), "{s:?}");
        }
    }

    #[test]
    fn rejects_out_of_range() {
        assert_eq!(parse("0"), Err(BodyweightError::OutOfRange));
        assert_eq!(parse("0.0"), Err(BodyweightError::OutOfRange));
        assert_eq!(parse("500.1"), Err(BodyweightError::OutOfRange));
        assert_eq!(parse("99999999999"), Err(BodyweightError::OutOfRange));
    }

    #[test]
    fn displays_one_decimal() {
        assert_eq!(Bodyweight::from_tenths(820).unwrap().to_string(), "82.0");
        assert_eq!(Bodyweight::from_tenths(825).unwrap().to_string(), "82.5");
        assert_eq!(Bodyweight::from_tenths(5).unwrap().to_string(), "0.5");
    }
}
