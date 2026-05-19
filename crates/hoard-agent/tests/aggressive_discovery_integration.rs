//! End-to-end tests for the 1.5.1 aggressive walker and the Ludusavi
//! fuzzy-match fallback.
//!
//! The walker fires inside [`hoard_agent::detection::detect_all`] **only**
//! for slugs that finish the main pipeline with empty `found_paths` — it's
//! the long-tail safety net for indies/GOG titles/odd layouts the catalog
//! templates can't expand. The fuzzy fallback resolves Steam apps whose
//! display name slugifies to something the catalog doesn't index exactly
//! (typos, "Definitive Edition" suffixes, minor localisation drift).
//!
//! Coverage:
//!
//! | Test                                            | What it pins                             |
//! |-------------------------------------------------|------------------------------------------|
//! | `walker_finds_save_dir_in_install_when_catalog_misses` | walker fires when catalog templates miss, returns Medium |
//! | `aggressive_discover_respects_caps_against_noisy_dirs` | direct API: high-fanout install dir stays bounded |
//! | `fuzzy_match_resolves_steam_app_with_typo`      | "Stardew Vally" cross-refs to `stardew-valley` via fuzzy |
//!
//! All tests share `ENV_LOCK`: cargo runs the binary's tests in parallel by
//! default and `std::env::set_var` is process-wide. The crate's own
//! `test_lock::ENV` is `pub(crate) #[cfg(test)]` and not reachable from an
//! integration-test binary, so we define a local mutex here — same pattern
//! as `detection_integration.rs`.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use hoard_agent::detection::{
    aggressive_discover, detect_all, Confidence, DetectionSource, DiscoveredSavePath,
};
use hoard_agent::manifest::Os;
use hoard_agent::state::CliState;
use hoard_manifest::ludusavi;

/// Process-wide serialisation for env-mutating tests. Mirrors the lock used
/// by `detection_integration.rs`.
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Linux env vars `detect_all` and the Steam scan inspect. Cleared/replaced
/// under [`with_isolated_linux_env`] so the host's real `$HOME` or XDG dirs
/// can't leak into the fixture.
const LINUX_ENV_VARS: &[&str] = &[
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
/// Holds `ENV_LOCK` for the whole call and restores the previous environment
/// on exit (even on panic) so concurrent tests can't race the env.
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

/// Single-thread tokio runtime; `detect_all` is async and each test runs
/// it to completion inside the env-guarded scope.
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
/// Mirrors the helper in `detection_integration.rs`. Returns the
/// `steamapps` path so callers can populate per-app install dirs.
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

/// Pick a synthetic appid that's guaranteed not to collide with any Steam
/// appid the embedded catalog knows about. Useful for tests that need a
/// `SteamApp` to *miss* the appid cross-reference so the fallback path
/// (slug or fuzzy) can fire.
fn unused_steam_appid(start: u64) -> u64 {
    let mut appid = start;
    while ludusavi::catalog()
        .iter()
        .any(|e| e.steam_app_id == Some(appid))
    {
        appid += 1;
    }
    appid
}

// =============================================================
// Tests
// =============================================================

/// End-to-end walker happy path. Stardew Valley's catalog Linux template
/// expands under `<xdgConfig>/StardewValley/Saves` — by *not* creating that
/// directory, the filesystem heuristic produces zero hits, so the Steam
/// cross-reference lands the slug with empty `found_paths`. The aggressive
/// walker then scans the install dir, finds `<install>/saves/main.sav`
/// (modified now), and emits a discovery promoted to `Medium` confidence
/// thanks to the recent `.sav` file. The integration check asserts on the
/// final `DetectionReport`: the slug surfaces with the walker-found path,
/// `Confidence::Medium` (the walker overrides the `Both/High` promotion that
/// `merge_fs_hit` would otherwise stamp — the walker's signal is heuristic
/// only), and the install dir hint preserved from the Steam scan.
#[test]
fn walker_finds_save_dir_in_install_when_catalog_misses() {
    with_isolated_linux_env(|home| {
        // Appid 413150 is Stardew Valley — present in the catalog with a
        // Linux template (`<xdgConfig>/StardewValley/Saves`) that we
        // deliberately do not materialise so the heuristic stays empty.
        let steamapps =
            build_steam_install(home, &[(413150, "Stardew Valley", "Stardew Valley")]);
        let install = steamapps.join("common").join("Stardew Valley");
        let save_dir = install.join("saves");
        std::fs::create_dir_all(&save_dir).unwrap();
        // Recent `.sav` file → walker promotes the dir to `Medium`.
        std::fs::write(save_dir.join("main.sav"), b"binary save data").unwrap();

        let state = CliState::default();
        let report = block_on_detect(Os::Linux, &state);

        let game = report
            .games
            .iter()
            .find(|g| g.slug == "stardew-valley")
            .expect("stardew-valley should appear in the report");
        assert_eq!(
            game.steam_app_id,
            Some(413150),
            "Steam cross-reference must preserve the appid; got {:?}",
            game.steam_app_id,
        );
        assert!(
            game.found_paths.iter().any(|p| p == &save_dir),
            "walker should discover {save_dir:?}; got {:?}",
            game.found_paths,
        );
        // The walker over-rides `merge_fs_hit`'s `Both/High` stamping
        // because its signal is heuristic, not catalog-backed. The
        // assertion accepts `Medium` (recent save file present) or higher
        // — never `Low`, since the `.sav` file forces promotion.
        assert!(
            matches!(game.confidence, Confidence::Medium | Confidence::High),
            "walker hit with recent .sav must be Medium or higher; got {:?}",
            game.confidence,
        );
        assert_eq!(
            game.install_dir.as_deref(),
            Some(install.as_path()),
            "install dir hint should be preserved from the Steam scan",
        );
    });
}

/// Direct exercise of the public `aggressive_discover` API against a
/// high-fanout install dir. The walker spec promises a per-root candidate
/// cap (see `AGGRESSIVE_WALK_MAX_CANDIDATES` in `detection.rs`) and a
/// per-root timeout; this test pushes the depth knob to an absurd value
/// and seeds the tree with many empty siblings to confirm:
///   1. the call returns in well under a wall-clock second even with
///      `max_depth = 64`, proving the timeout / cap is honoured rather
///      than relying on the depth bound alone;
///   2. the candidate count stays `<= 5` (the constant is `pub(crate)`
///      and not reachable from this binary, so the literal 5 is asserted
///      in line with the documented contract);
///   3. the partial result still contains real save-dir candidates,
///      i.e. the bail-out doesn't drop everything on its way out.
#[test]
fn aggressive_discover_respects_caps_against_noisy_dirs() {
    let tmp = tempfile::tempdir().unwrap();
    let install = tmp.path().join("BigInstall");
    std::fs::create_dir_all(&install).unwrap();
    // 50 sibling top-level dirs with one shallow save-like subdir each.
    // Walker should pick at most 5 (`AGGRESSIVE_WALK_MAX_CANDIDATES`)
    // before bailing — way below the depth bound, so the cap is what
    // stops it.
    for i in 0..50 {
        let branch = install.join(format!("branch_{i:02}"));
        std::fs::create_dir_all(branch.join("saves")).unwrap();
    }
    // Extra noise: deeply nested empty dirs the walker has to descend
    // past before realising there's no save-like name.
    for i in 0..20 {
        let mut p = install.join(format!("noise_{i:02}"));
        for d in 0..6 {
            p = p.join(format!("level_{d}"));
        }
        std::fs::create_dir_all(&p).unwrap();
    }

    let start = std::time::Instant::now();
    let hits = aggressive_discover(
        "noisy-slug",
        "Noisy Game",
        Some(&install),
        None,
        // Same default the production pipeline uses (the constant is
        // `pub(crate)`); a 1.5s budget is enough on any disk for the
        // walker to either fill the cap or time out gracefully.
        Duration::from_millis(1500),
        // `max_depth` cranked way past the default of 4 to prove the
        // walker doesn't rely on the depth bound to terminate — the
        // candidate cap and per-root timeout must do the job.
        64,
    );
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_secs(5),
        "aggressive walker hung past the 5s safety budget; elapsed = {elapsed:?}",
    );
    // The plan calls this out as `<= AGGRESSIVE_WALK_MAX_CANDIDATES`
    // (currently 5). Hard-coding the number here means a bump to the
    // constant will fail this test loudly rather than silently widen the
    // contract.
    assert!(
        hits.len() <= 5,
        "walker must respect the per-root candidate cap (<= 5); got {} hits: {hits:?}",
        hits.len(),
    );
    assert!(
        !hits.is_empty(),
        "with 50 save-like branches the walker should produce at least one partial hit before bailing",
    );
    // Sanity-check the discovered paths are *real* matches, not noise
    // leaking through the cap.
    for DiscoveredSavePath { path, .. } in &hits {
        let stem = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        assert!(
            stem.eq_ignore_ascii_case("saves"),
            "walker reported a non-save-like dir {path:?}; expected a 'saves' leaf",
        );
    }
}

/// `apply_steam_name_fallback`'s fuzzy branch end-to-end. The Steam app
/// reports its name as "Stardew Vally" (one-character typo). Slugify drops
/// it to `stardew-vally`, which the catalog doesn't index. The appid is a
/// synthetic value chosen above any Steam appid in the catalog so the
/// appid cross-reference can't pre-empt the fuzzy step. The catalog's
/// `stardew-valley` entry sits at Levenshtein distance 1 over `max(14,13) =
/// 14`, i.e. ≈ 0.071 < 0.15 = `FUZZY_NAME_THRESHOLD`, so the fuzzy match
/// fires and the slug shows up with `source = SteamLibrary` and
/// `confidence = Low`. A complementary assertion confirms the catalog
/// still ships the `stardew-valley` entry the test relies on; if Ludusavi
/// upstream ever renames or drops it we want a loud failure here.
#[test]
fn fuzzy_match_resolves_steam_app_with_typo() {
    // Catalog precondition: `stardew-valley` must exist. The plan asks
    // for an explicit assertion using `find_by_slug`, but the module only
    // exposes `find_by_steam_app_id` and `find_by_fuzzy_name` publicly,
    // so we walk the catalog directly — same pattern as
    // `detection_integration.rs::catalog_entry`.
    assert!(
        ludusavi::catalog()
            .iter()
            .any(|e| e.slug == "stardew-valley"),
        "embedded Ludusavi catalog must contain `stardew-valley` for this test",
    );

    with_isolated_linux_env(|home| {
        // Synthetic appid above any catalog value so the appid
        // cross-reference can't short-circuit the fuzzy fallback. The
        // start point matches the pattern used by the existing
        // `steam_name_fallback_picks_up_no_appid_entries` integration
        // test in `detection_integration.rs`.
        let appid = unused_steam_appid(90_000_001);
        // Confirm the slugified typo really doesn't collide with any
        // catalog slug — otherwise the slug-exact fallback would fire
        // instead of fuzzy and the test would silently change semantics.
        let typo_slug = ludusavi::slugify("Stardew Vally");
        assert!(
            !ludusavi::catalog().iter().any(|e| e.slug == typo_slug),
            "the typo slug {typo_slug:?} unexpectedly resolved exactly in the catalog; fuzzy path would be skipped",
        );

        let install_subdir = "Stardew Vally";
        let steamapps =
            build_steam_install(home, &[(appid, "Stardew Vally", install_subdir)]);

        let state = CliState::default();
        let report = block_on_detect(Os::Linux, &state);

        let game = report
            .games
            .iter()
            .find(|g| g.slug == "stardew-valley")
            .unwrap_or_else(|| {
                panic!(
                    "fuzzy match should surface `stardew-valley`; report had {} games: {:?}",
                    report.games.len(),
                    report
                        .games
                        .iter()
                        .map(|g| &g.slug)
                        .collect::<Vec<_>>(),
                )
            });
        assert_eq!(game.source, DetectionSource::SteamLibrary);
        assert_eq!(game.confidence, Confidence::Low);
        assert_eq!(game.steam_app_id, Some(appid));
        assert_eq!(
            game.install_dir.as_deref(),
            Some(steamapps.join("common").join(install_subdir).as_path()),
        );
        // No save folder materialised under either the catalog template
        // or the install dir → walker can't help and `found_paths` stays
        // empty. UI surfaces the amber "pick folder" prompt; this asserts
        // the fuzzy-match row is the *only* signal driving the slug here.
        assert!(
            game.found_paths.is_empty(),
            "no save folder on disk → found_paths should stay empty; got {:?}",
            game.found_paths,
        );
    });
}
