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
            }),
        );
    }

    (
        StatusCode::OK,
        Json(HealthResponse {
            status: "ok",
            version: env!("CARGO_PKG_VERSION"),
            uptime_secs: state.start_time.elapsed().as_secs(),
        }),
    )
}
