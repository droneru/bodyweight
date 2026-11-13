//! Domain model of Bodyweight. Pure logic, no IO.

mod exercise;
mod user;
mod weigh_in;

pub use exercise::{CustomExercise, ExerciseName, ExerciseNameError, StandardExercise};
pub use user::User;
pub use weigh_in::{Bodyweight, BodyweightError, WeighIn};
