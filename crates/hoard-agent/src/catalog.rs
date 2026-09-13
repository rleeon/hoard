//! The game catalogue (Ludusavi) as the app keeps it: refreshed from upstream
//! into the user's cache, with a small sidecar that says when.
//!
//! The service refreshes it (ADR 0021, Slice 8), so the window never loads it. A
//! desktop that meets an older service still refreshes on its own, through these
//! same functions.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

/// How stale the cached catalogue gets before a refresh: a day, which is how often
/// Ludusavi publishes, and what the Settings copy promises.
pub const REFRESH_AFTER: Duration = Duration::from_secs(24 * 60 * 60);

/// Sidecar next to the catalogue JSON, so "updated N days ago" needs no parse of
/// the catalogue itself.
const META_FILENAME: &str = "ludusavi-catalog.meta.json";

/// The catalogue in use.
#[derive(Debug, Clone, Serialize)]
pub struct CatalogStatus {
    pub games: usize,
    /// A downloaded copy is in use rather than the one the build shipped with.
    pub has_runtime_override: bool,
    /// Unix seconds of the last refresh, when there was one.
    pub updated_at: Option<u64>,
}

/// A refresh that went through.
#[derive(Debug, Clone, Serialize)]
pub struct CatalogUpdate {
    pub games: usize,
    pub updated_at: u64,
    pub size_bytes: u64,
    /// For diagnostics; never shown to users.
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Meta {
    version: u8,
    updated_at: u64,
    games: usize,
}

fn meta_path() -> Option<PathBuf> {
    let runtime = hoard_manifest::ludusavi::runtime_override_path()?;
    Some(runtime.with_file_name(META_FILENAME))
}

fn read_meta() -> Option<Meta> {
    let bytes = std::fs::read(meta_path()?).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Download the upstream manifest, convert it and store it as the runtime
/// override. `stage` hears `downloading`, `parsing`, `saving` and `done`, for a UI
/// that wants to say which.
///
/// The process that runs this keeps using the catalogue it loaded: the new one is
/// picked up by the next start, because swapping it under a running sweep would
/// make one report out of two catalogues.
pub async fn refresh(stage: impl Fn(&'static str)) -> anyhow::Result<CatalogUpdate> {
    let url = hoard_manifest::ludusavi::DEFAULT_UPSTREAM_URL;
    stage("downloading");
    tracing::info!(url, "fetching Ludusavi manifest");
    // The YAML is ~17 MB and GitHub raw is usually quick; the ceiling is for a
    // stream that stalls, which would otherwise hold the refresh for ever.
    let client = reqwest::Client::builder()
        .user_agent(concat!("hoard-agent/", env!("CARGO_PKG_VERSION")))
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

    stage("parsing");
    let games =
        tokio::task::spawn_blocking(move || hoard_manifest::ludusavi::save_runtime_override(&yaml))
            .await??;

    stage("saving");
    let path = hoard_manifest::ludusavi::runtime_override_path()
        .ok_or_else(|| anyhow::anyhow!("no cache dir on this system"))?;
    let size_bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let updated_at = now_secs();
    if let Some(mp) = meta_path() {
        let meta = Meta {
            version: 1,
            updated_at,
            games,
        };
        // Best effort: without it Settings says "unknown" instead of a date, and the
        // next check refreshes again, which is the safe side to err on.
        if let Ok(json) = serde_json::to_vec(&meta) {
            let _ = std::fs::write(&mp, json);
        }
    }
    stage("done");
    Ok(CatalogUpdate {
        games,
        updated_at,
        size_bytes,
        path: path.display().to_string(),
    })
}

/// The catalogue in use. It loads it when nothing has yet, which is why only the
/// service should ask.
pub fn status() -> CatalogStatus {
    let has_runtime_override = hoard_manifest::ludusavi::runtime_override_path()
        .map(|p| p.exists())
        .unwrap_or(false);
    CatalogStatus {
        games: hoard_manifest::ludusavi::catalog_size(),
        has_runtime_override,
        updated_at: read_meta().map(|m| m.updated_at),
    }
}

/// The cached copy is missing, unreadable or older than [`REFRESH_AFTER`].
pub fn is_stale() -> bool {
    match read_meta() {
        Some(meta) => now_secs().saturating_sub(meta.updated_at) >= REFRESH_AFTER.as_secs(),
        None => true,
    }
}

/// The Steam app id the catalogue gives a slug, for the cover art.
pub fn steam_app_id(slug: &str) -> Option<u64> {
    hoard_manifest::ludusavi::find_by_slug(slug).and_then(|e| e.steam_app_id)
}

/// What a window asks the service about one game instead of loading the catalogue.
pub fn facts(slug: &str) -> hoard_core::ipc::GameFacts {
    hoard_core::ipc::GameFacts {
        steam_app_id: steam_app_id(slug),
        shields: crate::savefilter::shields_for_slug(slug),
    }
}
