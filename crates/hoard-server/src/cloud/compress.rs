//! Background at-rest compression of content-addressed blobs.
//!
//! Clients upload raw bytes to presigned PUTs and that never changes; this
//! sweep walks blobs older than `min_age_hours` and rewrites the R2 object
//! as zstd, in place, at the same key. `cloud_blobs.size_bytes` keeps the
//! RAW size, since it feeds the storage trigger and everything the user sees,
//! while `stored_bytes` records what R2 actually bills. Compressed blobs
//! are served through the decompressing `/v1/cloud/blob/:token` proxy
//! (`routes::blob_proxy`), so any client, past or future, keeps receiving
//! byte-identical raw data. Steady state is "everything older than the age
//! gate is compressed": each tick drains batches back-to-back until no
//! eligible blob remains, then idles.
//!
//! The raw object is never destroyed unverified. Per blob:
//!
//!   1. claim the row (`encoding='zstd'`, `stored_bytes` NULL). From here
//!      the manifest routes hand out proxy URLs; while the object is still
//!      raw the proxy passes it through untouched (it only decompresses
//!      once `stored_bytes` lands).
//!   2. compress into a sibling `<key>.ztmp` object; the raw object is
//!      not touched.
//!   3. verify: stream `.ztmp` back, decompress, SHA-256 must equal the
//!      blob's content hash. A mismatch aborts, deletes the tmp and
//!      un-claims the row; the raw object was never at risk.
//!   4. only then overwrite the real key with the verified bytes
//!      (multipart, atomic on complete), re-verify the final object the
//!      same way, delete the tmp and finalize `stored_bytes`.
//!
//! Blobs that don't gain from compression (already-compressed save
//! formats) are left raw and marked `encoding = 'raw'`: a terminal state
//! the sweep skips and every reader treats as uncompressed.
//!
//! A blob whose R2 object no longer exists (refcount desync from a past
//! incident) is marked `encoding = 'missing'`, also terminal. Retrying
//! can't conjure the bytes back, and leaving the row claimed-but-eligible
//! is how a ≥`batch` cohort of them once turned the drain loop into a
//! permanent hot loop of HEADs (~10/s of Class B ops, 2026-07-18). For the
//! same reason the drain only keeps going while a batch actually makes
//! progress: persistent failures wait for the next tick and, after
//! [`MAX_ATTEMPTS`] of them, stop being picked up at all. "Waits for the
//! next tick" is not an exit: a blob that fails deterministically (stored
//! bytes that don't hash to `sha256`) retried every tick forever until the
//! attempt counter was added.
//!
//! Crash at any point is healed on retry: the claim keeps the blob in the
//! sweep, and a final object whose size differs from `size_bytes` is
//! re-verified by content, never re-compressed blind (zstd output can't
//! equal its input length, so the size test is a reliable "overwrite
//! already landed" signal). Races with deletion (version GC, account
//! purge, archive expiry) stay benign: everything deletes by the row's
//! `r2_key`, which never changes; if the row vanishes mid-flight the
//! freshly-written objects are removed instead of finalized.

use std::sync::Arc;
use std::time::Duration;

use async_compression::tokio::bufread::{ZstdDecoder, ZstdEncoder};
use async_compression::Level;
use sha2::{Digest, Sha256};
use sqlx::Row;
use tokio::io::AsyncReadExt;
use uuid::Uuid;

use crate::cloud::state::CloudState;
use crate::config::CompressionConfig;

/// Don't bother below this raw size: zstd frame overhead and the extra
/// Class A ops eat the win on tiny blobs.
const MIN_BLOB_BYTES: i64 = 4096;

/// Give up on a blob after this many failed attempts. The terminal states
/// above ('raw', 'missing') only cover blobs the sweep *evaluated*; a blob
/// that keeps failing used to have no exit, because the verification path
/// un-claims the row back to `encoding IS NULL`, exactly what the
/// eligibility query picks up. A blob whose stored bytes don't hash to
/// `sha256` therefore re-downloaded, re-compressed and re-verified itself
/// every tick forever (six of them ran from 2026-07-11 to 2026-08-03,
/// ~1.7k attempts/day of GET + multipart PUT + verify GET, for nothing).
/// The cap makes permanent failure cost a bounded number of R2 ops. It
/// counts *attempts*, not consecutive failures, so a transient R2 error
/// spends one too: with five of them a real blip still gets retried, and
/// a blob that is genuinely broken stops after ~25 minutes.
const MAX_ATTEMPTS: i16 = 5;

pub fn spawn(state: CloudState) {
    let Some(cfg) = state.config.cloud.as_ref().map(|c| c.compression.clone()) else {
        return;
    };
    if !cfg.enabled {
        return;
    }
    let cfg = Arc::new(cfg);
    tokio::spawn(async move {
        tracing::info!(
            min_age_hours = cfg.min_age_hours,
            batch = cfg.batch,
            level = cfg.level,
            "blob compression sweep: enabled"
        );
        let mut tick = tokio::time::interval(Duration::from_secs(cfg.sweep_secs.max(30)));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            // Drain: keep pulling full batches until the backlog is empty,
            // so "older than min_age ⇒ compressed" holds instead of
            // trickling `batch` blobs per tick forever. Only while batches
            // make progress, though: a full batch of failures must wait
            // for the next tick, not respin the same rows back-to-back.
            loop {
                match sweep_once(&state, &cfg).await {
                    Ok((picked, ok)) => {
                        if ok > 0 {
                            tracing::info!(blobs = ok, "blob compression sweep: batch done");
                        }
                        if picked < cfg.batch as usize || ok == 0 {
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "blob compression sweep failed");
                        break;
                    }
                }
            }
        }
    });
}

/// One batch: claim up to `batch` eligible blobs, compress each. Returns
/// `(picked, ok)`: how many were picked up and how many of those finished
/// (compressed, kept raw, or marked missing). Failures retry on a later
/// tick; the caller uses `ok` to stop draining when nothing progresses.
async fn sweep_once(state: &CloudState, cfg: &CompressionConfig) -> anyhow::Result<(usize, usize)> {
    // Eligible: raw (or claimed-but-unfinished) blobs old enough, with no
    // recent direct-download URL out in the wild, still referenced and not
    // frozen in the archive grace window.
    let rows = sqlx::query(
        r#"
        SELECT user_id, encode(sha256, 'hex') AS sha256, size_bytes
          FROM cloud_blobs
         WHERE (encoding IS NULL OR (encoding = 'zstd' AND stored_bytes IS NULL))
           AND compress_attempts < $5
           AND size_bytes >= $1
           AND refcount > 0
           AND purge_after IS NULL
           AND created_at < now() - make_interval(hours => $2)
           AND (last_presigned_at IS NULL
                OR last_presigned_at < now() - make_interval(hours => $3))
         ORDER BY created_at
         LIMIT $4
        "#,
    )
    .bind(MIN_BLOB_BYTES)
    .bind(cfg.min_age_hours as i32)
    .bind(cfg.idle_hours as i32)
    .bind(cfg.batch as i64)
    .bind(MAX_ATTEMPTS)
    .fetch_all(&state.pool)
    .await?;

    let n = rows.len();
    let mut ok = 0usize;
    for row in rows {
        let user_id: Uuid = row.get("user_id");
        let sha: String = row.get("sha256");
        let key = super::r2::key_for_blob(user_id, &sha);
        let raw_size: i64 = row.get("size_bytes");
        match compress_one(state, cfg, user_id, &sha, &key, raw_size).await {
            Ok(()) => ok += 1,
            Err(e) => {
                // Charge the attempt. Once it hits the cap the eligibility
                // query stops returning this blob, so a permanently broken
                // one can't keep spending R2 ops every tick.
                let attempts: Option<i16> = sqlx::query_scalar(
                    "UPDATE cloud_blobs SET compress_attempts = compress_attempts + 1
                      WHERE user_id = $1 AND sha256 = decode($2, 'hex')
                      RETURNING compress_attempts",
                )
                .bind(user_id)
                .bind(&sha)
                .fetch_optional(&state.pool)
                .await?;
                if attempts.is_some_and(|a| a >= MAX_ATTEMPTS) {
                    // Terminal, and deliberately *without* un-claiming. A row
                    // giving up while still `encoding='zstd', stored_bytes
                    // NULL` keeps being served through the decompressing
                    // proxy, which costs egress the presigned path wouldn't,
                    // but resetting it to NULL is not the fix. That state is
                    // ambiguous: phase 4 may have already overwritten the
                    // object with zstd bytes and died before `finalize`, and
                    // calling that raw would hand clients compressed bytes as
                    // if they were the save. Cheap and correct beats cheaper
                    // and corrupt; the healing path reclaims it if it can.
                    //
                    // The object stays readable either way, but a blob that
                    // never round-trips to its own content hash is a blob
                    // whose stored bytes disagree with what the client
                    // uploaded, and every version referencing it restores
                    // wrong.
                    tracing::error!(
                        error = %e,
                        %user_id,
                        sha,
                        attempts = MAX_ATTEMPTS,
                        "blob compress gave up after {MAX_ATTEMPTS} attempts — \
                         left raw and dropped from the sweep; if this was a \
                         verification failure the stored object does not match \
                         its content hash and needs investigating"
                    );
                } else {
                    tracing::warn!(error = %e, %user_id, sha, "blob compress failed; will retry");
                }
            }
        }
    }
    Ok((n, ok))
}

async fn compress_one(
    state: &CloudState,
    cfg: &CompressionConfig,
    user_id: Uuid,
    sha: &str,
    key: &str,
    raw_size: i64,
) -> anyhow::Result<()> {
    // Phase 1: claim. From here the manifest route proxies this blob.
    let claimed = sqlx::query(
        "UPDATE cloud_blobs SET encoding = 'zstd'
          WHERE user_id = $1 AND sha256 = decode($2, 'hex')
            AND (encoding IS NULL OR (encoding = 'zstd' AND stored_bytes IS NULL))",
    )
    .bind(user_id)
    .bind(sha)
    .execute(&state.pool)
    .await?;
    if claimed.rows_affected() == 0 {
        return Ok(());
    }

    let tmp_key = format!("{key}.ztmp");

    // The object is gone but the row survived (refcount desync from a past
    // incident). Terminal 'missing' state: retrying can't conjure the bytes
    // back, and leaving the row eligible is what once melted the drain loop
    // into a HEAD-per-retry hot loop. Loud on purpose: every referencing
    // version is unrestorable and someone should look at why.
    let Some(head) = state.r2.head(key).await? else {
        sqlx::query(
            "UPDATE cloud_blobs SET encoding = 'missing'
              WHERE user_id = $1 AND sha256 = decode($2, 'hex')
                AND encoding = 'zstd' AND stored_bytes IS NULL",
        )
        .bind(user_id)
        .bind(sha)
        .execute(&state.pool)
        .await?;
        tracing::warn!(
            %user_id,
            sha,
            key,
            "blob object missing in R2 — marked encoding='missing'; referencing versions are unrestorable"
        );
        return Ok(());
    };

    // Retry healing: an object whose size differs from the raw size means a
    // previous attempt already landed the overwrite. Verify it by content
    // (never re-compress blind, which would double-compress) and finalize.
    if head != raw_size {
        if decompressed_sha_matches(state, key, sha).await? {
            let _ = state.r2.delete_object(&tmp_key).await;
            return finalize(state, user_id, sha, key, &tmp_key, head, raw_size).await;
        }
        // Final object doesn't verify. The tmp (kept until after a good
        // final verify) is the recovery source.
        anyhow::ensure!(
            decompressed_sha_matches(state, &tmp_key, sha).await?,
            "final and tmp both fail verification for {key} — leaving untouched for inspection"
        );
        let reader = state.r2.get_reader(&tmp_key).await?;
        let stored = state.r2.put_from_reader(key, reader).await?;
        anyhow::ensure!(
            decompressed_sha_matches(state, key, sha).await?,
            "re-copied object failed verification for {key}"
        );
        let _ = state.r2.delete_object(&tmp_key).await;
        return finalize(state, user_id, sha, key, &tmp_key, stored, raw_size).await;
    }

    // Phase 2: compress into the sibling tmp object. The raw object is not
    // touched yet.
    let reader = state.r2.get_reader(key).await?;
    let encoder = ZstdEncoder::with_quality(reader, Level::Precise(cfg.level));
    let stored = state.r2.put_from_reader(&tmp_key, encoder).await?;

    // Not worth it? Plenty of game saves are already-compressed formats and
    // zstd only adds frame overhead. Keep the raw object, drop the tmp and
    // mark the row `encoding = 'raw'`, a terminal "evaluated, leave alone"
    // state the sweep never re-picks and every reader treats as raw (the
    // proxy and manifest only special-case 'zstd').
    if stored as f64 >= raw_size as f64 * 0.98 {
        let _ = state.r2.delete_object(&tmp_key).await;
        sqlx::query(
            "UPDATE cloud_blobs SET encoding = 'raw'
              WHERE user_id = $1 AND sha256 = decode($2, 'hex')
                AND encoding = 'zstd' AND stored_bytes IS NULL",
        )
        .bind(user_id)
        .bind(sha)
        .execute(&state.pool)
        .await?;
        tracing::info!(
            %user_id,
            sha,
            raw_bytes = raw_size,
            compressed_bytes = stored,
            "blob incompressible — kept raw"
        );
        return Ok(());
    }

    // Phase 3: verify the tmp round-trips to the exact content hash. This
    // also proves the raw source itself was intact (the hash chain starts
    // at the client's original bytes).
    if !decompressed_sha_matches(state, &tmp_key, sha).await? {
        let _ = state.r2.delete_object(&tmp_key).await;
        // Un-claim so the manifest goes back to the direct presigned path;
        // the raw object is untouched. Ending up here repeatedly points at
        // a corrupt source object. Loud on purpose.
        sqlx::query(
            "UPDATE cloud_blobs SET encoding = NULL
              WHERE user_id = $1 AND sha256 = decode($2, 'hex') AND stored_bytes IS NULL",
        )
        .bind(user_id)
        .bind(sha)
        .execute(&state.pool)
        .await?;
        anyhow::bail!("compressed tmp failed verification for {key} — raw left as-is");
    }

    // Phase 4: overwrite the real key with the verified bytes and verify
    // the final object too before dropping the tmp.
    let reader = state.r2.get_reader(&tmp_key).await?;
    let final_size = state.r2.put_from_reader(key, reader).await?;
    anyhow::ensure!(
        final_size == stored,
        "final copy size mismatch for {key}: {final_size} != {stored}"
    );
    anyhow::ensure!(
        decompressed_sha_matches(state, key, sha).await?,
        "final object failed verification for {key} — tmp retained for recovery"
    );
    let _ = state.r2.delete_object(&tmp_key).await;

    finalize(state, user_id, sha, key, &tmp_key, stored, raw_size).await
}

/// Record `stored_bytes`, closing the two-phase dance. If the row vanished
/// mid-flight (purge won a race after our claim), the freshly-written
/// objects are orphans nobody references, so delete them.
async fn finalize(
    state: &CloudState,
    user_id: Uuid,
    sha: &str,
    key: &str,
    tmp_key: &str,
    stored: i64,
    raw_size: i64,
) -> anyhow::Result<()> {
    let finalized = sqlx::query(
        "UPDATE cloud_blobs SET stored_bytes = $3
          WHERE user_id = $1 AND sha256 = decode($2, 'hex') AND encoding = 'zstd'",
    )
    .bind(user_id)
    .bind(sha)
    .bind(stored)
    .execute(&state.pool)
    .await?;
    if finalized.rows_affected() == 0 {
        tracing::warn!(%user_id, sha, "blob row vanished during compress — deleting orphan objects");
        let _ = state.r2.delete_object(tmp_key).await;
        state.r2.delete_object(key).await?;
        return Ok(());
    }
    tracing::info!(
        %user_id,
        sha,
        raw_bytes = raw_size,
        stored_bytes = stored,
        ratio = format!("{:.2}", stored as f64 / raw_size.max(1) as f64),
        "blob compressed at rest (verified)"
    );
    Ok(())
}

/// Stream the object, decompress, and compare the SHA-256 of the output
/// against the blob's content hash. Bounded memory. The hash chain reaches
/// back to the client's original upload, so a `true` here is an end-to-end
/// integrity proof of the compressed artifact.
async fn decompressed_sha_matches(
    state: &CloudState,
    key: &str,
    want_hex: &str,
) -> anyhow::Result<bool> {
    let reader = state.r2.get_reader(key).await?;
    let mut dec = ZstdDecoder::new(reader);
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 256 * 1024];
    loop {
        let n = dec.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()) == want_hex)
}
