//! Sign-in, sign-out and the current user, see docs/spec.md §3.
//!
//! The browser holds a random session token in an httpOnly cookie; YDB stores its SHA-256,
//! so a leaked `sessions` table cannot be used to sign in.

use std::time::{Duration, SystemTime};

use axum::extract::{FromRequestParts, Query, State};
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use axum_extra::extract::CookieJar;
use axum_extra::extract::cookie::{Cookie, SameSite};
use bodyweight_domain::User;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::yandex::avatar_url;
use crate::{ApiError, AppState};

const SESSION_COOKIE: &str = "bodyweight_session";
const OAUTH_STATE_COOKIE: &str = "bodyweight_oauth_state";
/// The OAuth state cookie is only needed by the callback.
const OAUTH_STATE_PATH: &str = "/api/auth/yandex";

const DAY: Duration = Duration::from_secs(24 * 60 * 60);
/// Sliding session lifetime.
const SESSION_TTL: Duration = Duration::from_secs(30 * 24 * 60 * 60);
/// The expiry is moved at most this often, so most requests do not write to YDB.
const SESSION_REFRESH_AFTER: Duration = DAY;
/// How long the user has to complete sign-in at Yandex.
const OAUTH_STATE_TTL: Duration = Duration::from_secs(10 * 60);

/// Where the SPA shows that sign-in did not work out.
const SIGN_IN_FAILED: &str = "/?sign_in=failed";
const MAX_LOGIN_LEN: usize = 64;

pub fn routes(dev_login_enabled: bool) -> Router<AppState> {
    let router = Router::new()
        .route("/me", get(me))
        .route("/auth/yandex/login", get(yandex_login))
        .route("/auth/yandex/callback", get(yandex_callback))
        .route("/auth/logout", post(logout));
    if dev_login_enabled {
        router.route("/auth/dev-login", post(dev_login))
    } else {
        router
    }
}

#[derive(Serialize)]
struct Me {
    login: String,
    display_name: String,
    avatar_url: Option<String>,
}

impl From<User> for Me {
    fn from(user: User) -> Self {
        Me {
            avatar_url: user.avatar_id.as_deref().map(avatar_url),
            login: user.login,
            display_name: user.display_name,
        }
    }
}

/// The signed-in user of a request to the API; the request is rejected with 401 if there is none.
///
/// Does not extend the session: the SPA calls `GET /api/me` on every load, and that does.
pub struct CurrentUser(pub User);

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Response> {
        let jar = CookieJar::from_headers(&parts.headers);
        let Some(cookie) = jar.get(SESSION_COOKIE) else {
            return Err(StatusCode::UNAUTHORIZED.into_response());
        };
        let session = state
            .storage
            .find_session(&session_id(cookie.value()), SystemTime::now())
            .await
            .map_err(|e| ApiError::from(e).into_response())?;
        match session {
            Some(session) => Ok(Self(session.user)),
            None => Err(StatusCode::UNAUTHORIZED.into_response()),
        }
    }
}

/// The signed-in user, or 401.
async fn me(State(state): State<AppState>, jar: CookieJar) -> Result<Response, ApiError> {
    let Some(token) = jar.get(SESSION_COOKIE).map(|c| c.value().to_owned()) else {
        return Ok(StatusCode::UNAUTHORIZED.into_response());
    };
    let id = session_id(&token);
    let now = SystemTime::now();
    let Some(session) = state.storage.find_session(&id, now).await? else {
        let jar = jar.remove(removal(SESSION_COOKIE, "/"));
        return Ok((jar, StatusCode::UNAUTHORIZED).into_response());
    };

    let refresh_at = session.expires_at - SESSION_TTL + SESSION_REFRESH_AFTER;
    let jar = if now >= refresh_at {
        state.storage.extend_session(&id, now + SESSION_TTL).await?;
        jar.add(session_cookie(token, state.secure_cookies))
    } else {
        jar
    };
    Ok((jar, Json(Me::from(session.user))).into_response())
}

/// Sends the browser to Yandex to sign in.
async fn yandex_login(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(yandex) = &state.yandex else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "Yandex sign-in is not configured",
        )
            .into_response();
    };
    let oauth_state = random_token();
    let url = yandex.authorize_url(&oauth_state);
    let state_cookie = cookie(
        OAUTH_STATE_COOKIE,
        oauth_state,
        OAUTH_STATE_PATH,
        OAUTH_STATE_TTL,
        state.secure_cookies,
    );
    (jar.add(state_cookie), Redirect::to(&url)).into_response()
}

#[derive(Deserialize)]
struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

/// Yandex sends the browser back here with a confirmation code.
async fn yandex_callback(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<CallbackQuery>,
) -> Result<Response, ApiError> {
    let expected_state = jar.get(OAUTH_STATE_COOKIE).map(|c| c.value().to_owned());
    let jar = jar.remove(removal(OAUTH_STATE_COOKIE, OAUTH_STATE_PATH));
    let failed = |jar: CookieJar| Ok((jar, Redirect::to(SIGN_IN_FAILED)).into_response());

    let Some(yandex) = &state.yandex else {
        return failed(jar);
    };
    if let Some(error) = &query.error {
        tracing::info!("Yandex sign-in was not completed: {error}");
        return failed(jar);
    }
    let (Some(code), Some(got_state), Some(expected_state)) =
        (&query.code, &query.state, &expected_state)
    else {
        tracing::warn!("Yandex callback without code, state or the state cookie");
        return failed(jar);
    };
    if got_state != expected_state {
        tracing::warn!("Yandex callback with a wrong state");
        return failed(jar);
    }

    let profile = match yandex.fetch_profile(code).await {
        Ok(profile) => profile,
        Err(e) => {
            tracing::error!("cannot get the Yandex profile: {e:#}");
            return failed(jar);
        }
    };
    let jar = sign_in(&state, jar, &profile.user, Some(&profile.uid)).await?;
    tracing::info!("{} signed in", profile.user.login);
    Ok((jar, Redirect::to("/")).into_response())
}

#[derive(Deserialize)]
struct DevLogin {
    login: String,
}

/// Signs in as any login without OAuth. Only routed when `BODYWEIGHT_DEV_LOGIN=1`.
async fn dev_login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<DevLogin>,
) -> Result<Response, ApiError> {
    let Some(login) = normalize_dev_login(&body.login) else {
        return Ok((StatusCode::BAD_REQUEST, "invalid login").into_response());
    };
    let user = User {
        display_name: login.clone(),
        login,
        avatar_id: None,
    };
    let jar = sign_in(&state, jar, &user, None).await?;
    Ok((jar, Json(Me::from(user))).into_response())
}

/// Deletes the session on the server and the cookie in the browser.
async fn logout(State(state): State<AppState>, jar: CookieJar) -> Result<Response, ApiError> {
    if let Some(cookie) = jar.get(SESSION_COOKIE) {
        state
            .storage
            .delete_session(&session_id(cookie.value()))
            .await?;
    }
    let jar = jar.remove(removal(SESSION_COOKIE, "/"));
    Ok((jar, StatusCode::NO_CONTENT).into_response())
}

/// Saves the user's profile and starts a session for them.
async fn sign_in(
    state: &AppState,
    jar: CookieJar,
    user: &User,
    yandex_uid: Option<&str>,
) -> Result<CookieJar, ApiError> {
    state.storage.upsert_user(user, yandex_uid).await?;
    let token = random_token();
    state
        .storage
        .create_session(
            &session_id(&token),
            &user.login,
            SystemTime::now() + SESSION_TTL,
        )
        .await?;
    Ok(jar.add(session_cookie(token, state.secure_cookies)))
}

/// Lowercased login if it looks like a Yandex login (or an email-like one).
fn normalize_dev_login(login: &str) -> Option<String> {
    let login = login.trim().to_ascii_lowercase();
    let valid = !login.is_empty()
        && login.len() <= MAX_LOGIN_LEN
        && login
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '@'));
    valid.then_some(login)
}

/// 256 random bits in hex.
fn random_token() -> String {
    hex::encode(rand::random::<[u8; 32]>())
}

/// The key of a session in YDB: SHA-256 of the cookie value.
fn session_id(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

fn cookie(
    name: &'static str,
    value: String,
    path: &'static str,
    ttl: Duration,
    secure: bool,
) -> Cookie<'static> {
    Cookie::build((name, value))
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Lax)
        .path(path)
        .max_age(time::Duration::seconds(ttl.as_secs() as i64))
        .build()
}

fn session_cookie(token: String, secure: bool) -> Cookie<'static> {
    cookie(SESSION_COOKIE, token, "/", SESSION_TTL, secure)
}

/// A cookie to pass to `CookieJar::remove`; the path must match the one it was set with.
fn removal(name: &'static str, path: &'static str) -> Cookie<'static> {
    Cookie::build((name, "")).path(path).build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_cookie_is_http_only_and_lives_30_days() {
        let cookie = session_cookie("abc".into(), true);
        assert_eq!(
            cookie.to_string(),
            "bodyweight_session=abc; HttpOnly; SameSite=Lax; Secure; Path=/; Max-Age=2592000"
        );
    }

    #[test]
    fn session_cookie_is_not_secure_over_plain_http() {
        assert_eq!(session_cookie("abc".into(), false).secure(), Some(false));
    }

    #[test]
    fn session_id_is_a_hash_of_the_token() {
        assert_eq!(
            session_id("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn random_tokens_differ() {
        let (a, b) = (random_token(), random_token());
        assert_eq!(a.len(), 64);
        assert_ne!(a, b);
    }

    #[test]
    fn dev_login_is_normalized_and_validated() {
        assert_eq!(normalize_dev_login(" Alice "), Some("alice".into()));
        assert_eq!(
            normalize_dev_login("a.b-c_d@ya.ru"),
            Some("a.b-c_d@ya.ru".into())
        );
        assert_eq!(normalize_dev_login(""), None);
        assert_eq!(normalize_dev_login("a b"), None);
        assert_eq!(normalize_dev_login(&"a".repeat(65)), None);
    }

    #[test]
    fn me_has_an_avatar_url_only_with_an_avatar() {
        let user = |avatar_id: Option<&str>| User {
            login: "ivan".into(),
            display_name: "Ivan".into(),
            avatar_id: avatar_id.map(Into::into),
        };
        assert_eq!(
            Me::from(user(Some("42"))).avatar_url.as_deref(),
            Some("https://avatars.yandex.net/get-yapic/42/islands-retina-middle")
        );
        assert_eq!(Me::from(user(None)).avatar_url, None);
    }
}
