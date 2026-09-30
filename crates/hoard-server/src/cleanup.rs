//! Background cleanup task: prunes redundant snapshots (ADR 0018, eje B)
//! and purges old tmp uploads, trashed snapshots and client logs.

use sqlx::{Row, SqlitePool};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use time::format_description::well_known::Rfc3339;
use tracing::{info, warn};

use crate::retention::{plan_prune, RetentionPolicy, SnapshotMeta};
use crate::store::BlobStore;

pub async fn run_periodic(
    pool: SqlitePool,
    data_dir: PathBuf,
    store: Arc<dyn BlobStore>,
    tmp_cleanup_hours: u64,
    trash_retention_days: u64,
    prune_policy: Option<RetentionPolicy>,
    repair: bool,
) {
    // Run every hour
    let mut interval = tokio::time::interval(Duration::from_secs(3600));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut last_repair: Option<tokio::time::Instant> = None;

    loop {
        interval.tick().await;
        // Once at startup and once a day after that. Ahead of the purges, so a
        // count the repair has just put right is the one they work with.
        if repair && last_repair.is_none_or(|at| at.elapsed() >= REPAIR_EVERY) {
            last_repair = Some(tokio::time::Instant::now());
            match repair_refcounts(&pool, &store, time::OffsetDateTime::now_utc()).await {
                Ok(report) => {
                    if report.users_skipped > 0 {
                        warn!(?report, "refcount repair left some users alone");
                    }
                    // More was due than one pass takes: back on the next tick
                    // instead of tomorrow.
                    if report.users_capped > 0 {
                        last_repair = None;
                    }
                }
                Err(e) => warn!(error = %e, "refcount repair error"),
            }
        }
        if let Err(e) = run_once(
            &pool,
            &data_dir,
            &store,
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
    store: &Arc<dyn BlobStore>,
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
    purge_trash(pool, data_dir, store, trash_retention_days).await?;
    purge_client_logs(pool, CLIENT_LOG_RETENTION_DAYS).await?;
    purge_dead_sessions(pool, SESSION_TOMBSTONE_DAYS).await?;
    // Forget machines that have not shown up in months, or the census
    // accumulates every laptop somebody used for one afternoon, forever.
    // Best-effort: this failing must not take down the rest of the cleanup.
    match crate::routes::devices::prune_stale(pool, DEVICE_RETENTION_DAYS).await {
        Ok(n) if n > 0 => info!(removed = n, "forgot devices with no recent activity"),
        Err(e) => warn!(error = %e, "device pruning error"),
        _ => {}
    }
    Ok(())
}

/// How long a dead browser session stays in `api_tokens` before the row goes.
///
/// Before the panel, nothing ever deleted a token row and that was right: they
/// are minted by hand, one per device, and a revoked one is audit trail. A
/// browser session is the opposite, minted on every sign-in and dead in two
/// weeks, so without this the table grows by one row per login forever. The
/// window is there so a "who logged in last month" question still has an
/// answer; only rows tagged as sessions are touched, never a device's token.
const SESSION_TOMBSTONE_DAYS: i64 = 30;

/// How long a device survives in the census without a sign of life. 90 days, the
/// same as cloud: long enough that a machine only switched on during the holidays
/// does not vanish, short enough that the listing still means something.
const DEVICE_RETENTION_DAYS: i64 = 90;

/// Age-weighted snapshot pruning (ADR 0018, eje B). For each save, decide
/// which live snapshots are redundant per `policy` and soft-delete them
/// (logical only: the bytes stay until the trash purge). Recoverable until
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
/// stay on disk with their blob refcounts intact: a trashed snapshot still
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

/// Drop browser sessions that expired or were revoked more than
/// `tombstone_days` ago. See [`SESSION_TOMBSTONE_DAYS`] for why this exists
/// only for sessions.
async fn purge_dead_sessions(pool: &SqlitePool, tombstone_days: i64) -> anyhow::Result<()> {
    let cutoff = (time::OffsetDateTime::now_utc() - time::Duration::days(tombstone_days))
        .format(&time::format_description::well_known::Rfc3339)?;
    let res = sqlx::query(
        "DELETE FROM api_tokens \
         WHERE device_name = ? \
           AND ((expires_at IS NOT NULL AND expires_at < ?) \
             OR (revoked_at IS NOT NULL AND revoked_at < ?))",
    )
    .bind(crate::routes::session::SESSION_DEVICE_NAME)
    .bind(&cutoff)
    .bind(&cutoff)
    .execute(pool)
    .await?;
    let removed = res.rows_affected();
    if removed > 0 {
        info!(removed, "purged dead browser sessions");
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

/// A content-addressed object scheduled for removal after the purge tx
/// commits. We keep the table + sha so we can re-check the refcount just before
/// unlinking: a concurrent upload may have re-referenced (and re-created) the
/// row in the gap between commit and the store delete, in which case removing
/// the object would corrupt a live blob/chunk. `key` is the storage-backend
/// key (same for local disk and S3, ADR 0020).
pub(crate) struct GcTarget {
    is_chunk: bool,
    sha: String,
    key: String,
}

/// GC blob/chunk objects only after the DB committed their removal, and only if
/// the row is still gone. A concurrent upload can re-reference an object in the
/// window between commit and delete, re-creating its row (refcount > 0) and
/// re-writing the object; deleting it here would corrupt that live object.
/// Re-check per target and skip any revived.
pub(crate) async fn unlink_released(
    pool: &SqlitePool,
    store: &Arc<dyn BlobStore>,
    user_id: &str,
    targets: Vec<GcTarget>,
) -> sqlx::Result<()> {
    for t in targets {
        let table = if t.is_chunk { "chunks" } else { "blobs" };
        let q = format!("SELECT refcount FROM {table} WHERE user_id = ? AND sha256 = ?");
        let revived = sqlx::query(&q)
            .bind(user_id)
            .bind(&t.sha)
            .fetch_optional(pool)
            .await?
            .map(|r| r.get::<i64, _>("refcount") > 0)
            .unwrap_or(false);
        if revived {
            continue;
        }
        if let Err(e) = store.delete(&t.key).await {
            warn!(key = %t.key, error = %e, "blob GC delete failed");
        }
    }
    Ok(())
}

/// What is being deleted, and so does not count as a reason to keep an object.
#[derive(Clone, Copy)]
enum Besides<'a> {
    Snapshot(&'a str),
    Save(&'a str),
    Nothing,
}

/// How many of the user's file rows use this object, not counting what is being
/// deleted. The refcount says what should be true; this asks the rows.
///
/// A whole-file sha counts for a blob only in a file stored whole: a chunked
/// file carries its sha too, and its bytes are in its chunks.
async fn references_left(
    conn: &mut sqlx::SqliteConnection,
    is_chunk: bool,
    user_id: &str,
    sha: &str,
    besides: Besides<'_>,
) -> sqlx::Result<i64> {
    let (extra, excluded) = match besides {
        Besides::Snapshot(id) => (" AND s.id != ?", Some(id)),
        Besides::Save(id) => (" AND s.save_id != ?", Some(id)),
        Besides::Nothing => ("", None),
    };
    let sql = if is_chunk {
        format!(
            "SELECT COUNT(*) FROM snapshot_file_chunks c
               JOIN snapshot_files sf ON sf.id = c.snapshot_file_id
               JOIN snapshots s ON s.id = sf.snapshot_id
               JOIN saves sv ON sv.id = s.save_id
              WHERE c.chunk_sha256 = ? AND sv.user_id = ?{extra}"
        )
    } else {
        format!(
            "SELECT COUNT(*) FROM snapshot_files sf
               JOIN snapshots s ON s.id = sf.snapshot_id
               JOIN saves sv ON sv.id = s.save_id
              WHERE sf.sha256 = ? AND sv.user_id = ?
                AND NOT EXISTS (SELECT 1 FROM snapshot_file_chunks c
                                 WHERE c.snapshot_file_id = sf.id){extra}"
        )
    };
    let mut q = sqlx::query_scalar::<_, i64>(&sql).bind(sha).bind(user_id);
    if let Some(id) = excluded {
        q = q.bind(id);
    }
    q.fetch_one(conn).await
}

/// The last line of defence before an object's row goes: a counter that has
/// run out is not proof that nothing uses it. If a version still does, the row
/// stays, with the count that is true, and the caller leaves the object alone.
/// A counter that was wrong then costs a warning instead of a save.
async fn keep_if_referenced(
    conn: &mut sqlx::SqliteConnection,
    is_chunk: bool,
    user_id: &str,
    sha: &str,
    besides: Besides<'_>,
) -> sqlx::Result<bool> {
    let left = references_left(conn, is_chunk, user_id, sha, besides).await?;
    if left == 0 {
        return Ok(false);
    }
    let table = if is_chunk { "chunks" } else { "blobs" };
    sqlx::query(&format!(
        "UPDATE {table} SET refcount = ? WHERE user_id = ? AND sha256 = ?"
    ))
    .bind(left)
    .bind(user_id)
    .bind(sha)
    .execute(conn)
    .await?;
    warn!(
        table,
        sha = %sha,
        references = left,
        "refcount ran out on an object versions still use; kept, and recounted"
    );
    Ok(true)
}

/// What deleting a save gave back: the bytes to refund, the objects to unlink
/// once the transaction is in, and the snapshots that went with it.
pub(crate) struct ReleasedSave {
    pub freed_bytes: i64,
    pub targets: Vec<GcTarget>,
    pub snapshot_ids: Vec<String>,
}

/// Give back every reference a save holds, live versions and trashed ones
/// alike, ahead of the `DELETE FROM saves` whose cascade takes the rows that
/// say what they were.
///
/// `DELETE /v1/saves/:id` used to go straight to that cascade. The snapshots
/// went, their `snapshot_files` went, and the `blobs` and `chunks` rows stayed
/// at the refcount they had: bytes nothing pointed at any more, that no purge
/// would ever reach, still counted against the user's quota.
///
/// One decrement per distinct sha, by as many references as the save held,
/// rather than one per reference as the trash purge does: a save is every
/// version of a game, and that is the same few thousand files a hundred times.
pub(crate) async fn release_save_refs(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    user_id: &str,
    save_id: &str,
) -> sqlx::Result<ReleasedSave> {
    let snapshot_ids: Vec<String> = sqlx::query("SELECT id FROM snapshots WHERE save_id = ?")
        .bind(save_id)
        .fetch_all(&mut **tx)
        .await?
        .iter()
        .map(|r| r.get::<String, _>("id"))
        .collect();

    let mut freed_bytes: i64 = 0;
    let mut targets: Vec<GcTarget> = Vec::new();
    for (table, is_chunk, refs_sql) in [
        (
            "blobs",
            false,
            "SELECT sf.sha256 AS sha, COUNT(*) AS n
               FROM snapshot_files sf
               JOIN snapshots s ON s.id = sf.snapshot_id
              WHERE s.save_id = ?
                AND NOT EXISTS (SELECT 1 FROM snapshot_file_chunks c
                                 WHERE c.snapshot_file_id = sf.id)
              GROUP BY sf.sha256",
        ),
        (
            "chunks",
            true,
            "SELECT sfc.chunk_sha256 AS sha, COUNT(*) AS n
               FROM snapshot_file_chunks sfc
               JOIN snapshot_files sf ON sf.id = sfc.snapshot_file_id
               JOIN snapshots s ON s.id = sf.snapshot_id
              WHERE s.save_id = ?
              GROUP BY sfc.chunk_sha256",
        ),
    ] {
        let refs: Vec<(String, i64)> = sqlx::query(refs_sql)
            .bind(save_id)
            .fetch_all(&mut **tx)
            .await?
            .iter()
            .map(|r| (r.get("sha"), r.get("n")))
            .collect();
        for (sha, n) in refs {
            sqlx::query(&format!(
                "UPDATE {table} SET refcount = refcount - ? WHERE user_id = ? AND sha256 = ?"
            ))
            .bind(n)
            .bind(user_id)
            .bind(&sha)
            .execute(&mut **tx)
            .await?;
            let left = sqlx::query(&format!(
                "SELECT refcount, size_bytes FROM {table} WHERE user_id = ? AND sha256 = ?"
            ))
            .bind(user_id)
            .bind(&sha)
            .fetch_optional(&mut **tx)
            .await?;
            let Some(left) = left else { continue };
            if left.get::<i64, _>("refcount") > 0 {
                continue;
            }
            let besides = Besides::Save(save_id);
            if keep_if_referenced(tx, is_chunk, user_id, &sha, besides).await? {
                continue;
            }
            sqlx::query(&format!(
                "DELETE FROM {table} WHERE user_id = ? AND sha256 = ?"
            ))
            .bind(user_id)
            .bind(&sha)
            .execute(&mut **tx)
            .await?;
            freed_bytes += left.get::<i64, _>("size_bytes");
            let key = if is_chunk {
                crate::store::chunk_key(user_id, &sha)
            } else {
                crate::store::blob_key(user_id, &sha)
            };
            targets.push(GcTarget { is_chunk, sha, key });
        }
    }

    Ok(ReleasedSave {
        freed_bytes,
        targets,
        snapshot_ids,
    })
}

/// One user's stored objects whose refcount does not match what their
/// snapshots reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefcountDrift {
    pub username: String,
    /// Rows nothing references: bytes on disk and in the quota that no purge
    /// will reach. What deleting a save left behind before it gave its
    /// references back.
    pub unreferenced_objects: i64,
    pub unreferenced_bytes: i64,
    /// How many of those the repair has already seen and dated: they go a week
    /// after that, unless a version uses them again first.
    pub marked_objects: i64,
    /// Referenced, but counted higher than the references there are: they
    /// outlive their last snapshot.
    pub overcounted_objects: i64,
    /// Counted lower than the references there are. The dangerous direction:
    /// the purge would free one while a snapshot still needs it.
    pub undercounted_objects: i64,
}

/// Compare every `blobs` and `chunks` refcount with the references that exist.
/// Read-only; users with nothing to report are left out.
pub async fn audit_refcounts(pool: &SqlitePool) -> sqlx::Result<Vec<RefcountDrift>> {
    let mut by_user: std::collections::BTreeMap<String, RefcountDrift> = Default::default();
    for (table, refs_sql) in [
        (
            "blobs",
            "SELECT sv.user_id AS user_id, sf.sha256 AS sha, COUNT(*) AS n
               FROM snapshot_files sf
               JOIN snapshots s ON s.id = sf.snapshot_id
               JOIN saves sv ON sv.id = s.save_id
              WHERE NOT EXISTS (SELECT 1 FROM snapshot_file_chunks c
                                 WHERE c.snapshot_file_id = sf.id)
              GROUP BY sv.user_id, sf.sha256",
        ),
        (
            "chunks",
            "SELECT sv.user_id AS user_id, sfc.chunk_sha256 AS sha, COUNT(*) AS n
               FROM snapshot_file_chunks sfc
               JOIN snapshot_files sf ON sf.id = sfc.snapshot_file_id
               JOIN snapshots s ON s.id = sf.snapshot_id
               JOIN saves sv ON sv.id = s.save_id
              GROUP BY sv.user_id, sfc.chunk_sha256",
        ),
    ] {
        let rows = sqlx::query(&format!(
            "WITH refs AS ({refs_sql})
             SELECT u.username AS username,
                    SUM(r.n IS NULL) AS unreferenced,
                    SUM(CASE WHEN r.n IS NULL THEN o.size_bytes ELSE 0 END) AS unreferenced_bytes,
                    SUM(r.n IS NULL AND o.unreferenced_since IS NOT NULL) AS marked,
                    SUM(r.n IS NOT NULL AND o.refcount > r.n) AS overcounted,
                    SUM(r.n IS NOT NULL AND o.refcount < r.n) AS undercounted
               FROM {table} o
               JOIN users u ON u.id = o.user_id
               LEFT JOIN refs r ON r.user_id = o.user_id AND r.sha = o.sha256
              GROUP BY u.username"
        ))
        .fetch_all(pool)
        .await?;
        for r in &rows {
            let username: String = r.get("username");
            let entry = by_user
                .entry(username.clone())
                .or_insert_with(|| RefcountDrift {
                    username,
                    unreferenced_objects: 0,
                    unreferenced_bytes: 0,
                    marked_objects: 0,
                    overcounted_objects: 0,
                    undercounted_objects: 0,
                });
            entry.unreferenced_objects += r.get::<i64, _>("unreferenced");
            entry.unreferenced_bytes += r.get::<i64, _>("unreferenced_bytes");
            entry.marked_objects += r.get::<i64, _>("marked");
            entry.overcounted_objects += r.get::<i64, _>("overcounted");
            entry.undercounted_objects += r.get::<i64, _>("undercounted");
        }
    }
    Ok(by_user
        .into_values()
        .filter(|d| {
            d.unreferenced_objects > 0 || d.overcounted_objects > 0 || d.undercounted_objects > 0
        })
        .collect())
}

const REPAIR_EVERY: Duration = Duration::from_secs(24 * 3600);
/// An unused object younger than this is not even dated: whatever wrote it may
/// not be done with it, and the leaks this is after are months old.
const REPAIR_MIN_AGE: time::Duration = time::Duration::hours(24);
/// From the day the repair dates an unused object to the day it deletes it.
const REPAIR_GRACE: time::Duration = time::Duration::days(7);
/// The most objects one pass deletes for one user. The user's lock is held
/// while they are unlinked, and on an S3 backend that is a request each: a
/// leak of 50,000 objects taken in one go would hold that user's uploads for
/// the better part of an hour. It also bounds what a kill between the commit
/// and the last unlink can strand.
const REPAIR_BATCH: i64 = 2000;

/// What one pass of [`repair_refcounts`] did.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RepairReport {
    /// Counters set to the number of references that exist.
    pub counts_fixed: u64,
    /// Dated as unused on an earlier pass, used again since.
    pub marks_cleared: u64,
    /// Unused objects dated on this pass. Nothing is deleted yet.
    pub marked_objects: u64,
    pub marked_bytes: i64,
    /// Dated at least [`REPAIR_GRACE`] ago and still unused: deleted.
    pub freed_objects: u64,
    pub freed_bytes: i64,
    /// Shas a version references that have no row at all. Nothing here can
    /// mend that; it is reported.
    pub dangling: u64,
    /// Users whose pass was abandoned, with nothing of theirs changed.
    pub users_skipped: u64,
    /// Users with more due than [`REPAIR_BATCH`]: the rest goes on the next pass.
    pub users_capped: u64,
}

struct RepairKind {
    table: &'static str,
    is_chunk: bool,
    refs: &'static str,
    /// sha and how many of the user's file rows use it.
    refs_sql: &'static str,
    /// The same rows counted another way, with no join to fan them out.
    rows_sql: &'static str,
}

const REPAIR_KINDS: [RepairKind; 2] = [
    RepairKind {
        table: "blobs",
        is_chunk: false,
        refs: "repair_refs_blobs",
        refs_sql: "SELECT sf.sha256, COUNT(*)
                     FROM snapshot_files sf
                     JOIN snapshots s ON s.id = sf.snapshot_id
                     JOIN saves sv ON sv.id = s.save_id
                    WHERE sv.user_id = ?
                      AND NOT EXISTS (SELECT 1 FROM snapshot_file_chunks c
                                       WHERE c.snapshot_file_id = sf.id)
                    GROUP BY sf.sha256",
        rows_sql: "SELECT COUNT(*) FROM snapshot_files
                    WHERE snapshot_id IN (SELECT id FROM snapshots WHERE save_id IN
                                           (SELECT id FROM saves WHERE user_id = ?))
                      AND id NOT IN (SELECT snapshot_file_id FROM snapshot_file_chunks)",
    },
    RepairKind {
        table: "chunks",
        is_chunk: true,
        refs: "repair_refs_chunks",
        refs_sql: "SELECT c.chunk_sha256, COUNT(*)
                     FROM snapshot_file_chunks c
                     JOIN snapshot_files sf ON sf.id = c.snapshot_file_id
                     JOIN snapshots s ON s.id = sf.snapshot_id
                     JOIN saves sv ON sv.id = s.save_id
                    WHERE sv.user_id = ?
                    GROUP BY c.chunk_sha256",
        rows_sql: "SELECT COUNT(*) FROM snapshot_file_chunks
                    WHERE snapshot_file_id IN (SELECT id FROM snapshot_files WHERE snapshot_id IN
                                                (SELECT id FROM snapshots WHERE save_id IN
                                                  (SELECT id FROM saves WHERE user_id = ?)))",
    },
];

/// A timestamp in the shape SQLite's `strftime` default writes `created_at`
/// in, so the two compare as text.
fn stamp(t: time::OffsetDateTime) -> anyhow::Result<String> {
    Ok(t.replace_nanosecond(0)?.format(&Rfc3339)?)
}

/// Put every refcount back to the number of references that exist, and free
/// what nothing uses, slowly.
///
/// For servers that deleted saves before that gave their references back (see
/// [`release_save_refs`]): their `blobs` and `chunks` carry counters that are
/// too high, some on objects no version uses at all, and nothing else would
/// ever bring them down. It needs no command and no operator: it runs with the
/// cleanup, and on a server with nothing wrong it changes nothing.
///
/// It deletes stored bytes on somebody else's server, unasked, so it is built
/// to be hard to get wrong:
///
/// - What is in use is read from the file rows of every version, live or in
///   the trash, under the user's lock, so no upload or purge moves under it.
/// - The references are counted twice, by two differently shaped queries. If
///   they disagree the user is left untouched.
/// - An unused object is not deleted when found. It is dated, and deleted by
///   the first pass that finds it still unused [`REPAIR_GRACE`] later. One
///   younger than [`REPAIR_MIN_AGE`] is not even dated.
/// - Before each row goes it is asked for once more, on its own, through the
///   index. A single reference found there abandons the user's pass.
/// - An object a version uses again meanwhile (an upload that reuses it) has
///   its date cleared and its counter set right.
/// - A counter that is lowered is covered by [`keep_if_referenced`]: no purge
///   deletes on a counter alone.
/// - The file goes only after the transaction is in, and only if the row is
///   still gone ([`unlink_released`]).
/// - `[retention] repair_refcounts = false` turns it off.
pub async fn repair_refcounts(
    pool: &SqlitePool,
    store: &Arc<dyn BlobStore>,
    now: time::OffsetDateTime,
) -> anyhow::Result<RepairReport> {
    let users = sqlx::query("SELECT id, username FROM users")
        .fetch_all(pool)
        .await?;
    let mut total = RepairReport::default();
    for u in &users {
        let user_id: String = u.get("id");
        let username: String = u.get("username");
        let r = match repair_user(pool, store, &user_id, now, REPAIR_BATCH).await {
            Ok(r) => r,
            Err(e) => {
                warn!(user = %username, error = %e, "refcount repair: left this user untouched");
                total.users_skipped += 1;
                continue;
            }
        };
        if r.counts_fixed > 0 || r.marks_cleared > 0 {
            info!(
                user = %username,
                counts_fixed = r.counts_fixed,
                back_in_use = r.marks_cleared,
                "refcount repair: counters set to the references that exist"
            );
        }
        if r.marked_objects > 0 {
            info!(
                user = %username,
                objects = r.marked_objects,
                bytes = r.marked_bytes,
                grace_days = REPAIR_GRACE.whole_days(),
                "refcount repair: found stored objects no version uses; they are deleted after the grace period unless something uses them again"
            );
        }
        if r.freed_objects > 0 {
            info!(
                user = %username,
                objects = r.freed_objects,
                bytes = r.freed_bytes,
                "refcount repair: deleted objects no version has used since they were found"
            );
        }
        if r.dangling > 0 {
            warn!(
                user = %username,
                objects = r.dangling,
                "refcount repair: versions reference objects the server has no record of"
            );
        }
        total.counts_fixed += r.counts_fixed;
        total.marks_cleared += r.marks_cleared;
        total.marked_objects += r.marked_objects;
        total.marked_bytes += r.marked_bytes;
        total.freed_objects += r.freed_objects;
        total.freed_bytes += r.freed_bytes;
        total.dangling += r.dangling;
        total.users_capped += r.users_capped;
    }
    Ok(total)
}

async fn repair_user(
    pool: &SqlitePool,
    store: &Arc<dyn BlobStore>,
    user_id: &str,
    now: time::OffsetDateTime,
    batch: i64,
) -> anyhow::Result<RepairReport> {
    let _objects = crate::blobs::lock_user(user_id).await;
    // One connection for the whole pass: the reference tables are temporary,
    // and a temporary table belongs to the connection that made it.
    let mut conn = pool.acquire().await?;
    let outcome = repair_user_on(&mut conn, user_id, now, batch).await;
    for kind in &REPAIR_KINDS {
        let _ = sqlx::query(&format!("DROP TABLE IF EXISTS temp.{}", kind.refs))
            .execute(&mut *conn)
            .await;
    }
    // Back to the pool before the unlinks ask it for one.
    drop(conn);
    let (report, targets) = outcome?;
    unlink_released(pool, store, user_id, targets).await?;
    Ok(report)
}

async fn repair_user_on(
    conn: &mut sqlx::SqliteConnection,
    user_id: &str,
    now: time::OffsetDateTime,
    batch: i64,
) -> anyhow::Result<(RepairReport, Vec<GcTarget>)> {
    use sqlx::Connection as _;

    let now_s = stamp(now)?;
    let old_enough = stamp(now - REPAIR_MIN_AGE)?;
    let grace_over = stamp(now - REPAIR_GRACE)?;

    // The counting is the slow part and it only reads, so it is done before
    // the write lock is taken: the user's lock already keeps their rows still,
    // and the other users' uploads are not made to wait behind a table scan.
    for kind in &REPAIR_KINDS {
        let refs = kind.refs;
        sqlx::query(&format!("DROP TABLE IF EXISTS temp.{refs}"))
            .execute(&mut *conn)
            .await?;
        sqlx::query(&format!(
            "CREATE TEMP TABLE {refs} (sha TEXT PRIMARY KEY, n INTEGER NOT NULL) WITHOUT ROWID"
        ))
        .execute(&mut *conn)
        .await?;
        sqlx::query(&format!(
            "INSERT INTO temp.{refs} (sha, n) {}",
            kind.refs_sql
        ))
        .bind(user_id)
        .execute(&mut *conn)
        .await?;

        let by_sha: i64 =
            sqlx::query_scalar(&format!("SELECT COALESCE(SUM(n), 0) FROM temp.{refs}"))
                .fetch_one(&mut *conn)
                .await?;
        let by_row: i64 = sqlx::query_scalar(kind.rows_sql)
            .bind(user_id)
            .fetch_one(&mut *conn)
            .await?;
        if by_sha != by_row {
            anyhow::bail!(
                "{}: {by_sha} references counted by sha, {by_row} counted by row",
                kind.table
            );
        }
    }

    let mut out = RepairReport::default();
    let mut targets: Vec<GcTarget> = Vec::new();
    let mut tx = conn.begin_with("BEGIN IMMEDIATE").await?;

    for kind in &REPAIR_KINDS {
        let (table, refs) = (kind.table, kind.refs);

        let fixed = sqlx::query(&format!(
            "UPDATE {table}
                SET refcount = (SELECT n FROM temp.{refs} r WHERE r.sha = {table}.sha256)
              WHERE user_id = ?
                AND sha256 IN (SELECT sha FROM temp.{refs})
                AND refcount != (SELECT n FROM temp.{refs} r WHERE r.sha = {table}.sha256)"
        ))
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
        out.counts_fixed += fixed.rows_affected();

        let cleared = sqlx::query(&format!(
            "UPDATE {table} SET unreferenced_since = NULL
              WHERE user_id = ? AND unreferenced_since IS NOT NULL
                AND sha256 IN (SELECT sha FROM temp.{refs})"
        ))
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
        out.marks_cleared += cleared.rows_affected();

        // Dated on an earlier pass, the grace over, and still in nobody's list.
        let due = sqlx::query(&format!(
            "SELECT sha256, size_bytes FROM {table}
              WHERE user_id = ? AND unreferenced_since IS NOT NULL
                AND unreferenced_since <= ? AND created_at <= ?
                AND sha256 NOT IN (SELECT sha FROM temp.{refs})
              LIMIT ?"
        ))
        .bind(user_id)
        .bind(&grace_over)
        .bind(&old_enough)
        .bind(batch - out.freed_objects as i64 + 1)
        .fetch_all(&mut *tx)
        .await?;
        for row in &due {
            if out.freed_objects as i64 >= batch {
                out.users_capped = 1;
                break;
            }
            let sha: String = row.get("sha256");
            let left =
                references_left(&mut tx, kind.is_chunk, user_id, &sha, Besides::Nothing).await?;
            if left != 0 {
                anyhow::bail!(
                    "{table} {sha}: unused by one count, {left} reference(s) by the other"
                );
            }
            let gone = sqlx::query(&format!(
                "DELETE FROM {table}
                  WHERE user_id = ? AND sha256 = ? AND unreferenced_since IS NOT NULL"
            ))
            .bind(user_id)
            .bind(&sha)
            .execute(&mut *tx)
            .await?;
            if gone.rows_affected() != 1 {
                continue;
            }
            out.freed_objects += 1;
            out.freed_bytes += row.get::<i64, _>("size_bytes");
            let key = if kind.is_chunk {
                crate::store::chunk_key(user_id, &sha)
            } else {
                crate::store::blob_key(user_id, &sha)
            };
            targets.push(GcTarget {
                is_chunk: kind.is_chunk,
                sha,
                key,
            });
        }

        let unused = format!(
            "FROM {table}
              WHERE user_id = ? AND unreferenced_since IS NULL AND created_at <= ?
                AND sha256 NOT IN (SELECT sha FROM temp.{refs})"
        );
        let (objects, bytes): (i64, i64) = sqlx::query_as(&format!(
            "SELECT COUNT(*), COALESCE(SUM(size_bytes), 0) {unused}"
        ))
        .bind(user_id)
        .bind(&old_enough)
        .fetch_one(&mut *tx)
        .await?;
        if objects > 0 {
            sqlx::query(&format!(
                "UPDATE {table} SET unreferenced_since = ?
                  WHERE user_id = ? AND unreferenced_since IS NULL AND created_at <= ?
                    AND sha256 NOT IN (SELECT sha FROM temp.{refs})"
            ))
            .bind(&now_s)
            .bind(user_id)
            .bind(&old_enough)
            .execute(&mut *tx)
            .await?;
            out.marked_objects += objects as u64;
            out.marked_bytes += bytes;
        }

        let dangling: i64 = sqlx::query_scalar(&format!(
            "SELECT COUNT(*) FROM temp.{refs} r
              WHERE NOT EXISTS (SELECT 1 FROM {table} o
                                 WHERE o.user_id = ? AND o.sha256 = r.sha)"
        ))
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;
        out.dangling += dangling as u64;
    }

    if out.freed_bytes > 0 {
        sqlx::query(
            "UPDATE users SET storage_used_bytes = MAX(0, storage_used_bytes - ?) WHERE id = ?",
        )
        .bind(out.freed_bytes)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok((out, targets))
}

/// Permanently delete snapshots that have outlived the trash window. With the
/// blob store (ADR 0018, eje C) this is where bytes actually get freed: each
/// purged snapshot decrements the refcount of every blob it referenced, and a
/// blob that reaches 0 is GC'd (row + file deleted, owner quota refunded).
async fn purge_trash(
    pool: &SqlitePool,
    data_dir: &Path,
    store: &Arc<dyn BlobStore>,
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

        // Held from before the transaction to the last unlink below, so no
        // commit can reuse one of these objects, or place it again, in between,
        // and no restore from the trash can bring the snapshot back midway.
        let _objects = crate::blobs::lock_user(&user_id).await;
        // IMMEDIATE because it reads first: a deferred transaction that reads
        // and then writes fails outright in WAL mode if another user's commit
        // landed in between, instead of waiting on the busy timeout.
        let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
        // Read again under the lock: the user may have restored it from the
        // trash while the purge waited behind a commit.
        let still_due = sqlx::query(
            "SELECT 1 FROM snapshots WHERE id = ? AND deleted_at IS NOT NULL AND deleted_at < ?",
        )
        .bind(&snap_id)
        .bind(&cutoff_str)
        .fetch_optional(&mut *tx)
        .await?
        .is_some();
        if !still_due {
            info!(snapshot = %snap_id, "trash purge: snapshot restored meanwhile, kept");
            continue;
        }

        // The whole-file shas this snapshot referenced (one row per file, dups
        // included so the refcount decrement matches the increment from
        // `create`). Chunked files have no blob row, so their decrement below
        // is a harmless no-op, since their bytes are freed via the chunk pass.
        let shas: Vec<String> =
            sqlx::query("SELECT sha256 FROM snapshot_files WHERE snapshot_id = ?")
                .bind(&snap_id)
                .fetch_all(&mut *tx)
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
        .fetch_all(&mut *tx)
        .await?
        .iter()
        .map(|r| r.get::<String, _>("sha"))
        .collect();

        let mut freed_bytes: i64 = 0;
        let mut gc_paths: Vec<GcTarget> = Vec::new();
        // Objects whose counter ran out while other versions still use them:
        // set to the count that is true, and not to be decremented again for
        // this snapshot's remaining rows.
        let mut recounted: std::collections::HashSet<&str> = Default::default();

        for sha in &shas {
            if recounted.contains(sha.as_str()) {
                continue;
            }
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
                    let besides = Besides::Snapshot(&snap_id);
                    if keep_if_referenced(&mut tx, false, &user_id, sha, besides).await? {
                        recounted.insert(sha);
                        continue;
                    }
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
                        key: crate::store::blob_key(&user_id, sha),
                    });
                }
            }
        }

        // Same dance for chunks: decrement, GC the chunk file + row at 0, and
        // refund the freed bytes. Done in the same tx as the blob pass so a
        // crash can't leave a chunk refcounted but unreferenced.
        for sha in &chunk_shas {
            if recounted.contains(sha.as_str()) {
                continue;
            }
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
                    let besides = Besides::Snapshot(&snap_id);
                    if keep_if_referenced(&mut tx, true, &user_id, sha, besides).await? {
                        recounted.insert(sha);
                        continue;
                    }
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
                        key: crate::store::chunk_key(&user_id, sha),
                    });
                }
            }
        }

        // Deletes snapshot_files too via ON DELETE CASCADE. Under the same
        // condition as the check above: if it no longer holds, the decrements
        // go back with the transaction.
        let purged = sqlx::query(
            "DELETE FROM snapshots WHERE id = ? AND deleted_at IS NOT NULL AND deleted_at < ?",
        )
        .bind(&snap_id)
        .bind(&cutoff_str)
        .execute(&mut *tx)
        .await?;
        if purged.rows_affected() != 1 {
            warn!(snapshot = %snap_id, "trash purge: snapshot changed under the purge, left alone");
            continue;
        }

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

        unlink_released(pool, store, &user_id, gc_paths).await?;
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

    /// The repair over chunks as well as blobs: a counter that is too high on a
    /// chunk a version uses is set right, and a chunk and a blob nothing uses
    /// are dated on one pass and deleted by the pass a week later, with the
    /// quota they held. The chunks of the live version are not touched.
    #[tokio::test]
    async fn repair_covers_chunks_and_blobs_alike() {
        let pool = mem_pool().await;
        let tmp = std::env::temp_dir().join(format!("hoard-test-{}", uuid::Uuid::new_v4()));
        let data_dir = tmp.as_path();

        let c_shared = "aa".to_string() + &"2".repeat(62);
        let c_own = "bb".to_string() + &"2".repeat(62);
        let c_leak = "cc".to_string() + &"2".repeat(62);
        let b_leak = "dd".to_string() + &"2".repeat(62);
        let whole = "ee".to_string() + &"2".repeat(62);

        sqlx::query("INSERT INTO users (id, username, password_hash, storage_used_bytes) VALUES ('u1','user','x',250)")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO games (slug, display_name) VALUES ('g','G')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO saves (id, user_id, game_slug, label, latest_version_num) VALUES ('sv','u1','g','default',1)")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO snapshots (id, save_id, version_num, total_size_bytes, file_count) VALUES ('s1','sv',1,170,1)")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO snapshot_files (id, snapshot_id, relative_path, size_bytes, sha256) VALUES ('f1','s1','save.dat',170,?)")
            .bind(&whole).execute(&pool).await.unwrap();
        for (ord, sha) in [(0, &c_shared), (1, &c_own)] {
            sqlx::query("INSERT INTO snapshot_file_chunks (snapshot_file_id, ordinal, chunk_sha256) VALUES ('f1',?,?)")
                .bind(ord as i64).bind(sha)
                .execute(&pool).await.unwrap();
        }
        // The shared chunk still carries the references of a save deleted long
        // ago; the leaked chunk and blob were only ever that save's.
        for (sha, size, rc) in [(&c_shared, 100, 5), (&c_own, 70, 1), (&c_leak, 50, 3)] {
            sqlx::query(
                "INSERT INTO chunks (user_id, sha256, size_bytes, refcount, created_at) VALUES ('u1',?,?,?,'2020-01-01T00:00:00Z')",
            )
            .bind(sha)
            .bind(size as i64)
            .bind(rc as i64)
            .execute(&pool)
            .await
            .unwrap();
            write_chunk(data_dir, "u1", sha, b"data").await;
        }
        sqlx::query(
            "INSERT INTO blobs (user_id, sha256, size_bytes, refcount, created_at) VALUES ('u1',?,30,2,'2020-01-01T00:00:00Z')",
        )
        .bind(&b_leak)
        .execute(&pool)
        .await
        .unwrap();
        write_blob(data_dir, "u1", &b_leak, b"data").await;

        let store: Arc<dyn BlobStore> =
            Arc::new(crate::store::LocalFs::new(data_dir.to_path_buf()));
        let now = time::OffsetDateTime::now_utc();

        let first = repair_refcounts(&pool, &store, now).await.unwrap();
        assert_eq!(
            first,
            RepairReport {
                counts_fixed: 1,
                marked_objects: 2,
                marked_bytes: 80,
                ..Default::default()
            }
        );
        assert!(crate::chunking::chunk_path(data_dir, "u1", &c_leak).exists());

        let later = repair_refcounts(&pool, &store, now + time::Duration::days(8))
            .await
            .unwrap();
        assert_eq!(
            later,
            RepairReport {
                freed_objects: 2,
                freed_bytes: 80,
                ..Default::default()
            }
        );
        let left: Vec<(String, i64)> =
            sqlx::query_as("SELECT sha256, refcount FROM chunks ORDER BY sha256")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(left, vec![(c_shared.clone(), 1), (c_own.clone(), 1)]);
        let blobs: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM blobs")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(blobs, 0);
        assert!(!crate::chunking::chunk_path(data_dir, "u1", &c_leak).exists());
        assert!(!crate::blobs::blob_path(data_dir, "u1", &b_leak).exists());
        assert!(crate::chunking::chunk_path(data_dir, "u1", &c_shared).exists());
        assert!(crate::chunking::chunk_path(data_dir, "u1", &c_own).exists());
        let used: i64 = sqlx::query_scalar("SELECT storage_used_bytes FROM users WHERE id='u1'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(used, 170);
        let _ = tokio::fs::remove_dir_all(&tmp).await;
    }

    /// More due than one pass takes: it deletes its batch, says so, and the
    /// next pass takes the rest.
    #[tokio::test]
    async fn a_pass_stops_at_its_batch_and_the_next_one_goes_on() {
        let pool = mem_pool().await;
        let tmp = std::env::temp_dir().join(format!("hoard-test-{}", uuid::Uuid::new_v4()));
        let data_dir = tmp.as_path();
        sqlx::query("INSERT INTO users (id, username, password_hash, storage_used_bytes) VALUES ('u1','user','x',30)")
            .execute(&pool).await.unwrap();
        let shas: Vec<String> = ["aa", "bb", "cc"]
            .iter()
            .map(|p| p.to_string() + &"3".repeat(62))
            .collect();
        for sha in &shas {
            sqlx::query(
                "INSERT INTO blobs (user_id, sha256, size_bytes, refcount, created_at, unreferenced_since)
                 VALUES ('u1',?,10,1,'2020-01-01T00:00:00Z','2020-02-01T00:00:00Z')",
            )
            .bind(sha)
            .execute(&pool)
            .await
            .unwrap();
            write_blob(data_dir, "u1", sha, b"data").await;
        }
        let store: Arc<dyn BlobStore> =
            Arc::new(crate::store::LocalFs::new(data_dir.to_path_buf()));
        let now = time::OffsetDateTime::now_utc();

        let first = repair_user(&pool, &store, "u1", now, 2).await.unwrap();
        assert_eq!((first.freed_objects, first.users_capped), (2, 1));
        let second = repair_user(&pool, &store, "u1", now, 2).await.unwrap();
        assert_eq!((second.freed_objects, second.users_capped), (1, 0));

        let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM blobs")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(left, 0);
        assert!(shas
            .iter()
            .all(|sha| !crate::blobs::blob_path(data_dir, "u1", sha).exists()));
        let _ = tokio::fs::remove_dir_all(&tmp).await;
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

        let store: Arc<dyn BlobStore> =
            Arc::new(crate::store::LocalFs::new(data_dir.to_path_buf()));
        purge_trash(&pool, data_dir, &store, 0).await.unwrap();

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

    /// The user restores a snapshot from the trash while the purge, having
    /// listed it as due, waits for the user's lock behind a commit. Once the
    /// purge gets in, the snapshot is no longer trashed and nothing of it goes.
    #[tokio::test]
    async fn purge_leaves_a_snapshot_restored_while_it_waited() {
        let pool = mem_pool().await;
        let tmp = std::env::temp_dir().join(format!("hoard-test-{}", uuid::Uuid::new_v4()));
        let data_dir = tmp.clone();
        let sha = "dd".to_string() + &"0".repeat(62);
        sqlx::query("INSERT INTO users (id, username, password_hash, storage_used_bytes) VALUES ('u-restore','user','x',10)")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO games (slug, display_name) VALUES ('g','G')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO saves (id, user_id, game_slug, label, latest_version_num) VALUES ('sv','u-restore','g','default',1)")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO snapshots (id, save_id, version_num, total_size_bytes, file_count, deleted_at) VALUES ('s1','sv',1,10,1,'2000-01-01T00:00:00Z')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO snapshot_files (id, snapshot_id, relative_path, size_bytes, sha256) VALUES ('f1','s1','a.sav',10,?)")
            .bind(&sha)
            .execute(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO blobs (user_id, sha256, size_bytes, refcount) VALUES ('u-restore',?,10,1)",
        )
        .bind(&sha)
        .execute(&pool)
        .await
        .unwrap();
        write_blob(&data_dir, "u-restore", &sha, b"data").await;

        let held = crate::blobs::lock_user("u-restore").await;
        let purge = {
            let (pool, data_dir) = (pool.clone(), data_dir.clone());
            let store: Arc<dyn BlobStore> = Arc::new(crate::store::LocalFs::new(data_dir.clone()));
            tokio::spawn(async move { purge_trash(&pool, &data_dir, &store, 0).await })
        };
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        sqlx::query("UPDATE snapshots SET deleted_at=NULL WHERE id='s1'")
            .execute(&pool)
            .await
            .unwrap();
        drop(held);
        purge.await.unwrap().unwrap();

        let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM snapshots WHERE id='s1'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(left, 1, "the restored snapshot was purged");
        let rc: i64 = sqlx::query_scalar("SELECT refcount FROM blobs WHERE sha256=?")
            .bind(&sha)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(rc, 1);
        assert!(crate::blobs::blob_path(&data_dir, "u-restore", &sha).exists());
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

        let store: Arc<dyn BlobStore> =
            Arc::new(crate::store::LocalFs::new(data_dir.to_path_buf()));
        purge_trash(&pool, data_dir, &store, 0).await.unwrap();

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
