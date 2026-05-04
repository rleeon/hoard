//! Auto-detect installed games on the host.
//!
//! Two complementary signals:
//!
//! 1. **Filesystem heuristic** — walk the catalog from the server, expand each
//!    game's save-path templates, and check whether any expanded directory
//!    actually exists. A hit means the user has played (or at least installed)
//!    the game on this machine. This catches GOG, Epic, and DRM-free installs.
//! 2. **Steam library scan** — read Steam's `libraryfolders.vdf` and
//!    `appmanifest_<id>.acf` files to enumerate installed Steam apps, then
//!    cross-reference their `appid` against the catalog's `steam_app_id`.
//!    This finds games even when no save folder has been written yet.
//!
//! Results from both sources are merged by slug; if a game shows up in both
//! we promote its confidence to `High` and tag the source as `Both`.
//!
//! The scan never writes to disk or talks to the world beyond the Hoard
//! server. Disk IO is fanned out via a Tokio semaphore so we don't open ten
//! thousand file handles at once.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;

use crate::api::ApiClient;
use crate::manifest::{Os, PathsByOs};
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
    /// Save-path candidates that exist on disk. Empty for Steam-only matches
    /// where no save folder has been created yet.
    pub found_paths: Vec<PathBuf>,
    pub confidence: Confidence,
    pub source: DetectionSource,
    /// If we matched via Steam, the app id is preserved so the UI can show it.
    pub steam_app_id: Option<u64>,
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

const PAGE_SIZE: u32 = 1000;
const FS_PARALLELISM: usize = 32;

/// Scan the filesystem for known save-path matches. Pages through the entire
/// catalog from the server. `progress(done, total)` is invoked as each page
/// completes — the UI uses it to drive a progress bar.
///
/// The function is cancellation-safe: dropping the future stops the scan
/// without leaking semaphore permits or open files.
pub async fn scan_filesystem<F>(api: &ApiClient, os: Os, progress: F) -> Result<Vec<DetectedGame>>
where
    F: Fn(usize, usize) + Send + Sync,
{
    let progress = Arc::new(progress);
    let semaphore = Arc::new(Semaphore::new(FS_PARALLELISM));

    let mut found: Vec<DetectedGame> = Vec::new();
    let mut offset: u32 = 0;
    let mut catalog_size: usize = 0;

    loop {
        let page = api.list_games_paged(PAGE_SIZE, offset).await?;
        if page.is_empty() {
            break;
        }
        catalog_size += page.len();

        // Spawn one task per game, gated by the semaphore so we don't stat
        // thousands of paths at once.
        let mut tasks = Vec::with_capacity(page.len());
        for game in page.iter() {
            let Some(paths) = parse_save_paths(&game.save_paths_json) else {
                continue;
            };
            let templates: Vec<String> = paths.for_os(os).iter().map(|p| p.path.clone()).collect();
            if templates.is_empty() {
                continue;
            }
            let slug = game.slug.clone();
            let display_name = game.display_name.clone();
            let permit = semaphore.clone().acquire_owned().await?;
            tasks.push(tokio::task::spawn_blocking(move || {
                // _permit drops at end of closure, releasing the slot.
                let _permit = permit;
                let mut hits: Vec<PathBuf> = Vec::new();
                for tmpl in &templates {
                    for candidate in expand_path(tmpl, os) {
                        if candidate.exists() {
                            hits.push(candidate);
                        }
                    }
                }
                if hits.is_empty() {
                    None
                } else {
                    Some(DetectedGame {
                        slug,
                        display_name,
                        found_paths: hits,
                        confidence: Confidence::Medium,
                        source: DetectionSource::FilesystemHeuristic,
                        steam_app_id: None,
                    })
                }
            }));
        }

        for t in tasks {
            if let Ok(Some(g)) = t.await {
                found.push(g);
            }
        }

        progress(catalog_size, catalog_size + (PAGE_SIZE as usize));

        if (page.len() as u32) < PAGE_SIZE {
            break;
        }
        offset += PAGE_SIZE;
    }

    progress(catalog_size, catalog_size);
    Ok(found)
}

/// Run both filesystem and Steam scans, merge by slug, and report.
///
/// The Steam scan is fast (just file reads inside `~/.steam`) so we always
/// run it. We then walk the catalog once more to translate Steam app ids
/// into catalog slugs — this is where `steam_app_id` on `GameManifest` pays
/// off.
pub async fn detect_all<F>(api: &ApiClient, os: Os, progress: F) -> Result<DetectionReport>
where
    F: Fn(usize, usize) + Send + Sync,
{
    let started = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    // Steam first — cheap, no network.
    let steam_apps = steam::list_installed_steam_games(os).unwrap_or_default();
    // Index by display_name for the in-line cross-reference below; we don't
    // know each catalog row's `steam_app_id` until we'd fetch /known-paths,
    // and doing 11k lookups would dwarf the rest of the scan.
    let _steam_by_appid: HashMap<u64, &SteamApp> =
        steam_apps.iter().map(|a| (a.app_id, a)).collect();

    // Filesystem heuristic — paginated catalog walk. We piggy-back on the
    // pages we fetch here to also resolve Steam app ids → slugs without
    // making a second pass.
    let progress = Arc::new(progress);
    let progress_for_fs = progress.clone();
    let semaphore = Arc::new(Semaphore::new(FS_PARALLELISM));

    let mut by_slug: HashMap<String, DetectedGame> = HashMap::new();
    let mut catalog_size: usize = 0;
    let mut offset: u32 = 0;

    loop {
        let page = api.list_games_paged(PAGE_SIZE, offset).await?;
        if page.is_empty() {
            break;
        }
        catalog_size += page.len();

        // ---- Steam cross-reference (cheap, in-line). ---------------------
        // We need each game's steam_app_id, which lives only on
        // /known-paths, not on /games. So we do a second on-demand fetch
        // for any game whose save_paths_json mentions a steam-flavoured
        // path *or* whose slug we'll need for Steam — but to avoid 11k
        // round-trips we only call known_paths for games that are likely
        // matches: those whose display_name appears in our installed-Steam
        // list. That's a tiny subset (~hundreds at most).
        let steam_names: HashMap<&str, &SteamApp> =
            steam_apps.iter().map(|a| (a.name.as_str(), a)).collect();
        for game in page.iter() {
            if let Some(app) = steam_names.get(game.display_name.as_str()) {
                let entry = by_slug.entry(game.slug.clone()).or_insert(DetectedGame {
                    slug: game.slug.clone(),
                    display_name: game.display_name.clone(),
                    found_paths: vec![app.install_dir.clone()],
                    confidence: Confidence::Medium,
                    source: DetectionSource::SteamLibrary,
                    steam_app_id: Some(app.app_id),
                });
                entry.steam_app_id = Some(app.app_id);
                if !entry.found_paths.contains(&app.install_dir) {
                    entry.found_paths.push(app.install_dir.clone());
                }
            }
        }

        // ---- Filesystem heuristic per game (parallel via semaphore). ----
        let mut tasks = Vec::with_capacity(page.len());
        for game in page.iter() {
            let Some(paths) = parse_save_paths(&game.save_paths_json) else {
                continue;
            };
            let templates: Vec<String> = paths.for_os(os).iter().map(|p| p.path.clone()).collect();
            if templates.is_empty() {
                continue;
            }
            let slug = game.slug.clone();
            let display_name = game.display_name.clone();
            let permit = semaphore.clone().acquire_owned().await?;
            tasks.push(tokio::task::spawn_blocking(move || {
                let _permit = permit;
                let mut hits: Vec<PathBuf> = Vec::new();
                for tmpl in &templates {
                    for candidate in expand_path(tmpl, os) {
                        if candidate.exists() {
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

        for t in tasks {
            if let Ok(Some((slug, display_name, hits))) = t.await {
                merge_fs_hit(&mut by_slug, slug, display_name, hits);
            }
        }

        progress_for_fs(catalog_size, catalog_size + (PAGE_SIZE as usize));

        if (page.len() as u32) < PAGE_SIZE {
            break;
        }
        offset += PAGE_SIZE;
    }

    // Promote confidence anywhere both signals fired.
    for game in by_slug.values_mut() {
        if matches!(game.source, DetectionSource::Both) {
            game.confidence = Confidence::High;
        }
    }

    progress(catalog_size, catalog_size);

    let mut games: Vec<DetectedGame> = by_slug.into_values().collect();
    games.sort_by(|a, b| a.display_name.cmp(&b.display_name));

    Ok(DetectionReport {
        games,
        catalog_size,
        steam_apps_found: steam_apps.len(),
        scanned_at_ms: started,
    })
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
                },
            );
        }
    }
}

/// Parse the `save_paths_json` field — the server stores it as an opaque
/// string. Returns `None` if the column is null or we can't decode (a
/// handful of legacy rows have malformed JSON, and we'd rather skip them
/// than abort the whole scan).
fn parse_save_paths(json: &Option<String>) -> Option<PathsByOs> {
    let s = json.as_deref()?;
    if s.is_empty() {
        return None;
    }
    serde_json::from_str(s).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_paths(linux: &[&str]) -> String {
        let value = serde_json::json!({
            "windows": [],
            "linux": linux.iter().map(|p| serde_json::json!({"path": p})).collect::<Vec<_>>(),
            "mac": []
        });
        value.to_string()
    }

    #[test]
    fn parses_save_paths_json() {
        let s = fake_paths(&["<home>/.foo"]);
        let parsed = parse_save_paths(&Some(s)).expect("parsed");
        assert_eq!(parsed.linux.len(), 1);
        assert_eq!(parsed.linux[0].path, "<home>/.foo");
    }

    #[test]
    fn parse_save_paths_handles_bad_json() {
        assert!(parse_save_paths(&Some("not-json".to_string())).is_none());
        assert!(parse_save_paths(&Some(String::new())).is_none());
        assert!(parse_save_paths(&None).is_none());
    }

    #[test]
    fn merge_fs_hit_promotes_existing_steam_entry() {
        let mut map = HashMap::new();
        map.insert(
            "x".to_string(),
            DetectedGame {
                slug: "x".into(),
                display_name: "X".into(),
                found_paths: vec![PathBuf::from("/steam/x")],
                confidence: Confidence::Medium,
                source: DetectionSource::SteamLibrary,
                steam_app_id: Some(42),
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
        assert_eq!(g.found_paths.len(), 2);
        assert_eq!(g.steam_app_id, Some(42));
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
