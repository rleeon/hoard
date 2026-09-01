//! `POST /v1/logs`: ingest of client diagnostic logs (self-hosted).
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
use hoard_core::wire::{LogBatch, LogIngestResponse};
use std::sync::Arc;

use crate::auth::AuthUser;
use crate::routes::health::ServerState;

/// Max log entries accepted in a single request. Pairs with the per-route
/// body-size limit to bound abuse.
pub const MAX_BATCH_ENTRIES: usize = 500;
/// Per-request body cap for the logs endpoint (~256 KiB).
pub const MAX_BATCH_BYTES: usize = 256 * 1024;

// The body (`LogBatch`, `LogEntry`, `DeviceMeta`) and the response live in
// `hoard_core::wire` (ADR 0021 C.6). This pair was real drift: the client declared
// `target` and `ts` as required and the server had them as `Option`.

// The level ordering and the rule about what gets stored live in
// `hoard_core::wire` (`level_rank`, `ships_at`, `CLOUD_MIN_RANK`), shared with the
// client. They used to be written three times, here, in the cloud namespace and in
// the agent's shipper, and a duplicated rule is a silent leak waiting its turn. If
// the client filters at one level and the server at another, either what the
// server throws away gets sent or what the client sends gets dropped, and nobody
// finds out.

pub async fn ingest(
    Extension(user): Extension<AuthUser>,
    State(state): State<Arc<ServerState>>,
    Json(batch): Json<LogBatch>,
) -> Result<(StatusCode, Json<LogIngestResponse>), StatusCode> {
    if batch.entries.len() > MAX_BATCH_ENTRIES {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }

    let user_id = user.user_id.to_string();
    let mut accepted = 0usize;

    for entry in &batch.entries {
        let id = uuid::Uuid::new_v4().to_string();
        let level = entry.level.trim().to_ascii_lowercase();
        let fields_json = entry.fields.as_ref().map(|v| v.to_string());

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
        .bind(batch.device.fingerprint.as_ref().map(|f| f.as_str()))
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

    Ok((StatusCode::OK, Json(LogIngestResponse { accepted })))
}

// The rule's matrix is tested once, where it lives: `hoard_core::wire`
// (`one_rule_decides_what_travels_and_what_is_stored`).
