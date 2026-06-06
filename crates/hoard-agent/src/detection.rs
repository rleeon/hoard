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
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use anyhow::Result;
use hoard_manifest::ludusavi::{self, LudusaviEntry};
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;

use crate::correlation::{self, CorrelationStore};
use crate::launchers::{self, LauncherApp};
use crate::manifest::Os;
use crate::pathexpand::{
    expand_path, expand_path_in_prefix, expand_path_in_prefix_as_user, expand_registry_path,
};
use crate::roots;
use crate::scoring;
use crate::state::CliState;
use crate::steam::{self, SteamApp};
use crate::wine_prefixes::{self, PrefixKind};

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
    // DETECCIÓN (recall, fase 1): `savedata`/`savefiles` son nombres de
    // carpeta muy comunes (Unity, muchos indies, ports de consola) que el
    // set original no reconocía. Match exacto-por-segmento, así que el
    // riesgo de falso positivo sigue siendo bajo (no matchea "save settings").
    "savedata",
    "save data",
    "save_data",
    "savefile",
    "savefiles",
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
    detect_all_inner(os, state, false, progress).await
}

/// Deep variant of [`detect_all`]: same pipeline plus the expensive passes the
/// periodic scan skips — arbitrary Wine prefixes (Heroic/CrossOver/Flatpak/
/// mounted media), Flatpak/Snap/EmuDeck save roots, deeper directory walks and
/// a relaxed precision gate. User-triggered only (the Library "deep scan"
/// tile), never on the automatic tick.
pub async fn detect_all_deep<F>(os: Os, state: &CliState, progress: F) -> Result<DetectionReport>
where
    F: Fn(usize, usize) + Send + Sync + 'static,
{
    detect_all_inner(os, state, true, progress).await
}

async fn detect_all_inner<F>(
    os: Os,
    state: &CliState,
    deep: bool,
    progress: F,
) -> Result<DetectionReport>
where
    F: Fn(usize, usize) + Send + Sync + 'static,
{
    let started = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    // Correlation store (ADR 0020, fase 3): the agent records which game
    // process was alive when a watched save was rewritten. Loaded best-effort
    // (empty if absent / unreadable) and fed into both the per-slug
    // aggressive walk and the phase-4 catalog-free pass, where it adds the
    // +0.50 process↔write bonus and unlocks `High` for corroborated dirs.
    let corr_store = CorrelationStore::default_path()
        .ok()
        .map(|p| CorrelationStore::load(&p))
        .unwrap_or_default();

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

    // Non-Steam launchers (1.5.2): Epic Games / GOG Galaxy / Microsoft Store.
    // Same shape as the Steam name fallback above — slugify the display
    // name, look up exact, fall back to fuzzy. Each function is a no-op on
    // non-Windows (or when its launcher data dir is absent), so calling
    // unconditionally costs nothing on hosts without the launcher.
    let epic_apps = launchers::list_installed_epic_games(os);
    let gog_apps = launchers::list_installed_gog_games(os);
    let ms_apps = launchers::list_installed_msstore_games(os);
    apply_launcher_name_fallback(catalog, &epic_apps, "epic", &mut by_slug);
    apply_launcher_name_fallback(catalog, &gog_apps, "gog", &mut by_slug);
    apply_launcher_name_fallback(catalog, &ms_apps, "msstore", &mut by_slug);

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

    // Registry expand (1.5.2): catalog entries with `registry` keys point at
    // HKEY_* paths whose value holds the save directory. On Windows we read
    // each registry value (via `pathexpand::expand_registry_path`) and treat
    // the result as a filesystem hit, merging it through the same path as
    // template expansion. On non-Windows the call returns an empty vec, so
    // this block is a no-op and never touches the Linux integration tests.
    let mut registry_hits = 0usize;
    for entry in catalog {
        if entry.registry.is_empty() {
            continue;
        }
        let mut hits: Vec<PathBuf> = Vec::new();
        let mut seen: HashSet<PathBuf> = HashSet::new();
        for reg in &entry.registry {
            for candidate in expand_registry_path(reg) {
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
        registry_hits += 1;
        tracing::debug!(
            slug = %entry.slug,
            count = hits.len(),
            "registry expand produced filesystem hits"
        );
        let refined = refine_save_dir(&entry.slug, hits);
        if refined.is_empty() {
            continue;
        }
        merge_fs_hit(
            &mut by_slug,
            entry.slug.clone(),
            entry.display_name.clone(),
            refined,
        );
    }
    if registry_hits > 0 {
        tracing::info!(slugs = registry_hits, "registry expand merged hits");
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

    // Whole-prefix Windows cross-reference: for prefixes NOT tied to a single
    // catalog game, expand EVERY catalog entry's Windows templates against each
    // real Windows user home inside the prefix — the native filesystem
    // heuristic, pointed at the prefix's `drive_c/`. Two prefix sources qualify:
    //   * Generic prefixes (plain `wine` / PlayOnLinux / `.desktop`), which
    //     aren't owned by any launcher.
    //   * Proton prefixes whose appid has NO catalog match — i.e. "non-Steam
    //     game" shortcuts the user added to Steam and runs through Proton. The
    //     appid-keyed Proton block above can't help those (no entry to expand),
    //     so a save belonging to a real catalog game went unseen.
    // One blocking task per (prefix, user) bounds the cost and keeps it off the
    // async runtime. The deep scan additionally discovers prefixes in arbitrary
    // locations (Heroic/CrossOver/Flatpak/mounted media).
    if os == Os::Linux {
        let all_prefixes = if deep {
            wine_prefixes::list_wine_prefixes_deep(os)
        } else {
            wine_prefixes::list_wine_prefixes(os)
        };
        let mut cross_ref_roots: Vec<PathBuf> = Vec::new();
        for p in all_prefixes {
            let keep = match p.kind {
                PrefixKind::Generic => true,
                // Only Proton prefixes the catalog can't resolve by appid.
                PrefixKind::Proton => p
                    .identifier
                    .parse::<u64>()
                    .ok()
                    .and_then(ludusavi::find_by_steam_app_id)
                    .is_none(),
                PrefixKind::Lutris | PrefixKind::Bottles => false,
            };
            if keep {
                cross_ref_roots.push(p.prefix_root);
            }
        }
        let mut targets: Vec<(PathBuf, String)> = Vec::new();
        for root in &cross_ref_roots {
            for user in roots::prefix_windows_users(root) {
                targets.push((root.clone(), user));
            }
        }
        let mut tasks = Vec::new();
        for (root, user) in targets {
            let permit = semaphore.clone().acquire_owned().await?;
            tasks.push(tokio::task::spawn_blocking(move || {
                let _permit = permit;
                let mut found: Vec<(String, String, Vec<PathBuf>)> = Vec::new();
                for entry in ludusavi::catalog() {
                    if entry.paths.windows.is_empty() {
                        continue;
                    }
                    let mut hits: Vec<PathBuf> = Vec::new();
                    let mut seen: HashSet<PathBuf> = HashSet::new();
                    for tmpl in entry.paths.windows.iter().map(|p| &p.path) {
                        for candidate in expand_path_in_prefix_as_user(tmpl, &root, &user) {
                            if candidate.exists() && seen.insert(candidate.clone()) {
                                hits.push(candidate);
                            }
                        }
                    }
                    if !hits.is_empty() {
                        found.push((entry.slug.clone(), entry.display_name.clone(), hits));
                    }
                }
                found
            }));
        }
        let mut generic_hits = 0usize;
        for t in tasks {
            match t.await {
                Ok(found) => {
                    for (slug, display_name, hits) in found {
                        generic_hits += 1;
                        merge_fs_hit(&mut by_slug, slug, display_name, hits);
                    }
                }
                Err(e) => tracing::warn!(error = %e, "generic-prefix scan task panicked"),
            }
        }
        if generic_hits > 0 {
            tracing::info!(slugs = generic_hits, "generic Wine prefix saves merged");
        }
    }

    // Steam Cloud (ADR 0019): some titles write their only save to
    // `<root>/userdata/<storeUserId>/<appid>/remote/` and never touch the
    // filesystem locations the catalog lists. Cross-reference every installed
    // app's appid against the per-user `userdata` dirs and merge any existing
    // `remote/` folder as a filesystem hit. Reuses `merge_fs_hit`, so a slug
    // already seen via the Steam cross-reference is promoted to Both/High.
    // No-op when Steam isn't installed (empty user dirs).
    {
        let libraries = steam::detect_steam_libraries(os);
        let user_dirs = steam::steam_user_dirs(&libraries).unwrap_or_default();
        if !user_dirs.is_empty() {
            let mut cloud_hits = 0usize;
            for app in &steam_apps {
                let Some(entry) = hoard_manifest::ludusavi::find_by_steam_app_id(app.app_id) else {
                    continue;
                };
                let mut hits: Vec<PathBuf> = Vec::new();
                let mut seen: HashSet<PathBuf> = HashSet::new();
                for ud in &user_dirs {
                    let remote = ud.join(app.app_id.to_string()).join("remote");
                    if remote.is_dir() && seen.insert(remote.clone()) {
                        hits.push(remote);
                    }
                }
                if hits.is_empty() {
                    continue;
                }
                cloud_hits += 1;
                merge_fs_hit(
                    &mut by_slug,
                    entry.slug.clone(),
                    entry.display_name.clone(),
                    hits,
                );
            }
            if cloud_hits > 0 {
                tracing::info!(slugs = cloud_hits, "Steam Cloud remote dirs merged");
            }
        }
    }

    // Promote confidence wherever both signals fired.
    for game in by_slug.values_mut() {
        if matches!(game.source, DetectionSource::Both) {
            game.confidence = Confidence::High;
        }
    }

    // Aggressive walker (1.5.1, extended in 1.5.2): for every slug that
    // survived the main pipeline without any `found_paths`, walk the install
    // dir and the matching Wine/Proton prefix looking for save-like
    // subdirs. Gated by `found_paths.is_empty()` so it never costs anything
    // on the happy path; covers the long tail of indies / GOG titles / odd
    // layouts that Ludusavi doesn't list or whose templates miss the real
    // save dir.
    //
    // 1.5.2 changes:
    //   * `prefix_root_by_slug` unifies Proton (lookup via Steam appid →
    //     slug) and Lutris/Bottles (lookup via `slugify(identifier)`), so
    //     the walker now reaches Wine-managed prefixes outside Steam.
    //   * `install_dir` for non-Steam launchers (Epic / GOG / MS) is
    //     already attached to the row in the launcher cross-reference
    //     block above; the walker reads it from `g.install_dir`, no
    //     separate map needed.
    let prefix_root_by_slug = build_prefix_root_by_slug(os);
    let unresolved_slugs: Vec<String> = by_slug
        .iter()
        .filter(|(_, g)| g.found_paths.is_empty())
        .map(|(s, _)| s.clone())
        .collect();
    for slug in unresolved_slugs {
        let (install_dir, prefix_root, display_name) = {
            let g = &by_slug[&slug];
            let install_dir = g.install_dir.clone();
            let prefix_root = prefix_root_by_slug.get(&slug).cloned();
            (install_dir, prefix_root, g.display_name.clone())
        };
        if install_dir.is_none() && prefix_root.is_none() {
            continue;
        }
        let discoveries = aggressive_discover_with(
            &slug,
            &display_name,
            install_dir.as_deref(),
            prefix_root.as_deref(),
            AGGRESSIVE_WALK_TIMEOUT,
            AGGRESSIVE_WALK_MAX_DEPTH,
            &corr_store,
        );
        if discoveries.is_empty() {
            continue;
        }
        let max_conf = discoveries
            .iter()
            .map(|d| d.confidence)
            .max_by_key(|c| confidence_rank(*c))
            .unwrap_or(Confidence::Low);
        let hits: Vec<PathBuf> = discoveries.into_iter().map(|d| d.path).collect();
        merge_fs_hit(&mut by_slug, slug.clone(), display_name, hits);
        if let Some(entry) = by_slug.get_mut(&slug) {
            // `merge_fs_hit` over a pre-existing Steam entry stamps
            // `Both`/`High`. The walker's signal is structurally weaker
            // (heuristic dir-name match, not a catalog hit), so pin the
            // confidence to the walker's own grading instead.
            entry.confidence = max_conf;
        }
    }

    // Phase 4 (ADR 0020): catalog-free discovery + attribution. A single
    // pass over the broad user save roots — scored WITH the correlation
    // store — surfaces save folders that no catalog/Steam signal claimed
    // (GUID names, non-English names, indies Ludusavi doesn't list) and
    // attributes each to a game by the process that wrote it. Gated to
    // correlation-corroborated or strong-static candidates so it never mints
    // phantom games from weak name-only matches; runs once, not per-slug, so
    // the I/O stays bounded.
    {
        let known: HashSet<PathBuf> = by_slug
            .values()
            .flat_map(|g| g.found_paths.iter().cloned())
            .collect();
        let attributed = discover_unattributed_mode(os, &corr_store, &known, deep);
        let mut new_games = 0usize;
        let mut merged = 0usize;
        for a in attributed {
            match by_slug.get_mut(&a.slug) {
                Some(existing) => {
                    if !existing.found_paths.contains(&a.path) {
                        existing.found_paths.push(a.path);
                        if confidence_rank(a.confidence) > confidence_rank(existing.confidence) {
                            existing.confidence = a.confidence;
                        }
                        merged += 1;
                    }
                }
                None => {
                    by_slug.insert(
                        a.slug.clone(),
                        DetectedGame {
                            slug: a.slug,
                            display_name: a.display_name,
                            found_paths: vec![a.path],
                            confidence: a.confidence,
                            source: DetectionSource::FilesystemHeuristic,
                            steam_app_id: None,
                            install_dir: None,
                        },
                    );
                    new_games += 1;
                }
            }
        }
        if new_games > 0 || merged > 0 {
            tracing::info!(
                new_games,
                merged,
                "phase 4: catalog-free saves attributed via correlation"
            );
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
    let mut fuzzy_added = 0usize;
    for app in steam_apps {
        if matched_appids.contains(&app.app_id) {
            continue;
        }
        let slug = ludusavi::slugify(&app.name);
        let (entry, via_fuzzy) = match catalog_by_slug.get(slug.as_str()) {
            Some(entry) => (*entry, false),
            None => {
                // Last-resort: fuzzy match on the slugified display name
                // (Levenshtein normalised, threshold 0.15). Catches Steam
                // titles whose slug diverges slightly from the catalog
                // (typos, "Definitive Edition" suffixes, minor localisation).
                let Some(entry) = ludusavi::find_by_fuzzy_name(&app.name, FUZZY_NAME_THRESHOLD)
                else {
                    continue;
                };
                (entry, true)
            }
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
        if via_fuzzy {
            fuzzy_added += 1;
            tracing::info!(
                steam_name = %app.name,
                catalog_slug = %entry.slug,
                "Steam → catalog fuzzy fallback matched with Confidence::Low"
            );
        } else {
            added += 1;
        }
    }
    if added > 0 {
        tracing::info!(
            added,
            "Steam → catalog slug fallback added entries with Confidence::Low"
        );
    }
    if fuzzy_added > 0 {
        tracing::info!(
            fuzzy_added,
            "Steam → catalog fuzzy-name fallback added entries with Confidence::Low"
        );
    }
}

/// Threshold for `find_by_fuzzy_name` in [`apply_steam_name_fallback`].
/// 0.15 ≈ one edit per ~7 characters. Conservative enough that
/// "Civilization V" vs "Civilization VI" (≈ 0.07) wouldn't trip the
/// fuzzy match for an unrelated query; restrictive enough that
/// random typos still resolve.
const FUZZY_NAME_THRESHOLD: f32 = 0.15;

/// Cross-reference non-Steam launcher apps (Epic / GOG / Microsoft Store)
/// against the Ludusavi catalog. Mirrors [`apply_steam_name_fallback`]:
/// slugify the launcher's display name, look up exact, fall back to fuzzy.
///
/// Rows synthesised here use `DetectionSource::SteamLibrary` as a neutral
/// "we know this game is installed via a launcher" tag. Adding a new
/// variant per launcher (`LauncherEpic`, `LauncherGog`, …) was considered
/// and rejected: the UI consumes `source` as an opaque grouping signal,
/// not a per-launcher icon, and the install dir hint on the row already
/// records the launcher (it points at the launcher's install path).
///
/// Confidence is pinned to `Low` because the match is structurally weaker
/// than Steam's `appid`: two unrelated launcher games sharing a slugifiable
/// name would collide. Rows without any Ludusavi match are **not**
/// inserted — surfacing every random launcher app would surface launcher
/// tooling and non-game executables. The walker still benefits from the
/// install_dir attached to matched rows.
fn apply_launcher_name_fallback(
    catalog: &[LudusaviEntry],
    apps: &[LauncherApp],
    launcher_tag: &str,
    by_slug: &mut HashMap<String, DetectedGame>,
) {
    if apps.is_empty() {
        return;
    }
    let catalog_by_slug: HashMap<&str, &LudusaviEntry> =
        catalog.iter().map(|e| (e.slug.as_str(), e)).collect();

    let mut exact_added = 0usize;
    let mut fuzzy_added = 0usize;
    for app in apps {
        let slug = ludusavi::slugify(&app.name);
        let (entry, via_fuzzy) = match catalog_by_slug.get(slug.as_str()) {
            Some(entry) => (*entry, false),
            None => {
                let Some(entry) = ludusavi::find_by_fuzzy_name(&app.name, FUZZY_NAME_THRESHOLD)
                else {
                    continue;
                };
                (entry, true)
            }
        };
        match by_slug.get_mut(&entry.slug) {
            Some(existing) => {
                // Already linked by Steam appid or another launcher — keep
                // the stronger row but stamp install_dir if it's still
                // empty (e.g. the slug came from `apply_steam_name_fallback`
                // without an install_dir hint).
                if existing.install_dir.is_none() {
                    existing.install_dir = Some(app.install_dir.clone());
                }
                continue;
            }
            None => {
                by_slug.insert(
                    entry.slug.clone(),
                    DetectedGame {
                        slug: entry.slug.clone(),
                        display_name: entry.display_name.clone(),
                        found_paths: Vec::new(),
                        confidence: Confidence::Low,
                        source: DetectionSource::SteamLibrary,
                        steam_app_id: None,
                        install_dir: Some(app.install_dir.clone()),
                    },
                );
            }
        }
        if via_fuzzy {
            fuzzy_added += 1;
            tracing::info!(
                launcher = %launcher_tag,
                app_name = %app.name,
                catalog_slug = %entry.slug,
                "launcher → catalog fuzzy fallback matched"
            );
        } else {
            exact_added += 1;
            tracing::info!(
                launcher = %launcher_tag,
                app_name = %app.name,
                catalog_slug = %entry.slug,
                "launcher → catalog slug match"
            );
        }
    }
    if exact_added > 0 || fuzzy_added > 0 {
        tracing::info!(
            launcher = %launcher_tag,
            exact = exact_added,
            fuzzy = fuzzy_added,
            "launcher cross-reference complete"
        );
    }
}

/// Build the slug → prefix_root map the aggressive walker consumes.
///
/// Unifies three prefix sources behind a single map keyed by Ludusavi slug:
///   * Proton: looked up via the Steam appid → catalog slug.
///   * Lutris and Bottles: the prefix's identifier is slugified directly
///     and used as the key (best-effort — no catalog lookup, the
///     identifier is whatever the user named the bottle / game dir).
///
/// On non-Linux hosts only the Proton wrapper has a chance to contribute,
/// matching the contract of [`wine_prefixes::list_wine_prefixes`].
fn build_prefix_root_by_slug(os: Os) -> HashMap<String, PathBuf> {
    let mut map: HashMap<String, PathBuf> = HashMap::new();
    for prefix in wine_prefixes::list_wine_prefixes(os) {
        let slug = match prefix.kind {
            PrefixKind::Proton => {
                // The identifier is the Steam appid stringified; look up
                // the catalog entry to recover the slug. A miss means
                // Ludusavi doesn't know the appid — silently skip; the
                // walker can't help without a slug to index against.
                let Ok(appid) = prefix.identifier.parse::<u64>() else {
                    continue;
                };
                let Some(entry) = ludusavi::find_by_steam_app_id(appid) else {
                    continue;
                };
                entry.slug.clone()
            }
            PrefixKind::Lutris | PrefixKind::Bottles => ludusavi::slugify(&prefix.identifier),
            // Generic prefixes have no per-game identifier — a single prefix
            // holds many games. They're handled by the dedicated generic-prefix
            // scan in `detect_all`, not the slug-keyed aggressive walker.
            PrefixKind::Generic => continue,
        };
        // First writer wins — multiple prefixes for the same slug
        // (e.g. a Steam install AND a Lutris install of the same game) is
        // an unusual setup; surface a debug log but don't try to merge
        // walks across prefix roots.
        if map.contains_key(&slug) {
            tracing::debug!(
                slug = %slug,
                kind = ?prefix.kind,
                "multiple Wine prefixes resolved to the same slug; keeping the first"
            );
            continue;
        }
        map.insert(slug, prefix.prefix_root);
    }
    map
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
        let mut subdirs = find_save_subdirs(&hit);
        // Content validation (ADR 0019): when a single hit yields several
        // save-named subdirs, prefer the ones that actually hold a recent
        // save-like file. Disambiguates real save folders from editor /
        // "save settings" dirs that merely match a name pattern. Advisory,
        // not a hard filter: if none qualify (e.g. all saves are old), keep
        // them all so we never regress a layout that simply hasn't been
        // touched lately.
        if subdirs.len() > 1 {
            let qualifying: Vec<PathBuf> = subdirs
                .iter()
                .filter(|d| dir_has_recent_save_file(d))
                .cloned()
                .collect();
            if !qualifying.is_empty() {
                subdirs = qualifying;
            }
        }
        for candidate in subdirs {
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
        tracing::info!(applied, orphaned, "manual_paths overrides applied");
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
                fallback_step
                    .kept
                    .push(app.install_dir.display().to_string());
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

/// Ordering helper for `Confidence`: `Low < Medium < High`. Used by the
/// aggressive-walker integration to fold multiple discoveries into one
/// grade.
fn confidence_rank(c: Confidence) -> u8 {
    match c {
        Confidence::Low => 0,
        Confidence::Medium => 1,
        Confidence::High => 2,
    }
}

/// Default per-root timeout for [`aggressive_discover`]. Intentionally
/// conservative: 1.5s is enough to walk a typical install dir on an SSD
/// (depth 4, a few hundred entries) but short enough to keep the overall
/// scan responsive when the heuristic misses on dozens of slugs.
pub(crate) const AGGRESSIVE_WALK_TIMEOUT: Duration = Duration::from_millis(1500);

/// Default depth cap for [`aggressive_discover`]. Four levels covers the
/// most common save-dir layouts (e.g. `<install>/data/save_games/slot1`)
/// without descending into asset trees.
pub(crate) const AGGRESSIVE_WALK_MAX_DEPTH: usize = 4;

/// Hard cap on the number of save-like dirs we report per walked root.
/// DETECCIÓN (fase 1, ADR 0020): el ADR pide eliminar este tope porque la
/// verdadera puerta de calidad ahora es el score de `scoring::score_dir`
/// (sólo cruzan el umbral las carpetas con evidencia real). Lo subimos
/// 5→16 en vez de quitarlo del todo: sigue actuando de cinturón de
/// seguridad ante un árbol patológico, pero ya no recorta candidatos
/// legítimos a los primeros cinco.
pub(crate) const AGGRESSIVE_WALK_MAX_CANDIDATES: usize = 16;

/// How often (in dirs visited) the walker re-checks the elapsed timeout.
/// `Instant::elapsed` is cheap but not free; sampling every N entries keeps
/// the overhead negligible without letting the walker run far past the cap.
const TIMEOUT_CHECK_INTERVAL: usize = 10;

/// Directory names we never descend into during the aggressive walk.
/// Anything binary-/asset-heavy (mostly large, never holds saves), plus the
/// usual VCS/build folders. Comparison is case-insensitive (see
/// [`is_skip_dir`]).
pub(crate) const WALK_SKIP: &[&str] = &[
    "bin",
    "lib",
    "libs",
    "locale",
    "locales",
    "languages",
    "audio",
    "video",
    "movies",
    "music",
    "fonts",
    "shaders",
    "content",
    "_commonredist",
    "vcredist",
    "dotnet",
    "node_modules",
    ".git",
    ".vs",
];

/// File extensions we treat as "save-like" when promoting a dir from
/// `Low` to `Medium` confidence. Compared case-insensitively. Kept tight on
/// purpose — `.cfg` / `.ini` would generate too many false positives from
/// engine config dirs.
pub(crate) const SAVE_FILE_EXTENSIONS: &[&str] = &["sav", "save", "profile", "json", "dat", "xml"];

/// How recent a save-like file has to be to promote a dir to `Medium`.
/// DETECCIÓN (recall, fase 1): subido 90→180 días. 90 dejaba fuera saves
/// de juegos jugados "la temporada pasada"; 180 los recupera y sigue
/// descartando data shippeada (que casi siempre tiene la mtime del install,
/// más vieja que medio año en cuanto el juego lleva tiempo instalado).
pub(crate) const RECENT_SAVE_FILE_WINDOW: Duration = Duration::from_secs(60 * 60 * 24 * 180);

/// One save-like path discovered by [`aggressive_discover`]. The `reason`
/// is forwarded to the diagnostics panel so a human can see *why* a path
/// got accepted (e.g. `"matches SAVE_PATTERNS"` vs
/// `"matches pattern + recent save-like files"`).
#[derive(Debug, Clone)]
pub struct DiscoveredSavePath {
    pub path: PathBuf,
    pub confidence: Confidence,
    pub reason: String,
}

/// Aggressive filesystem walker for slugs that finished the main pipeline
/// without any `found_paths`. Walks `install_dir` and the Proton prefix
/// user dir (if present) down to `max_depth`, skipping the
/// [`WALK_SKIP`] denylist and well-known asset roots, and collects dirs
/// whose name matches [`SAVE_PATTERNS`] or the `slot/profile/user<N>`
/// regex-shaped pattern.
///
/// Bails out per-root if `timeout_per_root` elapses (checked every 10
/// entries) or once [`AGGRESSIVE_WALK_MAX_CANDIDATES`] candidates are
/// found in that root. The function never panics: missing dirs and
/// permission errors are silently skipped.
///
/// `display_name` is reserved for future "game-like dir name" heuristics
/// (e.g. only descend into subdirs whose slugified name matches the slug
/// or its aliases). 1.5.1 ships the simpler pattern-match-only flavour;
/// the param is kept in the signature to avoid an API break later.
pub fn aggressive_discover(
    slug: &str,
    display_name: &str,
    install_dir: Option<&Path>,
    prefix_root: Option<&Path>,
    timeout_per_root: Duration,
    max_depth: usize,
) -> Vec<DiscoveredSavePath> {
    // Public API flavour: no correlation context, pure static scoring. The
    // engine path in `detect_all` calls `aggressive_discover_with` so the
    // per-slug walk also gets the process↔write bonus when the store
    // corroborates a candidate.
    aggressive_discover_with(
        slug,
        display_name,
        install_dir,
        prefix_root,
        timeout_per_root,
        max_depth,
        &CorrelationStore::default(),
    )
}

/// Correlation-aware core of [`aggressive_discover`]. The walk and grading
/// are identical; the `store` lets [`classify_dir_as_save_like`] add the
/// +0.50 process↔write bonus and unlock `High` for corroborated dirs.
pub(crate) fn aggressive_discover_with(
    _slug: &str,
    _display_name: &str,
    install_dir: Option<&Path>,
    prefix_root: Option<&Path>,
    timeout_per_root: Duration,
    max_depth: usize,
    store: &CorrelationStore,
) -> Vec<DiscoveredSavePath> {
    let mut out: Vec<DiscoveredSavePath> = Vec::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();

    if let Some(root) = install_dir {
        if root.is_dir() {
            walk_root_collecting(
                root,
                max_depth,
                timeout_per_root,
                &mut out,
                &mut seen,
                store,
            );
        }
    }

    if let Some(prefix) = prefix_root {
        // Proton prefixes mirror a Windows `C:\` drive under `drive_c/`.
        // The user-writable areas (AppData, Documents, save folders) all
        // live under `drive_c/users/steamuser/`, so walking from there
        // avoids the `drive_c/windows` / `drive_c/Program Files` noise.
        let user = prefix.join("drive_c/users/steamuser");
        if user.is_dir() {
            walk_root_collecting(
                &user,
                max_depth,
                timeout_per_root,
                &mut out,
                &mut seen,
                store,
            );
        }
    }

    out
}

/// Per-root timeout for the phase-4 broad-root walk. Slightly more generous
/// than the per-slug install walk because the user save roots (AppData,
/// `~/.local/share`, …) fan out wider, but still short enough that the whole
/// pass stays in the "detection takes seconds" budget. The
/// [`AGGRESSIVE_WALK_MAX_CANDIDATES`] cap bounds output per root regardless.
const PHASE4_WALK_TIMEOUT: Duration = Duration::from_millis(2000);

/// Depth cap for the phase-4 walk. Save folders under the user roots sit a
/// few levels deep (`<root>/<Company>/<Game>/Saves`), so 4 covers the common
/// layouts without descending into asset trees.
const PHASE4_WALK_MAX_DEPTH: usize = 4;

/// Deep-scan phase-4 budget: a deeper descent and a more generous per-root
/// timeout, since the user explicitly asked for an exhaustive look and the
/// sandbox/emulator roots (`~/.var/app/<id>/...`, `Emulation/saves/<system>`)
/// nest the actual save folder one or two levels lower than native layouts.
const PHASE4_DEEP_WALK_MAX_DEPTH: usize = 6;
const PHASE4_DEEP_WALK_TIMEOUT: Duration = Duration::from_millis(5000);

/// Generic path segments that are containers, never the game's name. When
/// attributing a discovered save folder we walk up past these (and past
/// recognised save-words) to reach the segment that actually names the game.
const GENERIC_SEGMENTS: &[&str] = &[
    "appdata",
    "roaming",
    "local",
    "locallow",
    "documents",
    "my games",
    "saved games",
    "savedgames",
    ".config",
    ".local",
    "share",
    "state",
    "users",
    "steamuser",
    "drive_c",
    "home",
];

/// A save folder discovered catalog-free (phase 4) and attributed to a game.
/// `slug`/`display_name` come from the attribution heuristic: the process
/// that wrote the folder (correlation) wins; otherwise the nearest
/// non-generic ancestor folder name.
#[derive(Debug, Clone)]
pub struct AttributedSave {
    pub slug: String,
    pub display_name: String,
    pub path: PathBuf,
    pub confidence: Confidence,
    pub reason: String,
}

/// Phase 4 (ADR 0020): catalog-free discovery + attribution.
///
/// One pass over the broad user save roots ([`roots::user_save_roots`]) and
/// the Wine/Proton prefixes, scoring every candidate **with** the correlation
/// store so GUID-named / non-English save folders that no catalog or Steam
/// signal could reach still surface. Each survivor is attributed to a game
/// name (process-correlation first, ancestor folder name as fallback).
///
/// Precision gate: in these broad roots a name-only `Low` hit is too weak to
/// mint a phantom game, so a candidate is only kept when the correlation
/// store corroborates it **or** it carries strong static evidence
/// (`Medium`/`High`). Anything already claimed by a catalog/Steam match
/// (`known_paths`) is skipped.
pub fn discover_unattributed(
    os: Os,
    store: &CorrelationStore,
    known_paths: &HashSet<PathBuf>,
) -> Vec<AttributedSave> {
    discover_unattributed_mode(os, store, known_paths, false)
}

/// `deep` variant: walks the broad sandbox/emulator roots
/// ([`roots::deep_save_roots`]) and arbitrarily-located Wine prefixes on top of
/// the standard roots, with a deeper walk, longer per-root timeout, and a
/// relaxed precision gate (keeps `Low` static-only hits the periodic scan
/// drops, since the deep scan is an explicit "find what's hiding" request).
pub fn discover_unattributed_mode(
    os: Os,
    store: &CorrelationStore,
    known_paths: &HashSet<PathBuf>,
    deep: bool,
) -> Vec<AttributedSave> {
    let mut out: Vec<AttributedSave> = Vec::new();
    let mut emitted: HashSet<PathBuf> = HashSet::new();

    let mut walk_roots = roots::user_save_roots(os);
    let prefixes = if deep {
        wine_prefixes::list_wine_prefixes_deep(os)
    } else {
        wine_prefixes::list_wine_prefixes(os)
    };
    for prefix in prefixes {
        walk_roots.extend(roots::prefix_user_roots(&prefix.prefix_root));
    }
    if deep {
        walk_roots.extend(roots::deep_save_roots(os));
    }

    let (max_depth, timeout) = if deep {
        (PHASE4_DEEP_WALK_MAX_DEPTH, PHASE4_DEEP_WALK_TIMEOUT)
    } else {
        (PHASE4_WALK_MAX_DEPTH, PHASE4_WALK_TIMEOUT)
    };

    for root in walk_roots {
        let mut hits: Vec<DiscoveredSavePath> = Vec::new();
        let mut seen: HashSet<PathBuf> = HashSet::new();
        walk_root_collecting(&root, max_depth, timeout, &mut hits, &mut seen, store);
        for hit in hits {
            let corroborated = store.signal_for(&hit.path).is_some();
            // Precision gate: weak static-only matches don't create games —
            // except in deep mode, where surfacing maybes is the whole point.
            if !deep && !corroborated && hit.confidence == Confidence::Low {
                continue;
            }
            if path_already_known(&hit.path, known_paths) {
                continue;
            }
            if !emitted.insert(hit.path.clone()) {
                continue;
            }
            let display_name = attribute_game_name(&hit.path, store);
            let slug = ludusavi::slugify(&display_name);
            if slug.is_empty() {
                continue;
            }
            out.push(AttributedSave {
                slug,
                display_name,
                path: hit.path,
                confidence: hit.confidence,
                reason: hit.reason,
            });
        }
    }

    out
}

/// True if `candidate` is already covered by a catalog/Steam hit: either it
/// equals a known path, sits inside one, or contains one.
fn path_already_known(candidate: &Path, known: &HashSet<PathBuf>) -> bool {
    known
        .iter()
        .any(|k| candidate.starts_with(k) || k.starts_with(candidate))
}

/// Best-effort game name for a phase-4 save folder.
///
/// 1. If the correlation store attributed a process to the folder, use it
///    (the process name minus `.exe` is the game far more often than not).
/// 2. Otherwise walk up from the folder past recognised save-words and
///    generic container segments; the first "real" segment names the game
///    (`…/My Games/Skyrim/Saves` → `Skyrim`).
fn attribute_game_name(path: &Path, store: &CorrelationStore) -> String {
    if let Some(proc) = store.attributed_name(path) {
        let trimmed = proc.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    let mut cur = Some(path);
    while let Some(dir) = cur {
        if let Some(name) = dir.file_name().and_then(|s| s.to_str()) {
            let lower = name.to_lowercase();
            let generic = GENERIC_SEGMENTS.contains(&lower.as_str());
            // A purely-numeric segment is an account/user id, not a game name
            // — SteamID64 (17 digits), per-user numeric profile dirs, etc.
            // `…/Gaddy Games/Plan B Terraform/76561197960287930/saves` must
            // attribute to "Plan B Terraform", not the SteamID. Keep climbing.
            let id_like = name.len() >= 6 && name.chars().all(|c| c.is_ascii_digit());
            if !generic && !id_like && !scoring::name_recognised(name) {
                return name.to_string();
            }
        }
        cur = dir.parent();
    }
    path.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string()
}

/// Walk one root, appending up to [`AGGRESSIVE_WALK_MAX_CANDIDATES`]
/// discoveries into `out`. Honours the timeout and the depth cap.
fn walk_root_collecting(
    root: &Path,
    max_depth: usize,
    timeout: Duration,
    out: &mut Vec<DiscoveredSavePath>,
    seen: &mut HashSet<PathBuf>,
    store: &CorrelationStore,
) {
    let start = Instant::now();
    let mut entries_checked: usize = 0;
    let initial = out.len();
    // Depth-first walk via an explicit stack. Save-like dirs are not
    // descended into (see the `continue` below), so the order in which we
    // exhaust branches doesn't change which paths qualify — only the order
    // they're appended to `out` before the cap kicks in. Each entry is
    // `(path, depth)` so we can cap descent at push-time.
    let mut stack: Vec<(PathBuf, usize)> = vec![(root.to_path_buf(), 0)];
    while let Some((dir, depth)) = stack.pop() {
        if out.len() - initial >= AGGRESSIVE_WALK_MAX_CANDIDATES {
            break;
        }
        if entries_checked.is_multiple_of(TIMEOUT_CHECK_INTERVAL) && start.elapsed() >= timeout {
            break;
        }
        entries_checked += 1;

        let Ok(read) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in read.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if !file_type.is_dir() {
                continue;
            }
            let path = entry.path();
            let name = entry.file_name();
            let Some(name_str) = name.to_str() else {
                continue;
            };
            if is_skip_dir(name_str) {
                continue;
            }
            // Never walk Hoard's own bookkeeping (conflict backups,
            // correlation/state json) nor the desktop trash — descending there
            // mints phantom "games" out of our own data (e.g. the timestamped
            // `conflicts/<id>/<ts>/autosave` folders) or out of deleted files.
            if is_internal_or_trash(&path) {
                continue;
            }
            if let Some(hit) = classify_dir_as_save_like(&path, name_str, store) {
                if seen.insert(path.clone()) {
                    out.push(hit);
                    if out.len() - initial >= AGGRESSIVE_WALK_MAX_CANDIDATES {
                        return;
                    }
                }
                // Even when this dir is itself save-like we don't descend
                // into it — saves typically live one level deep, and going
                // further just bloats the candidate list.
                continue;
            }
            if depth < max_depth {
                stack.push((path, depth + 1));
            }
        }
    }
}

/// True iff the directory name is in [`WALK_SKIP`] (case-insensitive).
fn is_skip_dir(name: &str) -> bool {
    let lower = name.to_lowercase();
    WALK_SKIP.contains(&lower.as_str())
}

/// True if `path` lives inside Hoard's own state dir (conflict backups,
/// `correlation.json`, `state.json`, …) or any desktop trash. The aggressive
/// walk and phase-4 discovery must skip these: our conflict backups are real
/// save bytes copied verbatim, so they score save-like and would resurface as
/// phantom games named after the backup timestamp; trashed folders are
/// deleted, not installed games.
fn is_internal_or_trash(path: &Path) -> bool {
    if let Ok(state_dir) = crate::config::CliConfig::state_dir() {
        if path.starts_with(&state_dir) {
            return true;
        }
    }
    path.components().any(|c| {
        let s = c.as_os_str().to_string_lossy();
        s == "Trash" || s.starts_with(".Trash")
    })
}

/// If a dir's name + contents look save-like, return a
/// [`DiscoveredSavePath`] graded by [`scoring::score_dir`] (fase 1, ADR
/// 0020). Below `SCORE_POSSIBLE` (0.35) the dir is dropped; `≥ SCORE_CONFIRMED`
/// (0.60) maps to `Medium`, the grey zone to `Low`. `High` is withheld until
/// the process-correlation signal of fase 3 exists. The `reason` carries the
/// numeric score and the signal breakdown for the diagnostics panel.
fn classify_dir_as_save_like(
    path: &Path,
    name: &str,
    store: &CorrelationStore,
) -> Option<DiscoveredSavePath> {
    // DETECCIÓN (fase 1+3, ADR 0020): el booleano name-only se sustituye por
    // el scoring graduado (`scoring::score_dir`: nombre + contenido +
    // recencia + negativas) MÁS el bonus de correlación proceso↔escritura
    // (+0.50) si el store corrobora el dir. Por debajo de `SCORE_POSSIBLE`
    // se descarta.
    //
    // Mapeo a `Confidence`: con el bucle de correlación cerrado, `High` ya
    // no se reserva — se concede cuando el score cruza el cutoff de
    // auto-confirmado Y la escritura quedó atribuida a un proceso de juego
    // (la señal que el ADR exige para certeza). Sin correlación, un score
    // alto puramente estático tope en `Medium`. El número va en `reason`
    // para el panel de diagnóstico.
    let breakdown = correlation::score_with_correlation(path, name, store);
    if breakdown.score < scoring::SCORE_POSSIBLE {
        return None;
    }
    // Corroboración para conceder `High`: correlación proceso↔escritura
    // (el store) O un comprimido con índice save-like verificado. Lo segundo
    // es evidencia directa —abrimos el .zip y vimos el save dentro— así que
    // vale tanto como la correlación y no exige una sesión de juego observada.
    let corroborated = store.signal_for(path).is_some() || breakdown.corroborated_by_content;
    let confidence = if breakdown.score >= scoring::SCORE_CONFIRMED {
        if corroborated {
            Confidence::High
        } else {
            Confidence::Medium
        }
    } else {
        Confidence::Low
    };
    Some(DiscoveredSavePath {
        path: path.to_path_buf(),
        confidence,
        reason: format!(
            "score {:.2}: {}",
            breakdown.score,
            breakdown.reasons.join(", ")
        ),
    })
}

/// Match the `slot|profile|user [sep] <digits>` pattern without pulling
/// in a regex dep. Case-insensitive; separator is optional non-alnum.
/// Examples: `slot1`, `Slot_2`, `profile-3`, `user 04`.
pub(crate) fn name_matches_slot_profile_user(name: &str) -> bool {
    let lower = name.to_lowercase();
    for prefix in ["slot", "profile", "user"] {
        let Some(rest) = lower.strip_prefix(prefix) else {
            continue;
        };
        if rest.is_empty() {
            continue;
        }
        // Skip an optional single non-alnum separator.
        let after_sep = rest
            .strip_prefix(|c: char| !c.is_alphanumeric())
            .unwrap_or(rest);
        if !after_sep.is_empty() && after_sep.chars().all(|c| c.is_ascii_digit()) {
            return true;
        }
    }
    false
}

/// True iff `dir` contains at least one regular file with an extension in
/// [`SAVE_FILE_EXTENSIONS`] modified inside [`RECENT_SAVE_FILE_WINDOW`].
pub(crate) fn dir_has_recent_save_file(dir: &Path) -> bool {
    let Ok(read) = std::fs::read_dir(dir) else {
        return false;
    };
    let now = SystemTime::now();
    for entry in read.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_file() {
            continue;
        }
        let path = entry.path();
        let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
            continue;
        };
        let ext_lower = ext.to_lowercase();
        if !SAVE_FILE_EXTENSIONS.contains(&ext_lower.as_str()) {
            continue;
        }
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        let Ok(modified) = meta.modified() else {
            continue;
        };
        if let Ok(age) = now.duration_since(modified) {
            if age <= RECENT_SAVE_FILE_WINDOW {
                return true;
            }
        }
    }
    false
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
        assert_eq!(refined, vec![root.join("save games"), root.join("saves")]);
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
            registry: Vec::new(),
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
            let save_dir = steamapps.join(
                "compatdata/413150/pfx/drive_c/users/steamuser/AppData/Roaming/StardewValley/Saves",
            );
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
            let save_dir = steamapps.join(
                "compatdata/413150/pfx/drive_c/users/steamuser/AppData/Roaming/StardewValley/Saves",
            );
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
            // Compare component-wise via `Path`, not as raw strings: on
            // Windows the `kept` entries are built with component joins and
            // mix `\\` with the embedded `/` separators, while `save_dir`
            // here is a single forward-slash join. `Path` equality
            // normalises separators on every host; string equality doesn't.
            assert!(
                proton_steps
                    .iter()
                    .any(|s| s.kept.iter().any(|k| Path::new(k) == save_dir)),
                "expected a proton_prefix step whose kept contains {}; got {proton_steps:?}",
                save_dir.display()
            );
        });
    }

    /// Walker happy path: a fresh save file inside a save-named dir under
    /// the install root is reported with `Medium` confidence.
    #[test]
    fn aggressive_discover_finds_save_dir_in_install() {
        let tmp = tempfile::tempdir().unwrap();
        let install = tmp.path().join("Game");
        let save_dir = install.join("data").join("SaveGames");
        std::fs::create_dir_all(&save_dir).unwrap();
        // A `.sav` file modified now → recent save-like file → Medium.
        std::fs::write(save_dir.join("slot1.sav"), b"binary").unwrap();

        let hits = aggressive_discover(
            "fake-slug",
            "Fake Game",
            Some(&install),
            None,
            AGGRESSIVE_WALK_TIMEOUT,
            AGGRESSIVE_WALK_MAX_DEPTH,
        );

        assert_eq!(hits.len(), 1, "expected exactly one hit, got {hits:?}");
        assert_eq!(hits[0].path, save_dir);
        assert_eq!(hits[0].confidence, Confidence::Medium);
        // DETECCIÓN (fase 1, ADR 0020): el reason ahora es el desglose del
        // score; un `.sav` reciente aporta "strong save ext" + "recent
        // save-like file".
        assert!(
            hits[0].reason.contains("recent save-like file"),
            "expected a 'recent save-like file' reason; got {:?}",
            hits[0].reason
        );
    }

    /// A `save/` dir nested under a `bin/` denylist entry must not be
    /// walked into — `WALK_SKIP` is honoured even when the inner dir name
    /// matches the save patterns.
    #[test]
    fn aggressive_discover_respects_skip_list() {
        let tmp = tempfile::tempdir().unwrap();
        let install = tmp.path().join("Game");
        std::fs::create_dir_all(install.join("bin").join("save")).unwrap();

        let hits = aggressive_discover(
            "fake-slug",
            "Fake Game",
            Some(&install),
            None,
            AGGRESSIVE_WALK_TIMEOUT,
            AGGRESSIVE_WALK_MAX_DEPTH,
        );

        assert!(
            hits.is_empty(),
            "WALK_SKIP must hide save dirs nested under denylisted parents; got {hits:?}"
        );
    }

    /// Empty `save/` stays `Low`; sibling `save/` with a fresh `.sav` rises
    /// to `Medium`. The promotion is observable only when a save-like
    /// file is present and recent.
    #[test]
    fn aggressive_discover_promotes_with_recent_savefile() {
        let tmp = tempfile::tempdir().unwrap();
        let empty_install = tmp.path().join("Empty");
        std::fs::create_dir_all(empty_install.join("save")).unwrap();
        let filled_install = tmp.path().join("Filled");
        std::fs::create_dir_all(filled_install.join("save")).unwrap();
        std::fs::write(filled_install.join("save").join("slot.sav"), b"...").unwrap();

        let empty_hits = aggressive_discover(
            "empty-slug",
            "Empty Game",
            Some(&empty_install),
            None,
            AGGRESSIVE_WALK_TIMEOUT,
            AGGRESSIVE_WALK_MAX_DEPTH,
        );
        let filled_hits = aggressive_discover(
            "filled-slug",
            "Filled Game",
            Some(&filled_install),
            None,
            AGGRESSIVE_WALK_TIMEOUT,
            AGGRESSIVE_WALK_MAX_DEPTH,
        );

        assert_eq!(empty_hits.len(), 1);
        assert_eq!(empty_hits[0].confidence, Confidence::Low);
        assert_eq!(filled_hits.len(), 1);
        assert_eq!(filled_hits[0].confidence, Confidence::Medium);
    }

    /// Both roots `None`, plus both roots pointing at missing paths,
    /// produce an empty vec without panicking.
    #[test]
    fn aggressive_discover_returns_empty_on_missing_dirs() {
        let none_hits = aggressive_discover(
            "fake-slug",
            "Fake Game",
            None,
            None,
            AGGRESSIVE_WALK_TIMEOUT,
            AGGRESSIVE_WALK_MAX_DEPTH,
        );
        assert!(none_hits.is_empty());

        let bogus = PathBuf::from("/definitely/not/a/real/path/zzz-aggressive-walker");
        let bogus_hits = aggressive_discover(
            "fake-slug",
            "Fake Game",
            Some(&bogus),
            Some(&bogus),
            AGGRESSIVE_WALK_TIMEOUT,
            AGGRESSIVE_WALK_MAX_DEPTH,
        );
        assert!(bogus_hits.is_empty());
    }

    /// Fase 3 (cierre del bucle): un dir con evidencia estática fuerte
    /// (`.sav` reciente) que sin correlación tope en `Medium` se promociona
    /// a `High` cuando el store atribuye la escritura a un proceso de juego.
    #[test]
    fn classify_unlocks_high_with_correlation() {
        let tmp = std::env::temp_dir().join(format!(
            "hoard-classify-high-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join("game.sav"), b"x").unwrap();

        // Sin correlación: strong ext (+0.30) lo deja en `Medium` como mucho,
        // nunca `High`.
        let empty = CorrelationStore::default();
        let plain = classify_dir_as_save_like(&tmp, "saves", &empty).unwrap();
        assert_ne!(plain.confidence, Confidence::High);

        // Con correlación: +0.50 cruza holgado el cutoff y desbloquea `High`.
        let mut store = CorrelationStore::default();
        store.record(
            &tmp,
            &[crate::correlation::GameProcess {
                name: "weirdgame.exe".into(),
                exe: None,
            }],
        );
        let corr = classify_dir_as_save_like(&tmp, "saves", &store).unwrap();
        assert_eq!(corr.confidence, Confidence::High);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// Corroboración por contenido: una carpeta `saves` con un `.zip` cuyo
    /// índice delata un save (Factorio: `control.lua`) se concede `High` sin
    /// necesidad de correlación de proceso — abrimos el archivo y lo vimos.
    #[test]
    fn classify_archive_content_unlocks_high_without_correlation() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let saves = dir.path().join("saves");
        std::fs::create_dir_all(&saves).unwrap();
        let file = std::fs::File::create(saves.join("world.zip")).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        zip.start_file("world/control.lua", opts).unwrap();
        zip.write_all(b"x").unwrap();
        zip.finish().unwrap();

        let empty = CorrelationStore::default();
        let hit = classify_dir_as_save_like(&saves, "saves", &empty).unwrap();
        assert_eq!(hit.confidence, Confidence::High);
    }

    /// Atribución (fase 4): el nombre del proceso que escribió la carpeta gana
    /// sobre la heurística de ancestro; sin proceso, se usa el primer segmento
    /// no genérico subiendo por el árbol.
    #[test]
    fn attribute_game_name_prefers_process_then_ancestor() {
        let path = PathBuf::from("/home/u/.local/share/Skyrim/Saves");

        // Sin correlación: sube desde `Saves` (save-word) hasta `Skyrim`.
        let empty = CorrelationStore::default();
        assert_eq!(attribute_game_name(&path, &empty), "Skyrim");

        // Con correlación: el proceso atribuido manda.
        let mut store = CorrelationStore::default();
        store.record(
            &path,
            &[crate::correlation::GameProcess {
                name: "EldenRing.exe".into(),
                exe: None,
            }],
        );
        assert_eq!(attribute_game_name(&path, &store), "EldenRing");
    }

    #[test]
    fn path_already_known_matches_ancestors_and_descendants() {
        let mut known = HashSet::new();
        known.insert(PathBuf::from("/games/a/Saves"));
        assert!(path_already_known(Path::new("/games/a/Saves"), &known));
        assert!(path_already_known(
            Path::new("/games/a/Saves/slot1"),
            &known
        ));
        assert!(path_already_known(Path::new("/games/a"), &known));
        assert!(!path_already_known(Path::new("/games/b/Saves"), &known));
    }

    /// Fase 4 end-to-end: una carpeta de nombre opaco (GUID) bajo un root de
    /// usuario, estáticamente invisible, aflora SÓLO porque la correlación la
    /// corrobora, y se atribuye al proceso que la escribió.
    #[test]
    fn discover_unattributed_rescues_correlated_guid_folder() {
        with_isolated_home(|home| {
            let xdg_data = home.join("xdg-data");
            let guid = xdg_data.join("a1b2c3d4-e5f6");
            std::fs::create_dir_all(&guid).unwrap();

            // Sin correlación no aflora nada (nombre opaco, carpeta vacía).
            let empty = CorrelationStore::default();
            let none = discover_unattributed(Os::Linux, &empty, &HashSet::new());
            assert!(
                none.iter().all(|a| a.path != guid),
                "opaque empty folder must stay invisible without correlation"
            );

            // Con correlación sí, y atribuida al proceso.
            let mut store = CorrelationStore::default();
            store.record(
                &guid,
                &[crate::correlation::GameProcess {
                    name: "mysterygame.exe".into(),
                    exe: None,
                }],
            );
            let found = discover_unattributed(Os::Linux, &store, &HashSet::new());
            let hit = found
                .iter()
                .find(|a| a.path == guid)
                .expect("correlated GUID folder should surface in phase 4");
            assert_eq!(hit.display_name, "mysterygame");
            assert_eq!(hit.slug, "mysterygame");

            // Si ya está reclamada por el catálogo, se omite.
            let mut known = HashSet::new();
            known.insert(guid.clone());
            let skipped = discover_unattributed(Os::Linux, &store, &known);
            assert!(skipped.iter().all(|a| a.path != guid));
        });
    }

    /// Regresión: los backups de conflictos del propio Hoard
    /// (`<state_dir>/conflicts/<id>/<ts>/autosave`) son bytes de save
    /// copiados verbatim, así que puntúan save-like y, antes del fix, afloraban
    /// como juegos fantasma nombrados con el timestamp. El walk debe saltarlos
    /// aunque la correlación los corrobore.
    #[test]
    fn discover_unattributed_skips_hoard_internal_conflicts() {
        with_isolated_home(|_home| {
            let state_dir = crate::config::CliConfig::state_dir().unwrap();
            let conflict = state_dir
                .join("conflicts")
                .join("a9d4b6d5-2df7-4633-b733-63708660d8e5")
                .join("2026-05-30T11-35-47.308147652Z")
                .join("autosave");
            std::fs::create_dir_all(&conflict).unwrap();
            std::fs::write(conflict.join("game.sav"), b"x").unwrap();

            // Correlación fuerte sobre la carpeta interna: aun así no debe salir.
            let mut store = CorrelationStore::default();
            store.record(
                &conflict,
                &[crate::correlation::GameProcess {
                    name: "openttd".into(),
                    exe: None,
                }],
            );
            let found = discover_unattributed(Os::Linux, &store, &HashSet::new());
            assert!(
                found.iter().all(|a| !a.path.starts_with(&state_dir)),
                "Hoard's own conflict backups must never surface as games: {:?}",
                found.iter().map(|a| &a.path).collect::<Vec<_>>()
            );
        });
    }
}
