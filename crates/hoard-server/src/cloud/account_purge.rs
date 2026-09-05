//! Hard-purge of soft-deleted accounts once their grace window elapses.
//!
//! `DELETE /v1/me` only flips `profiles.deleted_at` and freezes the account
//! (see `auth::require_active_account`). Nothing used to finish the job: the
//! "30-day purge" the delete response promised didn't exist, so a deleted
//! account sat frozen forever, its data never freed. This task closes that
//! loop: once a day it hard-deletes every account whose grace window has
//! passed:
//!
//! 1. Delete the account's R2 objects (deduped blobs, legacy archives, export
//!    ZIPs), because the DB cascade below can't reach object storage.
//! 2. `DELETE FROM profiles`, which cascades through the whole cloud schema
//!    (saves → versions → files, devices, subscriptions, sync_log, playtime,
//!    export_jobs, cloud_blobs) and NULLs the `audit_log` rows.
//!
//! The Supabase `auth.users` row is intentionally left: purging it needs the
//! admin API (service-role key) we don't wire here, and leaving it just means a
//! user who signs in again after the purge starts fresh with a new, empty
//! account, which is the intended outcome of a completed deletion.

use crate::cloud::routes::me::GRACE_DAYS;
use crate::cloud::state::CloudState;
use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;

/// Spawn the daily purge task. Detached: a failure just `warn!`s and the next
/// tick retries, exactly like the bandwidth/client-log sweepers.
pub fn spawn(state: CloudState) {
    tokio::spawn(async move {
        // Daily cadence. The first tick fires immediately; skip it so a
        // deploy doesn't run a heavy purge sweep during startup.
        let mut tick = tokio::time::interval(Duration::from_secs(24 * 60 * 60));
        tick.tick().await;
        loop {
            tick.tick().await;
            match purge_due(&state).await {
                Ok(0) => {}
                Ok(n) => {
                    tracing::info!(accounts = n, "account purge: hard-deleted expired accounts")
                }
                Err(e) => tracing::warn!(error = %e, "account purge: sweep failed"),
            }
        }
    });
}

/// Hard-delete every account past its grace window. Returns how many were
/// purged. Each account is handled independently: one failure logs and moves
/// on rather than aborting the sweep.
pub async fn purge_due(state: &CloudState) -> Result<usize, sqlx::Error> {
    let due: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT user_id FROM profiles
          WHERE deleted_at IS NOT NULL
            AND deleted_at < now() - make_interval(days => $1)",
    )
    .bind(GRACE_DAYS as i32)
    .fetch_all(&state.pool)
    .await?;

    let mut purged = 0usize;
    for (user_id,) in due {
        match purge_account(state, user_id).await {
            Ok(()) => purged += 1,
            Err(e) => {
                tracing::warn!(error = %e, user_id = %user_id, "account purge: account failed");
            }
        }
    }
    Ok(purged)
}

/// Delete one account's R2 objects then its DB rows (cascade). Best-effort on
/// R2: an object that fails to delete is logged and skipped rather than
/// blocking the DB purge: a leaked blob is a storage-cost nuisance, but a
/// half-purged account that keeps its DB rows is a correctness bug.
async fn purge_account(state: &CloudState, user_id: Uuid) -> Result<(), sqlx::Error> {
    for key in r2_keys_for(&state.pool, user_id).await? {
        if let Err(e) = state.r2.delete_object(&key).await {
            tracing::warn!(error = %e, r2_key = %key, user_id = %user_id, "account purge: R2 delete failed");
        }
    }

    // Cascades to saves, save_versions, save_version_files, devices,
    // subscriptions, sync_log, playtime, export_jobs, cloud_blobs; NULLs
    // audit_log.user_id.
    sqlx::query("DELETE FROM profiles WHERE user_id = $1")
        .bind(user_id)
        .execute(&state.pool)
        .await?;
    Ok(())
}

/// Every R2 object key the account owns: deduped content-addressed blobs,
/// legacy per-version archive blobs, and any export ZIPs.
async fn r2_keys_for(pool: &PgPool, user_id: Uuid) -> Result<Vec<String>, sqlx::Error> {
    let mut keys: Vec<String> = Vec::new();

    let blobs: Vec<(String,)> = sqlx::query_as("SELECT sha256 FROM cloud_blobs WHERE user_id = $1")
        .bind(user_id)
        .fetch_all(pool)
        .await?;
    keys.extend(
        blobs
            .into_iter()
            .map(|(sha,)| crate::cloud::r2::key_for_blob(user_id, &sha)),
    );

    // Legacy (non content-addressed) versions store one opaque archive each.
    let archives: Vec<(Option<String>,)> = sqlx::query_as(
        "SELECT sv.r2_key
           FROM save_versions sv
           JOIN saves s ON s.id = sv.save_id
          WHERE s.user_id = $1",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    keys.extend(
        archives
            .into_iter()
            .filter_map(|(k,)| k)
            .filter(|k| !k.is_empty()),
    );

    let exports: Vec<(Option<String>,)> =
        sqlx::query_as("SELECT r2_key FROM export_jobs WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(pool)
            .await?;
    keys.extend(
        exports
            .into_iter()
            .filter_map(|(k,)| k)
            .filter(|k| !k.is_empty()),
    );

    Ok(keys)
}
