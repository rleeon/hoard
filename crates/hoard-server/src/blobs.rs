//! Content-addressed blob store (ADR 0018, eje C).
//!
//! File bytes live once per user at `blobs/<user_id>/<sha[0:2]>/<sha256>`.
//! Snapshots stop owning a physical `v<n>/` folder — a version is purely the
//! list of its `snapshot_files` rows, each pointing at a blob by sha256.
//! `blobs.refcount` counts every referencing row (live or trashed); GC of the
//! file happens only when it reaches 0 during the trash purge.

use std::path::{Path, PathBuf};

use sqlx::{Row, SqlitePool};
use tracing::{info, warn};

/// Absolute on-disk path of a blob. Sharded by the first two hex chars of the
/// sha to avoid one giant directory.
pub fn blob_path(data_dir: &Path, user_id: &str, sha256: &str) -> PathBuf {
    let prefix = if sha256.len() >= 2 {
        &sha256[..2]
    } else {
        "00"
    };
    data_dir
        .join("blobs")
        .join(user_id)
        .join(prefix)
        .join(sha256)
}

/// One-time migration of legacy folder-based snapshots into the blob store.
///
/// Idempotent and best-effort: if `blobs` already has rows we assume the
/// migration ran; if there are no snapshots it's a fresh install and we do
/// nothing. Physical files are copied (not moved) into `blobs/` inside the
/// transaction, the per-user quota is recomputed to the sum of unique blob
/// sizes, and only after the commit are the old `v<n>/` and `trash/<id>/`
/// folders removed.
pub async fn backfill_from_folders(pool: &SqlitePool, data_dir: &Path) -> anyhow::Result<()> {
    let blob_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM blobs")
        .fetch_one(pool)
        .await?;
    if blob_count > 0 {
        return Ok(());
    }

    let snaps = sqlx::query(
        "SELECT s.id AS id, s.version_num AS version_num, s.deleted_at AS deleted_at,
                sv.user_id AS user_id, sv.game_slug AS game_slug, sv.label AS label
         FROM snapshots s JOIN saves sv ON sv.id = s.save_id",
    )
    .fetch_all(pool)
    .await?;

    if snaps.is_empty() {
        return Ok(());
    }

    info!(
        snapshots = snaps.len(),
        "migrating snapshots to content-addressed blob store (ADR 0018)"
    );

    let mut old_dirs: Vec<PathBuf> = Vec::new();
    let mut tx = pool.begin().await?;
    let mut missing_files = 0u64;
    let mut stored_blobs = 0u64;

    for snap in &snaps {
        let snap_id: String = snap.get("id");
        let version_num: i64 = snap.get("version_num");
        let deleted: Option<String> = snap.get("deleted_at");
        let user_id: String = snap.get("user_id");
        let game_slug: String = snap.get("game_slug");
        let label: String = snap.get("label");

        let base = if deleted.is_some() {
            data_dir.join("trash").join(&snap_id)
        } else {
            data_dir
                .join("data")
                .join(&user_id)
                .join(&game_slug)
                .join(&label)
                .join(format!("v{}", version_num))
        };

        let files = sqlx::query(
            "SELECT relative_path, size_bytes, sha256 FROM snapshot_files WHERE snapshot_id = ?",
        )
        .bind(&snap_id)
        .fetch_all(&mut *tx)
        .await?;

        for f in &files {
            let rel: String = f.get("relative_path");
            let size: i64 = f.get("size_bytes");
            let sha: String = f.get("sha256");

            // refcount must equal the number of referencing rows, regardless of
            // whether the physical file is still present.
            sqlx::query(
                "INSERT INTO blobs (user_id, sha256, size_bytes, refcount)
                 VALUES (?,?,?,1)
                 ON CONFLICT(user_id, sha256) DO UPDATE SET refcount = refcount + 1",
            )
            .bind(&user_id)
            .bind(&sha)
            .bind(size)
            .execute(&mut *tx)
            .await?;

            let dst = blob_path(data_dir, &user_id, &sha);
            if !dst.exists() {
                let src = base.join(&rel);
                if src.exists() {
                    if let Some(parent) = dst.parent() {
                        tokio::fs::create_dir_all(parent).await.ok();
                    }
                    if let Err(e) = tokio::fs::copy(&src, &dst).await {
                        warn!(error = %e, src = %src.display(), "blob backfill copy failed");
                    } else {
                        stored_blobs += 1;
                    }
                } else {
                    missing_files += 1;
                }
            }
        }

        if base.exists() {
            old_dirs.push(base);
        }
    }

    // Quota now means: sum of unique blob sizes plus unique chunk sizes
    // referenced by the user (ADR 0018 eje C + ADR 0019 Fase 4). Idempotent —
    // recomputed from scratch, so re-running never drifts.
    sqlx::query(
        "UPDATE users SET storage_used_bytes =
            (SELECT COALESCE(SUM(size_bytes),0) FROM blobs WHERE blobs.user_id = users.id)
          + (SELECT COALESCE(SUM(size_bytes),0) FROM chunks WHERE chunks.user_id = users.id)",
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    for dir in old_dirs {
        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    info!(
        stored = stored_blobs,
        missing = missing_files,
        "blob store migration complete"
    );
    if missing_files > 0 {
        warn!(
            missing = missing_files,
            "some snapshot files were absent on disk during migration; those versions may be incomplete"
        );
    }
    Ok(())
}
