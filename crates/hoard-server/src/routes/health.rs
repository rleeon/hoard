use axum::{extract::State, http::StatusCode, response::Json};
use hoard_core::wire::Health;
use sqlx::SqlitePool;
use std::sync::Arc;
use std::time::Instant;

pub struct ServerState {
    pub pool: SqlitePool,
    pub config: crate::config::Config,
    pub start_time: Instant,
    /// Blob/chunk storage backend (local disk or S3-compatible, ADR 0020).
    /// The snapshot upload/download and trash-purge paths go through this
    /// instead of touching blob files directly.
    pub store: std::sync::Arc<dyn crate::store::BlobStore>,
    /// Per-user SSE fan-out backing `GET /v1/events` (self-hosted push). The
    /// snapshot-commit path publishes here; listening devices pull on the
    /// event. Empty/unused on the cloud deployment (Supabase Realtime does the
    /// equivalent). See [`crate::routes::events`].
    pub events: crate::routes::events::EventBus,
    /// Peers whose `X-Forwarded-For` this server believes, resolved from
    /// `server.trusted_proxies` at boot so a request never re-parses config,
    /// and so a typo is reported once, on startup, instead of silently
    /// trusting nobody. See [`crate::clientip`].
    pub trusted_proxies: crate::clientip::TrustedProxies,
}

/// Build the body. `log_min_level` is always `"debug"` self-hosted (it keeps
/// every level); `mode` stays absent, and that is what tells the client to speak the
/// self-hosted protocol instead of `/v1/cloud/*`.
///
/// `cas` announces the content-addressed upload routes (`routes::cas`). It is a
/// capability, not a setting: this binary always has them, and a client that
/// doesn't see the flag is talking to a server old enough to only understand the
/// multipart upload. There is no way to infer it from `version`: client and
/// server are updated independently, which is the whole reason it's here.
fn body(status: &str, uptime_secs: u64) -> Health {
    Health {
        status: status.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_secs,
        log_min_level: Some("debug".to_string()),
        mode: None,
        cas: true,
        devices: true,
    }
}

pub async fn handler(State(state): State<Arc<ServerState>>) -> (StatusCode, Json<Health>) {
    // Quick DB ping
    let db_ok = sqlx::query("SELECT 1").execute(&state.pool).await.is_ok();
    let uptime_secs = state.start_time.elapsed().as_secs();

    if !db_ok {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(body("db_error", uptime_secs)),
        );
    }

    (StatusCode::OK, Json(body("ok", uptime_secs)))
}
