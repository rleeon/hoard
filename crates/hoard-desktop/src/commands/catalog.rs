//! Catalog (Ludusavi) update commands.
//!
//! The desktop ships a compile-time embedded copy of the Ludusavi catalog
//! (~20k games, see `hoard-manifest::ludusavi`). This module exposes
//! Tauri commands that let the frontend (or the app's own startup hook)
//! download a fresh copy from upstream and persist it as a *runtime
//! override* in the OS cache dir. The next process launch picks the
//! refreshed catalog up automatically; there's no in-process hot-swap
//! because mid-run swaps would make detection results inconsistent
//! across overlapping scans.
//!
//! Wire shape:
//!
//! - `update_catalog()`: the synchronous command behind the "Check for
//!   updates" button. Returns [`CatalogUpdateResult`] on success.
//! - `catalog_status()`: read-only metadata for the Settings page, so we
//!   can show "20,731 games · updated 3 days ago".
//! - `auto_update_catalog_in_background()`: the background loop spawned from
//!   `setup()`; it re-checks hourly and refreshes once the cached catalog is
//!   older than [`AUTO_UPDATE_AFTER`]. The loop (rather than the old
//!   launch-time one-shot) matters because the app lives in the tray for
//!   weeks: a check only at startup meant the catalog could silently go
//!   stale.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

/// How stale the cached catalog must be before the background auto-refresh
/// kicks in. One day, matching Ludusavi's ~daily upstream pushes; the
/// Settings copy promises the same cadence.
const AUTO_UPDATE_AFTER: Duration = Duration::from_secs(24 * 60 * 60);

/// How often the background loop re-checks staleness. Checking is a local
/// metadata read; the 17 MB download only happens past [`AUTO_UPDATE_AFTER`].
const RECHECK_EVERY: Duration = Duration::from_secs(60 * 60);

/// Sidecar metadata file written next to the catalog JSON. Tells the UI
/// when the user last refreshed and how big the result was, without
/// re-parsing the 7 MB JSON on every Settings render.
const META_FILENAME: &str = "ludusavi-catalog.meta.json";

/// Result returned from a successful catalog update.
#[derive(Debug, Clone, Serialize)]
pub struct CatalogUpdateResult {
    /// Number of games in the freshly written catalog.
    pub games: usize,
    /// Unix epoch seconds when the catalog was written.
    pub updated_at: u64,
    /// On-disk size of the JSON catalog in bytes.
    pub size_bytes: u64,
    /// Path on disk for diagnostics; never shown to end users.
    pub path: String,
}

/// Read-only status surfaced in Settings.
#[derive(Debug, Clone, Serialize)]
pub struct CatalogStatus {
    /// Number of games in the catalog currently in use. Reflects the
    /// runtime override if present, else the embedded count.
    pub games: usize,
    /// Whether a runtime override exists on disk. `false` means the app
    /// is using the version it shipped with.
    pub has_runtime_override: bool,
    /// Unix epoch seconds when the override was last written, if any.
    pub updated_at: Option<u64>,
}

/// Persisted sidecar shape. Versioned so we can change it without
/// invalidating older caches catastrophically.
#[derive(Debug, Serialize, Deserialize)]
struct CatalogMeta {
    version: u8,
    updated_at: u64,
    games: usize,
}

fn meta_path() -> Option<PathBuf> {
    let runtime = hoard_manifest::ludusavi::runtime_override_path()?;
    Some(runtime.with_file_name(META_FILENAME))
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Tauri command: download the upstream Ludusavi YAML, convert it to our
/// compact JSON, and write it as the runtime override. Emits
/// `catalog://update-progress` (`{ stage }`) at each major step so the
/// UI can show "Downloading…", "Parsing…", "Saving…" instead of a frozen
/// spinner.
#[tauri::command]
pub async fn update_catalog(app: AppHandle) -> Result<CatalogUpdateResult, String> {
    do_update(Some(app)).await.map_err(|e| e.to_string())
}

async fn do_update(app: Option<AppHandle>) -> Result<CatalogUpdateResult, anyhow::Error> {
    let url = hoard_manifest::ludusavi::DEFAULT_UPSTREAM_URL;
    emit_stage(&app, "downloading");
    tracing::info!(url, "fetching Ludusavi manifest");

    // 60s timeout: the YAML is ~17 MB and GitHub raw is generally fast,
    // but we don't want a wedged TCP stream to hang the spinner forever.
    let client = reqwest::Client::builder()
        .user_agent(concat!("hoard-desktop/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(60))
        .build()?;
    let yaml = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    tracing::info!(bytes = yaml.len(), "manifest downloaded");

    emit_stage(&app, "parsing");
    // Conversion is CPU-bound; spawn_blocking so the UI thread doesn't
    // stall on a 17 MB YAML parse + 20k entry transform (~1-2s).
    let games =
        tokio::task::spawn_blocking(move || hoard_manifest::ludusavi::save_runtime_override(&yaml))
            .await??;

    emit_stage(&app, "saving");
    let path = hoard_manifest::ludusavi::runtime_override_path()
        .ok_or_else(|| anyhow::anyhow!("no cache dir on this system"))?;
    let size_bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let updated_at = now_secs();

    // Sidecar so Settings can render "updated N days ago" without
    // re-parsing the 7 MB catalog on each visit.
    let meta = CatalogMeta {
        version: 1,
        updated_at,
        games,
    };
    if let Some(mp) = meta_path() {
        if let Ok(json) = serde_json::to_vec(&meta) {
            // Best-effort: a missing meta file just means the UI shows
            // "unknown" instead of a date. Not worth surfacing as an error.
            let _ = std::fs::write(&mp, json);
        }
    }

    emit_stage(&app, "done");
    Ok(CatalogUpdateResult {
        games,
        updated_at,
        size_bytes,
        path: path.display().to_string(),
    })
}

fn emit_stage(app: &Option<AppHandle>, stage: &str) {
    if let Some(handle) = app {
        let _ = handle.emit("catalog://update-progress", stage);
    }
}

/// Tauri command: report the currently-loaded catalog status. Cheap, since it
/// only touches the meta sidecar and never re-parses the catalog JSON.
#[tauri::command]
pub fn catalog_status() -> CatalogStatus {
    let games = hoard_manifest::ludusavi::catalog_size();

    let runtime_path = hoard_manifest::ludusavi::runtime_override_path();
    let has_runtime_override = runtime_path.as_ref().map(|p| p.exists()).unwrap_or(false);

    let updated_at = meta_path()
        .and_then(|mp| std::fs::read(&mp).ok())
        .and_then(|bytes| serde_json::from_slice::<CatalogMeta>(&bytes).ok())
        .map(|m| m.updated_at);

    CatalogStatus {
        games,
        has_runtime_override,
        updated_at,
    }
}

/// Background auto-update loop: spawned from `setup()` at app launch. Checks
/// immediately, then every [`RECHECK_EVERY`], refreshing whenever the cached
/// catalog is missing or older than [`AUTO_UPDATE_AFTER`]. Never blocks
/// startup, and silently swallows errors (the user's already running on the
/// embedded catalog, so a failed refresh is degradation, not breakage, and the
/// next tick retries).
pub fn auto_update_catalog_in_background(app: AppHandle) {
    // `tauri::async_runtime::spawn` rather than `tokio::spawn`: this is
    // called from `setup()` which runs *before* Tauri enters its event
    // loop, so there is no ambient Tokio runtime yet. Tauri provides its
    // own multi-thread runtime that's always available.
    tauri::async_runtime::spawn(async move {
        loop {
            let needs_refresh = match meta_path().and_then(|mp| std::fs::read(&mp).ok()) {
                None => true,
                Some(bytes) => match serde_json::from_slice::<CatalogMeta>(&bytes) {
                    Ok(meta) => {
                        let age = now_secs().saturating_sub(meta.updated_at);
                        age >= AUTO_UPDATE_AFTER.as_secs()
                    }
                    Err(_) => true,
                },
            };
            if needs_refresh {
                tracing::info!("Ludusavi catalog stale; running background refresh");
                match do_update(Some(app.clone())).await {
                    Ok(result) => tracing::info!(games = result.games, "auto-refresh complete"),
                    Err(e) => {
                        tracing::warn!(error = %e, "auto-refresh failed; using embedded catalog")
                    }
                }
            } else {
                tracing::debug!("Ludusavi catalog is fresh; skipping auto-update");
            }
            tokio::time::sleep(RECHECK_EVERY).await;
        }
    });
}
