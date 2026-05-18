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
use crate::pathexpand::{expand_path, expand_path_in_prefix};
use crate::state::CliState;
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
    /// User picked the save folder by hand; the override lives in
    /// `CliState::manual_paths` and wins over every heuristic. The row's
    /// `found_paths` is the chosen path verbatim and `confidence` is
    /// `High` — the user knows where their saves are better than any
    /// scrape.
    ManualOverride,
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

/// Folder names we treat as "this is the saves directory". Comparison is
/// case-insensitive and exact-on-segment: a directory called `Saves` matches,
/// `save settings` doesn't (otherwise the heuristic would back up the
/// settings folder by accident — see `docs/plans/detection.md` §9).
const SAVE_PATTERNS: &[&str] = &[
    "save",
    "saves",
    "savegame",
    "savegames",
    "save games",
    "save_games",
];

/// Slugs whose catalog entry points at a "game root" mixing saves with other
/// state (config, mods, telemetry) **and** whose actual save subdirectory has
/// a name that [`SAVE_PATTERNS`] can't recognise. The general heuristic in
/// [`refine_save_dir`] handles every Paradox-style "save games" or "Saves"
/// layout already; this list only exists for atypical cases that surface as
/// bug reports later.
///
/// Empty by design. Add `(slug, "subdir name")` entries here if a real game
/// hides its saves under a folder whose name doesn't match any save pattern.
const SAVE_DIR_OVERRIDES: &[(&str, &str)] = &[];

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
///
/// `state` is read for `manual_paths`: anything the user picked by hand in
/// the folder dialog wins over every heuristic. Pass a freshly-loaded
/// `CliState` (or `&CliState::default()` for the example/smoke binary that
/// has no on-disk state).
pub async fn detect_all<F>(os: Os, state: &CliState, progress: F) -> Result<DetectionReport>
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

    // Second pass: any Steam app whose `app_id` didn't match a catalog entry
    // above might still belong to a catalog entry that simply lacks
    // `steam_app_id`. Try a slug match against the game's display name.
    // This catches the long tail of Ludusavi entries scraped from
    // PCGamingWiki without a Steam appid attached.
    apply_steam_name_fallback(catalog, &steam_apps, &mut by_slug);

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
            let mut root_matched = false;
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
                        root_matched = true;
                    }
                }
            }
            // Refine root-pointing hits down to their save subdirectory
            // (or drop them entirely so the UI shows the amber alert).
            let refined = refine_save_dir(&slug, hits);
            if refined.is_empty() && root_matched {
                // The fs heuristic saw the game on disk but the refinement
                // discarded every hit (either an override demanded a subdir
                // that doesn't exist yet, or the general heuristic found no
                // save-named subdir under the root). Keep the slug in the
                // report with no path so the UI shows an amber alert
                // prompting the user to pick a folder instead of silently
                // dropping the game.
                Some((slug, display_name, Vec::new()))
            } else if refined.is_empty() {
                None
            } else {
                Some((slug, display_name, refined))
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

    // Proton/Wine prefixes: on Linux, expand the Windows save-path templates
    // of each catalog entry against any compatdata prefix Steam has for that
    // appid. Captures the (large) population of Windows-only games that
    // users run via Proton, which would otherwise be invisible because
    // their `paths.linux` is empty.
    if os == Os::Linux {
        let prefixes = steam::list_proton_prefixes(os);
        for prefix in &prefixes {
            let Some(entry) = hoard_manifest::ludusavi::find_by_steam_app_id(prefix.app_id) else {
                continue;
            };
            let mut hits: Vec<PathBuf> = Vec::new();
            let mut seen: HashSet<PathBuf> = HashSet::new();
            for tmpl in entry.paths.windows.iter().map(|p| &p.path) {
                let candidates = expand_path_in_prefix(tmpl, &prefix.prefix_root);
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
                continue;
            }
            // Same merge semantics as a native-fs hit: if the slug already
            // exists from the Steam cross-reference, promote to Both/High;
            // otherwise create a fresh entry.
            merge_fs_hit(
                &mut by_slug,
                entry.slug.clone(),
                entry.display_name.clone(),
                hits,
            );
        }
    }

    // Promote confidence wherever both signals fired.
    for game in by_slug.values_mut() {
        if matches!(game.source, DetectionSource::Both) {
            game.confidence = Confidence::High;
        }
    }

    // Apply user overrides last so they always win, regardless of how strong
    // a heuristic signal the upstream pipeline produced.
    apply_manual_overrides(&state.manual_paths, &mut by_slug);

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

/// Slug-based fallback for Steam apps whose `app_id` didn't show up in the
/// catalog's `steam_app_id` index. For each unmatched Steam app, slugify its
/// display name and look for a catalog entry with the same slug; if found,
/// insert it with `Confidence::Low` because the match is structurally
/// ambiguous (two unrelated games sharing a slugifiable name would collide).
///
/// Skips Steam apps already linked above (their `app_id` is present as
/// `steam_app_id` somewhere in `by_slug`) and slugs already in the dedupe
/// map (the appid pass owns the entry; never demote High → Low).
fn apply_steam_name_fallback(
    catalog: &[LudusaviEntry],
    steam_apps: &[SteamApp],
    by_slug: &mut HashMap<String, DetectedGame>,
) {
    if steam_apps.is_empty() {
        return;
    }
    let matched_appids: HashSet<u64> = by_slug.values().filter_map(|g| g.steam_app_id).collect();
    let catalog_by_slug: HashMap<&str, &LudusaviEntry> =
        catalog.iter().map(|e| (e.slug.as_str(), e)).collect();

    let mut added = 0usize;
    for app in steam_apps {
        if matched_appids.contains(&app.app_id) {
            continue;
        }
        let slug = ludusavi::slugify(&app.name);
        let Some(entry) = catalog_by_slug.get(slug.as_str()) else {
            continue;
        };
        if by_slug.contains_key(&entry.slug) {
            continue;
        }
        by_slug.insert(
            entry.slug.clone(),
            DetectedGame {
                slug: entry.slug.clone(),
                display_name: entry.display_name.clone(),
                found_paths: Vec::new(),
                confidence: Confidence::Low,
                source: DetectionSource::SteamLibrary,
                steam_app_id: Some(app.app_id),
                install_dir: Some(app.install_dir.clone()),
            },
        );
        added += 1;
    }
    if added > 0 {
        tracing::info!(
            added,
            "Steam → catalog slug fallback added entries with Confidence::Low"
        );
    }
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

/// Refine raw filesystem hits down to the actual save directory.
///
/// Order of operations per slug:
///
/// 1. If the slug appears in [`SAVE_DIR_OVERRIDES`], join the configured
///    subdir onto each hit and keep only the ones that exist as a directory.
///    Same semantics as the pre-1.5 hardcoded list.
/// 2. Otherwise, per hit:
///    * If the hit's last path segment matches [`SAVE_PATTERNS`] (exact,
///      case-insensitive), keep it as-is — it already points at a save dir.
///    * Else, list its immediate subdirectories and keep the ones whose
///      name matches [`SAVE_PATTERNS`]. Zero matches drops the hit so the
///      UI falls back to the amber "pick folder" alert; one or more matches
///      replace the hit with the matched subdirs.
///
/// The read_dir in step 2 adds one IO per ambiguous hit but the work runs
/// inside the same `FS_PARALLELISM` semaphore as the existing stat() pass,
/// so total concurrency stays bounded.
fn refine_save_dir(slug: &str, hits: Vec<PathBuf>) -> Vec<PathBuf> {
    if let Some((_, subdir)) = SAVE_DIR_OVERRIDES.iter().find(|(s, _)| *s == slug) {
        let mut refined: Vec<PathBuf> = Vec::new();
        for hit in hits {
            let candidate = hit.join(subdir);
            if candidate.is_dir() && !refined.contains(&candidate) {
                refined.push(candidate);
            }
        }
        return refined;
    }

    let mut refined: Vec<PathBuf> = Vec::new();
    for hit in hits {
        if segment_matches_save_pattern(&hit) {
            if !refined.contains(&hit) {
                refined.push(hit);
            }
            continue;
        }
        for candidate in find_save_subdirs(&hit) {
            if !refined.contains(&candidate) {
                refined.push(candidate);
            }
        }
    }
    refined
}

/// True iff the path's last segment matches one of [`SAVE_PATTERNS`]
/// case-insensitively. Exact-on-segment, not substring contains.
fn segment_matches_save_pattern(path: &std::path::Path) -> bool {
    path.file_name()
        .and_then(|s| s.to_str())
        .map(name_matches_save_pattern)
        .unwrap_or(false)
}

fn name_matches_save_pattern(name: &str) -> bool {
    let lower = name.to_lowercase();
    SAVE_PATTERNS.iter().any(|p| lower == *p)
}

/// Final pipeline step: stamp every `manual_paths` entry from `CliState`
/// onto the detection map so the user's chosen folder wins over every
/// heuristic.
///
/// Three cases per `(slug, path)`:
///
/// * Slug already in `by_slug` (heuristic found something): replace
///   `found_paths` with `[path]`, lift confidence to `High`, tag the row
///   `ManualOverride`. We keep `steam_app_id`/`install_dir` from the
///   existing entry so the UI still shows the Steam hint when available.
/// * Slug not in `by_slug` but present in the catalog: synthesise a fresh
///   row from the catalog's display name. Covers the "user added a game
///   the heuristic would never find" case.
/// * Slug not in the catalog: log WARN and leave the override on disk. The
///   override stays in `state.json` so the user can clean it up; we don't
///   silently drop it because the catalog can grow back into the slug
///   (Ludusavi refreshes weekly) and we don't want the user's manual work
///   discarded between sessions.
fn apply_manual_overrides(
    manual_paths: &HashMap<String, PathBuf>,
    by_slug: &mut HashMap<String, DetectedGame>,
) {
    if manual_paths.is_empty() {
        return;
    }
    let catalog = ludusavi::catalog();
    let mut applied = 0usize;
    let mut orphaned = 0usize;
    for (slug, path) in manual_paths {
        if let Some(existing) = by_slug.get_mut(slug) {
            existing.found_paths = vec![path.clone()];
            existing.confidence = Confidence::High;
            existing.source = DetectionSource::ManualOverride;
            applied += 1;
            continue;
        }
        if let Some(entry) = catalog.iter().find(|e| &e.slug == slug) {
            by_slug.insert(
                slug.clone(),
                DetectedGame {
                    slug: entry.slug.clone(),
                    display_name: entry.display_name.clone(),
                    found_paths: vec![path.clone()],
                    confidence: Confidence::High,
                    source: DetectionSource::ManualOverride,
                    steam_app_id: entry.steam_app_id,
                    install_dir: None,
                },
            );
            applied += 1;
            continue;
        }
        tracing::warn!(
            slug = %slug,
            path = %path.display(),
            "manual_paths entry references a slug missing from the catalog; \
             keeping the override on disk so a future catalog refresh can pick it up"
        );
        orphaned += 1;
    }
    if applied > 0 || orphaned > 0 {
        tracing::info!(
            applied,
            orphaned,
            "manual_paths overrides applied"
        );
    }
}

/// One leg of a [`DetectionTrace`]: the result of a single pipeline step
/// for the slug being diagnosed. `kind` names the step
/// (`"manual_override"`, `"steam_appid"`, `"name_fallback"`,
/// `"filesystem"`, `"proton_prefix"`, `"refine"`).
///
/// `template` is the input the step worked from (a Ludusavi save-path
/// template, a Steam appid, a slugified name, …). `expanded` is what the
/// step produced before any existence/filtering check; `kept` are the
/// outputs that survived; `dropped` records the rejects with an
/// explanation per path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceStep {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expanded: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kept: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dropped: Vec<DroppedPath>,
}

/// A candidate path the pipeline rejected, with a human-readable reason.
/// Surfaced in the diagnostics UI so the user can tell *why* their game
/// didn't show up — "path doesn't exist", "expand_path returned nothing",
/// "slug not in catalog".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DroppedPath {
    pub path: String,
    pub reason: String,
}

/// Replayable trace of the detection pipeline for a single slug, used by
/// the hidden `/diagnostics` route in the desktop UI. Mirrors the steps in
/// [`detect_all`] but writes each step's input/output into [`TraceStep`]
/// records instead of merging them into a global [`DetectionReport`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionTrace {
    pub slug: String,
    pub attempts: Vec<TraceStep>,
}

/// Reproduce the detection pipeline for a single slug, recording every
/// step into a [`DetectionTrace`] instead of building a report. Used by
/// the desktop app's hidden diagnostics panel — the answer to "why
/// doesn't this game show up in my library?" is now mechanical.
///
/// The real [`detect_all`] is untouched: this is a parallel implementation
/// that hits the same primitives ([`expand_path`],
/// [`expand_path_in_prefix`], [`refine_save_dir`]) but writes traces.
/// Drift between the two will surface as silent disagreements — the
/// integration tests in `tests/detection_integration.rs` (P-DET-7) are the
/// guard against that.
pub async fn diagnose(slug: &str, os: Os, state: &CliState) -> DetectionTrace {
    let mut attempts: Vec<TraceStep> = Vec::new();

    // ---- Step 1: manual_override ------------------------------------
    // Recorded first because a manual path beats every heuristic — if
    // there's an override, the rest of the trace is informative only.
    let mut manual_step = TraceStep {
        kind: "manual_override".into(),
        template: None,
        expanded: Vec::new(),
        kept: Vec::new(),
        dropped: Vec::new(),
    };
    if let Some(p) = state.manual_paths.get(slug) {
        manual_step.template = Some("CliState::manual_paths".into());
        manual_step.expanded.push(p.display().to_string());
        if p.is_dir() {
            manual_step.kept.push(p.display().to_string());
        } else {
            manual_step.dropped.push(DroppedPath {
                path: p.display().to_string(),
                reason: "manual override path doesn't exist or isn't a directory".into(),
            });
        }
    }
    attempts.push(manual_step);

    // ---- Step 2: steam_appid ----------------------------------------
    // A slug not in the catalog short-circuits the rest of the trace —
    // the remaining steps can't expand templates we don't have.
    let catalog = ludusavi::catalog();
    let Some(entry) = catalog.iter().find(|e| e.slug == slug) else {
        attempts.push(TraceStep {
            kind: "steam_appid".into(),
            template: None,
            expanded: Vec::new(),
            kept: Vec::new(),
            dropped: vec![DroppedPath {
                path: slug.into(),
                reason: "slug not in catalog".into(),
            }],
        });
        return DetectionTrace {
            slug: slug.into(),
            attempts,
        };
    };

    let steam_apps = steam::list_installed_steam_games(os).unwrap_or_default();
    let mut steam_step = TraceStep {
        kind: "steam_appid".into(),
        template: entry.steam_app_id.map(|id| format!("steam_app_id={id}")),
        expanded: Vec::new(),
        kept: Vec::new(),
        dropped: Vec::new(),
    };
    let mut steam_appid_matched = false;
    if let Some(appid) = entry.steam_app_id {
        steam_step.expanded.push(format!("appid={appid}"));
        if let Some(app) = steam_apps.iter().find(|a| a.app_id == appid) {
            steam_step.kept.push(app.install_dir.display().to_string());
            steam_appid_matched = true;
        } else {
            steam_step.dropped.push(DroppedPath {
                path: format!("appid={appid}"),
                reason: "Steam library doesn't include this app".into(),
            });
        }
    } else {
        steam_step.dropped.push(DroppedPath {
            path: slug.into(),
            reason: "catalog entry has no steam_app_id".into(),
        });
    }
    attempts.push(steam_step);

    // ---- Step 3: name_fallback --------------------------------------
    // Only meaningful when the appid path didn't already link the slug.
    let mut fallback_step = TraceStep {
        kind: "name_fallback".into(),
        template: None,
        expanded: Vec::new(),
        kept: Vec::new(),
        dropped: Vec::new(),
    };
    if !steam_appid_matched && !steam_apps.is_empty() {
        for app in &steam_apps {
            let slugified = ludusavi::slugify(&app.name);
            fallback_step.expanded.push(slugified.clone());
            if slugified == slug {
                fallback_step.template = Some(format!("slugify({:?})", app.name));
                fallback_step.kept.push(app.install_dir.display().to_string());
            }
        }
        if fallback_step.kept.is_empty() {
            fallback_step.dropped.push(DroppedPath {
                path: slug.into(),
                reason: "no Steam app name slugifies to this slug".into(),
            });
        }
    }
    attempts.push(fallback_step);

    // ---- Step 4: filesystem -----------------------------------------
    // One step per save-path template that applies to the current OS.
    // Collects raw hits to feed into the refinement step below.
    let templates = paths_for_os(entry, os);
    let mut raw_hits: Vec<PathBuf> = Vec::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();
    for tmpl in &templates {
        let mut step = TraceStep {
            kind: "filesystem".into(),
            template: Some(tmpl.clone()),
            expanded: Vec::new(),
            kept: Vec::new(),
            dropped: Vec::new(),
        };
        let candidates = expand_path(tmpl, os);
        if candidates.is_empty() {
            step.dropped.push(DroppedPath {
                path: tmpl.clone(),
                reason: "expand_path produced no candidates (unknown placeholder or unset env)"
                    .into(),
            });
        }
        for c in candidates {
            step.expanded.push(c.display().to_string());
            if c.exists() {
                step.kept.push(c.display().to_string());
                if seen.insert(c.clone()) {
                    raw_hits.push(c);
                }
            } else {
                step.dropped.push(DroppedPath {
                    path: c.display().to_string(),
                    reason: "path doesn't exist on disk".into(),
                });
            }
        }
        attempts.push(step);
    }

    // ---- Step 5: proton_prefix (Linux only) -------------------------
    // For each compatdata prefix whose appid matches the catalog entry,
    // try every Windows save-path template against the prefix root.
    if os == Os::Linux {
        if let Some(appid) = entry.steam_app_id {
            for prefix in steam::list_proton_prefixes(os) {
                if prefix.app_id != appid {
                    continue;
                }
                for tmpl in entry.paths.windows.iter().map(|p| &p.path) {
                    let mut step = TraceStep {
                        kind: "proton_prefix".into(),
                        template: Some(format!(
                            "{} (under {})",
                            tmpl,
                            prefix.prefix_root.display()
                        )),
                        expanded: Vec::new(),
                        kept: Vec::new(),
                        dropped: Vec::new(),
                    };
                    let candidates = expand_path_in_prefix(tmpl, &prefix.prefix_root);
                    if candidates.is_empty() {
                        step.dropped.push(DroppedPath {
                            path: tmpl.clone(),
                            reason: "expand_path_in_prefix produced no candidates \
                                     (placeholder doesn't map to a Wine path)"
                                .into(),
                        });
                    }
                    for c in candidates {
                        step.expanded.push(c.display().to_string());
                        if c.exists() {
                            step.kept.push(c.display().to_string());
                            if seen.insert(c.clone()) {
                                raw_hits.push(c);
                            }
                        } else {
                            step.dropped.push(DroppedPath {
                                path: c.display().to_string(),
                                reason: "path doesn't exist under the Proton prefix".into(),
                            });
                        }
                    }
                    attempts.push(step);
                }
            }
        }
    }

    // ---- Step 6: refine ---------------------------------------------
    // Whatever survived the filesystem + proton passes goes through the
    // save-dir refinement. Any hit that lost its place (root replaced
    // by its subdir, or dropped because no save-named subdir exists) is
    // recorded so the user can see *why* a path that looked promising
    // didn't make the final cut.
    let pre_refine: Vec<PathBuf> = raw_hits.clone();
    let refined = refine_save_dir(slug, raw_hits);
    let refined_set: HashSet<PathBuf> = refined.iter().cloned().collect();
    let mut refine_step = TraceStep {
        kind: "refine".into(),
        template: None,
        expanded: pre_refine.iter().map(|p| p.display().to_string()).collect(),
        kept: refined.iter().map(|p| p.display().to_string()).collect(),
        dropped: Vec::new(),
    };
    for p in &pre_refine {
        if !refined_set.contains(p) {
            refine_step.dropped.push(DroppedPath {
                path: p.display().to_string(),
                reason: "root replaced by its save subdir, or dropped because no \
                         save-named subdir exists under it"
                    .into(),
            });
        }
    }
    attempts.push(refine_step);

    DetectionTrace {
        slug: slug.into(),
        attempts,
    }
}

/// List immediate subdirectories of `path` whose name matches a save
/// pattern. Returns empty if the path can't be read.
fn find_save_subdirs(path: &std::path::Path) -> Vec<PathBuf> {
    let Ok(read) = std::fs::read_dir(path) else {
        return Vec::new();
    };
    let mut matches: Vec<PathBuf> = Vec::new();
    for entry in read.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let Some(name_str) = name.to_str() else {
            continue;
        };
        if name_matches_save_pattern(name_str) {
            matches.push(entry.path());
        }
    }
    matches.sort();
    matches
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

    /// A hit whose last path segment is already a save-named folder is
    /// passed through untouched — the heuristic doesn't need to descend.
    #[test]
    fn refine_save_dir_keeps_path_with_save_in_name() {
        let p = PathBuf::from("/home/x/.config/StardewValley/Saves");
        let refined = refine_save_dir("stardew-valley", vec![p.clone()]);
        assert_eq!(refined, vec![p]);
    }

    /// Paradox layout: catalog points at a game root that contains a
    /// `save games/` subdir among other folders. The general heuristic
    /// must collapse the hit to the subdir without needing an override.
    #[test]
    fn refine_save_dir_finds_subdir_save_games() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("Paradox Interactive").join("Stellaris");
        std::fs::create_dir_all(root.join("save games")).unwrap();
        std::fs::create_dir_all(root.join("mod")).unwrap();

        let refined = refine_save_dir("stellaris", vec![root.clone()]);
        assert_eq!(refined, vec![root.join("save games")]);
    }

    /// Same as above with a `Saves/` subdir (the most common shape).
    #[test]
    fn refine_save_dir_finds_subdir_saves() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("SomeGame");
        std::fs::create_dir_all(root.join("Saves")).unwrap();
        std::fs::create_dir_all(root.join("Config")).unwrap();

        let refined = refine_save_dir("some-game", vec![root.clone()]);
        assert_eq!(refined, vec![root.join("Saves")]);
    }

    /// Root exists but contains no save-named subdir — the hit is dropped
    /// so the UI surfaces the amber "pick folder" alert instead of
    /// tracking the root (and its mods, config, telemetry…) by mistake.
    #[test]
    fn refine_save_dir_drops_when_no_save_subdir() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("GameRoot");
        std::fs::create_dir_all(root.join("mod")).unwrap();
        std::fs::create_dir_all(root.join("config")).unwrap();

        let refined = refine_save_dir("game-root", vec![root]);
        assert!(refined.is_empty());
    }

    /// Two save-named subdirs (rare, but happens when a game splits cloud
    /// vs local saves). Surface both and let the UI / picker handle it.
    #[test]
    fn refine_save_dir_returns_multiple_when_ambiguous() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("AmbiguousGame");
        std::fs::create_dir_all(root.join("saves")).unwrap();
        std::fs::create_dir_all(root.join("save games")).unwrap();

        let refined = refine_save_dir("ambiguous-game", vec![root.clone()]);
        // find_save_subdirs sorts, so the order is deterministic.
        assert_eq!(
            refined,
            vec![root.join("save games"), root.join("saves")]
        );
    }

    /// Build a synthetic catalog entry for the slug-fallback tests. The
    /// path templates are empty because the fallback path never touches
    /// them — it only matches by slug and copies display_name.
    fn synthetic_entry(slug: &str, display_name: &str, app_id: Option<u64>) -> LudusaviEntry {
        LudusaviEntry {
            slug: slug.into(),
            display_name: display_name.into(),
            steam_app_id: app_id,
            paths: hoard_manifest::ludusavi::LudusaviPaths::default(),
        }
    }

    fn synthetic_steam_app(app_id: u64, name: &str, install: &str) -> SteamApp {
        SteamApp {
            app_id,
            name: name.into(),
            install_dir: PathBuf::from(install),
        }
    }

    /// Happy path: a catalog entry without `steam_app_id` whose slug matches
    /// the slugified Steam display name surfaces in the report with
    /// `Confidence::Low` and `source=SteamLibrary`.
    #[test]
    fn steam_to_catalog_fallback_matches_by_slugified_name() {
        let catalog = vec![synthetic_entry("test-game", "Test Game", None)];
        let steam_apps = vec![synthetic_steam_app(999, "Test Game", "/steam/Test Game")];
        let mut by_slug: HashMap<String, DetectedGame> = HashMap::new();

        apply_steam_name_fallback(&catalog, &steam_apps, &mut by_slug);

        let g = by_slug
            .get("test-game")
            .expect("fallback should insert test-game");
        assert_eq!(g.source, DetectionSource::SteamLibrary);
        assert_eq!(g.confidence, Confidence::Low);
        assert_eq!(g.steam_app_id, Some(999));
        assert_eq!(g.install_dir, Some(PathBuf::from("/steam/Test Game")));
        assert!(g.found_paths.is_empty());
        assert_eq!(g.display_name, "Test Game");
    }

    /// If the appid cross-reference already linked this Steam app, the
    /// fallback must skip it — never demote a High/Medium entry to Low.
    #[test]
    fn steam_to_catalog_fallback_skips_when_appid_already_matched() {
        let catalog = vec![synthetic_entry("test-game", "Test Game", Some(999))];
        let steam_apps = vec![synthetic_steam_app(999, "Test Game", "/steam/Test Game")];
        let mut by_slug: HashMap<String, DetectedGame> = HashMap::new();
        // Simulate the appid pass having already inserted the entry.
        by_slug.insert(
            "test-game".into(),
            DetectedGame {
                slug: "test-game".into(),
                display_name: "Test Game".into(),
                found_paths: Vec::new(),
                confidence: Confidence::Medium,
                source: DetectionSource::SteamLibrary,
                steam_app_id: Some(999),
                install_dir: Some(PathBuf::from("/steam/Test Game")),
            },
        );

        apply_steam_name_fallback(&catalog, &steam_apps, &mut by_slug);

        let g = &by_slug["test-game"];
        // Confidence stays Medium; the fallback did not overwrite.
        assert_eq!(g.confidence, Confidence::Medium);
        assert_eq!(by_slug.len(), 1);
    }

    /// Steam apps whose slugified name is not in the catalog produce no
    /// noise — the dedupe map stays empty.
    #[test]
    fn steam_to_catalog_fallback_skips_unknown_titles() {
        let catalog = vec![synthetic_entry("known-game", "Known Game", None)];
        let steam_apps = vec![synthetic_steam_app(
            42,
            "Completely Unrelated Title",
            "/steam/unknown",
        )];
        let mut by_slug: HashMap<String, DetectedGame> = HashMap::new();

        apply_steam_name_fallback(&catalog, &steam_apps, &mut by_slug);

        assert!(by_slug.is_empty());
    }

    /// Same Stellaris regression test as before, adapted to the new
    /// function name. Covers the "root exists but no save-named subdir"
    /// regression — historically Hoard would back up the entire root.
    #[test]
    fn refine_save_dir_drops_paradox_root_without_save_games() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("Paradox Interactive").join("Stellaris");
        // Root exists (config + mods present) but no `save games/` yet —
        // user installed Stellaris but hasn't created a campaign.
        std::fs::create_dir_all(root.join("mod")).unwrap();

        let refined = refine_save_dir("stellaris", vec![root]);
        assert!(refined.is_empty());
    }

    // The detect_all test below mutates process-wide env (HOME / XDG_*) so
    // we serialise it against any other test in this crate that does the
    // same via the crate-wide test_lock. Held across the (single) tokio
    // runtime block — fine because detect_all takes seconds, not long
    // enough to starve other tests.
    fn with_isolated_home<F: FnOnce(&std::path::Path)>(f: F) {
        let _guard = crate::test_lock::ENV
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let tmp = tempfile::tempdir().unwrap();
        let prev = [
            ("HOME", std::env::var_os("HOME")),
            ("XDG_DATA_HOME", std::env::var_os("XDG_DATA_HOME")),
            ("XDG_CONFIG_HOME", std::env::var_os("XDG_CONFIG_HOME")),
            ("XDG_STATE_HOME", std::env::var_os("XDG_STATE_HOME")),
            ("XDG_CACHE_HOME", std::env::var_os("XDG_CACHE_HOME")),
        ];
        std::env::set_var("HOME", tmp.path());
        // Pin the XDG dirs under the tempdir too so the native-Linux
        // filesystem heuristic can't accidentally hit real saves.
        std::env::set_var("XDG_DATA_HOME", tmp.path().join("xdg-data"));
        std::env::set_var("XDG_CONFIG_HOME", tmp.path().join("xdg-config"));
        std::env::set_var("XDG_STATE_HOME", tmp.path().join("xdg-state"));
        std::env::set_var("XDG_CACHE_HOME", tmp.path().join("xdg-cache"));

        f(tmp.path());

        for (name, value) in prev {
            match value {
                Some(v) => std::env::set_var(name, v),
                None => std::env::remove_var(name),
            }
        }
    }

    /// Manual override replaces a heuristic hit: the existing row's
    /// `found_paths` is swapped for `[override]`, source becomes
    /// `ManualOverride`, confidence is lifted to `High`, and the Steam
    /// hint (`install_dir`, `steam_app_id`) is preserved.
    #[test]
    fn manual_override_replaces_heuristic_hit() {
        let mut by_slug: HashMap<String, DetectedGame> = HashMap::new();
        by_slug.insert(
            "stellaris".into(),
            DetectedGame {
                slug: "stellaris".into(),
                display_name: "Stellaris".into(),
                found_paths: vec![PathBuf::from("/wrong/path")],
                confidence: Confidence::Medium,
                source: DetectionSource::FilesystemHeuristic,
                steam_app_id: Some(281990),
                install_dir: Some(PathBuf::from("/steam/stellaris")),
            },
        );
        let mut overrides = HashMap::new();
        overrides.insert(
            "stellaris".to_string(),
            PathBuf::from("/home/x/Stellaris/save games"),
        );

        apply_manual_overrides(&overrides, &mut by_slug);

        let g = &by_slug["stellaris"];
        assert_eq!(g.source, DetectionSource::ManualOverride);
        assert_eq!(g.confidence, Confidence::High);
        assert_eq!(
            g.found_paths,
            vec![PathBuf::from("/home/x/Stellaris/save games")]
        );
        assert_eq!(g.steam_app_id, Some(281990));
        assert_eq!(g.install_dir, Some(PathBuf::from("/steam/stellaris")));
    }

    /// Manual override for a slug the heuristic never produced: the catalog
    /// is consulted to fill display_name, the row is synthesised, and
    /// `found_paths` is exactly the override.
    #[test]
    fn manual_override_creates_entry_from_catalog_when_absent() {
        let mut by_slug: HashMap<String, DetectedGame> = HashMap::new();
        let mut overrides = HashMap::new();
        // Stardew Valley is in the embedded catalog; the heuristic produced
        // nothing here (empty `by_slug`).
        overrides.insert(
            "stardew-valley".to_string(),
            PathBuf::from("/custom/stardew"),
        );

        apply_manual_overrides(&overrides, &mut by_slug);

        let g = by_slug
            .get("stardew-valley")
            .expect("override should synthesise the slug from the catalog");
        assert_eq!(g.source, DetectionSource::ManualOverride);
        assert_eq!(g.confidence, Confidence::High);
        assert_eq!(g.found_paths, vec![PathBuf::from("/custom/stardew")]);
        assert!(g.install_dir.is_none());
        assert_eq!(g.display_name, "Stardew Valley");
    }

    /// Orphaned override (slug not in the catalog): the override stays on
    /// disk; the report carries no row for it. We don't drop the entry from
    /// `manual_paths` because the catalog refreshes weekly and a future
    /// refresh might add the slug back.
    #[test]
    fn manual_override_orphaned_slug_does_not_panic_or_insert() {
        let mut by_slug: HashMap<String, DetectedGame> = HashMap::new();
        let mut overrides = HashMap::new();
        overrides.insert(
            "definitely-not-a-real-slug-zzz".to_string(),
            PathBuf::from("/nowhere"),
        );

        apply_manual_overrides(&overrides, &mut by_slug);

        assert!(by_slug.is_empty());
    }

    /// End-to-end through `detect_all`: a `CliState` with a manual_path for
    /// a slug the filesystem can't find still surfaces the slug in the
    /// report with `source=ManualOverride`. Uses the same isolated-HOME
    /// harness as the other integration test in this module.
    #[test]
    fn manual_override_surfaces_slug_filesystem_cannot_find() {
        with_isolated_home(|home| {
            // No Steam install, no on-disk save folder for the slug — the
            // heuristic produces nothing. Only the manual_path entry can
            // make the slug appear.
            let override_dir = home.join("custom-stardew");
            std::fs::create_dir_all(&override_dir).unwrap();

            let mut state = CliState::default();
            state.set_manual_path("stardew-valley", override_dir.clone());

            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let report = rt
                .block_on(detect_all(Os::Linux, &state, |_, _| {}))
                .unwrap();

            let stardew = report
                .games
                .iter()
                .find(|g| g.slug == "stardew-valley")
                .expect("manual_paths override should surface stardew-valley");
            assert_eq!(stardew.source, DetectionSource::ManualOverride);
            assert_eq!(stardew.confidence, Confidence::High);
            assert_eq!(stardew.found_paths, vec![override_dir]);
        });
    }

    /// Stardew Valley is Windows-only in the Ludusavi catalog's
    /// `<winAppData>/StardewValley/Saves` sense, but on Linux it's
    /// commonly played via Proton. When a compatdata prefix for its
    /// appid (413150) exists and contains the AppData/Roaming save
    /// folder, detect_all() should report stardew-valley with
    /// source=Both (steam library scan + proton prefix expand) and
    /// found_paths pointing at the prefix.
    #[test]
    fn proton_prefix_expand_surfaces_stardew_save_on_linux() {
        with_isolated_home(|home| {
            // Minimal Steam install: one library, one appmanifest, the
            // compatdata prefix with the save folder.
            let steam = home.join(".steam/steam");
            let steamapps = steam.join("steamapps");
            std::fs::create_dir_all(&steamapps).unwrap();
            std::fs::write(
                steamapps.join("libraryfolders.vdf"),
                format!(
                    "\"libraryfolders\"\n{{\n  \"0\" {{ \"path\" \"{}\" }}\n}}\n",
                    steam.display()
                ),
            )
            .unwrap();
            std::fs::write(
                steamapps.join("appmanifest_413150.acf"),
                "\"AppState\"\n{\n  \"appid\" \"413150\"\n  \"name\" \"Stardew Valley\"\n  \"installdir\" \"Stardew Valley\"\n}\n",
            )
            .unwrap();
            let save_dir = steamapps
                .join("compatdata/413150/pfx/drive_c/users/steamuser/AppData/Roaming/StardewValley/Saves");
            std::fs::create_dir_all(&save_dir).unwrap();

            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let state = CliState::default();
            let report = rt
                .block_on(detect_all(Os::Linux, &state, |_, _| {}))
                .unwrap();

            let stardew = report
                .games
                .iter()
                .find(|g| g.slug == "stardew-valley")
                .expect("stardew-valley should appear in the report");
            assert_eq!(stardew.source, DetectionSource::Both);
            assert_eq!(stardew.confidence, Confidence::High);
            assert_eq!(stardew.steam_app_id, Some(413150));
            assert!(
                stardew.found_paths.iter().any(|p| p == &save_dir),
                "found_paths should contain the prefix save dir; got {:?}",
                stardew.found_paths
            );
        });
    }

    /// `diagnose` for a slug that isn't in the catalog short-circuits
    /// after recording the manual_override (empty, no user override) and
    /// steam_appid (dropped: "slug not in catalog") steps. The remaining
    /// pipeline steps don't run because there's no catalog entry to
    /// expand templates against.
    #[test]
    fn diagnose_unknown_slug_records_manual_and_steam_steps() {
        let state = CliState::default();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let trace = rt.block_on(diagnose(
            "definitely-not-a-real-slug-zzz",
            Os::Linux,
            &state,
        ));

        assert_eq!(trace.slug, "definitely-not-a-real-slug-zzz");
        // Exactly the two steps the short-circuit emits.
        assert_eq!(trace.attempts.len(), 2);

        let manual = &trace.attempts[0];
        assert_eq!(manual.kind, "manual_override");
        assert!(manual.template.is_none());
        assert!(manual.expanded.is_empty());
        assert!(manual.kept.is_empty());
        assert!(manual.dropped.is_empty());

        let steam = &trace.attempts[1];
        assert_eq!(steam.kind, "steam_appid");
        assert!(!steam.dropped.is_empty());
        assert!(
            steam
                .dropped
                .iter()
                .any(|d| d.reason.contains("slug not in catalog")),
            "expected a dropped entry with 'slug not in catalog'; got {:?}",
            steam.dropped
        );
    }

    /// `diagnose` on Linux for a slug whose Steam appid has a synthetic
    /// compatdata prefix records a `proton_prefix` step whose `kept`
    /// contains the save path under the prefix.
    #[test]
    fn diagnose_records_proton_prefix_step_for_stardew() {
        with_isolated_home(|home| {
            let steam = home.join(".steam/steam");
            let steamapps = steam.join("steamapps");
            std::fs::create_dir_all(&steamapps).unwrap();
            std::fs::write(
                steamapps.join("libraryfolders.vdf"),
                format!(
                    "\"libraryfolders\"\n{{\n  \"0\" {{ \"path\" \"{}\" }}\n}}\n",
                    steam.display()
                ),
            )
            .unwrap();
            std::fs::write(
                steamapps.join("appmanifest_413150.acf"),
                "\"AppState\"\n{\n  \"appid\" \"413150\"\n  \"name\" \"Stardew Valley\"\n  \"installdir\" \"Stardew Valley\"\n}\n",
            )
            .unwrap();
            let save_dir = steamapps
                .join("compatdata/413150/pfx/drive_c/users/steamuser/AppData/Roaming/StardewValley/Saves");
            std::fs::create_dir_all(&save_dir).unwrap();

            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let state = CliState::default();
            let trace = rt.block_on(diagnose("stardew-valley", Os::Linux, &state));

            assert_eq!(trace.slug, "stardew-valley");
            let proton_steps: Vec<&TraceStep> = trace
                .attempts
                .iter()
                .filter(|s| s.kind == "proton_prefix")
                .collect();
            assert!(
                !proton_steps.is_empty(),
                "diagnose should record at least one proton_prefix step; got kinds {:?}",
                trace.attempts.iter().map(|s| &s.kind).collect::<Vec<_>>()
            );
            let save_str = save_dir.display().to_string();
            assert!(
                proton_steps
                    .iter()
                    .any(|s| s.kept.iter().any(|k| k == &save_str)),
                "expected a proton_prefix step whose kept contains {save_str}; got {proton_steps:?}"
            );
        });
    }
}
