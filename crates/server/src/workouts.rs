//! Workouts API, see docs/spec.md §1.1–1.3.

use std::collections::HashSet;
use std::time::SystemTime;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use bodyweight_domain::{ExerciseEntry, ExerciseId, Reps, Set, Weight, Workout, WorkoutId};
use bodyweight_storage::StorageError;
use serde::{Deserialize, Serialize};

use crate::auth::CurrentUser;
use crate::dates::{parse_date, today};
use crate::{ApiError, AppState};

pub fn routes() -> Router<AppState> {
    Router::new().route("/workouts", get(list)).route(
        "/workouts/{id}",
        get(get_workout).put(put_workout).delete(delete_workout),
    )
}

/// A workout in the list: what the Workouts screen shows in a row.
#[derive(Serialize)]
struct WorkoutSummary {
    id: String,
    /// `YYYY-MM-DD`.
    date: String,
    note: String,
    /// The exercises of the workout in the order they first appear, without repeats.
    exercise_ids: Vec<String>,
}

impl From<Workout> for WorkoutSummary {
    fn from(workout: Workout) -> Self {
        let mut seen = HashSet::new();
        let exercise_ids = workout
            .entries
            .iter()
            .map(|e| e.exercise.as_str())
            .filter(|id| seen.insert(*id))
            .map(str::to_owned)
            .collect();
        Self {
            id: workout.id.to_string(),
            date: workout.date.to_string(),
            note: workout.note,
            exercise_ids,
        }
    }
}

/// A whole workout, both as returned and as accepted by the API.
#[derive(Serialize, Deserialize)]
struct WorkoutBody {
    /// `YYYY-MM-DD`.
    date: String,
    /// Empty if there is no note.
    note: String,
    entries: Vec<EntryBody>,
}

#[derive(Serialize, Deserialize)]
struct EntryBody {
    exercise_id: String,
    sets: Vec<SetBody>,
}

#[derive(Serialize, Deserialize)]
struct SetBody {
    reps: u32,
    /// Kilograms such as `72.5`, a string so that no floats are involved.
    /// `null` for a set done with bodyweight only.
    weight: Option<String>,
}

impl From<Workout> for WorkoutBody {
    fn from(workout: Workout) -> Self {
        let entry = |entry: ExerciseEntry| EntryBody {
            exercise_id: entry.exercise.as_str().to_owned(),
            sets: entry
                .sets
                .into_iter()
                .map(|set| SetBody {
                    reps: set.reps.get(),
                    weight: set.weight.map(|w| w.to_string()),
                })
                .collect(),
        };
        Self {
            date: workout.date.to_string(),
            note: workout.note,
            entries: workout.entries.into_iter().map(entry).collect(),
        }
    }
}

impl WorkoutBody {
    /// Validates everything that does not need the database. The error is a message
    /// for the user.
    fn into_workout(self, id: WorkoutId) -> Result<Workout, String> {
        let date = parse_date(&self.date, today())?;
        let entries = self
            .entries
            .into_iter()
            .map(|entry| {
                let sets = entry
                    .sets
                    .into_iter()
                    .map(|set| {
                        let weight = set
                            .weight
                            .map(|w| w.trim().parse::<Weight>())
                            .transpose()
                            .map_err(|e| e.to_string())?;
                        let reps = Reps::new(set.reps).map_err(|e| e.to_string())?;
                        Ok(Set { reps, weight })
                    })
                    .collect::<Result<_, String>>()?;
                Ok(ExerciseEntry {
                    exercise: ExerciseId::parse(&entry.exercise_id),
                    sets,
                })
            })
            .collect::<Result<_, String>>()?;
        Workout::new(id, date, &self.note, entries).map_err(|e| e.to_string())
    }
}

/// The user's workouts, newest first.
async fn list(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<Json<Vec<WorkoutSummary>>, ApiError> {
    let workouts = state.storage.list_workouts(&user.login).await?;
    Ok(Json(workouts.into_iter().map(Into::into).collect()))
}

async fn get_workout(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    // An id of another form cannot have been saved.
    let Ok(id) = id.parse::<WorkoutId>() else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    Ok(match state.storage.get_workout(&user.login, &id).await? {
        Some(workout) => Json(WorkoutBody::from(workout)).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    })
}

/// Creates the workout with this id or replaces it as a whole. The id is picked by the
/// client, so that repeating the request is harmless.
async fn put_workout(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<String>,
    Json(body): Json<WorkoutBody>,
) -> Result<Response, ApiError> {
    let bad_request = |message: String| Ok((StatusCode::BAD_REQUEST, message).into_response());

    let id = match id.parse::<WorkoutId>() {
        Ok(id) => id,
        Err(e) => return bad_request(e.to_string()),
    };
    let workout = match body.into_workout(id) {
        Ok(workout) => workout,
        Err(message) => return bad_request(message),
    };

    let custom_ids: Vec<&str> = workout
        .entries
        .iter()
        .filter_map(|entry| match &entry.exercise {
            ExerciseId::Standard(_) => None,
            ExerciseId::Custom(id) => Some(id.as_str()),
        })
        .collect();
    if !custom_ids.is_empty() {
        let known = state.storage.list_custom_exercises(&user.login).await?;
        if let Some(unknown) = custom_ids
            .into_iter()
            .find(|id| !known.iter().any(|e| e.id == *id))
        {
            return bad_request(format!("unknown exercise {unknown:?}"));
        }
    }

    let saved = state
        .storage
        .put_workout(&user.login, &workout, SystemTime::now())
        .await;
    match saved {
        Ok(()) => Ok(StatusCode::NO_CONTENT.into_response()),
        Err(StorageError::UnsupportedDate(_)) => bad_request("date is too far in the past".into()),
        Err(e) => Err(e.into()),
    }
}

async fn delete_workout(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let Ok(id) = id.parse::<WorkoutId>() else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    let found = state.storage.delete_workout(&user.login, &id).await?;
    Ok(if found {
        StatusCode::NO_CONTENT.into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    })
}
