//! Auto-detect installed games on the host **without talking to the server**.
//!
//! Detection runs against the catalog embedded in [`hoard_manifest`]:
//! ~20k games imported from the Ludusavi public manifest at build time, plus
//! the hand-curated TOML entries. Both sources are merged so the user sees
//! every game that has a save-path definition we know about, full stop —
//! no server round-trips, no "only ten games found" because the admin
//! hasn't run a manifest import yet.
//!
//! Two complementary signals decide whether a game is *installed*:
//!
//! 1. **Filesystem heuristic** — for each catalog entry, expand its
//!    save-path templates against the local environment (`<winAppData>`,
//!    `<xdgData>`, `<home>`, …) and check whether any expanded directory
//!    actually exists. A hit means the user has played (or at least
//!    installed) the game on this machine. Catches GOG, Epic, DRM-free,
//!    pirated installs — anything that left a save folder behind.
//! 2. **Steam library scan** — read Steam's `libraryfolders.vdf` and
//!    `appmanifest_<id>.acf` files to enumerate installed Steam apps,
//!    then cross-reference their `appid` against the catalog. Finds
//!    games even when no save folder has been written yet.
//!
//! Results from both sources are merged by slug; if a game shows up in
//! both we promote its confidence to `High` and tag the source as `Both`.
//!
//! Disk IO for the filesystem heuristic is fanned out via a Tokio
//! semaphore so we don't open thousands of file handles at once. The
//! `progress(done, total)` callback fires as we churn through the
//! catalog; the Tauri command pipes it to the frontend as a
//! `library://scan-progress` event.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use hoard_manifest::ludusavi::{self, LudusaviEntry};
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;

use crate::manifest::Os;
use crate::pathexpand::expand_path;
use crate::steam::{self, SteamApp};

/// How sure we are that the game is actually installed locally.
///
/// `High` means we have two independent signals (e.g. filesystem hit + Steam
/// manifest), `Medium` means one strong signal, `Low` is reserved for
/// ambiguous matches we still want to surface to the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    Low,
    Medium,
    High,
}

/// Where the detection signal came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DetectionSource {
    FilesystemHeuristic,
    SteamLibrary,
    Both,
}

/// One game we believe is installed on this machine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedGame {
    pub slug: String,
    pub display_name: String,
    /// **Save**-path candidates that exist on disk. Never contains the game's
    /// install directory — that lives in [`install_dir`] so the UI can show
    /// it as a hint without us accidentally backing up the game binary.
    /// Empty for Steam-only matches where no save folder has been created yet.
    pub found_paths: Vec<PathBuf>,
    pub confidence: Confidence,
    pub source: DetectionSource,
    /// If we matched via Steam, the app id is preserved so the UI can show it.
    pub steam_app_id: Option<u64>,
    /// Steam install directory (e.g. `…/steamapps/common/Stellaris`). Only
    /// set when we matched via Steam. Surfaced to the UI as a hint near the
    /// folder picker — **must not** be used as a backup path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub install_dir: Option<PathBuf>,
}

/// Aggregate result of a detection pass. The numeric counts let the UI show a
/// summary banner ("Found 47 games") without re-counting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionReport {
    pub games: Vec<DetectedGame>,
    pub catalog_size: usize,
    pub steam_apps_found: usize,
    pub scanned_at_ms: u64,
}

/// Cap on how many filesystem stats we run concurrently. 32 is well below
/// any reasonable file-descriptor limit while still saturating an SSD.
const FS_PARALLELISM: usize = 32;

/// Granularity of the progress callback. Firing once per game on a 20k-entry
/// catalog would spam the IPC channel; we batch by chunks of this many.
const PROGRESS_CHUNK: usize = 256;

/// Run filesystem + Steam scans against the embedded catalog, merge by slug,
/// and report.
///
/// `progress(done, total)` fires as we work through the catalog so the UI
/// can drive a progress bar. The future is cancellation-safe: dropping it
/// stops the scan without leaking semaphore permits or open files.
///
/// This function does **not** touch the network — the catalog ships in the
/// binary. That keeps the desktop app working on first launch on a fresh
/// Windows machine before the user has even pointed it at a server.
pub async fn detect_all<F>(os: Os, progress: F) -> Result<DetectionReport>
where
    F: Fn(usize, usize) + Send + Sync + 'static,
{
    let started = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    // ---- Steam scan ---------------------------------------------------
    // Cheap (just file reads under the Steam install) so we always run it.
    // A failure here means Steam isn't installed or the user revoked
    // access — log it loudly so the agent log shows *why* a Steam-heavy
    // user got an empty scan, then fall through to the filesystem pass.
    let steam_apps = match steam::list_installed_steam_games(os) {
        Ok(apps) => apps,
        Err(e) => {
            tracing::warn!(error = %e, "Steam library scan failed; continuing without it");
            Vec::new()
        }
    };
    tracing::info!(count = steam_apps.len(), "Steam apps discovered");

    let steam_by_appid: HashMap<u64, &SteamApp> =
        steam_apps.iter().map(|a| (a.app_id, a)).collect();

    // ---- Catalog walk -------------------------------------------------
    let catalog = ludusavi::catalog();
    let catalog_size = catalog.len();
    tracing::info!(catalog_size, "Detecting against embedded catalog");

    let progress = Arc::new(progress);
    let semaphore = Arc::new(Semaphore::new(FS_PARALLELISM));

    let mut by_slug: HashMap<String, DetectedGame> = HashMap::new();

    // Steam cross-reference is O(catalog) but cheap (just hashmap lookups);
    // do it up-front so every Steam-installed game shows up even if the
    // filesystem pass below skips it because no save folder exists yet.
    for entry in catalog {
        let Some(appid) = entry.steam_app_id else {
            continue;
        };
        let Some(app) = steam_by_appid.get(&appid) else {
            continue;
        };
        by_slug.insert(
            entry.slug.clone(),
            DetectedGame {
                slug: entry.slug.clone(),
                display_name: entry.display_name.clone(),
                // Steam tells us where the game is *installed*, not where it
                // writes saves. Leaving `found_paths` empty is critical: the
                // UI's track() falls back to the folder picker when this is
                // empty, instead of silently backing up the install dir.
                found_paths: Vec::new(),
                confidence: Confidence::Medium,
                source: DetectionSource::SteamLibrary,
                steam_app_id: Some(app.app_id),
                install_dir: Some(app.install_dir.clone()),
            },
        );
    }
    tracing::info!(
        steam_matches = by_slug.len(),
        "Steam → catalog cross-reference complete"
    );

    // Filesystem heuristic: spawn one blocking task per game, gated by the
    // semaphore. Each task expands every Windows/Linux/Mac template that
    // applies to the current OS and stat()s every candidate path.
    let mut tasks = Vec::new();
    for entry in catalog {
        let templates: Vec<String> = paths_for_os(entry, os);
        if templates.is_empty() {
            continue;
        }
        let slug = entry.slug.clone();
        let display_name = entry.display_name.clone();
        let permit = semaphore.clone().acquire_owned().await?;
        tasks.push(tokio::task::spawn_blocking(move || {
            // _permit drops at end of closure, releasing the slot.
            let _permit = permit;
            let mut hits: Vec<PathBuf> = Vec::new();
            let mut seen: HashSet<PathBuf> = HashSet::new();
            for tmpl in &templates {
                let candidates = expand_path(tmpl, os);
                if candidates.is_empty() {
                    // Unknown placeholder or unset env var — pathexpand
                    // already returns vec![] for those. Useful to log
                    // once per *unknown* template so the agent log can
                    // tell us what's missing in pathexpand.
                    if !tmpl.is_empty() && tmpl.starts_with('<') {
                        tracing::trace!(template = %tmpl, "expand_path returned no candidates");
                    }
                    continue;
                }
                for candidate in candidates {
                    if !candidate.exists() {
                        continue;
                    }
                    if seen.insert(candidate.clone()) {
                        hits.push(candidate);
                    }
                }
            }
            if hits.is_empty() {
                None
            } else {
                Some((slug, display_name, hits))
            }
        }));
    }

    let total_tasks = tasks.len();
    progress(0, total_tasks);

    let mut done = 0usize;
    for t in tasks {
        match t.await {
            Ok(Some((slug, display_name, hits))) => {
                merge_fs_hit(&mut by_slug, slug, display_name, hits);
            }
            Ok(None) => {}
            Err(e) => {
                tracing::warn!(error = %e, "filesystem-heuristic task panicked");
            }
        }
        done += 1;
        // Batch progress events so we don't spam the IPC channel for every
        // single one of the catalog's ~20k entries.
        if done.is_multiple_of(PROGRESS_CHUNK) {
            progress(done, total_tasks);
        }
    }

    // Promote confidence wherever both signals fired.
    for game in by_slug.values_mut() {
        if matches!(game.source, DetectionSource::Both) {
            game.confidence = Confidence::High;
        }
    }

    progress(total_tasks, total_tasks);

    let mut games: Vec<DetectedGame> = by_slug.into_values().collect();
    games.sort_by(|a, b| a.display_name.cmp(&b.display_name));

    tracing::info!(
        detected = games.len(),
        catalog_size,
        steam_apps = steam_apps.len(),
        "Detection complete"
    );

    Ok(DetectionReport {
        games,
        catalog_size,
        steam_apps_found: steam_apps.len(),
        scanned_at_ms: started,
    })
}

/// Pull the list of save-path template strings that apply to the requested
/// OS for a single Ludusavi entry. Strips constraints/tags — detection only
/// cares about the path itself.
fn paths_for_os(entry: &LudusaviEntry, os: Os) -> Vec<String> {
    let slot = match os {
        Os::Windows => &entry.paths.windows,
        Os::Linux => &entry.paths.linux,
        Os::Mac => &entry.paths.mac,
    };
    slot.iter().map(|p| p.path.clone()).collect()
}

/// Merge a filesystem hit into the dedupe map, promoting source/confidence
/// when an existing Steam entry is already present.
fn merge_fs_hit(
    by_slug: &mut HashMap<String, DetectedGame>,
    slug: String,
    display_name: String,
    hits: Vec<PathBuf>,
) {
    match by_slug.get_mut(&slug) {
        Some(existing) => {
            // Both signals — strongest possible match.
            existing.source = DetectionSource::Both;
            existing.confidence = Confidence::High;
            for h in hits {
                if !existing.found_paths.contains(&h) {
                    existing.found_paths.push(h);
                }
            }
        }
        None => {
            by_slug.insert(
                slug.clone(),
                DetectedGame {
                    slug,
                    display_name,
                    found_paths: hits,
                    confidence: Confidence::Medium,
                    source: DetectionSource::FilesystemHeuristic,
                    steam_app_id: None,
                    install_dir: None,
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_fs_hit_promotes_existing_steam_entry() {
        let mut map = HashMap::new();
        map.insert(
            "x".to_string(),
            DetectedGame {
                slug: "x".into(),
                display_name: "X".into(),
                found_paths: Vec::new(),
                confidence: Confidence::Medium,
                source: DetectionSource::SteamLibrary,
                steam_app_id: Some(42),
                install_dir: Some(PathBuf::from("/steam/x")),
            },
        );

        merge_fs_hit(
            &mut map,
            "x".to_string(),
            "X".to_string(),
            vec![PathBuf::from("/save/x")],
        );

        let g = &map["x"];
        assert_eq!(g.source, DetectionSource::Both);
        assert_eq!(g.confidence, Confidence::High);
        // Only the real save path; install_dir stays out of found_paths.
        assert_eq!(g.found_paths, vec![PathBuf::from("/save/x")]);
        assert_eq!(g.steam_app_id, Some(42));
        assert_eq!(g.install_dir, Some(PathBuf::from("/steam/x")));
    }

    #[test]
    fn merge_fs_hit_creates_new_entry_when_absent() {
        let mut map = HashMap::new();
        merge_fs_hit(
            &mut map,
            "y".to_string(),
            "Y".to_string(),
            vec![PathBuf::from("/save/y")],
        );
        let g = &map["y"];
        assert_eq!(g.source, DetectionSource::FilesystemHeuristic);
        assert_eq!(g.confidence, Confidence::Medium);
        assert!(g.steam_app_id.is_none());
    }
}
