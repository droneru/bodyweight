use std::fmt;

use chrono::NaiveDate;

use crate::{Bodyweight, WeighIn, Weight};

/// A set's weight divided by the user's bodyweight, rounded down to tenths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BodyweightRatio(u32);

impl BodyweightRatio {
    pub fn of(weight: Weight, bodyweight: Bodyweight) -> Self {
        // (quarters / 4) / (tenths / 10), in tenths: 100 * quarters / (4 * tenths).
        Self(25 * weight.quarters() / bodyweight.tenths())
    }

    pub fn tenths(self) -> u32 {
        self.0
    }
}

impl fmt::Display for BodyweightRatio {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.0 / 10, self.0 % 10)
    }
}

/// The bodyweight to compare the sets of a workout on `date` with: from the latest weigh-in
/// on that date or before it, if there is one.
pub fn bodyweight_on(weigh_ins: &[WeighIn], date: NaiveDate) -> Option<Bodyweight> {
    weigh_ins
        .iter()
        .filter(|w| w.date <= date)
        .max_by_key(|w| w.date)
        .map(|w| w.bodyweight)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ratio(weight: &str, bodyweight: &str) -> String {
        BodyweightRatio::of(weight.parse().unwrap(), bodyweight.parse().unwrap()).to_string()
    }

    fn weigh_in(day: u32, bodyweight: &str) -> WeighIn {
        WeighIn {
            date: NaiveDate::from_ymd_opt(2026, 11, day).unwrap(),
            bodyweight: bodyweight.parse().unwrap(),
        }
    }

    #[test]
    fn ratio_is_rounded_down_to_tenths() {
        assert_eq!(ratio("100", "80"), "1.2"); // 1.25
        assert_eq!(ratio("103.75", "80"), "1.2"); // 1.297
        assert_eq!(ratio("104", "80"), "1.3");
        assert_eq!(ratio("80", "80"), "1.0");
        assert_eq!(ratio("79.75", "80"), "0.9");
        assert_eq!(ratio("5", "82.5"), "0.0");
        assert_eq!(ratio("1000", "40.1"), "24.9");
    }

    #[test]
    fn bodyweight_comes_from_the_latest_weigh_in_up_to_the_date() {
        let weigh_ins = [weigh_in(10, "81"), weigh_in(3, "80"), weigh_in(7, "82.5")];
        let on = |day| {
            let date = NaiveDate::from_ymd_opt(2026, 11, day).unwrap();
            bodyweight_on(&weigh_ins, date).map(|b| b.to_string())
        };
        assert_eq!(on(2), None);
        assert_eq!(on(3).as_deref(), Some("80.0"));
        assert_eq!(on(6).as_deref(), Some("80.0"));
        assert_eq!(on(7).as_deref(), Some("82.5"));
        assert_eq!(on(30).as_deref(), Some("81.0"));
        assert_eq!(bodyweight_on(&[], NaiveDate::MAX), None);
    }
}
