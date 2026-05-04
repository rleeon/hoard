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
use std::path::{Path, PathBuf};
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
}

/// Result of a successful upload.
#[derive(Debug, Clone)]
pub struct UploadOutcome {
    pub snapshot: Snapshot,
    pub file_count: usize,
    pub total_bytes: u64,
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
                let size = entry.metadata()?.len();
                out.push(UploadFile {
                    relative_path: rel,
                    absolute_path: path,
                    size_bytes: size,
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

    let mut form = multipart::Form::new();
    let mut uploaded = 0u64;
    progress(0, total_bytes);
    for f in &files {
        let bytes = tokio::fs::read(&f.absolute_path)
            .await
            .with_context(|| format!("reading {}", f.absolute_path.display()))?;
        let len = bytes.len() as u64;
        let part = multipart::Part::bytes(bytes)
            .file_name(f.relative_path.clone())
            .mime_str("application/octet-stream")?;
        // Server keys files by the field NAME = "files" and reads the
        // relative path from the multipart filename header.
        form = form.part("files", part);
        uploaded += len;
        progress(uploaded, total_bytes);
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
        state.saves.insert(
            save_id.to_string(),
            SaveState {
                local_path: local_path.to_path_buf(),
                game_slug: save.game_slug,
                label: save.label,
                last_backup_at: Some(OffsetDateTime::now_utc()),
                last_version_num: Some(last_version_num),
                paused: was_paused,
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
