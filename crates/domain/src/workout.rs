use std::fmt;
use std::str::FromStr;

use chrono::NaiveDate;

use crate::StandardExercise;

/// A user's training session on a date. A user may have several workouts on the same date.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workout {
    pub id: WorkoutId,
    pub date: NaiveDate,
    /// Free text, empty if there is no note.
    pub note: String,
    pub entries: Vec<ExerciseEntry>,
}

impl Workout {
    pub const MAX_NOTE_CHARS: usize = 2000;
    pub const MAX_ENTRIES: usize = 50;

    /// Trims the note and checks the size limits. Whether the date is acceptable depends on
    /// today's date and is up to the caller.
    pub fn new(
        id: WorkoutId,
        date: NaiveDate,
        note: &str,
        entries: Vec<ExerciseEntry>,
    ) -> Result<Self, WorkoutError> {
        let note = note.trim();
        if note.chars().count() > Self::MAX_NOTE_CHARS {
            return Err(WorkoutError::NoteTooLong);
        }
        if entries.len() > Self::MAX_ENTRIES {
            return Err(WorkoutError::TooManyEntries);
        }
        if entries
            .iter()
            .any(|e| e.sets.len() > ExerciseEntry::MAX_SETS)
        {
            return Err(WorkoutError::TooManySets);
        }
        Ok(Self {
            id,
            date,
            note: note.to_owned(),
            entries,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum WorkoutError {
    #[error("note must be at most 2000 characters")]
    NoteTooLong,
    #[error("a workout can have at most 50 exercises")]
    TooManyEntries,
    #[error("an exercise can have at most 50 sets")]
    TooManySets,
}

/// Identifies a workout among the workouts of its user: 32 lowercase hex digits,
/// picked at random by whoever creates the workout.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorkoutId(String);

impl WorkoutId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for WorkoutId {
    type Err = WorkoutIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let is_lower_hex = |b: u8| b.is_ascii_digit() || (b'a'..=b'f').contains(&b);
        if s.len() == 32 && s.bytes().all(is_lower_hex) {
            Ok(Self(s.to_owned()))
        } else {
            Err(WorkoutIdError)
        }
    }
}

impl fmt::Display for WorkoutId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("workout id must be 32 lowercase hex digits")]
pub struct WorkoutIdError;

/// One exercise performed within a workout, together with its sets, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExerciseEntry {
    pub exercise: ExerciseId,
    pub sets: Vec<Set>,
}

impl ExerciseEntry {
    pub const MAX_SETS: usize = 50;
}

/// Refers to an exercise from the catalog: a standard one or a custom exercise of the user.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExerciseId {
    Standard(StandardExercise),
    /// The id of a `CustomExercise`.
    Custom(String),
}

impl ExerciseId {
    /// An id that is not of a standard exercise is taken as the id of a custom one.
    pub fn parse(id: &str) -> Self {
        match StandardExercise::from_id(id) {
            Some(exercise) => Self::Standard(exercise),
            None => Self::Custom(id.to_owned()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Standard(exercise) => exercise.id(),
            Self::Custom(id) => id,
        }
    }
}

/// A single performance of an exercise. Without a weight it was done with bodyweight only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Set {
    pub reps: Reps,
    pub weight: Option<Weight>,
}

/// How many times the movement was performed in one set: an integer from 1 to `Reps::MAX`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Reps(u32);

impl Reps {
    pub const MAX: u32 = 1000;

    pub fn new(reps: u32) -> Result<Self, RepsError> {
        if (1..=Self::MAX).contains(&reps) {
            Ok(Self(reps))
        } else {
            Err(RepsError)
        }
    }

    pub fn get(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("reps must be between 1 and 1000")]
pub struct RepsError;

/// The external load of a set in kilograms, in steps of 0.25, stored as quarters of a kilogram.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Weight(u32);

impl Weight {
    /// Upper bound that catches typos such as a missing decimal point.
    pub const MAX_QUARTERS: u32 = 4_000;

    pub fn from_quarters(quarters: u32) -> Result<Self, WeightError> {
        if quarters == 0 || quarters > Self::MAX_QUARTERS {
            return Err(WeightError::OutOfRange);
        }
        Ok(Self(quarters))
    }

    pub fn quarters(self) -> u32 {
        self.0
    }
}

impl FromStr for Weight {
    type Err = WeightError;

    /// Parses kilograms like `70`, `72.5` or `71.25`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (int_part, frac_part) = s.split_once('.').unwrap_or((s, ""));
        let is_digits = |p: &str| p.bytes().all(|b| b.is_ascii_digit());
        if int_part.is_empty() || !is_digits(int_part) || !is_digits(frac_part) {
            return Err(WeightError::Invalid);
        }
        if s.ends_with('.') {
            return Err(WeightError::Invalid);
        }

        let quarters = match frac_part.trim_end_matches('0') {
            "" => 0,
            "25" => 1,
            "5" => 2,
            "75" => 3,
            _ => return Err(WeightError::Step),
        };
        let kg: u32 = int_part.parse().map_err(|_| WeightError::OutOfRange)?;
        let total = kg
            .checked_mul(4)
            .and_then(|v| v.checked_add(quarters))
            .ok_or(WeightError::OutOfRange)?;
        Self::from_quarters(total)
    }
}

impl fmt::Display for Weight {
    /// The shortest form: `70`, `72.5`, `71.25`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let frac = ["", ".25", ".5", ".75"][(self.0 % 4) as usize];
        write!(f, "{}{frac}", self.0 / 4)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum WeightError {
    #[error("weight must be a number of kilograms, e.g. 72.5")]
    Invalid,
    #[error("weight step is 0.25 kg")]
    Step,
    #[error("weight must be greater than 0 and at most 1000 kg")]
    OutOfRange,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn weight(s: &str) -> Result<u32, WeightError> {
        s.parse::<Weight>().map(Weight::quarters)
    }

    fn id() -> WorkoutId {
        "0123456789abcdef0123456789abcdef".parse().unwrap()
    }

    fn date() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 13).unwrap()
    }

    fn entry(sets: usize) -> ExerciseEntry {
        let set = Set {
            reps: Reps::new(5).unwrap(),
            weight: None,
        };
        ExerciseEntry {
            exercise: ExerciseId::Standard(StandardExercise::Squat),
            sets: vec![set; sets],
        }
    }

    #[test]
    fn weight_parses_quarter_steps() {
        assert_eq!(weight("70"), Ok(280));
        assert_eq!(weight("72.5"), Ok(290));
        assert_eq!(weight("72.50"), Ok(290));
        assert_eq!(weight("71.25"), Ok(285));
        assert_eq!(weight("0.75"), Ok(3));
        assert_eq!(weight("1000"), Ok(4000));
    }

    #[test]
    fn weight_rejects_other_steps() {
        for s in ["70.1", "70.2", "70.125", "70.05"] {
            assert_eq!(weight(s), Err(WeightError::Step), "{s:?}");
        }
    }

    #[test]
    fn weight_rejects_malformed() {
        for s in ["", ".5", "70.", "72,5", "-1", "1e2", " 70", "70.5.5"] {
            assert_eq!(weight(s), Err(WeightError::Invalid), "{s:?}");
        }
    }

    #[test]
    fn weight_rejects_out_of_range() {
        assert_eq!(weight("0"), Err(WeightError::OutOfRange));
        assert_eq!(weight("0.00"), Err(WeightError::OutOfRange));
        assert_eq!(weight("1000.25"), Err(WeightError::OutOfRange));
        assert_eq!(weight("99999999999"), Err(WeightError::OutOfRange));
    }

    #[test]
    fn weight_displays_the_shortest_form() {
        for s in ["70", "72.5", "71.25", "0.75"] {
            assert_eq!(s.parse::<Weight>().unwrap().to_string(), s);
        }
    }

    #[test]
    fn reps_are_at_least_one() {
        assert_eq!(Reps::new(1).map(Reps::get), Ok(1));
        assert_eq!(Reps::new(1000).map(Reps::get), Ok(1000));
        assert_eq!(Reps::new(0), Err(RepsError));
        assert_eq!(Reps::new(1001), Err(RepsError));
    }

    #[test]
    fn workout_id_is_32_lowercase_hex_digits() {
        assert!(
            "0123456789abcdef0123456789abcdef"
                .parse::<WorkoutId>()
                .is_ok()
        );
        for s in [
            "",
            "0123456789abcdef0123456789abcde",
            "0123456789abcdef0123456789abcdef0",
            "0123456789ABCDEF0123456789ABCDEF",
            "0123456789abcdef0123456789abcdeg",
        ] {
            assert_eq!(s.parse::<WorkoutId>(), Err(WorkoutIdError), "{s:?}");
        }
    }

    #[test]
    fn exercise_id_is_standard_or_custom() {
        assert_eq!(
            ExerciseId::parse("squat"),
            ExerciseId::Standard(StandardExercise::Squat)
        );
        let custom = ExerciseId::parse("abc");
        assert_eq!(custom, ExerciseId::Custom("abc".into()));
        assert_eq!(custom.as_str(), "abc");
    }

    #[test]
    fn workout_trims_the_note() {
        let workout = Workout::new(id(), date(), "  legs\n", vec![entry(3)]).unwrap();
        assert_eq!(workout.note, "legs");
        assert_eq!(workout.entries, vec![entry(3)]);
    }

    #[test]
    fn workout_size_is_limited() {
        let note = "ж".repeat(Workout::MAX_NOTE_CHARS);
        assert!(Workout::new(id(), date(), &note, vec![]).is_ok());
        assert_eq!(
            Workout::new(id(), date(), &format!("{note}ж"), vec![]),
            Err(WorkoutError::NoteTooLong)
        );

        let entries = vec![entry(ExerciseEntry::MAX_SETS); Workout::MAX_ENTRIES];
        assert!(Workout::new(id(), date(), "", entries).is_ok());
        assert_eq!(
            Workout::new(id(), date(), "", vec![entry(0); Workout::MAX_ENTRIES + 1]),
            Err(WorkoutError::TooManyEntries)
        );
        assert_eq!(
            Workout::new(id(), date(), "", vec![entry(ExerciseEntry::MAX_SETS + 1)]),
            Err(WorkoutError::TooManySets)
        );
    }
}
