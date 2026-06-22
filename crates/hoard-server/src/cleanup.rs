//! Background cleanup task: prunes redundant snapshots (ADR 0018, eje B)
//! and purges old tmp uploads, trashed snapshots and client logs.

use sqlx::{Row, SqlitePool};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use time::format_description::well_known::Rfc3339;
use tracing::{info, warn};

use crate::retention::{plan_prune, RetentionPolicy, SnapshotMeta};

pub async fn run_periodic(
    pool: SqlitePool,
    data_dir: PathBuf,
    tmp_cleanup_hours: u64,
    trash_retention_days: u64,
    prune_policy: Option<RetentionPolicy>,
) {
    // Run every hour
    let mut interval = tokio::time::interval(Duration::from_secs(3600));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        interval.tick().await;
        if let Err(e) = run_once(
            &pool,
            &data_dir,
            tmp_cleanup_hours,
            trash_retention_days,
            prune_policy.as_ref(),
        )
        .await
        {
            warn!(error = %e, "cleanup task error");
        }
    }
}

pub async fn run_once(
    pool: &SqlitePool,
    data_dir: &Path,
    tmp_cleanup_hours: u64,
    trash_retention_days: u64,
    prune_policy: Option<&RetentionPolicy>,
) -> anyhow::Result<()> {
    // Prune first so freshly-trashed snapshots can be purged in the same
    // cycle once they age past `trash_retention_days`.
    if let Some(policy) = prune_policy {
        if let Err(e) = prune_snapshots(pool, policy).await {
            warn!(error = %e, "snapshot pruning error");
        }
    }
    purge_tmp(data_dir, tmp_cleanup_hours).await?;
    purge_trash(pool, data_dir, trash_retention_days).await?;
    purge_client_logs(pool, CLIENT_LOG_RETENTION_DAYS).await?;
    Ok(())
}

/// Age-weighted snapshot pruning (ADR 0018, eje B). For each save, decide
/// which live snapshots are redundant per `policy` and soft-delete them
/// (logical only — the bytes stay until the trash purge). Recoverable until
/// then. Runtime queries (not the `query!` macro) so this doesn't depend on
/// the `.sqlx` offline cache being regenerated.
async fn prune_snapshots(pool: &SqlitePool, policy: &RetentionPolicy) -> anyhow::Result<()> {
    let save_ids: Vec<String> = sqlx::query("SELECT id FROM saves")
        .fetch_all(pool)
        .await?
        .iter()
        .map(|r| r.get::<String, _>("id"))
        .collect();

    let mut pruned_total = 0u64;
    for save_id in save_ids {
        let rows = sqlx::query(
            "SELECT s.id AS id, s.version_num AS version_num, s.created_at AS created_at,
                    s.is_pinned AS is_pinned, s.total_size_bytes AS total_size_bytes,
                    sv.user_id AS user_id, sv.game_slug AS game_slug, sv.label AS label
             FROM snapshots s JOIN saves sv ON sv.id = s.save_id
             WHERE s.save_id = ? AND s.deleted_at IS NULL
             ORDER BY s.version_num DESC",
        )
        .bind(&save_id)
        .fetch_all(pool)
        .await?;

        if rows.len() <= 1 {
            continue;
        }

        let metas: Vec<SnapshotMeta> = rows
            .iter()
            .map(|r| {
                let created_at: String = r.get("created_at");
                let created_unix = time::OffsetDateTime::parse(&created_at, &Rfc3339)
                    .map(|t| t.unix_timestamp())
                    // Fall back to ordering by version so an unparseable
                    // timestamp never makes us treat it as "epoch" (which
                    // would look ancient and get pruned aggressively).
                    .unwrap_or_else(|_| r.get::<i64, _>("version_num"));
                SnapshotMeta {
                    id: r.get("id"),
                    created_unix,
                    is_pinned: r.get::<i64, _>("is_pinned") != 0,
                    size_bytes: r.get("total_size_bytes"),
                }
            })
            .collect();

        let to_prune = plan_prune(&metas, policy);
        if to_prune.is_empty() {
            continue;
        }

        // Index row details by snapshot id for the soft-delete pass.
        for id in &to_prune {
            let Some(row) = rows.iter().find(|r| &r.get::<String, _>("id") == id) else {
                continue;
            };
            let user_id: String = row.get("user_id");
            if let Err(e) = soft_delete_pruned(pool, id, &user_id).await {
                warn!(snapshot_id = %id, error = %e, "prune soft-delete failed");
            } else {
                pruned_total += 1;
            }
        }
    }

    if pruned_total > 0 {
        info!(pruned = pruned_total, "pruned redundant snapshots");
    }
    Ok(())
}

/// Soft-delete one snapshot as part of retention pruning. With the blob store
/// (ADR 0018, eje C) this is purely logical: mark deleted and audit. The bytes
/// stay on disk with their blob refcounts intact — a trashed snapshot still
/// pins its blobs (and quota) until `purge_trash` decrements refcounts and GCs
/// any blob that reaches 0. No quota change and no folder move here.
async fn soft_delete_pruned(
    pool: &SqlitePool,
    snapshot_id: &str,
    user_id: &str,
) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;

    sqlx::query(
        "UPDATE snapshots SET deleted_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE id = ?",
    )
    .bind(snapshot_id)
    .execute(&mut *tx)
    .await?;

    let audit_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO audit_log (id, user_id, event_type, entity_id)
         VALUES (?,?,'snapshot.pruned',?)",
    )
    .bind(&audit_id)
    .bind(user_id)
    .bind(snapshot_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

/// Client diagnostic logs are kept for 14 days on both branches.
const CLIENT_LOG_RETENTION_DAYS: i64 = 14;

async fn purge_client_logs(pool: &SqlitePool, retention_days: i64) -> anyhow::Result<()> {
    let cutoff = time::OffsetDateTime::now_utc() - time::Duration::days(retention_days);
    let cutoff_str = cutoff.format(&time::format_description::well_known::Rfc3339)?;
    // Runtime query (not the `query!` macro) so this doesn't depend on the
    // .sqlx offline cache being regenerated.
    let res = sqlx::query("DELETE FROM client_logs WHERE received_at < ?")
        .bind(&cutoff_str)
        .execute(pool)
        .await?;
    let removed = res.rows_affected();
    if removed > 0 {
        info!(removed, "purged expired client logs");
    }
    Ok(())
}

async fn purge_tmp(data_dir: &Path, max_age_hours: u64) -> anyhow::Result<()> {
    let tmp_dir = data_dir.join("tmp");
    if !tmp_dir.exists() {
        return Ok(());
    }
    let max_age = Duration::from_secs(max_age_hours * 3600);
    let now = SystemTime::now();

    let mut entries = tokio::fs::read_dir(&tmp_dir).await?;
    let mut removed = 0;
    while let Some(entry) = entries.next_entry().await? {
        let meta = entry.metadata().await?;
        let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        if let Ok(age) = now.duration_since(mtime) {
            if age > max_age {
                let _ = tokio::fs::remove_dir_all(entry.path()).await;
                removed += 1;
            }
        }
    }
    if removed > 0 {
        info!(removed, "purged stale tmp uploads");
    }
    Ok(())
}

/// A content-addressed object scheduled for file removal after the purge tx
/// commits. We keep the table + sha so we can re-check the refcount just before
/// unlinking: a concurrent upload may have re-referenced (and re-created) the
/// row in the gap between commit and `remove_file`, in which case deleting the
/// file would corrupt a live blob/chunk.
struct GcTarget {
    is_chunk: bool,
    sha: String,
    path: PathBuf,
}

/// Permanently delete snapshots that have outlived the trash window. With the
/// blob store (ADR 0018, eje C) this is where bytes actually get freed: each
/// purged snapshot decrements the refcount of every blob it referenced, and a
/// blob that reaches 0 is GC'd (row + file deleted, owner quota refunded).
async fn purge_trash(
    pool: &SqlitePool,
    data_dir: &Path,
    retention_days: u64,
) -> anyhow::Result<()> {
    let cutoff = time::OffsetDateTime::now_utc() - time::Duration::days(retention_days as i64);
    let cutoff_str = cutoff.format(&time::format_description::well_known::Rfc3339)?;

    // Pair each expired snapshot with its owner so we can credit the right user.
    let rows = sqlx::query(
        "SELECT s.id AS id, sv.user_id AS user_id
         FROM snapshots s JOIN saves sv ON sv.id = s.save_id
         WHERE s.deleted_at IS NOT NULL AND s.deleted_at < ?",
    )
    .bind(&cutoff_str)
    .fetch_all(pool)
    .await?;

    let mut removed = 0u64;
    for row in &rows {
        let snap_id: String = row.get("id");
        let user_id: String = row.get("user_id");

        // The whole-file shas this snapshot referenced (one row per file, dups
        // included so the refcount decrement matches the increment from
        // `create`). Chunked files have no blob row, so their decrement below
        // is a harmless no-op — their bytes are freed via the chunk pass.
        let shas: Vec<String> =
            sqlx::query("SELECT sha256 FROM snapshot_files WHERE snapshot_id = ?")
                .bind(&snap_id)
                .fetch_all(pool)
                .await?
                .iter()
                .map(|r| r.get::<String, _>("sha256"))
                .collect();

        // The chunk shas this snapshot referenced (ADR 0019, Fase 4): one row
        // per chunk reference, dups included, matching the per-chunk increment.
        let chunk_shas: Vec<String> = sqlx::query(
            "SELECT sfc.chunk_sha256 AS sha
             FROM snapshot_file_chunks sfc
             JOIN snapshot_files sf ON sf.id = sfc.snapshot_file_id
             WHERE sf.snapshot_id = ?",
        )
        .bind(&snap_id)
        .fetch_all(pool)
        .await?
        .iter()
        .map(|r| r.get::<String, _>("sha"))
        .collect();

        let mut tx = pool.begin().await?;
        let mut freed_bytes: i64 = 0;
        let mut gc_paths: Vec<GcTarget> = Vec::new();

        for sha in &shas {
            sqlx::query(
                "UPDATE blobs SET refcount = refcount - 1 WHERE user_id = ? AND sha256 = ?",
            )
            .bind(&user_id)
            .bind(sha)
            .execute(&mut *tx)
            .await?;

            let remaining = sqlx::query(
                "SELECT refcount, size_bytes FROM blobs WHERE user_id = ? AND sha256 = ?",
            )
            .bind(&user_id)
            .bind(sha)
            .fetch_optional(&mut *tx)
            .await?;

            if let Some(r) = remaining {
                let rc: i64 = r.get("refcount");
                if rc <= 0 {
                    let size: i64 = r.get("size_bytes");
                    sqlx::query("DELETE FROM blobs WHERE user_id = ? AND sha256 = ?")
                        .bind(&user_id)
                        .bind(sha)
                        .execute(&mut *tx)
                        .await?;
                    freed_bytes += size;
                    gc_paths.push(GcTarget {
                        is_chunk: false,
                        sha: sha.clone(),
                        path: crate::blobs::blob_path(data_dir, &user_id, sha),
                    });
                }
            }
        }

        // Same dance for chunks: decrement, GC the chunk file + row at 0, and
        // refund the freed bytes. Done in the same tx as the blob pass so a
        // crash can't leave a chunk refcounted but unreferenced.
        for sha in &chunk_shas {
            sqlx::query(
                "UPDATE chunks SET refcount = refcount - 1 WHERE user_id = ? AND sha256 = ?",
            )
            .bind(&user_id)
            .bind(sha)
            .execute(&mut *tx)
            .await?;

            let remaining = sqlx::query(
                "SELECT refcount, size_bytes FROM chunks WHERE user_id = ? AND sha256 = ?",
            )
            .bind(&user_id)
            .bind(sha)
            .fetch_optional(&mut *tx)
            .await?;

            if let Some(r) = remaining {
                let rc: i64 = r.get("refcount");
                if rc <= 0 {
                    let size: i64 = r.get("size_bytes");
                    sqlx::query("DELETE FROM chunks WHERE user_id = ? AND sha256 = ?")
                        .bind(&user_id)
                        .bind(sha)
                        .execute(&mut *tx)
                        .await?;
                    freed_bytes += size;
                    gc_paths.push(GcTarget {
                        is_chunk: true,
                        sha: sha.clone(),
                        path: crate::chunking::chunk_path(data_dir, &user_id, sha),
                    });
                }
            }
        }

        // Deletes snapshot_files too via ON DELETE CASCADE.
        sqlx::query("DELETE FROM snapshots WHERE id = ?")
            .bind(&snap_id)
            .execute(&mut *tx)
            .await?;

        if freed_bytes > 0 {
            sqlx::query(
                "UPDATE users SET storage_used_bytes = MAX(0, storage_used_bytes - ?) WHERE id = ?",
            )
            .bind(freed_bytes)
            .bind(&user_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        // GC blob/chunk files only after the DB committed their removal — and
        // only if the row is still gone. A concurrent upload can re-reference an
        // object in the window between commit and unlink, re-creating its row
        // (refcount > 0) and re-writing the file; deleting it here would corrupt
        // that live object. Re-check per target and skip any that came back.
        for t in gc_paths {
            let table = if t.is_chunk { "chunks" } else { "blobs" };
            let q = format!("SELECT refcount FROM {table} WHERE user_id = ? AND sha256 = ?");
            let revived = sqlx::query(&q)
                .bind(&user_id)
                .bind(&t.sha)
                .fetch_optional(pool)
                .await?
                .map(|r| r.get::<i64, _>("refcount") > 0)
                .unwrap_or(false);
            if revived {
                continue;
            }
            let _ = tokio::fs::remove_file(&t.path).await;
        }
        // Legacy: drop any pre-migration trash folder if it still exists.
        let _ = tokio::fs::remove_dir_all(data_dir.join("trash").join(&snap_id)).await;
        removed += 1;
    }

    if removed > 0 {
        info!(removed, "purged trashed snapshots");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqliteConnectOptions;
    use std::str::FromStr;

    async fn mem_pool() -> SqlitePool {
        let opts = SqliteConnectOptions::from_str("sqlite::memory:")
            .unwrap()
            .pragma("foreign_keys", "ON");
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1) // in-memory DB lives in one connection
            .connect_with(opts)
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    async fn write_blob(data_dir: &Path, user: &str, sha: &str, bytes: &[u8]) {
        let p = crate::blobs::blob_path(data_dir, user, sha);
        tokio::fs::create_dir_all(p.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(&p, bytes).await.unwrap();
    }

    /// Purging a trashed snapshot decrements blob refcounts, GCs only the blobs
    /// that reach 0, refunds exactly the freed bytes, and leaves the surviving
    /// snapshot fully intact (ADR 0018, eje C).
    #[tokio::test]
    async fn purge_decrements_refcount_and_gcs_at_zero() {
        let pool = mem_pool().await;
        let tmp = std::env::temp_dir().join(format!("hoard-test-{}", uuid::Uuid::new_v4()));
        let data_dir = tmp.as_path();

        let sha_shared = "aa".to_string() + &"0".repeat(62); // refcount 2
        let sha_only1 = "bb".to_string() + &"0".repeat(62); // refcount 1, GC'd
        let sha_only2 = "cc".to_string() + &"0".repeat(62); // refcount 1, survives

        // Seed: user (used = 100+50+70), game, save, two snapshots, files, blobs.
        sqlx::query("INSERT INTO users (id, username, password_hash, storage_used_bytes) VALUES ('u1','user','x',220)")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO games (slug, display_name) VALUES ('g','G')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO saves (id, user_id, game_slug, label, latest_version_num) VALUES ('sv','u1','g','default',2)")
            .execute(&pool).await.unwrap();

        // s1 is trashed (deleted_at in the past); s2 is live.
        sqlx::query("INSERT INTO snapshots (id, save_id, version_num, total_size_bytes, file_count, deleted_at) VALUES ('s1','sv',1,150,2,'2000-01-01T00:00:00Z')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO snapshots (id, save_id, version_num, total_size_bytes, file_count) VALUES ('s2','sv',2,170,2)")
            .execute(&pool).await.unwrap();

        for (id, snap, sha, size) in [
            ("f1", "s1", &sha_shared, 100),
            ("f2", "s1", &sha_only1, 50),
            ("f3", "s2", &sha_shared, 100),
            ("f4", "s2", &sha_only2, 70),
        ] {
            sqlx::query("INSERT INTO snapshot_files (id, snapshot_id, relative_path, size_bytes, sha256) VALUES (?,?,?,?,?)")
                .bind(id).bind(snap).bind(id).bind(size).bind(sha)
                .execute(&pool).await.unwrap();
        }
        for (sha, size, rc) in [
            (&sha_shared, 100, 2),
            (&sha_only1, 50, 1),
            (&sha_only2, 70, 1),
        ] {
            sqlx::query(
                "INSERT INTO blobs (user_id, sha256, size_bytes, refcount) VALUES ('u1',?,?,?)",
            )
            .bind(sha)
            .bind(size)
            .bind(rc)
            .execute(&pool)
            .await
            .unwrap();
            write_blob(data_dir, "u1", sha, b"data").await;
        }

        purge_trash(&pool, data_dir, 0).await.unwrap();

        // s1 gone (cascade removed its files); s2 intact.
        let s1: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM snapshots WHERE id='s1'")
            .fetch_one(&pool)
            .await
            .unwrap();
        let s2: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM snapshots WHERE id='s2'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(s1, 0);
        assert_eq!(s2, 1);

        // Shared blob decremented to 1, still present; only-s1 blob GC'd.
        let rc_shared: i64 = sqlx::query_scalar("SELECT refcount FROM blobs WHERE sha256=?")
            .bind(&sha_shared)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(rc_shared, 1);
        let only1: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM blobs WHERE sha256=?")
            .bind(&sha_only1)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(only1, 0);

        // Disk: shared + only2 present, only1 file removed.
        assert!(crate::blobs::blob_path(data_dir, "u1", &sha_shared).exists());
        assert!(crate::blobs::blob_path(data_dir, "u1", &sha_only2).exists());
        assert!(!crate::blobs::blob_path(data_dir, "u1", &sha_only1).exists());

        // Quota refunded by exactly the 50 freed bytes (220 → 170).
        let used: i64 = sqlx::query_scalar("SELECT storage_used_bytes FROM users WHERE id='u1'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(used, 170);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    async fn write_chunk(data_dir: &Path, user: &str, sha: &str, bytes: &[u8]) {
        let p = crate::chunking::chunk_path(data_dir, user, sha);
        tokio::fs::create_dir_all(p.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(&p, bytes).await.unwrap();
    }

    /// The chunk store (ADR 0019, Fase 4) GCs exactly like blobs: purging a
    /// trashed chunked snapshot decrements each referenced chunk, GCs only the
    /// chunks that reach 0, refunds their bytes, and leaves chunks still pinned
    /// by the surviving snapshot intact. The whole-file sha on snapshot_files
    /// has no blob row, so the blob pass is a harmless no-op for chunked files.
    #[tokio::test]
    async fn purge_decrements_chunk_refcount_and_gcs_at_zero() {
        let pool = mem_pool().await;
        let tmp = std::env::temp_dir().join(format!("hoard-test-{}", uuid::Uuid::new_v4()));
        let data_dir = tmp.as_path();

        let c_shared = "aa".to_string() + &"1".repeat(62); // refcount 2
        let c_only1 = "bb".to_string() + &"1".repeat(62); // refcount 1, GC'd
        let c_only2 = "cc".to_string() + &"1".repeat(62); // refcount 1, survives
        let whole1 = "dd".to_string() + &"1".repeat(62); // f1 whole-file sha (no blob)
        let whole2 = "ee".to_string() + &"1".repeat(62); // f2 whole-file sha (no blob)

        sqlx::query("INSERT INTO users (id, username, password_hash, storage_used_bytes) VALUES ('u1','user','x',220)")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO games (slug, display_name) VALUES ('g','G')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO saves (id, user_id, game_slug, label, latest_version_num) VALUES ('sv','u1','g','default',2)")
            .execute(&pool).await.unwrap();

        // s1 trashed (past deleted_at); s2 live. Each has one chunked file.
        sqlx::query("INSERT INTO snapshots (id, save_id, version_num, total_size_bytes, file_count, deleted_at) VALUES ('s1','sv',1,150,1,'2000-01-01T00:00:00Z')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO snapshots (id, save_id, version_num, total_size_bytes, file_count) VALUES ('s2','sv',2,170,1)")
            .execute(&pool).await.unwrap();

        // Chunked files: snapshot_files row carries the whole-file sha (no blob),
        // the bytes live in snapshot_file_chunks.
        sqlx::query("INSERT INTO snapshot_files (id, snapshot_id, relative_path, size_bytes, sha256) VALUES ('f1','s1','save.dat',150,?)")
            .bind(&whole1).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO snapshot_files (id, snapshot_id, relative_path, size_bytes, sha256) VALUES ('f2','s2','save.dat',170,?)")
            .bind(&whole2).execute(&pool).await.unwrap();

        for (file_id, ord, sha) in [
            ("f1", 0, &c_shared),
            ("f1", 1, &c_only1),
            ("f2", 0, &c_shared),
            ("f2", 1, &c_only2),
        ] {
            sqlx::query("INSERT INTO snapshot_file_chunks (snapshot_file_id, ordinal, chunk_sha256) VALUES (?,?,?)")
                .bind(file_id).bind(ord as i64).bind(sha)
                .execute(&pool).await.unwrap();
        }
        for (sha, size, rc) in [(&c_shared, 100, 2), (&c_only1, 50, 1), (&c_only2, 70, 1)] {
            sqlx::query(
                "INSERT INTO chunks (user_id, sha256, size_bytes, refcount) VALUES ('u1',?,?,?)",
            )
            .bind(sha)
            .bind(size as i64)
            .bind(rc as i64)
            .execute(&pool)
            .await
            .unwrap();
            write_chunk(data_dir, "u1", sha, b"data").await;
        }

        purge_trash(&pool, data_dir, 0).await.unwrap();

        // s1 gone (cascade removed its files + chunk refs); s2 intact.
        let s1: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM snapshots WHERE id='s1'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(s1, 0);
        let sfc1: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM snapshot_file_chunks WHERE snapshot_file_id='f1'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(sfc1, 0, "chunk refs cascade-deleted with the purged file");

        // Shared chunk decremented to 1, still present; only-s1 chunk GC'd.
        let rc_shared: i64 = sqlx::query_scalar("SELECT refcount FROM chunks WHERE sha256=?")
            .bind(&c_shared)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(rc_shared, 1);
        let only1: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM chunks WHERE sha256=?")
            .bind(&c_only1)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(only1, 0);

        assert!(crate::chunking::chunk_path(data_dir, "u1", &c_shared).exists());
        assert!(crate::chunking::chunk_path(data_dir, "u1", &c_only2).exists());
        assert!(!crate::chunking::chunk_path(data_dir, "u1", &c_only1).exists());

        // Quota refunded by exactly the 50 freed bytes (220 → 170).
        let used: i64 = sqlx::query_scalar("SELECT storage_used_bytes FROM users WHERE id='u1'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(used, 170);

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
