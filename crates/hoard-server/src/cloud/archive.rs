//! Archived games: the "black box" for saves that can't fit the plan.
//!
//! When a user's *live* footprint exceeds their storage limit and purging old
//! versions wouldn't bring it under (the current saves alone are too big, e.g.
//! a Pro→Free downgrade), deleting history frees nothing useful and just
//! destroys it. Instead the desktop offers to *archive* the heaviest games.
//!
//! Archiving a save (see [`archive_save`]):
//!   1. Marks `saves.archived_at`, and the client then excludes it from every sync
//!      path and the sync manifest omits it, so the cloud-side change can never
//!      propagate a delete to the user's local disk.
//!   2. De-references its content-addressed blobs. Each blob whose last live
//!      reference goes away hits refcount 0, which credits the freed space via
//!      the existing `sync_blob_storage` trigger, so the quota drops *now* and
//!      sync resumes for everything else, but instead of deleting the R2 object
//!      we stamp `cloud_blobs.purge_after` and keep it, so the save stays
//!      downloadable during the grace window.
//!   3. A daily cron ([`purge_expired`]) hard-deletes archived saves and their
//!      frozen blobs once the window elapses.
//!
//! Reactivating ([`reactivate_save`]) before the window elapses re-references
//! the blobs (clearing `purge_after`) and clears `archived_at`, subject to the
//! quota still fitting, typically done right after upgrading to Pro.
//!
//! This mirrors the account soft-delete + `account_purge` pattern, scoped to a
//! single game.

use super::incidents::{self, Kind};
use crate::cloud::auth::CloudUser;
use crate::cloud::blob_refs;
use crate::cloud::errors::CloudError;
use crate::cloud::quota;
use crate::cloud::state::CloudState;
use axum::{
    extract::{Path, State},
    response::{IntoResponse, Json, Response},
    Extension,
};
use futures::StreamExt;
use serde::Serialize;
use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;

/// Grace window (days) an archived game is frozen: downloadable, out of quota,
/// before the cron hard-deletes it and its blobs. Used symmetrically when
/// stamping `purge_after` on archive and when selecting due rows in the cron.
pub const ARCHIVE_GRACE_DAYS: i64 = 7;

// ---- archive / reactivate

async fn owned_save(state: &CloudState, user_id: Uuid, save_id: &str) -> Result<(), CloudError> {
    let owner: Option<Uuid> = sqlx::query_scalar("SELECT user_id FROM saves WHERE id = $1")
        .bind(save_id)
        .fetch_optional(&state.pool)
        .await?;
    match owner {
        None => Err(CloudError::NotFound("save not found")),
        Some(o) if o != user_id => Err(CloudError::Forbidden("save belongs to a different user")),
        Some(_) => Ok(()),
    }
}

/// Reject an upload targeting an archived save. While a save is frozen it must
/// not be re-uploaded: a new version would revive its blobs (cas_commit's
/// `ON CONFLICT ... refcount + 1`) and re-inflate the quota the archive just
/// freed, defeating the whole point, and un-freezing a game the user chose to
/// park. The `save_archived` code lets the client recognise this and stop
/// retrying instead of surfacing a generic error. No-op for a save that doesn't
/// exist yet (first upload) or isn't archived.
pub(crate) async fn ensure_not_archived(
    state: &CloudState,
    user_id: Uuid,
    save_id: &str,
) -> Result<(), CloudError> {
    let archived: Option<Option<time::OffsetDateTime>> =
        sqlx::query_scalar("SELECT archived_at FROM saves WHERE id = $1 AND user_id = $2")
            .bind(save_id)
            .bind(user_id)
            .fetch_optional(&state.pool)
            .await?;
    if let Some(Some(_)) = archived {
        return Err(CloudError::ForbiddenCode {
            code: "save_archived",
            message: "this game is archived — reactivate it to sync again",
        });
    }
    Ok(())
}

/// Bytes that archiving this save would free: blobs whose every reference
/// comes from this save (`refcount <= n`), so de-referencing drops them to 0.
/// Shared blobs free nothing.
async fn exclusive_bytes(
    conn: &mut sqlx::PgConnection,
    user_id: Uuid,
    refs: &[(Vec<u8>, i64)],
) -> Result<i64, CloudError> {
    let (shas, counts): (Vec<Vec<u8>>, Vec<i64>) = refs.iter().cloned().unzip();
    let bytes: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(b.size_bytes), 0)::bigint
           FROM cloud_blobs b
           JOIN unnest($2::bytea[], $3::bigint[]) AS t(sha, n) ON b.sha256 = t.sha
          WHERE b.user_id = $1 AND b.refcount > 0 AND b.refcount <= t.n",
    )
    .bind(user_id)
    .bind(&shas)
    .bind(&counts)
    .fetch_one(conn)
    .await?;
    Ok(bytes)
}

#[derive(Debug, Serialize)]
pub struct ArchiveOut {
    pub save_id: String,
    pub archived: bool,
    /// RFC3339. When the grace window ends and the save is hard-deleted.
    pub purge_after: String,
    /// Bytes freed from the quota by archiving (deduped, this save's exclusive
    /// blobs).
    pub freed_bytes: i64,
}

/// Archive one save: freeze its blobs (out of quota, kept in R2 for the grace
/// window) and mark it archived. Idempotent: archiving an already-archived
/// save just returns its current state. The local copy is never touched.
pub async fn archive_save(
    state: &CloudState,
    user_id: Uuid,
    save_id: &str,
) -> Result<ArchiveOut, CloudError> {
    owned_save(state, user_id, save_id).await?;

    // One transaction with the save row locked: two archive calls in flight
    // used to both pass the "already archived?" check and freeze the blobs
    // twice, and a request cut short left the save archived with its blobs
    // still charged.
    let mut tx = state.pool.begin().await?;
    let existing: Option<Option<time::OffsetDateTime>> =
        sqlx::query_scalar("SELECT archived_at FROM saves WHERE id = $1 FOR UPDATE")
            .bind(save_id)
            .fetch_optional(&mut *tx)
            .await?;
    if let Some(Some(archived_at)) = existing {
        tx.rollback().await.ok();
        return Ok(ArchiveOut {
            save_id: save_id.to_string(),
            archived: true,
            purge_after: fmt_ts(archived_at + time::Duration::days(ARCHIVE_GRACE_DAYS)),
            freed_bytes: 0,
        });
    }

    let refs = blob_refs::committed_refs(&mut tx, save_id, None).await?;
    let freed = exclusive_bytes(&mut tx, user_id, &refs).await?;

    // Marked archived in the same transaction as the release, so the client
    // and the sync manifest never see the blobs gone while the save still
    // looks live, and never read that as "the save vanished".
    let archived_at: time::OffsetDateTime = sqlx::query_scalar(
        "UPDATE saves SET archived_at = now(), updated_at = now() WHERE id = $1 RETURNING archived_at",
    )
    .bind(save_id)
    .fetch_one(&mut *tx)
    .await?;

    // Legacy (non content-addressed) versions charge storage via save_versions,
    // not blobs. Soft-delete them so the trigger credits their space; the cron
    // drops their R2 objects when the save expires. No-op for all-CA saves.
    sqlx::query(
        "UPDATE save_versions SET deleted_at = now()
            WHERE save_id = $1 AND content_addressed = FALSE AND deleted_at IS NULL",
    )
    .bind(save_id)
    .execute(&mut *tx)
    .await?;

    blob_refs::release(&mut tx, user_id, &refs, ARCHIVE_GRACE_DAYS * 24 * 60 * 60).await?;
    tx.commit().await?;

    tracing::info!(user_id = %user_id, save_id, freed_bytes = freed, "archive: game archived");
    Ok(ArchiveOut {
        save_id: save_id.to_string(),
        archived: true,
        purge_after: fmt_ts(archived_at + time::Duration::days(ARCHIVE_GRACE_DAYS)),
        freed_bytes: freed,
    })
}

/// Re-reference an archived save's blobs (clearing `purge_after`) and clear
/// `archived_at`, if it still fits the quota. Rejects with 402 if reactivating
/// would exceed the plan (upgrade to Pro / free space first). Fails if the grace
/// window already elapsed (the data is gone / going).
pub async fn reactivate_save(
    state: &CloudState,
    user_id: Uuid,
    save_id: &str,
) -> Result<(), CloudError> {
    owned_save(state, user_id, save_id).await?;

    let mut tx = state.pool.begin().await?;
    let archived_at: Option<time::OffsetDateTime> =
        sqlx::query_scalar("SELECT archived_at FROM saves WHERE id = $1 FOR UPDATE")
            .bind(save_id)
            .fetch_optional(&mut *tx)
            .await?
            .flatten();
    let Some(archived_at) = archived_at else {
        return Err(CloudError::BadRequest("save is not archived".into()));
    };
    if archived_at < time::OffsetDateTime::now_utc() - time::Duration::days(ARCHIVE_GRACE_DAYS) {
        return Err(CloudError::NotFound("archived save expired and was purged"));
    }

    // Bytes that will re-count once reactivated: currently-frozen blobs
    // (refcount 0) this save references. Gate them against the quota so a Free
    // user can't reactivate straight back over the limit.
    let refs = blob_refs::committed_refs(&mut tx, save_id, None).await?;
    let (shas, _): (Vec<Vec<u8>>, Vec<i64>) = refs.iter().cloned().unzip();
    let reclaim: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(size_bytes), 0)::bigint FROM cloud_blobs
          WHERE user_id = $1 AND sha256 = ANY($2) AND refcount = 0",
    )
    .bind(user_id)
    .bind(&shas)
    .fetch_one(&mut *tx)
    .await?;
    let (limits, info) = quota::load(&state.pool, user_id)
        .await?
        .ok_or(CloudError::NotFound("no profile"))?;
    if quota::would_exceed(info.used_bytes, reclaim.max(0) as u64, limits.storage_bytes) {
        return Err(CloudError::ForbiddenCode {
            code: "quota_exceeded",
            message: "reactivating this game would exceed your plan — upgrade to Pro or free space",
        });
    }

    blob_refs::retake(&mut tx, user_id, &refs).await?;

    // Restore soft-deleted legacy versions and unarchive.
    sqlx::query(
        "UPDATE save_versions SET deleted_at = NULL
            WHERE save_id = $1 AND content_addressed = FALSE AND deleted_at IS NOT NULL",
    )
    .bind(save_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE saves SET archived_at = NULL, updated_at = now() WHERE id = $1")
        .bind(save_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    // The countdown is off, so the warning about it should be able to fire
    // again if the game is archived a second time.
    let _ = crate::cloud::notices::clear(
        &state.pool,
        user_id,
        crate::cloud::notices::Kind::ArchiveExpiring,
        save_id,
    )
    .await;

    tracing::info!(user_id = %user_id, save_id, "archive: game reactivated");
    Ok(())
}

// ---- http handlers

/// `POST /v1/cloud/saves/:save_id/archive`
pub async fn archive_handler(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
    Path(save_id): Path<String>,
) -> Result<Response, CloudError> {
    let out = archive_save(&state, user.user_id, &save_id).await?;
    Ok(Json(out).into_response())
}

/// `POST /v1/cloud/saves/:save_id/reactivate`
pub async fn reactivate_handler(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
    Path(save_id): Path<String>,
) -> Result<Response, CloudError> {
    reactivate_save(&state, user.user_id, &save_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT.into_response())
}

#[derive(Debug, Serialize)]
pub struct GameFootprint {
    pub save_id: String,
    pub game_slug: String,
    pub label: String,
    /// Bytes freed from the quota by archiving this game (deduped exclusive
    /// blobs). What the dialog ranks the heaviest games by.
    pub freeable_bytes: i64,
    pub archived: bool,
    /// RFC3339 hard-delete instant, present only while archived.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purge_after: Option<String>,
}

/// Bytes that only come back when **every** save in `save_ids` is archived.
///
/// Content addressing means two saves can point at the same blob, most often
/// because the same folder ended up tracked twice under two slugs (the mars /
/// surviving-mars-relaunched case: 1.25 GB, 60% of a Free quota). Those bytes
/// are exclusive to *neither* save, so they vanish from both `freeable_bytes`
/// figures and the dialog silently under-reports what the account is carrying,
/// and archiving either game alone frees nothing at all, which reads as the
/// feature being broken.
#[derive(Debug, Serialize)]
pub struct SharedGroup {
    pub save_ids: Vec<String>,
    pub bytes: i64,
}

/// Blobs referenced by more than one *live* save, grouped by the exact set of
/// saves referencing them. Archived saves are excluded: their references are
/// already released, so they can't hold anything hostage.
///
/// Split out of the handler so it can be exercised against a real database
/// without standing up a `CloudState`. The array decode (`text[]` to `Vec<String>`,
/// which hinges on `saves.id` being TEXT rather than UUID) is the kind of thing
/// that compiles happily and 500s in production.
pub async fn shared_groups(pool: &PgPool, user_id: Uuid) -> Result<Vec<SharedGroup>, CloudError> {
    let rows: Vec<(Vec<String>, i64)> = sqlx::query_as(
        r#"
        WITH per_blob AS (
            SELECT f.sha256, array_agg(DISTINCT f.save_id ORDER BY f.save_id) AS save_ids
            FROM manifest_files f
            JOIN saves s ON s.id = f.save_id
            WHERE s.user_id = $1 AND s.archived_at IS NULL
            GROUP BY f.sha256
            HAVING COUNT(DISTINCT f.save_id) > 1
        )
        SELECT pb.save_ids, COALESCE(SUM(b.size_bytes), 0)::bigint AS bytes
        FROM per_blob pb
        JOIN cloud_blobs b ON b.user_id = $1 AND b.sha256 = pb.sha256
        WHERE b.refcount > 0
        GROUP BY pb.save_ids
        -- A group that frees nothing is not a group. Two saves sharing an empty
        -- file satisfy the condition above and reached the client as "shares 0 B
        -- with X: archive both to free it".
        HAVING COALESCE(SUM(b.size_bytes), 0) > 0
        ORDER BY 2 DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(save_ids, bytes)| SharedGroup { save_ids, bytes })
        .collect())
}

#[derive(Debug, Serialize)]
pub struct StorageGamesOut {
    pub plan: &'static str,
    pub used_bytes: u64,
    pub limit_bytes: u64,
    /// How many bytes over the limit the *live* footprint is (0 if within).
    pub over_bytes: u64,
    pub games: Vec<GameFootprint>,
    /// Blobs shared between two or more live saves, grouped by the exact set
    /// sharing them. A client picking what to archive adds a group's `bytes`
    /// only once its whole `save_ids` set is selected.
    #[serde(default)]
    pub shared_groups: Vec<SharedGroup>,
}

/// `GET /v1/cloud/storage/games`: per-game freeable footprint plus the quota
/// figures, so the desktop can render the "archive the heaviest to fit" dialog
/// and know how much to free.
pub async fn storage_games(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
) -> Result<Response, CloudError> {
    let (_, info) = match quota::load(&state.pool, user.user_id).await? {
        Some(x) => x,
        None => return Err(CloudError::NotFound("no profile")),
    };

    // Per save: bytes freed if archived (exclusive blobs, refcount <= refs
    // contributed here). Cast the SUM to bigint (NUMERIC → i64 would fail to
    // decode). Archived saves report 0 freeable (already out of quota).
    let rows: Vec<(String, String, String, i64, Option<time::OffsetDateTime>)> = sqlx::query_as(
        r#"
        WITH refs AS (
            SELECT f.save_id, f.sha256, COUNT(DISTINCT f.version_num) AS refs_here
            FROM manifest_files f
            JOIN saves s ON s.id = f.save_id
            WHERE s.user_id = $1
            GROUP BY f.save_id, f.sha256
        ),
        freeable AS (
            SELECT r.save_id,
                   COALESCE(SUM(CASE WHEN b.refcount <= r.refs_here THEN b.size_bytes ELSE 0 END), 0)::bigint AS bytes
            FROM refs r
            JOIN cloud_blobs b ON b.user_id = $1 AND b.sha256 = r.sha256
            GROUP BY r.save_id
        )
        SELECT s.id, s.game_slug, s.label,
               COALESCE(fr.bytes, 0)::bigint AS freeable,
               s.archived_at
        FROM saves s
        LEFT JOIN freeable fr ON fr.save_id = s.id
        WHERE s.user_id = $1
        ORDER BY COALESCE(fr.bytes, 0) DESC
        "#,
    )
    .bind(user.user_id)
    .fetch_all(&state.pool)
    .await?;

    let games = rows
        .into_iter()
        .map(
            |(save_id, game_slug, label, freeable, archived_at)| GameFootprint {
                save_id,
                game_slug,
                label,
                freeable_bytes: freeable,
                archived: archived_at.is_some(),
                purge_after: archived_at
                    .map(|a| fmt_ts(a + time::Duration::days(ARCHIVE_GRACE_DAYS))),
            },
        )
        .collect();

    let shared_groups = shared_groups(&state.pool, user.user_id).await?;

    let over = info.used_bytes.saturating_sub(info.limit_bytes);
    Ok(Json(StorageGamesOut {
        plan: info.plan,
        used_bytes: info.used_bytes,
        limit_bytes: info.limit_bytes,
        over_bytes: over,
        games,
        shared_groups,
    })
    .into_response())
}

// ---- expiry cron

/// Spawn the daily expiry sweep. Detached like the other sweepers: a failure
/// `warn!`s and the next tick retries.
pub fn spawn(state: CloudState) {
    tokio::spawn(async move {
        // Once after boot, then daily. See the note in `abandoned::spawn`: a
        // task that only fires 24h in never fires at all on a machine that
        // idles to zero and gets restarted.
        tokio::time::sleep(Duration::from_secs(5 * 60)).await;
        let mut tick = tokio::time::interval(Duration::from_secs(24 * 60 * 60));
        tick.tick().await; // the immediate one; the wait happens after the work
        loop {
            match purge_expired(&state).await {
                Ok((0, 0)) => {}
                Ok((saves, blobs)) => tracing::info!(
                    saves,
                    blobs,
                    "archive purge: hard-deleted expired archived games"
                ),
                Err(e) => {
                    tracing::warn!(error = %e, "archive purge: sweep failed");
                    incidents::record(Kind::Delete, "archive purge: sweep failed");
                }
            }
            tick.tick().await;
        }
    });
}

/// Hard-delete archived saves past the grace window and GC frozen blobs whose
/// window elapsed. Returns `(saves_deleted, blobs_deleted)`.
pub async fn purge_expired(state: &CloudState) -> Result<(usize, usize), sqlx::Error> {
    // 1. Archived saves due. Drop legacy R2 objects, then delete the save (the
    //    FK cascade removes its versions + file manifests). CA blob rows are
    //    handled by the blob GC below (their refcount is already 0).
    let due_saves: Vec<(String,)> = sqlx::query_as(
        "SELECT id FROM saves
          WHERE archived_at IS NOT NULL
            AND archived_at < now() - make_interval(days => $1)",
    )
    .bind(ARCHIVE_GRACE_DAYS as i32)
    .fetch_all(&state.pool)
    .await?;

    let mut saves_deleted = 0usize;
    for (save_id,) in &due_saves {
        let keys: Vec<(String,)> = sqlx::query_as(
            "SELECT r2_key FROM save_versions
                WHERE save_id = $1 AND content_addressed = FALSE AND r2_key <> ''",
        )
        .bind(save_id)
        .fetch_all(&state.pool)
        .await?;
        for (key,) in keys {
            if let Err(e) = state.r2.delete_object(&key).await {
                tracing::warn!(error = %e, r2_key = %key, "archive purge: legacy R2 delete failed");
                incidents::record(Kind::Delete, "archive purge: legacy R2 delete failed");
            }
        }
        sqlx::query("DELETE FROM saves WHERE id = $1")
            .bind(save_id)
            .execute(&state.pool)
            .await?;
        // `email_notices` keys per-save rows on the save id with no foreign key
        // to lean on, so the row has to go with the save it talks about.
        let _ = crate::cloud::notices::clear_scope(&state.pool, save_id).await;
        saves_deleted += 1;
    }

    // 2. Blobs whose window elapsed and are still unreferenced: frozen by an
    //    archive, or released by a delete.
    let blobs_deleted = purge_due_blobs(state, None).await?;

    Ok((saves_deleted, blobs_deleted))
}

/// Rows locked and objects deleted per transaction.
const GC_BATCH: i64 = 100;

const GC_CONCURRENCY: usize = 16;

/// Delete the object and then the row of every blob at `refcount = 0` whose
/// `purge_after` has passed. `only` narrows it to some accounts (tests share a
/// database); `None` is everyone. Returns how many went.
///
/// The rows are locked (`FOR UPDATE SKIP LOCKED`) for as long as their objects
/// are being deleted, and only rows still at 0 are dropped. Before, the rows
/// were read, the objects deleted and the rows then dropped unconditionally,
/// so a re-upload of the same file in between lost its bytes, and its row went
/// with a refcount above zero, handing back quota the version still used. An
/// upload that wants one of these rows now waits for the batch to finish.
///
/// An object that fails to delete keeps its row and comes back tomorrow. It
/// used to lose the row anyway, leaving bytes in the bucket nothing pointed at.
pub async fn purge_due_blobs(
    state: &CloudState,
    only: Option<&[Uuid]>,
) -> Result<usize, sqlx::Error> {
    let only: Option<Vec<Uuid>> = only.map(<[Uuid]>::to_vec);
    let mut deleted = 0usize;
    loop {
        let mut tx = state.pool.begin().await?;
        let batch: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT user_id, encode(sha256, 'hex') FROM cloud_blobs
              WHERE refcount = 0 AND purge_after IS NOT NULL AND purge_after < now()
                AND ($2::uuid[] IS NULL OR user_id = ANY($2))
              ORDER BY purge_after
              LIMIT $1
                FOR UPDATE SKIP LOCKED",
        )
        .bind(GC_BATCH)
        .bind(&only)
        .fetch_all(&mut *tx)
        .await?;
        if batch.is_empty() {
            tx.rollback().await.ok();
            return Ok(deleted);
        }

        let outcomes: Vec<((Uuid, String), bool)> =
            futures::stream::iter(batch.into_iter().map(|(user_id, sha)| {
                let r2 = state.r2.clone();
                async move {
                    let key = super::r2::key_for_blob(user_id, &sha);
                    let ok = match r2.delete_object(&key).await {
                        Ok(()) => true,
                        Err(e) => {
                            tracing::warn!(error = %e, r2_key = %key, "blob GC: R2 delete failed, retrying tomorrow");
                            incidents::record(Kind::Delete, "archive purge: blob R2 delete failed");
                            false
                        }
                    };
                    ((user_id, sha), ok)
                }
            }))
            .buffer_unordered(GC_CONCURRENCY)
            .collect()
            .await;

        let (gone, kept): (Vec<_>, Vec<_>) = outcomes.into_iter().partition(|(_, ok)| *ok);
        let (gone_users, gone_shas): (Vec<Uuid>, Vec<String>) =
            gone.into_iter().map(|(k, _)| k).unzip();
        let (kept_users, kept_shas): (Vec<Uuid>, Vec<String>) =
            kept.into_iter().map(|(k, _)| k).unzip();

        let res = sqlx::query(
            "DELETE FROM cloud_blobs b
              USING unnest($1::uuid[], $2::text[]) AS t(u, s)
              WHERE b.user_id = t.u AND b.sha256 = decode(t.s, 'hex') AND b.refcount = 0",
        )
        .bind(&gone_users)
        .bind(&gone_shas)
        .execute(&mut *tx)
        .await?;
        // Pushed a day out so the rest of this round doesn't pick them again.
        sqlx::query(
            "UPDATE cloud_blobs b SET purge_after = now() + interval '1 day'
               FROM unnest($1::uuid[], $2::text[]) AS t(u, s)
              WHERE b.user_id = t.u AND b.sha256 = decode(t.s, 'hex') AND b.refcount = 0",
        )
        .bind(&kept_users)
        .bind(&kept_shas)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        deleted += res.rows_affected() as usize;
        // A whole batch refused is the bucket, not the blobs: stop the round
        // rather than walk every due row failing one at a time.
        if gone_users.is_empty() {
            return Ok(deleted);
        }
    }
}

// ---- helpers

fn fmt_ts(t: time::OffsetDateTime) -> String {
    t.format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}
