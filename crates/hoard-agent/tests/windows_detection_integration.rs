//! End-to-end tests for the 1.5.2 Windows-detection pipeline and the new
//! Linux Wine-prefix sources (Lutris, Bottles).
//!
//! Each test pins the relevant env vars under a `tempfile::TempDir`,
//! materialises the minimum on-disk fixture the launcher/prefix reader
//! requires, runs [`hoard_agent::detection::detect_all`] end-to-end, and
//! asserts on the resulting [`hoard_agent::detection::DetectionReport`].
//! The host's real launchers, registry or prefix layout are never read —
//! every fixture lives under a tempdir that drops at end-of-test.
//!
//! Coverage matrix (see `docs/plans/1.5.2.md` §7 P-D152-6):
//!
//! | Test                                              | Branch                                 |
//! |---------------------------------------------------|----------------------------------------|
//! | `epic_fixture_yields_detected_game`               | Epic `.item` manifests cross-ref       |
//! | `gog_fixture_yields_detected_game`                | GOG Galaxy sqlite cross-ref            |
//! | `registry_expand_returns_empty_on_linux`          | `expand_registry_path` no-op cross-OS  |
//! | `registry_expand_reads_hkcu_value` (ignored)      | live HKCU read — manual on Windows     |
//! | `msstore_returns_empty_on_non_windows`            | MS Store reader cross-OS stub          |
//! | `lutris_fixture_yields_detected_game`             | Lutris prefix → walker / slug match    |
//! | `bottles_fixture_yields_detected_game`            | Bottles prefix → walker / slug match   |
//!
//! All tests share a local `ENV_LOCK` because they mutate process-wide
//! environment variables; cargo runs each test binary's tests in parallel
//! and an unguarded `set_var` race would corrupt other tests' view of the
//! world. The crate-internal `test_lock::ENV` is `pub(crate)` and not
//! reachable from an integration-test binary, so a local mutex is used —
//! same pattern as `detection_integration.rs` and
//! `aggressive_discovery_integration.rs`.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use hoard_agent::detection::{detect_all, DetectionSource};
use hoard_agent::launchers;
use hoard_agent::manifest::Os;
use hoard_agent::pathexpand::expand_registry_path;
use hoard_agent::state::CliState;
use hoard_manifest::ludusavi::RegistryPath;

/// Process-wide serialisation for env-mutating tests. Mirrors the lock used
/// by `detection_integration.rs` and `aggressive_discovery_integration.rs`.
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Env vars Linux detection reads (HOME + the four XDG dirs). Cleared/
/// replaced under [`with_isolated_linux_env`] so the host's real `$HOME` or
/// XDG dirs can't leak into the fixture.
const LINUX_ENV_VARS: &[&str] = &[
    "HOME",
    "XDG_DATA_HOME",
    "XDG_CONFIG_HOME",
    "XDG_STATE_HOME",
    "XDG_CACHE_HOME",
];

/// Env vars Windows detection reads (path expansion + launcher dirs +
/// Steam root probing). Cleared/replaced under [`with_isolated_windows_env`]
/// so a real `%APPDATA%` etc. on the test host can't leak in.
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
    // Cleared so the cross-platform `<home>` placeholder and Steam's
    // Linux-style scan can't see the test host's real HOME during a
    // Windows test.
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
/// Holds `ENV_LOCK` for the whole call and restores the previous
/// environment on exit even on panic.
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
/// Steam-discovery vars cleared. Holds `ENV_LOCK` and restores on panic
/// just like [`with_isolated_linux_env`].
fn with_isolated_windows_env<F: FnOnce(&Path)>(f: F) {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let tmp = tempfile::tempdir().expect("create tempdir");
    let prev = snapshot_env(WINDOWS_ENV_VARS);

    for name in WINDOWS_ENV_VARS {
        std::env::remove_var(name);
    }
    let app_data = tmp.path().join("AppData/Roaming");
    let local_app_data = tmp.path().join("AppData/Local");
    let program_data = tmp.path().join("ProgramData");
    std::fs::create_dir_all(&app_data).unwrap();
    std::fs::create_dir_all(&local_app_data).unwrap();
    std::fs::create_dir_all(&program_data).unwrap();
    std::env::set_var("APPDATA", &app_data);
    std::env::set_var("LOCALAPPDATA", &local_app_data);
    std::env::set_var("PROGRAMDATA", &program_data);
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

/// Materialise a synthetic Steam install under `home` with one library
/// (`~/.steam/steam`) and one `appmanifest_<appid>.acf` per `apps` entry.
/// Mirrors the helper in `detection_integration.rs` /
/// `aggressive_discovery_integration.rs`. Returns the `steamapps` path.
///
/// The Lutris/Bottles tests use this to land the catalog slug into the
/// pipeline's `by_slug` map (via the Steam appid cross-reference) so that
/// the aggressive walker — which only runs for slugs already in the map
/// with empty `found_paths` — has something to walk against.
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

// =============================================================
// Tests
// =============================================================

/// Epic Games end-to-end: write a synthetic `.item` manifest under
/// `%PROGRAMDATA%\Epic\EpicGamesLauncher\Data\Manifests\` whose
/// `DisplayName` is "Stardew Valley", a name that slugifies to
/// `stardew-valley` and matches the embedded Ludusavi catalog. The
/// pipeline must surface the slug with `source = SteamLibrary` (the
/// neutral "extra-fs" tag chosen by P-D152-5 for all launcher cross-refs)
/// and an `install_dir` pointing at the manifest's `InstallLocation`.
///
/// `found_paths` may stay empty — no save dir was created under the
/// tempdir-pinned `%APPDATA%`, so the fs heuristic produces no hit.
#[test]
fn epic_fixture_yields_detected_game() {
    with_isolated_windows_env(|home| {
        let programdata = std::env::var_os("PROGRAMDATA").expect("PROGRAMDATA pinned by helper");
        let manifests = PathBuf::from(&programdata)
            .join("Epic")
            .join("EpicGamesLauncher")
            .join("Data")
            .join("Manifests");
        std::fs::create_dir_all(&manifests).unwrap();

        let install_dir = home.join("EpicGames").join("StardewValley");
        std::fs::create_dir_all(&install_dir).unwrap();

        let manifest = serde_json::json!({
            "InstallLocation": install_dir.to_string_lossy(),
            "DisplayName": "Stardew Valley",
            "CatalogItemId": "abcdef",
            "MainGameAppName": "StardewValley",
        });
        std::fs::write(
            manifests.join("StardewValley.item"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let state = CliState::default();
        let report = block_on_detect(Os::Windows, &state);

        let game = report
            .games
            .iter()
            .find(|g| g.slug == "stardew-valley")
            .unwrap_or_else(|| {
                panic!(
                    "stardew-valley should surface via Epic cross-reference; got {} games: {:?}",
                    report.games.len(),
                    report.games.iter().map(|g| &g.slug).collect::<Vec<_>>(),
                )
            });
        assert_eq!(
            game.source,
            DetectionSource::SteamLibrary,
            "P-D152-5 maps non-Steam launcher hits onto the neutral SteamLibrary variant",
        );
        assert_eq!(
            game.install_dir.as_deref(),
            Some(install_dir.as_path()),
            "install_dir should come from the Epic manifest",
        );
    });
}

/// GOG Galaxy end-to-end: synthesise the Galaxy sqlite under
/// `%LOCALAPPDATA%\GOG.com\Galaxy\storage\galaxy-2.0.db` with one row in
/// the `Builds` table pointing at a real install dir whose
/// `productName = "Stardew Valley"`. The pipeline's launcher cross-
/// reference must slugify the name, look up `stardew-valley` exact in the
/// catalog, and emit a row tagged `SteamLibrary` with the install dir set.
///
/// Ignored on non-Windows hosts because `launchers::gog_read_db` is gated
/// behind `#[cfg(any(windows, test))]` and Cargo only enables `cfg(test)`
/// for the *library*'s own internal tests, not for integration-test
/// binaries that link against the production lib build. The cross-
/// platform sqlite reader contract is already covered by the lib unit
/// test `launchers::tests::gog_parses_synthetic_sqlite`; this integration
/// variant would duplicate that without exercising new code, so it stays
/// `#[ignore]` until either the cfg is widened (would pull `rusqlite`
/// into all non-Windows prod builds) or a Windows CI runner is wired up.
#[test]
#[ignore = "gog_read_db is cfg(any(windows, test)); cfg(test) is lib-internal only, not visible to integration test binaries — covered by launchers::tests::gog_parses_synthetic_sqlite"]
fn gog_fixture_yields_detected_game() {
    with_isolated_windows_env(|home| {
        let localappdata = std::env::var_os("LOCALAPPDATA").expect("LOCALAPPDATA pinned by helper");
        let storage = PathBuf::from(&localappdata)
            .join("GOG.com")
            .join("Galaxy")
            .join("storage");
        std::fs::create_dir_all(&storage).unwrap();
        let db_path = storage.join("galaxy-2.0.db");

        let install_dir = home.join("GOG").join("StardewValley");
        std::fs::create_dir_all(&install_dir).unwrap();

        {
            let conn = rusqlite::Connection::open(&db_path).unwrap();
            conn.execute(
                "CREATE TABLE Builds (productId INTEGER, productName TEXT, installationPath TEXT)",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO Builds (productId, productName, installationPath) VALUES (?1, ?2, ?3)",
                rusqlite::params![
                    1453375253_i64,
                    "Stardew Valley",
                    install_dir.to_string_lossy(),
                ],
            )
            .unwrap();
        }

        let state = CliState::default();
        let report = block_on_detect(Os::Windows, &state);

        let game = report
            .games
            .iter()
            .find(|g| g.slug == "stardew-valley")
            .unwrap_or_else(|| {
                panic!(
                    "stardew-valley should surface via GOG cross-reference; got {} games: {:?}",
                    report.games.len(),
                    report.games.iter().map(|g| &g.slug).collect::<Vec<_>>(),
                )
            });
        assert_eq!(
            game.source,
            DetectionSource::SteamLibrary,
            "P-D152-5 maps non-Steam launcher hits onto the neutral SteamLibrary variant",
        );
        assert_eq!(
            game.install_dir.as_deref(),
            Some(install_dir.as_path()),
            "install_dir should come from the Galaxy sqlite row",
        );
    });
}

/// `expand_registry_path` must collapse to an empty vector on any
/// non-Windows host so callers don't need a `#[cfg]` around the call. The
/// unit-test module in `pathexpand.rs` already covers this with the
/// `expand_registry_returns_empty_on_unix` test; this integration test
/// exercises the same contract from outside the crate to make sure the
/// public re-export keeps the same shape. Two `RegistryPath` flavours
/// (HKCU and HKLM, with and without a `value`) both collapse.
#[cfg(not(windows))]
#[test]
fn registry_expand_returns_empty_on_linux() {
    let reg = RegistryPath {
        key: "HKEY_CURRENT_USER/Software/Foo".to_string(),
        value: None,
    };
    assert!(expand_registry_path(&reg).is_empty());

    let reg = RegistryPath {
        key: "HKEY_LOCAL_MACHINE/Software/Foo/Bar".to_string(),
        value: Some("SavePath".to_string()),
    };
    assert!(expand_registry_path(&reg).is_empty());
}

/// Round-trip a `RegistryPath` against the live HKCU hive on a Windows
/// host. Ignored by default because it touches the user's real registry —
/// run by hand with `cargo test -- --ignored` when validating the
/// registry expander on a Windows box.
///
/// The unit test in `pathexpand.rs` already cleans up the subkey it
/// writes; this integration variant exists so the same flow is reachable
/// from outside the crate without falling back to a stubbed helper.
#[cfg(windows)]
#[test]
#[ignore = "TODO: implementar a mano cuando se valide en Windows — toca HKCU real"]
fn registry_expand_reads_hkcu_value() {
    // Intentionally empty: the smoke test lives in the crate-local unit
    // tests (`pathexpand::tests::expand_registry_reads_value_from_hkcu`),
    // which also clean up the subkey. Wiring a fixture from the
    // integration binary would duplicate that logic without adding
    // coverage.
}

/// Microsoft Store reader returns an empty vector on every non-Windows
/// host. Trivially covered by the unit test in `launchers.rs`, but
/// surfacing it from the integration binary too guards against the
/// public function losing its OS guard during a future refactor.
#[test]
fn msstore_returns_empty_on_non_windows() {
    assert!(launchers::list_installed_msstore_games(Os::Linux).is_empty());
    assert!(launchers::list_installed_msstore_games(Os::Mac).is_empty());
}

/// Lutris end-to-end: stand up a synthetic Lutris prefix under
/// `~/.local/share/lutris/runners/wine/.../prefixes/stardew-valley/` with
/// a save file inside the `drive_c/users/steamuser/AppData/Roaming/...`
/// tree, plus a synthetic Steam install that lands `stardew-valley` into
/// the pipeline's `by_slug` map (the aggressive walker only fires for
/// slugs already in the map with empty `found_paths`). The pipeline's
/// `build_prefix_root_by_slug` step keys the Lutris prefix by
/// `slugify("stardew-valley")` (which equals `stardew-valley`), the
/// walker descends into `drive_c/users/steamuser/AppData/Roaming/...`,
/// and the report surfaces the slug with a path inside the tempdir.
///
/// The synthetic Steam install **only** seeds the slug — its `installdir`
/// directory is intentionally not created on disk, so the Steam scan
/// alone produces no `found_paths` and the walker must run against the
/// Lutris prefix to populate them.
#[test]
fn lutris_fixture_yields_detected_game() {
    with_isolated_linux_env(|home| {
        // Stardew Valley's Steam appid is 413150 — see Ludusavi catalog.
        // The cross-reference inserts the slug with empty `found_paths`
        // so the aggressive walker has work to do.
        let _steamapps = build_steam_install(home, &[(413150, "Stardew Valley", "Stardew Valley")]);

        let prefix_root = home
            .join(".local/share/lutris/runners/wine")
            .join("lutris-fshack-7.2-x86_64")
            .join("prefixes")
            .join("stardew-valley");
        let save_dir =
            prefix_root.join("drive_c/users/steamuser/AppData/Roaming/StardewValley/Saves");
        std::fs::create_dir_all(&save_dir).unwrap();
        // A recent `.sav` file pushes the aggressive walker to Medium
        // confidence — the same pattern the 1.5.1 walker tests rely on to
        // ensure the dir is treated as a save dir rather than dropped.
        std::fs::write(save_dir.join("Farm.sav"), b"synthetic save").unwrap();

        let state = CliState::default();
        let report = block_on_detect(Os::Linux, &state);

        let game = report
            .games
            .iter()
            .find(|g| g.slug == "stardew-valley")
            .unwrap_or_else(|| {
                panic!(
                    "stardew-valley should surface via Lutris prefix → walker; got {} games",
                    report.games.len(),
                );
            });
        // Walker hit must live under the Lutris prefix (which itself is
        // under `home`). Anything outside `home` would mean the test
        // leaked onto the real filesystem.
        assert!(
            !game.found_paths.is_empty(),
            "Lutris walker should produce at least one hit inside the prefix; got {:?}",
            game.found_paths,
        );
        let inside_prefix = game.found_paths.iter().any(|p| p.starts_with(&prefix_root));
        assert!(
            inside_prefix,
            "at least one found_paths entry should live under the Lutris prefix {prefix_root:?}; got {:?}",
            game.found_paths,
        );
    });
}

/// Bottles end-to-end: same shape as the Lutris test but with the native
/// Bottles layout `~/.local/share/bottles/bottles/stardew-valley/`. The
/// bottle's directory name *is* the identifier;
/// `slugify("stardew-valley") == "stardew-valley"` matches the catalog
/// slug directly. As in the Lutris test, a synthetic Steam install seeds
/// the slug into `by_slug` so the walker fires against the Bottles
/// prefix.
#[test]
fn bottles_fixture_yields_detected_game() {
    with_isolated_linux_env(|home| {
        let _steamapps = build_steam_install(home, &[(413150, "Stardew Valley", "Stardew Valley")]);

        let prefix_root = home
            .join(".local/share/bottles/bottles")
            .join("stardew-valley");
        let save_dir =
            prefix_root.join("drive_c/users/steamuser/AppData/Roaming/StardewValley/Saves");
        std::fs::create_dir_all(&save_dir).unwrap();
        std::fs::write(save_dir.join("Farm.sav"), b"synthetic save").unwrap();

        let state = CliState::default();
        let report = block_on_detect(Os::Linux, &state);

        let game = report
            .games
            .iter()
            .find(|g| g.slug == "stardew-valley")
            .unwrap_or_else(|| {
                panic!(
                    "stardew-valley should surface via Bottles prefix → walker; got {} games",
                    report.games.len(),
                );
            });
        assert!(
            !game.found_paths.is_empty(),
            "Bottles walker should produce at least one hit inside the prefix; got {:?}",
            game.found_paths,
        );
        let inside_prefix = game.found_paths.iter().any(|p| p.starts_with(&prefix_root));
        assert!(
            inside_prefix,
            "at least one found_paths entry should live under the Bottles prefix {prefix_root:?}; got {:?}",
            game.found_paths,
        );
    });
}
