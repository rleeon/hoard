//! CLI wrapper around `hoard_agent::restore::download_snapshot`.
//!
//! The streaming download / decode / SHA-verify / extract logic lives in the
//! agent crate. This file is the clap front-end and the indicatif progress
//! bar.

use anyhow::{anyhow, Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use hoard_agent::api::ApiClient;
use hoard_agent::config::CliConfig;
use hoard_agent::restore::{download_snapshot, resolve_version, RestoreOptions};
use hoard_agent::state::CliState;

pub async fn apply(
    save_id: String,
    version: Option<i64>,
    to: Option<PathBuf>,
    no_verify: bool,
    force: bool,
) -> Result<()> {
    let (cfg, _) = CliConfig::load_default()?;
    let token = cfg.require_token()?;
    let client = ApiClient::new(cfg.server.url.clone(), token)?;

    let version = resolve_version(&client, &save_id, version).await?;

    let dest = match to {
        Some(p) => p,
        None => {
            let (state, _) = CliState::load_default()?;
            state
                .saves
                .get(&save_id)
                .map(|s| s.local_path.clone())
                .ok_or_else(|| {
                    anyhow!("no remembered local path for save {save_id}; pass --to <PATH>")
                })?
        }
    };

    println!(
        "restoring v{} of {} to {}",
        version,
        save_id,
        dest.display()
    );

    let pb = Arc::new(Mutex::new(ProgressBar::new_spinner()));
    {
        let bar = pb.lock().unwrap();
        bar.set_style(
            ProgressStyle::with_template("{spinner} {wide_bar} {bytes} ({bytes_per_sec})")
                .unwrap()
                .progress_chars("=> "),
        );
    }
    let pb_for_cb = pb.clone();
    let on_progress = move |downloaded: u64, total: u64| {
        let bar = pb_for_cb.lock().unwrap();
        if total > 0 && bar.length().unwrap_or(0) != total {
            bar.set_length(total);
        }
        bar.set_position(downloaded);
    };

    let options = RestoreOptions {
        skip_verify: no_verify,
        force,
    };
    let outcome = download_snapshot(&client, &save_id, version, &dest, options, on_progress)
        .await
        .context("restore failed")?;

    {
        let bar = pb.lock().unwrap();
        bar.finish_with_message("done");
    }

    println!(
        "restored {} files ({}) to {}",
        outcome.files_extracted,
        fmt_bytes(outcome.bytes_extracted),
        outcome.destination.display()
    );
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
