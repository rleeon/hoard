//! Auto-detect installed games on the host **without talking to the server**.
//!
//! Detection runs against the catalog embedded in [`hoard_manifest`]:
//! ~20k games imported from the Ludusavi public manifest at build time, plus
//! the hand-curated TOML entries. Both sources are merged so the user sees
//! every game that has a save-path definition we know about, full stop:
//! no server round-trips, no "only ten games found" because the admin
//! hasn't run a manifest import yet.
//!
//! Two complementary signals decide whether a game is *installed*:
//!
//! 1. Filesystem heuristic: for each catalog entry, expand its
//!    save-path templates against the local environment (`<winAppData>`,
//!    `<xdgData>`, `<home>`, …) and check whether any expanded directory
//!    actually exists. A hit means the user has played (or at least
//!    installed) the game on this machine. Catches GOG, Epic, DRM-free,
//!    pirated installs, anything that left a save folder behind.
//! 2. Steam library scan: read Steam's `libraryfolders.vdf` and
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

use std::cell::OnceCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use anyhow::Result;
use hoard_manifest::ludusavi::{self, LudusaviEntry};
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;

use crate::correlation::{self, CorrelationStore};
use crate::emulators;
use crate::junkdirs;
use crate::launchers::{self, LauncherApp};
use crate::manifest::Os;
use crate::pathexpand::{
    self, expand_path_in_prefix, expand_path_in_prefix_as_user, expand_path_scoped,
    expand_registry_path,
};
use crate::roots;
use crate::scoring;
use crate::state::CliState;
use crate::steam::{self, SteamApp};
use crate::wine_prefixes::{self, PrefixKind};
use crate::wrappers;
use hoard_core::kernel::fileclass;

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
    /// `CliState::manual_paths` and leads `found_paths` with `High`: the user
    /// knows where their saves are better than any scrape.
    ///
    /// Leads it, does **not** replace it: whatever the heuristic found stays
    /// behind it. See [`apply_manual_overrides`].
    ManualOverride,
}

/// One game we believe is installed on this machine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedGame {
    pub slug: String,
    pub display_name: String,
    /// **Save**-path candidates that exist on disk. Never contains the game's
    /// install directory, which lives in [`install_dir`] so the UI can show
    /// it as a hint without us accidentally backing up the game binary.
    /// Empty for Steam-only matches where no save folder has been created yet.
    pub found_paths: Vec<PathBuf>,
    pub confidence: Confidence,
    /// Per-path confidence, aligned 1:1 with [`found_paths`] and sorted
    /// strongest-first alongside it. Lets the UI show a distinct grade per
    /// save folder (e.g. the real `~/Saved Games/.../saves` as `High` vs an
    /// almost-empty Steam-Cloud stub as `Low`) and lets automatic tracking
    /// pick the **best** path instead of whichever source happened to be
    /// pushed first. `confidence` above stays the rolled-up max. `default`
    /// keeps older cached reports loading without migration.
    #[serde(default)]
    pub path_confidences: Vec<Confidence>,
    /// Per-path WHY, aligned 1:1 with [`found_paths`]: the scored reasons
    /// that put each folder where it ended up ("name exact, strong save ext,
    /// recent save-like file", or the correlation note). The breakdown was
    /// already computed by [`grade_and_rank_paths`] and thrown away, which is
    /// why "why did it pick THIS folder" was unanswerable, locally or from
    /// support. Empty strings are placeholders for paths this build didn't
    /// re-grade (single-path rows inherit the rolled-up grade without extra
    /// I/O); an empty vec means the row predates the field.
    #[serde(default)]
    pub path_reasons: Vec<String>,
    pub source: DetectionSource,
    /// If we matched via Steam, the app id is preserved so the UI can show it.
    pub steam_app_id: Option<u64>,
    /// Steam install directory (e.g. `…/steamapps/common/Stellaris`). Only
    /// set when we matched via Steam. Surfaced to the UI as a hint near the
    /// folder picker, and **must not** be used as a backup path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub install_dir: Option<PathBuf>,
    /// Detection finished without a save folder for this game.
    ///
    /// The row is still true (the game IS installed, that is what the Steam
    /// manifest or the launcher said) but there is nothing here to back up
    /// until someone points at a folder. Said out loud rather than left to be
    /// inferred from an empty `found_paths`, because the inference is what made
    /// the row a dead end: a caller reading the list sees a detected game, and
    /// "detected" is not what this is. A frontend answers it with the folder
    /// picker; a machine caller branches on the field.
    ///
    /// Two ways to get here, and both are worth showing: nothing on disk looked
    /// like this game's save folder, or what the catalog named turned out to
    /// hold no save at all (see `drop_folders_without_saves`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub needs_folder: bool,
    /// The catalog says this game supports Steam Cloud.
    ///
    /// **Informational only.** It is deliberately not an input to confidence,
    /// ordering, or auto-track: plenty of people want a second copy precisely
    /// *because* Steam Cloud exists (it only covers the Steam copy, it can be
    /// disabled per-game, and it has no history to roll back to). The UI just
    /// says so next to the game; nothing in the pipeline reads it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub steam_cloud: bool,
}

/// Aggregate result of a detection pass. The numeric counts let the UI show a
/// summary banner ("Found 47 games") without re-counting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionReport {
    pub games: Vec<DetectedGame>,
    pub catalog_size: usize,
    pub steam_apps_found: usize,
    pub scanned_at_ms: u64,
    /// Per-stage counters + wall time for this pass. `default` keeps cached
    /// reports from older builds loading without migration.
    #[serde(default)]
    pub stats: DetectionStats,
    /// Tracked folders that look like the game's own backup mirror of another
    /// detected folder (P9). Read-only on purpose: nothing here re-points a
    /// save; surfacing the suggestion is as far as the pipeline goes, because
    /// silent repointing is what broke slot pairing in aug-2026. `default`
    /// keeps older cached reports loading.
    #[serde(default)]
    pub mirror_warnings: Vec<MirrorWarning>,
}

/// One already-tracked folder that [`detect_tracked_mirrors`] flagged as a
/// backup mirror of a better candidate sitting right next to it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MirrorWarning {
    pub save_id: String,
    pub game_slug: String,
    pub label: String,
    /// The tracked folder that looks like the mirror (`…/SaveGamesBackup`).
    pub tracked_path: PathBuf,
    /// The sibling that looks like the real save (`…/SaveGames/<id>`).
    pub suggested_path: PathBuf,
    /// Why: `"mirror of <suggested> (content superset)"` when the structural
    /// check passed, `"name-only"` when only the suffix relation matched.
    pub reason: String,
}

/// What each pipeline stage contributed to one detection pass, plus the wall
/// time of the whole pass. Serialized with the report, so it lands in the
/// scan cache and in the `Detection complete` log line, to make scan cost
/// and per-stage yield measurable across machines instead of guessed.
/// Counters are "slugs this stage merged/added", not raw path candidates.
///
/// `#[serde(default)]` goes on the container rather than field by field: that way a
/// cache written by an earlier version, missing the counter just added, still
/// loads, and the next one added will not break anything either. Without this,
/// `wrapper_slugs` threw away the user's entire detection cache on update
/// ("detection cache malformed; ignoring: missing field `wrapper_slugs`"). It
/// repairs itself on the next scan, but the library starts cold, and that should
/// not happen over adding a counter.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DetectionStats {
    pub duration_ms: u64,
    pub steam_appid_matches: usize,
    pub steam_name_exact: usize,
    pub steam_name_fuzzy: usize,
    pub launcher_exact: usize,
    pub launcher_fuzzy: usize,
    pub fs_template_slugs: usize,
    pub registry_slugs: usize,
    pub proton_slugs: usize,
    pub generic_prefix_slugs: usize,
    pub steam_cloud_slugs: usize,
    /// Saves found inside a Steam-emulator / repack wrapper (`GSE Saves`,
    /// `CODEX`, …), where the subfolder name is the Steam appid.
    pub wrapper_slugs: usize,
    pub walker_slugs: usize,
    pub phase4_new_games: usize,
    pub phase4_merged_paths: usize,
    pub manual_applied: usize,
    pub manual_orphaned: usize,
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
/// settings folder by accident; see `docs/plans/detection.md` §9).
const SAVE_PATTERNS: &[&str] = &[
    "save",
    "saves",
    "savegame",
    "savegames",
    "save games",
    "save_games",
    // Recall, phase 1: `savedata` and `savefiles` are very common folder names
    // (Unity, many indies, console ports) the original set did not recognise.
    // Matched exactly per segment, so the false-positive risk stays low (it does
    // not match "save settings").
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
/// This function does **not** touch the network: the catalog ships in the
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
/// periodic scan skips: arbitrary Wine prefixes (Heroic/CrossOver/Flatpak/
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
    let wall = Instant::now();
    let mut stats = DetectionStats::default();

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
    // access, so log it loudly and the agent log shows *why* a Steam-heavy
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
                path_confidences: Vec::new(),
                path_reasons: Vec::new(),
                confidence: Confidence::Medium,
                source: DetectionSource::SteamLibrary,
                steam_app_id: Some(app.app_id),
                install_dir: Some(app.install_dir.clone()),
                needs_folder: false,
                steam_cloud: false,
            },
        );
    }
    stats.steam_appid_matches = by_slug.len();
    tracing::info!(
        steam_matches = by_slug.len(),
        "Steam → catalog cross-reference complete"
    );

    // Second pass: any Steam app whose `app_id` didn't match a catalog entry
    // above might still belong to a catalog entry that simply lacks
    // `steam_app_id`. Try a slug match against the game's display name.
    // This catches the long tail of Ludusavi entries scraped from
    // PCGamingWiki without a Steam appid attached.
    (stats.steam_name_exact, stats.steam_name_fuzzy) =
        apply_steam_name_fallback(catalog, &steam_apps, &mut by_slug);

    // Non-Steam launchers (1.5.2): Epic Games / GOG Galaxy / Microsoft Store.
    // Same shape as the Steam name fallback above: slugify the display
    // name, look up exact, fall back to fuzzy. Each function is a no-op on
    // non-Windows (or when its launcher data dir is absent), so calling
    // unconditionally costs nothing on hosts without the launcher.
    let epic_apps = launchers::list_installed_epic_games(os);
    let gog_apps = launchers::list_installed_gog_games(os);
    let ms_apps = launchers::list_installed_msstore_games(os);
    for (apps, tag) in [
        (&epic_apps, "epic"),
        (&gog_apps, "gog"),
        (&ms_apps, "msstore"),
    ] {
        let (exact, fuzzy) = apply_launcher_name_fallback(catalog, apps, tag, &mut by_slug);
        stats.launcher_exact += exact;
        stats.launcher_fuzzy += fuzzy;
    }

    // Filesystem heuristic: spawn one blocking task per game, gated by the
    // semaphore. Each task expands every Windows/Linux/Mac template that
    // applies to the current OS and stat()s every candidate path.
    // `<base>`-relative templates need to know where each game is installed.
    // Built once, shared by every task.
    // `<root>` is the storefront root, and Steam is not the only storefront
    // (`pathexpand::NON_STEAM_STORE_ROOTS`). A root that doesn't own the
    // template costs one stat that misses; leaving it out costs every game
    // whose save lives under another store its save folder.
    let mut store_roots = steam::detect_steam_libraries(os);
    store_roots.extend(roots::other_store_roots(os));
    let install_index = install_dir_index(os, &steam_apps);
    tracing::info!(
        install_dirs = install_index.len(),
        store_roots = store_roots.len(),
        "install-dir index built (resolves <base>)"
    );

    let mut tasks = Vec::new();
    for entry in catalog {
        let templates: Vec<String> = paths_for_os(entry, os);
        if templates.is_empty() {
            continue;
        }
        let slug = entry.slug.clone();
        let display_name = entry.display_name.clone();
        let scope = scope_for(
            entry,
            &install_index,
            &store_roots,
            by_slug
                .get(&entry.slug)
                .and_then(|g| g.install_dir.as_deref()),
        );
        let permit = semaphore.clone().acquire_owned().await?;
        tasks.push(tokio::task::spawn_blocking(move || {
            // _permit drops at end of closure, releasing the slot.
            let _permit = permit;
            let mut hits: Vec<PathBuf> = Vec::new();
            let mut seen: HashSet<PathBuf> = HashSet::new();
            let mut root_matched = false;
            for tmpl in &templates {
                let candidates = expand_path_scoped(tmpl, os, &scope);
                if candidates.is_empty() {
                    // Unknown placeholder or unset env var; pathexpand
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
                stats.fs_template_slugs += 1;
                merge_fs_hit_graded(&mut by_slug, slug, display_name, hits, &corr_store);
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
    stats.registry_slugs = registry_hits;
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
            stats.proton_slugs += 1;
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
    // real Windows user home inside the prefix. The native filesystem
    // heuristic, pointed at the prefix's `drive_c/`. Two prefix sources qualify:
    //   * Generic prefixes (plain `wine` / PlayOnLinux / `.desktop`), which
    //     aren't owned by any launcher.
    //   * Proton prefixes whose appid has NO catalog match, i.e. "non-Steam
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
                        // Grade the hit's contents so a verified save-like
                        // archive (Factorio's zipped saves) corroborates `High`
                        // instead of being stuck at the `Medium` floor.
                        merge_fs_hit_graded(&mut by_slug, slug, display_name, hits, &corr_store);
                    }
                }
                Err(e) => tracing::warn!(error = %e, "generic-prefix scan task panicked"),
            }
        }
        stats.generic_prefix_slugs = generic_hits;
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
                let shields = crate::savefilter::shields_for_slug(&entry.slug);
                let mut hits: Vec<PathBuf> = Vec::new();
                let mut seen: HashSet<PathBuf> = HashSet::new();
                for ud in &user_dirs {
                    let Some(dir) = steam_cloud_dir_for(ud, app.app_id, &shields) else {
                        continue;
                    };
                    if seen.insert(dir.clone()) {
                        hits.push(dir);
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
            stats.steam_cloud_slugs = cloud_hits;
            if cloud_hits > 0 {
                tracing::info!(slugs = cloud_hits, "Steam Cloud remote dirs merged");
            }
        }
    }

    // Steam-emulator / repack wrappers: `<APPDATA>/GSE Saves/<appid>/remote`
    // and friends. The subfolder IS the Steam appid, so the game resolves
    // against the catalog with its real name and cover instead of whatever
    // the generic walk guessed. That guess is where "GSE Saves tracked under
    // the Windows username" came from. On Linux the same repacks run under
    // Proton, so the prefixes get the same treatment.
    {
        let mut hits: Vec<wrappers::WrapperHit> = wrappers::discover_wrappers(os);
        if os == Os::Linux {
            let prefixes = if deep {
                wine_prefixes::list_wine_prefixes_deep(os)
            } else {
                wine_prefixes::list_wine_prefixes(os)
            };
            for p in &prefixes {
                for user in roots::prefix_windows_users(&p.prefix_root) {
                    hits.extend(wrappers::discover_wrappers_in_prefix(&p.prefix_root, &user));
                }
            }
        }
        let mut merged = 0usize;
        for hit in hits {
            // An appid resolves to the catalog entry; without one, the folder
            // name is the best label available (it's usually the game's).
            let (slug, display_name) = match hit.app_id.and_then(ludusavi::find_by_steam_app_id) {
                Some(entry) => (entry.slug.clone(), entry.display_name.clone()),
                None => {
                    let name = hit
                        .app_id
                        .and_then(ludusavi::title_for_app_id)
                        .map(str::to_string)
                        .unwrap_or_else(|| hit.folder.clone());
                    // Wrappers are organised by appid, so when the catalogue does
                    // not know that appid the "folder name" IS the appid: that is
                    // how the `2059170` and `2479090` saves were born. A number
                    // does not name a game and says nothing to the next machine.
                    if segment_names_no_game(&name) {
                        tracing::debug!(
                            wrapper = hit.wrapper,
                            folder = %name,
                            path = %hit.path.display(),
                            "detect: wrapper entry has no name of its own, skipped"
                        );
                        continue;
                    }
                    let slug = ludusavi::slugify(&name);
                    if slug.is_empty() {
                        continue;
                    }
                    (slug, name)
                }
            };
            merged += 1;
            tracing::debug!(
                wrapper = hit.wrapper,
                slug = %slug,
                path = %hit.path.display(),
                "repack wrapper save merged"
            );
            merge_fs_hit_graded(
                &mut by_slug,
                slug,
                display_name,
                vec![hit.path],
                &corr_store,
            );
        }
        stats.wrapper_slugs = merged;
        if merged > 0 {
            tracing::info!(slugs = merged, "repack/emulator wrapper saves merged");
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
    // One pass over the standard roots for the whole loop: the time cap becomes
    // per pass rather than per game. Lazy, because the good case is that no game is
    // left unresolved, and there the index is never looked at: building it anyway
    // is seven roots at `NAME_LOOKUP_TIMEOUT` each, up to 2.8 s on a cold disk, on
    // detection's critical path, for nothing.
    let host_dirs = OnceCell::new();
    let mut prefix_indexes: HashMap<PathBuf, NamedDirs> = HashMap::new();
    for slug in unresolved_slugs {
        let (install_dir, prefix_root, display_name) = {
            let g = &by_slug[&slug];
            let install_dir = g.install_dir.clone();
            let prefix_root = prefix_root_by_slug.get(&slug).cloned();
            (install_dir, prefix_root, g.display_name.clone())
        };
        // Three attempts, from the most exact to the most expensive, and the order
        // is the fix: the other way round, a game whose catalogue path did not
        // resolve ended up offering a folder inside the installation (3.6 GB in the
        // case that exposed this) with the right one a single `read_dir` away in
        // `LocalLow`.
        //
        // It runs even with no `install_dir` and no prefix: a non-Steam game has
        // neither, and until now it got no fallback at all (the `continue` below
        // discarded it before anything was looked at).
        let shields = crate::savefilter::shields_for_slug(&slug);

        // 1. The installDir: an exact string Valve wrote, resolved with one `stat`
        //    per root rather than a budgeted sweep. A game whose install folder has
        //    a code name (one title installs into `prj_juniper`) looks nothing like
        //    its commercial name, so no name search finds it.
        let mut discoveries = discover_by_install_dir_name(
            os,
            &slug,
            install_dir.as_deref(),
            prefix_root.as_deref(),
            &shields,
        );
        if !discoveries.is_empty() {
            tracing::debug!(
                slug = %slug,
                hits = discoveries.len(),
                "detection: found the save folder by the game's install-dir name"
            );
        }
        // 2. By name in the roots where saves really live. The index is only built
        //    when it is needed: the host's roots once, the prefix's once per prefix,
        //    since several games share one and it must not be rescanned.
        if discoveries.is_empty() {
            let extra_names: Vec<String> = install_dir
                .as_deref()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .map(|n| vec![n.to_string()])
                .unwrap_or_default();
            let host = || host_dirs.get_or_init(|| NamedDirs::scan(&roots::user_save_roots(os)));
            let index = match prefix_root.as_deref() {
                Some(prefix) => {
                    if !prefix_indexes.contains_key(prefix) {
                        let mut idx = NamedDirs::scan(&roots::prefix_user_roots(prefix));
                        idx.absorb(host());
                        prefix_indexes.insert(prefix.to_path_buf(), idx);
                    }
                    &prefix_indexes[prefix]
                }
                None => host(),
            };
            discoveries = discover_by_name(index, &display_name, &extra_names, &shields);
            if !discoveries.is_empty() {
                tracing::debug!(
                    slug = %slug,
                    hits = discoveries.len(),
                    "detection: found the save folder by name in a standard root"
                );
            }
        }
        // 3. Barrer el directorio de instalación y el prefijo.
        if discoveries.is_empty() {
            if install_dir.is_none() && prefix_root.is_none() {
                continue;
            }
            discoveries = aggressive_discover_with(
                &slug,
                &display_name,
                install_dir.as_deref(),
                prefix_root.as_deref(),
                AGGRESSIVE_WALK_TIMEOUT,
                AGGRESSIVE_WALK_MAX_DEPTH,
                &corr_store,
            );
        }
        if discoveries.is_empty() {
            continue;
        }
        let max_conf = discoveries
            .iter()
            .map(|d| d.confidence)
            .max_by_key(|c| confidence_rank(*c))
            .unwrap_or(Confidence::Low);
        let hits: Vec<PathBuf> = discoveries.into_iter().map(|d| d.path).collect();
        stats.walker_slugs += 1;
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
    // pass over the broad user save roots, scored WITH the correlation
    // store, surfaces save folders that no catalog/Steam signal claimed
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
                            path_confidences: vec![a.confidence],
                            path_reasons: vec![a.reason],
                            confidence: a.confidence,
                            source: DetectionSource::FilesystemHeuristic,
                            steam_app_id: None,
                            install_dir: None,
                            needs_folder: false,
                            steam_cloud: false,
                        },
                    );
                    new_games += 1;
                }
            }
        }
        stats.phase4_new_games = new_games;
        stats.phase4_merged_paths = merged;
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
    (stats.manual_applied, stats.manual_orphaned) =
        apply_manual_overrides(&state.manual_paths, &mut by_slug);

    // Backfill Steam app ids from the catalog by exact slug. A game found
    // purely by filesystem heuristic (a Wine prefix, a native Linux path, a
    // correlation hit) never carried a Steam appid, so the UI had no capsule to
    // fetch and fell back to the letter tile, which is why Factorio/OpenTTD (and
    // every Wrapple cover, since the only logged playtime was Factorio's) showed
    // no art. The detected slug *is* the Ludusavi slug, so an exact catalog
    // match is unambiguous. Only fills a missing id; an id already resolved by a
    // stronger structural signal (Steam install, appid prefix) is left alone.
    {
        let catalog_by_slug: HashMap<&str, &LudusaviEntry> =
            catalog.iter().map(|e| (e.slug.as_str(), e)).collect();
        for (slug, game) in by_slug.iter_mut() {
            let entry = catalog_by_slug.get(slug.as_str());
            if game.steam_app_id.is_none() {
                if let Some(entry) = entry {
                    game.steam_app_id = entry.steam_app_id;
                }
            }
            // An informational note, nothing more. It is filled in here and never
            // read again: it neither sorts, nor scores, nor changes auto-track. The
            // UI draws it and that is all.
            game.steam_cloud = entry.is_some_and(|e| e.cloud_steam);
        }
    }

    // Grade + rank each game's save paths individually. Different sources can
    // hand the same game wildly different folders (a real `~/Saved Games`
    // tree full of saves next to an almost-empty Steam-Cloud stub) and until
    // now they were merged in arbitrary order, so `found_paths[0]` (what
    // automatic tracking backs up) could be the junk one. Re-score every path
    // and sort strongest-first, keeping `path_confidences` aligned, so the UI
    // can show a grade per folder and auto-track picks the best.
    grade_and_rank_paths(&mut by_slug, &corr_store);

    // Last word on every row: a game with no folder says so, instead of leaving
    // it to be inferred from an empty list. Stamped here, after the offer filter
    // has had its say, so it covers both ways of arriving at nothing, never
    // found, and found-then-rejected. See [`DetectedGame::needs_folder`].
    let mut without_folder = 0usize;
    for g in by_slug.values_mut() {
        g.needs_folder = g.found_paths.is_empty();
        if g.needs_folder {
            without_folder += 1;
        }
    }
    if without_folder > 0 {
        tracing::info!(
            games = without_folder,
            "detection: games installed with no save folder located; they need a folder picked"
        );
    }

    progress(total_tasks, total_tasks);

    let mut games: Vec<DetectedGame> = by_slug.into_values().collect();
    games.sort_by(|a, b| a.display_name.cmp(&b.display_name));

    // P1: for every slug where several folders were in play, ship the choice
    // and its reasons. This is the "why did it pick THIS folder" answer that
    // used to die inside grade_and_rank_paths. Deduped per (slug, path) per
    // process so the exempt channel doesn't fill with the same verdict every
    // 10-minute tick.
    for g in games.iter().filter(|g| g.found_paths.len() > 1) {
        if matches!(g.source, DetectionSource::ManualOverride) {
            continue;
        }
        let reason = g.path_reasons.first().cloned().unwrap_or_default();
        crate::telemetry::ranked_choice(&g.slug, &g.found_paths[0], &reason);
    }

    // P9: evaluate already-tracked folders against the mirror rule. Fixing
    // scoring does NOT heal rows tracked before the fix: run_scan skips
    // tracked slugs entirely, so without this the mirror keeps uploading
    // forever. Read-only: it warns, it never re-points.
    let mirror_warnings = detect_tracked_mirrors(state, &games);
    for w in &mirror_warnings {
        tracing::warn!(
            slug = %w.game_slug,
            save_id = %w.save_id,
            tracked = %w.tracked_path.display(),
            suggested = %w.suggested_path.display(),
            "tracked folder looks like the game's own backup mirror; suggest re-pointing"
        );
        crate::telemetry::tracked_mirror(
            &w.game_slug,
            &w.save_id,
            &w.tracked_path,
            &w.suggested_path,
        );
    }

    stats.duration_ms = wall.elapsed().as_millis() as u64;
    tracing::info!(
        detected = games.len(),
        catalog_size,
        steam_apps = steam_apps.len(),
        duration_ms = stats.duration_ms,
        stats = ?stats,
        "Detection complete"
    );

    Ok(DetectionReport {
        games,
        catalog_size,
        steam_apps_found: steam_apps.len(),
        scanned_at_ms: started,
        stats,
        mirror_warnings,
    })
}

/// Directory names under a Steam library that are Steam's own plumbing, so
/// a game folder scan doesn't offer them as install candidates.
const LIB_SYSTEM_DIRS: &[&str] = &[
    "steamapps",
    "steam",
    "userdata",
    "config",
    "appcache",
    "logs",
    "dumps",
    "bin",
    "package",
    "public",
    "clientui",
    "controller_base",
    "music",
    "compatibilitytools.d",
];

/// Every install folder on this host, keyed by lowercase folder name.
///
/// This is what resolves `<base>`: a manifest entry says "I install into a
/// folder called `ELDEN RING`", and this map says where that folder actually
/// is. Built once per scan from two sources, cheapest first:
///
/// 1. the parsed Steam appmanifests (exact, no extra I/O, since we already have
///    them), and
/// 2. one `read_dir` of each library's `steamapps/common` **and** of the
///    library root itself, which is where portable and repack installs sit
///    (`D:\Games\<Game>` next to `D:\steamapps`).
///
/// Listing beats stat-per-candidate by a wide margin here: ~19k catalog
/// entries carry an `installDir` name, and probing each against every
/// library would be tens of thousands of syscalls for a handful of hits.
fn install_dir_index(os: Os, steam_apps: &[SteamApp]) -> HashMap<String, PathBuf> {
    let mut out: HashMap<String, PathBuf> = HashMap::new();
    let mut add = |path: PathBuf| {
        if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
            out.entry(name.to_lowercase()).or_insert(path);
        }
    };
    // Installed Steam games win: the appmanifest is authoritative.
    for app in steam_apps {
        add(app.install_dir.clone());
    }
    for lib in steam::detect_steam_libraries(os) {
        for dir in [lib.join("steamapps").join("common"), lib.clone()] {
            let Ok(read) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in read.flatten() {
                let Ok(ft) = entry.file_type() else { continue };
                if !ft.is_dir() {
                    continue;
                }
                let name = entry.file_name();
                let Some(name) = name.to_str() else { continue };
                if LIB_SYSTEM_DIRS.contains(&name.to_lowercase().as_str()) {
                    continue;
                }
                add(entry.path());
            }
        }
    }
    out
}

/// The `<base>` / `<root>` candidates for one catalog entry.
fn scope_for(
    entry: &LudusaviEntry,
    index: &HashMap<String, PathBuf>,
    store_roots: &[PathBuf],
    steam_install: Option<&Path>,
) -> pathexpand::PathScope {
    let mut install_dirs: Vec<PathBuf> = Vec::new();
    // The game's own Steam install dir is the most reliable answer.
    if let Some(p) = steam_install {
        install_dirs.push(p.to_path_buf());
    }
    for name in &entry.install_dirs {
        if let Some(p) = index.get(&name.to_lowercase()) {
            if !install_dirs.contains(p) {
                install_dirs.push(p.clone());
            }
        }
    }
    pathexpand::PathScope {
        install_dirs,
        store_roots: store_roots.to_vec(),
    }
}

/// Pull the list of save-path template strings that apply to the requested
/// OS for a single Ludusavi entry. Strips constraints and tags: detection only
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
///
/// Returns `(exact_added, fuzzy_added)` for the scan stats.
fn apply_steam_name_fallback(
    catalog: &[LudusaviEntry],
    steam_apps: &[SteamApp],
    by_slug: &mut HashMap<String, DetectedGame>,
) -> (usize, usize) {
    if steam_apps.is_empty() {
        return (0, 0);
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
                path_confidences: Vec::new(),
                path_reasons: Vec::new(),
                confidence: Confidence::Low,
                source: DetectionSource::SteamLibrary,
                steam_app_id: Some(app.app_id),
                install_dir: Some(app.install_dir.clone()),
                needs_folder: false,
                steam_cloud: false,
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
    (added, fuzzy_added)
}

/// Threshold for `find_by_fuzzy_name` in [`apply_steam_name_fallback`].
/// 0.15 is about one edit per seven characters, enough slack for typos and minor
/// suffix noise. Note the threshold alone would NOT stop cross-sequel
/// matches ("civilization-v" vs "civilization-vi" ≈ 0.07 sits well inside
/// it); the numeral veto in `fuzzy_match_in` is what rejects those.
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
/// inserted: surfacing every random launcher app would surface launcher
/// tooling and non-game executables. The walker still benefits from the
/// install_dir attached to matched rows.
///
/// Returns `(exact_added, fuzzy_added)` for the scan stats.
fn apply_launcher_name_fallback(
    catalog: &[LudusaviEntry],
    apps: &[LauncherApp],
    launcher_tag: &str,
    by_slug: &mut HashMap<String, DetectedGame>,
) -> (usize, usize) {
    if apps.is_empty() {
        return (0, 0);
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
                // Already linked by Steam appid or another launcher, so keep
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
                        path_confidences: Vec::new(),
                        path_reasons: Vec::new(),
                        confidence: Confidence::Low,
                        source: DetectionSource::SteamLibrary,
                        steam_app_id: None,
                        install_dir: Some(app.install_dir.clone()),
                        needs_folder: false,
                        steam_cloud: false,
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
    (exact_added, fuzzy_added)
}

/// Build the slug → prefix_root map the aggressive walker consumes.
///
/// Unifies three prefix sources behind a single map keyed by Ludusavi slug:
///   * Proton: looked up via the Steam appid → catalog slug.
///   * Lutris and Bottles: the prefix's identifier is slugified directly
///     and used as the key (best-effort, with no catalog lookup, since the
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
                // Ludusavi doesn't know the appid, so silently skip; the
                // walker can't help without a slug to index against.
                let Ok(appid) = prefix.identifier.parse::<u64>() else {
                    continue;
                };
                let Some(entry) = ludusavi::find_by_steam_app_id(appid) else {
                    continue;
                };
                entry.slug.clone()
            }
            // Lutris names its prefix directory after its own game slug, and
            // the manifest carries that slug (`id.lutris`, 4.1k entries), so
            // it resolves exactly. Slugifying the directory name is the
            // fallback for Bottles, and for the Lutris games the manifest
            // doesn't list, but it only works when the folder happens to be
            // named like the game.
            PrefixKind::Lutris => ludusavi::find_by_lutris_slug(&prefix.identifier)
                .map(|e| e.slug.clone())
                .unwrap_or_else(|| ludusavi::slugify(&prefix.identifier)),
            PrefixKind::Bottles => ludusavi::slugify(&prefix.identifier),
            // Generic prefixes have no per-game identifier: a single prefix
            // holds many games. They're handled by the dedicated generic-prefix
            // scan in `detect_all`, not the slug-keyed aggressive walker.
            PrefixKind::Generic => continue,
        };
        // First writer wins; multiple prefixes for the same slug
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
    // Single choke point for every catalog-driven stage: a template that
    // resolves to a whole profile or a shared engine root is a loose
    // template, not a save. Offering it would have the user back up their
    // Documents folder, or every RenPy game at once.
    let offered = hits.len();
    let hits: Vec<PathBuf> = hits
        .into_iter()
        .filter(|p| {
            let broad = is_too_broad(p);
            if broad {
                tracing::debug!(slug = %slug, path = %p.display(),
                    "dropping candidate: it is a profile/engine root, not one game's save folder");
            }
            !broad
        })
        .collect();
    // Paths were offered and ALL of them were too broad: there is nothing to show
    // and no row to invent either. Note this is not the same as an empty `hits` on
    // the way in, which is the deliberate signal for "I saw the game on disk but
    // not its save folder", and that does create the row with an empty
    // `found_paths` so the UI asks for a folder.
    if offered > 0 && hits.is_empty() && !by_slug.contains_key(&slug) {
        return;
    }
    match by_slug.get_mut(&slug) {
        Some(existing) => {
            // Both signals: the strongest possible match.
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
                    path_confidences: Vec::new(),
                    path_reasons: Vec::new(),
                    confidence: Confidence::Medium,
                    source: DetectionSource::FilesystemHeuristic,
                    steam_app_id: None,
                    install_dir: None,
                    needs_folder: false,
                    steam_cloud: false,
                },
            );
        }
    }
}

/// Merge a filesystem hit, then upgrade the slug to `High` if any hit's content
/// grades `High`. `merge_fs_hit` floors a *new* slug at `Medium` and never runs
/// the scorer; the catalog-template and generic-prefix scans point straight at
/// a save dir, so without this they'd cap at `Medium` even when the content is
/// direct evidence (verified archive index, or a rotating ≥3 strong-ext save
/// set like openttd's `autosave/`). Only ever upgrades: a weak score keeps the
/// `Medium` floor, never downgrades. A slug already present (e.g. Steam
/// cross-ref) is left to `merge_fs_hit`'s Both/High promotion.
fn merge_fs_hit_graded(
    by_slug: &mut HashMap<String, DetectedGame>,
    slug: String,
    display_name: String,
    hits: Vec<PathBuf>,
    corr_store: &CorrelationStore,
) {
    let graded = hits
        .iter()
        .filter_map(|h| {
            let name = h.file_name()?.to_string_lossy().into_owned();
            classify_dir_as_save_like(h, &name, corr_store)
        })
        .map(|d| d.confidence)
        .max_by_key(|c| confidence_rank(*c));
    let existed = by_slug.contains_key(&slug);
    merge_fs_hit(by_slug, slug.clone(), display_name, hits);
    if !existed && graded == Some(Confidence::High) {
        if let Some(g) = by_slug.get_mut(&slug) {
            g.confidence = Confidence::High;
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
///      case-insensitive), keep it as-is: it already points at a save dir.
///    * Else, list its immediate subdirectories and keep the ones whose
///      name matches [`SAVE_PATTERNS`]. Zero matches drops the hit so the
///      UI falls back to the amber "pick folder" alert; one or more matches
///      replace the hit with the matched subdirs.
///    * Zero matches but the hit turns out to be a **folder with one
///      subfolder per save** ([`is_nest_of_save_dirs`]) keeps the hit whole:
///      there the catalog was already pointing at the right place.
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
        // The candidate is a FILE. Over 4,900 games in the catalogue only have
        // templates like that (`<winAppData>/Game/save.dat`, `<base>/140.sav`) and
        // until now they were lost whole: refinement looked for a save subfolder,
        // did not find one (a file has no subfolders) and threw it away, leaving the
        // game with the amber "pick a folder" alert.
        //
        // The folder containing it is preferred, since that is what the user expects
        // to back up and it groups the sibling saves. Only when that folder is too
        // broad to offer (the profile, Documents, the game's install root) is the
        // lone file tracked.
        //
        // …or when the folder keeps mods, Workshop or cache next to the save
        // (`junkdirs::holds_foreign_subdir`). That does not make it broad, since it
        // belongs to the game, and `is_too_broad` approves it, but it does make
        // it the GAME's folder rather than its saves' folder, and adopting it
        // whole uploads hundreds of megabytes of content nobody asked for.
        // Issue #17: Teardown's save is a `savegame.xml` of a few KB, and its
        // folder dragged along 42 MB of `mods\` across 173 files. There we
        // track the lone file, which is what the catalog named.
        if hit.is_file() {
            let candidate = match hit.parent() {
                Some(parent)
                    if !parent.as_os_str().is_empty()
                        && !is_too_broad(parent)
                        && !junkdirs::holds_foreign_subdir(parent) =>
                {
                    parent.to_path_buf()
                }
                _ => hit.clone(),
            };
            if !refined.contains(&candidate) {
                refined.push(candidate);
            }
            continue;
        }
        // Both "keep the hit whole" branches below are gated on this. A
        // template that resolves to a whole profile root and *also* has "save"
        // in its last segment (`.../Saved Games` on its own, and inside a Proton
        // prefix especially, where `is_too_broad` alone is blind) was kept
        // whole and offered as one game's folder. Refining into its
        // subdirectories is still fine, and is what the rest of the loop does.
        let keep_whole = !never_offer_whole(&hit);
        if keep_whole && segment_matches_save_pattern(&hit) {
            if !refined.contains(&hit) {
                refined.push(hit);
            }
            continue;
        }
        let subdirs_found = find_save_subdirs(&hit);
        if keep_whole
            && subdirs_found.is_empty()
            && !dir_is_empty(&hit)
            && (hit_name_suggests_saves(&hit) || is_nest_of_save_dirs(&hit))
        {
            // The catalogue pointed HERE and the folder's name says so
            // ("SavedArksLocal", "SaveData"), but it is not one of
            // `SAVE_PATTERNS`' exact spellings and there is no save-named subfolder
            // inside to refine down to. Throwing it away lost the hit: on one
            // user's Windows, ARK's `<base>/ShooterGame/Saved/SavedArksLocal`
            // existed with saves in it and came out as an amber alert. A folder
            // with several ambiguous subfolders (`profiles/`, `settings/`) does not
            // meet the bar and still gives the amber alert, which is right: there we
            // do not know which subfolder it is.
            //
            // The other shape kept whole is the NEST: the folder holds no
            // saves of its own but one subfolder per save
            // (`.../Cyberpunk 2077/AutoSave-0/sav.dat`). There is no ambiguity
            // there either, since everything inside belongs to the same game, and
            // dropping it left the game with no path at all even though the
            // catalog had pointed at exactly the right place. See
            // [`is_nest_of_save_dirs`].
            if !refined.contains(&hit) {
                refined.push(hit);
            }
            continue;
        }
        let mut subdirs = subdirs_found;
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
    // P2 (the incident's veto): a refined entry that is another entry's backup
    // mirror (name-plus-suffix AND a content superset) is the game's own
    // rotating copy, never the save. Dropped outright here so it can't reach
    // `found_paths`, get probed for correlation, or lead auto-track. The
    // superset condition is what makes the veto safe: without it a `-bak`
    // folder that merely sits next to a save would be condemned by its name.
    let mirrors: Vec<bool> = refined
        .iter()
        .map(|c| refined.iter().any(|o| o != c && is_backup_mirror(c, o)))
        .collect();
    let mut kept: Vec<PathBuf> = Vec::with_capacity(refined.len());
    for (cand, is_mirror) in refined.into_iter().zip(mirrors) {
        if is_mirror {
            tracing::debug!(
                path = %cand.display(),
                "dropping candidate: it is the game's own backup mirror of another candidate"
            );
            continue;
        }
        if !kept.contains(&cand) {
            kept.push(cand);
        }
    }
    kept
}

/// The last segment *talks about* saves without being one of the exact spellings:
/// `SavedArksLocal`, `SaveData`, `save_games`. Looser than
/// [`name_matches_save_pattern`] on purpose, and only used to decide whether to
/// keep a hit the catalogue already pointed at, never to invent candidates out of
/// nothing.
fn hit_name_suggests_saves(path: &Path) -> bool {
    path.file_name()
        .and_then(|s| s.to_str())
        .is_some_and(junkdirs::looks_like_save_dir_name)
}

fn dir_is_empty(path: &Path) -> bool {
    std::fs::read_dir(path).is_ok_and(|mut r| r.next().is_none())
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
/// * Slug already in `by_slug` (heuristic found something): the chosen path
///   moves to the **front** of `found_paths` with `High`, and the heuristic's
///   hits stay behind it. `steam_app_id`/`install_dir` are kept from the
///   existing entry so the UI still shows the Steam hint.
///
///   Leading rather than replacing is what fixes the aug-2026 Factorio case:
///   pointing at a folder by hand, to add it as a second one or just to try
///   it, left the card showing **only** that folder, and the game's real save
///   folder vanished from the list with nothing to say it was still there. The
///   manual path already wins by going first (it is what automatic tracking
///   picks and what the UI proposes); wiping the rest added nothing and hid the
///   good one.
/// * Slug not in `by_slug` but present in the catalog: synthesise a fresh
///   row from the catalog's display name. Covers the "user added a game
///   the heuristic would never find" case.
/// * Slug not in the catalog: log WARN and leave the override on disk. The
///   override stays in `state.json` so the user can clean it up; we don't
///   silently drop it because the catalog can grow back into the slug
///   (the desktop refreshes it daily in the background, see `catalog.rs`)
///   and we don't want the user's manual work discarded between sessions.
fn apply_manual_overrides(
    manual_paths: &HashMap<String, PathBuf>,
    by_slug: &mut HashMap<String, DetectedGame>,
) -> (usize, usize) {
    if manual_paths.is_empty() {
        return (0, 0);
    }
    let catalog = ludusavi::catalog();
    let mut applied = 0usize;
    let mut orphaned = 0usize;
    for (slug, path) in manual_paths {
        if let Some(existing) = by_slug.get_mut(slug) {
            promote_manual_path(existing, path);
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
                    path_confidences: vec![Confidence::High],
                    path_reasons: vec![String::new()],
                    confidence: Confidence::High,
                    source: DetectionSource::ManualOverride,
                    steam_app_id: entry.steam_app_id,
                    install_dir: None,
                    needs_folder: false,
                    steam_cloud: false,
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
    (applied, orphaned)
}

/// Puts `path` first in `found_paths` with `High` confidence and leaves the rest
/// behind it. A path already in the list is moved to the front rather than
/// duplicated.
///
/// `path_confidences` runs 1:1 with `found_paths` (the UI relies on it to grade
/// each folder and automatic tracking to keep the best one) so it is reordered
/// alongside. It gets resized first in case the entry came from a cached report
/// written before that field existed.
fn promote_manual_path(game: &mut DetectedGame, path: &Path) {
    game.path_confidences
        .resize(game.found_paths.len(), game.confidence);
    if let Some(i) = game.found_paths.iter().position(|p| same_path(p, path)) {
        game.found_paths.remove(i);
        game.path_confidences.remove(i);
    }
    game.found_paths.insert(0, path.to_path_buf());
    game.path_confidences.insert(0, Confidence::High);
}

/// The exact same folder, Windows casing accounted for. Unlike
/// [`paths_overlap`], which also counts nesting: all this needs is to avoid
/// listing one path twice.
fn same_path(a: &Path, b: &Path) -> bool {
    if cfg!(windows) {
        a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
    } else {
        a == b
    }
}

/// One leg of a [`DetectionTrace`]: the result of a single pipeline step
/// for the slug being diagnosed. `kind` names the step
/// (`"manual_override"`, `"steam_appid"`, `"name_fallback"`,
/// `"launcher_fallback"`, `"filesystem"`, `"registry"`, `"proton_prefix"`,
/// `"generic_prefix"`, `"steam_cloud"`, `"wrapper"`, `"refine"`,
/// `"aggressive_walk"`, `"correlation"`).
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
/// didn't show up: "path doesn't exist", "expand_path returned nothing",
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
/// the desktop app's hidden diagnostics panel: the answer to "why
/// doesn't this game show up in my library?" is now mechanical.
///
/// The real [`detect_all`] is untouched: this is a parallel implementation
/// that hits the same primitives ([`expand_path`],
/// [`expand_path_in_prefix_as_user`], [`refine_save_dir`],
/// [`aggressive_discover_with`]) but writes traces. Every stage of the real
/// pipeline has a step here; the only structural difference is phase 4
/// (catalog-free discovery), which is global rather than per-slug, so the
/// `correlation` step covers its per-slug signal by listing which observed
/// dirs the store attributes to this slug. If you add a stage to
/// [`detect_all_inner`], add its step here in the same order; the
/// integration tests in `tests/detection_integration.rs` (P-DET-7) guard
/// against behavioural drift in the shared primitives.
pub async fn diagnose(slug: &str, os: Os, state: &CliState) -> DetectionTrace {
    let mut attempts: Vec<TraceStep> = Vec::new();

    // ---- Step 1: manual_override ------------------------------------
    // Recorded first because a manual path beats every heuristic: if
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
    // A slug not in the catalog short-circuits the rest of the trace,
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
    // Same store the real pipeline loads: feeds the walk step's grading and
    // the final `correlation` step.
    let corr_store = CorrelationStore::default_path()
        .ok()
        .map(|p| CorrelationStore::load(&p))
        .unwrap_or_default();
    // Install dir recovered from whichever launcher signal matches; the
    // aggressive-walk step needs it, exactly like the real pipeline reads it
    // off the merged row.
    let mut install_dir_hint: Option<PathBuf> = None;
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
            install_dir_hint = Some(app.install_dir.clone());
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
                if install_dir_hint.is_none() {
                    install_dir_hint = Some(app.install_dir.clone());
                }
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

    // ---- Step 4: launcher_fallback ----------------------------------
    // Epic / GOG / Microsoft Store cross-reference. Exact slug match only,
    // the real pipeline also fuzzy-matches, but the exact miss is already
    // the answer the user needs ("your launcher spells the name differently").
    let mut launcher_step = TraceStep {
        kind: "launcher_fallback".into(),
        template: None,
        expanded: Vec::new(),
        kept: Vec::new(),
        dropped: Vec::new(),
    };
    let launcher_apps = [
        ("epic", launchers::list_installed_epic_games(os)),
        ("gog", launchers::list_installed_gog_games(os)),
        ("msstore", launchers::list_installed_msstore_games(os)),
    ];
    let mut any_launcher_app = false;
    for (tag, apps) in &launcher_apps {
        for app in apps {
            any_launcher_app = true;
            let slugified = ludusavi::slugify(&app.name);
            launcher_step.expanded.push(format!("{tag}: {slugified}"));
            if slugified == slug {
                launcher_step.template = Some(format!("slugify({:?}) [{tag}]", app.name));
                launcher_step
                    .kept
                    .push(app.install_dir.display().to_string());
                if install_dir_hint.is_none() {
                    install_dir_hint = Some(app.install_dir.clone());
                }
            }
        }
    }
    if any_launcher_app && launcher_step.kept.is_empty() {
        launcher_step.dropped.push(DroppedPath {
            path: slug.into(),
            reason: "no Epic/GOG/MS Store app name slugifies to this slug".into(),
        });
    }
    attempts.push(launcher_step);

    // ---- Step 5: filesystem -----------------------------------------
    // One step per save-path template that applies to the current OS.
    // Collects raw hits to feed into the refinement step below.
    let templates = paths_for_os(entry, os);
    // The same `<base>` and `<root>` resolution the real pass uses. A diagnostic
    // that expanded templates differently would describe a pipeline that
    // doesn't exist.
    let diag_scope = scope_for(
        entry,
        &install_dir_index(os, &steam_apps),
        &steam::detect_steam_libraries(os),
        install_dir_hint.as_deref(),
    );
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
        let candidates = expand_path_scoped(tmpl, os, &diag_scope);
        if candidates.is_empty() {
            step.dropped.push(DroppedPath {
                path: tmpl.clone(),
                reason: if tmpl.starts_with("<base>") && diag_scope.install_dirs.is_empty() {
                    "template is relative to the install dir (<base>) and the game's \
                     install folder wasn't found on this machine"
                        .into()
                } else {
                    "expand_path produced no candidates (unknown placeholder or unset env)"
                        .to_string()
                },
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

    // ---- Step 6: registry -------------------------------------------
    // Catalog `registry` keys point at HKEY_* values holding the save dir.
    // Hits feed the same refinement as template hits, mirroring the real
    // pipeline. On non-Windows `expand_registry_path` returns nothing.
    for reg in &entry.registry {
        let mut step = TraceStep {
            kind: "registry".into(),
            template: Some(reg.key.clone()),
            expanded: Vec::new(),
            kept: Vec::new(),
            dropped: Vec::new(),
        };
        let candidates = expand_registry_path(reg);
        if candidates.is_empty() {
            step.dropped.push(DroppedPath {
                path: reg.key.clone(),
                reason: "expand_registry_path produced no candidates \
                         (non-Windows host, or key/value missing)"
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
                    reason: "registry value points at a path that doesn't exist".into(),
                });
            }
        }
        attempts.push(step);
    }

    // ---- Step 7: proton_prefix (Linux only) -------------------------
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

    // Paths that the real pipeline merges directly, without refinement
    // (generic-prefix and Steam Cloud hits). Tracked so the walk step below
    // can honour the same "only walk when nothing found a path" gate.
    let mut merged_direct: Vec<PathBuf> = Vec::new();

    // ---- Step 8: generic_prefix (Linux only) ------------------------
    // Wine prefixes not owned by a catalog-resolvable Proton appid: expand
    // this entry's Windows templates against every real Windows user inside
    // each prefix, exactly like the whole-catalog cross-reference does.
    if os == Os::Linux && !entry.paths.windows.is_empty() {
        for prefix in wine_prefixes::list_wine_prefixes(os) {
            let qualifies = match prefix.kind {
                PrefixKind::Generic => true,
                PrefixKind::Proton => prefix
                    .identifier
                    .parse::<u64>()
                    .ok()
                    .and_then(ludusavi::find_by_steam_app_id)
                    .is_none(),
                PrefixKind::Lutris | PrefixKind::Bottles => false,
            };
            if !qualifies {
                continue;
            }
            for user in roots::prefix_windows_users(&prefix.prefix_root) {
                let mut step = TraceStep {
                    kind: "generic_prefix".into(),
                    template: Some(format!(
                        "windows templates under {} as user {:?}",
                        prefix.prefix_root.display(),
                        user
                    )),
                    expanded: Vec::new(),
                    kept: Vec::new(),
                    dropped: Vec::new(),
                };
                for tmpl in entry.paths.windows.iter().map(|p| &p.path) {
                    for c in expand_path_in_prefix_as_user(tmpl, &prefix.prefix_root, &user) {
                        step.expanded.push(c.display().to_string());
                        if c.exists() {
                            step.kept.push(c.display().to_string());
                            merged_direct.push(c);
                        } else {
                            step.dropped.push(DroppedPath {
                                path: c.display().to_string(),
                                reason: "path doesn't exist under the Wine prefix".into(),
                            });
                        }
                    }
                }
                attempts.push(step);
            }
        }
    }

    // ---- Step 9: steam_cloud ----------------------------------------
    // `userdata/<storeUserId>/<appid>/remote/`: some titles write their
    // only save there. Merged directly (no refinement) like the pipeline.
    if let Some(appid) = entry.steam_app_id {
        let mut step = TraceStep {
            kind: "steam_cloud".into(),
            template: Some(format!(
                "userdata/<user>/{appid}/remote (or the appid dir itself)"
            )),
            expanded: Vec::new(),
            kept: Vec::new(),
            dropped: Vec::new(),
        };
        let libraries = steam::detect_steam_libraries(os);
        let user_dirs = steam::steam_user_dirs(&libraries).unwrap_or_default();
        if user_dirs.is_empty() {
            step.dropped.push(DroppedPath {
                path: format!("appid={appid}"),
                reason: "no Steam userdata dirs found on this machine".into(),
            });
        }
        let shields = crate::savefilter::shields_for_slug(slug);
        for ud in &user_dirs {
            let app_dir = ud.join(appid.to_string());
            step.expanded
                .push(app_dir.join("remote").display().to_string());
            match steam_cloud_dir_for(ud, appid, &shields) {
                Some(dir) => {
                    step.kept.push(dir.display().to_string());
                    merged_direct.push(dir);
                }
                None => step.dropped.push(DroppedPath {
                    path: app_dir.display().to_string(),
                    reason: "no remote/ dir for this appid under this Steam user, and the appid dir holds no save".into(),
                }),
            }
        }
        attempts.push(step);
    }

    // ---- Step 9b: wrapper --------------------------------------------
    // Steam-emulator / repack containers, keyed by appid, plus the same
    // containers inside every Wine prefix on Linux. Mirrors the pipeline's
    // stage: whatever it finds is already narrowed to the save dir.
    {
        let mut step = TraceStep {
            kind: "wrapper".into(),
            template: Some("<repack wrappers>/<appid>".into()),
            expanded: Vec::new(),
            kept: Vec::new(),
            dropped: Vec::new(),
        };
        let mut hits = wrappers::discover_wrappers(os);
        if os == Os::Linux {
            for p in wine_prefixes::list_wine_prefixes(os) {
                for user in roots::prefix_windows_users(&p.prefix_root) {
                    hits.extend(wrappers::discover_wrappers_in_prefix(&p.prefix_root, &user));
                }
            }
        }
        for hit in hits {
            let hit_slug = match hit.app_id.and_then(ludusavi::find_by_steam_app_id) {
                Some(e) => e.slug.clone(),
                None => ludusavi::slugify(&hit.folder),
            };
            step.expanded.push(hit.path.display().to_string());
            if hit_slug == slug {
                step.kept.push(hit.path.display().to_string());
                merged_direct.push(hit.path);
            } else {
                step.dropped.push(DroppedPath {
                    path: hit.path.display().to_string(),
                    reason: format!("wrapper entry belongs to '{hit_slug}', not this slug"),
                });
            }
        }
        if step.expanded.is_empty() {
            step.dropped.push(DroppedPath {
                path: slug.into(),
                reason: "no repack/emulator wrapper folders on this machine".into(),
            });
        }
        attempts.push(step);
    }

    // ---- Step 10: refine --------------------------------------------
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

    // ---- Step 11: aggressive_walk -----------------------------------
    // Mirrors the pipeline gate: the walker only runs for slugs that ended
    // the main pipeline without a single save path.
    let mut walk_step = TraceStep {
        kind: "aggressive_walk".into(),
        template: None,
        expanded: Vec::new(),
        kept: Vec::new(),
        dropped: Vec::new(),
    };
    if refined.is_empty() && merged_direct.is_empty() {
        let prefix_root = build_prefix_root_by_slug(os).remove(slug);
        walk_step.template = Some(format!(
            "install_dir={install_dir_hint:?}, prefix_root={prefix_root:?}"
        ));
        if install_dir_hint.is_none() && prefix_root.is_none() {
            walk_step.dropped.push(DroppedPath {
                path: slug.into(),
                reason: "nothing to walk: no install dir hint and no Wine/Proton prefix \
                         resolved for this slug"
                    .into(),
            });
        } else {
            let discoveries = aggressive_discover_with(
                slug,
                &entry.display_name,
                install_dir_hint.as_deref(),
                prefix_root.as_deref(),
                AGGRESSIVE_WALK_TIMEOUT,
                AGGRESSIVE_WALK_MAX_DEPTH,
                &corr_store,
            );
            if discoveries.is_empty() {
                walk_step.dropped.push(DroppedPath {
                    path: slug.into(),
                    reason: "walk found no save-like dirs under the available roots".into(),
                });
            }
            for d in discoveries {
                walk_step
                    .kept
                    .push(format!("{} ({})", d.path.display(), d.reason));
            }
        }
    } else {
        walk_step.template = Some("skipped: earlier stages already produced save paths".into());
    }
    attempts.push(walk_step);

    // ---- Step 12: correlation (phase-4 signal) ------------------------
    // Phase 4 proper is catalog-free and global, so it can't be replayed for
    // one slug; what CAN be shown is its per-slug input, every observed
    // process↔write whose attributed name slugifies to this slug.
    let mut corr_step = TraceStep {
        kind: "correlation".into(),
        template: Some(format!("{} observed dirs in the store", corr_store.len())),
        expanded: Vec::new(),
        kept: Vec::new(),
        dropped: Vec::new(),
    };
    for (dir, obs) in corr_store.iter() {
        let attributed = corr_store.attributed_name(dir).unwrap_or_default();
        if ludusavi::slugify(&attributed) == slug {
            corr_step.kept.push(format!(
                "{} (process {:?}, {} hits)",
                dir.display(),
                obs.process_name,
                obs.hits
            ));
        }
    }
    if corr_step.kept.is_empty() {
        corr_step.dropped.push(DroppedPath {
            path: slug.into(),
            reason: "no observed write is attributed to this slug (game never seen \
                     running while a watched dir changed)"
                .into(),
        });
    }
    attempts.push(corr_step);

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
///
/// ADR 0020 asks for this cap to go, because the real quality gate is now
/// `scoring::score_dir`'s score (only folders with real evidence cross the
/// threshold). It was raised from 5 to 16 rather than removed outright: it still
/// acts as a seatbelt against a pathological tree, but it no longer trims
/// legitimate candidates down to the first five.
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
/// purpose: `.cfg` and `.ini` would generate too many false positives from
/// engine config dirs.
pub(crate) const SAVE_FILE_EXTENSIONS: &[&str] = &["sav", "save", "profile", "json", "dat", "xml"];

/// How recent a save-like file has to be to promote a dir to `Medium`.
///
/// Raised from 90 to 180 days. 90 left out saves from games played "last season";
/// 180 recovers them and still discards shipped data, which almost always carries
/// the install's mtime, older than half a year as soon as the game has been
/// installed a while.
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

/// How long the name lookup may spend on a single root, and how many
/// directories it may look at there. It reads one level and one level below it,
/// so the ceiling matters mostly in `LocalLow`, where a busy machine has a few
/// dozen publisher folders.
const NAME_LOOKUP_TIMEOUT: Duration = Duration::from_millis(400);
const NAME_LOOKUP_MAX_DIRS: usize = 400;

/// Normalise a folder or game name for comparison: lowercase, and drop
/// everything that is not a letter or a digit.
///
/// `Hell Maiden`, `HellMaiden` and `hell-maiden` all have to compare equal, so
/// studios name the folder however they please, and the catalogue's
/// `display_name` is the only thing we can hold it against.
fn name_key(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Does this directory hold anything a player would miss?
///
/// The gate that makes the name lookup safe to trust. `AppData/LocalLow/<Studio>/<Game>`
/// exists for **every** Unity game whether or not it saves there (the engine
/// drops `Player.log` and `Unity/<guid>/Analytics/` in it regardless) so
/// matching on the name alone would confidently recommend a folder of logs. It
/// is exactly the folder a user picked by hand when Hoard left them to guess,
/// and then "nothing to back up" was all they got (ago-2026).
///
/// `fileclass` already knows the difference and is the same judgement the
/// backup itself makes, so a folder that passes here cannot produce an empty
/// snapshot. Bounded: two levels and a handful of entries is enough to tell a
/// save folder from a log folder.
fn holds_player_data(dir: &Path, shields: &[String]) -> bool {
    fn scan(dir: &Path, shields: &[String], depth: usize, budget: &mut usize) -> bool {
        if depth > 2 || *budget == 0 {
            return false;
        }
        let Ok(read) = std::fs::read_dir(dir) else {
            return false;
        };
        let mut subdirs: Vec<PathBuf> = Vec::new();
        for entry in read.flatten() {
            if *budget == 0 {
                return false;
            }
            *budget -= 1;
            let Ok(ft) = entry.file_type() else { continue };
            let path = entry.path();
            if ft.is_dir() {
                subdirs.push(path);
                continue;
            }
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            if fileclass::classify(name, shields).is_backed_up() {
                return true;
            }
        }
        subdirs.iter().any(|d| scan(d, shields, depth + 1, budget))
    }
    let mut budget = 200usize;
    scan(dir, shields, 0, &mut budget)
}

/// How deep the offer filter looks inside a candidate folder, and how many
/// entries it is allowed to touch. Generous compared with
/// [`holds_player_data`]'s two levels, because this walk **rejects**: running
/// out of road has to mean "don't know", and the fewer folders end up there the
/// better the filter works.
const OFFER_SCAN_MAX_DEPTH: usize = 3;
const OFFER_SCAN_BUDGET: usize = 400;

/// What a candidate folder holds, as far as deciding whether to offer it goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FolderContents {
    /// At least one file inside is player data. Offer it.
    SaveData,
    /// Files inside, and [`fileclass`] says not one of them is player data:
    /// settings, logs, engine bookkeeping. Not a save folder.
    NoSaveData,
    /// No files at all. The game is installed and has not written a save yet,
    /// which is a real state, not a mistake.
    Empty,
    /// The walk ran out of depth or budget before it could say. Never used to
    /// reject: a save that lives four levels down is not evidence of absence.
    Unknown,
}

/// Look inside a candidate folder and say what it holds.
///
/// The counterpart of [`holds_player_data`], and deliberately stricter: that one
/// asks "is there anything worth backing up here", which a folder holding one
/// `settings.ini` passes: config is uploaded so it is never lost, so it counts
/// as backup-worthy. Offering is a different question. `~/.config/SiNKR` holds
/// exactly one `settings.ini` and the catalog points at it, so it was offered
/// next to the folder that holds the actual saves, and the decoy scenario of an
/// external measurement pass turned up twenty more of the same shape.
///
/// Only [`fileclass::FileClass::SaveData`] counts here, and the manifest's own
/// file patterns come in as `shields` so a game whose saves really are `.ini`
/// (582 catalog templates say `*.ini`) is not judged by extension alone.
fn inspect_folder(dir: &Path, shields: &[String]) -> FolderContents {
    struct Walk<'a> {
        shields: &'a [String],
        budget: usize,
        saw_file: bool,
        truncated: bool,
    }

    impl Walk<'_> {
        /// `rel` is the path so far **relative to the candidate root**, with
        /// `/` separators, the shape [`fileclass::classify`] expects. Passing
        /// only the file name would blind its segment rules, and those are what
        /// recognise the Unity analytics queue and the engine telemetry dirs
        /// that make up most of a false offer's contents.
        fn scan(&mut self, dir: &Path, rel: &str, depth: usize) -> bool {
            if depth > OFFER_SCAN_MAX_DEPTH {
                self.truncated = true;
                return false;
            }
            let Ok(read) = std::fs::read_dir(dir) else {
                // Unreadable is not empty and not junk: it is unknown.
                self.truncated = true;
                return false;
            };
            let mut subdirs: Vec<(PathBuf, String)> = Vec::new();
            for entry in read.flatten() {
                if self.budget == 0 {
                    self.truncated = true;
                    return false;
                }
                self.budget -= 1;
                let Ok(ft) = entry.file_type() else { continue };
                let name = entry.file_name();
                let Some(name) = name.to_str() else { continue };
                let child = if rel.is_empty() {
                    name.to_string()
                } else {
                    format!("{rel}/{name}")
                };
                if ft.is_dir() {
                    subdirs.push((entry.path(), child));
                    continue;
                }
                self.saw_file = true;
                if fileclass::classify(&child, self.shields) == fileclass::FileClass::SaveData {
                    return true;
                }
            }
            for (path, child) in &subdirs {
                if self.scan(path, child, depth + 1) {
                    return true;
                }
            }
            false
        }
    }

    let mut walk = Walk {
        shields,
        budget: OFFER_SCAN_BUDGET,
        saw_file: false,
        truncated: false,
    };
    if walk.scan(dir, "", 0) {
        return FolderContents::SaveData;
    }
    let (saw_file, truncated) = (walk.saw_file, walk.truncated);
    if truncated {
        return FolderContents::Unknown;
    }
    if saw_file {
        FolderContents::NoSaveData
    } else {
        FolderContents::Empty
    }
}

/// Drop the candidate folders that hold no player data, and report which of the
/// survivors are empty.
///
/// Two outcomes and two different answers, which is the whole point of telling
/// them apart:
///
/// * **Files inside, none of them a save.** Dropped. There is nothing to back
///   up there and there never was; offering it is how a user ends up tracking a
///   settings folder and getting "nothing to back up" for their trouble.
/// * **No files at all.** Kept, but never above `Low`. The game is installed
///   and has not been played, and the folder it will save into is genuinely
///   useful to show: hiding it would mean a freshly installed game looks
///   undetected. `Low` is also what keeps automatic tracking off it, alongside
///   the empty-folder check auto-track already makes.
///
/// A folder the walk could not finish reading is kept as-is: the filter only
/// ever removes something it has seen all of.
fn drop_folders_without_saves(g: &mut DetectedGame) -> HashSet<PathBuf> {
    let shields = crate::savefilter::shields_for_slug(&g.slug);
    let mut empty: HashSet<PathBuf> = HashSet::new();
    let mut kept: Vec<PathBuf> = Vec::with_capacity(g.found_paths.len());
    for path in std::mem::take(&mut g.found_paths) {
        match inspect_folder(&path, &shields) {
            FolderContents::NoSaveData => {
                tracing::debug!(
                    slug = %g.slug,
                    path = %path.display(),
                    "offer filter: nothing inside is player data; not offering this folder"
                );
            }
            FolderContents::Empty => {
                empty.insert(path.clone());
                kept.push(path);
            }
            FolderContents::SaveData | FolderContents::Unknown => kept.push(path),
        }
    }
    g.found_paths = kept;
    empty
}

/// What an empty candidate folder gets told about itself.
const EMPTY_OFFER_REASON: &str = "empty: the game has not written a save here yet";

/// Pin every empty folder's **path** grade to `Low`.
///
/// A folder that scored `High` on its name and its position is still a folder
/// with nothing in it, and saying so is the difference between "we found your
/// saves" and "this is where they will be". Applied before the ranking in the
/// multi-path case, so an empty folder also sinks below a sibling that holds
/// something instead of leading `found_paths` and becoming what automatic
/// tracking picks.
///
/// The game's own grade is deliberately left alone. `DetectedGame::confidence`
/// answers "is this game installed here", and an empty save folder is no
/// evidence against that: the game is installed, it has not been played. It is
/// also what lets an untouched folder adopt a save that already exists in the
/// cloud, which is exactly the case a `Low` here would break.
fn cap_empty_offers(g: &mut DetectedGame, empty: &HashSet<PathBuf>) {
    if empty.is_empty() {
        return;
    }
    for (i, path) in g.found_paths.iter().enumerate() {
        if !empty.contains(path) {
            continue;
        }
        if let Some(c) = g.path_confidences.get_mut(i) {
            *c = Confidence::Low;
        }
        if let Some(r) = g.path_reasons.get_mut(i) {
            *r = EMPTY_OFFER_REASON.into();
        }
    }
}

/// The directories of the standard save roots, indexed by normalised name.
///
/// Built **once per detection pass** and shared by every unresolved slug. The
/// first cut re-scanned the roots per slug: each one is capped at
/// [`NAME_LOOKUP_TIMEOUT`], so thirty unresolved games across seven roots put a
/// theoretical minute and a half of cold-disk scanning on the critical path.
/// Scanned once, the cap is a per-pass ceiling instead of a per-game one.
///
/// Two levels are indexed, which is what covers the overwhelming majority:
///   - `<root>/<Game>`, the game's own folder;
///   - `<root>/<Studio>/<Game>`, the Unity convention, and what a good many
///     others copy.
///
/// A third level is not guessing territory we want to be in.
#[derive(Debug, Default)]
pub struct NamedDirs {
    by_key: HashMap<String, Vec<PathBuf>>,
}

impl NamedDirs {
    pub fn scan(roots: &[PathBuf]) -> Self {
        let mut by_key: HashMap<String, Vec<PathBuf>> = HashMap::new();
        for root in roots {
            let start = Instant::now();
            let mut looked = 0usize;
            let Ok(read) = std::fs::read_dir(root) else {
                continue;
            };
            for entry in read.flatten() {
                if looked >= NAME_LOOKUP_MAX_DIRS || start.elapsed() >= NAME_LOOKUP_TIMEOUT {
                    break;
                }
                looked += 1;
                let Ok(ft) = entry.file_type() else { continue };
                if !ft.is_dir() {
                    continue;
                }
                let path = entry.path();
                let name = entry.file_name();
                let Some(name) = name.to_str() else { continue };
                by_key.entry(name_key(name)).or_default().push(path.clone());

                let Ok(inner) = std::fs::read_dir(&path) else {
                    continue;
                };
                for sub in inner.flatten() {
                    // The clock here too: without it, a root with few top-level
                    // folders and a great many inside (a prolific studio's
                    // `LocalLow`, a cold disk) was only braked by the entry cap,
                    // which is a number rather than a time.
                    if looked >= NAME_LOOKUP_MAX_DIRS || start.elapsed() >= NAME_LOOKUP_TIMEOUT {
                        break;
                    }
                    looked += 1;
                    let Ok(ft) = sub.file_type() else { continue };
                    if !ft.is_dir() {
                        continue;
                    }
                    let sub_name = sub.file_name();
                    let Some(sub_name) = sub_name.to_str() else {
                        continue;
                    };
                    by_key
                        .entry(name_key(sub_name))
                        .or_default()
                        .push(sub.path());
                }
            }
        }
        Self { by_key }
    }

    /// Adds another index's entries. A game under Proton can save inside the prefix
    /// or in the real `$HOME` (cross-platform native ones do), so its index looks in
    /// both places.
    fn absorb(&mut self, other: &NamedDirs) {
        for (k, paths) in &other.by_key {
            let slot = self.by_key.entry(k.clone()).or_default();
            for p in paths {
                if !slot.contains(p) {
                    slot.push(p.clone());
                }
            }
        }
    }

    fn paths_for(&self, keys: &[String]) -> Vec<PathBuf> {
        let mut out: Vec<PathBuf> = Vec::new();
        for k in keys {
            if let Some(hits) = self.by_key.get(k) {
                for p in hits {
                    if !out.contains(p) {
                        out.push(p.clone());
                    }
                }
            }
        }
        out
    }
}

/// Look for a game's save folder **by name** among the directories of the roots
/// where save folders actually live, before resorting to walking its install
/// directory.
///
/// This is the gap that pointed a self-hoster's client at a 3.6 GB folder: when
/// a catalogued game's declared path does not resolve (the game saves somewhere
/// else, or the entry is stale) the only fallback was to walk the install dir
/// and offer whatever looked save-shaped inside it. The standard roots were
/// never consulted for a game that *had* a catalogue entry, even though
/// [`roots::user_save_roots`] already lists the very place most of them save
/// (`LocalLow` is in there, labelled Unity's `persistentDataPath`).
///
/// Proton prefixes matter more here than the host's own roots for a Windows game
/// on Linux: the real `LocalLow` is inside `pfx/drive_c/users/steamuser/`, and
/// the home directory of the person running the game has nothing to do with it.
/// The caller passes an index covering both.
///
/// Every hit must pass [`holds_player_data`], so a folder that only holds engine
/// logs is not offered, which is precisely what a name-only match gets wrong.
pub fn discover_by_name(
    index: &NamedDirs,
    display_name: &str,
    extra_names: &[String],
    shields: &[String],
) -> Vec<DiscoveredSavePath> {
    let mut wanted: Vec<String> = vec![name_key(display_name)];
    for n in extra_names {
        let k = name_key(n);
        if !k.is_empty() && !wanted.contains(&k) {
            wanted.push(k);
        }
    }
    // A two-letter name matches anything, and recommending the wrong folder costs
    // more than recommending none.
    wanted.retain(|k| k.len() >= 3);
    if wanted.is_empty() {
        return Vec::new();
    }

    index
        .paths_for(&wanted)
        .into_iter()
        .filter(|p| holds_player_data(p, shields))
        .map(|path| DiscoveredSavePath {
            path,
            confidence: Confidence::High,
            reason: "folder named after the game in a standard save root".to_string(),
        })
        .collect()
}

/// The Steam Cloud folder for one appid under one Steam account, if there is
/// one.
///
/// The documented layout is `userdata/<storeUserId>/<appid>/remote/`, and that
/// is the only shape the pipeline looked for. Not every game uses it: Mojo:
/// Hanako writes straight into `userdata/<storeUserId>/892630`, one level up,
/// and every title that does was invisible: its only save is here, so missing
/// it meant missing the game.
///
/// `remote/` still wins where it exists: it is the folder Valve documents and
/// the one whose contents are the game's, while the appid folder also holds
/// Steam's own bookkeeping next to it. The fallback is gated on the folder
/// holding actual player data, which is what keeps a stray `remotecache.vdf`
/// from passing for a save.
fn steam_cloud_dir_for(user_dir: &Path, app_id: u64, shields: &[String]) -> Option<PathBuf> {
    let app_dir = user_dir.join(app_id.to_string());
    let remote = app_dir.join("remote");
    if remote.is_dir() {
        return Some(remote);
    }
    if app_dir.is_dir() && inspect_folder(&app_dir, shields) == FolderContents::SaveData {
        return Some(app_dir);
    }
    None
}

/// Look for a game's save folder under the standard roots by its **installDir**
/// the folder name Steam's own `appmanifest_<appid>.acf` records, which is not
/// always the name the game is sold under.
///
/// Aven Colony installs into `prj_juniper` and saves into
/// `<xdgData>/prj_juniper/savegames`. Nothing about "Aven Colony" appears
/// anywhere near the save, so every name-based lookup misses it, and the catalog
/// has no Linux path to expand: the game came back installed and with nowhere
/// to back up. The codename was on disk the whole time, in a field already
/// parsed and already carried on the row.
///
/// This is the same pairing [`discover_unattributed`] does with the correlation
/// store, with a signal that needs no observed play session: an exact string
/// match against a name Valve wrote down. It is a direct `stat` per root rather
/// than a scan, so unlike [`NamedDirs`] it has no budget to run out of, which
/// is what makes it worth having next to a lookup that already passes the
/// installDir in as an extra name.
///
/// The folder is refined the same way a catalog hit is (`prj_juniper` →
/// `prj_juniper/savegames`), falling back to the folder itself for games that
/// save straight into it. Minting a path is a stronger act than keeping one, so
/// the gate is the strict one: something inside has to be player data, not
/// merely worth backing up. A folder sharing the install name and holding one
/// `settings.ini` is the same decoy [`inspect_folder`] exists to reject.
fn discover_by_install_dir_name(
    os: Os,
    slug: &str,
    install_dir: Option<&Path>,
    prefix_root: Option<&Path>,
    shields: &[String],
) -> Vec<DiscoveredSavePath> {
    let Some(name) = install_dir
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
    else {
        return Vec::new();
    };
    // Same floor as the name lookup: a two-character folder name matches
    // anything, and pointing at the wrong folder costs more than pointing at
    // none.
    if name_key(name).len() < 3 {
        return Vec::new();
    }

    let mut search_roots = roots::user_save_roots(os);
    if let Some(prefix) = prefix_root {
        search_roots.extend(roots::prefix_user_roots(prefix));
    }

    let mut out: Vec<DiscoveredSavePath> = Vec::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();
    for root in search_roots {
        let candidate = root.join(name);
        if !candidate.is_dir() {
            continue;
        }
        let refined = refine_save_dir(slug, vec![candidate.clone()]);
        let hits = if refined.is_empty() {
            vec![candidate]
        } else {
            refined
        };
        for hit in hits {
            if !seen.insert(hit.clone())
                || inspect_folder(&hit, shields) != FolderContents::SaveData
            {
                continue;
            }
            out.push(DiscoveredSavePath {
                path: hit,
                confidence: Confidence::High,
                reason: format!("folder named after the game's install dir ({name})"),
            });
        }
    }
    out
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
        // The user-writable areas (AppData, Documents, save folders) all live
        // under a user's profile there, so walking from those avoids the
        // `drive_c/windows` / `drive_c/Program Files` noise.
        //
        // `steamuser` used to be hardcoded here, which is right for Steam's
        // own prefixes and wrong for every other launcher: Heroic, Lutris and
        // Bottles name the profile after the real account, so their prefixes
        // yielded nothing at all. `prefix_windows_users` takes whichever
        // profiles actually exist, the same rule the generic-prefix stage
        // has always used.
        for user in roots::prefix_windows_users(prefix) {
            let home = prefix.join("drive_c/users").join(&user);
            if home.is_dir() {
                walk_root_collecting(
                    &home,
                    max_depth,
                    timeout_per_root,
                    &mut out,
                    &mut seen,
                    store,
                );
            }
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

/// `true` when a path segment is a container, an identifier or an account
/// name: anything but the name of a game. Walking up to attribute a save
/// folder skips these; landing on one is the whole reason production grew
/// saves called `user`, `steam`, `settings` and `2059170`.
///
/// One list, three sources, and they have to be the same three the loader
/// quarantines against or we mint names that state loading then rejects:
/// [`hoard_core::ids::is_generic_name`] for the static plumbing, the
/// machine-minted ids and the too-short segments; and
/// [`crate::agent::is_generic_identity_token`] for the components of THIS
/// user's home path, which no static list can know: an OEM Windows box whose
/// account is literally `user` was the single biggest source of the bad names.
fn segment_names_no_game(name: &str) -> bool {
    hoard_core::ids::is_generic_name(name)
        || crate::agent::is_generic_identity_token(&hoard_core::ids::canon_token(name))
}

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
    /// Set when the attribution landed on a catalog entry that carries an
    /// appid: the cover art and the cross-device identity come from it.
    pub steam_app_id: Option<u64>,
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

    discover_in_roots(walk_roots, store, known_paths, deep)
}

/// Scan ONE user-chosen folder (the Library's "scan folder" / "track another
/// folder" / "no save folder yet" flows) and attribute every folder that holds
/// data to a game name: the user points Hoard at a place and gets back "we
/// found <Game> here".
///
/// **This is deliberately not the periodic scan's question.** Out in the broad
/// roots the question is "does this look enough like a save to mint a game?",
/// and a weak candidate has to be dropped or the Library fills with phantoms.
/// Here the user has already answered it, by pointing at this folder, so the
/// question becomes the much simpler "which folders under it hold their own
/// data?". Scoring still runs, but only to *grade* a hit (so the UI can rank
/// and label it), never to veto one. That's what makes a folder full of saves
/// with a proprietary extension (the common case in `Saved Games`, where the
/// scored walk found one game in four) come back complete.
///
/// The walk emits a folder and stops descending as soon as it holds files of
/// its own, so:
///
/// * `…/Saved Games` (only `desktop.ini` + one dir per game) yields one
///   candidate per game, not the container;
/// * `.../Saved Games/Surviving Mars Relaunched`, the folder the user picked,
///   *is* the save folder and yields itself. The old walk only ever classified
///   the children of the root, so pointing straight at a game's own folder
///   found nothing at all;
/// * a game whose saves sit in `…/<Game>/slot1/` yields `slot1`, attributed to
///   `<Game>` by the ancestor rule.
///
/// `known_paths` still skips folders already covered by a tracked save, so the
/// list only ever offers new candidates.
pub fn discover_in_folder(
    root: &Path,
    store: &CorrelationStore,
    known_paths: &HashSet<PathBuf>,
) -> Vec<AttributedSave> {
    let mut out: Vec<AttributedSave> = Vec::new();
    let start = Instant::now();
    let mut entries_checked: usize = 0;
    let mut stack: Vec<(PathBuf, usize)> = vec![(root.to_path_buf(), 0)];

    while let Some((dir, depth)) = stack.pop() {
        if out.len() >= EXPLICIT_WALK_MAX_CANDIDATES {
            break;
        }
        if entries_checked.is_multiple_of(TIMEOUT_CHECK_INTERVAL)
            && start.elapsed() >= EXPLICIT_WALK_TIMEOUT
        {
            break;
        }
        entries_checked += 1;

        let is_root = depth == 0;
        if !is_root && (is_internal_or_trash(&dir) || is_too_broad(&dir)) {
            continue;
        }
        // The folder the user picked is offered whatever happens; for the ones
        // inside it, a config, cache or log name is a flat no (and a dead end
        // besides: it is not descended into either).
        if !is_root && dir_name_is_negative(&dir) {
            continue;
        }
        // Emulator save root: it's a container of one folder per title, so
        // what's offered is those, and the walk stops here. Without the stop
        // the same titles come back a second time from below, and any
        // subfolder deeper than a title would be offered on its own.
        if emulators::save_root_at(&dir).is_some() {
            if !path_already_known(&dir, known_paths) {
                push_attributed(&mut out, &dir, store);
            }
            continue;
        }
        // One subfolder per save inside (the Cyberpunk 2077 shape): the folder
        // the user wants tracked is THIS one, not each of the ones inside.
        // Pointing at it returned seventeen identical rows, one per save, all
        // named the same, and none of them was the game's folder, so there was
        // no way to back up the manual saves without filing each one by hand.
        // Emitted whole, and the walk doesn't descend past it.
        if is_nest_of_save_dirs(&dir) {
            if !path_already_known(&dir, known_paths) {
                push_attributed(&mut out, &dir, store);
            }
            continue;
        }
        if holds_own_data(&dir) {
            if !path_already_known(&dir, known_paths) {
                push_attributed(&mut out, &dir, store);
            }
            // Emitted, so we stop descending. The user wants the game's folder, not
            // every save subfolder inside it.
            //
            // Except in the one they picked: there we descend anyway, because a
            // folder can be both things. `Documents` has loose files AND a folder
            // per game inside; stopping at it would return one useless result and
            // hide exactly what they were looking for.
            if !is_root {
                continue;
            }
        }
        if depth >= EXPLICIT_WALK_MAX_DEPTH {
            continue;
        }
        let Ok(read) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in read.flatten() {
            let Ok(ft) = entry.file_type() else { continue };
            if !ft.is_dir() || ft.is_symlink() {
                continue;
            }
            let name = entry.file_name();
            let Some(name_str) = name.to_str() else {
                continue;
            };
            if is_skip_dir(name_str) {
                continue;
            }
            stack.push((entry.path(), depth + 1));
        }
    }

    out
}

/// Depth and budget of the explicit-folder sweep. More generous than the periodic
/// one, since it is a one-off request from the user with the app waiting in front
/// of them, but bounded all the same: with no cap, pointing at `C:\` would be a
/// walk across the whole disk.
const EXPLICIT_WALK_MAX_DEPTH: usize = 6;
const EXPLICIT_WALK_TIMEOUT: Duration = Duration::from_millis(8000);
/// Cap on finds. Well above the periodic sweep's 16: here one legitimate folder
/// (`Saved Games`) already brings a dozen games, and cutting at 16 would hide
/// exactly what the user was looking for.
const EXPLICIT_WALK_MAX_CANDIDATES: usize = 64;

/// Files that do not count as a folder's own data: the ones the operating system
/// or the file manager creates. Without this list the `desktop.ini` Windows leaves
/// in `Saved Games` would turn the whole container into a single candidate and hide
/// the games inside it.
const OS_JUNK_FILES: &[&str] = &["desktop.ini", "thumbs.db", ".ds_store", "icon\r"];

/// `true` when the directory holds data of its own: at least one file that is not
/// system noise, and not everything being images (those are screenshots).
///
/// It is the predicate that separates "a container to descend through" from "this
/// is the folder". It looks at neither names nor extensions on purpose: a
/// proprietary extension is exactly what the scored sweep cannot recognise.
fn holds_own_data(dir: &Path) -> bool {
    let Ok(read) = std::fs::read_dir(dir) else {
        return false;
    };
    let mut files = 0usize;
    let mut images = 0usize;
    for entry in read.flatten() {
        let Ok(ft) = entry.file_type() else { continue };
        if !ft.is_file() {
            continue;
        }
        let name = entry.file_name();
        let lower = name.to_string_lossy().to_ascii_lowercase();
        if OS_JUNK_FILES.contains(&lower.as_str()) {
            continue;
        }
        files += 1;
        if matches!(
            lower.rsplit_once('.').map(|(_, e)| e),
            Some("png" | "jpg" | "jpeg" | "bmp" | "gif" | "webp")
        ) {
            images += 1;
        }
    }
    files > 0 && images < files
}

/// `true` when the folder keeps its saves not in files of its own but in **one
/// subfolder per save**: the Cyberpunk 2077 shape
/// (`…/Cyberpunk 2077/AutoSave-0/sav.dat`, `…/ManualSave-3/sav.dat`), which a
/// fair number of modern games share.
///
/// It exists because that shape slipped through all three paths at once and
/// left the user with no way to reach the right folder by any of them. The
/// catalog points straight at the game's folder, but [`find_save_subdirs`]
/// recognises none of [`SAVE_PATTERNS`] inside it (`AutoSave-0` is not one of
/// those spellings) so the hit was dropped whole and the game surfaced with
/// no path. What was left were the loose folders phase 4 rescued one by one: a
/// separate "game" per save, and only the ones the game had written to lately
/// (the autosaves), never the manual ones. A player who saves by hand got a
/// row per slot and had to re-point Hoard after every new save.
///
/// Deliberately conservative, because the expensive mistake is the opposite
/// one, swallowing a whole install directory, or a container of SEVERAL
/// games, as if it were one save, so every condition has to hold:
///
/// * the folder is one nobody may be handed whole ([`never_offer_whole`]:
///   `Saved Games`, `Documents`, `AppData`, a Proton prefix…);
/// * it holds **no data of its own**. A folder that does is not a nest, it is
///   a folder with saves *and* other things in it (an install directory with
///   `settings_game.cfg` next to `saves/`, say) and the existing paths grade
///   that correctly already. Swallowing it whole would back up the game;
/// * at least **two** subfolders hold data of their own
///   ([`holds_own_data`]);
/// * **every** subfolder that holds data is named like a save
///   ([`name_is_save_slot`]). One foreign child with content (`mods`,
///   `config`) and this stops being a nest: we no longer know what is what,
///   and the amber "pick a folder" alert is a better answer than a guess;
/// * and none of them is one of the exact [`SAVE_PATTERNS`] spellings. A child
///   literally called `saves` means the folder is a *container of saves*, not
///   one save (`.../common/Planet S` with `saves/` and `saves_migrated/` inside
///   is an install directory, and the answer there is to descend into `saves`,
///   which is what refinement and the walk already do. Found by sweeping this
///   machine's real folders: 129,383 directories, and that install dir was one
///   of the three the rule fired on.
fn is_nest_of_save_dirs(dir: &Path) -> bool {
    if never_offer_whole(dir) || holds_own_data(dir) {
        return false;
    }
    let Ok(read) = std::fs::read_dir(dir) else {
        return false;
    };
    let mut slots = 0usize;
    for entry in read.flatten() {
        let Ok(ft) = entry.file_type() else { continue };
        if !ft.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let Some(name_str) = name.to_str() else {
            continue;
        };
        if is_skip_dir(name_str) {
            continue;
        }
        if !holds_own_data(&entry.path()) {
            continue;
        }
        if !name_is_save_slot(name_str) || name_matches_save_pattern(name_str) {
            return false;
        }
        slots += 1;
    }
    slots >= 2
}

/// `true` when the name gives the folder away as ONE save inside the game's
/// folder: `AutoSave-0`, `ManualSave-3`, `QuickSave`, `SaveGame01`, `slot1`,
/// `profile2`.
///
/// Says nothing about a bare `saves`: that is a folder *of* saves, and the
/// caller has to tell the two apart itself; see [`is_nest_of_save_dirs`].
fn name_is_save_slot(name: &str) -> bool {
    junkdirs::looks_like_save_dir_name(name) || name_matches_slot_profile_user(name)
}

/// A nest ([`is_nest_of_save_dirs`]) graded by the BEST of its children: the
/// correlation that corroborates `…/Cyberpunk 2077/AutoSave-3` corroborates
/// the game's folder, which is where the game was writing. Without this the
/// nest would score as what the scoring sees (a folder without a single file
/// in it) and phase 4's precision gate would drop it.
///
/// `None` when it isn't a nest, or when no child grades save-like: a nest of
/// folders that aren't saves is not a finding.
fn classify_nest_as_save_like(dir: &Path, store: &CorrelationStore) -> Option<DiscoveredSavePath> {
    if !is_nest_of_save_dirs(dir) {
        return None;
    }
    let read = std::fs::read_dir(dir).ok()?;
    let mut best: Option<DiscoveredSavePath> = None;
    for entry in read.flatten() {
        let Ok(ft) = entry.file_type() else { continue };
        if !ft.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let Some(name_str) = name.to_str() else {
            continue;
        };
        let Some(graded) = classify_dir_as_save_like(&entry.path(), name_str, store) else {
            continue;
        };
        if best
            .as_ref()
            .is_none_or(|b| confidence_rank(graded.confidence) > confidence_rank(b.confidence))
        {
            best = Some(graded);
        }
    }
    let best = best?;
    Some(DiscoveredSavePath {
        path: dir.to_path_buf(),
        confidence: best.confidence,
        reason: format!("one folder per save inside ({})", best.reason),
    })
}

/// `true` when the folder's name gives it away as config, cache, logs or
/// screenshots, and never a game's save folder.
fn dir_name_is_negative(dir: &Path) -> bool {
    dir.file_name()
        .and_then(|s| s.to_str())
        .map(|n| {
            let lower = n.to_lowercase();
            scoring::NEGATIVE_NAME_VOCAB.contains(&lower.as_str())
        })
        .unwrap_or(false)
}

/// What to offer for a folder that turns out to be an emulator's save root.
///
/// `None` means it isn't one, so carry on with the ordinary attribution.
///
/// The root of an emulator is a **container of one folder per title**, not a
/// save. Offering it whole produced the two failures this answers: a slug torn
/// off the tree's plumbing (`dev_hdd0`, from rpcs3's
/// `dev_hdd0/home/<profile>/savedata`, on macOS and on RetroDECK), and a save
/// that can never back up: one rpcs3 root logged "nothing to back up and this
/// save has never had a snapshot" 224 times and was still logging it in
/// ago-2026. The likeliest reason is the profile id: the catalog template says
/// `00000001`, and a user whose active rpcs3 profile is `00000002` has a
/// `00000001/savedata` that stays empty forever.
///
/// So the answer is one row per title, named the way the "add emulator" dialog
/// names them (`emu-<id>-<title>`), and an **empty vec** (refuse) when there
/// is no title inside to name. Refusing is the honest end of an empty
/// container: there is nothing there to back up, and the dialog is the flow
/// built for adding it once there is.
///
/// A root that holds files of its own is not a container at all (RetroArch
/// keeps flat `.srm`s in `saves/`); that one is offered, but under the
/// emulator's name instead of whatever segment the ancestor walk landed on.
///
/// Unlike the dialog, these rows carry no process list, so they don't need its
/// `shared_processes` pin: nothing here can make the emulator's executable
/// count as playing all of them at once.
fn emulator_candidates(dir: &Path, store: &CorrelationStore) -> Option<Vec<AttributedSave>> {
    let Some(def) = emulators::save_root_at(dir) else {
        // Inside a root rather than at it: the sweep descends to where the files
        // are. What gets offered is the title's folder, with the same name it would
        // have had if the root had been entered.
        let (def, title) = emulators::save_root_above(dir)?;
        return Some(vec![emulator_title_save(def, &title)]);
    };
    let titles = emulators::titles_in(def, dir);

    if titles.is_empty() {
        if emulators::has_direct_file(dir) {
            // A flat save: the folder IS the save, it was just badly named.
            let graded = classify_dir_as_save_like(dir, def.display_name, store);
            return Some(vec![AttributedSave {
                slug: format!("emu-{}", def.id),
                display_name: def.display_name.to_string(),
                path: dir.to_path_buf(),
                confidence: graded
                    .as_ref()
                    .map(|g| g.confidence)
                    .unwrap_or(Confidence::Medium),
                reason: format!("{}'s own save folder ({})", def.display_name, def.system),
                steam_app_id: None,
            }]);
        }
        tracing::info!(
            emulator = def.id,
            path = %dir.display(),
            "detect: emulator save root with no title inside, not offering it as a save"
        );
        crate::telemetry::emulator_root_skipped(def.id, dir);
        return Some(Vec::new());
    }

    Some(
        titles
            .iter()
            .map(|t| emulator_title_save(def, &t.path))
            .collect(),
    )
}

/// A title folder inside `def`'s save root, with the name and slug the "add
/// emulator" dialog would give it: the title's ID rather than its name, because it
/// is the only thing both machines call the same.
fn emulator_title_save(def: &'static emulators::EmulatorDef, title: &Path) -> AttributedSave {
    let title_id = title
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    AttributedSave {
        slug: format!("emu-{}-{}", def.id, ludusavi::slugify(title_id)),
        display_name: format!("{}: {}", def.display_name, title_id),
        path: title.to_path_buf(),
        confidence: Confidence::Medium,
        reason: format!("one {} title inside its save root", def.display_name),
        steam_app_id: None,
    }
}

/// Append candidates the caller doesn't already have. Several hits under the
/// same emulator title all fold back to that one folder, so without this the
/// same title lands in the list once per file the walk stopped at.
fn extend_without_repeats(out: &mut Vec<AttributedSave>, found: Vec<AttributedSave>) {
    for save in found {
        if !out.iter().any(|s| s.path == save.path) {
            out.push(save);
        }
    }
}

/// Attribute one discovered folder to a game and append it. The scoring runs
/// only to grade the hit (`Low` when it wouldn't even qualify on its own),
/// in this walk it never decides whether the folder makes the list.
fn push_attributed(out: &mut Vec<AttributedSave>, dir: &Path, store: &CorrelationStore) {
    // An emulator root is attributed to no game: it is split per title or refused.
    // It goes before everything else because its name comes from the emulator's
    // plumbing, not from the user's save tree.
    if let Some(found) = emulator_candidates(dir, store) {
        extend_without_repeats(out, found);
        return;
    }
    let name = dir.file_name().and_then(|s| s.to_str()).unwrap_or_default();
    let graded = classify_dir_as_save_like(dir, name, store);
    let Some(display_name) = attribute_game_name(dir, store) else {
        tracing::debug!(
            path = %dir.display(),
            "detect: no segment of this path names a game, not offering it"
        );
        return;
    };
    let slug = ludusavi::slugify(&display_name);
    if slug.is_empty() {
        return;
    }
    let steam_app_id = catalog_app_id(&display_name);
    out.push(AttributedSave {
        slug,
        display_name,
        path: dir.to_path_buf(),
        confidence: graded
            .as_ref()
            .map(|g| g.confidence)
            .unwrap_or(Confidence::Low),
        reason: graded
            .map(|g| g.reason)
            .unwrap_or_else(|| "folder holds data (you pointed us here)".to_string()),
        steam_app_id,
    });
}

/// The catalogue appid for an already-attributed name, when it names it exactly.
/// It only confirms what [`attribute_game_name`] resolved, never guessing again,
/// and it is what gives the find its cover art.
fn catalog_app_id(display_name: &str) -> Option<u64> {
    ludusavi::find_by_canon_name(display_name).and_then(|e| e.steam_app_id)
}

/// Shared core of [`discover_unattributed_mode`] and [`discover_in_folder`]:
/// walk each root, grade every candidate with the correlation store, apply the
/// precision gate, and attribute survivors to a game name. `deep` relaxes both
/// the walk budget and the gate (keeps `Low` static-only hits).
fn discover_in_roots(
    walk_roots: Vec<PathBuf>,
    store: &CorrelationStore,
    known_paths: &HashSet<PathBuf>,
    deep: bool,
) -> Vec<AttributedSave> {
    let mut out: Vec<AttributedSave> = Vec::new();
    let mut emitted: HashSet<PathBuf> = HashSet::new();

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
            // Precision gate: weak static-only matches don't create games,
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
            if let Some(found) = emulator_candidates(&hit.path, store) {
                extend_without_repeats(&mut out, found);
                continue;
            }
            let Some(display_name) = attribute_game_name(&hit.path, store) else {
                tracing::debug!(
                    path = %hit.path.display(),
                    "detect: no segment of this path names a game, not offering it"
                );
                continue;
            };
            let slug = ludusavi::slugify(&display_name);
            if slug.is_empty() {
                continue;
            }
            let steam_app_id = catalog_app_id(&display_name);
            out.push(AttributedSave {
                slug,
                display_name,
                path: hit.path,
                confidence: hit.confidence,
                reason: hit.reason,
                steam_app_id,
            });
        }
    }

    out
}

/// True if `candidate` is already covered by a catalog/Steam hit: either it
/// equals a known path, sits inside one, or contains one.
fn path_already_known(candidate: &Path, known: &HashSet<PathBuf>) -> bool {
    known.iter().any(|k| paths_overlap(candidate, k))
}

/// `true` when two paths overlap: they are the same folder, or one hangs off the
/// other. It is the "this folder is already covered" predicate used by both phase
/// 4's discovery and auto-track (which without it tracked the same folder once per
/// name correlation attributed to it).
///
/// On Windows it compares case-insensitively: `C:\Users\...\Saved Games` and
/// `c:\users\...\saved games` are the SAME folder, and the path stored in the
/// state and the one coming out of the walk do not always agree on case.
pub fn paths_overlap(a: &Path, b: &Path) -> bool {
    path_is_inside(a, b) || path_is_inside(b, a)
}

/// `true` when `inner` IS `outer` or hangs off it. The directed half of
/// [`paths_overlap`], for when the useful answer isn't "they overlap" but
/// which one is inside which: a save tracked inside the folder being
/// added is not fixed the same way as one that contains it.
///
/// Case-insensitive on Windows, for the same reason as [`paths_overlap`].
pub fn path_is_inside(inner: &Path, outer: &Path) -> bool {
    if cfg!(windows) {
        // `starts_with` compares by COMPONENT, so lowercasing the whole string does
        // not change the semantics (the separators are still where they were).
        let (inner, outer) = (
            PathBuf::from(inner.to_string_lossy().to_lowercase()),
            PathBuf::from(outer.to_string_lossy().to_lowercase()),
        );
        inner.starts_with(&outer)
    } else {
        inner.starts_with(outer)
    }
}

/// Best-effort game name for a phase-4 save folder.
///
/// 1. If the correlation store attributed a process to the folder, use it
///    (the process name minus `.exe` is the game far more often than not).
/// 2. Otherwise walk up from the folder past recognised save-words and
///    generic container segments; the first "real" segment names the game
///    (`…/My Games/Skyrim/Saves` → `Skyrim`).
///
/// `None` when neither ladder reaches a name that could belong to a game. The
/// caller must then drop the candidate rather than file it under whatever
/// segment was nearest: production carries saves called `user` (13 accounts),
/// `steam` (11), `settings`, `logs` and bare Steam appids, all minted here,
/// all of them unpairable across machines and unsearchable in the library.
fn attribute_game_name(path: &Path, store: &CorrelationStore) -> Option<String> {
    let obs = store.signal_for(path);
    // The attributed process name, only when it still passes the CURRENT rules: an
    // attribution poisoned before the `is_installer_like` fix (say
    // `Codex Windows Sandbox Setup.exe`) is still in the persisted store and must not
    // rename a save that its folder names perfectly well.
    let proc_name: Option<&str> = obs
        .map(|o| o.process_name.trim())
        .filter(|n| !n.is_empty() && crate::correlation::is_game_like(n, None));

    // ---- signals the CATALOGUE can confirm, most direct first
    //
    // Any of the three gives an authoritative title, which is why they all come
    // before the raw names: a raw name is unstable (the churn from one app name to
    // the next) and sometimes simply worse than the data already in front of us.
    if let Some(name) = proc_name {
        // 1. The executable, when it belongs to a single manifest game.
        for probe in [name.to_string(), format!("{name}.exe")] {
            if let Some(title) = ludusavi::title_for_exe(&probe) {
                return Some(title.to_string());
            }
        }
    }
    // 2. The executable's INSTALL FOLDER. It counts when the exe's name is
    //    ambiguous: `Mars.exe` is claimed by two games in the catalogue, so step 1
    //    vetoes it, correctly, but the exe lives in
    //    `C:\Games\Surviving Mars Relaunched\Mars.exe` and that folder leaves no
    //    room for doubt. The same trick `process_identity_candidates` uses to match
    //    live processes.
    if let Some(dir) = obs
        .and_then(|o| o.exe.as_deref())
        .and_then(|e| e.parent())
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
    {
        // `title_for_canon_name` rather than `find_by_canon_name`: here only a NAME
        // is being looked for, and the catalogue only carries games with a save
        // path. An edition the manifest only knows by its title is not in it, so
        // looking there returned nothing and the save ended up named after the
        // executable.
        if let Some(title) = ludusavi::title_for_canon_name(dir) {
            return Some(title.to_string());
        }
    }
    // 3. The save's first ancestor with a name of its own, when it is a manifest
    //    game. `.../Saved Games/Surviving Mars Relaunched/<steamid>` is one.
    let ancestor = meaningful_ancestor(path);
    if let Some(title) = ancestor.and_then(ludusavi::title_for_canon_name) {
        return Some(title.to_string());
    }
    // 4. The same ancestor, but with the catalogue's title as a PREFIX: the folder
    //    carries a qualifier the catalogue does not have (`Surviving Mars
    //    Relaunched`, `... Definitive Edition`). Without this every edition
    if let Some(entry) = ancestor.and_then(ludusavi::find_by_name_prefix) {
        return Some(entry.display_name.clone());
    }

    // ---- and when the catalogue recognises nothing, the raw names
    //
    // Raw, but through the same gate: a process called `game.exe` or `steam.exe`
    // names things as badly as the path segment already vetoed, and here there is no
    // catalogue behind it to contradict it.
    if let Some(name) = proc_name {
        // Without the extension: the name goes to the UI, not to a matcher.
        let bare = name.strip_suffix(".exe").unwrap_or(name);
        if !segment_names_no_game(bare) {
            return Some(prettify_process_name(bare));
        }
    }
    // `ancestor` already passed the gate when it was chosen; without one there is no
    // name. `path`'s last segment is NOT a substitute: it is precisely the one the
    // climb just discarded.
    ancestor.map(str::to_string)
}

/// The first segment climbing up from `path` that could be a game's name: not a
/// generic container (`AppData`, `Saved Games`), not an account id
/// (`.../Plan B Terraform/76561197960287930/saves` has to give "Plan B Terraform",
/// not the SteamID), and not a save word.
///
/// `None` means the climb reached the filesystem root without finding one.
/// That is an answer, not a failure: it is what `~/.local/share/<opaque>` and
/// `C:\Users\user\AppData\Local` really have to say about which game wrote
/// there, and inventing a name from the last segment looked at, which is what
/// this used to fall back to, is how `local`, `user` and `logs` became games.
fn meaningful_ancestor(path: &Path) -> Option<&str> {
    let mut cur = Some(path);
    while let Some(dir) = cur {
        if let Some(name) = dir.file_name().and_then(|s| s.to_str()) {
            if !segment_names_no_game(name) && !scoring::name_recognised(name) {
                return Some(name);
            }
        }
        cur = dir.parent();
    }
    None
}

/// Turn a raw process name into a presentable Library display name:
/// `"plan_b-terraform"` → `"Plan B Terraform"`. Only all-lowercase names are
/// touched: a name that already carries any uppercase ("NieRAutomata",
/// "DOOMEternal") is left verbatim, since mangling its casing is worse than
/// showing it raw. The slug is unaffected either way: `slugify` lowercases
/// and folds separators, so `slugify(prettified) == slugify(raw)`.
fn prettify_process_name(name: &str) -> String {
    if name.chars().any(|c| c.is_ascii_uppercase()) {
        return name.to_string();
    }
    let pretty = name
        .split(['_', '-', ' '])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    if pretty.is_empty() {
        name.to_string()
    } else {
        pretty
    }
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
    // exhaust branches doesn't change which paths qualify, only the order
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
            // correlation/state json) nor the desktop trash: descending there
            // mints phantom "games" out of our own data (e.g. the timestamped
            // `conflicts/<id>/<ts>/autosave` folders) or out of deleted files.
            if is_internal_or_trash(&path) {
                continue;
            }
            // A nest (one subfolder per save) is emitted whole and not
            // descended into: otherwise every save of the same game entered the
            // list as its own "game", and between them they ate the root's
            // whole candidate budget.
            if let Some(hit) = classify_nest_as_save_like(&path, store) {
                if seen.insert(path.clone()) {
                    out.push(hit);
                    if out.len() - initial >= AGGRESSIVE_WALK_MAX_CANDIDATES {
                        return;
                    }
                }
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
                // into it: saves typically live one level deep, and going
                // further just bloats the candidate list.
                continue;
            }
            if depth < max_depth {
                stack.push((path, depth + 1));
            }
        }
    }
}

/// True iff the walker should not descend into a directory with this name:
/// the asset/build denylist, or anything that is regenerable cache.
///
/// The cache half is [`junkdirs::is_cache_dir_name`], which matches by
/// **suffix** on a separator-stripped name, so `AnvilDX12Cache`,
/// `FortniteShaderCache` and `Shader Cache` are all caught. The old
/// exact-match negative vocabulary in `scoring` only ever saw a bare
/// `shadercache`, and it merely subtracted score instead of pruning the walk.
fn is_skip_dir(name: &str) -> bool {
    let lower = name.to_lowercase();
    WALK_SKIP.contains(&lower.as_str()) || junkdirs::is_cache_dir_name(name)
}

/// Roots that must never be offered as one game's save folder, resolved once.
fn blocked_roots() -> &'static HashSet<PathBuf> {
    static BLOCKED: std::sync::OnceLock<HashSet<PathBuf>> = std::sync::OnceLock::new();
    BLOCKED.get_or_init(|| junkdirs::blocked_roots(Os::current()))
}

/// `true` when a candidate IS a whole profile/engine root rather than one
/// game's folder inside it (see [`junkdirs::blocked_roots`]).
///
/// Exact match on purpose: `AppData/Roaming/RenPy` is every RenPy game on the
/// machine and must never be a save, while `AppData/Roaming/RenPy/MyGame` is
/// exactly right.
fn is_too_broad(path: &Path) -> bool {
    blocked_roots().contains(path)
}

/// `true` when this folder must never be handed over as one game's save folder,
/// whole.
///
/// Two guards, because each covers what the other cannot. [`is_too_broad`]
/// compares against roots resolved for the running OS, so on Linux it knows
/// nothing about the Windows-shaped profile inside a Proton prefix, so
/// `…/pfx/drive_c/users/steamuser/Saved Games` is not in its set, and that is
/// where the Windows rules living inside `drive_c` have bitten before.
/// [`junkdirs::dangerous_sync_root`] is structural: it reads the shape of the
/// path, recognises a prefix by its tail, and applies the Windows rules from
/// `drive_c` down.
///
/// The pair is what add-to-library and the backup already enforce, so anything
/// detection offers past this point is something the rest of the app will
/// accept: offering a folder that `hoard add` then refuses is a dead end the
/// user has to work out on their own.
fn never_offer_whole(dir: &Path) -> bool {
    is_too_broad(dir) || crate::junkdirs::dangerous_sync_root(dir).is_some()
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

/// Grade a single save folder, keeping the WHY. Catalog/Steam paths are never
/// dropped here: an already-attributed path stays in the list even when it
/// scores low (it's a real candidate, just weak evidence, a near-empty
/// Steam-Cloud stub *should* read `Low`).
fn grade_path_reasoned(path: &Path, store: &CorrelationStore) -> (Confidence, String) {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    match classify_dir_as_save_like(path, name, store) {
        Some(d) => (d.confidence, d.reason),
        None => (
            Confidence::Low,
            "below the save-like floor (0.35): no name, content or recency signal".into(),
        ),
    }
}

/// Basenames of every regular file under `dir`, recursively. Bounded like the
/// scoring scans (depth + shared budget) so a pathological tree can't turn a
/// comparison into a walk of the whole drive; symlinks never count.
///
/// `false` means "unknown", meaning the budget ran out, and callers must treat that
/// as NO answer: an incomplete listing must never decide a demotion.
fn collect_file_basenames(
    dir: &Path,
    depth: usize,
    budget: &mut usize,
    out: &mut HashSet<String>,
) -> bool {
    if depth == 0 || *budget == 0 {
        return false;
    }
    let Ok(read) = std::fs::read_dir(dir) else {
        return true; // unreadable = empty set, not unknown
    };
    for entry in read.flatten() {
        if *budget == 0 {
            return false;
        }
        let Ok(ft) = entry.file_type() else { continue };
        if ft.is_symlink() {
            continue;
        }
        let path = entry.path();
        if ft.is_dir() {
            if !collect_file_basenames(&path, depth - 1, budget, out) {
                return false;
            }
        } else if ft.is_file() {
            *budget -= 1;
            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                out.insert(name.to_lowercase());
            }
        }
    }
    true
}

/// Is `copy` the game's own backup mirror of `original`? (P2,
/// DETECCION-REVISION §4 R3.) Two conditions, BOTH required:
///
/// * Name relation: some ancestor of `original`, the folder itself or a
///   parent, sits next to `copy` and `copy` is its name plus a backup
///   suffix. This covers both shapes: same-parent twins (`Saves` vs
///   `SavesOld`) and the incident's shape, where the copy hugs the original
///   from one level up (`SaveGames/<id>` vs `SaveGamesBackup`).
/// * **Content superset**: every file basename under `original` also exists
///   somewhere under `copy`. A rotating mirror accumulates copies, so it
///   always holds for a live twin; two unrelated folders fail it. Names only,
///   not sizes or hashes, since each backup pass rewrites the saves, so bytes and
///   sizes differ by design.
///
/// Either check failing (or being *unknowable*, with the budget exhausted) returns
/// `false`: this function gates a demotion, so its false-positive cost is the
/// expensive one. A `-bak` sibling without the content relation stays exactly
/// where it was.
pub(crate) fn is_backup_mirror(copy: &Path, original: &Path) -> bool {
    use crate::junkdirs::{ends_with_backup_suffix, normalize_dir_name};
    // Cheap gate first: no backup suffix on the copy, nothing to talk about.
    if !ends_with_backup_suffix(
        copy.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default(),
    ) {
        return false;
    }
    let copy_norm = normalize_dir_name(
        copy.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default(),
    );
    let Some(copy_parent) = copy.parent() else {
        return false;
    };

    // Deepest ancestor of `original` whose parent is `copy`'s parent and
    // whose name (normalized) is a strict prefix of `copy`'s.
    let mut anc = Some(original);
    let mut name_related = false;
    while let Some(a) = anc {
        if a.parent() == Some(copy_parent) {
            if let Some(an) = a.file_name().and_then(|s| s.to_str()) {
                let an = normalize_dir_name(an);
                if !an.is_empty() && copy_norm.len() > an.len() && copy_norm.starts_with(&an) {
                    name_related = true;
                    break;
                }
            }
        }
        anc = a.parent();
    }
    if !name_related {
        return false;
    }

    const NAMES_DEPTH: usize = 4;
    let mut orig_budget: usize = 2048;
    let mut orig_names: HashSet<String> = HashSet::new();
    let mut copy_budget: usize = 4096;
    let mut copy_names: HashSet<String> = HashSet::new();
    if !collect_file_basenames(original, NAMES_DEPTH, &mut orig_budget, &mut orig_names) {
        return false;
    }
    if orig_names.is_empty() {
        // An empty "original" would make the superset vacuous and let any
        // suffixed neighbour be condemned by nothing at all.
        return false;
    }
    if !collect_file_basenames(copy, NAMES_DEPTH, &mut copy_budget, &mut copy_names) {
        return false;
    }
    orig_names.iter().all(|n| copy_names.contains(n))
}

/// P9: check every tracked save against the mirror rule. A row tracked before
/// the scoring fixes never gets re-evaluated by the pipeline (`run_scan`
/// skips tracked slugs), so this is the only path that can notice "you are
/// backing up `SaveGamesBackup` while your actual save sits next door".
///
/// Strictness mirrors [`grade_and_rank_paths`]: a warning needs either the
/// full structural twin (name relation + content superset) or, weaker, just
/// the suffix relation: the reason string says which one fired so the UI and
/// support can weigh it. Purely read-only: repointing stays a user act.
fn detect_tracked_mirrors(state: &CliState, games: &[DetectedGame]) -> Vec<MirrorWarning> {
    let mut candidates: Vec<&PathBuf> = games.iter().flat_map(|g| g.found_paths.iter()).collect();
    candidates.sort();
    candidates.dedup();

    let mut out = Vec::new();
    for (save_id, s) in state.saves.iter() {
        if s.paused {
            continue;
        }
        let tracked = &s.local_path;
        // The mirror rule compares against OTHER folders; a candidate inside
        // (or containing) the tracked folder is not a sibling, it is the same
        // data at a different granularity.
        let mut best_strict: Option<&PathBuf> = None;
        let mut best_name_only: Option<&PathBuf> = None;
        for cand in &candidates {
            if *cand == tracked || paths_overlap(cand, tracked) {
                continue;
            }
            if is_backup_mirror(tracked, cand) {
                best_strict = Some(cand);
                break; // sorted: deterministic pick
            }
            // Name-only relation: the suffix half of the rule without the
            // superset. Weaker evidence, kept separately.
            use crate::junkdirs::{ends_with_backup_suffix, normalize_dir_name};
            let tn = tracked
                .file_name()
                .and_then(|s| s.to_str())
                .map(normalize_dir_name)
                .unwrap_or_default();
            let cn = cand
                .file_name()
                .and_then(|s| s.to_str())
                .map(normalize_dir_name)
                .unwrap_or_default();
            if ends_with_backup_suffix(
                tracked
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default(),
            ) && !cn.is_empty()
                && tn.starts_with(&cn)
                && tn.len() > cn.len()
            {
                best_name_only = Some(cand);
            }
        }
        if let Some(suggested) = best_strict.or(best_name_only) {
            let strict = best_strict.is_some();
            out.push(MirrorWarning {
                save_id: save_id.clone(),
                game_slug: s.game_slug.clone(),
                label: s.label.clone(),
                tracked_path: tracked.clone(),
                suggested_path: (*suggested).clone(),
                reason: format!(
                    "{} of {}",
                    if strict { "mirror" } else { "name-only" },
                    suggested.display()
                ),
            });
        }
    }
    out.sort_by(|a, b| (&a.game_slug, &a.save_id).cmp(&(&b.game_slug, &b.save_id)));
    out
}

/// Re-score every game's `found_paths`, sort them strongest-first, and fill
/// `path_confidences` aligned 1:1. Single-path games skip the extra I/O and
/// just inherit the game's rolled-up confidence. Manual-override rows are left
/// untouched (the user's pick is authoritative and already `High`).
fn grade_and_rank_paths(
    by_slug: &mut std::collections::HashMap<String, DetectedGame>,
    store: &CorrelationStore,
) {
    for g in by_slug.values_mut() {
        if g.found_paths.is_empty() {
            g.path_confidences.clear();
            g.path_reasons.clear();
            continue;
        }
        // Offer filter (see [`drop_folders_without_saves`]): existing on disk is
        // not the same as holding a save, and until now the pipeline treated it
        // as if it were. A folder the catalog named, that exists, and that holds
        // nothing but settings is not an answer to "where are my saves".
        //
        // A hand-picked folder is exempt: the user said this one, and being told
        // their own choice looks empty to us is not an improvement.
        let manual = matches!(g.source, DetectionSource::ManualOverride);
        let empty_offers = if manual {
            HashSet::new()
        } else {
            drop_folders_without_saves(g)
        };
        if g.found_paths.is_empty() {
            // Everything it had was settings folders. The row stays (the game
            // IS installed) with no path, which is the state the UI answers
            // with the folder picker.
            g.path_confidences.clear();
            g.path_reasons.clear();
            continue;
        }
        if manual || g.found_paths.len() == 1 {
            // Trust the existing grade; just make the parallel vecs match.
            g.path_confidences = vec![g.confidence; g.found_paths.len()];
            g.path_reasons = vec![String::new(); g.found_paths.len()];
            cap_empty_offers(g, &empty_offers);
            continue;
        }
        let graded: Vec<(PathBuf, Confidence, String)> = g
            .found_paths
            .iter()
            .map(|p| {
                if empty_offers.contains(p) {
                    return (p.clone(), Confidence::Low, EMPTY_OFFER_REASON.to_string());
                }
                let (c, r) = grade_path_reasoned(p, store);
                (p.clone(), c, r)
            })
            .collect();
        // P2: a backup mirror of ANOTHER candidate never leads, even when its
        // confidence would tie or win: the mirror is written constantly, so
        // correlation loves it, and that is precisely the Wukong failure.
        // Stable sort keeps discovery order everywhere else.
        let mirrors: Vec<bool> = graded
            .iter()
            .map(|(p, _, _)| {
                graded
                    .iter()
                    .any(|(q, _, _)| q != p && is_backup_mirror(p, q))
            })
            .collect();
        let mut order: Vec<usize> = (0..graded.len()).collect();
        order.sort_by_key(|&i| (mirrors[i], std::cmp::Reverse(confidence_rank(graded[i].1))));
        let ranked: Vec<_> = order.iter().map(|&i| graded[i].clone()).collect();

        // An empty folder never decides the game's grade; see
        // [`cap_empty_offers`]. With nothing but empty folders left there is
        // nothing to re-roll from, so the grade the pipeline arrived at stands.
        if let Some(max) = ranked
            .iter()
            .filter(|(p, _, _)| !empty_offers.contains(p))
            .map(|(_, c, _)| *c)
            .max_by_key(|c| confidence_rank(*c))
        {
            g.confidence = max;
        }
        g.found_paths = ranked.iter().map(|(p, _, _)| p.clone()).collect();
        g.path_confidences = ranked.iter().map(|(_, c, _)| *c).collect();
        g.path_reasons = ranked.iter().map(|(_, _, r)| r.clone()).collect();
    }
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
    // Same rule as the catalog stages, at the walk's funnel: a profile or
    // shared-engine root is never one game's save, however well it scores.
    if is_too_broad(path) {
        return None;
    }
    // Phase 1 and 3 (ADR 0020): the name-only boolean is replaced by graded scoring
    // (`scoring::score_dir`: name plus content plus recency plus negatives) PLUS the
    // process-to-write correlation bonus (+0.50) when the store corroborates the dir.
    // Below `SCORE_POSSIBLE` it is discarded.
    //
    // The bonus is not reserved: it is granted when the score crosses the
    // auto-confirm cutoff, which is the signal the ADR demands for certainty.
    // Without correlation, a purely static high score caps at `Medium`. The number
    // goes into `reason` for the diagnostics panel.
    let breakdown = correlation::score_with_correlation(path, name, store);
    if breakdown.score < scoring::SCORE_POSSIBLE {
        return None;
    }
    // Corroboration for granting `High`: process-to-write correlation (the store)
    // OR an archive with a verified save-like index. The second is direct evidence,
    // since we opened the `.zip` and saw the save inside, so it counts as much as
    // correlation and demands no observed play session.
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
                path_confidences: Vec::new(),
                path_reasons: Vec::new(),
                confidence: Confidence::Medium,
                source: DetectionSource::SteamLibrary,
                steam_app_id: Some(42),
                install_dir: Some(PathBuf::from("/steam/x")),
                needs_folder: false,
                steam_cloud: false,
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
    /// passed through untouched: the heuristic doesn't need to descend.
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

    /// A literal-file template widens to its folder, which is the recall fix
    /// for the ~4,900 catalog entries that only name a file, but only when the
    /// folder is the save's and not the game's.
    #[test]
    fn refine_save_dir_widens_a_file_hit_to_its_clean_folder() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("SomeGame");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("savegame.xml");
        std::fs::write(&file, b"<save/>").unwrap();

        let refined = refine_save_dir("some-game", vec![file]);
        assert_eq!(refined, vec![dir]);
    }

    /// Issue #17: `AppData\Local\Teardown` holds `savegame.xml` next to a
    /// `mods\` folder of promo art. Widening there turned a few KB of save into
    /// 42 MB across 173 files, so a folder with foreign content keeps the file.
    #[test]
    fn refine_save_dir_keeps_the_file_when_the_folder_holds_mods() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("Teardown");
        std::fs::create_dir_all(dir.join("mods").join("promo")).unwrap();
        let file = dir.join("savegame.xml");
        std::fs::write(&file, b"<save/>").unwrap();

        let refined = refine_save_dir("teardown", vec![file.clone()]);
        assert_eq!(refined, vec![file]);
    }

    /// Same guard, cache flavour: a `ShaderCache/` sibling is just as much a
    /// sign that the folder belongs to the game rather than to its saves.
    #[test]
    fn refine_save_dir_keeps_the_file_when_the_folder_holds_cache() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("SomeGame");
        std::fs::create_dir_all(dir.join("ShaderCache")).unwrap();
        let file = dir.join("player.sav");
        std::fs::write(&file, b"x").unwrap();

        let refined = refine_save_dir("some-game", vec![file.clone()]);
        assert_eq!(refined, vec![file]);
    }

    /// The guard must not fire on a folder whose odd name merely *contains* a
    /// foreign word while still announcing itself as saves.
    #[test]
    fn refine_save_dir_widens_when_the_sibling_still_says_saves() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("SomeGame");
        std::fs::create_dir_all(dir.join("SaveMods")).unwrap();
        let file = dir.join("savegame.xml");
        std::fs::write(&file, b"<save/>").unwrap();

        let refined = refine_save_dir("some-game", vec![file]);
        assert_eq!(refined, vec![dir]);
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

    /// Root exists but contains no save-named subdir, so the hit is dropped
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

    /// One folder per save inside the game's own: the Cyberpunk 2077 shape,
    /// where the catalog points at `…/CD Projekt Red/Cyberpunk 2077` and there
    /// is no `Saves/` inside to refine down to. The hit is kept whole; before,
    /// the game was left with no path and all that showed up were the loose
    /// autosaves phase 4 rescued one at a time.
    fn cyberpunk_shaped(root: &Path) -> PathBuf {
        let game = root.join("CD Projekt Red").join("Cyberpunk 2077");
        for slot in [
            "AutoSave-0",
            "AutoSave-1",
            "ManualSave-0",
            "ManualSave-1",
            "QuickSave-0",
        ] {
            let dir = game.join(slot);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("sav.dat"), b"save").unwrap();
            std::fs::write(dir.join("metadata.9.json"), b"{}").unwrap();
            std::fs::write(dir.join("screenshot.png"), b"png").unwrap();
        }
        game
    }

    #[test]
    fn refine_save_dir_keeps_a_folder_of_per_save_folders() {
        let tmp = tempfile::tempdir().unwrap();
        let game = cyberpunk_shaped(tmp.path());

        let refined = refine_save_dir("cyberpunk-2077", vec![game.clone()]);
        assert_eq!(refined, vec![game]);
    }

    /// The publisher's folder is NOT a nest: its child is the game's folder,
    /// which holds no files of its own. Without that condition a
    /// `CD Projekt Red/` with two games inside would be tracked as a single
    /// save.
    #[test]
    fn a_publisher_folder_is_not_a_nest() {
        let tmp = tempfile::tempdir().unwrap();
        let game = cyberpunk_shaped(tmp.path());
        let publisher = game.parent().unwrap();

        assert!(is_nest_of_save_dirs(&game));
        assert!(!is_nest_of_save_dirs(publisher));
    }

    /// Inside a Proton prefix the Windows rules are the ones that apply, and
    /// `is_too_broad` cannot see them: it holds the roots resolved for the
    /// running OS, and on Linux `<winSavedGames>` expands to nothing. So a
    /// prefix's own `Saved Games`, with a couple of slot-shaped folders in it,
    /// which is all the nest test asks for, passed as one game's save folder.
    /// The structural guard is what catches it, by the tail of the path.
    #[test]
    fn a_windows_root_inside_a_proton_prefix_is_not_a_nest() {
        let tmp = tempfile::tempdir().unwrap();
        let profile = tmp
            .path()
            .join("SteamLibrary/steamapps/compatdata/1091500/pfx/drive_c/users/steamuser");

        for root in ["Saved Games", "Documents", "AppData/LocalLow"] {
            let dir = profile.join(root);
            for slot in ["Save1", "Save2", "Save3"] {
                let slot_dir = dir.join(slot);
                std::fs::create_dir_all(&slot_dir).unwrap();
                std::fs::write(slot_dir.join("game.sav"), b"save").unwrap();
            }
            assert!(
                !is_nest_of_save_dirs(&dir),
                "swallowed a whole {root} inside the prefix"
            );
            assert!(
                !refine_save_dir("some-game", vec![dir.clone()]).contains(&dir),
                "offered the whole {root} of the prefix as one game's folder"
            );
            // One level down IS a game's folder, and still qualifies.
            let game = dir.join("Some Studio").join("Some Game");
            for slot in ["Save1", "Save2"] {
                let slot_dir = game.join(slot);
                std::fs::create_dir_all(&slot_dir).unwrap();
                std::fs::write(slot_dir.join("game.sav"), b"save").unwrap();
            }
            assert!(is_nest_of_save_dirs(&game), "{root}: lost the real nest");
        }
    }

    /// A foreign child WITH content breaks the nest: mixed in with mods we no
    /// longer know what is a save, and the amber alert beats swallowing the
    /// whole folder.
    #[test]
    fn a_foreign_data_child_disqualifies_the_nest() {
        let tmp = tempfile::tempdir().unwrap();
        let game = cyberpunk_shaped(tmp.path());
        assert!(is_nest_of_save_dirs(&game));

        let mods = game.join("mods");
        std::fs::create_dir_all(&mods).unwrap();
        std::fs::write(mods.join("cyberware.archive"), b"mod").unwrap();

        assert!(!is_nest_of_save_dirs(&game));
        assert!(refine_save_dir("cyberpunk-2077", vec![game]).is_empty());
    }

    /// Pointing at the game's folder returns ONE row, the folder, and not
    /// one per save. This is the flow a user reaches for when detection misses,
    /// and it used to return seventeen identical "Cyberpunk 2077" rows.
    #[test]
    fn scanning_a_nest_offers_the_folder_itself_once() {
        let tmp = tempfile::tempdir().unwrap();
        let game = cyberpunk_shaped(tmp.path());
        let store = CorrelationStore::default();

        let found = discover_in_folder(&game, &store, &HashSet::new());
        assert_eq!(found.len(), 1, "{found:#?}");
        assert_eq!(found[0].path, game);

        // And from the publisher's container, the same: one row, the game's
        // folder, not five save folders.
        let from_above = discover_in_folder(game.parent().unwrap(), &store, &HashSet::new());
        assert_eq!(from_above.len(), 1, "{from_above:#?}");
        assert_eq!(from_above[0].path, game);
    }

    /// The phase 4 sweep emits the nest, not each loose save: the correlation
    /// that corroborates a child corroborates the game's folder. Without this
    /// the Library filled up with a row per autosave, all under the same name,
    /// and between them they ate the root's candidate budget.
    #[test]
    fn phase_four_surfaces_the_nest_and_not_its_slots() {
        with_isolated_home(|home| {
            let game = cyberpunk_shaped(&home.join("xdg-data"));

            let mut store = CorrelationStore::default();
            store.record(
                &game.join("AutoSave-1"),
                &[crate::correlation::GameProcess {
                    name: "cyberpunk2077.exe".into(),
                    exe: None,
                }],
            );

            let found = discover_unattributed(Os::Linux, &store, &HashSet::new());
            assert!(
                found.iter().any(|a| a.path == game),
                "the game's folder must surface: {found:#?}"
            );
            assert!(
                !found
                    .iter()
                    .any(|a| a.path.starts_with(&game) && a.path != game),
                "no loose save may surface on its own: {found:#?}"
            );
        });
    }

    /// An install directory is not a nest. Found by running the rule over this
    /// machine's real folders: a Steam install with `saves/` and
    /// `saves_migrated/` inside qualified, so the aggressive walk stopped there
    /// and offered the game's whole installation instead of descending into
    /// `saves` the way it did before. Two things rule it out, and either alone
    /// is enough: the folder holds a file of its own, and one of its children
    /// is called `saves`: a container of saves, not one save.
    #[test]
    fn an_install_dir_with_a_saves_folder_is_not_a_nest() {
        let tmp = tempfile::tempdir().unwrap();
        let install = tmp.path().join("steamapps/common/Planet S");
        for sub in ["saves", "saves_migrated"] {
            let dir = install.join(sub);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("world.sav"), b"save").unwrap();
        }
        std::fs::write(install.join("settings_game.cfg"), b"cfg").unwrap();
        assert!(!is_nest_of_save_dirs(&install));

        // Still not one with the loose file gone: `saves` names a folder OF
        // saves, and descending into it is the right answer.
        std::fs::remove_file(install.join("settings_game.cfg")).unwrap();
        assert!(!is_nest_of_save_dirs(&install));

        // And the walk goes back to offering the save folder, not the install.
        let store = CorrelationStore::default();
        let found = discover_in_folder(&install, &store, &HashSet::new());
        assert!(
            found.iter().any(|f| f.path == install.join("saves")),
            "lost the real save folder: {found:#?}"
        );
    }

    /// The parent of a nest holds nothing of its own, which is what makes
    /// everything inside it a save. A folder that holds data *and* has
    /// save-named subfolders is something else, and the ordinary grading
    /// already handles it.
    #[test]
    fn a_folder_with_files_of_its_own_is_not_a_nest() {
        let tmp = tempfile::tempdir().unwrap();
        let game = cyberpunk_shaped(tmp.path());
        assert!(is_nest_of_save_dirs(&game));

        std::fs::write(game.join("graphics.ini"), b"[video]").unwrap();
        assert!(!is_nest_of_save_dirs(&game));
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
    /// them: it only matches by slug and copies display_name.
    fn synthetic_entry(slug: &str, display_name: &str, app_id: Option<u64>) -> LudusaviEntry {
        LudusaviEntry {
            slug: slug.into(),
            display_name: display_name.into(),
            steam_app_id: app_id,
            paths: hoard_manifest::ludusavi::LudusaviPaths::default(),
            registry: Vec::new(),
            install_dirs: Vec::new(),
            launch_exes: Vec::new(),
            steam_extra_ids: Vec::new(),
            lutris_slug: None,
            cloud_steam: false,
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
    /// fallback must skip it: never demote a High/Medium entry to Low.
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
                path_confidences: Vec::new(),
                path_reasons: Vec::new(),
                confidence: Confidence::Medium,
                source: DetectionSource::SteamLibrary,
                steam_app_id: Some(999),
                install_dir: Some(PathBuf::from("/steam/Test Game")),
                needs_folder: false,
                steam_cloud: false,
            },
        );

        apply_steam_name_fallback(&catalog, &steam_apps, &mut by_slug);

        let g = &by_slug["test-game"];
        // Confidence stays Medium; the fallback did not overwrite.
        assert_eq!(g.confidence, Confidence::Medium);
        assert_eq!(by_slug.len(), 1);
    }

    /// Steam apps whose slugified name is not in the catalog produce no
    /// noise: the dedupe map stays empty.
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

    /// Steam Cloud without the `remote/` level: Mojo: Hanako writes straight
    /// into `userdata/<storeUserId>/892630`. Its only save lives there, so the
    /// game was invisible, not merely short a path.
    #[test]
    fn a_steam_cloud_save_without_a_remote_dir_is_found() {
        let tmp = tempfile::tempdir().unwrap();
        let user = tmp.path().join("userdata/76561198041773665");
        let app = user.join("892630");
        std::fs::create_dir_all(&app).unwrap();
        std::fs::write(app.join("slot1.sav"), "x").unwrap();

        assert_eq!(steam_cloud_dir_for(&user, 892630, &[]), Some(app));
    }

    /// Where both exist, `remote/` is the answer: it is the folder Valve
    /// documents, and the appid folder around it also holds Steam's own
    /// bookkeeping.
    #[test]
    fn the_remote_dir_still_wins_where_there_is_one() {
        let tmp = tempfile::tempdir().unwrap();
        let user = tmp.path().join("userdata/1");
        let app = user.join("646270");
        let remote = app.join("remote");
        std::fs::create_dir_all(&remote).unwrap();
        std::fs::write(remote.join("save.dat"), "x").unwrap();
        std::fs::write(app.join("stray.dat"), "x").unwrap();

        assert_eq!(steam_cloud_dir_for(&user, 646270, &[]), Some(remote));
    }

    /// Steam's own bookkeeping is not a save. An appid folder holding nothing
    /// but `remotecache.vdf` is what every Cloud-enabled game has, played or
    /// not, and offering it would mint a save folder for all of them.
    #[test]
    fn an_appid_dir_holding_only_steam_bookkeeping_is_not_a_save() {
        let tmp = tempfile::tempdir().unwrap();
        let user = tmp.path().join("userdata/1");
        let app = user.join("999999");
        std::fs::create_dir_all(&app).unwrap();
        std::fs::write(app.join("remotecache.vdf"), "x").unwrap();

        assert_eq!(steam_cloud_dir_for(&user, 999999, &[]), None);
        // And an appid this account never touched at all.
        assert_eq!(steam_cloud_dir_for(&user, 123, &[]), None);
    }

    /// The installDir is a different string from the retail name often enough to
    /// matter: Aven Colony installs into `prj_juniper` and saves into
    /// `<xdgData>/prj_juniper/savegames`. Nothing that looks like "Aven Colony"
    /// exists anywhere near the save.
    #[test]
    fn the_install_dir_name_finds_a_codenamed_save_folder() {
        with_isolated_home(|home| {
            let data = home.join("xdg-data");
            let saves = data.join("prj_juniper/savegames");
            std::fs::create_dir_all(&saves).unwrap();
            std::fs::write(saves.join("colony1.sav"), "x").unwrap();

            let install = PathBuf::from("/lib/steamapps/common/prj_juniper");

            // The name index cannot help: nothing here is named after the game.
            let empty = NamedDirs::default();
            assert!(discover_by_name(&empty, "Aven Colony", &[], &[]).is_empty());

            let found =
                discover_by_install_dir_name(Os::Linux, "aven-colony", Some(&install), None, &[]);
            let paths: Vec<&PathBuf> = found.iter().map(|d| &d.path).collect();
            assert_eq!(paths, vec![&saves], "refined down to the save subdir");
            assert_eq!(found[0].confidence, Confidence::High);
        });
    }

    /// A game that saves straight into its install-named folder, with no save
    /// subdir to refine down to, still gets the folder itself.
    #[test]
    fn the_install_dir_name_keeps_a_folder_with_no_save_subdir() {
        with_isolated_home(|home| {
            let data = home.join("xdg-data");
            let dir = data.join("prj_juniper");
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("colony1.sav"), "x").unwrap();

            let install = PathBuf::from("/lib/steamapps/common/prj_juniper");
            let found =
                discover_by_install_dir_name(Os::Linux, "aven-colony", Some(&install), None, &[]);
            assert_eq!(found.len(), 1, "{found:?}");
            assert_eq!(found[0].path, dir);
        });
    }

    /// And a folder that merely shares the name while holding no player data is
    /// still not offered: the install-dir signal is exact, not a licence.
    #[test]
    fn the_install_dir_name_does_not_offer_a_settings_folder() {
        with_isolated_home(|home| {
            let data = home.join("xdg-data");
            let dir = data.join("prj_juniper");
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("settings.ini"), "x").unwrap();

            let install = PathBuf::from("/lib/steamapps/common/prj_juniper");
            assert!(discover_by_install_dir_name(
                Os::Linux,
                "aven-colony",
                Some(&install),
                None,
                &[]
            )
            .is_empty());
        });
    }

    /// A folder that exists and holds nothing but settings is not a save
    /// folder. `~/.config/SiNKR` holds one `settings.ini`; the catalog points
    /// at it, so it was offered alongside the folder with the actual saves.
    #[test]
    fn a_settings_only_folder_is_not_offered() {
        let tmp = tempfile::tempdir().unwrap();
        let decoy = tmp.path().join("SiNKR");
        std::fs::create_dir_all(&decoy).unwrap();
        std::fs::write(decoy.join("settings.ini"), "fullscreen=1").unwrap();

        assert_eq!(inspect_folder(&decoy, &[]), FolderContents::NoSaveData);
    }

    /// The same folder when the manifest says `.ini` IS this game's save
    /// format. 582 catalog templates do, which is why extension alone can't
    /// decide.
    #[test]
    fn a_shielded_ini_is_player_data() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("Game");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("slot1.ini"), "hp=3").unwrap();

        assert_eq!(inspect_folder(&dir, &[]), FolderContents::NoSaveData);
        assert_eq!(
            inspect_folder(&dir, &["*.ini".to_string()]),
            FolderContents::SaveData
        );
    }

    /// Engine leftovers are not player data either, which is the whole Unity
    /// `LocalLow` shape, which exists for every Unity game whether it saves
    /// there or not.
    #[test]
    fn an_engine_log_folder_is_not_offered() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("Harrow House").join("SiNKR");
        std::fs::create_dir_all(dir.join("Unity/abc-123/Analytics")).unwrap();
        std::fs::write(dir.join("Player.log"), "boot").unwrap();
        std::fs::write(dir.join("Unity/abc-123/Analytics/events"), "{}").unwrap();

        assert_eq!(inspect_folder(&dir, &[]), FolderContents::NoSaveData);

        // The real one, one level down, is the one worth offering.
        let saves = dir.join("saves");
        std::fs::create_dir_all(&saves).unwrap();
        std::fs::write(saves.join("profile1.dat"), "x").unwrap();
        assert_eq!(inspect_folder(&saves, &[]), FolderContents::SaveData);
    }

    /// Installed, never played: the folder is real and empty, and that is a
    /// state worth showing rather than hiding.
    #[test]
    fn an_empty_save_folder_is_told_apart_from_a_junk_one() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("Saves");
        std::fs::create_dir_all(dir.join("nothing/in/here")).unwrap();

        assert_eq!(inspect_folder(&dir, &[]), FolderContents::Empty);
    }

    /// Running out of road means "don't know", never "no". A save four levels
    /// down is not evidence that there is no save.
    #[test]
    fn a_folder_deeper_than_the_walk_is_unknown_not_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("Game");
        let deep = dir.join("a/b/c/d/e");
        std::fs::create_dir_all(&deep).unwrap();
        std::fs::write(deep.join("save.dat"), "x").unwrap();

        assert_eq!(inspect_folder(&dir, &[]), FolderContents::Unknown);
    }

    /// The end-to-end shape of the filter: the settings decoy goes, the real
    /// save folder stays, and the game keeps its grade.
    #[test]
    fn grading_drops_the_decoy_and_keeps_the_save_folder() {
        let tmp = tempfile::tempdir().unwrap();
        let real = tmp.path().join("LocalLow/Harrow House/SiNKR/saves");
        std::fs::create_dir_all(&real).unwrap();
        std::fs::write(real.join("profile1.dat"), "x").unwrap();
        let decoy = tmp.path().join(".config/SiNKR");
        std::fs::create_dir_all(&decoy).unwrap();
        std::fs::write(decoy.join("settings.ini"), "fullscreen=1").unwrap();

        let mut by_slug = HashMap::new();
        by_slug.insert(
            "sinkr".to_string(),
            DetectedGame {
                slug: "sinkr".into(),
                display_name: "SiNKR".into(),
                found_paths: vec![decoy.clone(), real.clone()],
                path_confidences: Vec::new(),
                path_reasons: Vec::new(),
                confidence: Confidence::Medium,
                source: DetectionSource::FilesystemHeuristic,
                steam_app_id: None,
                install_dir: None,
                needs_folder: false,
                steam_cloud: false,
            },
        );

        grade_and_rank_paths(&mut by_slug, &CorrelationStore::default());

        let g = &by_slug["sinkr"];
        assert_eq!(g.found_paths, vec![real], "only the folder with saves");
        assert_eq!(g.path_confidences.len(), g.found_paths.len());
    }

    /// An empty folder is offered, but never above `Low` and never ahead of a
    /// sibling that holds something: `found_paths[0]` is what automatic
    /// tracking picks.
    #[test]
    fn an_empty_folder_is_offered_last_and_low() {
        let tmp = tempfile::tempdir().unwrap();
        let empty = tmp.path().join("Saves");
        std::fs::create_dir_all(&empty).unwrap();
        let full = tmp.path().join("savegames");
        std::fs::create_dir_all(&full).unwrap();
        std::fs::write(full.join("slot1.sav"), "x").unwrap();

        let mut by_slug = HashMap::new();
        by_slug.insert(
            "x".to_string(),
            DetectedGame {
                slug: "x".into(),
                display_name: "X".into(),
                found_paths: vec![empty.clone(), full.clone()],
                path_confidences: Vec::new(),
                path_reasons: Vec::new(),
                confidence: Confidence::High,
                source: DetectionSource::FilesystemHeuristic,
                steam_app_id: None,
                install_dir: None,
                needs_folder: false,
                steam_cloud: false,
            },
        );

        grade_and_rank_paths(&mut by_slug, &CorrelationStore::default());

        let g = &by_slug["x"];
        assert_eq!(g.found_paths.len(), 2, "the empty one is kept, not hidden");
        assert_eq!(g.found_paths[0], full, "the one with saves leads");
        assert_eq!(g.path_confidences[1], Confidence::Low);
        assert!(
            g.path_reasons[1].starts_with("empty:"),
            "{:?}",
            g.path_reasons
        );
    }

    /// A game whose every candidate was a settings folder keeps its row and
    /// loses its paths: "installed, we don't know where it saves" is the state
    /// the folder picker answers, and it beats pointing at the wrong folder.
    #[test]
    fn a_game_left_with_no_offerable_folder_keeps_its_row() {
        let tmp = tempfile::tempdir().unwrap();
        let decoy = tmp.path().join("Game");
        std::fs::create_dir_all(&decoy).unwrap();
        std::fs::write(decoy.join("graphics.cfg"), "1920x1080").unwrap();

        let mut by_slug = HashMap::new();
        by_slug.insert(
            "x".to_string(),
            DetectedGame {
                slug: "x".into(),
                display_name: "X".into(),
                found_paths: vec![decoy],
                path_confidences: Vec::new(),
                path_reasons: Vec::new(),
                confidence: Confidence::High,
                source: DetectionSource::Both,
                steam_app_id: Some(42),
                install_dir: None,
                needs_folder: false,
                steam_cloud: false,
            },
        );

        grade_and_rank_paths(&mut by_slug, &CorrelationStore::default());

        let g = &by_slug["x"];
        assert!(g.found_paths.is_empty());
        assert!(g.path_confidences.is_empty());
        assert_eq!(g.slug, "x", "the game is still reported");
    }

    /// The user's own pick is never second-guessed.
    #[test]
    fn a_hand_picked_folder_is_exempt_from_the_offer_filter() {
        let tmp = tempfile::tempdir().unwrap();
        let chosen = tmp.path().join("Game");
        std::fs::create_dir_all(&chosen).unwrap();
        std::fs::write(chosen.join("settings.ini"), "x").unwrap();

        let mut by_slug = HashMap::new();
        by_slug.insert(
            "x".to_string(),
            DetectedGame {
                slug: "x".into(),
                display_name: "X".into(),
                found_paths: vec![chosen.clone()],
                path_confidences: Vec::new(),
                path_reasons: Vec::new(),
                confidence: Confidence::High,
                source: DetectionSource::ManualOverride,
                steam_app_id: None,
                install_dir: None,
                needs_folder: false,
                steam_cloud: false,
            },
        );

        grade_and_rank_paths(&mut by_slug, &CorrelationStore::default());

        assert_eq!(by_slug["x"].found_paths, vec![chosen]);
        assert_eq!(by_slug["x"].confidence, Confidence::High);
    }

    /// Same Stellaris regression test as before, adapted to the new
    /// function name. Covers the "root exists but no save-named subdir"
    /// regression: historically Hoard would back up the entire root.
    #[test]
    fn refine_save_dir_drops_paradox_root_without_save_games() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("Paradox Interactive").join("Stellaris");
        // Root exists (config and mods present) but no `save games/` yet, so
        // user installed Stellaris but hasn't created a campaign.
        std::fs::create_dir_all(root.join("mod")).unwrap();

        let refined = refine_save_dir("stellaris", vec![root]);
        assert!(refined.is_empty());
    }

    // The detect_all test below mutates process-wide env (HOME / XDG_*) so
    // we serialise it against any other test in this crate that does the
    // same via the crate-wide test_lock. Held across the (single) tokio
    // runtime block, which is fine because detect_all takes seconds, not long
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

    /// The manual override **leads** what the heuristic found: the chosen path
    /// goes first with `High`, the heuristic stays behind with its own grade,
    /// the row flips to `ManualOverride` and the Steam hint (`install_dir`,
    /// `steam_app_id`) survives.
    #[test]
    fn manual_override_leads_heuristic_hit() {
        let mut by_slug: HashMap<String, DetectedGame> = HashMap::new();
        by_slug.insert(
            "stellaris".into(),
            DetectedGame {
                slug: "stellaris".into(),
                display_name: "Stellaris".into(),
                found_paths: vec![PathBuf::from("/wrong/path")],
                path_confidences: vec![Confidence::Medium],
                path_reasons: vec![String::new()],
                confidence: Confidence::Medium,
                source: DetectionSource::FilesystemHeuristic,
                steam_app_id: Some(281990),
                install_dir: Some(PathBuf::from("/steam/stellaris")),
                needs_folder: false,
                steam_cloud: false,
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
            vec![
                PathBuf::from("/home/x/Stellaris/save games"),
                PathBuf::from("/wrong/path"),
            ],
            "the hand-picked one goes first; the heuristic stays behind"
        );
        assert_eq!(
            g.path_confidences,
            vec![Confidence::High, Confidence::Medium],
            "grades stay 1:1 with found_paths"
        );
        assert_eq!(g.steam_app_id, Some(281990));
        assert_eq!(g.install_dir, Some(PathBuf::from("/steam/stellaris")));
    }

    /// The aug-2026 Factorio case: the user hand-picked a folder of their own
    /// on the desktop and the game's REAL folder vanished from the card. The
    /// real one has to stay listed, behind the chosen one.
    #[test]
    fn manual_override_keeps_the_real_folder_visible() {
        let real = PathBuf::from("/home/x/AppData/Roaming/Factorio/saves");
        let picked = PathBuf::from("/home/x/Desktop/saves");
        let mut by_slug: HashMap<String, DetectedGame> = HashMap::new();
        by_slug.insert(
            "factorio".into(),
            DetectedGame {
                slug: "factorio".into(),
                display_name: "Factorio".into(),
                found_paths: vec![real.clone()],
                path_confidences: vec![Confidence::High],
                path_reasons: vec![String::new()],
                confidence: Confidence::High,
                source: DetectionSource::FilesystemHeuristic,
                steam_app_id: None,
                install_dir: None,
                needs_folder: false,
                steam_cloud: false,
            },
        );
        let overrides = HashMap::from([("factorio".to_string(), picked.clone())]);

        apply_manual_overrides(&overrides, &mut by_slug);

        assert_eq!(by_slug["factorio"].found_paths, vec![picked, real]);
    }

    /// Hand-picking a folder the heuristic was ALREADY proposing promotes it to
    /// the front instead of listing it twice.
    #[test]
    fn manual_override_does_not_duplicate_a_path_already_found() {
        let a = PathBuf::from("/games/a/saves");
        let b = PathBuf::from("/games/b/saves");
        let mut by_slug: HashMap<String, DetectedGame> = HashMap::new();
        by_slug.insert(
            "stellaris".into(),
            DetectedGame {
                slug: "stellaris".into(),
                display_name: "Stellaris".into(),
                found_paths: vec![a.clone(), b.clone()],
                path_confidences: vec![Confidence::Medium, Confidence::Low],
                path_reasons: vec![String::new(), String::new()],
                confidence: Confidence::Medium,
                source: DetectionSource::FilesystemHeuristic,
                steam_app_id: None,
                install_dir: None,
                needs_folder: false,
                steam_cloud: false,
            },
        );
        let overrides = HashMap::from([("stellaris".to_string(), b.clone())]);

        apply_manual_overrides(&overrides, &mut by_slug);

        let g = &by_slug["stellaris"];
        assert_eq!(g.found_paths, vec![b, a]);
        assert_eq!(
            g.path_confidences,
            vec![Confidence::High, Confidence::Medium]
        );
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
            // No Steam install, no on-disk save folder for the slug, so the
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
        // The reason is now the score's breakdown; a recent `.sav` contributes
        // "strong save ext" plus "recent save-like file".
        assert!(
            hits[0].reason.contains("recent save-like file"),
            "expected a 'recent save-like file' reason; got {:?}",
            hits[0].reason
        );
    }

    /// The walker had `steamuser` hard-coded, so a Heroic, Lutris or Bottles prefix,
    /// which names the profile with the real account, gave nothing.
    #[test]
    fn the_walker_reaches_a_prefix_whose_user_is_not_steamuser() {
        let tmp = tempfile::tempdir().unwrap();
        let prefix = tmp.path().join("prefix");
        let save_dir = prefix.join("drive_c/users/insider/AppData/Roaming/MyGame/Saves");
        std::fs::create_dir_all(&save_dir).unwrap();
        std::fs::write(save_dir.join("slot1.sav"), b"binary").unwrap();

        let hits = aggressive_discover(
            "my-game",
            "My Game",
            None,
            Some(&prefix),
            AGGRESSIVE_WALK_TIMEOUT,
            AGGRESSIVE_WALK_MAX_DEPTH,
        );
        assert_eq!(hits.len(), 1, "esperado un hallazgo, salió {hits:?}");
        assert_eq!(hits[0].path, save_dir);
    }

    /// Y el caso de Steam sigue funcionando igual.
    #[test]
    fn the_walker_still_reaches_a_steamuser_prefix() {
        let tmp = tempfile::tempdir().unwrap();
        let prefix = tmp.path().join("pfx");
        let save_dir = prefix.join("drive_c/users/steamuser/Documents/My Games/G/Saves");
        std::fs::create_dir_all(&save_dir).unwrap();
        std::fs::write(save_dir.join("a.sav"), b"binary").unwrap();

        let hits = aggressive_discover(
            "g",
            "G",
            None,
            Some(&prefix),
            AGGRESSIVE_WALK_TIMEOUT,
            AGGRESSIVE_WALK_MAX_DEPTH,
        );
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(hits[0].path, save_dir);
    }

    /// A `save/` dir nested under a `bin/` denylist entry must not be
    /// walked into: `WALK_SKIP` is honoured even when the inner dir name
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

    /// Phase 3 closing the loop: a dir with strong static evidence (a recent `.sav`)
    /// that would cap at `Medium` without correlation is promoted to `High` when the
    /// store attributes the write to a game process.
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

    /// Corroboration by content: a `saves` folder with a `.zip` whose index gives a
    /// save away (Factorio: `control.lua`) is granted `High` with no need for process
    /// correlation, because we opened the archive and saw it.
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

    /// A detection cache written by an EARLIER version has to keep loading. A real
    /// regression (Windows, 2026-07-30): adding `wrapper_slugs` with no default made
    /// serde reject the whole existing `detection.json` and the library started cold
    /// after an update.
    #[test]
    fn an_older_cached_report_still_loads() {
        // Stats de antes de `wrapper_slugs` (y de cualquier contador futuro).
        let json = r#"{
            "games": [],
            "catalog_size": 21687,
            "steam_apps_found": 11,
            "scanned_at_ms": 1,
            "stats": {
                "duration_ms": 4648,
                "steam_appid_matches": 5,
                "fs_template_slugs": 16,
                "walker_slugs": 2,
                "phase4_new_games": 3,
                "manual_applied": 1
            }
        }"#;
        let report: DetectionReport =
            serde_json::from_str(json).expect("una caché vieja debe seguir cargando");
        assert_eq!(report.stats.fs_template_slugs, 16);
        assert_eq!(
            report.stats.wrapper_slugs, 0,
            "el contador nuevo por defecto"
        );

        // And a report with no `stats` block at all (older still).
        let older = r#"{"games":[],"catalog_size":1,"steam_apps_found":0,"scanned_at_ms":1}"#;
        let report: DetectionReport = serde_json::from_str(older).unwrap();
        assert_eq!(report.stats, DetectionStats::default());
    }

    /// And a game row cached by an earlier version too.
    #[test]
    fn an_older_cached_game_row_still_loads() {
        let json = r#"{
            "slug": "factorio", "display_name": "Factorio",
            "found_paths": ["/a/b"], "confidence": "high",
            "source": "filesystem_heuristic", "steam_app_id": 427520
        }"#;
        let g: DetectedGame = serde_json::from_str(json).unwrap();
        assert_eq!(g.slug, "factorio");
        assert!(!g.steam_cloud, "el aviso nuevo por defecto");
        assert!(g.path_confidences.is_empty());
    }

    /// A real case (a user's Windows, 2026-07-30): with `<base>` resolved, ARK's
    /// template pointed at `.../ShooterGame/Saved/SavedArksLocal`, which existed with
    /// saves in it, and refinement threw it away for not being one of
    /// `SAVE_PATTERNS`' exact spellings, leaving an amber alert over a perfect hit.
    #[test]
    fn a_hit_whose_name_speaks_of_saves_is_kept_even_if_not_an_exact_pattern() {
        let tmp = tempfile::tempdir().unwrap();
        let hit = tmp.path().join("SavedArksLocal");
        std::fs::create_dir_all(hit.join("Extinction_WP")).unwrap();
        std::fs::write(hit.join("Most Recent Template.arkcharactersetting"), b"x").unwrap();

        assert_eq!(
            refine_save_dir("ark-survival-ascended", vec![hit.clone()]),
            vec![hit]
        );
    }

    /// But the root of a game that mixes saves with mods and config still gives the
    /// amber alert: there we do not know which subfolder it is, and tracking the whole
    /// thing would upload the mods.
    #[test]
    fn a_game_root_without_save_subdirs_still_yields_the_amber_alert() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("Stellaris");
        std::fs::create_dir_all(root.join("mod")).unwrap();
        std::fs::create_dir_all(root.join("settings")).unwrap();

        assert!(refine_save_dir("stellaris", vec![root]).is_empty());
    }

    /// And the new rule does not apply to an EMPTY folder: there is nothing to back
    /// up and offering it would only produce empty snapshots. (An EXACT `SAVE_PATTERNS`
    /// spelling is kept even when empty, which is prior behaviour: the folder exists
    /// and the game will write there.)
    #[test]
    fn the_relaxed_rule_does_not_offer_an_empty_folder() {
        let tmp = tempfile::tempdir().unwrap();
        let hit = tmp.path().join("SavedArksLocal");
        std::fs::create_dir_all(&hit).unwrap();
        assert!(refine_save_dir("x", vec![hit]).is_empty());
    }

    /// A template pointing at a FILE (4,900 catalogue games only have these). Before,
    /// refinement found no save subfolder and threw the find away, leaving the game
    /// with the amber alert. Now the folder containing it is offered, which is what
    /// the user expects to back up and what groups the sibling saves.
    #[test]
    fn a_file_hit_resolves_to_its_containing_folder() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("Sonic");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("ssr_save.bin");
        std::fs::write(&file, b"x").unwrap();

        assert_eq!(refine_save_dir("sonic", vec![file]), vec![dir]);
    }

    /// But when that folder is too broad to offer, the lone file is tracked rather
    /// than the whole profile proposed.
    #[test]
    fn a_file_in_a_too_broad_folder_is_tracked_on_its_own() {
        let Some(home) = directories::UserDirs::new().map(|u| u.home_dir().to_path_buf()) else {
            return;
        };
        let docs = home.join("Documents");
        if !docs.is_dir() {
            return;
        }
        // Nothing is created in the user's home: the path only has to exist as a
        // file for refinement, so a real one is used when there is one and otherwise
        // we bail.
        let Some(existing) = std::fs::read_dir(&docs).ok().and_then(|mut r| {
            r.find_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.is_file())
        }) else {
            return;
        };
        assert_eq!(
            refine_save_dir("some-game", vec![existing.clone()]),
            vec![existing],
            "Documentos entero no puede ser el save; el fichero sí"
        );
    }

    /// A loose template resolving to the whole profile, or to a shared engine root,
    /// cannot end up as a game's save folder: that would be syncing the whole of
    /// Documents, or mixing every RenPy game together.
    #[test]
    fn a_profile_or_engine_root_is_never_merged_as_a_save() {
        let Some(home) = directories::UserDirs::new().map(|u| u.home_dir().to_path_buf()) else {
            return;
        };
        let mut by_slug: HashMap<String, DetectedGame> = HashMap::new();
        merge_fs_hit(
            &mut by_slug,
            "loose-template-game".into(),
            "Loose Template Game".into(),
            vec![home.clone(), home.join("Documents")],
        );
        assert!(
            by_slug.is_empty(),
            "no debería haber creado nada: {by_slug:?}"
        );

        // La carpeta de UN juego dentro de esa misma raíz sí entra.
        merge_fs_hit(
            &mut by_slug,
            "real-game".into(),
            "Real Game".into(),
            vec![home.join("Documents/My Games/Real Game")],
        );
        assert_eq!(by_slug.len(), 1);
    }

    /// The walk cannot rescue it by score either.
    #[test]
    fn the_walk_refuses_to_classify_a_blocked_root() {
        let Some(home) = directories::UserDirs::new().map(|u| u.home_dir().to_path_buf()) else {
            return;
        };
        let store = CorrelationStore::default();
        assert!(classify_dir_as_save_like(&home, "home", &store).is_none());
    }

    /// The cache with the game's name in front of it (`AnvilDX12Cache`) is exactly
    /// what `scoring`'s exact set did not catch.
    #[test]
    fn the_walk_skips_prefixed_cache_dirs() {
        for n in [
            "AnvilDX12Cache",
            "FortniteShaderCache",
            "Shader Cache",
            "logs",
        ] {
            assert!(is_skip_dir(n), "{n} debería saltarse");
        }
        assert!(!is_skip_dir("saves"));
        assert!(!is_skip_dir("SaveGames"));
    }

    /// An emulator's root, entered the way the user enters it: by pointing at the
    /// folder. What comes out is one row per title, with the name and slug the
    /// emulator dialog would give it, and never the root, which is a container with
    /// not one file to back up.
    #[test]
    fn pointing_at_an_emulator_root_lists_its_titles_not_the_root() {
        let tmp = tempfile::tempdir().unwrap();
        // The real shape: `<whatever>/rpcs3/dev_hdd0/home/<profile>/savedata`.
        let root = tmp.path().join("rpcs3/dev_hdd0/home/00000001/savedata");
        for title in ["BLUS30443-AUTOSAVE", "NPUB30493-SAVEDATA01"] {
            let d = root.join(title);
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join("PARAM.SFO"), b"x").unwrap();
        }

        let found = discover_in_folder(&root, &CorrelationStore::default(), &HashSet::new());
        let mut rows: Vec<(String, String)> = found
            .iter()
            .map(|a| (a.slug.clone(), a.display_name.clone()))
            .collect();
        rows.sort();
        assert_eq!(
            rows,
            vec![
                (
                    "emu-rpcs3-blus30443-autosave".to_string(),
                    "RPCS3: BLUS30443-AUTOSAVE".to_string()
                ),
                (
                    "emu-rpcs3-npub30493-savedata01".to_string(),
                    "RPCS3: NPUB30493-SAVEDATA01".to_string()
                ),
            ]
        );
        // And no row is the root: it was the root that could back nothing up.
        assert!(
            found.iter().all(|a| a.path != root),
            "la raíz del emulador no se ofrece como save"
        );
        // Nor does the slug that came out of the emulator's tree survive.
        assert!(found.iter().all(|a| a.slug != "dev-hdd0"));
    }

    /// And an empty root (rpcs3 installed, profile never used, the case of the 224
    /// "nothing to back up" lines) offers nothing at all.
    #[test]
    fn an_empty_emulator_root_is_refused_instead_of_tracked() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("rpcs3/dev_hdd0/home/00000001/savedata");
        std::fs::create_dir_all(&root).unwrap();

        let found = discover_in_folder(&root, &CorrelationStore::default(), &HashSet::new());
        assert!(
            found.is_empty(),
            "una raíz de emulador sin un solo título dentro no es un save: {found:?}"
        );
    }

    /// The sweep does not land on the root: it descends to where the files are.
    /// Entering above it has to give exactly the same as pointing at it, and one row
    /// per title even with several files inside.
    #[test]
    fn walking_into_an_emulator_root_from_above_still_files_it_by_title() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("rpcs3/dev_hdd0/home/00000001/savedata");
        let title = root.join("BLUS30443-AUTOSAVE");
        std::fs::create_dir_all(title.join("deep")).unwrap();
        std::fs::write(title.join("PARAM.SFO"), b"x").unwrap();
        std::fs::write(title.join("deep/SDATA"), b"x").unwrap();

        let found = discover_in_folder(
            &tmp.path().join("rpcs3"),
            &CorrelationStore::default(),
            &HashSet::new(),
        );
        assert_eq!(
            found.len(),
            1,
            "una fila por título, no una por fichero: {found:?}"
        );
        assert_eq!(found[0].slug, "emu-rpcs3-blus30443-autosave");
        assert_eq!(found[0].path, title);
    }

    /// The names production actually minted saves under, plus the ones a walk
    /// runs into on the way there. Every one of them is a container, an
    /// account or an identifier, never a title, and every one has to be
    /// refused at the moment of naming, not just quarantined afterwards.
    #[test]
    fn a_generic_segment_never_names_a_game() {
        // Slugs seen in production (ago-2026), with the accounts behind them:
        // user (13), steam (11), cd (4), and the loose ones.
        for name in [
            "user",
            "steam",
            "cd",
            "settings",
            "local",
            "logs",
            "game",
            // Steam appids that reached the library bare, through a repack
            // wrapper whose appid the catalog doesn't carry.
            "2059170",
            "2479090",
            // And the plumbing any path passes through on the way up.
            "AppData",
            "Roaming",
            "LocalLow",
            "Saved Games",
            "My Games",
            "steamapps",
            "common",
            "compatdata",
            "drive_c",
            "steamuser",
            "userdata",
            "remote",
            "Documents",
            "Public",
            ".config",
            ".local",
            // Identifiers the machine invents for itself: SteamID64, a profile uuid,
            // the hex ids Citra derives from console keys.
            "76561197960287930",
            "00000001",
            "0004000000033400",
            "a1b2c3d4e5f6a7b8",
            // And the plumbing of a Linux handheld: an emulator front-end's
            // per-emulator tree (which IS one of our own deep-scan roots) and
            // the system's container store.
            "Emulation",
            "storage",
            "roms",
            "containers",
            "overlay",
        ] {
            assert!(
                segment_names_no_game(name),
                "{name:?} no nombra ningún juego y debe vetarse al bautizar"
            );
        }

        // The control: real folder names that must NOT be lost.
        for name in [
            "Stellaris",
            "Elden Ring",
            "Surviving Mars Relaunched",
            "Cyberpunk 2077",
            "Project 64",
            "S.T.A.L.K.E.R.",
            "2064 Read Only Memories",
            "DOOM",
            "NieRAutomata",
        ] {
            assert!(
                !segment_names_no_game(name),
                "{name:?} es un nombre de juego perfectamente válido"
            );
        }
    }

    /// The guard, now on the attribution ladder: climb the path looking for a useful
    /// name, and when there is none above, say there is none.
    ///
    /// The left-hand case is the one that filled production: the last segment looked
    /// at was used as a substitute, so a whole path of plumbing ended up christening
    /// a game with the nearest piece of it.
    #[test]
    fn attribution_refuses_a_path_that_names_no_game() {
        let empty = CorrelationStore::default();
        let cases: &[(&str, Option<&str>)] = &[
            // ---- nothing here names a game, so the answer is "none"
            //
            // An OEM Windows account literally called `user`: 13 users.
            ("C:/Users/user/AppData/Local", None),
            ("C:/Users/user/AppData/LocalLow", None),
            // `cd` bajo pura fontanería: 4 usuarios.
            ("C:/Users/user/AppData/Roaming/cd", None),
            // A bare `local`, the Linux half of the same failure.
            ("/home/u/.local/share", None),
            // `steam` / `common`: 11 usuarios.
            ("/home/u/.steam/steam/steamapps/common", None),
            // The bare appid, with the SteamID64 in between.
            (
                "/home/u/.local/share/Steam/userdata/76561197960287930/2059170/remote",
                None,
            ),
            // ---- and what must not break while fixing the above
            //
            // The same Steam tree, but with the game inside: it climbs from the
            // save word and stops at the title.
            (
                "/home/u/.steam/steam/steamapps/common/Stellaris/save games",
                Some("Stellaris"),
            ),
            // A game the catalogue does not know, under the `user` account: the
            // account's veto must not take the title below it down with it.
            (
                "C:/Users/user/Documents/My Games/Frobnicate Deluxe/Saves",
                Some("Frobnicate Deluxe"),
            ),
        ];

        for (raw, expected) in cases {
            let got = attribute_game_name(Path::new(raw), &empty);
            assert_eq!(got.as_deref(), *expected, "{raw}\n{}", catalog_source());
        }
    }

    /// Attribution (phase 4): the name of the process that wrote the folder beats
    /// the ancestor heuristic; with no process, the first non-generic segment
    /// climbing the tree is used.
    #[test]
    fn attribute_game_name_prefers_process_then_ancestor() {
        let path = PathBuf::from("/home/u/.local/share/Skyrim/Saves");

        // Sin correlación: sube desde `Saves` (save-word) hasta `Skyrim`.
        let empty = CorrelationStore::default();
        assert_eq!(
            attribute_game_name(&path, &empty).as_deref(),
            Some("Skyrim")
        );

        // Con correlación: el proceso atribuido manda.
        let mut store = CorrelationStore::default();
        store.record(
            &path,
            &[crate::correlation::GameProcess {
                name: "EldenRing.exe".into(),
                exe: None,
            }],
        );
        assert_eq!(
            attribute_game_name(&path, &store).as_deref(),
            Some("EldenRing")
        );
    }

    /// The catalogue's title beats the process's raw name. That is what stabilises
    /// attribution: the process that wins the correlation changes between scans (one
    /// game's folder went through three different app names) and each new name
    /// created a new slug and a new row.
    #[test]
    fn attribution_prefers_the_catalog_title_over_the_process_name() {
        let path = PathBuf::from("/home/u/.factorio/saves");
        let mut store = CorrelationStore::default();
        store.record(
            &path,
            &[crate::correlation::GameProcess {
                name: "factorio.exe".into(),
                exe: None,
            }],
        );
        let expected =
            ludusavi::title_for_exe("factorio.exe").expect("el manifiesto declara este ejecutable");
        assert_eq!(
            attribute_game_name(&path, &store).as_deref(),
            Some(expected)
        );
        // And the resulting slug is stable, which is what avoids the new row.
        assert_eq!(ludusavi::slugify(expected), "factorio");
    }

    /// A real case from a user's Windows (2026-07-30): `Mars.exe` is claimed by TWO
    /// catalogue games, so the ambiguity veto rejects it, which is right, because
    /// guessing would have christened the save "Mars Underground". What was wrong was
    /// falling back to the raw name with two signals in front of it that the catalogue
    /// does confirm: the executable's folder and the save's ancestor. The result was a
    /// game called "Mars" and a second amber row for the SAME folder under the good
    /// slug.
    #[test]
    fn an_ambiguous_exe_falls_back_to_the_catalog_not_to_the_raw_name() {
        let save = PathBuf::from("/home/u/Saved Games/Surviving Mars Relaunched/76561197960271872");
        let mut store = CorrelationStore::default();
        store.record(
            &save,
            &[crate::correlation::GameProcess {
                name: "Mars.exe".into(),
                exe: Some(PathBuf::from("/games/Surviving Mars Relaunched/Mars.exe")),
            }],
        );
        // The veto still stands: `mars.exe` does not resolve on its own.
        assert!(
            ludusavi::title_for_exe("mars.exe").is_none(),
            "mars.exe lo reclama más de un juego; no debe resolver"
        );
        let name = attribute_game_name(&save, &store).expect("la ruta nombra un juego");
        assert_eq!(name, "Surviving Mars: Relaunched", "{}", catalog_source());
        assert_eq!(ludusavi::slugify(&name), "surviving-mars-relaunched");
    }

    /// What the data being measured against is, for the message of a test that
    /// matches specific manifest titles.
    ///
    /// These tests assert things about data that comes from outside, and the
    /// catalogue that gets loaded is not always the one the binary ships: if the app
    /// has refreshed the manifest, the files in `~/.cache/hoard/` win.
    ///
    /// It only reports the loaded size: checking whether the override file exists
    /// says nothing here, because the test points `XDG_CACHE_HOME` at its tempdir at
    /// this instant and the catalogue is already loaded from before.
    fn catalog_source() -> String {
        format!(
            "catálogo cargado: {} juegos. Si esto falla en local y pasa en CI, la app refrescó \
             el manifiesto en ~/.cache/hoard y esos datos mandan; para medir como CI: \
             XDG_CACHE_HOME=$(mktemp -d) cargo test",
            ludusavi::catalog_size()
        )
    }

    /// And with the save's folder opaque, the executable's is enough.
    #[test]
    fn the_executables_install_folder_names_the_game() {
        let save = PathBuf::from("/home/u/.local/share/a1b2c3d4-guid/data");
        let mut store = CorrelationStore::default();
        store.record(
            &save,
            &[crate::correlation::GameProcess {
                name: "Mars.exe".into(),
                exe: Some(PathBuf::from("/opt/Surviving Mars Relaunched/Mars.exe")),
            }],
        );
        assert_eq!(
            attribute_game_name(&save, &store).as_deref(),
            Some("Surviving Mars: Relaunched"),
            "{}",
            catalog_source()
        );
    }

    /// A folder named after a catalogue game comes out with its pretty title, not
    /// with the directory's raw name.
    #[test]
    fn attribution_upgrades_a_folder_name_to_its_catalog_title() {
        let path = PathBuf::from("/home/u/.local/share/StardewValley/Saves");
        let name = attribute_game_name(&path, &CorrelationStore::default());
        assert_eq!(name.as_deref(), Some("Stardew Valley"));
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

    /// Phase 4 end to end: a folder with an opaque name (a GUID) under a save root
    /// scores as possible, the correlation corroborates it, and it is attributed to
    /// the process that wrote it.
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
            // The process name is prettified for the Library (title case); the slug
            // comes from slugifying the display, so it does not change.
            assert_eq!(hit.display_name, "Mysterygame");
            assert_eq!(hit.slug, "mysterygame");

            // If the catalogue already claims it, it is skipped.
            let mut known = HashSet::new();
            known.insert(guid.clone());
            let skipped = discover_unattributed(Os::Linux, &store, &known);
            assert!(skipped.iter().all(|a| a.path != guid));
        });
    }

    /// A regression: Hoard's own conflict backups are copied verbatim, so they score
    /// save-like and, before the fix, surfaced as phantom games named after the
    /// timestamp. The walk has to skip them even when correlation corroborates them.
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

    /// The Library "add from folder" flow: scanning an arbitrary folder (not a
    /// standard save root) must find a save-like dir inside it and attribute it
    /// to the game named by the folder above the save-word: no catalog, no
    /// correlation. A folder already covered by a tracked save is skipped.
    #[test]
    fn discover_in_folder_finds_and_attributes_inside_arbitrary_dir() {
        let uniq = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = std::env::temp_dir().join(format!("hoard-scan-folder-{uniq}"));
        let saves = base.join("MyGame").join("Saves");
        std::fs::create_dir_all(&saves).unwrap();
        std::fs::write(saves.join("slot1.sav"), b"savedata").unwrap();

        let store = CorrelationStore::default();
        let found = discover_in_folder(&base, &store, &HashSet::new());
        let hit = found
            .iter()
            .find(|a| a.path == saves)
            .expect("save-like dir inside the chosen folder should surface");
        assert_eq!(hit.display_name, "MyGame");
        assert_eq!(hit.slug, "mygame");

        // Already tracked ⇒ filtered out.
        let mut known = HashSet::new();
        known.insert(saves.clone());
        let skipped = discover_in_folder(&base, &store, &known);
        assert!(skipped.iter().all(|a| a.path != saves));

        let _ = std::fs::remove_dir_all(&base);
    }

    /// A unique temporary folder for the explicit-sweep tests.
    fn scratch_dir(tag: &str) -> PathBuf {
        let uniq = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("hoard-{tag}-{uniq}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Regresión: apuntar directamente a la carpeta de partidas de un juego
    /// (`…/Saved Games/Surviving Mars Relaunched`) no encontraba nada, porque
    /// the walk only classified the CHILDREN of the chosen root. The folder that
    /// el usuario señala es un candidato como cualquier otro.
    #[test]
    fn discover_in_folder_offers_the_chosen_folder_itself() {
        let base = scratch_dir("scan-self");
        let game = base.join("Surviving Mars Relaunched");
        std::fs::create_dir_all(&game).unwrap();
        std::fs::write(game.join("MyColony.sav"), b"savedata").unwrap();

        let store = CorrelationStore::default();
        let found = discover_in_folder(&game, &store, &HashSet::new());
        assert!(
            found.iter().any(|a| a.path == game),
            "the folder the user picked must be offered: {:?}",
            found.iter().map(|a| &a.path).collect::<Vec<_>>()
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    /// Una carpeta puede ser save Y contenedor: `Documents` tiene ficheros
    /// loose files and a folder per game inside. It is offered (it is the chosen
    /// one) but without hiding what is underneath.
    #[test]
    fn discover_in_folder_descends_past_a_chosen_folder_that_also_holds_files() {
        let base = scratch_dir("scan-both");
        std::fs::write(base.join("notes.txt"), b"x").unwrap();
        let game = base.join("My Games").join("Stellaris");
        std::fs::create_dir_all(&game).unwrap();
        std::fs::write(game.join("empire.sav"), b"savedata").unwrap();

        let store = CorrelationStore::default();
        let found = discover_in_folder(&base, &store, &HashSet::new());
        assert!(
            found.iter().any(|a| a.path == game),
            "loose files in the chosen folder must not hide the games under it: {:?}",
            found.iter().map(|a| &a.path).collect::<Vec<_>>()
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    /// Regresión: escanear `Saved Games` devolvía un juego de cuatro, porque
    /// a save folder with a proprietary extension does not clear the scored sweep's
    /// bar.
    ///
    /// It also covers the `desktop.ini` Windows leaves in `Saved Games`: without
    /// filtering it, the container would count as a folder with data of its own and
    /// would hide the four games inside.
    #[test]
    fn discover_in_folder_lists_every_game_under_a_container() {
        let base = scratch_dir("scan-container");
        std::fs::write(base.join("desktop.ini"), b"[.ShellClassInfo]").unwrap();
        for (game, file) in [
            ("JWE 3", "slot0.sav"),
            ("Surviving Mars Relaunched", "colony.autosave"),
            ("Shift At Midnight", "profile.pss"),
            ("Planet S", "world.dat"),
        ] {
            let dir = base.join(game);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join(file), b"savedata").unwrap();
        }

        let store = CorrelationStore::default();
        let found = discover_in_folder(&base, &store, &HashSet::new());
        assert_eq!(
            found.len(),
            4,
            "every game folder must surface, not just the ones with a known save extension: {:?}",
            found.iter().map(|a| &a.path).collect::<Vec<_>>()
        );
        assert!(
            found.iter().all(|a| a.path != base),
            "the container itself is not a save folder; its desktop.ini doesn't make it one"
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    /// Config, cache and logs are not saves however many files they hold, and a
    /// carpeta de capturas tampoco.
    #[test]
    fn discover_in_folder_skips_config_and_screenshots() {
        let base = scratch_dir("scan-negative");
        for (dir, file) in [
            ("Config", "settings.ini"),
            ("Logs", "run.log"),
            ("Screenshots", "shot.png"),
            ("Saves", "slot1.sav"),
        ] {
            let d = base.join(dir);
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join(file), b"x").unwrap();
        }

        let store = CorrelationStore::default();
        let found = discover_in_folder(&base, &store, &HashSet::new());
        let paths: Vec<_> = found.iter().map(|a| a.path.clone()).collect();
        assert_eq!(paths, vec![base.join("Saves")], "got {paths:?}");

        let _ = std::fs::remove_dir_all(&base);
    }

    /// El calificador de la carpeta (`Relaunched`, `Definitive Edition`…) no
    /// puede inventar un juego nuevo cuando el catálogo tiene el título dentro.
    #[test]
    fn name_prefix_resolves_edition_folders_to_the_catalog_game() {
        // Against the real catalogue: if "Surviving Mars" is in it, the qualified
        // folder has to resolve to that title rather than to itself.
        let Some(entry) = ludusavi::find_by_slug("surviving-mars") else {
            return; // catálogo recortado en este build: nada que comprobar
        };
        let resolved = ludusavi::find_by_name_prefix("Surviving Mars Relaunched");
        assert_eq!(resolved.map(|e| &e.slug), Some(&entry.slug));
        // And the guardrail: a single-word title does not swallow another game.
        assert!(
            ludusavi::find_by_name_prefix("Fallout New Vegas").is_none_or(|e| e.slug != "fallout")
        );
    }

    // ---- P2/P9: backup mirrors (DETECCION-REVISION §4 R3, §8) -------------
    //
    // Every fixture lives in a tempdir and touches neither the catalog nor
    // XDG_CACHE_HOME: the only input is the tree it builds itself. That
    // isolation is deliberate: a refreshed `~/.cache/hoard` has silently
    // decided other tests in this file before.

    /// A tree with `n` saves sharing their names under each given root.
    fn write_saves(base: &std::path::Path, files: &[&str]) {
        for f in files {
            let p = base.join(f);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, b"x").unwrap();
        }
    }

    fn tracked_state(slug: &str, label: &str, path: &Path) -> CliState {
        let mut state = CliState::default();
        state.saves.insert(
            "save-1".to_string(),
            crate::state::SaveState {
                local_path: path.to_path_buf(),
                game_slug: slug.to_string(),
                label: label.to_string(),
                last_backup_at: None,
                last_version_num: None,
                paused: false,
                preset: None,
                allow_device_local: None,
                set_hash: None,
                processes: Vec::new(),
                shared_processes: false,
            },
        );
        state
    }

    #[test]
    fn mirror_needs_suffix_relation_and_content_superset() {
        let tmp = tempfile::tempdir().unwrap();
        let orig = tmp.path().join("Saves");
        let twin = tmp.path().join("SavesOld");
        write_saves(&orig, &["slot1.sav", "slot2.sav", "slot3.sav"]);
        write_saves(&twin, &["slot1.sav", "slot2.sav", "slot3.sav"]);

        assert!(is_backup_mirror(&twin, &orig), "full twin must match");
        assert!(
            !is_backup_mirror(&orig, &twin),
            "the relation is directed: the ORIGINAL never mirrors its copy"
        );

        // Prefix, not suffix: `BackupSaves` next to `Saves` is no twin.
        let prefixed = tmp.path().join("BackupSaves");
        write_saves(&prefixed, &["slot1.sav"]);
        assert!(!is_backup_mirror(&prefixed, &orig));
    }

    #[test]
    fn wukong_shape_mirror_is_recognised_across_two_levels() {
        // The incident's exact shape: the copy wraps the real save's parent,
        // and its saves hang two levels further down inside it.
        let tmp = tempfile::tempdir().unwrap();
        let saved = tmp.path().join("Saved");
        let real = saved.join("SaveGames").join("76561199002555123");
        let mirror = saved.join("SaveGamesBackup");
        write_saves(&real, &["slot.sav", "profile.sav", "meta.sav"]);
        write_saves(
            &mirror.join("01RealtimeBackup").join("2026-08-20_104233"),
            &["slot.sav", "profile.sav", "meta.sav"],
        );
        write_saves(
            &mirror.join("02HourlyBackup").join("2026-08-20_110000"),
            &["slot.sav", "profile.sav", "meta.sav"],
        );
        assert!(is_backup_mirror(&mirror, &real));
    }

    #[test]
    fn a_bak_sibling_without_the_content_relation_is_not_a_mirror() {
        // The negative case that makes the veto safe: `-bak` with no superset
        // (aquí ni siquiera contiene un save) no altera nada.
        let tmp = tempfile::tempdir().unwrap();
        let orig = tmp.path().join("NobodyT");
        let bak = tmp.path().join("NobodyT-bak");
        write_saves(&orig, &["slot.sav"]);
        write_saves(&bak, &["notes.txt"]);

        assert!(!is_backup_mirror(&bak, &orig));
    }

    #[test]
    fn an_empty_original_cannot_condemn_its_suffixed_neighbour() {
        // With no content to compare, the superset would hold vacuously and
        // any suffixed sibling would be condemned by nothing at all.
        let tmp = tempfile::tempdir().unwrap();
        let orig = tmp.path().join("Saves");
        std::fs::create_dir_all(&orig).unwrap();
        let twin = tmp.path().join("SavesOld");
        write_saves(&twin, &["whatever.sav"]);
        assert!(!is_backup_mirror(&twin, &orig));
    }

    /// The incident fixture down the refinement path: the mirror arrives as a
    /// catalog hit and must not come out of `refine_save_dir` alive.
    #[test]
    fn refine_drops_the_wukong_mirror_but_keeps_the_real_save() {
        let tmp = tempfile::tempdir().unwrap();
        let saved = tmp.path().join("Saved");
        let real_dir = saved.join("SaveGames").join("76561199002555123");
        let mirror = saved.join("SaveGamesBackup");
        write_saves(&real_dir, &["slot.sav", "profile.sav", "meta.sav"]);
        write_saves(
            &mirror.join("02HourlyBackup").join("2026-08-20_104233"),
            &["slot.sav", "profile.sav", "meta.sav"],
        );
        // Exactly as the pipeline delivers them: the `*.sav` template yields
        // the FILE, and `SaveGamesBackup` the whole directory.
        let hits = vec![real_dir.join("slot.sav"), mirror.clone()];
        let refined = refine_save_dir("black-myth-wukong", hits);
        assert_eq!(
            refined,
            vec![real_dir.clone()],
            "the mirror must be gone and the real save kept"
        );
    }

    /// Brief check 3: with both folders candidate, the real one leads
    /// `found_paths` even when the mirror ties on confidence, and the
    /// razones viajan alineadas (P1).
    #[test]
    fn wukong_ranking_leads_with_the_real_save_and_records_why() {
        let tmp = tempfile::tempdir().unwrap();
        let real = tmp.path().join("SaveGames").join("76561199002555123");
        let mirror = tmp.path().join("SaveGamesBackup");
        write_saves(&real, &["slot.sav", "profile.sav", "meta.sav"]);
        write_saves(
            &mirror.join("01RealtimeBackup").join("2026-08-20_104233"),
            &["slot.sav", "profile.sav", "meta.sav"],
        );
        let mut map = HashMap::new();
        map.insert(
            "black-myth-wukong".to_string(),
            DetectedGame {
                slug: "black-myth-wukong".into(),
                display_name: "Black Myth: Wukong".into(),
                // Catalog discovery order: the mirror came first. Ranking has
                // to correct that.
                found_paths: vec![mirror.clone(), real.clone()],
                path_confidences: Vec::new(),
                path_reasons: Vec::new(),
                confidence: Confidence::Medium,
                source: DetectionSource::FilesystemHeuristic,
                steam_app_id: Some(2358720),
                install_dir: None,
                needs_folder: false,
                steam_cloud: true,
            },
        );
        grade_and_rank_paths(&mut map, &CorrelationStore::default());
        let g = &map["black-myth-wukong"];
        assert_eq!(g.found_paths[0], real, "the real save leads");
        assert_eq!(g.found_paths[1], mirror, "the mirror falls behind");
        assert_eq!(g.found_paths.len(), g.path_confidences.len());
        assert_eq!(g.found_paths.len(), g.path_reasons.len());
        // P1: the winner's reason explains the pick, so it is not empty.
        assert!(
            !g.path_reasons[0].is_empty(),
            "the leading path must carry its why"
        );
    }

    /// Brief check 5: a row ALREADY tracking the mirror produces the warning
    /// pointing at the right sibling, without mutating any state.
    #[test]
    fn a_tracked_mirror_warns_with_the_right_sibling() {
        let tmp = tempfile::tempdir().unwrap();
        let real = tmp.path().join("Saved").join("SaveGames").join("uid1");
        let mirror = tmp.path().join("Saved").join("SaveGamesBackup");
        write_saves(&real, &["slot.sav", "profile.sav", "meta.sav"]);
        write_saves(
            &mirror.join("03DailyBackup").join("2026-08-20"),
            &["slot.sav", "profile.sav", "meta.sav"],
        );
        let state = tracked_state("black-myth-wukong", "main", &mirror);
        let games = vec![DetectedGame {
            slug: "black-myth-wukong".into(),
            display_name: "Black Myth: Wukong".into(),
            found_paths: vec![real.clone()],
            path_confidences: vec![Confidence::High],
            path_reasons: Vec::new(),
            confidence: Confidence::High,
            source: DetectionSource::FilesystemHeuristic,
            steam_app_id: Some(2358720),
            install_dir: None,
            needs_folder: false,
            steam_cloud: true,
        }];
        let warnings = detect_tracked_mirrors(&state, &games);
        assert_eq!(warnings.len(), 1, "one warning for the tracked mirror");
        let w = &warnings[0];
        assert_eq!(w.save_id, "save-1");
        assert_eq!(w.tracked_path, mirror);
        assert_eq!(w.suggested_path, real, "must point at the REAL sibling");
        assert!(w.reason.starts_with("mirror of"));
        // Read-only by construction (&CliState); the assert makes it explicit.
        assert_eq!(
            state.saves["save-1"].local_path, mirror,
            "nothing may re-point the row by itself"
        );
    }

    #[test]
    fn a_tracked_folder_without_a_better_sibling_stays_quiet() {
        let tmp = tempfile::tempdir().unwrap();
        let real = tmp.path().join("saves");
        write_saves(&real, &["slot.sav"]);
        let state = tracked_state("factorio", "main", &real);
        let warnings = detect_tracked_mirrors(&state, &[]);
        assert!(warnings.is_empty());
    }
}
