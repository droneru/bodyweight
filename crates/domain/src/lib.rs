//! Domain model of Bodyweight. Pure logic, no IO.

mod user;
mod weigh_in;

pub use user::User;
pub use weigh_in::{Bodyweight, BodyweightError, WeighIn};
