//! CLI wrapper around `hoard_agent::backup::upload_directory`.
//!
//! All the heavy lifting (walking the source dir, building the multipart body,
//! upload, state.json bookkeeping) lives in the agent crate. Here we only
//! handle clap arguments, the indicatif progress bar, and stdout messages.

use anyhow::{anyhow, Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;

use hoard_agent::api::ApiClient;
use hoard_agent::backup::{remember_save, upload_directory_checked, BackupResult};
use hoard_agent::config::CliConfig;
use hoard_agent::state::CliState;
use hoard_core::wire::VersionOrigin;

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
    let prev_sig = state.saves.get(&save_id).and_then(|s| s.set_hash.clone());
    // Our last-synced version for this save is the server's fast-forward base.
    let base_version = state.saves.get(&save_id).and_then(|s| s.last_version_num);
    let game_slug = state
        .saves
        .get(&save_id)
        .map(|s| s.game_slug.clone())
        .unwrap_or_default();
    let label = state
        .saves
        .get(&save_id)
        .map(|s| s.label.clone())
        .unwrap_or_default();
    let result = upload_directory_checked(
        &client,
        &save_id,
        &game_slug,
        &label,
        &source,
        prev_sig.as_deref(),
        base_version,
        // With no head to compare against: the D.8.3 anti-relaunch check exists
        // for the engine, which restarts on its own and already brings the
        // observed manifest. A `hoard backup` is an explicit one-off order from
        // the user, and fetching the manifest just to guess whether it could be
        // skipped would be an extra request on a command that already knows what
        // it wants.
        None,
        // `hoard backup` is typed by a person: it is a deliberate copy, and
        // retention should treat it as one.
        VersionOrigin::Manual,
        on_progress,
        || {},
    )
    .await
    .context("upload failed")?;

    let (outcome, signature) = match result {
        BackupResult::Skipped => {
            pb.finish_and_clear();
            println!("no changes since last backup — skipped");
            return Ok(());
        }
        BackupResult::Unchanged { signature } => {
            pb.finish_and_clear();
            println!("no changes since last backup — skipped");
            // Persist the refreshed composite signature so the next run hits
            // the cheap fast path instead of re-reading every file.
            if let Some(s) = state.saves.get_mut(&save_id) {
                s.set_hash = Some(signature);
            }
            state.save(&state_path)?;
            return Ok(());
        }
        BackupResult::Uploaded { outcome, signature } => (outcome, signature),
        // Unreachable with `head: None` above, but the compiler does not know
        // that, and the day it is reachable, saying so is the right answer.
        BackupResult::AlreadyLanded {
            version_num,
            signature,
        } => {
            pb.finish_and_clear();
            println!("already on the server as v{version_num} — nothing to upload");
            if let Some(s) = state.saves.get_mut(&save_id) {
                s.set_hash = Some(signature);
                s.last_version_num = Some(version_num);
            }
            state.save(&state_path)?;
            return Ok(());
        }
    };
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
    // Cache the fresh signature so an unchanged re-run is skipped next time.
    if let Some(s) = state.saves.get_mut(&save_id) {
        s.set_hash = Some(signature);
    }
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
