//! Library half of the snapshot upload flow.
//!
//! The CLI and the desktop app both want to walk a directory, build a
//! multipart body, and POST it to the server. The two diverge on how they
//! present progress (indicatif progress bar vs a Tauri event stream), so we
//! expose the work as an async function with a `progress` callback.
//!
//! State-file bookkeeping (`saves` map in `state.json`) lives here too so the
//! GUI gets it for free.

use anyhow::{anyhow, bail, Context, Result};
use reqwest::multipart;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use time::OffsetDateTime;

use crate::api::{ApiClient, Snapshot};
use crate::state::{CliState, SaveState};

/// One file enumerated from the source directory.
#[derive(Debug, Clone)]
pub struct UploadFile {
    /// Forward-slash relative path used as the multipart filename header.
    pub relative_path: String,
    /// Absolute path on disk.
    pub absolute_path: PathBuf,
    /// File size in bytes (read once during the walk so progress totals are correct).
    pub size_bytes: u64,
    /// Last-modified time, captured during the walk. Used only to build the
    /// cheap skip-by-set-hash signature; `None` if the platform/FS didn't
    /// report one.
    pub modified: Option<SystemTime>,
}

/// Result of a successful upload.
#[derive(Debug, Clone)]
pub struct UploadOutcome {
    pub snapshot: Snapshot,
    pub file_count: usize,
    pub total_bytes: u64,
}

/// Outcome of a skip-aware backup ([`upload_directory_checked`]).
#[derive(Debug, Clone)]
pub enum BackupResult {
    /// The save's set signature matched the cached one — nothing was uploaded.
    Skipped,
    /// A new snapshot was created. `signature` is the freshly-computed set
    /// signature the caller should persist for the next skip check.
    Uploaded {
        outcome: UploadOutcome,
        signature: String,
    },
}

/// Cheap signature over the sorted `(relative_path, size, mtime)` set.
///
/// Deliberately *not* a content hash: it never reads file bytes, so it adds
/// no IO on top of the directory walk. Two walks with identical paths, sizes
/// and mtimes produce the same signature — which is exactly the "watcher
/// settled but nothing was actually written" case we want to skip. It will
/// not catch a rewrite that preserves size *and* mtime while changing bytes
/// (rare for game saves), trading that corner for zero read overhead.
pub fn compute_set_signature(files: &[UploadFile]) -> String {
    let mut h = Sha256::new();
    for f in files {
        h.update(f.relative_path.as_bytes());
        h.update([0u8]);
        h.update(f.size_bytes.to_le_bytes());
        let mtime_nanos = f
            .modified
            .and_then(|m| m.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        h.update(mtime_nanos.to_le_bytes());
        h.update([0u8]);
    }
    hex::encode(h.finalize())
}

/// Walk `root` recursively and return all regular files, sorted by relative path.
///
/// Symlinks are skipped on purpose: we don't want to follow links out of the
/// save directory, and tar archives with symlinks make restore ambiguous.
pub fn walk_source(root: &Path) -> Result<Vec<UploadFile>> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in
            std::fs::read_dir(&dir).with_context(|| format!("reading dir {}", dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            let ft = entry.file_type()?;
            if ft.is_dir() {
                stack.push(path);
            } else if ft.is_file() {
                let rel = path
                    .strip_prefix(root)
                    .map_err(|e| anyhow!("strip_prefix: {e}"))?
                    .to_string_lossy()
                    .replace('\\', "/");
                let meta = entry.metadata()?;
                out.push(UploadFile {
                    relative_path: rel,
                    absolute_path: path,
                    size_bytes: meta.len(),
                    modified: meta.modified().ok(),
                });
            }
            // symlinks: ignored on purpose.
        }
    }
    out.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    Ok(out)
}

/// Upload a directory as a new snapshot for `save_id`.
///
/// `progress(uploaded, total)` is called once per file as it's added to the
/// multipart form. Both values are byte counts. The callback is `Fn` so the
/// caller can wire any UI on top.
pub async fn upload_directory<F>(
    client: &ApiClient,
    save_id: &str,
    source: &Path,
    progress: F,
) -> Result<UploadOutcome>
where
    F: Fn(u64, u64),
{
    let source = source
        .canonicalize()
        .with_context(|| format!("source path does not exist: {}", source.display()))?;
    if !source.is_dir() {
        bail!("source must be a directory: {}", source.display());
    }

    let files = walk_source(&source)?;
    if files.is_empty() {
        bail!("no files found in {}", source.display());
    }
    let total_bytes: u64 = files.iter().map(|f| f.size_bytes).sum();
    let file_count = files.len();

    // Ingesta adaptativa por forma del save (ADR 0019): muchos archivos
    // pequeños viajan mejor como un único tar (un round-trip, un handle) que
    // como N partes multipart. El umbral es por número de archivos; el server
    // desempaqueta el campo `pack` y deduplica por-archivo igual que el modo
    // normal, así que el modelo de almacenamiento no cambia.
    const PACK_THRESHOLD: usize = 500;

    let mut form = multipart::Form::new();
    progress(0, total_bytes);

    if file_count > PACK_THRESHOLD {
        // Build the tar on the fly through an in-memory pipe and stream it as
        // the request body — never materialising the whole archive in RAM.
        let (writer, reader) = tokio::io::duplex(256 * 1024);
        let files_for_tar = files.clone();
        tokio::spawn(async move {
            let mut tar = tokio_tar::Builder::new(writer);
            for f in &files_for_tar {
                if let Err(e) = tar
                    .append_path_with_name(&f.absolute_path, &f.relative_path)
                    .await
                {
                    // Dropping the writer truncates the tar; the server then
                    // rejects it as a malformed pack, surfacing as an upload
                    // error rather than a silent partial snapshot.
                    tracing::warn!(error = %e, path = %f.relative_path, "pack tar build error");
                    return;
                }
            }
            if let Ok(mut inner) = tar.into_inner().await {
                let _ = tokio::io::AsyncWriteExt::shutdown(&mut inner).await;
            }
        });
        let stream = tokio_util::io::ReaderStream::new(reader);
        let body = reqwest::Body::wrap_stream(stream);
        let part = multipart::Part::stream(body)
            .file_name("pack.tar")
            .mime_str("application/x-tar")?;
        form = form.part("pack", part);
        progress(total_bytes, total_bytes);
    } else {
        let mut uploaded = 0u64;
        for f in &files {
            // Stream each file from disk instead of reading it whole into RAM:
            // open the handle, wrap it as a byte stream and hand it to reqwest
            // as a streaming multipart part. A 2 GB save no longer means 2 GB
            // of process memory.
            let file = tokio::fs::File::open(&f.absolute_path)
                .await
                .with_context(|| format!("reading {}", f.absolute_path.display()))?;
            let stream = tokio_util::io::ReaderStream::new(file);
            let body = reqwest::Body::wrap_stream(stream);
            let part = multipart::Part::stream_with_length(body, f.size_bytes)
                .file_name(f.relative_path.clone())
                .mime_str("application/octet-stream")?;
            // Server keys files by the field NAME = "files" and reads the
            // relative path from the multipart filename header.
            form = form.part("files", part);
            uploaded += f.size_bytes;
            progress(uploaded, total_bytes);
        }
    }

    let snap = client
        .snapshot_upload(save_id, form)
        .await
        .context("uploading snapshot")?;

    Ok(UploadOutcome {
        snapshot: snap,
        file_count,
        total_bytes,
    })
}

/// Skip-aware wrapper around [`upload_directory`] (ADR 0019).
///
/// Computes the cheap set signature of `source`; if it equals
/// `prev_signature` the directory hasn't changed since the last upload and
/// we return [`BackupResult::Skipped`] without touching the network. Otherwise
/// it uploads and returns the fresh signature for the caller to persist.
pub async fn upload_directory_checked<F>(
    client: &ApiClient,
    save_id: &str,
    source: &Path,
    prev_signature: Option<&str>,
    progress: F,
) -> Result<BackupResult>
where
    F: Fn(u64, u64),
{
    let canonical = source
        .canonicalize()
        .with_context(|| format!("source path does not exist: {}", source.display()))?;
    if !canonical.is_dir() {
        bail!("source must be a directory: {}", canonical.display());
    }
    let files = walk_source(&canonical)?;
    if files.is_empty() {
        bail!("no files found in {}", canonical.display());
    }
    let signature = compute_set_signature(&files);
    if prev_signature == Some(signature.as_str()) {
        return Ok(BackupResult::Skipped);
    }
    let outcome = upload_directory(client, save_id, &canonical, progress).await?;
    Ok(BackupResult::Uploaded { outcome, signature })
}

/// Persist (or refresh) the `(save_id → local_path)` mapping in `state.json`.
///
/// If `remember` is true, fetch the save's metadata from the server and write
/// a fresh entry. If false but an entry already exists, just bump the
/// `last_backup_at` and `last_version_num` fields.
pub async fn remember_save(
    client: &ApiClient,
    state: &mut CliState,
    save_id: &str,
    local_path: &Path,
    last_version_num: i64,
    remember: bool,
) -> Result<()> {
    if remember {
        let save = client.get_save(save_id).await?;
        // Preserve any user-set pause flag if the entry already existed —
        // re-fetching from the server shouldn't silently un-pause it.
        let was_paused = state.saves.get(save_id).map(|s| s.paused).unwrap_or(false);
        // Preserve the skip-by-hash signature across a metadata refresh too,
        // so re-remembering a save doesn't force a redundant next upload.
        let prev_hash = state.saves.get(save_id).and_then(|s| s.set_hash.clone());
        state.saves.insert(
            save_id.to_string(),
            SaveState {
                local_path: local_path.to_path_buf(),
                game_slug: save.game_slug,
                label: save.label,
                last_backup_at: Some(OffsetDateTime::now_utc()),
                last_version_num: Some(last_version_num),
                paused: was_paused,
                set_hash: prev_hash,
            },
        );
    } else if let Some(existing) = state.saves.get(save_id).cloned() {
        state.saves.insert(
            save_id.to_string(),
            SaveState {
                last_backup_at: Some(OffsetDateTime::now_utc()),
                last_version_num: Some(last_version_num),
                ..existing
            },
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uf(rel: &str, size: u64, mtime_secs: u64) -> UploadFile {
        UploadFile {
            relative_path: rel.to_string(),
            absolute_path: PathBuf::from("/x").join(rel),
            size_bytes: size,
            modified: Some(UNIX_EPOCH + std::time::Duration::from_secs(mtime_secs)),
        }
    }

    #[test]
    fn signature_stable_for_identical_set() {
        let a = vec![uf("a.sav", 10, 100), uf("b.sav", 20, 200)];
        let b = vec![uf("a.sav", 10, 100), uf("b.sav", 20, 200)];
        assert_eq!(compute_set_signature(&a), compute_set_signature(&b));
    }

    #[test]
    fn signature_changes_on_size_mtime_or_path() {
        let base = vec![uf("a.sav", 10, 100)];
        let base_sig = compute_set_signature(&base);
        assert_ne!(base_sig, compute_set_signature(&vec![uf("a.sav", 11, 100)]));
        assert_ne!(base_sig, compute_set_signature(&vec![uf("a.sav", 10, 101)]));
        assert_ne!(base_sig, compute_set_signature(&vec![uf("b.sav", 10, 100)]));
    }

    #[test]
    fn signature_distinguishes_extra_file() {
        let one = vec![uf("a.sav", 10, 100)];
        let two = vec![uf("a.sav", 10, 100), uf("b.sav", 5, 50)];
        assert_ne!(compute_set_signature(&one), compute_set_signature(&two));
    }
}
