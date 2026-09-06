//! Deriving a version's insight from the manifests the server already holds.
//!
//! The decision itself is `hoard_core::kernel::insight`: pure, tested, shared.
//! This module is only the IO around it: read the version's manifest and the
//! one before it, hand both to the kernel, store what comes back.
//!
//! It runs **after** the commit transaction, never inside it. What it produces
//! is how a row is labelled, and no amount of it is worth failing an upload
//! that already landed: every caller logs a failure and moves on.
//!
//! Both backends are here, side by side, because they are the same three steps
//! against two different databases; splitting them by dialect would hide that
//! the cloud and the self-hosted history are supposed to say the same thing.

use anyhow::Result;
use hoard_core::kernel::insight::{insight_from_manifests, ManifestFile, VersionInsight};
#[cfg(feature = "cloud")]
use sqlx::PgPool;
use sqlx::SqlitePool;

/// Manifest rows we are willing to load to label one row of history.
///
/// One save in production carried 35,143 files over 24,784 blobs because the
/// game writes one file per map chunk, and loading two manifests that size to
/// decide what to *call* the version is the wrong trade at any speed. Past this
/// the version keeps the plain label it has always had.
const MAX_MANIFEST_ROWS: usize = 20_000;

/// Compute and store the insight for a freshly committed cloud version, and
/// hand it back so the response can carry it without a second read.
#[cfg(feature = "cloud")]
pub async fn record_cloud(
    pool: &PgPool,
    save_id: &str,
    version: i64,
) -> Result<Option<VersionInsight>> {
    let cur = cloud_manifest(pool, save_id, version).await?;
    if cur.is_empty() || cur.len() > MAX_MANIFEST_ROWS {
        return Ok(None);
    }
    // The version before this one that actually committed. Skipping the
    // uncommitted (`sha256 = ''`) rows matters: a failed upload leaves one
    // behind, and diffing against a version that has no manifest would report
    // every file as new.
    let prev: Option<i64> = sqlx::query_scalar(
        "SELECT MAX(version_num) FROM save_versions
          WHERE save_id = $1 AND version_num < $2 AND sha256 <> ''",
    )
    .bind(save_id)
    .bind(version)
    .fetch_one(pool)
    .await?;

    let prev_files = match prev {
        Some(p) => cloud_manifest(pool, save_id, p).await?,
        None => Vec::new(),
    };
    let insight = insight_from_manifests(&cur, &prev_files, &[]);
    if insight.is_empty() {
        return Ok(None);
    }

    sqlx::query("UPDATE save_versions SET insight = $1 WHERE save_id = $2 AND version_num = $3")
        .bind(sqlx::types::Json(&insight))
        .bind(save_id)
        .bind(version)
        .execute(pool)
        .await?;
    Ok(Some(insight))
}

#[cfg(feature = "cloud")]
async fn cloud_manifest(pool: &PgPool, save_id: &str, version: i64) -> Result<Vec<ManifestFile>> {
    let rows: Vec<(String, String, i64, Option<i64>)> = sqlx::query_as(
        "SELECT relative_path, encode(sha256, 'hex'), size_bytes, modified_at
           FROM manifest_files
          WHERE save_id = $1 AND version_num = $2
          LIMIT $3",
    )
    .bind(save_id)
    .bind(version)
    .bind(MAX_MANIFEST_ROWS as i64 + 1)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(into_manifest_file).collect())
}

/// Compute and store the insight for a freshly committed self-hosted snapshot,
/// and hand it back so the response can carry it without a second read.
pub async fn record_selfhosted(
    pool: &SqlitePool,
    save_id: &str,
    version: i64,
) -> Result<Option<VersionInsight>> {
    let cur = selfhosted_manifest(pool, save_id, version).await?;
    if cur.is_empty() || cur.len() > MAX_MANIFEST_ROWS {
        return Ok(None);
    }
    let prev: Option<i64> = sqlx::query_scalar(
        "SELECT MAX(version_num) FROM snapshots
          WHERE save_id = ? AND version_num < ? AND deleted_at IS NULL",
    )
    .bind(save_id)
    .bind(version)
    .fetch_one(pool)
    .await?;

    let prev_files = match prev {
        Some(p) => selfhosted_manifest(pool, save_id, p).await?,
        None => Vec::new(),
    };
    let insight = insight_from_manifests(&cur, &prev_files, &[]);
    if insight.is_empty() {
        return Ok(None);
    }

    // Stored as text: SQLite has no JSON type and nothing here queries inside
    // the value.
    let encoded = serde_json::to_string(&insight)?;
    sqlx::query("UPDATE snapshots SET insight = ? WHERE save_id = ? AND version_num = ?")
        .bind(encoded)
        .bind(save_id)
        .bind(version)
        .execute(pool)
        .await?;
    Ok(Some(insight))
}

async fn selfhosted_manifest(
    pool: &SqlitePool,
    save_id: &str,
    version: i64,
) -> Result<Vec<ManifestFile>> {
    let rows: Vec<(String, String, i64, Option<i64>)> = sqlx::query_as(
        "SELECT f.relative_path, f.sha256, f.size_bytes, f.modified_at
           FROM snapshot_files f
           JOIN snapshots s ON s.id = f.snapshot_id
          WHERE s.save_id = ? AND s.version_num = ?
          LIMIT ?",
    )
    .bind(save_id)
    .bind(version)
    .bind(MAX_MANIFEST_ROWS as i64 + 1)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(into_manifest_file).collect())
}

fn into_manifest_file(
    (relative_path, sha256, size_bytes, modified_at): (String, String, i64, Option<i64>),
) -> ManifestFile {
    ManifestFile {
        relative_path,
        sha256,
        size_bytes,
        modified_at,
    }
}

/// How many versions one history listing may backfill in the background.
///
/// Everything committed before this existed has no insight, and the answer to
/// "when does the user's history get labelled" should not be "never, unless you
/// re-upload". Listing a save's history says exactly which versions someone is
/// looking at, so that is when the missing ones get computed: capped, off the
/// response path, and idempotent, so two windows open at once just do the same
/// harmless UPDATE twice.
pub const BACKFILL_PER_LISTING: usize = 25;

/// Does this row want computing again?
///
/// Missing, or written by an older schema than this binary derives. The rules
/// that pick what a row says get better (a name that used to come out as
/// `murray heath_31852938(m)` now comes out as `murray heath`) and a stored
/// label must not outlive the improvement. The row still serves what it has
/// meanwhile, so nothing blinks to empty while the recompute runs.
pub fn needs_refresh(stored: Option<&VersionInsight>) -> bool {
    match stored {
        None => true,
        Some(i) => i.schema < hoard_core::kernel::insight::SCHEMA,
    }
}

/// Read a stored insight back, tolerating anything the column may hold.
///
/// The column is written by this server and by no one else, but it is persisted
/// state: a row could carry the shape of a version from two releases ago, or a
/// half-written value from a bug we haven't found yet. A history listing that
/// 500s because one version's label doesn't parse is a far worse failure than a
/// row without a label, so this never propagates an error.
pub fn parse_stored(raw: Option<&str>) -> Option<VersionInsight> {
    let raw = raw?;
    match serde_json::from_str(raw) {
        Ok(v) => Some(v),
        Err(e) => {
            tracing::debug!(error = %e, "insight: stored value did not parse; row goes out plain");
            None
        }
    }
}
