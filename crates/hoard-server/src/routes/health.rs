use axum::{extract::State, http::StatusCode, response::Json};
use serde::Serialize;
use sqlx::SqlitePool;
use std::sync::Arc;
use std::time::Instant;

pub struct ServerState {
    pub pool: SqlitePool,
    pub config: crate::config::Config,
    pub start_time: Instant,
}

#[derive(Serialize)]
pub struct HealthResponse {
    status: &'static str,
    version: &'static str,
    uptime_secs: u64,
    /// Minimum log level this server accepts for client-log ingest. The
    /// client reads this on connect and filters at source. Self-hosted
    /// keeps everything, so this is always `"debug"`.
    log_min_level: &'static str,
}

pub async fn handler(State(state): State<Arc<ServerState>>) -> (StatusCode, Json<HealthResponse>) {
    // Quick DB ping
    let db_ok = sqlx::query("SELECT 1").execute(&state.pool).await.is_ok();

    if !db_ok {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(HealthResponse {
                status: "db_error",
                version: env!("CARGO_PKG_VERSION"),
                uptime_secs: state.start_time.elapsed().as_secs(),
                log_min_level: "debug",
            }),
        );
    }

    (
        StatusCode::OK,
        Json(HealthResponse {
            status: "ok",
            version: env!("CARGO_PKG_VERSION"),
            uptime_secs: state.start_time.elapsed().as_secs(),
            log_min_level: "debug",
        }),
    )
}
