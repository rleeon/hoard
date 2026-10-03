//! CLI wrapper around `hoard_agent::restore::download_snapshot`.
//!
//! The streaming download / decode / SHA-verify / extract logic lives in the
//! agent crate. This file is the clap front-end and the indicatif progress
//! bar.

use anyhow::{anyhow, Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use hoard_agent::library;
use hoard_agent::restore::{download_snapshot_gated, resolve_version, RestoreOptions};
use hoard_agent::state::CliState;

use crate::commands::link;
use crate::output;

/// What restoring this version would do to the folder, file by file.
#[derive(Serialize)]
pub struct PreviewOut {
    /// Files the version brings with different bytes: these get overwritten.
    /// Listed individually, because a count is not enough when the answer
    /// decides whether someone loses a session.
    pub modified: Vec<String>,
    pub added: Vec<String>,
    /// On disk and not in the version. **Nothing deletes them**, but they are
    /// the saves made *after* the version being restored, so they are the ones
    /// worth reading before saying yes.
    pub local_only: Vec<String>,
    /// Real totals. The lists above stop at 200 entries; these never do.
    pub modified_count: usize,
    pub added_count: usize,
    pub local_only_count: usize,
    pub unchanged: usize,
    pub bytes_to_write: u64,
    /// False on versions that don't publish per-file hashes: then modified and
    /// unchanged can't be told apart and the numbers are an upper bound.
    pub comparable: bool,
}

#[derive(Serialize)]
pub struct RestoredOut {
    pub files_extracted: u64,
    pub bytes_extracted: u64,
    /// Files that were already byte-identical on disk: copied locally instead
    /// of downloaded.
    pub files_reused: u64,
    pub bytes_reused: u64,
    pub destination: String,
    /// Where the files the restore replaced were copied first. Absent when it
    /// replaced nothing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kept_in: Option<String>,
}

#[derive(Serialize)]
pub struct RestoreOut {
    pub save_id: String,
    pub version: i64,
    pub destination: String,
    pub dry_run: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<PreviewOut>,
    /// Why the preview is missing, when it is. Not being able to look is never
    /// a reason to block a restore, but it is a reason to say so out loud.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview_error: Option<String>,
    /// Absent on `--dry-run`: nothing was written.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restored: Option<RestoredOut>,
    /// Set with `--remember`: the destination is now this save's folder on this
    /// machine, and this says whether the sync service has picked that up.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remembered: Option<String>,
}

#[allow(clippy::too_many_arguments)]
pub async fn apply(
    save_id: String,
    version: Option<i64>,
    to: Option<PathBuf>,
    no_verify: bool,
    force: bool,
    dry_run: bool,
    allow_ini: bool,
    remember: bool,
) -> Result<()> {
    // Before the state: resolving the session is also what picks which
    // account's `state.json` gets loaded.
    let client = link::resolve_session().await?.client;

    let version = resolve_version(&client, &save_id, version).await?;

    let remembered_path = CliState::load_default()
        .ok()
        .and_then(|(state, _)| state.saves.get(&save_id).map(|s| s.local_path.clone()));
    let dest = match to {
        Some(p) => p,
        None => remembered_path.clone().ok_or_else(|| {
            anyhow!("no remembered local path for save {save_id}; pass --to <PATH>")
        })?,
    };

    // `--remember` makes `dest` this save's folder here, through the same door as
    // linking it from the desktop's Library. Checked now, before anything lands in
    // it, and recorded only once the restore went through. A dry run checks too,
    // and writes nothing: it is how to ask whether the folder would be accepted.
    let home = if remember {
        Some(
            library::plan_home_for_restore(&client, &save_id, &dest)
                .await
                .with_context(|| format!("can't keep {} as this save's folder", dest.display()))?,
        )
    } else {
        None
    };

    // What is allowed to be written. The manifest's patterns can only be
    // consulted when we know which game the folder belongs to; a bare `--to` over
    // a save that is not in the local state gets no shields and the kernel
    // decides on its own.
    // A save new to this machine has no row until the restore is done, so with
    // `--remember` its game comes from the plan.
    let slug = match &home {
        Some(home) => Some(home.game_slug.clone()),
        None => CliState::load_default()
            .ok()
            .and_then(|(st, _)| st.saves.get(&save_id).map(|s| s.game_slug.clone())),
    };
    let shields = slug
        .as_deref()
        .map(hoard_agent::savefilter::shields_for_slug)
        .unwrap_or_default();
    // And what of the version is the game itself, should `dest` be its install.
    let narrow = slug
        .as_deref()
        .map(hoard_agent::backup::SourceFilter::for_slug);
    let gate = hoard_core::kernel::fileclass::RestoreGate {
        shields,
        allow_device_local: allow_ini,
    };

    // What is going to happen to the folder. Nothing is downloaded: it crosses
    // the version's manifest with what is on disk. Always shown, because
    // restoring overwrites and that deserves saying beforehand; with `--dry-run`
    // it is all the command does.
    let (preview, preview_error) = match hoard_agent::preview::restore_preview(
        &client,
        &save_id,
        version,
        &dest,
        &gate,
        narrow.as_ref(),
    )
    .await
    {
        Ok(p) => (
            Some(PreviewOut {
                modified: p.modified,
                added: p.added,
                local_only: p.local_only,
                modified_count: p.modified_count,
                added_count: p.added_count,
                local_only_count: p.local_only_count,
                unchanged: p.unchanged,
                bytes_to_write: p.bytes_to_write,
                comparable: p.comparable,
            }),
            None,
        ),
        // Not being able to look at what changes is no reason to block a
        // restore.
        Err(e) => (None, Some(format!("{e:#}"))),
    };

    if dry_run {
        let out = RestoreOut {
            save_id: save_id.clone(),
            version,
            destination: dest.display().to_string(),
            dry_run: true,
            preview,
            preview_error,
            restored: None,
            remembered: None,
        };
        return output::emit(&out, |out| print_preview(out, true));
    }

    if !output::json() {
        let out = RestoreOut {
            save_id: save_id.clone(),
            version,
            destination: dest.display().to_string(),
            dry_run: false,
            preview: None,
            preview_error: None,
            restored: None,
            remembered: None,
        };
        // Always shown, because restoring overwrites and that deserves saying
        // beforehand.
        let shown = RestoreOut {
            preview: preview.as_ref().map(clone_preview),
            preview_error: preview_error.clone(),
            ..out
        };
        print_preview(&shown, false);
        println!(
            "restoring v{} of {} to {}",
            version,
            save_id,
            dest.display()
        );
    }

    let pb = Arc::new(Mutex::new(ProgressBar::new_spinner()));
    {
        let bar = pb.lock().unwrap();
        // indicatif draws on stderr, so it never reaches the JSON envelope;
        // hidden under `--json` anyway, because a spinner nobody watches is
        // just noise in a log.
        if output::json() {
            bar.set_draw_target(indicatif::ProgressDrawTarget::hidden());
        }
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
        // Extraction goes straight into `dest`, so that's also the folder worth
        // deduping against: identical bytes already there aren't downloaded again.
        reuse_from: Some(dest.clone()),
        gate,
        backup_root: None,
        staging_root: None,
        narrow: narrow.clone(),
    };
    // The service watches this folder and backs it up five seconds after it
    // goes quiet. Mid-restore that would publish a half-written folder as a
    // version, and every other device would pull it with a sha that checks out.
    // Only when the restore goes into that folder: a hold, and above all the
    // head handed over when letting go, would otherwise speak for a folder
    // this restore never touched.
    let watched_here = remembered_path.as_deref() == Some(dest.as_path());
    let mut hold = None;
    if watched_here || home.is_some() {
        // A folder `--remember` is about to adopt is not watched until the
        // reload after the restore: finding no save there is expected.
        let (taken, warning) = link::hold_for_restore(&save_id, watched_here).await;
        if let Some(warning) = warning {
            eprintln!("warning: {warning}");
        }
        hold = taken;
    }
    let interrupt = PlacementGuard::install();
    let restored = async {
        let before_place = async {
            if let Some(hold) = hold.as_mut() {
                hold.until_held().await;
            }
            interrupt.placing(true);
        };
        let outcome = download_snapshot_gated(
            &client,
            &save_id,
            version,
            &dest,
            options,
            on_progress,
            before_place,
        )
        .await
        .context("restore failed");
        interrupt.placing(false);
        let outcome = outcome?;

        {
            let bar = pb.lock().unwrap();
            bar.finish_with_message("done");
        }

        // Recorded only now that the files are in. Then the service is told,
        // because it rereads the watched set only when told.
        let remembered = match home {
            Some(home) => {
                home.commit(&client, version, narrow.as_ref())
                    .await
                    .with_context(|| {
                        format!(
                            "restored to {}, but couldn't keep it as this save's folder",
                            dest.display()
                        )
                    })?;
                Some(link::notify_reload().await.to_string())
            }
            None => None,
        };
        Ok::<_, anyhow::Error>((outcome, remembered))
    };
    // Ctrl+C outside the placement drops the restore rather than ending the
    // process on the spot, so its staging folder goes away with it and the hold
    // is let go below. Exiting at once left a whole download in the staging
    // folder until the weekly sweep.
    let restored = tokio::select! {
        r = restored => r,
        () = interrupt.cancelled() => Err(anyhow!("interrupted")),
    };
    // On every way out, so a failed restore does not leave the save held until
    // the lease runs out. After one that went through, the service is handed
    // the newest version the server holds, so the restored folder goes up as
    // the next one instead of losing a merge against it.
    if let Some(hold) = hold {
        let adopt_head = match &restored {
            Ok(_) => resolve_version(&client, &save_id, None).await.ok(),
            Err(_) => None,
        };
        hold.release(adopt_head).await;
    }
    interrupt.exit_if_interrupted();
    let (outcome, remembered) = restored?;

    let out = RestoreOut {
        save_id,
        version,
        destination: outcome.destination.display().to_string(),
        dry_run: false,
        preview,
        preview_error,
        restored: Some(RestoredOut {
            files_extracted: outcome.files_extracted as u64,
            bytes_extracted: outcome.bytes_extracted,
            files_reused: outcome.files_reused as u64,
            bytes_reused: outcome.bytes_reused,
            destination: outcome.destination.display().to_string(),
            kept_in: outcome.kept_in.as_ref().map(|p| p.display().to_string()),
        }),
        remembered,
    };

    output::emit(&out, |out| {
        let Some(r) = &out.restored else { return };
        println!(
            "restored {} files ({}) to {}",
            r.files_extracted,
            fmt_bytes(r.bytes_extracted),
            r.destination
        );
        if r.files_reused > 0 {
            println!(
                "  {} of them ({}) were already on disk — copied, not downloaded",
                r.files_reused,
                fmt_bytes(r.bytes_reused)
            );
        }
        if let Some(kept) = &r.kept_in {
            println!("  the files it replaced were copied to {kept} first");
        }
        if let Some(applied) = &out.remembered {
            println!(
                "  {} is now this save's folder here ({applied})",
                r.destination
            );
        }
    })
}

fn clone_preview(p: &PreviewOut) -> PreviewOut {
    PreviewOut {
        modified: p.modified.clone(),
        added: p.added.clone(),
        local_only: p.local_only.clone(),
        modified_count: p.modified_count,
        added_count: p.added_count,
        local_only_count: p.local_only_count,
        unchanged: p.unchanged,
        bytes_to_write: p.bytes_to_write,
        comparable: p.comparable,
    }
}

/// The human rendering of what a restore would do.
///
/// `full` (that is, `--dry-run`) names every file, because the whole point of the
/// dry run is to decide, and "3 files overwritten" does not say whether one of
/// them is the campaign you played last night. Without it, the same figures stay
/// on one line above the restore that follows.
fn print_preview(out: &RestoreOut, full: bool) {
    if let Some(e) = &out.preview_error {
        println!("couldn't check what changes ({e})");
        return;
    }
    let Some(p) = &out.preview else { return };

    if !p.comparable {
        println!(
            "v{} of {} doesn't list its files one by one, so there is nothing to \
             compare against {} — the restore would overwrite the folder without a preview",
            out.version, out.save_id, out.destination
        );
        return;
    }

    println!(
        "{} file(s) overwritten, {} created, {} already match, {} only here ({} to write)",
        p.modified_count,
        p.added_count,
        p.unchanged,
        p.local_only_count,
        indicatif::HumanBytes(p.bytes_to_write),
    );

    if !full {
        return;
    }

    let listed = |label: &str, files: &[String], total: usize| {
        if total == 0 {
            return;
        }
        println!();
        println!("{label} ({total}):");
        for f in files {
            println!("  {f}");
        }
        if total > files.len() {
            println!("  … and {} more not listed", total - files.len());
        }
    };
    listed("overwritten", &p.modified, p.modified_count);
    listed("created", &p.added, p.added_count);
    listed(
        "only on disk (kept, but newer than this version)",
        &p.local_only,
        p.local_only_count,
    );
    println!();
    println!(
        "dry run: nothing was written to {}. Re-run without --dry-run to restore.",
        out.destination
    );
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

/// Ctrl+C while the restored files are being swapped in would leave the folder
/// half old, half new, with the originals parked where nobody looks. That
/// phase is short, so the interrupt waits for it; outside it Ctrl+C cancels the
/// restore, which cleans up its staging folder, and the command stops.
struct PlacementGuard {
    placing: Arc<std::sync::atomic::AtomicBool>,
    interrupted: Arc<std::sync::atomic::AtomicBool>,
    cancel: Arc<tokio::sync::Notify>,
    listener: tokio::task::JoinHandle<()>,
}

impl PlacementGuard {
    fn install() -> Self {
        use std::sync::atomic::Ordering;
        let placing = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let interrupted = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let cancel = Arc::new(tokio::sync::Notify::new());
        let (p, i, c) = (placing.clone(), interrupted.clone(), cancel.clone());
        let listener = tokio::spawn(async move {
            while tokio::signal::ctrl_c().await.is_ok() {
                i.store(true, Ordering::SeqCst);
                if p.load(Ordering::SeqCst) {
                    eprintln!("\nfinishing putting the files in place; stopping right after");
                } else {
                    c.notify_one();
                }
            }
        });
        Self {
            placing,
            interrupted,
            cancel,
            listener,
        }
    }

    fn placing(&self, on: bool) {
        self.placing.store(on, std::sync::atomic::Ordering::SeqCst);
    }

    /// Resolves on a Ctrl+C outside the placement.
    async fn cancelled(&self) {
        self.cancel.notified().await;
    }

    /// Honour a Ctrl+C that arrived during the placement, now that it is over
    /// and the service has been told.
    fn exit_if_interrupted(&self) {
        if self.interrupted.load(std::sync::atomic::Ordering::SeqCst) {
            std::process::exit(130);
        }
    }
}

impl Drop for PlacementGuard {
    fn drop(&mut self) {
        self.listener.abort();
    }
}
