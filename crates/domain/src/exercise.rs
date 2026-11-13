use std::fmt;
use std::str::FromStr;

/// An exercise defined in code, available to every user. Only these have PRs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StandardExercise {
    PullUps,
    BenchPress,
    Squat,
    Deadlift,
}

impl StandardExercise {
    /// In the order they are shown to the user.
    pub const ALL: [Self; 4] = [Self::PullUps, Self::BenchPress, Self::Squat, Self::Deadlift];

    /// Stable identifier, used in the API and in the database.
    pub fn id(self) -> &'static str {
        match self {
            Self::PullUps => "pull-ups",
            Self::BenchPress => "bench-press",
            Self::Squat => "squat",
            Self::Deadlift => "deadlift",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::PullUps => "Pull-ups",
            Self::BenchPress => "Bench press",
            Self::Squat => "Squat",
            Self::Deadlift => "Deadlift",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|e| e.id() == id)
    }
}

/// An exercise a user created for themselves. Private to that user, has no PRs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomExercise {
    pub id: String,
    pub name: ExerciseName,
    /// An archived exercise is not offered for new workouts but stays in the old ones.
    pub archived: bool,
}

/// The name of a custom exercise: trimmed, not empty, a single line of at most
/// `ExerciseName::MAX_CHARS` characters.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExerciseName(String);

impl ExerciseName {
    pub const MAX_CHARS: usize = 60;

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for ExerciseName {
    type Err = ExerciseNameError;

    /// Trims the surrounding whitespace before validating.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let name = s.trim();
        if name.is_empty() {
            return Err(ExerciseNameError::Empty);
        }
        if name.chars().count() > Self::MAX_CHARS {
            return Err(ExerciseNameError::TooLong);
        }
        if name.chars().any(char::is_control) {
            return Err(ExerciseNameError::Invalid);
        }
        Ok(Self(name.to_owned()))
    }
}

impl fmt::Display for ExerciseName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ExerciseNameError {
    #[error("exercise name must not be empty")]
    Empty,
    #[error("exercise name must be at most 60 characters")]
    TooLong,
    #[error("exercise name must be a single line of text")]
    Invalid,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Result<String, ExerciseNameError> {
        s.parse::<ExerciseName>().map(|n| n.as_str().to_owned())
    }

    #[test]
    fn standard_exercises_are_found_by_id() {
        for exercise in StandardExercise::ALL {
            assert_eq!(StandardExercise::from_id(exercise.id()), Some(exercise));
        }
        assert_eq!(StandardExercise::from_id("Squat"), None);
        assert_eq!(StandardExercise::from_id(""), None);
    }

    #[test]
    fn name_is_trimmed() {
        assert_eq!(parse("  Overhead press \n"), Ok("Overhead press".into()));
        assert_eq!(parse("Жим стоя"), Ok("Жим стоя".into()));
    }

    #[test]
    fn rejects_empty_names() {
        for s in ["", " ", "\t\n"] {
            assert_eq!(parse(s), Err(ExerciseNameError::Empty), "{s:?}");
        }
    }

    #[test]
    fn length_is_counted_in_characters() {
        let longest = "ж".repeat(ExerciseName::MAX_CHARS);
        assert_eq!(parse(&longest), Ok(longest.clone()));
        assert_eq!(
            parse(&format!("{longest}ж")),
            Err(ExerciseNameError::TooLong)
        );
    }

    #[test]
    fn rejects_control_characters() {
        assert_eq!(parse("Dips\nweighted"), Err(ExerciseNameError::Invalid));
    }
}
