//! `POST /v1/cloud/logs`: ingest of client diagnostic logs (cloud).
//!
//! Cloud stores only *key events*: the client already filters to the level
//! advertised in `/v1/health` (WARN+), and we re-filter here as defense in
//! depth so a misbehaving client can't dump DEBUG/TRACE into the SaaS DB.
//!
//! The minimum is WARN rather than INFO because with the whole population
//! sending, operational INFO fills the table with noise and the 14-day prune takes
//! the rare cases with it. The exceptions are the `EXEMPT_TARGETS`: detection
//! contradictions and Hoard Screen telemetry come in whatever their level. They
//! are the signal collected on purpose, and filtering them by level would be
//! collecting everything except what matters.
//!
//! The wire shape and batch caps are shared with the self-hosted route
//! (`crate::routes::logs`). On top of that we resolve `device_id` by matching
//! `(user_id, fingerprint)` against the `devices` table when possible.

use axum::{extract::State, http::StatusCode, response::Json, Extension};
use uuid::Uuid;

use crate::cloud::auth::CloudUser;
use crate::cloud::errors::CloudError;
use crate::cloud::state::CloudState;
use crate::routes::logs::MAX_BATCH_ENTRIES;
use hoard_core::wire::{ships_at, LogBatch, LogIngestResponse, CLOUD_MIN_RANK};

pub async fn ingest(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
    Json(batch): Json<LogBatch>,
) -> Result<(StatusCode, Json<LogIngestResponse>), CloudError> {
    if batch.entries.len() > MAX_BATCH_ENTRIES {
        return Err(CloudError::BadRequest(format!(
            "batch too large: {} entries (max {})",
            batch.entries.len(),
            MAX_BATCH_ENTRIES
        )));
    }

    // Resolve the registered device (best-effort) so the log row can link to
    // it. Inline metadata is stored regardless, so a missing match is fine.
    let device_id: Option<Uuid> = if let Some(fp) = batch.device.fingerprint.as_ref() {
        sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM devices WHERE user_id = $1 AND fingerprint = $2",
        )
        .bind(user.user_id)
        .bind(fp.as_str())
        .fetch_optional(&state.pool)
        .await?
    } else {
        None
    };

    let mut accepted = 0usize;
    for entry in &batch.entries {
        let level = entry.level.trim().to_ascii_lowercase();
        if !ships_at(entry, CLOUD_MIN_RANK) {
            continue; // defense in depth: never store below WARN in cloud
        }
        let fields_json = entry.fields.as_ref().map(|v| v.to_string());

        sqlx::query(
            "INSERT INTO client_logs
                (user_id, device_id, device_name, device_os, device_fingerprint,
                 app_version, level, target, message, fields, client_ts)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10::jsonb, $11::timestamptz)",
        )
        .bind(user.user_id)
        .bind(device_id)
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
        .await?;

        accepted += 1;
    }

    Ok((StatusCode::OK, Json(LogIngestResponse { accepted })))
}
