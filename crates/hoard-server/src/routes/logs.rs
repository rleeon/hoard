//! `POST /v1/logs` — ingest of client diagnostic logs (self-hosted).
//!
//! Connected apps (desktop/CLI) ship batches of their `tracing` events here.
//! Self-hosted accepts *every* level; the wire shape and the batch caps are
//! shared with the cloud route (`cloud::routes::logs`), which additionally
//! filters to INFO+.

use axum::{
    extract::{Extension, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::auth::AuthUser;
use crate::routes::health::ServerState;

/// Max log entries accepted in a single request. Pairs with the per-route
/// body-size limit to bound abuse.
pub const MAX_BATCH_ENTRIES: usize = 500;
/// Per-request body cap for the logs endpoint (~256 KiB).
pub const MAX_BATCH_BYTES: usize = 256 * 1024;

/// Device metadata, sent once per batch (identical for every entry).
#[derive(Debug, Clone, Deserialize)]
pub struct DeviceMeta {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub os: Option<String>,
    #[serde(default)]
    pub fingerprint: Option<String>,
    #[serde(default)]
    pub app_version: Option<String>,
}

/// A single log line.
#[derive(Debug, Clone, Deserialize)]
pub struct LogEntry {
    pub level: String,
    #[serde(default)]
    pub target: Option<String>,
    pub message: String,
    /// Structured fields as a JSON object. Optional.
    #[serde(default)]
    pub fields: Option<serde_json::Value>,
    /// Client-side timestamp (RFC3339). Optional.
    #[serde(default)]
    pub ts: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LogBatch {
    pub device: DeviceMeta,
    pub entries: Vec<LogEntry>,
}

#[derive(Debug, Serialize)]
pub struct IngestResponse {
    pub accepted: usize,
}

/// Rank a level string for filtering. Unknown levels rank as INFO so they're
/// never silently dropped by the cloud filter.
pub fn level_rank(level: &str) -> u8 {
    match level.trim().to_ascii_lowercase().as_str() {
        "trace" => 0,
        "debug" => 1,
        "warn" => 3,
        "error" => 4,
        _ => 2, // "info" and anything unrecognised
    }
}

#[cfg(test)]
mod tests {
    use super::level_rank;

    #[test]
    fn cloud_info_filter() {
        // Cloud keeps INFO and above, drops TRACE/DEBUG. (CLOUD_MIN_RANK == 2)
        assert!(level_rank("trace") < 2);
        assert!(level_rank("debug") < 2);
        assert!(level_rank("info") >= 2);
        assert!(level_rank("WARN") >= 2);
        assert!(level_rank("error") >= 2);
        // Unknown levels are never silently dropped (treated as INFO).
        assert!(level_rank("notice") >= 2);
    }
}

pub async fn ingest(
    Extension(user): Extension<AuthUser>,
    State(state): State<Arc<ServerState>>,
    Json(batch): Json<LogBatch>,
) -> Result<(StatusCode, Json<IngestResponse>), StatusCode> {
    if batch.entries.len() > MAX_BATCH_ENTRIES {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }

    let user_id = user.user_id.to_string();
    let mut accepted = 0usize;

    for entry in &batch.entries {
        let id = uuid::Uuid::new_v4().to_string();
        let level = entry.level.trim().to_ascii_lowercase();
        let fields_json = entry
            .fields
            .as_ref()
            .map(|v| v.to_string());

        let res = sqlx::query(
            "INSERT INTO client_logs
                (id, user_id, device_name, device_os, device_fingerprint,
                 app_version, level, target, message, fields, client_ts)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&user_id)
        .bind(&batch.device.name)
        .bind(&batch.device.os)
        .bind(&batch.device.fingerprint)
        .bind(&batch.device.app_version)
        .bind(&level)
        .bind(&entry.target)
        .bind(&entry.message)
        .bind(&fields_json)
        .bind(&entry.ts)
        .execute(&state.pool)
        .await;

        match res {
            Ok(_) => accepted += 1,
            Err(e) => {
                tracing::error!(error = %e, "client log insert failed");
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    }

    Ok((StatusCode::OK, Json(IngestResponse { accepted })))
}
