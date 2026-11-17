use std::fmt;

use chrono::{Days, NaiveDate};

use crate::{ExerciseId, Reps, Set, StandardExercise, Weight, Workout};

/// The window a PR is computed over.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Period {
    /// The last 90 days, today included.
    Days90,
    AllTime,
}

impl Period {
    pub const ALL: [Self; 2] = [Self::Days90, Self::AllTime];

    /// Whether a workout on `date` falls into the period that ends `today`.
    pub fn contains(self, date: NaiveDate, today: NaiveDate) -> bool {
        match self {
            Self::Days90 => today - Days::new(89) <= date && date <= today,
            Self::AllTime => true,
        }
    }
}

/// A set's one-rep-max estimate by the Epley formula `w × (1 + r / 30)`; for one rep it is
/// the weight itself. Kept exact, in 1/120 of a kilogram.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Estimated1Rm(u32);

impl Estimated1Rm {
    pub fn of(weight: Weight, reps: Reps) -> Self {
        // (quarters / 4) × (30 + reps) / 30 kg.
        let thirtieths = if reps.get() == 1 { 30 } else { 30 + reps.get() };
        Self(weight.quarters() * thirtieths)
    }
}

impl fmt::Display for Estimated1Rm {
    /// Kilograms rounded to tenths, without a zero tenth: `120`, `81.7`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tenths = (self.0 + 6) / 12;
        write!(f, "{}", tenths / 10)?;
        if !tenths.is_multiple_of(10) {
            write!(f, ".{}", tenths % 10)?;
        }
        Ok(())
    }
}

/// What a PR is measured in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PrValue {
    /// Pull-ups: the reps of a set without a weight.
    MaxReps(Reps),
    /// The Big Three: the estimate of a set with a weight and at most 12 reps.
    Estimated1Rm(Estimated1Rm),
}

/// A user's best result in a standard exercise over a period, with the set and the date
/// it came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pr {
    pub value: PrValue,
    pub set: Set,
    pub date: NaiveDate,
}

/// The most reps a set may have to count towards the estimated 1RM.
const MAX_REPS_FOR_1RM: u32 = 12;

/// What the set is worth as a PR in the exercise, if it counts at all.
fn value(exercise: StandardExercise, set: Set) -> Option<PrValue> {
    match (exercise, set.weight) {
        (StandardExercise::PullUps, None) => Some(PrValue::MaxReps(set.reps)),
        (StandardExercise::PullUps, Some(_)) => None,
        (_, Some(weight)) if set.reps.get() <= MAX_REPS_FOR_1RM => {
            Some(PrValue::Estimated1Rm(Estimated1Rm::of(weight, set.reps)))
        }
        _ => None,
    }
}

/// The PR in the exercise over the period that ends `today`, or `None` if no set counts.
/// Of equal results the one with the earliest date is the PR. `workouts` are newest first,
/// the way the storage lists them.
pub fn pr(
    exercise: StandardExercise,
    period: Period,
    workouts: &[Workout],
    today: NaiveDate,
) -> Option<Pr> {
    let mut best: Option<Pr> = None;
    // Oldest first, so that among equal results on one date the first one done stays.
    for workout in workouts.iter().rev() {
        if !period.contains(workout.date, today) {
            continue;
        }
        let entries = workout
            .entries
            .iter()
            .filter(|e| e.exercise == ExerciseId::Standard(exercise));
        for set in entries.flat_map(|e| &e.sets) {
            let Some(value) = value(exercise, *set) else {
                continue;
            };
            let better =
                best.is_none_or(|b| value > b.value || (value == b.value && workout.date < b.date));
            if better {
                best = Some(Pr {
                    value,
                    set: *set,
                    date: workout.date,
                });
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ExerciseEntry;
    use StandardExercise::{BenchPress, Deadlift, PullUps, Squat};

    fn date(month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, month, day).unwrap()
    }

    fn set(reps: u32, weight: &str) -> Set {
        Set {
            reps: Reps::new(reps).unwrap(),
            weight: (!weight.is_empty()).then(|| weight.parse().unwrap()),
        }
    }

    fn workout(date: NaiveDate, exercise: StandardExercise, sets: &[Set]) -> Workout {
        let entry = ExerciseEntry {
            exercise: ExerciseId::Standard(exercise),
            sets: sets.to_vec(),
        };
        let id = "0123456789abcdef0123456789abcdef".parse().unwrap();
        Workout::new(id, date, "", vec![entry]).unwrap()
    }

    /// The PR as `value from reps×weight on date`.
    fn show(pr: Option<Pr>) -> String {
        let Some(pr) = pr else {
            return "—".into();
        };
        let value = match pr.value {
            PrValue::MaxReps(reps) => reps.get().to_string(),
            PrValue::Estimated1Rm(e) => e.to_string(),
        };
        let weight = pr.set.weight.map(|w| w.to_string()).unwrap_or_default();
        format!("{value} from {}×{weight} on {}", pr.set.reps.get(), pr.date)
    }

    fn e1rm(weight: &str, reps: u32) -> String {
        Estimated1Rm::of(weight.parse().unwrap(), Reps::new(reps).unwrap()).to_string()
    }

    const TODAY: (u32, u32) = (11, 17);

    fn all_time(exercise: StandardExercise, workouts: &[Workout]) -> String {
        show(pr(
            exercise,
            Period::AllTime,
            workouts,
            date(TODAY.0, TODAY.1),
        ))
    }

    #[test]
    fn estimated_1rm_follows_epley() {
        assert_eq!(e1rm("90", 10), "120");
        assert_eq!(e1rm("70", 5), "81.7"); // 81.666…
        assert_eq!(e1rm("60", 3), "66");
        assert_eq!(e1rm("72.5", 4), "82.2"); // 82.166…
        assert_eq!(e1rm("0.25", 2), "0.3"); // 0.2666…
    }

    #[test]
    fn estimated_1rm_of_one_rep_is_the_weight() {
        assert_eq!(e1rm("100", 1), "100");
        assert_eq!(e1rm("102.5", 1), "102.5");
        assert_eq!(e1rm("101.25", 1), "101.3");
    }

    #[test]
    fn the_big_three_pr_is_the_highest_estimated_1rm() {
        // The example of docs/spec.md §2.2.
        let workouts = [workout(
            date(10, 12),
            BenchPress,
            &[set(1, "100"), set(10, "90")],
        )];
        assert_eq!(
            all_time(BenchPress, &workouts),
            "120 from 10×90 on 2026-10-12"
        );
        assert_eq!(all_time(Squat, &workouts), "—");
    }

    #[test]
    fn sets_without_a_weight_or_with_more_than_12_reps_do_not_count() {
        let workouts = [workout(
            date(11, 1),
            Deadlift,
            &[set(13, "100"), set(20, ""), set(12, "60"), set(5, "")],
        )];
        assert_eq!(all_time(Deadlift, &workouts), "84 from 12×60 on 2026-11-01");
        let workouts = [workout(
            date(11, 1),
            Deadlift,
            &[set(13, "100"), set(5, "")],
        )];
        assert_eq!(all_time(Deadlift, &workouts), "—");
    }

    #[test]
    fn the_pull_ups_pr_is_the_most_reps_without_a_weight() {
        let workouts = [
            workout(
                date(11, 3),
                PullUps,
                &[set(8, ""), set(15, "10"), set(7, "")],
            ),
            workout(date(11, 1), PullUps, &[set(9, ""), set(6, "")]),
        ];
        assert_eq!(all_time(PullUps, &workouts), "9 from 9× on 2026-11-01");
        let workouts = [workout(date(11, 3), PullUps, &[set(15, "10")])];
        assert_eq!(all_time(PullUps, &workouts), "—");
    }

    #[test]
    fn of_equal_results_the_earliest_is_the_pr() {
        // 100 × 6 and 90 × 10 both give 120.
        let workouts = [
            workout(date(11, 5), Squat, &[set(10, "90")]),
            workout(date(11, 3), Squat, &[set(6, "100"), set(10, "90")]),
            workout(date(11, 4), Squat, &[set(10, "90")]),
        ];
        assert_eq!(all_time(Squat, &workouts), "120 from 6×100 on 2026-11-03");
    }

    #[test]
    fn the_90_days_end_today() {
        let today = date(11, 17);
        let first = date(8, 20);
        assert_eq!(today - first, chrono::TimeDelta::days(89));
        assert!(Period::Days90.contains(first, today));
        assert!(Period::Days90.contains(today, today));
        assert!(!Period::Days90.contains(date(8, 19), today));
        assert!(!Period::Days90.contains(date(11, 18), today));
        assert!(Period::AllTime.contains(date(1, 1), today));
        assert!(Period::AllTime.contains(date(11, 18), today));
    }

    #[test]
    fn a_pr_is_computed_over_its_period() {
        let workouts = [
            workout(date(11, 18), BenchPress, &[set(1, "200")]),
            workout(date(11, 1), BenchPress, &[set(5, "70")]),
            workout(date(8, 19), BenchPress, &[set(5, "90")]),
        ];
        let today = date(11, 17);
        assert_eq!(
            show(pr(BenchPress, Period::Days90, &workouts, today)),
            "81.7 from 5×70 on 2026-11-01"
        );
        assert_eq!(
            show(pr(BenchPress, Period::AllTime, &workouts, today)),
            "200 from 1×200 on 2026-11-18"
        );
        assert_eq!(
            show(pr(BenchPress, Period::Days90, &workouts[2..], today)),
            "—"
        );
    }

    #[test]
    fn custom_exercises_have_no_part_in_prs() {
        let mut other = workout(date(11, 1), Squat, &[set(5, "300")]);
        other.entries[0].exercise = ExerciseId::Custom("squat-like".into());
        assert_eq!(all_time(Squat, &[other]), "—");
    }
}
