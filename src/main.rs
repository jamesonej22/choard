mod config;

use std::{env, fs, path::Path, str::FromStr};

use anyhow::{Context, Result};
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use config::AppConfig;
use serde::Serialize;
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
struct AppState {
    pool: SqlitePool,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing()?;
    let config = AppConfig::load()?;
    let pool = initialize_database(&config.database.path).await?;
    let address = config.socket_address();
    let listener = TcpListener::bind(&address)
        .await
        .with_context(|| format!("failed to bind HTTP server to {address}"))?;

    info!(%address, "household tracker server is listening");
    axum::serve(listener, app(AppState { pool }))
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("HTTP server failed")?;

    Ok(())
}

fn init_tracing() -> Result<()> {
    let filter = match env::var("RUST_LOG") {
        Ok(value) => EnvFilter::from_str(&value).context("invalid RUST_LOG filter")?,
        Err(env::VarError::NotPresent) => EnvFilter::new("info"),
        Err(error) => return Err(error).context("failed to read RUST_LOG"),
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .try_init()
        .map_err(|error| anyhow::anyhow!("failed to initialize tracing subscriber: {error}"))
}

async fn initialize_database(path: &Path) -> Result<SqlitePool> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create database directory {}", parent.display()))?;
    }

    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .foreign_keys(true);
    SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .with_context(|| format!("failed to open SQLite database {}", path.display()))
}

fn app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}

async fn health(State(state): State<AppState>) -> Response {
    match sqlx::query_scalar::<_, i64>("SELECT 1")
        .fetch_one(&state.pool)
        .await
    {
        Ok(_) => (StatusCode::OK, Json(HealthResponse { status: "ok" })).into_response(),
        Err(error) => {
            error!(%error, "health check failed because SQLite is unavailable");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(HealthResponse {
                    status: "unavailable",
                }),
            )
                .into_response()
        }
    }
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut terminate =
            match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
                Ok(signal) => signal,
                Err(error) => {
                    error!(%error, "failed to listen for SIGTERM");
                    return;
                }
            };

        tokio::select! {
            result = tokio::signal::ctrl_c() => {
                if let Err(error) = result {
                    error!(%error, "failed to listen for Ctrl-C");
                }
            }
            _ = terminate.recv() => info!("received SIGTERM"),
        }
    }

    #[cfg(not(unix))]
    if let Err(error) = tokio::signal::ctrl_c().await {
        error!(%error, "failed to listen for Ctrl-C");
    }

    info!("shutting down server");
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    use super::*;

    #[tokio::test]
    async fn health_returns_ok_when_database_is_available() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("in-memory SQLite connection should succeed");

        let response = app(AppState { pool })
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .expect("health request should be valid"),
            )
            .await
            .expect("health route should respond");

        assert_eq!(response.status(), StatusCode::OK);
    }
}
