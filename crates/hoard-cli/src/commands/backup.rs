use anyhow::{anyhow, bail, Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::multipart;
use std::path::{Path, PathBuf};
use time::OffsetDateTime;

use crate::api::ApiClient;
use crate::config::CliConfig;
use crate::state::{CliState, SaveState};

pub async fn run(save_id: String, source: Option<PathBuf>, remember: bool) -> Result<()> {
    let (cfg, _) = CliConfig::load_default()?;
    let token = cfg.require_token()?;
    let client = ApiClient::new(cfg.server.url.clone(), token)?;

    // Resolve source from state if not provided.
    let (state, state_path) = CliState::load_default()?;
    let source = match source {
        Some(p) => p,
        None => state
            .saves
            .get(&save_id)
            .map(|s| s.local_path.clone())
            .ok_or_else(|| anyhow!(
                "no remembered local path for save {save_id}; pass --from <PATH>"
            ))?,
    };

    let source = source.canonicalize().with_context(|| {
        format!("source path does not exist: {}", source.display())
    })?;
    if !source.is_dir() {
        bail!("source must be a directory: {}", source.display());
    }

    // Walk the directory and collect (relative_path, absolute_path, size)
    let files = walk(&source)?;
    if files.is_empty() {
        bail!("no files found in {}", source.display());
    }
    let total_bytes: u64 = files.iter().map(|(_, _, sz)| *sz).sum();
    println!(
        "uploading {} files ({}) from {}",
        files.len(),
        fmt_bytes(total_bytes),
        source.display()
    );

    // Build multipart form. We read each file fully — keeps things simple and
    // robust across reqwest's stream-body trait issues. For very large saves
    // we could switch to streamed file bodies, but our typical save is small.
    let mut form = multipart::Form::new();
    let pb = ProgressBar::new(total_bytes);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner} {wide_bar} {bytes}/{total_bytes} ({bytes_per_sec}, eta {eta})",
        )
        .unwrap()
        .progress_chars("=> "),
    );

    for (rel, abs, _sz) in &files {
        let bytes = tokio::fs::read(abs).await
            .with_context(|| format!("reading {}", abs.display()))?;
        let len = bytes.len() as u64;
        let part = multipart::Part::bytes(bytes)
            .file_name(rel.clone())
            .mime_str("application/octet-stream")?;
        // Server keys files by the field NAME = "files" and reads the
        // relative path from the multipart filename header.
        form = form.part("files", part);
        pb.inc(len);
    }
    pb.finish_with_message("uploaded");

    let snap = client.snapshot_upload(&save_id, form).await
        .context("uploading snapshot")?;
    println!(
        "snapshot v{} created ({} files, {})",
        snap.version_num,
        snap.file_count,
        fmt_bytes(snap.total_size_bytes as u64)
    );

    if remember {
        let mut state = state;
        let save = client.get_save(&save_id).await?;
        state.saves.insert(
            save_id.clone(),
            SaveState {
                local_path: source,
                game_slug: save.game_slug,
                label: save.label,
                last_backup_at: Some(OffsetDateTime::now_utc()),
                last_version_num: Some(snap.version_num),
            },
        );
        state.save(&state_path)?;
    } else if let Some(existing) = state.saves.get(&save_id).cloned() {
        // Update timestamps if we already remembered this save.
        let mut state = state;
        state.saves.insert(
            save_id,
            SaveState {
                last_backup_at: Some(OffsetDateTime::now_utc()),
                last_version_num: Some(snap.version_num),
                ..existing
            },
        );
        state.save(&state_path)?;
    }

    Ok(())
}

fn walk(root: &Path) -> Result<Vec<(String, PathBuf, u64)>> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)
            .with_context(|| format!("reading dir {}", dir.display()))?
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
                out.push((rel, path, size));
            }
            // symlinks: ignored on purpose.
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

fn fmt_bytes(b: u64) -> String {
    let b = b as f64;
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    if b >= GB {
        format!("{:.2} GiB", b / GB)
    } else if b >= MB {
        format!("{:.2} MiB", b / MB)
    } else if b >= KB {
        format!("{:.2} KiB", b / KB)
    } else {
        format!("{} B", b as u64)
    }
}
