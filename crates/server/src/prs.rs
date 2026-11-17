//! PRs API, see docs/spec.md §2. PRs are computed on every request and never stored.

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use bodyweight_domain::{Period, Pr, PrValue, StandardExercise, pr};
use serde::Serialize;

use crate::auth::CurrentUser;
use crate::dates::today;
use crate::{ApiError, AppState};

pub fn routes() -> Router<AppState> {
    Router::new().route("/prs", get(list))
}

/// The PRs of a standard exercise; `null` for a period without a set that counts.
#[derive(Serialize)]
struct ExercisePrs {
    exercise_id: &'static str,
    days_90: Option<PrBody>,
    all_time: Option<PrBody>,
}

#[derive(Serialize)]
struct PrBody {
    /// The date of the workout, `YYYY-MM-DD`.
    date: String,
    /// The set the PR comes from. For pull-ups its reps are the PR.
    reps: u32,
    /// Kilograms such as `72.5`; `null` for pull-ups.
    weight: Option<String>,
    /// Kilograms rounded to tenths, such as `81.7`; `null` for pull-ups.
    estimated_1rm: Option<String>,
}

impl From<Pr> for PrBody {
    fn from(pr: Pr) -> Self {
        Self {
            date: pr.date.to_string(),
            reps: pr.set.reps.get(),
            weight: pr.set.weight.map(|w| w.to_string()),
            estimated_1rm: match pr.value {
                PrValue::MaxReps(_) => None,
                PrValue::Estimated1Rm(estimate) => Some(estimate.to_string()),
            },
        }
    }
}

/// The user's PRs in every standard exercise, over the last 90 days and over all time.
async fn list(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<Json<Vec<ExercisePrs>>, ApiError> {
    let workouts = state.storage.list_workouts(&user.login).await?;
    let today = today();
    let of = |exercise, period| pr(exercise, period, &workouts, today).map(PrBody::from);
    Ok(Json(
        StandardExercise::ALL
            .into_iter()
            .map(|exercise| ExercisePrs {
                exercise_id: exercise.id(),
                days_90: of(exercise, Period::Days90),
                all_time: of(exercise, Period::AllTime),
            })
            .collect(),
    ))
}
