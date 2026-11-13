//! Web server: the API under `/api` and the built SPA for everything else.

// The ydb SDK futures are deeply nested, see https://github.com/rust-lang/rust/issues/152942
#![recursion_limit = "256"]

mod auth;
mod exercises;
#[cfg(test)]
mod integration_tests;
mod weigh_ins;
mod yandex;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use axum::Router;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use bodyweight_storage::Storage;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

/// Address to listen on, `BODYWEIGHT_LISTEN`.
const DEFAULT_LISTEN: &str = "127.0.0.1:3000";
/// Directory with the built SPA, `BODYWEIGHT_WEB_DIR`.
const DEFAULT_WEB_DIR: &str = "web/dist";
/// Address users open the app at, `BODYWEIGHT_PUBLIC_URL`. The default is the Vite dev server.
const DEFAULT_PUBLIC_URL: &str = "http://localhost:5173";
/// Registered as the Redirect URI of the Yandex OAuth application.
const YANDEX_CALLBACK_PATH: &str = "/api/auth/yandex/callback";

#[derive(Clone)]
struct AppState {
    storage: Arc<Storage>,
    /// `None` if `BODYWEIGHT_YANDEX_CLIENT_ID` and `BODYWEIGHT_YANDEX_CLIENT_SECRET` are not set.
    yandex: Option<Arc<yandex::YandexOAuth>>,
    /// Cookies get the `Secure` attribute when the app is served over HTTPS.
    secure_cookies: bool,
}

/// An unexpected failure: logged, and the client gets a bare 500.
struct ApiError(anyhow::Error);

impl<E: Into<anyhow::Error>> From<E> for ApiError {
    fn from(error: E) -> Self {
        Self(error.into())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        tracing::error!("request failed: {:#}", self.0);
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tower_http=info".into()),
        )
        .init();

    let listen: SocketAddr = std::env::var("BODYWEIGHT_LISTEN")
        .unwrap_or_else(|_| DEFAULT_LISTEN.into())
        .parse()
        .context("invalid BODYWEIGHT_LISTEN")?;
    let web_dir = PathBuf::from(
        std::env::var("BODYWEIGHT_WEB_DIR").unwrap_or_else(|_| DEFAULT_WEB_DIR.into()),
    );

    let public_url = std::env::var("BODYWEIGHT_PUBLIC_URL")
        .unwrap_or_else(|_| DEFAULT_PUBLIC_URL.into())
        .trim_end_matches('/')
        .to_owned();
    let dev_login = std::env::var("BODYWEIGHT_DEV_LOGIN").is_ok_and(|v| v == "1");
    let yandex = match (
        non_empty_env("BODYWEIGHT_YANDEX_CLIENT_ID"),
        non_empty_env("BODYWEIGHT_YANDEX_CLIENT_SECRET"),
    ) {
        (Some(id), Some(secret)) => Some(Arc::new(yandex::YandexOAuth::new(
            id,
            secret,
            format!("{public_url}{YANDEX_CALLBACK_PATH}"),
        ))),
        (None, None) => None,
        _ => anyhow::bail!(
            "set both BODYWEIGHT_YANDEX_CLIENT_ID and BODYWEIGHT_YANDEX_CLIENT_SECRET, or neither"
        ),
    };
    if yandex.is_none() {
        tracing::warn!("Yandex sign-in is off: BODYWEIGHT_YANDEX_CLIENT_ID is not set");
    }
    if dev_login {
        tracing::warn!("dev login is on: anyone can sign in as any user");
    }

    let storage = Storage::connect_from_env()
        .await
        .context("failed to connect to YDB")?;
    let state = AppState {
        storage: Arc::new(storage),
        yandex,
        secure_cookies: public_url.starts_with("https://"),
    };

    let listener = tokio::net::TcpListener::bind(listen)
        .await
        .with_context(|| format!("cannot listen on {listen}"))?;
    tracing::info!(
        "listening on http://{listen} for {public_url}, serving SPA from {}",
        web_dir.display()
    );

    axum::serve(listener, app(web_dir, state, dev_login))
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

fn non_empty_env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}

fn app(web_dir: PathBuf, state: AppState, dev_login: bool) -> Router {
    // Unknown paths fall back to index.html so client-side routes work on reload.
    let spa = ServeDir::new(&web_dir).fallback(ServeFile::new(web_dir.join("index.html")));
    Router::new()
        .nest("/api", api(dev_login).with_state(state))
        .fallback_service(spa)
        .layer(TraceLayer::new_for_http())
}

fn api(dev_login: bool) -> Router<AppState> {
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .merge(auth::routes(dev_login))
        .merge(weigh_ins::routes())
        .merge(exercises::routes())
        .fallback(|| async { axum::http::StatusCode::NOT_FOUND })
}

/// Resolves on Ctrl+C or SIGTERM (sent by `docker stop`).
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.ok();
    };
    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            s.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}
