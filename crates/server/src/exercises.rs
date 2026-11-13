//! Exercises API: the catalog of standard exercises and the user's custom exercises,
//! see docs/spec.md §1.4.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch};
use axum::{Json, Router};
use bodyweight_domain::{CustomExercise, ExerciseName, StandardExercise};
use bodyweight_storage::DeleteExercise;
use serde::{Deserialize, Serialize};

use crate::auth::CurrentUser;
use crate::{ApiError, AppState};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/exercises", get(list).post(create))
        .route("/exercises/{id}", patch(update).delete(delete))
}

#[derive(Serialize)]
struct Exercises {
    standard: Vec<StandardExerciseBody>,
    /// Ordered by name, archived ones included.
    custom: Vec<CustomExerciseBody>,
}

#[derive(Serialize)]
struct StandardExerciseBody {
    id: &'static str,
    name: &'static str,
}

#[derive(Serialize)]
struct CustomExerciseBody {
    id: String,
    name: String,
    archived: bool,
}

impl From<CustomExercise> for CustomExerciseBody {
    fn from(exercise: CustomExercise) -> Self {
        Self {
            id: exercise.id,
            name: exercise.name.to_string(),
            archived: exercise.archived,
        }
    }
}

/// The standard exercises and the user's custom exercises.
async fn list(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<Json<Exercises>, ApiError> {
    let custom = state.storage.list_custom_exercises(&user.login).await?;
    Ok(Json(Exercises {
        standard: StandardExercise::ALL
            .into_iter()
            .map(|e| StandardExerciseBody {
                id: e.id(),
                name: e.name(),
            })
            .collect(),
        custom: custom.into_iter().map(Into::into).collect(),
    }))
}

#[derive(Deserialize)]
struct CreateExercise {
    name: String,
}

/// Creates a custom exercise and returns it.
async fn create(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<CreateExercise>,
) -> Result<Response, ApiError> {
    let name = match body.name.parse::<ExerciseName>() {
        Ok(name) => name,
        Err(e) => return Ok(bad_request(e.to_string())),
    };
    let exercise = CustomExercise {
        id: hex::encode(rand::random::<[u8; 16]>()),
        name,
        archived: false,
    };
    state
        .storage
        .create_custom_exercise(&user.login, &exercise)
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(CustomExerciseBody::from(exercise)),
    )
        .into_response())
}

/// Absent fields are left as they are.
#[derive(Deserialize)]
struct UpdateExercise {
    name: Option<String>,
    archived: Option<bool>,
}

/// Renames and/or (un)archives a custom exercise.
async fn update(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<String>,
    Json(body): Json<UpdateExercise>,
) -> Result<Response, ApiError> {
    let name = match body.name.as_deref().map(str::parse::<ExerciseName>) {
        Some(Ok(name)) => Some(name),
        Some(Err(e)) => return Ok(bad_request(e.to_string())),
        None => None,
    };
    let found = state
        .storage
        .update_custom_exercise(&user.login, &id, name.as_ref(), body.archived)
        .await?;
    Ok(no_content_if(found))
}

/// Deletes a custom exercise, unless it is used in a workout.
async fn delete(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let outcome = state
        .storage
        .delete_custom_exercise(&user.login, &id)
        .await?;
    Ok(match outcome {
        DeleteExercise::Deleted => StatusCode::NO_CONTENT.into_response(),
        DeleteExercise::InUse => (
            StatusCode::CONFLICT,
            "exercise is used in a workout, archive it instead",
        )
            .into_response(),
        DeleteExercise::NotFound => StatusCode::NOT_FOUND.into_response(),
    })
}

fn bad_request(message: String) -> Response {
    (StatusCode::BAD_REQUEST, message).into_response()
}

fn no_content_if(found: bool) -> Response {
    if found {
        StatusCode::NO_CONTENT.into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}
