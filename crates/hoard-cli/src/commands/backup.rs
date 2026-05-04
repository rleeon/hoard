//! CLI wrapper around `hoard_agent::backup::upload_directory`.
//!
//! All the heavy lifting (walking the source dir, building the multipart body,
//! upload, state.json bookkeeping) lives in the agent crate. Here we only
//! handle clap arguments, the indicatif progress bar, and stdout messages.

use anyhow::{anyhow, Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;

use hoard_agent::api::ApiClient;
use hoard_agent::backup::{remember_save, upload_directory};
use hoard_agent::config::CliConfig;
use hoard_agent::state::CliState;

pub async fn run(save_id: String, source: Option<PathBuf>, remember: bool) -> Result<()> {
    let (cfg, _) = CliConfig::load_default()?;
    let token = cfg.require_token()?;
    let client = ApiClient::new(cfg.server.url.clone(), token)?;

    let (mut state, state_path) = CliState::load_default()?;
    let source = match source {
        Some(p) => p,
        None => state
            .saves
            .get(&save_id)
            .map(|s| s.local_path.clone())
            .ok_or_else(|| {
                anyhow!("no remembered local path for save {save_id}; pass --from <PATH>")
            })?,
    };

    let pb = ProgressBar::new(0);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner} {wide_bar} {bytes}/{total_bytes} ({bytes_per_sec}, eta {eta})",
        )
        .unwrap()
        .progress_chars("=> "),
    );
    let pb_clone = pb.clone();
    let on_progress = move |uploaded: u64, total: u64| {
        // First call sets the length; subsequent calls update position.
        if pb_clone.length().unwrap_or(0) != total {
            pb_clone.set_length(total);
        }
        pb_clone.set_position(uploaded);
    };

    println!("uploading from {}", source.display());
    let outcome = upload_directory(&client, &save_id, &source, on_progress)
        .await
        .context("upload failed")?;
    pb.finish_with_message("uploaded");

    println!(
        "snapshot v{} created ({} files, {})",
        outcome.snapshot.version_num,
        outcome.snapshot.file_count,
        fmt_bytes(outcome.snapshot.total_size_bytes as u64)
    );

    // Resolve the canonical path the agent used so the state file is consistent.
    let canonical = source.canonicalize().unwrap_or(source);
    remember_save(
        &client,
        &mut state,
        &save_id,
        &canonical,
        outcome.snapshot.version_num,
        remember,
    )
    .await?;
    state.save(&state_path)?;

    Ok(())
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
