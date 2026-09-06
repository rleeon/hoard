//! Account-export worker.
//!
//! `POST /v1/me/export` only enqueues a row in `export_jobs` (status
//! `pending`). This background task is what actually fulfils it: it claims a
//! pending job, bundles the latest live version of every one of the user's
//! saves into a single ZIP, streams it to R2 under `exports/<user>/<job>.zip`,
//! marks the job `done` with a size + 7-day expiry, and emails the user a
//! download link (best-effort; the link is also surfaced in-app via
//! `GET /v1/me/export`, so email being unconfigured doesn't break the feature).
//!
//! Content-addressed versions (the current format) are expanded to their real
//! files at `<game>/<label>/<relative_path>`; legacy opaque versions are
//! included as-is (`<game>/<label>_v<n>.tar.zst`) since unpacking them here
//! would duplicate the client's restore pipeline for a shrinking set of old
//! accounts.

use crate::cloud::state::CloudState;
use crate::cloud::{email, r2};
use std::io::Write;
use std::time::Duration;
use tempfile::NamedTempFile;
use time::OffsetDateTime;
use uuid::Uuid;

/// How often to look for pending jobs. Exports aren't latency-sensitive; a
/// short-ish poll keeps "request → download" to well under a minute without
/// hammering the DB.
const POLL_INTERVAL: Duration = Duration::from_secs(30);

/// A job stuck in `running` past this is treated as dead and failed. The worker
/// runs inside the API process, which Fly stops when the app scales to zero
/// (`min_machines_running = 0`); a machine killed mid-build leaves its row
/// `running` forever. Without recovery that row also wedges the feature, because
/// `create_export_job` dedupes on any pending or running job, so every retry just
/// re-polls the corpse. Reaping it to `failed` unblocks a fresh request.
const STALE_RUNNING_AFTER: Duration = Duration::from_secs(60 * 60);

/// How long a finished export ZIP lives in R2 before the sweep expires it.
/// 7 is also the SigV4 presign ceiling, so the emailed link (presigned for one
/// day less) is always valid for the object's whole life.
const EXPORT_TTL_DAYS: i64 = 7;

/// Presign TTL for the emailed download link. One day under the object's life
/// so the link never outlives the bytes it points at.
const LINK_TTL: Duration = Duration::from_secs(6 * 24 * 3600);

/// Spawn the daemon. Idempotent per process; call once from `cloud::run`.
pub fn spawn(state: CloudState) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(POLL_INTERVAL);
        loop {
            ticker.tick().await;
            // Recover jobs orphaned in `running` by a machine that stopped
            // mid-build, before claiming new work, or the dedup in
            // `create_export_job` keeps every retry pinned to the dead row.
            if let Err(e) = reap_stale(&state).await {
                tracing::warn!(error = %e, "export stale-job reap failed");
            }
            // Drain every pending job this tick, one at a time (exports are
            // heavy; serialising keeps peak memory to a single blob).
            loop {
                match claim_next(&state).await {
                    Ok(Some((job_id, user_id))) => {
                        if let Err(e) = run_job(&state, job_id, user_id).await {
                            tracing::error!(%job_id, %user_id, error = %e, "export job failed");
                            mark_failed(&state, job_id, &e.to_string()).await;
                        }
                    }
                    Ok(None) => break,
                    Err(e) => {
                        tracing::error!(error = %e, "export claim query failed");
                        break;
                    }
                }
            }
            if let Err(e) = expire_due(&state).await {
                tracing::warn!(error = %e, "export expiry sweep failed");
            }
        }
    });
}

/// Atomically claim the oldest pending job, flipping it to `running`.
/// `FOR UPDATE SKIP LOCKED` makes this safe even if two server replicas run the
/// worker: each grabs a different row, none blocks.
async fn claim_next(state: &CloudState) -> Result<Option<(Uuid, Uuid)>, sqlx::Error> {
    sqlx::query_as(
        "UPDATE export_jobs SET status = 'running', started_at = now()
           WHERE id = (
               SELECT id FROM export_jobs WHERE status = 'pending'
               ORDER BY requested_at
               LIMIT 1
               FOR UPDATE SKIP LOCKED
           )
         RETURNING id, user_id",
    )
    .fetch_optional(&state.pool)
    .await
}

/// Fail any job wedged in `running` past `STALE_RUNNING_AFTER`. Returns the
/// number reaped. Uses `started_at` (set by `claim_next`); a NULL guards the
/// rare row claimed before this column existed.
async fn reap_stale(state: &CloudState) -> Result<u64, sqlx::Error> {
    let res = sqlx::query(
        "UPDATE export_jobs
            SET status = 'failed', finished_at = now(),
                error = 'export interrupted (worker stopped mid-build); retry'
          WHERE status = 'running'
            AND started_at IS NOT NULL
            AND started_at < now() - $1::interval",
    )
    .bind(format!("{} seconds", STALE_RUNNING_AFTER.as_secs()))
    .execute(&state.pool)
    .await?;
    let n = res.rows_affected();
    if n > 0 {
        tracing::warn!(reaped = n, "reclaimed stale running export jobs");
    }
    Ok(n)
}

async fn run_job(state: &CloudState, job_id: Uuid, user_id: Uuid) -> anyhow::Result<()> {
    let (tmp, size, file_count) = build_export_zip(state, user_id).await?;

    let key = r2::key_for_export(user_id, job_id);
    state.r2.put_file(&key, tmp.path()).await?;
    // ZIP is safely in R2 now; the local temp file can go.
    drop(tmp);

    let expires = OffsetDateTime::now_utc() + time::Duration::days(EXPORT_TTL_DAYS);
    sqlx::query(
        "UPDATE export_jobs
            SET status = 'done', r2_key = $2, size_bytes = $3,
                finished_at = now(), expires_at = $4, error = NULL
          WHERE id = $1",
    )
    .bind(job_id)
    .bind(&key)
    .bind(size)
    .bind(expires)
    .execute(&state.pool)
    .await?;

    tracing::info!(%job_id, %user_id, size, file_count, "export job done");

    // Email the download link. Best-effort; it never fails the job.
    if let Err(e) = notify_by_email(state, user_id, &key, expires).await {
        tracing::warn!(%job_id, error = %e, "export email failed");
    }
    Ok(())
}

/// Build the ZIP on a temp file on disk. Every blob is streamed from R2 through
/// a fixed buffer straight into the archive, so peak heap is that buffer and not
/// a whole file: the bucket holds objects past 800 MB against a 512 MB machine,
/// and a compressed one would need its decoded copy resident at the same time.
/// Returns the temp file (kept alive for upload), the archive size in bytes, and
/// the number of entries written.
async fn build_export_zip(
    state: &CloudState,
    user_id: Uuid,
) -> anyhow::Result<(NamedTempFile, i64, i64)> {
    let saves: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT id, game_slug, label FROM saves
           WHERE user_id = $1 ORDER BY game_slug, label",
    )
    .bind(user_id)
    .fetch_all(&state.pool)
    .await?;

    let tmp = NamedTempFile::new()?;
    // Independent fd on the same inode: the ZipWriter writes through this, the
    // upload later re-opens `tmp.path()`. `tmp` itself keeps the file alive.
    let handle = tmp.as_file().try_clone()?;
    let mut zip = zip::ZipWriter::new(std::io::BufWriter::new(handle));
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .large_file(true);

    let mut file_count: i64 = 0;

    // A short README so an empty export (or a partial one after blob-fetch
    // failures) is never a confusing zero-byte download.
    zip.start_file("README.txt", opts)?;
    zip.write_all(EXPORT_README.as_bytes())?;

    for (save_id, game_slug, label) in &saves {
        let dir = entry_dir(game_slug, label);

        let ver: Option<(i64, bool, String)> = sqlx::query_as(
            "SELECT version_num, content_addressed, r2_key FROM save_versions
               WHERE save_id = $1 AND deleted_at IS NULL
               ORDER BY version_num DESC LIMIT 1",
        )
        .bind(save_id)
        .fetch_optional(&state.pool)
        .await?;
        let Some((version_num, content_addressed, legacy_key)) = ver else {
            continue; // no live version — nothing to export for this save
        };

        if content_addressed {
            let files: Vec<(String, String)> = sqlx::query_as(
                "SELECT relative_path, encode(sha256, 'hex') FROM manifest_files
                   WHERE save_id = $1 AND version_num = $2",
            )
            .bind(save_id)
            .bind(version_num)
            .fetch_all(&state.pool)
            .await?;

            for (rel, sha) in files {
                let key = r2::key_for_blob(user_id, &sha);
                // Opening the stream is what fails for a missing object, and it
                // fails before `start_file`, so a skip still leaves no entry
                // behind. A read that dies mid-blob cannot be taken back once
                // the entry is open, so that one fails the job instead.
                let reader = match state.r2.get_reader(&key).await {
                    Ok(r) => r,
                    Err(e) => {
                        tracing::warn!(%save_id, rel, error = %e, "export skipped missing blob");
                        continue;
                    }
                };
                // The export must contain the RAW file: undo the at-rest
                // zstd rewrite when (and only when) it completed, the same
                // rule as the blob proxy (`stored_bytes` set).
                let compressed = blob_is_compressed(state, user_id, &sha).await;
                let name = format!("{dir}/{}", sanitize_rel(&rel));
                zip.start_file(name, opts)?;
                stream_into_zip(reader, compressed, &mut zip).await?;
                file_count += 1;
            }
        } else if !legacy_key.is_empty() {
            // These ship as-is, still zstd: the name says `.tar.zst` and the
            // client unpacks them. Nothing to decode on the way out.
            match state.r2.get_reader(&legacy_key).await {
                Ok(reader) => {
                    let name = format!("{dir}_v{version_num}.tar.zst");
                    zip.start_file(name, opts)?;
                    stream_into_zip(reader, false, &mut zip).await?;
                    file_count += 1;
                }
                Err(e) => {
                    tracing::warn!(%save_id, error = %e, "export skipped missing legacy archive");
                }
            }
        }
    }

    let mut writer = zip.finish()?;
    writer.flush()?;
    drop(writer); // release the cloned fd before we stat the path

    let size = tmp.as_file().metadata()?.len() as i64;
    Ok((tmp, size, file_count))
}

/// Presign a download link and email it to the account holder.
async fn notify_by_email(
    state: &CloudState,
    user_id: Uuid,
    key: &str,
    expires: OffsetDateTime,
) -> anyhow::Result<()> {
    let Some(cfg) = state.config.cloud.as_ref().map(|c| &c.email) else {
        return Ok(());
    };
    if !email::is_configured(cfg) {
        return Ok(());
    }
    let to: Option<String> = sqlx::query_scalar("SELECT email FROM profiles WHERE user_id = $1")
        .bind(user_id)
        .fetch_optional(&state.pool)
        .await?;
    let Some(to) = to.filter(|e| !e.is_empty()) else {
        return Ok(());
    };
    let link = state.r2.presign_get(key, Some(LINK_TTL)).await?;
    email::send_export_ready(cfg, &to, &link.url, expires).await?;
    Ok(())
}

async fn mark_failed(state: &CloudState, job_id: Uuid, error: &str) {
    // Truncate defensively: `error` may carry a long provider body.
    let error: String = error.chars().take(500).collect();
    if let Err(e) = sqlx::query(
        "UPDATE export_jobs SET status = 'failed', finished_at = now(), error = $2 WHERE id = $1",
    )
    .bind(job_id)
    .bind(error)
    .execute(&state.pool)
    .await
    {
        tracing::error!(%job_id, error = %e, "could not mark export job failed");
    }
}

/// Expire finished exports whose TTL elapsed: delete the R2 object and flip the
/// row to `expired` so the account page stops offering a dead link.
async fn expire_due(state: &CloudState) -> Result<(), sqlx::Error> {
    let rows: Vec<(Uuid, Option<String>)> = sqlx::query_as(
        "SELECT id, r2_key FROM export_jobs
           WHERE status = 'done' AND expires_at IS NOT NULL AND expires_at < now()",
    )
    .fetch_all(&state.pool)
    .await?;
    for (id, r2_key) in rows {
        if let Some(key) = r2_key.filter(|k| !k.is_empty()) {
            if let Err(e) = state.r2.delete_object(&key).await {
                tracing::warn!(%id, error = %e, "export expiry: R2 delete failed");
            }
        }
        sqlx::query("UPDATE export_jobs SET status = 'expired', r2_key = NULL WHERE id = $1")
            .bind(id)
            .execute(&state.pool)
            .await?;
    }
    Ok(())
}

/// Directory prefix for a save's entries. `default` labels collapse to just the
/// game slug so the common single-slot case doesn't nest a redundant folder.
/// Whether the at-rest compression rewrite completed for this blob. The
/// only state where the stored object is zstd instead of raw (same rule as
/// `routes::blob_proxy`). Errors and missing rows read as "raw": worst case
/// the ZIP carries the object as-is instead of failing the whole export.
async fn blob_is_compressed(state: &CloudState, user_id: Uuid, sha: &str) -> bool {
    sqlx::query_as::<_, (Option<String>, Option<i64>)>(
        "SELECT encoding, stored_bytes FROM cloud_blobs
            WHERE user_id = $1 AND sha256 = decode($2, 'hex')",
    )
    .bind(user_id)
    .bind(sha)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten()
    .map(|(enc, stored)| enc.as_deref() == Some("zstd") && stored.is_some())
    .unwrap_or(false)
}

/// Copy one R2 object into the open ZIP entry, decoding the at-rest zstd frame
/// on the way when `compressed`. The buffer is the whole memory cost.
///
/// `ZipWriter` is a blocking writer over the temp file, which is what lets a
/// plain `write_all` sit in this loop: it is a local disk write, not a network
/// one, and the awaits either side of it are where the time actually goes.
async fn stream_into_zip<W: std::io::Write + std::io::Seek>(
    reader: impl tokio::io::AsyncBufRead + Unpin + Send,
    compressed: bool,
    zip: &mut zip::ZipWriter<W>,
) -> anyhow::Result<()> {
    use tokio::io::AsyncReadExt;
    // `Send` is not decoration: this runs inside the worker's `tokio::spawn`,
    // and boxing drops the auto trait the concrete readers carry.
    let mut src: Box<dyn tokio::io::AsyncRead + Unpin + Send> = if compressed {
        Box::new(async_compression::tokio::bufread::ZstdDecoder::new(reader))
    } else {
        Box::new(reader)
    };
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = src.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        zip.write_all(&buf[..n])?;
    }
    Ok(())
}

fn entry_dir(game_slug: &str, label: &str) -> String {
    let g = sanitize_component(game_slug);
    if label == "default" || label.is_empty() {
        g
    } else {
        format!("{g}/{}", sanitize_component(label))
    }
}

/// Normalise a client-supplied relative path into safe ZIP entry components:
/// forward-slash separated, no `.`/`..`/empty segments, no absolute anchor.
fn sanitize_rel(rel: &str) -> String {
    let parts: Vec<String> = rel
        .split(['/', '\\'])
        .filter(|s| !s.is_empty() && *s != "." && *s != "..")
        .map(sanitize_component)
        .collect();
    if parts.is_empty() {
        "file".to_string()
    } else {
        parts.join("/")
    }
}

/// Keep a single path component to sane characters. Slashes are already handled
/// by the callers; here we just neutralise control chars and the anchors.
fn sanitize_component(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| {
            if c.is_control() || c == '/' || c == '\\' {
                '_'
            } else {
                c
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches(['.', ' ']);
    if trimmed.is_empty() {
        "_".to_string()
    } else {
        trimmed.to_string()
    }
}

const EXPORT_README: &str = "\
This archive is a full export of your Hoard Cloud saves.

Each game is a folder holding the latest saved version of that game's files at
their original relative paths. Older versions and trashed saves are not
included. Files named `*.tar.zst` are legacy archives from before the
content-addressed storage format — decompress with `zstd -d` then `tar -xf`.

Generated by Hoard Cloud.
";
