//! Weigh-ins API, see docs/spec.md §1.5.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, put};
use axum::{Json, Router};
use bodyweight_domain::{Bodyweight, WeighIn};
use bodyweight_storage::StorageError;
use serde::{Deserialize, Serialize};

use crate::auth::CurrentUser;
use crate::dates::{parse_date, today};
use crate::{ApiError, AppState};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/weigh-ins", get(list))
        .route("/weigh-ins/{date}", put(put_weigh_in))
}

#[derive(Serialize)]
struct WeighInBody {
    /// `YYYY-MM-DD`.
    date: String,
    /// Kilograms with one decimal, e.g. `82.5`. A string, so that no floats are involved.
    bodyweight: String,
}

/// The user's weigh-ins, newest first.
async fn list(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<Json<Vec<WeighInBody>>, ApiError> {
    let weigh_ins = state.storage.list_weigh_ins(&user.login).await?;
    Ok(Json(
        weigh_ins
            .into_iter()
            .map(|w| WeighInBody {
                date: w.date.to_string(),
                bodyweight: w.bodyweight.to_string(),
            })
            .collect(),
    ))
}

#[derive(Deserialize)]
struct PutWeighIn {
    bodyweight: String,
}

/// Saves the weigh-in on the date, replacing an existing one.
async fn put_weigh_in(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(date): Path<String>,
    Json(body): Json<PutWeighIn>,
) -> Result<Response, ApiError> {
    let bad_request = |message: String| Ok((StatusCode::BAD_REQUEST, message).into_response());

    let date = match parse_date(&date, today()) {
        Ok(date) => date,
        Err(message) => return bad_request(message.into()),
    };
    let bodyweight = match body.bodyweight.trim().parse::<Bodyweight>() {
        Ok(bodyweight) => bodyweight,
        Err(e) => return bad_request(e.to_string()),
    };

    let weigh_in = WeighIn { date, bodyweight };
    match state.storage.put_weigh_in(&user.login, weigh_in).await {
        Ok(()) => Ok(StatusCode::NO_CONTENT.into_response()),
        Err(StorageError::UnsupportedDate(_)) => bad_request("date is too far in the past".into()),
        Err(e) => Err(e.into()),
    }
}
