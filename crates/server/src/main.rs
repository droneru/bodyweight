//! Web server: the API under `/api` and the built SPA for everything else.

use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::Context;
use axum::Router;
use axum::routing::get;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

/// Address to listen on, `BODYWEIGHT_LISTEN`.
const DEFAULT_LISTEN: &str = "127.0.0.1:3000";
/// Directory with the built SPA, `BODYWEIGHT_WEB_DIR`.
const DEFAULT_WEB_DIR: &str = "web/dist";

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

    let listener = tokio::net::TcpListener::bind(listen)
        .await
        .with_context(|| format!("cannot listen on {listen}"))?;
    tracing::info!(
        "listening on http://{listen}, serving SPA from {}",
        web_dir.display()
    );

    axum::serve(listener, app(web_dir))
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

fn app(web_dir: PathBuf) -> Router {
    // Unknown paths fall back to index.html so client-side routes work on reload.
    let spa = ServeDir::new(&web_dir).fallback(ServeFile::new(web_dir.join("index.html")));
    Router::new()
        .nest("/api", api())
        .fallback_service(spa)
        .layer(TraceLayer::new_for_http())
}

fn api() -> Router {
    Router::new()
        .route("/health", get(|| async { "ok" }))
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
