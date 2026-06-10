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
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use time::OffsetDateTime;
use tokio::io::AsyncReadExt;

use crate::api::{ApiClient, CloudCasFileEntry, CloudCasInit, Snapshot};
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
    /// The cheap set signature matched the cached one — nothing was read or
    /// uploaded. (Fast path.)
    Skipped,
    /// The cheap signature drifted (the game rewrote its save files, bumping
    /// mtimes) but the actual bytes are identical to the last upload, so no
    /// new snapshot was created. `signature` is the refreshed composite the
    /// caller should persist so the *next* check hits the fast path again
    /// instead of re-hashing the whole save every cycle.
    Unchanged { signature: String },
    /// A new snapshot was created. `signature` is the freshly-computed
    /// composite signature the caller should persist for the next skip check.
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

/// Content signature over the sorted `(relative_path, bytes)` set.
///
/// Unlike [`compute_set_signature`] this *reads every file*, so it's only used
/// as a fallback when the cheap signature drifted: many games (and some
/// background launchers / cloud-sync daemons) rewrite save files on a timer,
/// bumping the mtime without changing a single byte. The cheap check would
/// treat that as a change and cut a redundant snapshot every few hours; this
/// confirms whether the bytes actually moved before we upload.
async fn compute_content_signature(files: &[UploadFile]) -> Result<String> {
    use tokio::io::AsyncReadExt;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 128 * 1024];
    for f in files {
        h.update(f.relative_path.as_bytes());
        h.update([0u8]);
        let mut file = tokio::fs::File::open(&f.absolute_path)
            .await
            .with_context(|| format!("hashing {}", f.absolute_path.display()))?;
        loop {
            let n = file
                .read(&mut buf)
                .await
                .with_context(|| format!("reading {}", f.absolute_path.display()))?;
            if n == 0 {
                break;
            }
            h.update(&buf[..n]);
        }
        h.update([0u8]);
    }
    Ok(hex::encode(h.finalize()))
}

/// Persisted skip signature is a composite `"<cheap>:<content>"`. We split it
/// back into its two halves; a legacy value with no `:` (pre-fallback state
/// files held only the cheap hash) is treated as cheap-only with no known
/// content hash, so the first drift after upgrading reads bytes once and then
/// stores the composite.
fn split_signature(sig: Option<&str>) -> (Option<&str>, Option<&str>) {
    match sig {
        None => (None, None),
        Some(s) => match s.split_once(':') {
            Some((cheap, content)) => (Some(cheap), Some(content)),
            None => (Some(s), None),
        },
    }
}

fn join_signature(cheap: &str, content: &str) -> String {
    format!("{cheap}:{content}")
}

/// Walk `root` recursively and return all regular files, sorted by relative path.
///
/// Symlinks are skipped on purpose: we don't want to follow links out of the
/// save directory, and tar archives with symlinks make restore ambiguous.
/// Transient lock files a running game keeps exclusively open. They carry no
/// save data and on Windows can't even be opened for reading while the game
/// holds them (sharing violation, os error 32), which used to abort the whole
/// backup mid-walk — e.g. Minecraft's `.minecraft/saves/<world>/session.lock`.
/// Matched case-insensitively on the file name.
const TRANSIENT_LOCK_FILES: &[&str] = &["session.lock"];

fn is_transient_lock(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    TRANSIENT_LOCK_FILES.iter().any(|n| lower == *n)
}

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
                // Skip lock files the game holds open: they're never save data
                // and trying to read them aborts the backup while the game runs.
                if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                    if is_transient_lock(name) {
                        continue;
                    }
                }
                let rel = path
                    .strip_prefix(root)
                    .map_err(|e| anyhow!("strip_prefix: {e}"))?
                    .to_string_lossy()
                    .replace('\\', "/");
                // A file we can't stat (locked, vanished mid-walk, permission)
                // is skipped with a warning rather than failing the whole
                // upload — one unreadable transient file shouldn't lose the
                // backup of everything else.
                let meta = match entry.metadata() {
                    Ok(m) => m,
                    Err(e) => {
                        tracing::warn!(path = %path.display(), error = %e, "skipping unreadable file");
                        continue;
                    }
                };
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
///
/// `game_slug` and `label` are only consulted on the Hoard Cloud path, where
/// the server keys the save row on `(user_id, game_slug, label)` and the
/// snapshot list endpoints don't exist. They're ignored self-hosted.
pub async fn upload_directory<F>(
    client: &ApiClient,
    save_id: &str,
    game_slug: &str,
    label: &str,
    source: &Path,
    base_version: Option<i64>,
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

    // Hoard Cloud (api.hoard.services) speaks a different protocol: the
    // self-hosted `/v1/saves/:id/snapshots` multipart endpoint doesn't exist
    // there. Pack the save into a single tar.zst, declare the upload, PUT the
    // bytes straight to R2 via a presigned URL, then commit.
    if client.is_cloud().await {
        return upload_directory_cloud(
            client,
            save_id,
            game_slug,
            label,
            &files,
            total_bytes,
            base_version,
            progress,
        )
        .await;
    }

    // Ingesta adaptativa por forma del save (ADR 0019): muchos archivos
    // pequeños viajan mejor como un único tar (un round-trip, un handle) que
    // como N partes multipart. El umbral es por número de archivos; el server
    // desempaqueta el campo `pack` y deduplica por-archivo igual que el modo
    // normal, así que el modelo de almacenamiento no cambia.
    const PACK_THRESHOLD: usize = 500;

    let mut form = multipart::Form::new();
    // Declare the base version so the server can reject a non-fast-forward
    // (another device advanced this save since we last synced).
    if let Some(b) = base_version {
        form = form.text("base_version", b.to_string());
    }
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

/// Hoard Cloud upload (content-addressed): hash each file → declare manifest
/// → upload only the blobs the server is missing → commit.
///
/// Unlike the old archive path this never packs the whole save: each file is
/// its own R2 object keyed by its whole-file SHA-256, so a 600 MB save the
/// game rewrote in place with 10 MB of real change costs a 10 MB upload. Files
/// are never decompressed — the game's `.v3`/zip blobs are deduped whole.
#[allow(clippy::too_many_arguments)]
async fn upload_directory_cloud<F>(
    client: &ApiClient,
    save_id: &str,
    game_slug: &str,
    label: &str,
    files: &[UploadFile],
    total_bytes: u64,
    base_version: Option<i64>,
    progress: F,
) -> Result<UploadOutcome>
where
    F: Fn(u64, u64),
{
    let file_count = files.len();
    progress(0, total_bytes);

    // 1. Whole-file SHA-256 of every file — the dedup key.
    let mut shas: Vec<String> = Vec::with_capacity(file_count);
    for f in files {
        shas.push(hash_file(&f.absolute_path).await?);
    }

    // 2. Declare the manifest; the server replies with the blobs it lacks.
    let manifest: Vec<CloudCasFileEntry> = files
        .iter()
        .zip(&shas)
        .map(|(f, sha)| CloudCasFileEntry {
            relative_path: f.relative_path.clone(),
            sha256: sha.clone(),
            size_bytes: f.size_bytes as i64,
            modified_at: f
                .modified
                .and_then(|m| m.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64),
        })
        .collect();

    let init = client
        .cloud_cas_init(&CloudCasInit {
            save_id: save_id.to_string(),
            game_slug: game_slug.to_string(),
            label: Some(label.to_string()),
            device_name: None,
            notes: None,
            backup_only: false,
            base_version,
            files: manifest,
        })
        .await
        .context("cloud cas init")?;

    // 3. Upload only the missing blobs. Files sharing a SHA upload once.
    let mut by_sha: HashMap<&str, &UploadFile> = HashMap::new();
    for (f, sha) in files.iter().zip(&shas) {
        by_sha.entry(sha.as_str()).or_insert(f);
    }
    let upload_total: u64 = init
        .missing
        .iter()
        .map(|b| b.size_bytes.max(0) as u64)
        .sum();
    // Progress is reported against the bytes actually transferred, so the bar
    // reflects the real (deduped) upload rather than the whole save size.
    let denom = upload_total.max(1);
    let mut uploaded = 0u64;
    progress(0, denom);
    for blob in &init.missing {
        let Some(f) = by_sha.get(blob.sha256.as_str()) else {
            bail!(
                "server requested a blob not in the manifest: {}",
                blob.sha256
            );
        };
        let body = file_to_body(&f.absolute_path).await?;
        client
            .put_presigned(&blob.upload, body, f.size_bytes)
            .await
            .with_context(|| format!("uploading {}", f.relative_path))?;
        uploaded += f.size_bytes;
        progress(uploaded, denom);
    }

    // 4. Commit — the server verifies the new blobs landed and finalizes.
    // The commit must target the *canonical* cloud save id: when another
    // device already created this (game, label) under a different id, the
    // server resolved ours to that one at init, and committing against our
    // local id would 404 forever.
    let canonical_id = init.save_id.as_deref().unwrap_or(save_id);
    if canonical_id != save_id {
        tracing::info!(
            local_save_id = save_id,
            canonical_save_id = canonical_id,
            game_slug,
            label,
            "cloud save id diverged — committing against the canonical cloud id"
        );
    }
    let commit = client
        .cloud_cas_commit(canonical_id, init.version_num)
        .await
        .context("cloud cas commit")?;

    // Synthesize a Snapshot for the shared `UploadOutcome` shape.
    // `total_size_bytes` is the logical save size (sum of file sizes), matching
    // self-hosted snapshot semantics.
    let snapshot = Snapshot {
        id: String::new(),
        save_id: Some(commit.save_id),
        version_num: commit.version_num,
        parent_version: base_version,
        file_count: file_count as i64,
        total_size_bytes: total_bytes as i64,
        is_pinned: false,
        created_at: OffsetDateTime::now_utc(),
        deleted_at: None,
    };
    Ok(UploadOutcome {
        snapshot,
        file_count,
        total_bytes,
    })
}

/// SHA-256 of a file's bytes, read in fixed-size chunks.
async fn hash_file(path: &Path) -> Result<String> {
    let mut file = tokio::fs::File::open(path)
        .await
        .with_context(|| format!("hashing {}", path.display()))?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 256 * 1024];
    loop {
        let n = file
            .read(&mut buf)
            .await
            .with_context(|| format!("reading {}", path.display()))?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(hex::encode(h.finalize()))
}

/// Wrap a file as a streaming reqwest body for the presigned PUT.
async fn file_to_body(path: &Path) -> Result<reqwest::Body> {
    let file = tokio::fs::File::open(path)
        .await
        .with_context(|| format!("opening {}", path.display()))?;
    let stream = tokio_util::io::ReaderStream::new(file);
    Ok(reqwest::Body::wrap_stream(stream))
}

/// Skip-aware wrapper around [`upload_directory`] (ADR 0019).
///
/// Two-tier check against the persisted composite `prev_signature`:
/// 1. Cheap `(path, size, mtime)` signature matches → [`BackupResult::Skipped`],
///    no file read, no network.
/// 2. Cheap drifted (usually just an mtime bump from a game/daemon rewriting
///    saves on a timer) → read bytes once; if the content hash matches the
///    stored one → [`BackupResult::Unchanged`] carrying the refreshed composite
///    so the next cycle hits the fast path again instead of re-hashing.
/// 3. Bytes actually moved → upload and return [`BackupResult::Uploaded`].
///
/// The signature persisted by the caller is `"<cheap>:<content>"`.
#[allow(clippy::too_many_arguments)]
pub async fn upload_directory_checked<F, G>(
    client: &ApiClient,
    save_id: &str,
    game_slug: &str,
    label: &str,
    source: &Path,
    prev_signature: Option<&str>,
    base_version: Option<i64>,
    progress: F,
    on_upload_start: G,
) -> Result<BackupResult>
where
    F: Fn(u64, u64),
    G: FnOnce(),
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
    let (prev_cheap, prev_content) = split_signature(prev_signature);
    let cheap = compute_set_signature(&files);
    if prev_cheap == Some(cheap.as_str()) {
        // Fast path: the cheap (path, size, mtime) signature is unchanged, so
        // the bytes can't have moved either — skip without reading any file.
        return Ok(BackupResult::Skipped);
    }
    // The cheap signature drifted. That's often just an mtime bump (a game or
    // background daemon rewriting save files on a timer), so confirm whether
    // the actual bytes changed before cutting a snapshot.
    let content = compute_content_signature(&files).await?;
    if prev_content == Some(content.as_str()) {
        return Ok(BackupResult::Unchanged {
            signature: join_signature(&cheap, &content),
        });
    }
    // The bytes genuinely moved: we're about to push a real snapshot. Signal
    // it now (after every skip/unchanged check) so callers only surface a
    // "uploading…" notice when something actually uploads.
    on_upload_start();
    let outcome = upload_directory(
        client,
        save_id,
        game_slug,
        label,
        &canonical,
        base_version,
        progress,
    )
    .await?;
    Ok(BackupResult::Uploaded {
        outcome,
        signature: join_signature(&cheap, &content),
    })
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
        let prev_preset = state.saves.get(save_id).and_then(|s| s.preset.clone());
        state.saves.insert(
            save_id.to_string(),
            SaveState {
                local_path: local_path.to_path_buf(),
                game_slug: save.game_slug,
                label: save.label,
                last_backup_at: Some(OffsetDateTime::now_utc()),
                last_version_num: Some(last_version_num),
                paused: was_paused,
                preset: prev_preset,
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
        let base = [uf("a.sav", 10, 100)];
        let base_sig = compute_set_signature(&base);
        assert_ne!(base_sig, compute_set_signature(&[uf("a.sav", 11, 100)]));
        assert_ne!(base_sig, compute_set_signature(&[uf("a.sav", 10, 101)]));
        assert_ne!(base_sig, compute_set_signature(&[uf("b.sav", 10, 100)]));
    }

    #[test]
    fn signature_distinguishes_extra_file() {
        let one = vec![uf("a.sav", 10, 100)];
        let two = vec![uf("a.sav", 10, 100), uf("b.sav", 5, 50)];
        assert_ne!(compute_set_signature(&one), compute_set_signature(&two));
    }

    #[test]
    fn split_join_round_trip() {
        assert_eq!(split_signature(None), (None, None));
        // Legacy cheap-only state (pre-fallback): no content half.
        assert_eq!(split_signature(Some("abc")), (Some("abc"), None));
        let composite = join_signature("cheap", "content");
        assert_eq!(composite, "cheap:content");
        assert_eq!(
            split_signature(Some(&composite)),
            (Some("cheap"), Some("content"))
        );
    }

    #[tokio::test]
    async fn content_signature_ignores_mtime_but_tracks_bytes() {
        let dir = std::env::temp_dir().join(format!("hoard-sig-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("save.dat");
        std::fs::write(&path, b"hello world").unwrap();
        let mk = |mtime: u64| UploadFile {
            relative_path: "save.dat".to_string(),
            absolute_path: path.clone(),
            size_bytes: 11,
            modified: Some(UNIX_EPOCH + std::time::Duration::from_secs(mtime)),
        };
        // Cheap signature drifts with mtime, content signature does not.
        let a = vec![mk(100)];
        let b = vec![mk(999)];
        assert_ne!(compute_set_signature(&a), compute_set_signature(&b));
        assert_eq!(
            compute_content_signature(&a).await.unwrap(),
            compute_content_signature(&b).await.unwrap()
        );
        // Changing the bytes does move the content signature.
        let before = compute_content_signature(&a).await.unwrap();
        std::fs::write(&path, b"hello WORLD").unwrap();
        let after = compute_content_signature(&a).await.unwrap();
        assert_ne!(before, after);
        std::fs::remove_dir_all(&dir).ok();
    }
}
