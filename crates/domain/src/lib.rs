//! Domain model of Bodyweight. Pure logic, no IO.

mod bodyweight_ratio;
mod exercise;
mod pr;
mod user;
mod weigh_in;
mod workout;

pub use bodyweight_ratio::{BodyweightRatio, bodyweight_on};
pub use exercise::{CustomExercise, ExerciseName, ExerciseNameError, StandardExercise};
pub use pr::{Estimated1Rm, Period, Pr, PrValue, pr};
pub use user::User;
pub use weigh_in::{Bodyweight, BodyweightError, WeighIn};
pub use workout::{
    ExerciseEntry, ExerciseId, Reps, RepsError, Set, Weight, WeightError, Workout, WorkoutError,
    WorkoutId, WorkoutIdError,
};
