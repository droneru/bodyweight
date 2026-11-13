//! Weigh-ins API, see docs/spec.md §1.5.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, put};
use axum::{Json, Router};
use bodyweight_domain::{Bodyweight, WeighIn};
use bodyweight_storage::StorageError;
use chrono::{Days, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::auth::CurrentUser;
use crate::{ApiError, AppState};

/// "Today" is counted in this time zone, see docs/spec.md.
const TIME_ZONE: chrono_tz::Tz = chrono_tz::Europe::Moscow;
/// How far past the server's today a weigh-in may be dated. The slack covers users whose
/// local date is ahead of the server's.
const MAX_DAYS_AHEAD: u64 = 2;

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

    let today = chrono::Utc::now().with_timezone(&TIME_ZONE).date_naive();
    let date = match parse_date(&date, today) {
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

/// Parses a strict `YYYY-MM-DD` date that is at most `MAX_DAYS_AHEAD` days after `today`.
fn parse_date(s: &str, today: NaiveDate) -> Result<NaiveDate, &'static str> {
    const INVALID: &str = "date must be in the YYYY-MM-DD format";
    if s.len() != 10 {
        return Err(INVALID);
    }
    let date = NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| INVALID)?;
    if date > today + Days::new(MAX_DAYS_AHEAD) {
        return Err("date must be at most 2 days ahead of today");
    }
    Ok(date)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn accepts_dates_up_to_two_days_ahead() {
        let today = date(2026, 11, 13);
        assert_eq!(parse_date("2020-02-29", today), Ok(date(2020, 2, 29)));
        assert_eq!(parse_date("2026-11-13", today), Ok(today));
        assert_eq!(parse_date("2026-11-15", today), Ok(date(2026, 11, 15)));
        assert!(parse_date("2026-11-16", today).is_err());
    }

    #[test]
    fn rejects_malformed_dates() {
        let today = date(2026, 11, 13);
        for s in ["", "2026-1-5", "2026-02-30", "13.11.2026", "2026-11-13T00"] {
            assert!(parse_date(s, today).is_err(), "{s:?}");
        }
    }
}
