//! End-to-end tests for the detection pipeline.
//!
//! Each test pins `HOME` / `XDG_*` (or the Windows equivalents) to a
//! tempdir, materialises the minimum on-disk shape required to exercise one
//! branch of [`hoard_agent::detection::detect_all`], runs the full pipeline
//! against the embedded Ludusavi catalog, and asserts on the resulting
//! [`hoard_agent::detection::DetectionReport`]. The host's real saves,
//! Steam install, or Proton prefixes are never read — every fixture lives
//! under a `tempfile::TempDir` that drops at end-of-test.
//!
//! Coverage matrix (see `docs/plans/detection.md` §3, §4.1 and P-DET-7):
//!
//! | Test                                              | Branch                              |
//! |---------------------------------------------------|-------------------------------------|
//! | `fs_heuristic_finds_native_linux_save`            | filesystem heuristic, native Linux  |
//! | `fs_heuristic_finds_windows_appdata_save`         | filesystem heuristic, Windows       |
//! | `proton_prefix_finds_windows_only_game_on_linux`  | Proton prefix expand on Linux       |
//! | `refine_drops_paradox_root_without_save_games`    | refine drops bare game root         |
//! | `refine_promotes_paradox_save_games_subdir`       | refine collapses root → save subdir |
//! | `steam_name_fallback_picks_up_no_appid_entries`   | Steam → catalog slug fallback       |
//! | `manual_override_wins_over_heuristic`             | `CliState::manual_paths` override   |
//!
//! All tests share `ENV_LOCK` because they mutate process-wide environment
//! variables; cargo runs the tests in this binary on multiple threads by
//! default and an unguarded `set_var` race would corrupt other tests' view
//! of the world. The crate's own `test_lock::ENV` is `pub(crate)` and not
//! reachable from an integration-test binary, but each test binary is a
//! separate process so a local mutex is enough.

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use hoard_agent::detection::{detect_all, Confidence, DetectionSource};
use hoard_agent::manifest::Os;
use hoard_agent::pathexpand::expand_path;
use hoard_agent::state::CliState;
use hoard_manifest::ludusavi;

/// Serialises every test in this binary that mutates the process
/// environment. `set_var` is process-global and tokio's current-thread
/// runtime is fine running serially, so taking the lock for the duration
/// of each test keeps the cost bounded and the races impossible.
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Env vars Linux detection reads. Cleared/replaced under
/// [`with_isolated_linux_env`] so a real `$HOME` on the test host can't
/// leak into the fixture.
const LINUX_ENV_VARS: &[&str] = &[
    "HOME",
    "XDG_DATA_HOME",
    "XDG_CONFIG_HOME",
    "XDG_STATE_HOME",
    "XDG_CACHE_HOME",
];

/// Env vars Windows detection reads (path expansion + Steam root probing).
/// Cleared/replaced under [`with_isolated_windows_env`].
const WINDOWS_ENV_VARS: &[&str] = &[
    "APPDATA",
    "LOCALAPPDATA",
    "USERPROFILE",
    "PUBLIC",
    "PROGRAMDATA",
    "WINDIR",
    "ProgramFiles(x86)",
    "ProgramFiles",
    "PROGRAMFILES",
    // Cleared too so the cross-platform `<home>` placeholder and Steam's
    // Linux scan can't see the test host's real HOME during a Windows test.
    "HOME",
    "XDG_DATA_HOME",
    "XDG_CONFIG_HOME",
    "XDG_STATE_HOME",
    "XDG_CACHE_HOME",
];

fn snapshot_env(names: &[&'static str]) -> Vec<(&'static str, Option<OsString>)> {
    names.iter().map(|n| (*n, std::env::var_os(n))).collect()
}

fn restore_env(prev: Vec<(&'static str, Option<OsString>)>) {
    for (name, value) in prev {
        match value {
            Some(v) => std::env::set_var(name, v),
            None => std::env::remove_var(name),
        }
    }
}

/// Run `f` with `HOME` + the four XDG dirs pinned under a fresh tempdir.
/// Restores the previous environment on exit even on panic, and holds
/// `ENV_LOCK` for the whole call so concurrent tests can't race the env.
fn with_isolated_linux_env<F: FnOnce(&Path)>(f: F) {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let tmp = tempfile::tempdir().expect("create tempdir");
    let prev = snapshot_env(LINUX_ENV_VARS);
    std::env::set_var("HOME", tmp.path());
    std::env::set_var("XDG_DATA_HOME", tmp.path().join("xdg-data"));
    std::env::set_var("XDG_CONFIG_HOME", tmp.path().join("xdg-config"));
    std::env::set_var("XDG_STATE_HOME", tmp.path().join("xdg-state"));
    std::env::set_var("XDG_CACHE_HOME", tmp.path().join("xdg-cache"));

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(tmp.path())));

    restore_env(prev);
    if let Err(e) = result {
        std::panic::resume_unwind(e);
    }
}

/// Run `f` with the Windows path env vars pinned under a tempdir and the
/// Steam-discovery vars cleared so the host's real Steam install is
/// invisible. Cross-platform tokens (`HOME`, XDG) are wiped for the same
/// reason. Holds `ENV_LOCK` and restores on panic just like
/// [`with_isolated_linux_env`].
fn with_isolated_windows_env<F: FnOnce(&Path)>(f: F) {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let tmp = tempfile::tempdir().expect("create tempdir");
    let prev = snapshot_env(WINDOWS_ENV_VARS);

    for name in WINDOWS_ENV_VARS {
        std::env::remove_var(name);
    }
    let app_data = tmp.path().join("AppData/Roaming");
    let local_app_data = tmp.path().join("AppData/Local");
    std::fs::create_dir_all(&app_data).unwrap();
    std::fs::create_dir_all(&local_app_data).unwrap();
    std::env::set_var("APPDATA", &app_data);
    std::env::set_var("LOCALAPPDATA", &local_app_data);
    std::env::set_var("USERPROFILE", tmp.path());

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(tmp.path())));

    restore_env(prev);
    if let Err(e) = result {
        std::panic::resume_unwind(e);
    }
}

/// Single-thread tokio runtime — detect_all is async and the tests run it
/// to completion synchronously inside the env-guarded scope.
fn block_on_detect(os: Os, state: &CliState) -> hoard_agent::detection::DetectionReport {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build runtime");
    rt.block_on(detect_all(os, state, |_, _| {}))
        .expect("detect_all should not error")
}

/// Look up an entry in the embedded catalog by slug. Tests rely on a
/// handful of well-known slugs (Stardew Valley, Stellaris); if any of them
/// disappears upstream we want a loud error here rather than a confusing
/// `Option::None` later.
fn catalog_entry(slug: &str) -> &'static ludusavi::LudusaviEntry {
    ludusavi::catalog()
        .iter()
        .find(|e| e.slug == slug)
        .unwrap_or_else(|| panic!("embedded Ludusavi catalog should contain slug {slug:?}"))
}

/// Materialise a synthetic Steam install under `home` with one library
/// (`~/.steam/steam`) and one `appmanifest_<appid>.acf` per `apps` entry.
/// Returns the `steamapps` path so callers can drop compatdata prefixes or
/// inspect the per-app `common/` layout in assertions.
fn build_steam_install(home: &Path, apps: &[(u64, &str, &str)]) -> PathBuf {
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
    for (appid, name, install_dir) in apps {
        std::fs::write(
            steamapps.join(format!("appmanifest_{appid}.acf")),
            format!(
                "\"AppState\"\n{{\n  \"appid\" \"{appid}\"\n  \"name\" \"{name}\"\n  \"installdir\" \"{install_dir}\"\n}}\n"
            ),
        )
        .unwrap();
    }
    steamapps
}

/// Create a Proton/Wine prefix under `<steamapps>/compatdata/<app_id>/pfx`
/// with the `drive_c/users/steamuser` skeleton in place so subsequent
/// `expand_path_in_prefix` writes land in a real-shaped layout. Returns
/// the `pfx/` path itself, which is what
/// [`hoard_agent::steam::ProtonPrefix::prefix_root`] points at.
fn build_compatdata_prefix(steamapps: &Path, app_id: u64) -> PathBuf {
    let pfx = steamapps.join(format!("compatdata/{app_id}/pfx"));
    std::fs::create_dir_all(pfx.join("drive_c/users/steamuser")).unwrap();
    pfx
}

/// Expand the first Linux save-path template for `slug`, panicking with a
/// descriptive message if either lookup fails. Returns the candidate path
/// the filesystem heuristic would stat.
fn first_linux_expansion(slug: &str) -> PathBuf {
    let entry = catalog_entry(slug);
    let template = entry
        .paths
        .linux
        .first()
        .unwrap_or_else(|| panic!("slug {slug} has no linux save-path templates"));
    expand_path(&template.path, Os::Linux)
        .into_iter()
        .next()
        .unwrap_or_else(|| {
            panic!(
                "linux template {:?} for {slug} did not expand",
                template.path
            )
        })
}

/// Sibling of [`first_linux_expansion`] for the Windows side.
fn first_windows_expansion(slug: &str) -> PathBuf {
    let entry = catalog_entry(slug);
    let template = entry
        .paths
        .windows
        .first()
        .unwrap_or_else(|| panic!("slug {slug} has no windows save-path templates"));
    expand_path(&template.path, Os::Windows)
        .into_iter()
        .next()
        .unwrap_or_else(|| {
            panic!(
                "windows template {:?} for {slug} did not expand",
                template.path
            )
        })
}

// =============================================================
// Tests
// =============================================================

/// Native-Linux save dir: the catalog template
/// `<xdgConfig>/StardewValley/Saves` expands under the pinned
/// `XDG_CONFIG_HOME` tempdir; creating the directory there should make the
/// filesystem heuristic report `stardew-valley` with that path and
/// `source = FilesystemHeuristic`.
#[test]
fn fs_heuristic_finds_native_linux_save() {
    with_isolated_linux_env(|_home| {
        let save_dir = first_linux_expansion("stardew-valley");
        std::fs::create_dir_all(&save_dir).unwrap();

        let state = CliState::default();
        let report = block_on_detect(Os::Linux, &state);

        let game = report
            .games
            .iter()
            .find(|g| g.slug == "stardew-valley")
            .expect("stardew-valley should appear in the report");
        assert_eq!(game.source, DetectionSource::FilesystemHeuristic);
        assert_eq!(game.confidence, Confidence::Medium);
        assert!(
            game.found_paths.iter().any(|p| p == &save_dir),
            "found_paths should contain {save_dir:?}; got {:?}",
            game.found_paths,
        );
        assert!(
            game.steam_app_id.is_none(),
            "no Steam install in this fixture, steam_app_id should be None; got {:?}",
            game.steam_app_id
        );
    });
}

/// Same pipeline branch but on Windows: `<winAppData>` resolves to the
/// pinned `APPDATA` tempdir, the heuristic stats the resulting path, and
/// the slug is reported with `FilesystemHeuristic`. Catches regressions
/// where path expansion or refine logic diverges between OSes.
#[test]
fn fs_heuristic_finds_windows_appdata_save() {
    with_isolated_windows_env(|_home| {
        let save_dir = first_windows_expansion("stardew-valley");
        std::fs::create_dir_all(&save_dir).unwrap();

        let state = CliState::default();
        let report = block_on_detect(Os::Windows, &state);

        let game = report
            .games
            .iter()
            .find(|g| g.slug == "stardew-valley")
            .expect("stardew-valley should appear in the report");
        assert_eq!(game.source, DetectionSource::FilesystemHeuristic);
        assert_eq!(game.confidence, Confidence::Medium);
        assert!(
            game.found_paths.iter().any(|p| p == &save_dir),
            "found_paths should contain {save_dir:?}; got {:?}",
            game.found_paths,
        );
    });
}

/// The headline P-DET-1 behaviour: a Windows-only game (Stardew Valley
/// here, appid 413150) shows up on a Linux host because Steam has a
/// Proton prefix for it under `compatdata/`. `source = Both` confirms the
/// Steam cross-reference fired alongside the prefix expand; the save
/// path under the prefix lands in `found_paths`.
#[test]
fn proton_prefix_finds_windows_only_game_on_linux() {
    with_isolated_linux_env(|home| {
        let steamapps = build_steam_install(home, &[(413150, "Stardew Valley", "Stardew Valley")]);
        let pfx = build_compatdata_prefix(&steamapps, 413150);
        let save_dir = pfx.join("drive_c/users/steamuser/AppData/Roaming/StardewValley/Saves");
        std::fs::create_dir_all(&save_dir).unwrap();

        let state = CliState::default();
        let report = block_on_detect(Os::Linux, &state);

        let game = report
            .games
            .iter()
            .find(|g| g.slug == "stardew-valley")
            .expect("stardew-valley should appear in the report");
        assert_eq!(game.source, DetectionSource::Both);
        assert_eq!(game.confidence, Confidence::High);
        assert_eq!(game.steam_app_id, Some(413150));
        assert!(
            game.found_paths.iter().any(|p| p == &save_dir),
            "found_paths should contain the prefix save dir {save_dir:?}; got {:?}",
            game.found_paths,
        );
    });
}

/// Stellaris's catalog template `<xdgData>/Paradox Interactive/Stellaris`
/// expands to a game root that mixes saves with config and mods. With
/// only `mod/` and `settings/` populated (no `save games/` yet — common
/// for a fresh install before the user starts a campaign), the refine
/// step must drop every raw hit. `root_matched` is still true, so the
/// slug appears with empty `found_paths` and the UI surfaces an amber
/// "pick folder" alert instead of tracking the bare root.
#[test]
fn refine_drops_paradox_root_without_save_games() {
    with_isolated_linux_env(|_home| {
        let root = first_linux_expansion("stellaris");
        std::fs::create_dir_all(root.join("mod")).unwrap();
        std::fs::create_dir_all(root.join("settings")).unwrap();

        let state = CliState::default();
        let report = block_on_detect(Os::Linux, &state);

        let game = report
            .games
            .iter()
            .find(|g| g.slug == "stellaris")
            .expect("stellaris should appear with empty found_paths, not be dropped");
        assert!(
            game.found_paths.is_empty(),
            "stellaris should have empty found_paths (UI would show amber); got {:?}",
            game.found_paths,
        );
        assert_eq!(game.source, DetectionSource::FilesystemHeuristic);
    });
}

/// Same fixture as above but with `save games/` created. The general
/// refine heuristic must collapse the raw root hit down to the save
/// subdirectory — never propose the bare root, never silently drop the
/// game. Regression guard for Paradox-family layouts (CK3, EU4, HoI4,
/// Imperator, Victoria 3) that historically caused Hoard to back up the
/// entire game tree.
#[test]
fn refine_promotes_paradox_save_games_subdir() {
    with_isolated_linux_env(|_home| {
        let root = first_linux_expansion("stellaris");
        let save_games = root.join("save games");
        std::fs::create_dir_all(&save_games).unwrap();
        std::fs::create_dir_all(root.join("mod")).unwrap();
        std::fs::create_dir_all(root.join("settings")).unwrap();

        let state = CliState::default();
        let report = block_on_detect(Os::Linux, &state);

        let game = report
            .games
            .iter()
            .find(|g| g.slug == "stellaris")
            .expect("stellaris should appear with the refined subdir");
        assert!(
            game.found_paths.iter().any(|p| p == &save_games),
            "found_paths should include the save-games subdir {save_games:?}; got {:?}",
            game.found_paths,
        );
        assert!(
            !game.found_paths.iter().any(|p| p == &root),
            "found_paths must NOT include the bare root {root:?}; got {:?}",
            game.found_paths,
        );
    });
}

/// `apply_steam_name_fallback` end-to-end: pick a real catalog entry
/// without `steam_app_id` and emit a synthetic `appmanifest_*.acf` whose
/// display name slugifies to the entry's slug. The fallback inserts a
/// row with `Confidence::Low` and `source = SteamLibrary`, even though
/// the appid cross-reference produced nothing.
#[test]
fn steam_name_fallback_picks_up_no_appid_entries() {
    with_isolated_linux_env(|home| {
        // Scan the catalog at runtime so this test survives upstream
        // churn — the first qualifying entry is enough.
        let entry = ludusavi::catalog()
            .iter()
            .find(|e| {
                e.steam_app_id.is_none()
                    && !e.display_name.is_empty()
                    && ludusavi::slugify(&e.display_name) == e.slug
            })
            .expect(
                "catalog must contain at least one no-appid entry whose name slugifies cleanly",
            );

        // Synthetic appid that no catalog entry claims, so the appid
        // cross-reference can't pre-empt the fallback path.
        let mut appid: u64 = 90_000_001;
        while ludusavi::catalog()
            .iter()
            .any(|e| e.steam_app_id == Some(appid))
        {
            appid += 1;
        }

        let install_dir = "FallbackInstallDir";
        let steamapps = build_steam_install(home, &[(appid, &entry.display_name, install_dir)]);

        let state = CliState::default();
        let report = block_on_detect(Os::Linux, &state);

        let game = report
            .games
            .iter()
            .find(|g| g.slug == entry.slug)
            .unwrap_or_else(|| {
                panic!(
                    "slug fallback should surface {} (display name {:?})",
                    entry.slug, entry.display_name
                )
            });
        assert_eq!(game.source, DetectionSource::SteamLibrary);
        assert_eq!(game.confidence, Confidence::Low);
        assert_eq!(game.steam_app_id, Some(appid));
        assert_eq!(
            game.install_dir.as_deref(),
            Some(steamapps.join("common").join(install_dir).as_path()),
        );
        assert!(
            game.found_paths.is_empty(),
            "no save folder on disk → found_paths should be empty; got {:?}",
            game.found_paths,
        );
    });
}

/// `CliState::manual_paths` beats every heuristic. Set up a heuristic
/// hit at the catalog-expanded native-Linux path AND a manual override
/// pointing at a completely different directory; the report must show
/// the override path with `source = ManualOverride` and `confidence =
/// High`, with the heuristic path absent from `found_paths`.
#[test]
fn manual_override_wins_over_heuristic() {
    with_isolated_linux_env(|home| {
        let heuristic_dir = first_linux_expansion("stardew-valley");
        std::fs::create_dir_all(&heuristic_dir).unwrap();

        let override_dir = home.join("user-picked/stardew");
        std::fs::create_dir_all(&override_dir).unwrap();

        let mut manual_paths = HashMap::new();
        manual_paths.insert("stardew-valley".to_string(), override_dir.clone());
        let state = CliState {
            manual_paths,
            ..CliState::default()
        };

        let report = block_on_detect(Os::Linux, &state);

        let game = report
            .games
            .iter()
            .find(|g| g.slug == "stardew-valley")
            .expect("stardew-valley should appear in the report");
        assert_eq!(game.source, DetectionSource::ManualOverride);
        assert_eq!(game.confidence, Confidence::High);
        assert_eq!(game.found_paths, vec![override_dir]);
        assert!(
            !game.found_paths.iter().any(|p| p == &heuristic_dir),
            "manual override must replace the heuristic path; got {:?}",
            game.found_paths,
        );
    });
}
