//! `/v1/cloud/saves*`: upload and download flows for cloud-stored snapshots.

use crate::cloud::auth::CloudUser;
use crate::cloud::bandwidth;
use crate::cloud::errors::CloudError;
use crate::cloud::loopguard;
use crate::cloud::plans::Plan;
use crate::cloud::quota;
use crate::cloud::r2;
use crate::cloud::state::CloudState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    Extension,
};
use futures::{StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use uuid::Uuid;

/// Body for `POST /v1/cloud/saves`. The client states *intent*: how big the
/// snapshot will be, which save it belongs to, optional notes. The server
/// validates plan + quota, mints a presigned PUT URL for R2, and returns
/// it. Once the upload finishes, the client calls `commit` with the actual
/// sha256.
#[derive(Debug, Deserialize)]
pub struct UploadInit {
    pub save_id: String,
    pub game_slug: String,
    #[serde(default)]
    pub label: Option<String>,
    pub size_bytes: u64,
    /// Number of files inside the (opaque) tar.zst this version packs. The
    /// server can't introspect the blob, so the client declares it; we store
    /// it purely so the History view can show "N archivos". Older clients omit
    /// it → 0 (unknown), same as pre-existing rows.
    #[serde(default)]
    pub file_count: i64,
    /// Optional, used for analytics + device-limit check.
    #[serde(default)]
    pub device_name: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    /// When true, this save is excluded from the multi-device manifest pull
    /// and other devices won't auto-restore it. Manual download via
    /// `/v1/cloud/saves/:id/versions/:n/download` still works. Toggleable
    /// per-save by the client (per the 1.6.1 "modo ahorro" UX).
    #[serde(default)]
    pub backup_only: bool,
    /// The version the client based this upload on (its last-synced version
    /// for this save). When present and it no longer matches the server's
    /// `latest_version_num`, the upload is a non-fast-forward (another device
    /// advanced the save) and is rejected with 409 so the client can pull +
    /// resolve instead of clobbering the other device's line.
    #[serde(default)]
    pub base_version: Option<i64>,
}

/// 409 response for a divergent (non-fast-forward) push. Carries the current
/// head so the client knows which version it must reconcile against, and the
/// canonical save id it must reconcile *as*.
///
/// The id matters as much as the version. Clients key saves by their own
/// device-local id; `resolve_save_row` accepts one that this server has never
/// seen and resolves it by `(user, game_slug, label)`. When that happens the
/// client is pushing against a row whose id it doesn't know, so it can't find
/// itself in the manifest afterwards: it looks itself up by the local id,
/// finds nothing, concludes there is nothing to pull, and parks the conflict
/// with no way out. Answering with the row we actually rejected against is
/// what lets it reconcile instead of stalling.
#[derive(Debug, Serialize)]
struct NonFastForwardResponse {
    error: &'static str,
    code: &'static str,
    head_version: i64,
    base_version: i64,
    /// The cloud's own id for the save this upload resolved to. Equal to the
    /// requested id in the common case; different when two devices track the
    /// same (game, label) under different local ids.
    save_id: String,
}

impl IntoResponse for NonFastForwardResponse {
    fn into_response(self) -> Response {
        (StatusCode::CONFLICT, Json(self)).into_response()
    }
}

/// 413 response for a single upload that exceeds the per-save cap. Wire
/// shape mirrors the 402 quota response so the client toast layer can
/// reuse one structured-error path.
#[derive(Debug, Serialize)]
struct SaveTooLargeResponse {
    error: &'static str,
    code: &'static str,
    plan: &'static str,
    limit_bytes: u64,
    actual_bytes: u64,
    upgrade_url: String,
}

impl IntoResponse for SaveTooLargeResponse {
    fn into_response(self) -> Response {
        (StatusCode::PAYLOAD_TOO_LARGE, Json(self)).into_response()
    }
}

#[derive(Debug, Serialize)]
pub struct UploadInitOut {
    pub version_num: i64,
    pub r2_key: String,
    pub upload: r2::PresignedUrl,
    pub quota: quota::QuotaInfo,
}

pub async fn init_upload(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
    Json(body): Json<UploadInit>,
) -> Result<Response, CloudError> {
    // 1. Per-save size cap: the cheapest check, and it doesn't touch the DB.
    //    Resolve the plan from the cached load() in `check_storage` below
    //    would mean two queries; do a tiny direct lookup so we can 413
    //    before incurring the storage SUM.
    let plan = match plan_for_user(&state, user.user_id).await? {
        Some(p) => p,
        None => return Err(CloudError::NotFound("no profile")),
    };
    let limits = plan.limits();
    if body.size_bytes > limits.max_save_size_bytes {
        let upgrade_url = state
            .config
            .cloud
            .as_ref()
            .map(|c| c.upgrade_url.clone())
            .unwrap_or_else(crate::config::default_upgrade_url);
        return Ok(SaveTooLargeResponse {
            error: "save exceeds per-save size limit",
            code: "save_too_large",
            plan: plan.as_str(),
            limit_bytes: limits.max_save_size_bytes,
            actual_bytes: body.size_bytes,
            upgrade_url,
        }
        .into_response());
    }

    // 2. Bandwidth window: a pre-upload check so we 429 *before* presigning.
    //    The PUT itself goes direct to R2 so we can't intercept the bytes;
    //    we credit the window in `commit_upload` once R2 head confirms the
    //    object landed.
    if let Err(resp) = bandwidth::check(&state, user.user_id, body.size_bytes, &limits).await {
        log_sync_block(
            &state,
            user.user_id,
            "bandwidth_block",
            &body.save_id,
            Some(&body.game_slug),
            body.size_bytes,
        )
        .await;
        return Ok(resp);
    }

    // 3. Storage quota.
    let info = match quota::check_storage(&state, user.user_id, body.size_bytes).await {
        Ok(i) => i,
        Err(resp) => {
            return Ok(paced_quota_reject(
                &state,
                user.user_id,
                &body.save_id,
                Some(&body.game_slug),
                body.size_bytes,
                resp,
            )
            .await);
        }
    };

    // 4. Ensure the saves row exists. UPSERT semantics: the first version
    //    of a save creates it; subsequent versions just bump latest_version_num.
    //    `backup_only` is captured per-save: a row toggled to backup_only=true
    //    on a later upload stays out of the manifest until explicitly
    //    re-enabled by the client.
    let mut conn = state.pool.acquire().await?;
    let save_row = resolve_save_row(
        &mut conn,
        &body.save_id,
        user.user_id,
        &body.game_slug,
        &body.label,
        body.backup_only,
    )
    .await?;

    let head = save_row.1;

    // Fast-forward check (the DAG's enforcement). A base that no longer matches
    // the head means another device pushed since the client last synced.
    //
    // No base at all used to skip the check entirely, and that is the hole: a
    // client sends no base when its local cursor is null, and a machine that has
    // never synced this save, or one whose state was rebuilt. Against a save
    // with history that is the *least* trustworthy upload there is, and it was
    // the only one allowed through unchecked. In aug-2026 a device with a null
    // cursor pushed its folder over a head ten versions ahead: the other
    // machine's copy stayed in history, but the head became a folder that had
    // never seen it, which reads exactly like "it stopped syncing".
    //
    // Treated as base 0: fine against an empty save (the first upload), a
    // non-fast-forward against anything else. The client already knows this
    // answer: it reconciles and pulls before retrying.
    let base = body.base_version.unwrap_or(0);
    if base != head && !save_has_no_history(&mut conn, &save_row.0, head).await? {
        return Ok(NonFastForwardResponse {
            error: "non-fast-forward: another device advanced this save since your base version",
            code: "non_fast_forward",
            head_version: head,
            base_version: base,
            save_id: save_row.0.clone(),
        }
        .into_response());
    }
    if base != head {
        tracing::warn!(
            user_id = %user.user_id,
            save_id = %save_row.0,
            game_slug = %body.game_slug,
            base_version = base,
            "init_upload: client cursor outlived the save's history — restarting it at version 1"
        );
    }

    let next_version = head + 1;
    // Root version has no parent; otherwise it descends from the current head.
    let parent_version: Option<i64> = (head > 0).then_some(head);
    let r2_key = r2::key_for_snapshot(user.user_id, &save_row.0, next_version as u64);

    // 5. Insert the (pending) save_versions row. We only know size and key
    //    so far; sha256 is filled in by `commit`. Until then the row is
    //    pending and storage_bytes hasn't been credited (trigger runs on
    //    INSERT but with the *requested* size; if the upload fails or never
    //    commits, the cleanup cron deletes pending rows older than 1h).
    sqlx::query(
        r#"
        INSERT INTO save_versions (save_id, version_num, size_bytes, sha256, r2_key, notes, parent_version, file_count, device_name)
        VALUES ($1, $2, $3, '', $4, $5, $6, $7, $8)
        "#,
    )
    .bind(&save_row.0)
    .bind(next_version)
    .bind(body.size_bytes as i64)
    .bind(&r2_key)
    .bind(body.notes.as_deref())
    .bind(parent_version)
    .bind(body.file_count.max(0))
    .bind(body.device_name.as_deref())
    .execute(&state.pool)
    .await?;

    // 6. Mint the presigned PUT URL.
    let upload = state
        .r2
        .presign_put(&r2_key, None)
        .await
        .map_err(CloudError::Internal)?;

    Ok(Json(UploadInitOut {
        version_num: next_version,
        r2_key,
        upload,
        quota: info,
    })
    .into_response())
}

#[derive(Debug, Deserialize)]
pub struct UploadCommit {
    pub sha256: String,
    /// Actual size as observed by the client post-upload. We verify against
    /// R2 head to make sure the user didn't lie.
    pub size_bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct UploadCommitOut {
    pub save_id: String,
    pub version_num: i64,
    pub committed: bool,
}

pub async fn commit_upload(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
    Path((save_id, version)): Path<(String, i64)>,
    Json(body): Json<UploadCommit>,
) -> Result<Response, CloudError> {
    // Owner check first: never trust a save_id from the request without
    // verifying the JWT subject owns it.
    let owner: Option<Uuid> = sqlx::query_scalar("SELECT user_id FROM saves WHERE id = $1")
        .bind(&save_id)
        .fetch_optional(&state.pool)
        .await?;
    let Some(owner) = owner else {
        return Err(CloudError::NotFound("save not found"));
    };
    if owner != user.user_id {
        return Err(CloudError::Forbidden("save belongs to a different user"));
    }
    crate::cloud::archive::ensure_not_archived(&state, user.user_id, &save_id).await?;

    // The committed hash is persisted (and, for CAS, addresses R2 objects).
    // Reject anything that isn't a canonical sha256.
    if !r2::is_valid_sha256(&body.sha256) {
        return Err(CloudError::BadRequest("invalid sha256".into()));
    }

    // Look up the pending row.
    let row: Option<(String, i64)> = sqlx::query_as(
        "SELECT r2_key, size_bytes FROM save_versions
            WHERE save_id = $1 AND version_num = $2",
    )
    .bind(&save_id)
    .bind(version)
    .fetch_optional(&state.pool)
    .await?;
    let Some((r2_key, expected_size)) = row else {
        return Err(CloudError::NotFound("version not found"));
    };

    // Confirm the object actually landed in R2 with the right size.
    let head_size = state
        .r2
        .head(&r2_key)
        .await
        .map_err(CloudError::Internal)?
        .ok_or(CloudError::BadRequest(
            "R2 object missing — upload not complete".into(),
        ))?;
    if head_size as u64 != body.size_bytes {
        return Err(CloudError::BadRequest(format!(
            "size mismatch: client says {} but R2 has {}",
            body.size_bytes, head_size
        )));
    }

    // Re-enforce the per-save cap and storage quota against the REAL object
    // size. `init_upload` only saw the client-declared `size_bytes`, and the
    // presigned PUT carries no content-length limit, so a client can declare a
    // tiny size to pass the init gates and then upload an arbitrarily large
    // object. The pending row already charged the init-declared size; the
    // final footprint swaps that for `head_size`. On reject, best-effort drop
    // the R2 object + pending row so a refused commit can't squat storage.
    let (limits, info) = quota::load(&state.pool, user.user_id)
        .await?
        .ok_or(CloudError::NotFound("no profile"))?;
    let upgrade_url = || {
        state
            .config
            .cloud
            .as_ref()
            .map(|c| c.upgrade_url.clone())
            .unwrap_or_else(crate::config::default_upgrade_url)
    };
    let real = head_size.max(0) as u64;
    let reject = real > limits.max_save_size_bytes
        || quota::would_exceed(
            info.used_bytes.saturating_sub(expected_size.max(0) as u64),
            real,
            limits.storage_bytes,
        );
    if reject {
        if let Err(e) = state.r2.delete_object(&r2_key).await {
            tracing::warn!(error = %e, r2_key = %r2_key, "commit_upload: orphan object cleanup after quota reject failed");
        }
        sqlx::query(
            "DELETE FROM save_versions WHERE save_id = $1 AND version_num = $2 AND sha256 = ''",
        )
        .bind(&save_id)
        .bind(version)
        .execute(&state.pool)
        .await?;
        if real > limits.max_save_size_bytes {
            return Ok(SaveTooLargeResponse {
                error: "save exceeds per-save size limit",
                code: "save_too_large",
                plan: info.plan,
                limit_bytes: limits.max_save_size_bytes,
                actual_bytes: real,
                upgrade_url: upgrade_url(),
            }
            .into_response());
        }
        return Ok(paced_quota_reject(
            &state,
            user.user_id,
            &save_id,
            None,
            real,
            quota::quota_response(&info, real, upgrade_url()).into_response(),
        )
        .await);
    }

    // Atomically finalize: stamp sha256, bump the parent save's latest_version_num.
    let mut tx = state.pool.begin().await?;
    sqlx::query(
        "UPDATE save_versions SET sha256 = $1, size_bytes = $2
            WHERE save_id = $3 AND version_num = $4",
    )
    .bind(&body.sha256)
    .bind(head_size)
    .bind(&save_id)
    .bind(version)
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE saves SET latest_version_num = $1, updated_at = now() WHERE id = $2")
        .bind(version)
        .bind(&save_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO sync_log (user_id, save_id, version_num, kind, bytes)
             VALUES ($1, $2, $3, 'upload', $4)",
    )
    .bind(user.user_id)
    .bind(&save_id)
    .bind(version)
    .bind(head_size)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    // Credit the bandwidth window with the observed size. Done after the
    // commit so a failed upload doesn't eat into the user's quota. We log
    // and swallow errors here: the upload itself was successful, and a stale
    // bandwidth counter is recoverable, a 500 returned to the client is
    // not.
    if let Err(e) = bandwidth::record(&state.pool, user.user_id, head_size as u64).await {
        tracing::warn!(error = %e, user_id = %user.user_id, "bandwidth: record failed on upload");
    }

    // Reclaim space if over the plan threshold (purges old content-addressed
    // versions; off the response path). Then enforce the user's own
    // max-versions cap on the fresh history.
    let st = state.clone();
    let uid = user.user_id;
    tokio::spawn(async move {
        if let Err(e) = crate::cloud::purge::maybe_purge(&st, uid).await {
            tracing::warn!(error = ?e, user_id = %uid, "quota purge after commit failed");
        }
        if let Err(e) = crate::cloud::purge::prune_version_caps(&st, uid).await {
            tracing::warn!(error = ?e, user_id = %uid, "version-cap prune after commit failed");
        }
    });

    Ok(Json(UploadCommitOut {
        save_id,
        version_num: version,
        committed: true,
    })
    .into_response())
}

// ===========================================================================
// Content-addressed (per-file SHA dedup) upload + download.
//
// The client declares a manifest of (relative_path, sha256, size). The server
// answers which whole-file blobs it doesn't already have; the client PUTs only
// those to R2 (one object per blob, keyed by SHA), then commits. Bytes shared
// with a previous version, even most of a 600 MB save the game rewrote in place,
// are never re-uploaded.
// ===========================================================================

/// One file in a content-addressed manifest.
///
/// Constructible from tests (`orphaned_cursor`), which build manifests to check
/// what a push does and does not cover.
#[derive(Debug, Deserialize)]
pub struct CasFileEntry {
    pub relative_path: String,
    pub sha256: String,
    pub size_bytes: i64,
    /// Source file mtime (unix seconds), preserved on restore. Optional.
    #[serde(default)]
    pub modified_at: Option<i64>,
}

/// Body for `POST /v1/cloud/saves/cas`. Same intent as [`UploadInit`] but
/// carries the per-file manifest instead of a single archive size.
#[derive(Debug, Deserialize)]
pub struct CasInit {
    pub save_id: String,
    pub game_slug: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub device_name: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub backup_only: bool,
    #[serde(default)]
    pub base_version: Option<i64>,
    pub files: Vec<CasFileEntry>,
}

/// A blob the server doesn't have yet, so the client must PUT it.
#[derive(Debug, Serialize)]
pub struct CasMissingBlob {
    pub sha256: String,
    pub size_bytes: i64,
    pub r2_key: String,
    pub upload: r2::PresignedUrl,
}

#[derive(Debug, Serialize)]
pub struct CasInitOut {
    /// Canonical cloud save id. May differ from the id the client sent when
    /// (user, game_slug, label) already resolves to another save, so the client
    /// must use this id for the commit URL and re-key its local state.
    pub save_id: String,
    pub version_num: i64,
    /// Blobs the client must upload. The full file set is whatever it declared
    /// in `files`; everything not listed here is already stored server-side.
    pub missing: Vec<CasMissingBlob>,
    pub quota: quota::QuotaInfo,
}

pub async fn cas_init(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
    Json(body): Json<CasInit>,
) -> Result<Response, CloudError> {
    if body.files.is_empty() {
        return Err(CloudError::BadRequest("empty manifest".into()));
    }
    // Every content hash is interpolated into an R2 key and stored verbatim,
    // reject anything that isn't a canonical sha256 before it gets that far.
    if let Some(bad) = body.files.iter().find(|f| !r2::is_valid_sha256(&f.sha256)) {
        return Err(CloudError::BadRequest(format!(
            "invalid sha256 in manifest: {:?}",
            bad.sha256
        )));
    }

    // Don't accept uploads for a game the user archived: it would revive the
    // frozen blobs and re-inflate the quota. The client stops retrying on this.
    crate::cloud::archive::ensure_not_archived(&state, user.user_id, &body.save_id).await?;

    let plan = match plan_for_user(&state, user.user_id).await? {
        Some(p) => p,
        None => return Err(CloudError::NotFound("no profile")),
    };
    let limits = plan.limits();

    // Logical save size = sum of all file sizes. Drives the per-save cap.
    let logical_size: i64 = body.files.iter().map(|f| f.size_bytes.max(0)).sum();
    if logical_size as u64 > limits.max_save_size_bytes {
        let upgrade_url = state
            .config
            .cloud
            .as_ref()
            .map(|c| c.upgrade_url.clone())
            .unwrap_or_else(crate::config::default_upgrade_url);
        return Ok(SaveTooLargeResponse {
            error: "save exceeds per-save size limit",
            code: "save_too_large",
            plan: plan.as_str(),
            limit_bytes: limits.max_save_size_bytes,
            actual_bytes: logical_size as u64,
            upgrade_url,
        }
        .into_response());
    }

    // Dedup the manifest down to unique (sha -> size). Then find which of those
    // the user already has stored, so we only charge bandwidth and storage for,
    // and only presign, the genuinely new bytes.
    let mut unique: BTreeMap<String, i64> = BTreeMap::new();
    for f in &body.files {
        unique
            .entry(f.sha256.clone())
            .or_insert(f.size_bytes.max(0));
    }
    let all_shas: Vec<String> = unique.keys().cloned().collect();
    let existing: Vec<(String,)> =
        sqlx::query_as(
            "SELECT encode(sha256, 'hex') FROM cloud_blobs
              WHERE user_id = $1
                AND sha256 = ANY(ARRAY(SELECT decode(u, 'hex') FROM unnest($2::text[]) AS u))",
        )
            .bind(user.user_id)
            .bind(&all_shas)
            .fetch_all(&state.pool)
            .await?;
    let existing: std::collections::HashSet<String> = existing.into_iter().map(|(s,)| s).collect();

    let missing_shas: Vec<(&String, &i64)> = unique
        .iter()
        .filter(|(sha, _)| !existing.contains(*sha))
        .collect();
    let new_bytes: u64 = missing_shas.iter().map(|(_, sz)| **sz as u64).sum();

    // Bandwidth + storage are charged only on the bytes we'll actually move
    // and persist. A save rewritten in place with 10 MB of real change costs
    // 10 MB here, not 600 MB.
    if let Err(resp) = bandwidth::check(&state, user.user_id, new_bytes, &limits).await {
        tracing::warn!(
            user_id = %user.user_id,
            requested_save_id = %body.save_id,
            game_slug = %body.game_slug,
            new_bytes,
            "cas_init: rejected by bandwidth window"
        );
        log_sync_block(
            &state,
            user.user_id,
            "bandwidth_block",
            &body.save_id,
            Some(&body.game_slug),
            new_bytes,
        )
        .await;
        return Ok(resp);
    }
    let info = match quota::check_storage(&state, user.user_id, new_bytes).await {
        Ok(i) => i,
        Err(resp) => {
            tracing::warn!(
                user_id = %user.user_id,
                requested_save_id = %body.save_id,
                game_slug = %body.game_slug,
                new_bytes,
                "cas_init: rejected by storage quota"
            );
            return Ok(paced_quota_reject(
                &state,
                user.user_id,
                &body.save_id,
                Some(&body.game_slug),
                new_bytes,
                resp,
            )
            .await);
        }
    };

    // Ensure the saves row exists (same UPSERT semantics as the archive path).
    //
    // Everything from here to the manifest insert runs in one transaction, and
    // `resolve_save_row` has to run *inside* it: its UPDATE takes the row lock
    // on the saves row, held until commit, and that is what serializes
    // concurrent cas_inits for the same save. Without the lock two requests read
    // the same head and the second INSERT into save_versions dies on the
    // (save_id, version_num) unique constraint: the "db_error" bursts when
    // several devices or sweeps fire at once.
    let mut tx = state.pool.begin().await?;
    let save_row = resolve_save_row(
        &mut tx,
        &body.save_id,
        user.user_id,
        &body.game_slug,
        &body.label,
        body.backup_only,
    )
    .await?;
    let head = save_row.1;

    // The client keys saves by its own device-local id; the server keys by
    // (user, game_slug, label). When two devices track the same game with
    // different local ids they resolve to one cloud save here, so log it loudly,
    // it's the usual root cause of "another device is ahead" upload failures.
    if save_row.0 != body.save_id {
        tracing::warn!(
            requested_save_id = %body.save_id,
            canonical_save_id = %save_row.0,
            game_slug = %body.game_slug,
            client_label = body.label.as_deref().unwrap_or("default"),
            head_version = head,
            "cas_init: save id divergence — client save_id resolved to a different cloud save (cross-device collision)"
        );
    }

    tracing::info!(
        user_id = %user.user_id,
        save_id = %save_row.0,
        game_slug = %body.game_slug,
        files = body.files.len(),
        unique_blobs = unique.len(),
        missing_blobs = missing_shas.len(),
        new_bytes,
        head_version = head,
        base_version = ?body.base_version,
        "cas_init: request"
    );

    // No base means a null local cursor, which against a save with history is
    // the least trustworthy upload there is; see the same check on the archive
    // path for the incident. Treated as base 0: fine on a first upload, a
    // non-fast-forward against anything else.
    let base = body.base_version.unwrap_or(0);
    // An empty row is the one mismatch that isn't a divergence; see
    // `save_has_no_history`. It reads the versions inside the same transaction
    // that holds the row lock, so a sibling push can't land one in between.
    let orphaned_cursor = base != head && save_has_no_history(&mut tx, &save_row.0, head).await?;
    if orphaned_cursor {
        tracing::warn!(
            user_id = %user.user_id,
            save_id = %save_row.0,
            requested_save_id = %body.save_id,
            game_slug = %body.game_slug,
            base_version = base,
            "cas_init: client cursor outlived the save's history — restarting it at version 1"
        );
    }
    // The other way a mismatched base is harmless: the push already contains
    // every file the head has, so nothing of the head's can be lost by writing
    // the next version from it.
    let covers_head = base != head
        && !orphaned_cursor
        && manifest_covers_head(&mut tx, &save_row.0, head, &body.files).await?;
    if covers_head {
        tracing::warn!(
            user_id = %user.user_id,
            save_id = %save_row.0,
            game_slug = %body.game_slug,
            head_version = head,
            base_version = base,
            "cas_init: base diverged but the manifest contains the whole head — fast-forwarding it"
        );
    }
    if base != head && !orphaned_cursor && !covers_head {
        tracing::warn!(
            save_id = %save_row.0,
            requested_save_id = %body.save_id,
            head_version = head,
            base_version = base,
            had_base = body.base_version.is_some(),
            "cas_init: rejected non_fast_forward"
        );
        return Ok(NonFastForwardResponse {
            error: "non-fast-forward: another device advanced this save since your base version",
            code: "non_fast_forward",
            head_version: head,
            base_version: base,
            save_id: save_row.0.clone(),
        }
        .into_response());
    }

    let next_version = head + 1;
    let parent_version: Option<i64> = (head > 0).then_some(head);

    // Replace any stale pending row at this version (an earlier init that never
    // committed). Its manifest rows cascade away with it.
    sqlx::query(
        "DELETE FROM save_versions WHERE save_id = $1 AND version_num = $2 AND sha256 = ''",
    )
    .bind(&save_row.0)
    .bind(next_version)
    .execute(&mut *tx)
    .await?;

    // Pending content-addressed version. sha256 = '' until commit; r2_key is
    // unused (blobs carry their own keys). The storage trigger skips
    // content_addressed rows, so this charges nothing; blobs do, at commit.
    let (version_id,): (i64,) = sqlx::query_as(
        r#"
        INSERT INTO save_versions
            (save_id, version_num, size_bytes, sha256, r2_key, notes, parent_version, file_count, content_addressed, device_name)
        VALUES ($1, $2, $3, '', '', $4, $5, $6, TRUE, $7)
        RETURNING id
        "#,
    )
    .bind(&save_row.0)
    .bind(next_version)
    .bind(logical_size)
    .bind(body.notes.as_deref())
    .bind(parent_version)
    .bind(body.files.len() as i64)
    .bind(body.device_name.as_deref())
    .fetch_one(&mut *tx)
    .await?;

    // Record the manifest in one round-trip via UNNEST.
    let paths: Vec<String> = body.files.iter().map(|f| f.relative_path.clone()).collect();
    let shas: Vec<String> = body.files.iter().map(|f| f.sha256.clone()).collect();
    let sizes: Vec<i64> = body.files.iter().map(|f| f.size_bytes.max(0)).collect();
    let mtimes: Vec<Option<i64>> = body.files.iter().map(|f| f.modified_at).collect();
    sqlx::query(
        r#"
        INSERT INTO save_version_files (save_id, version_num, relative_path, sha256, size_bytes, modified_at)
        SELECT $1, $2, p, decode(s, 'hex'), z, m
          FROM UNNEST($3::text[], $4::text[], $5::bigint[], $6::bigint[]) AS t(p, s, z, m)
        ON CONFLICT (save_id, version_num, relative_path) DO NOTHING
        "#,
    )
    .bind(&save_row.0)
    .bind(next_version)
    .bind(&paths)
    .bind(&shas)
    .bind(&sizes)
    .bind(&mtimes)
    .execute(&mut *tx)
    .await?;

    // Second write, into the interned tables. The old table above is still what
    // everything reads; this only fills the new shape so the backfill has less
    // to catch up on and the two can be compared before the cutover.
    //
    // One statement, because the entry ids have to exist before the references
    // can point at them. `ins` returns only the rows it actually inserted, and
    // the `file_entries` join sees the table as it was when the statement
    // began, so the COALESCE takes whichever side has the id: freshly inserted,
    // or already in the catalogue from an earlier version.
    sqlx::query(
        r#"
        WITH input AS (
            SELECT p, decode(s, 'hex') AS sha, z, m
              FROM UNNEST($2::text[], $3::text[], $4::bigint[], $5::bigint[]) AS t(p, s, z, m)
        ),
        ins AS (
            INSERT INTO file_entries (save_id, relative_path, sha256, size_bytes)
            SELECT $1, p, sha, z FROM input
            ON CONFLICT (save_id, relative_path, sha256) DO NOTHING
            RETURNING id, relative_path, sha256
        )
        INSERT INTO version_files (version_id, entry_id, modified_at)
        SELECT $6, COALESCE(ins.id, fe.id), i.m
          FROM input i
          LEFT JOIN ins ON ins.relative_path = i.p AND ins.sha256 = i.sha
          LEFT JOIN file_entries fe
                 ON fe.save_id = $1 AND fe.relative_path = i.p AND fe.sha256 = i.sha
         WHERE COALESCE(ins.id, fe.id) IS NOT NULL
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(&save_row.0)
    .bind(&paths)
    .bind(&shas)
    .bind(&sizes)
    .bind(&mtimes)
    .bind(version_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    // Mint a presigned PUT for each missing blob.
    let mut missing = Vec::with_capacity(missing_shas.len());
    for (sha, size) in missing_shas {
        let key = r2::key_for_blob(user.user_id, sha);
        let upload = state
            .r2
            .presign_put(&key, None)
            .await
            .map_err(CloudError::Internal)?;
        missing.push(CasMissingBlob {
            sha256: sha.clone(),
            size_bytes: *size,
            r2_key: key,
            upload,
        });
    }

    tracing::info!(
        save_id = %save_row.0,
        version_num = next_version,
        missing_blobs = missing.len(),
        new_bytes,
        "cas_init: accepted — presigned PUTs minted"
    );

    Ok(Json(CasInitOut {
        save_id: save_row.0,
        version_num: next_version,
        missing,
        quota: info,
    })
    .into_response())
}

/// `POST /v1/cloud/saves/:save_id/versions/:version/cas/commit`: finalize a
/// content-addressed upload. Verifies each new blob landed in R2, bumps blob
/// refcounts (storage is charged on a blob's first reference), stamps the
/// version's manifest digest and advances the save head. Idempotent and
/// race-safe: the commit is claimed with a guarded `sha256 = ''` update.
/// R2 round trips kept in flight while a commit verifies and cleans up its
/// blobs. Each one is a bodyless request, so the ceiling here is R2's appetite
/// for concurrent connections rather than this machine's 512 MB.
const BLOB_CONCURRENCY: usize = 32;

pub async fn cas_commit(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
    Path((save_id, version)): Path<(String, i64)>,
) -> Result<Response, CloudError> {
    let owner: Option<Uuid> = sqlx::query_scalar("SELECT user_id FROM saves WHERE id = $1")
        .bind(&save_id)
        .fetch_optional(&state.pool)
        .await?;
    let Some(owner) = owner else {
        return Err(CloudError::NotFound("save not found"));
    };
    if owner != user.user_id {
        return Err(CloudError::Forbidden("save belongs to a different user"));
    }
    crate::cloud::archive::ensure_not_archived(&state, user.user_id, &save_id).await?;

    // The version must exist and be a content-addressed row.
    let vrow: Option<(String, bool)> = sqlx::query_as(
        "SELECT sha256, content_addressed FROM save_versions
            WHERE save_id = $1 AND version_num = $2",
    )
    .bind(&save_id)
    .bind(version)
    .fetch_optional(&state.pool)
    .await?;
    let Some((cur_sha, content_addressed)) = vrow else {
        tracing::warn!(
            save_id = %save_id,
            version,
            "cas_commit: version not found (often a cross-device save id mismatch — init landed under a different cloud save)"
        );
        return Err(CloudError::NotFound("version not found"));
    };
    if !content_addressed {
        tracing::warn!(save_id = %save_id, version, "cas_commit: version is not content-addressed");
        return Err(CloudError::BadRequest(
            "version is not content-addressed".into(),
        ));
    }
    if !cur_sha.is_empty() {
        // Already committed: idempotent success.
        return Ok(Json(UploadCommitOut {
            save_id,
            version_num: version,
            committed: true,
        })
        .into_response());
    }

    // Full manifest, ordered, for the digest; distinct shas for refcounting.
    let manifest: Vec<(String, String, i64)> = sqlx::query_as(
        "SELECT relative_path, encode(sha256, 'hex'), size_bytes FROM save_version_files
            WHERE save_id = $1 AND version_num = $2
         ORDER BY relative_path",
    )
    .bind(&save_id)
    .bind(version)
    .fetch_all(&state.pool)
    .await?;
    if manifest.is_empty() {
        return Err(CloudError::BadRequest("version has no manifest".into()));
    }

    // Manifest digest: a stable content identity for the whole version, used
    // wherever the archive's whole-blob sha256 was (list/sync/integrity).
    let mut hasher = Sha256::new();
    for (path, sha, size) in &manifest {
        hasher.update(path.as_bytes());
        hasher.update([0u8]);
        hasher.update(sha.as_bytes());
        hasher.update([0u8]);
        hasher.update(size.to_le_bytes());
        hasher.update([0u8]);
    }
    let digest = hex::encode(hasher.finalize());

    // Distinct (sha -> size).
    let mut unique: BTreeMap<String, i64> = BTreeMap::new();
    for (_, sha, size) in &manifest {
        unique.entry(sha.clone()).or_insert(*size);
    }
    let all_shas: Vec<String> = unique.keys().cloned().collect();
    let existing: Vec<(String,)> =
        sqlx::query_as(
            "SELECT encode(sha256, 'hex') FROM cloud_blobs
              WHERE user_id = $1
                AND sha256 = ANY(ARRAY(SELECT decode(u, 'hex') FROM unnest($2::text[]) AS u))",
        )
            .bind(user.user_id)
            .bind(&all_shas)
            .fetch_all(&state.pool)
            .await?;
    let existing: std::collections::HashSet<String> = existing.into_iter().map(|(s,)| s).collect();

    // Verify every new blob actually landed in R2 and trust R2's reported
    // size, never the client's declared manifest size, for accounting.
    // A malicious client can understate `size_bytes` at init to slip past the
    // quota check, then PUT arbitrarily large objects to the presigned URLs
    // (which carry no content-length limit). The real footprint is whatever
    // bytes actually landed, so that's what we charge and re-check below.
    //
    // Asked as one listing of the user's blob prefix rather than one HEAD per
    // blob. The difference is not a micro-optimisation: `cas_commit` has to
    // answer inside the client's fixed 60 s timeout, and the version that found
    // this carried 35,143 files over 24,784 distinct blobs because the game
    // writes one file per map chunk. Per-blob that is 24,784 round trips, tens of
    // minutes in sequence and still tens of seconds fanned out, and every
    // timeout sent the client back to re-upload the same 398 MB, hourly, for a
    // version that could never land. A listing answers all of them in ~26.
    let mut new_bytes: u64 = 0;
    let mut actual_size: BTreeMap<String, i64> = BTreeMap::new();
    let mut landed = state
        .r2
        .blob_sizes(user.user_id)
        .await
        .map_err(CloudError::Internal)?;

    // A listing is the bulk answer, not the authority: an object PUT moments
    // ago may not be in it yet. So anything it doesn't show is asked for
    // directly before we are willing to call it missing: normally nothing, and
    // a handful at worst, which is what keeps this off the per-blob path.
    //
    // The futures are built eagerly into a Vec of owned values rather than
    // mapped straight off the iterator: a closure holding a borrow inside the
    // stream trips rustc's "FnOnce is not general enough" false positive once
    // the handler is wrapped by the router. Same shape, and the same reason, as
    // the upload fan-out in `hoard-agent::backup`.
    let mut probes = Vec::new();
    for sha in unique
        .keys()
        .filter(|sha| !existing.contains(*sha) && !landed.contains_key(*sha))
        .cloned()
    {
        let r2 = state.r2.clone();
        let uid = user.user_id;
        probes.push(async move {
            let size = r2.head(&r2::key_for_blob(uid, &sha)).await?;
            Ok::<_, anyhow::Error>((sha, size))
        });
    }
    if !probes.is_empty() {
        let probed: Vec<(String, Option<i64>)> = futures::stream::iter(probes)
            .buffer_unordered(BLOB_CONCURRENCY)
            .try_collect()
            .await
            .map_err(CloudError::Internal)?;
        for (sha, size) in probed {
            if let Some(size) = size {
                landed.insert(sha, size);
            }
        }
    }

    // Walked in manifest order, not in completion order: a half-finished upload
    // has to name the same missing blob on every attempt, or the same failure
    // reads as a different one each time it is retried.
    for sha in unique.keys() {
        if existing.contains(sha) {
            continue;
        }
        let size = *landed
            .get(sha)
            .ok_or_else(|| CloudError::BadRequest(format!("blob {sha} was not uploaded")))?;
        new_bytes += size.max(0) as u64;
        actual_size.insert(sha.clone(), size);
    }

    // Re-enforce the storage quota against the REAL uploaded bytes. `cas_init`
    // only saw client-declared sizes; this is the authoritative gate before we
    // reference (and charge) the blobs. On reject, best-effort delete the
    // orphaned blobs so a refused commit can't squat un-accounted R2 storage.
    if let Err(resp) = quota::check_storage(&state, user.user_id, new_bytes).await {
        let mut cleanup = Vec::with_capacity(actual_size.len());
        for sha in actual_size.keys() {
            let r2 = state.r2.clone();
            let uid = user.user_id;
            let sha = sha.clone();
            cleanup.push(async move {
                if let Err(e) = r2.delete_object(&r2::key_for_blob(uid, &sha)).await {
                    tracing::warn!(error = %e, sha = %sha, "cas_commit: orphan blob cleanup after quota reject failed");
                }
            });
        }
        futures::stream::iter(cleanup)
            .buffer_unordered(BLOB_CONCURRENCY)
            .for_each(|()| async {})
            .await;
        return Ok(paced_quota_reject(&state, user.user_id, &save_id, None, new_bytes, resp).await);
    }

    // Claim + finalize atomically. The guarded update fences concurrent commits
    // and makes a double-commit a no-op (0 rows → someone else won the race).
    let mut tx = state.pool.begin().await?;
    let claimed = sqlx::query(
        "UPDATE save_versions SET sha256 = $1
            WHERE save_id = $2 AND version_num = $3
              AND content_addressed = TRUE AND sha256 = ''",
    )
    .bind(&digest)
    .bind(&save_id)
    .bind(version)
    .execute(&mut *tx)
    .await?;
    if claimed.rows_affected() == 0 {
        // Lost the race: another commit finalized it. Treat as success.
        tx.rollback().await.ok();
        return Ok(Json(UploadCommitOut {
            save_id,
            version_num: version,
            committed: true,
        })
        .into_response());
    }

    // Bump refcounts: +1 per distinct blob this version references. New blobs
    // are inserted at refcount 1 (their first reference charges storage via the
    // cloud_blobs trigger) with the size R2 actually reported, never the
    // client's declared size; existing ones just increment (the bound size is
    // ignored by the ON CONFLICT path, so their already-charged size stands).
    // `purge_after = NULL` un-trashes a blob that archiving had frozen: if the
    // user re-uploads a file whose blob is sitting in the 7-day archive grace
    // window, this revives it and the expiry cron must no longer sweep it.
    //
    // One statement, not one per blob. Row by row this held a write transaction
    // open for a round trip per blob against the pooler, which on a 24,784-blob
    // version is the second half of the same stall. `unique` is keyed by sha, so
    // no key repeats within the arrays, because `ON CONFLICT DO UPDATE` would reject
    // the whole statement if one did, since it cannot touch a row twice.
    let blob_shas: Vec<String> = unique.keys().cloned().collect();
    let blob_sizes: Vec<i64> = unique
        .iter()
        .map(|(sha, declared)| actual_size.get(sha).copied().unwrap_or(*declared))
        .collect();
    sqlx::query(
        r#"
        INSERT INTO cloud_blobs (user_id, sha256, size_bytes, refcount)
        SELECT $1, decode(s, 'hex'), z, 1
          FROM UNNEST($2::text[], $3::bigint[]) AS t(s, z)
        ON CONFLICT (user_id, sha256)
        DO UPDATE SET refcount = cloud_blobs.refcount + 1, purge_after = NULL
        "#,
    )
    .bind(user.user_id)
    .bind(&blob_shas)
    .bind(&blob_sizes)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "UPDATE saves SET latest_version_num = $1, updated_at = now()
            WHERE id = $2 AND latest_version_num < $1",
    )
    .bind(version)
    .bind(&save_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO sync_log (user_id, save_id, version_num, kind, bytes)
             VALUES ($1, $2, $3, 'upload', $4)",
    )
    .bind(user.user_id)
    .bind(&save_id)
    .bind(version)
    .bind(new_bytes as i64)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    // Credit bandwidth with the bytes actually transferred (the new blobs).
    if let Err(e) = bandwidth::record(&state.pool, user.user_id, new_bytes).await {
        tracing::warn!(error = %e, user_id = %user.user_id, "bandwidth: record failed on cas commit");
    }

    tracing::info!(
        save_id = %save_id,
        version,
        blobs = unique.len(),
        new_bytes,
        "cas_commit: committed"
    );

    // What the history row will say about this version. Off the critical path
    // and never fatal: the version is committed, and a row that fails to get a
    // label is a cosmetic loss.
    if let Err(e) = crate::insight::record_cloud(&state.pool, &save_id, version).await {
        tracing::warn!(error = ?e, save_id = %save_id, version, "insight: not recorded");
    }

    // Reclaim space if this commit pushed the user over their plan threshold.
    // Off the response path: a slow R2 delete sweep mustn't delay the client.
    // Then enforce the user's own max-versions cap on the fresh history.
    let st = state.clone();
    let uid = user.user_id;
    let sid = save_id.clone();
    tokio::spawn(async move {
        // What the history row will say about this version. Off the response
        // path and never fatal: the version is committed, and a row that fails
        // to get a label is a cosmetic loss, not two more queries on the
        // client's 60 s commit budget.
        if let Err(e) = crate::insight::record_cloud(&st.pool, &sid, version).await {
            tracing::warn!(error = ?e, save_id = %sid, version, "insight: not recorded");
        }
        if let Err(e) = crate::cloud::purge::maybe_purge(&st, uid).await {
            tracing::warn!(error = ?e, user_id = %uid, "quota purge after commit failed");
        }
        if let Err(e) = crate::cloud::purge::prune_version_caps(&st, uid).await {
            tracing::warn!(error = ?e, user_id = %uid, "version-cap prune after commit failed");
        }
    });

    Ok(Json(UploadCommitOut {
        save_id,
        version_num: version,
        committed: true,
    })
    .into_response())
}

/// One file in a version manifest response. `download` is populated only when
/// the request asked to presign (the restore path); the History detail view
/// omits it and pays no bandwidth.
#[derive(Debug, Serialize)]
pub struct ManifestFile {
    pub relative_path: String,
    pub sha256: String,
    pub size_bytes: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download: Option<r2::PresignedUrl>,
}

#[derive(Debug, Serialize)]
pub struct VersionManifestOut {
    /// False for legacy archive versions, and the client then falls back to the
    /// whole-archive `download` endpoint.
    pub content_addressed: bool,
    pub files: Vec<ManifestFile>,
}

#[derive(Debug, Deserialize)]
pub struct ManifestQuery {
    /// When true, mint a presigned GET per (unique) blob and charge bandwidth.
    /// When false (the default) just list the files: cheap, no bandwidth.
    #[serde(default)]
    pub presign: bool,
}

/// `GET /v1/cloud/saves/:save_id/versions/:version/manifest`: the per-file
/// manifest of a content-addressed version. With `?presign=true` it also mints
/// a download URL per blob (restore); without it, just the listing (History).
pub async fn version_manifest(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
    Path((save_id, version)): Path<(String, i64)>,
    axum::extract::Query(q): axum::extract::Query<ManifestQuery>,
) -> Result<Response, CloudError> {
    let vrow: Option<(Uuid, String, bool)> = sqlx::query_as(
        r#"
        SELECT s.user_id, s.game_slug, sv.content_addressed
          FROM save_versions sv
          JOIN saves s ON s.id = sv.save_id
         WHERE sv.save_id = $1 AND sv.version_num = $2
        "#,
    )
    .bind(&save_id)
    .bind(version)
    .fetch_optional(&state.pool)
    .await?;
    let Some((owner, game_slug, content_addressed)) = vrow else {
        return Err(CloudError::NotFound("version not found"));
    };
    if owner != user.user_id {
        return Err(CloudError::Forbidden("not your save"));
    }
    if !content_addressed {
        return Ok(Json(VersionManifestOut {
            content_addressed: false,
            files: Vec::new(),
        })
        .into_response());
    }

    let rows: Vec<(String, String, i64, Option<i64>)> = sqlx::query_as(
        "SELECT relative_path, encode(sha256, 'hex'), size_bytes, modified_at FROM save_version_files
            WHERE save_id = $1 AND version_num = $2
         ORDER BY relative_path",
    )
    .bind(&save_id)
    .bind(version)
    .fetch_all(&state.pool)
    .await?;

    if !q.presign {
        let files = rows
            .into_iter()
            .map(
                |(relative_path, sha256, size_bytes, modified_at)| ManifestFile {
                    relative_path,
                    sha256,
                    size_bytes,
                    modified_at,
                    download: None,
                },
            )
            .collect();
        return Ok(Json(VersionManifestOut {
            content_addressed: true,
            files,
        })
        .into_response());
    }

    // Before anything expensive: is this the same version, again, for the
    // umpteenth time today? The brake goes here rather than next to the
    // `sync_log` insert below so a paced client costs one SELECT instead of a
    // bandwidth debit, a blob lookup and one presigned URL per file.
    if let Some(pace) = loopguard::download_brake(&state, user.user_id, &save_id, version).await {
        tracing::warn!(
            user_id = %user.user_id,
            %save_id,
            %game_slug,
            version_num = version,
            downloads_24h = pace.seen,
            retry_after_secs = pace.retry_after_secs,
            "cloud: pacing repeated downloads of the same save version (cas manifest)"
        );
        log_sync_block(
            &state,
            user.user_id,
            "restore_paced",
            &save_id,
            Some(&game_slug),
            0,
        )
        .await;
        return Ok(loopguard::restore_loop_response(pace));
    }

    // Presign path: charge bandwidth for the unique bytes downloaded, then mint
    // one GET per distinct blob and map it back onto every file that uses it.
    let mut unique_size: BTreeMap<String, i64> = BTreeMap::new();
    for (_, sha, size, _) in &rows {
        unique_size.entry(sha.clone()).or_insert(*size);
    }
    let download_bytes: u64 = unique_size.values().map(|s| *s as u64).sum();

    let plan = plan_for_user(&state, user.user_id)
        .await?
        .unwrap_or(Plan::Free);
    let limits = plan.limits();
    if let Err(resp) = bandwidth::check(&state, user.user_id, download_bytes, &limits).await {
        return Ok(resp);
    }

    // Blobs the compression sweep claimed are served through the
    // decompressing proxy instead of a direct presigned GET (the object may
    // hold zstd bytes; the client must keep receiving raw). Everything else
    // stays on the presigned path.
    let shas: Vec<String> = unique_size.keys().cloned().collect();
    let compressed: std::collections::BTreeSet<String> = sqlx::query_as::<_, (String,)>(
        "SELECT encode(sha256, 'hex') FROM cloud_blobs
            WHERE user_id = $1
              AND sha256 = ANY(ARRAY(SELECT decode(u, 'hex') FROM unnest($2::text[]) AS u))
              AND encoding = 'zstd'",
    )
    .bind(user.user_id)
    .bind(&shas)
    .fetch_all(&state.pool)
    .await?
    .into_iter()
    .map(|(s,)| s)
    .collect();

    let ttl_secs = state
        .config
        .cloud
        .as_ref()
        .map(|c| c.r2.presign_ttl_secs)
        .unwrap_or(3600);
    let public_base = state.config.server.public_url.trim_end_matches('/');

    let mut url_for: BTreeMap<String, r2::PresignedUrl> = BTreeMap::new();
    for sha in unique_size.keys() {
        let presigned = if compressed.contains(sha) {
            let token = super::blob_proxy::mint_token(&state, user.user_id, sha, ttl_secs);
            r2::PresignedUrl {
                method: "GET".to_string(),
                url: format!("{public_base}/v1/cloud/blob/{token}"),
                expires_in_secs: ttl_secs,
            }
        } else {
            let key = r2::key_for_blob(user.user_id, sha);
            state
                .r2
                .presign_get(&key, None)
                .await
                .map_err(CloudError::Internal)?
        };
        url_for.insert(sha.clone(), presigned);
    }

    // Stamp the download so the sweep won't overwrite an object while a
    // just-minted direct URL might still be in flight.
    sqlx::query(
        "UPDATE cloud_blobs SET last_presigned_at = now()
            WHERE user_id = $1
              AND sha256 = ANY(ARRAY(SELECT decode(u, 'hex') FROM unnest($2::text[]) AS u))",
    )
    .bind(user.user_id)
    .bind(&shas)
    .execute(&state.pool)
    .await?;

    sqlx::query(
        "INSERT INTO sync_log (user_id, save_id, version_num, kind, bytes)
             VALUES ($1, $2, $3, 'download', $4)",
    )
    .bind(user.user_id)
    .bind(&save_id)
    .bind(version)
    .bind(download_bytes as i64)
    .execute(&state.pool)
    .await?;
    warn_on_repeat_download(&state, user.user_id, &save_id, &game_slug, version).await;
    if let Err(e) = bandwidth::record(&state.pool, user.user_id, download_bytes).await {
        tracing::warn!(error = %e, user_id = %user.user_id, "bandwidth: record failed on cas manifest");
    }

    let files = rows
        .into_iter()
        .map(
            |(relative_path, sha256, size_bytes, modified_at)| ManifestFile {
                download: url_for.get(&sha256).cloned(),
                relative_path,
                sha256,
                size_bytes,
                modified_at,
            },
        )
        .collect();
    Ok(Json(VersionManifestOut {
        content_addressed: true,
        files,
    })
    .into_response())
}

#[derive(Debug, Serialize)]
pub struct DownloadOut {
    pub save_id: String,
    pub version_num: i64,
    pub sha256: String,
    pub size_bytes: i64,
    pub download: r2::PresignedUrl,
}

pub async fn download(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
    Path((save_id, version)): Path<(String, i64)>,
) -> Result<Response, CloudError> {
    let row: Option<(Uuid, String, String, String, i64, bool)> = sqlx::query_as(
        r#"
        SELECT s.user_id, s.game_slug, sv.r2_key, sv.sha256, sv.size_bytes, sv.content_addressed
          FROM save_versions sv
          JOIN saves s ON s.id = sv.save_id
         WHERE sv.save_id = $1 AND sv.version_num = $2
           AND sv.deleted_at IS NULL
        "#,
    )
    .bind(&save_id)
    .bind(version)
    .fetch_optional(&state.pool)
    .await?;
    let Some((owner, game_slug, r2_key, sha256, size, content_addressed)) = row else {
        return Err(CloudError::NotFound("version not found"));
    };
    if owner != user.user_id {
        return Err(CloudError::Forbidden("not your save"));
    }
    if content_addressed {
        // Content-addressed versions have no single archive blob. The client
        // must pull them through the per-file manifest endpoint instead.
        return Err(CloudError::BadRequest(
            "version is content-addressed — use the manifest endpoint".into(),
        ));
    }

    // Same brake as the manifest path: a legacy archive pulled in a loop is
    // the more expensive of the two, since every retry is the whole save
    // rather than the blobs that changed.
    if let Some(pace) = loopguard::download_brake(&state, user.user_id, &save_id, version).await {
        tracing::warn!(
            user_id = %user.user_id,
            %save_id,
            %game_slug,
            version_num = version,
            downloads_24h = pace.seen,
            retry_after_secs = pace.retry_after_secs,
            "cloud: pacing repeated downloads of the same save version (archive)"
        );
        log_sync_block(
            &state,
            user.user_id,
            "restore_paced",
            &save_id,
            Some(&game_slug),
            0,
        )
        .await;
        return Ok(loopguard::restore_loop_response(pace));
    }

    // Bandwidth gate. Downloads count against the same window as uploads
    // (one quota, one counter). We credit optimistically below since the
    // presigned URL almost always gets used; if the client never fetches
    // it the overcount falls off in 15 min.
    let plan = plan_for_user(&state, user.user_id)
        .await?
        .unwrap_or(Plan::Free);
    let limits = plan.limits();
    if let Err(resp) = bandwidth::check(&state, user.user_id, size.max(0) as u64, &limits).await {
        return Ok(resp);
    }

    let presigned = state
        .r2
        .presign_get(&r2_key, None)
        .await
        .map_err(CloudError::Internal)?;

    sqlx::query(
        "INSERT INTO sync_log (user_id, save_id, version_num, kind, bytes)
             VALUES ($1, $2, $3, 'download', $4)",
    )
    .bind(user.user_id)
    .bind(&save_id)
    .bind(version)
    .bind(size)
    .execute(&state.pool)
    .await?;
    warn_on_repeat_download(&state, user.user_id, &save_id, &game_slug, version).await;

    if let Err(e) = bandwidth::record(&state.pool, user.user_id, size.max(0) as u64).await {
        tracing::warn!(error = %e, user_id = %user.user_id, "bandwidth: record failed on download");
    }

    Ok(Json(DownloadOut {
        save_id,
        version_num: version,
        sha256,
        size_bytes: size,
        download: presigned,
    })
    .into_response())
}

/// One row in the cloud version history. Mirrors the self-hosted
/// `SnapshotSummary` wire shape (and the agent's `Snapshot`) so the desktop's
/// History view renders cloud and self-hosted snapshots through one type.
#[derive(Debug, Serialize)]
pub struct VersionEntry {
    /// String id for parity with self-hosted (which keys on a UUID). The
    /// version number is unique within a save and all the client needs.
    pub id: String,
    pub save_id: String,
    pub version_num: i64,
    pub parent_version: Option<i64>,
    pub file_count: i64,
    pub total_size_bytes: i64,
    pub is_pinned: bool,
    /// The machine this version came from. `None` on everything uploaded before it
    /// was stored, and by older clients: the history stays quiet rather than
    /// inventarse un equipo.
    pub device_name: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: time::OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<time::OffsetDateTime>,
    /// What the row says about this version: which save it is about, what changed
    /// since the previous one. Absent on everything uploaded before the server
    /// derived it, and the client draws it as it always did.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insight: Option<hoard_core::kernel::insight::VersionInsight>,
}

#[derive(Debug, Deserialize)]
pub struct ListVersionsQuery {
    #[serde(default)]
    pub include_deleted: bool,
}

/// `GET /v1/cloud/saves/:save_id/versions`: the full committed version history
/// for a save. The sync manifest only carries the latest version; this
/// surfaces every committed one so History can list and restore any of them.
/// The R2 blobs and `save_versions` rows already persist per version; this
/// just exposes them. Pending (uncommitted, `sha256 = ''`) rows are excluded.
pub async fn list_versions(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
    Path(save_id): Path<String>,
    axum::extract::Query(q): axum::extract::Query<ListVersionsQuery>,
) -> Result<Json<Vec<VersionEntry>>, CloudError> {
    let owner: Option<Uuid> = sqlx::query_scalar("SELECT user_id FROM saves WHERE id = $1")
        .bind(&save_id)
        .fetch_optional(&state.pool)
        .await?;
    let Some(owner) = owner else {
        return Err(CloudError::NotFound("save not found"));
    };
    if owner != user.user_id {
        return Err(CloudError::Forbidden("save belongs to a different user"));
    }

    type Row = (
        i64,
        i64,
        Option<i64>,
        i64,
        bool,
        Option<String>,
        time::OffsetDateTime,
        Option<time::OffsetDateTime>,
        Option<sqlx::types::JsonValue>,
    );
    let rows: Vec<Row> = sqlx::query_as(
        r#"
        SELECT version_num, size_bytes, parent_version, file_count, is_pinned, device_name, created_at, deleted_at, insight
          FROM save_versions
         WHERE save_id = $1
           AND sha256 <> ''
           AND ($2 OR deleted_at IS NULL)
      ORDER BY version_num DESC
        "#,
    )
    .bind(&save_id)
    .bind(q.include_deleted)
    .fetch_all(&state.pool)
    .await?;

    let out = rows
        .into_iter()
        .map(
            |(
                version_num,
                size,
                parent,
                file_count,
                is_pinned,
                device_name,
                created,
                deleted,
                insight,
            )| {
                VersionEntry {
                    id: version_num.to_string(),
                    save_id: save_id.clone(),
                    version_num,
                    parent_version: parent,
                    file_count,
                    total_size_bytes: size,
                    is_pinned,
                    device_name,
                    created_at: created,
                    deleted_at: deleted,
                    // Stored state, so a value this binary can't read is a row
                    // without a label, never a failed listing.
                    insight: insight.and_then(|v| serde_json::from_value(v).ok()),
                }
            },
        )
        .collect::<Vec<VersionEntry>>();

    // Label whatever this listing found unlabelled. Everything uploaded before
    // the server derived any of this has no insight, and the history page is
    // exactly where someone notices; computing it here means the next load
    // shows it, without a migration that walks every version of every user.
    let pending: Vec<i64> = out
        .iter()
        .filter(|v: &&VersionEntry| crate::insight::needs_refresh(v.insight.as_ref()))
        .map(|v| v.version_num)
        .take(crate::insight::BACKFILL_PER_LISTING)
        .collect();
    if !pending.is_empty() {
        let pool = state.pool.clone();
        let sid = save_id.clone();
        tokio::spawn(async move {
            for version in pending {
                if let Err(e) = crate::insight::record_cloud(&pool, &sid, version).await {
                    tracing::debug!(error = ?e, save_id = %sid, version, "insight: backfill failed");
                }
            }
        });
    }

    Ok(Json(out))
}

/// `DELETE /v1/cloud/saves/:save_id/versions/:version`: drop a single
/// committed version. We delete the R2 blob (best-effort) then the row (the
/// storage_bytes trigger credits the freed space). If the deleted version was
/// the save's head, `latest_version_num` is repointed at the highest
/// remaining committed version; if none remain the whole save row goes so it
/// stops showing up empty in the manifest.
pub async fn delete_version(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
    Path((save_id, version)): Path<(String, i64)>,
) -> Result<Response, CloudError> {
    let owner: Option<Uuid> = sqlx::query_scalar("SELECT user_id FROM saves WHERE id = $1")
        .bind(&save_id)
        .fetch_optional(&state.pool)
        .await?;
    let Some(owner) = owner else {
        return Err(CloudError::NotFound("save not found"));
    };
    if owner != user.user_id {
        return Err(CloudError::Forbidden("save belongs to a different user"));
    }

    let row: Option<(String, bool)> = sqlx::query_as(
        "SELECT r2_key, content_addressed FROM save_versions WHERE save_id = $1 AND version_num = $2",
    )
    .bind(&save_id)
    .bind(version)
    .fetch_optional(&state.pool)
    .await?;
    let Some((r2_key, content_addressed)) = row else {
        return Err(CloudError::NotFound("version not found"));
    };

    if content_addressed {
        // Gather this version's distinct blobs before the manifest cascades
        // away, then drop the version row (cascades save_version_files) and
        // release one reference per blob.
        let shas: Vec<(String,)> = sqlx::query_as(
            "SELECT DISTINCT sha256 FROM save_version_files WHERE save_id = $1 AND version_num = $2",
        )
        .bind(&save_id)
        .bind(version)
        .fetch_all(&state.pool)
        .await?;

        sqlx::query("DELETE FROM save_versions WHERE save_id = $1 AND version_num = $2")
            .bind(&save_id)
            .bind(version)
            .execute(&state.pool)
            .await?;

        release_blobs(&state, user.user_id, shas.into_iter().map(|(s,)| (s, 1))).await;
    } else {
        if let Err(e) = state.r2.delete_object(&r2_key).await {
            tracing::warn!(error = %e, r2_key = %r2_key, "cloud delete version: R2 object delete failed");
        }
        sqlx::query("DELETE FROM save_versions WHERE save_id = $1 AND version_num = $2")
            .bind(&save_id)
            .bind(version)
            .execute(&state.pool)
            .await?;
    }

    // Repoint head / drop the save if it's now empty.
    let new_head: Option<(i64,)> = sqlx::query_as(
        "SELECT MAX(version_num) FROM save_versions WHERE save_id = $1 AND sha256 <> ''",
    )
    .bind(&save_id)
    .fetch_optional(&state.pool)
    .await?;
    match new_head.and_then(|(v,)| (v > 0).then_some(v)) {
        Some(head) => {
            sqlx::query(
                "UPDATE saves SET latest_version_num = $1, updated_at = now() WHERE id = $2",
            )
            .bind(head)
            .bind(&save_id)
            .execute(&state.pool)
            .await?;
        }
        None => {
            sqlx::query("DELETE FROM saves WHERE id = $1 AND user_id = $2")
                .bind(&save_id)
                .bind(user.user_id)
                .execute(&state.pool)
                .await?;
        }
    }

    Ok(StatusCode::NO_CONTENT.into_response())
}

/// `DELETE /v1/cloud/saves/:save_id`: wipe a cloud save and every version
/// it holds so the user can reclaim storage. We drop the R2 blobs first
/// (best-effort) then cascade-delete the DB rows; the storage_bytes trigger
/// credits the freed space back as each `save_versions` row goes.
pub async fn delete_save(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
    Path(save_id): Path<String>,
) -> Result<Response, CloudError> {
    // Owner check: never trust a save_id from the request.
    let owner: Option<Uuid> = sqlx::query_scalar("SELECT user_id FROM saves WHERE id = $1")
        .bind(&save_id)
        .fetch_optional(&state.pool)
        .await?;
    let Some(owner) = owner else {
        return Err(CloudError::NotFound("save not found"));
    };
    if owner != user.user_id {
        return Err(CloudError::Forbidden("save belongs to a different user"));
    }

    // Legacy archive versions: one opaque R2 object each. Purge before the
    // rows (and their keys) cascade away.
    let keys: Vec<(String,)> = sqlx::query_as(
        "SELECT r2_key FROM save_versions
            WHERE save_id = $1 AND content_addressed = FALSE AND r2_key <> ''",
    )
    .bind(&save_id)
    .fetch_all(&state.pool)
    .await?;
    for (key,) in &keys {
        if let Err(e) = state.r2.delete_object(key).await {
            // A leaked blob is recoverable by a later sweep; a 500 here would
            // strand the row pointing at a key we already tried to drop.
            tracing::warn!(error = %e, r2_key = %key, "cloud delete: R2 object delete failed");
        }
    }

    // Content-addressed versions: release one reference per distinct version of
    // this save that pointed at each blob. Read the counts before the manifest
    // cascades away with the save.
    let blob_refs: Vec<(String, i64)> = sqlx::query_as(
        "SELECT encode(sha256, 'hex'), COUNT(DISTINCT version_num) FROM save_version_files
            WHERE save_id = $1 GROUP BY sha256",
    )
    .bind(&save_id)
    .fetch_all(&state.pool)
    .await?;

    // Cascade: deleting the save removes its save_versions + save_version_files
    // rows (FK ON DELETE CASCADE). The save_versions storage trigger skips
    // content-addressed rows; blob storage is credited as refcounts hit 0 below.
    sqlx::query("DELETE FROM saves WHERE id = $1 AND user_id = $2")
        .bind(&save_id)
        .bind(user.user_id)
        .execute(&state.pool)
        .await?;

    release_blobs(&state, user.user_id, blob_refs).await;

    Ok(StatusCode::NO_CONTENT.into_response())
}

/// Body for `PATCH /v1/cloud/saves/:save_id` ??? just the new label.
#[derive(Debug, Deserialize)]
pub struct RenameSaveRequest {
    pub label: String,
}

/// Wire shape mirroring the agent's `Save` struct so the client can
/// deserialize the rename response into the same type self-hosted uses.
/// Omits the optional fields (`user_id`, `snapshot_count`,
/// `total_size_bytes`) ??? the agent struct defaults them on absence.
#[derive(Debug, Serialize)]
pub struct SaveSummary {
    pub id: String,
    pub game_slug: String,
    pub label: String,
    pub latest_version_num: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: time::OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: time::OffsetDateTime,
}

/// `PATCH /v1/cloud/saves/:save_id` ??? rename a cloud save's label. The cloud
/// analogue of the self-hosted `PATCH /v1/saves/:id`. Enforces
/// The saves row an upload belongs to: `(id, head_version)`, creating it when
/// this really is a new save.
///
/// Two ways a client can be out of step with the row, and the upsert this
/// replaces could only survive one of them at a time:
///
/// * **The label is stale.** Another machine renamed the save; this one still
///   uploads under the name it last saw. Conflicting on
///   `(user_id, game_slug, label)` matched nothing, so Postgres tried a real
///   insert of an `id` that already existed and raised a unique violation on the
///   primary key, surfacing as `cloud cas init: server error (500)` on a save
///   whose only sin was being renamed elsewhere.
/// * **The id is stale.** The client minted its own `save_id` and the row in the
///   cloud has a different one (a re-install, a state wipe, a self-hosted server
///   rebuilt). Conflicting on `id` alone would insert a second row for a
///   `(user, game, label)` that is already taken, and die on *that* constraint
///   instead. One real account had both at once: the desktop's `main` row for a
///   game carried an id the cloud had never heard of, and had been quietly
///   riding the label conflict for months.
///
/// So neither key alone is enough and Postgres takes one conflict target per
/// statement. Resolve first, in the order identity actually runs: the `id` names
/// the row, the label only names a slot within a game.
///
/// The row's label wins over the client's on purpose: adopting the incoming one
/// would silently undo the rename made on the other machine.
pub async fn resolve_save_row(
    conn: &mut sqlx::PgConnection,
    save_id: &str,
    user_id: uuid::Uuid,
    game_slug: &str,
    label: &Option<String>,
    backup_only: bool,
) -> Result<(String, i64), CloudError> {
    let by_id: Option<(String, i64)> = sqlx::query_as(
        "UPDATE saves SET updated_at = now(), backup_only = $3
         WHERE id = $1 AND user_id = $2
         RETURNING id, latest_version_num",
    )
    .bind(save_id)
    .bind(user_id)
    .bind(backup_only)
    .fetch_optional(&mut *conn)
    .await?;
    if let Some(row) = by_id {
        return Ok(row);
    }

    let label = label.clone().unwrap_or_else(|| "default".to_string());
    let by_label: Option<(String, i64)> = sqlx::query_as(
        "UPDATE saves SET updated_at = now(), backup_only = $4
         WHERE user_id = $1 AND game_slug = $2 AND label = $3
         RETURNING id, latest_version_num",
    )
    .bind(user_id)
    .bind(game_slug)
    .bind(&label)
    .bind(backup_only)
    .fetch_optional(&mut *conn)
    .await?;
    if let Some(row) = by_label {
        return Ok(row);
    }

    // Genuinely new. The conflict target still matters: two devices can race the
    // first upload of the same save, and the loser has to find the winner's row
    // rather than fail.
    Ok(sqlx::query_as(
        r#"
        INSERT INTO saves (id, user_id, game_slug, label, latest_version_num, backup_only)
        VALUES ($1, $2, $3, $4, 0, $5)
        ON CONFLICT (user_id, game_slug, label)
        DO UPDATE SET updated_at = now(), backup_only = EXCLUDED.backup_only
        RETURNING id, latest_version_num
        "#,
    )
    .bind(save_id)
    .bind(user_id)
    .bind(game_slug)
    .bind(&label)
    .bind(backup_only)
    .fetch_one(&mut *conn)
    .await?)
}

/// Is this row empty of history? Only asked when a push's base doesn't match the
/// head, and only when that head is 0.
///
/// A head of 0 on a row with no versions at all is not a divergence: there is no
/// version for another device to have advanced past, and nothing a fast-forward
/// could bury. What it actually means is that the client is carrying the cursor
/// of a previous life: the row it used to push to was deleted (a game
/// un-archived and dropped, a purge, an account rebuilt) and the one it is
/// talking to now is the empty row `resolve_save_row` just minted for it.
///
/// Rejecting that as a non-fast-forward is a trap with no exit. The client's
/// base only moves when an upload lands or a reconcile pulls a head, and here
/// neither can ever happen: the upload is refused, and the reconcile finds no
/// remote history to pull. It retries, gets the same 409, and gives up for good
/// after its conflict budget, with the rejection blaming "another device" that
/// does not exist. Worse, the row minted to answer it is rolled back with the
/// refusal, so the next attempt starts from exactly the same nothing.
///
/// Accepting is the safe half of the choice, not the risky one: an empty row has
/// no content to lose, and the push becomes version 1 of a save whose history is
/// genuinely gone.
pub async fn save_has_no_history(
    conn: &mut sqlx::PgConnection,
    save_id: &str,
    head: i64,
) -> Result<bool, CloudError> {
    if head != 0 {
        return Ok(false);
    }
    // `latest_version_num` is the fast answer, but it is bookkeeping; the
    // versions are the fact. Only reached on a mismatch against head 0, so this
    // costs nothing on the hot path.
    let existing: i64 = sqlx::query_scalar("SELECT count(*) FROM save_versions WHERE save_id = $1")
        .bind(save_id)
        .fetch_one(&mut *conn)
        .await?;
    Ok(existing == 0)
}

/// Does this push already carry everything the head has?
///
/// The second question asked of a mismatched base, after `save_has_no_history`,
/// and the one that covers a head with real content. A non-fast-forward is
/// refused to stop a push from burying a version it never saw, but a manifest
/// that lists every (path, sha) the head lists cannot bury it: the content is
/// all still there, in the version about to be written, byte for byte. What the
/// client is doing then is not overwriting the head, it is descending from it
/// with extra files or none.
///
/// This is the same judgement the agent makes for itself when a reconcile tells
/// it "your folder already holds the head" and it rebases onto that head, but
/// made here, from the manifest already in the request, so it also works for
/// clients too old to make it. Those clients could not: reading the head out of
/// a 409 body only landed in ago-2026, and before it a rejection left them
/// knowing they diverged but not from what. One save spent ten days retrying
/// every ten minutes against a head of 2 it already contained.
///
/// A *strict* superset: the push must carry the whole head **and** something of
/// its own. Both halves earn their place. Dropping a file the head has is
/// exactly the burial the 409 exists for, and still gets one. Carrying the head
/// and nothing else is a client that has no new content to write: the agent
/// settles on the head rather than re-uploading it, and minting an identical
/// version here would pad the history for a device that only lost its place.
pub async fn manifest_covers_head(
    conn: &mut sqlx::PgConnection,
    save_id: &str,
    head: i64,
    files: &[CasFileEntry],
) -> Result<bool, CloudError> {
    if head <= 0 {
        return Ok(false);
    }
    let head_files: Vec<(String, String)> = sqlx::query_as(
        "SELECT relative_path, encode(sha256, 'hex') FROM save_version_files
          WHERE save_id = $1 AND version_num = $2",
    )
    .bind(save_id)
    .bind(head)
    .fetch_all(&mut *conn)
    .await?;
    // A head with no manifest rows is not something to reason about: it is a
    // pending version, or one from before content addressing. Say no and let
    // the ordinary rejection stand.
    if head_files.is_empty() {
        return Ok(false);
    }
    let incoming: std::collections::HashSet<(&str, &str)> = files
        .iter()
        .map(|f| (f.relative_path.as_str(), f.sha256.as_str()))
        .collect();
    let covers_all = head_files
        .iter()
        .all(|(path, sha)| incoming.contains(&(path.as_str(), sha.as_str())));
    Ok(covers_all && incoming.len() > head_files.len())
}

/// `UNIQUE(user_id, game_slug, label)` -> 409 on collision. R2 keys are keyed
/// by `save_id` + version (not by label), so no blob rename is needed ??? just
/// the DB row.
pub async fn rename_save(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
    Path(save_id): Path<String>,
    Json(body): Json<RenameSaveRequest>,
) -> Result<Json<SaveSummary>, CloudError> {
    let new_label = body.label.trim();
    if new_label.is_empty() {
        return Err(CloudError::BadRequest("label can't be empty".to_string()));
    }

    // Owner check ??? never trust a save_id from the request.
    let row: Option<(Uuid, String, String)> =
        sqlx::query_as("SELECT user_id, game_slug, label FROM saves WHERE id = $1")
            .bind(&save_id)
            .fetch_optional(&state.pool)
            .await?;
    let Some((owner, game_slug, old_label)) = row else {
        return Err(CloudError::NotFound("save not found"));
    };
    if owner != user.user_id {
        return Err(CloudError::Forbidden("save belongs to a different user"));
    }

    if new_label == old_label {
        // No-op: return the current state without touching the DB.
        return fetch_save_summary(&state, &save_id, &game_slug).await;
    }

    // UPDATE with UNIQUE(user_id, game_slug, label) -> 409 on collision.
    let result = sqlx::query(
        "UPDATE saves SET label = $1, updated_at = now() WHERE id = $2 AND user_id = $3",
    )
    .bind(new_label)
    .bind(&save_id)
    .bind(user.user_id)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => {}
        Err(e) => {
            if e.to_string().contains("unique") || e.to_string().contains("UNIQUE") {
                return Err(CloudError::Conflict("label collision"));
            }
            return Err(CloudError::Db(e));
        }
    }

    fetch_save_summary(&state, &save_id, &game_slug).await
}

/// Fetch the current state of a save row for the rename response.
async fn fetch_save_summary(
    state: &CloudState,
    save_id: &str,
    game_slug: &str,
) -> Result<Json<SaveSummary>, CloudError> {
    let (label, latest_version_num, created_at, updated_at): (
        String,
        i64,
        time::OffsetDateTime,
        time::OffsetDateTime,
    ) = sqlx::query_as(
        "SELECT label, latest_version_num, created_at, updated_at FROM saves WHERE id = $1",
    )
    .bind(save_id)
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(SaveSummary {
        id: save_id.to_string(),
        game_slug: game_slug.to_string(),
        label,
        latest_version_num,
        created_at,
        updated_at,
    }))
}

/// Release `n` references from each blob, deleting the R2 object + row when a
/// blob's refcount reaches zero (the cloud_blobs trigger credits the freed
/// storage on the 0-transition). Best-effort: a failure here only leaks a blob,
/// recoverable by a later sweep, so errors are logged not propagated.
pub(crate) async fn release_blobs<I>(state: &CloudState, user_id: Uuid, blobs: I)
where
    I: IntoIterator<Item = (String, i64)>,
{
    for (sha, dec) in blobs {
        let row: Result<Option<(i64,)>, _> = sqlx::query_as(
            "UPDATE cloud_blobs SET refcount = GREATEST(0, refcount - $3)
                WHERE user_id = $1 AND sha256 = decode($2, 'hex')
             RETURNING refcount",
        )
        .bind(user_id)
        .bind(&sha)
        .bind(dec)
        .fetch_optional(&state.pool)
        .await;
        match row {
            Ok(Some((refcount,))) if refcount <= 0 => {
                let key = r2::key_for_blob(user_id, &sha);
                if let Err(e) = state.r2.delete_object(&key).await {
                    tracing::warn!(error = %e, r2_key = %key, "cloud blob GC: R2 delete failed");
                }
                if let Err(e) =
                    sqlx::query("DELETE FROM cloud_blobs WHERE user_id = $1 AND sha256 = $2")
                        .bind(user_id)
                        .bind(&sha)
                        .execute(&state.pool)
                        .await
                {
                    tracing::warn!(error = %e, sha = %sha, "cloud blob GC: row delete failed");
                }
            }
            Ok(_) => {}
            Err(e) => {
                tracing::warn!(error = %e, sha = %sha, "cloud blob GC: refcount decrement failed");
            }
        }
    }
}

/// Record a rejected sync in `sync_log` so failed syncs land in the same
/// analytics stream as successful uploads/downloads. `kind` is `'quota_block'`
/// (over storage limit) or `'bandwidth_block'` (over the moving bandwidth
/// window). Both extend the enum documented in migration 0006. Without this a
/// sync that 402/429s leaves no trace and the failure rate is invisible.
///
/// Best-effort: a logging failure must never turn a clean rejection into a 500.
/// The FK `save_id` column stays NULL because on the init paths the `saves` row
/// may not exist yet; the save id and game slug always ride in `metadata`.
async fn log_sync_block(
    state: &CloudState,
    user_id: Uuid,
    kind: &str,
    save_id: &str,
    game_slug: Option<&str>,
    bytes: u64,
) {
    let meta = serde_json::json!({ "save_id": save_id, "game_slug": game_slug }).to_string();
    if let Err(e) = sqlx::query(
        "INSERT INTO sync_log (user_id, kind, bytes, metadata)
             VALUES ($1, $2, $3, $4::jsonb)",
    )
    .bind(user_id)
    .bind(kind)
    .bind(i64::try_from(bytes).unwrap_or(i64::MAX))
    .bind(meta)
    .execute(&state.pool)
    .await
    {
        tracing::warn!(error = %e, %user_id, %kind, "failed to record sync block in sync_log");
    }
}

/// A quota refusal, plus a wait once the account has been refused often enough
/// this hour to prove that nobody on the other end is listening.
///
/// The plain 402 stays the default, and it is the honest answer: an account
/// that fills up earns one refusal per tracked save on the sweep that
/// discovers it, and a dozen refusals in ten seconds is a library, not a loop.
/// What stops being honest is answering the same 402 to the same client 342
/// times in three hours (aug-2026, one account; 148 in a day, another). Every
/// client shipped up to v1.1.2 reads that 402 as a failure of *this* save and
/// comes straight back with the next one. Those same clients do understand a
/// 429, so past the threshold that is what they get.
///
/// The quota figures are reloaded for the paced body rather than pulled out of
/// `plain`, because by then it is a serialised `Response` and there is nothing
/// left to read. A window that says "slow down" without saying "you are full"
/// sends the user looking for a network problem they don't have.
async fn paced_quota_reject(
    state: &CloudState,
    user_id: Uuid,
    save_id: &str,
    game_slug: Option<&str>,
    requested: u64,
    plain: Response,
) -> Response {
    log_sync_block(state, user_id, "quota_block", save_id, game_slug, requested).await;
    let Some(pace) = loopguard::quota_brake(state, user_id).await else {
        return plain;
    };
    tracing::warn!(
        %user_id,
        %save_id,
        blocks_last_hour = pace.seen,
        retry_after_secs = pace.retry_after_secs,
        "cloud: pacing a full account that keeps retrying"
    );
    let detail = match quota::load(&state.pool, user_id).await {
        Ok(Some((_, info))) => serde_json::json!({
            "plan": info.plan,
            "used_bytes": info.used_bytes,
            "limit_bytes": info.limit_bytes,
            "requested_bytes": requested,
            "upgrade_url": state
                .config
                .cloud
                .as_ref()
                .map(|c| c.upgrade_url.clone())
                .unwrap_or_else(crate::config::default_upgrade_url),
        }),
        _ => serde_json::json!({ "requested_bytes": requested }),
    };
    loopguard::quota_paced_response(pace, detail)
}

/// How many downloads of the *same* (user, save, version) inside 24h stop
/// looking like a user and start looking like a loop.
///
/// Deliberately lax, because it's allowed to be: this only writes a log line.
/// Re-restoring one version two or three times in a day is ordinary: testing a
/// save, hopping between machines, undoing a bad session. By five there's no
/// benign reading left. A false positive costs one WARN; a false negative cost
/// us eight days and ~60 GB/day of the July-2026 incident, so if this number is
/// wrong it's wrong on the high side.
///
/// Note the interaction with the client-side escalating backoff (hoard-agent's
/// `AUTO_RESTORE_FAILURE_BACKOFF_SECS`): a *fixed* client now retries ~24×/day
/// at worst, not ~1440×, so reaching 5 takes far longer than it did during the
/// incident. That's the point: the threshold still trips, but tripping now
/// means "this has been broken for a while", which is exactly the signal an
/// operator wants. An old or third-party client with no backoff still trips it
/// within minutes.
const REPEAT_DOWNLOAD_WARN_THRESHOLD: i64 = 5;

/// Emit an operator signal when the same save version is downloaded over and
/// over by the same user inside 24h.
///
/// Only a log line, the *first* of two thresholds on the same signal. This one
/// is allowed to cry wolf at five, because all it costs is a WARN and its job is
/// to stop the loop being invisible (it was found by chance, eight days and
/// 10,6 GB in). The one that changes what the client gets lives in
/// [`crate::cloud::loopguard`] and sits higher up, at eight, where no benign
/// reading survives. Keep them in that order: a signal that only ever fires
/// together with the brake tells an operator nothing they can act on early.
///
/// Best-effort by construction: if the count query hiccups we log at debug and
/// move on. Failing a paid download because an observability query timed out
/// would be a worse bug than the one this detects.
async fn warn_on_repeat_download(
    state: &CloudState,
    user_id: Uuid,
    save_id: &str,
    game_slug: &str,
    version: i64,
) {
    // Counts the row the caller just inserted, so `n` is "including this one".
    // Backed by idx_sync_log_repeat_download (migration 0034).
    let row: Result<(i64,), _> = sqlx::query_as(
        "SELECT count(*) FROM sync_log
             WHERE user_id = $1 AND save_id = $2 AND version_num = $3
               AND kind = 'download' AND at > now() - interval '24 hours'",
    )
    .bind(user_id)
    .bind(save_id)
    .bind(version)
    .fetch_one(&state.pool)
    .await;
    match row {
        Ok((n,)) if n >= REPEAT_DOWNLOAD_WARN_THRESHOLD => {
            tracing::warn!(
                %user_id,
                %save_id,
                %game_slug,
                version_num = version,
                downloads_24h = n,
                "cloud: repeated downloads of the same save version — possible client restore loop"
            );
        }
        Ok(_) => {}
        Err(e) => {
            tracing::debug!(
                error = %e,
                %user_id,
                %save_id,
                "cloud: repeat-download counter query failed; ignoring"
            );
        }
    }
}

/// Tiny one-query helper to fetch a user's plan tag without going through
/// the full quota::load pipeline. Used by paths that need the plan but
/// don't need (yet) the storage figures.
async fn plan_for_user(state: &CloudState, user_id: Uuid) -> Result<Option<Plan>, CloudError> {
    let row: Option<(String,)> = sqlx::query_as("SELECT plan FROM profiles WHERE user_id = $1")
        .bind(user_id)
        .fetch_optional(&state.pool)
        .await?;
    Ok(row.map(|r| Plan::from_str(&r.0).unwrap_or(Plan::Free)))
}
