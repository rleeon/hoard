//! Enumerators for games installed by non-Steam launchers.
//!
//! Three sources, all Windows-native:
//!
//! - **Epic Games**: JSON `.item` manifests under
//!   `%PROGRAMDATA%\Epic\EpicGamesLauncher\Data\Manifests\`.
//! - **GOG Galaxy**: sqlite database at
//!   `%LOCALAPPDATA%\GOG.com\Galaxy\storage\galaxy-2.0.db`. Schema drifts
//!   between Galaxy releases, so the reader probes several known tables
//!   in order and returns whatever the first match yields.
//! - **Microsoft Store / Xbox Game Pass**: registry under
//!   `HKCU\Software\Microsoft\GamingServices\PackageRepository\Package`.
//!
//! The public surface is OS-agnostic: each function takes an `Os` and
//! returns `Vec::new()` on non-Windows. Epic and GOG perform their
//! filesystem / sqlite work in OS-agnostic Rust so the readers can be
//! exercised cross-platform with synthetic fixtures (test runs on Linux
//! still verify the parsers). The Microsoft Store reader requires the
//! Windows registry, so its real implementation lives under
//! `#[cfg(windows)]` while the non-Windows build gets a vec-empty stub.

use std::path::PathBuf;

use serde::Deserialize;

use crate::manifest::Os;

/// Which launcher discovered the game. Mirrors the `LauncherKind`
/// terminology used in the §4.2 plan: Steam stays in `steam.rs`, the
/// rest land here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LauncherKind {
    Epic,
    Gog,
    MicrosoftStore,
}

/// One installed game discovered through a non-Steam launcher.
///
/// `app_id` is the launcher's stable identifier (Epic CatalogItemId, GOG
/// productId stringified, MS package family name) — used as the key when
/// merging into the by-slug map in `detect_all`. `install_dir` is the
/// path the launcher itself recorded; the aggressive walker scans it for
/// save files in step 5 of the pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LauncherApp {
    pub launcher: LauncherKind,
    pub app_id: String,
    pub name: String,
    pub install_dir: PathBuf,
}

// ---- Epic Games --------------------------------------------------------

/// JSON shape of an Epic `.item` manifest, trimmed to the fields we need.
///
/// Epic ships many additional fields per manifest (LaunchExecutable,
/// AppCategories, FormatVersion, etc.) — `serde` ignores unknown fields
/// by default so this minimal struct survives schema additions.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct EpicManifest {
    install_location: Option<String>,
    display_name: Option<String>,
    catalog_item_id: Option<String>,
    /// Some entries (DLCs, prereqs) populate `MainGameAppName` instead of
    /// `CatalogItemId`; we fall back to it when the catalog id is empty.
    main_game_app_name: Option<String>,
}

/// Enumerate Epic Games Launcher installations.
///
/// Reads `%PROGRAMDATA%\Epic\EpicGamesLauncher\Data\Manifests\*.item` and
/// returns one entry per valid manifest. Manifests missing
/// `InstallLocation`, or pointing at a path that is not a directory, are
/// skipped. The reader is cross-platform — tests can prepare a fixture
/// tree on Linux and call `list_installed_epic_games(Os::Windows)`.
pub fn list_installed_epic_games(os: Os) -> Vec<LauncherApp> {
    if !matches!(os, Os::Windows) {
        return Vec::new();
    }
    let Some(programdata) = std::env::var_os("PROGRAMDATA") else {
        return Vec::new();
    };
    let manifests_dir = PathBuf::from(programdata)
        .join("Epic")
        .join("EpicGamesLauncher")
        .join("Data")
        .join("Manifests");
    let entries = match std::fs::read_dir(&manifests_dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut out: Vec<LauncherApp> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let is_item = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("item"));
        if !is_item {
            continue;
        }
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "Epic manifest unreadable; skipping"
                );
                continue;
            }
        };
        let manifest: EpicManifest = match serde_json::from_str(&text) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "Epic manifest parse failed; skipping"
                );
                continue;
            }
        };
        let install_location = match manifest.install_location {
            Some(s) if !s.is_empty() => PathBuf::from(s),
            _ => continue,
        };
        if !install_location.is_dir() {
            continue;
        }
        let app_id = manifest
            .catalog_item_id
            .filter(|s| !s.is_empty())
            .or(manifest.main_game_app_name)
            .unwrap_or_default();
        if app_id.is_empty() {
            continue;
        }
        let name = manifest.display_name.unwrap_or_else(|| app_id.clone());
        out.push(LauncherApp {
            launcher: LauncherKind::Epic,
            app_id,
            name,
            install_dir: install_location,
        });
    }

    out.sort_by(|a, b| a.app_id.cmp(&b.app_id));
    out.dedup_by(|a, b| a.app_id == b.app_id);
    tracing::info!(apps = out.len(), "Epic Games scan complete");
    out
}

// ---- GOG Galaxy --------------------------------------------------------

/// Enumerate games tracked by GOG Galaxy.
///
/// Reads `%LOCALAPPDATA%\GOG.com\Galaxy\storage\galaxy-2.0.db` in
/// read-only mode and probes the tables the various Galaxy releases use
/// to record installed products. Returns `Vec::new()` if the database is
/// absent, unreadable, or none of the probed shapes are present — never
/// panics, never propagates a sqlite error.
///
/// Like the Epic reader this runs on any host: tests can hand-build a
/// sqlite fixture on Linux and pass `Os::Windows` to exercise it.
pub fn list_installed_gog_games(os: Os) -> Vec<LauncherApp> {
    if !matches!(os, Os::Windows) {
        return Vec::new();
    }
    // `rusqlite` is a `cfg(windows)` prod dep and a `cfg(not(windows))`
    // dev-dep — the reader is available in real Windows builds and in
    // tests on any host, but not in a non-Windows release build (the
    // function still honours its public contract because the caller
    // already has to claim `Os::Windows` to reach this branch).
    #[cfg(any(windows, test))]
    {
        let Some(local) = std::env::var_os("LOCALAPPDATA") else {
            return Vec::new();
        };
        let db = PathBuf::from(local)
            .join("GOG.com")
            .join("Galaxy")
            .join("storage")
            .join("galaxy-2.0.db");
        if !db.is_file() {
            return Vec::new();
        }
        gog_read_db(&db)
    }
    #[cfg(not(any(windows, test)))]
    {
        Vec::new()
    }
}

/// Open the Galaxy database read-only and try the known table shapes in
/// order. The first successful query wins. The function is OS-agnostic
/// because `rusqlite` with the `bundled` feature ships its own SQLite, so
/// the same code path runs on Linux test hosts and Windows targets.
#[cfg(any(windows, test))]
fn gog_read_db(db: &std::path::Path) -> Vec<LauncherApp> {
    use rusqlite::{Connection, OpenFlags};

    let conn = match Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_ONLY) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(
                path = %db.display(),
                error = %e,
                "GOG Galaxy db open failed; skipping"
            );
            return Vec::new();
        }
    };

    // Candidate (productId, productName, installationPath) shapes. Galaxy
    // has shipped at least these tables over the years; we probe in this
    // order and stop at the first that yields rows.
    const QUERIES: &[&str] = &[
        "SELECT productId, productName, installationPath \
         FROM Builds WHERE installationPath IS NOT NULL AND installationPath <> ''",
        "SELECT productId, productName, installationPath \
         FROM InstalledExternalProducts \
         WHERE installationPath IS NOT NULL AND installationPath <> ''",
        "SELECT productId, productName, installationPath \
         FROM LimitedDetails WHERE installationPath IS NOT NULL AND installationPath <> ''",
    ];

    for sql in QUERIES {
        let mut stmt = match conn.prepare(sql) {
            Ok(s) => s,
            Err(e) => {
                tracing::debug!(error = %e, sql, "GOG Galaxy probe failed; trying next");
                continue;
            }
        };
        let rows = stmt.query_map([], |row| {
            let product_id: i64 = row.get(0)?;
            let name: String = row.get(1)?;
            let install_path: String = row.get(2)?;
            Ok((product_id, name, install_path))
        });
        let rows = match rows {
            Ok(r) => r,
            Err(e) => {
                tracing::debug!(error = %e, sql, "GOG Galaxy query_map failed; trying next");
                continue;
            }
        };

        let mut out: Vec<LauncherApp> = Vec::new();
        for row in rows.flatten() {
            let (product_id, name, install_path) = row;
            let install_dir = PathBuf::from(&install_path);
            if !install_dir.is_dir() {
                continue;
            }
            out.push(LauncherApp {
                launcher: LauncherKind::Gog,
                app_id: product_id.to_string(),
                name,
                install_dir,
            });
        }
        if !out.is_empty() {
            out.sort_by(|a, b| a.app_id.cmp(&b.app_id));
            out.dedup_by(|a, b| a.app_id == b.app_id);
            tracing::info!(apps = out.len(), sql, "GOG Galaxy scan complete");
            return out;
        }
    }

    tracing::info!(
        path = %db.display(),
        "GOG Galaxy db opened but no known table yielded rows"
    );
    Vec::new()
}

// ---- Microsoft Store / Xbox Game Pass ---------------------------------

/// Enumerate games installed via Microsoft Store / Xbox Game Pass.
///
/// Each installed game has a subkey under
/// `HKCU\Software\Microsoft\GamingServices\PackageRepository\Package`
/// carrying a `Root` value with the install directory. The subkey name
/// is the package family name and doubles as the app id; without
/// AppxManifest metadata the display name falls back to that same value.
///
/// Unlike Epic and GOG this reader cannot be exercised cross-platform —
/// `winreg::RegKey` does not compile on non-Windows targets — so the
/// real implementation is `#[cfg(windows)]` and the non-Windows build
/// gets a vec-empty stub.
pub fn list_installed_msstore_games(os: Os) -> Vec<LauncherApp> {
    if !matches!(os, Os::Windows) {
        return Vec::new();
    }
    msstore_impl()
}

#[cfg(windows)]
fn msstore_impl() -> Vec<LauncherApp> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let root_key =
        match hkcu.open_subkey("Software\\Microsoft\\GamingServices\\PackageRepository\\Package") {
            Ok(k) => k,
            Err(e) => {
                tracing::debug!(
                    error = %e,
                    "MS Store registry root absent; nothing to enumerate"
                );
                return Vec::new();
            }
        };

    let mut out: Vec<LauncherApp> = Vec::new();
    for name in root_key.enum_keys().flatten() {
        let subkey = match root_key.open_subkey(&name) {
            Ok(k) => k,
            Err(e) => {
                tracing::debug!(
                    package = %name,
                    error = %e,
                    "MS Store subkey unreadable; skipping"
                );
                continue;
            }
        };
        let root_value: String = match subkey.get_value("Root") {
            Ok(v) => v,
            Err(_) => continue,
        };
        let install_dir = PathBuf::from(root_value);
        if !install_dir.is_dir() {
            continue;
        }
        out.push(LauncherApp {
            launcher: LauncherKind::MicrosoftStore,
            app_id: name.clone(),
            name,
            install_dir,
        });
    }

    out.sort_by(|a, b| a.app_id.cmp(&b.app_id));
    out.dedup_by(|a, b| a.app_id == b.app_id);
    tracing::info!(apps = out.len(), "Microsoft Store scan complete");
    out
}

#[cfg(not(windows))]
fn msstore_impl() -> Vec<LauncherApp> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// Run `f` with the named env var set to `value`, holding the
    /// crate's process-wide env lock so parallel tests don't race.
    /// Restores the previous value (or removes it if unset before).
    fn with_env<F: FnOnce()>(var: &str, value: &Path, f: F) {
        let _guard = crate::test_lock::ENV
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let prev = std::env::var_os(var);
        std::env::set_var(var, value);
        f();
        match prev {
            Some(v) => std::env::set_var(var, v),
            None => std::env::remove_var(var),
        }
    }

    // ---- Epic ----------------------------------------------------------

    #[test]
    fn epic_returns_empty_on_non_windows() {
        let tmp = tempfile::tempdir().unwrap();
        with_env("PROGRAMDATA", tmp.path(), || {
            assert!(list_installed_epic_games(Os::Linux).is_empty());
            assert!(list_installed_epic_games(Os::Mac).is_empty());
        });
    }

    #[test]
    fn epic_returns_empty_when_dir_missing() {
        let tmp = tempfile::tempdir().unwrap();
        // PROGRAMDATA exists but the Epic subtree does not.
        with_env("PROGRAMDATA", tmp.path(), || {
            assert!(list_installed_epic_games(Os::Windows).is_empty());
        });
    }

    #[test]
    fn epic_parses_item_manifests() {
        let tmp = tempfile::tempdir().unwrap();
        let programdata = tmp.path();
        let manifests = programdata
            .join("Epic")
            .join("EpicGamesLauncher")
            .join("Data")
            .join("Manifests");
        std::fs::create_dir_all(&manifests).unwrap();

        let install_dir = tmp.path().join("Games").join("Some Game");
        std::fs::create_dir_all(&install_dir).unwrap();

        let manifest_json = serde_json::json!({
            "FormatVersion": 0,
            "InstallLocation": install_dir.to_string_lossy(),
            "DisplayName": "Some Game",
            "CatalogItemId": "abc123",
            "MainGameAppName": "SomeGame",
            "LaunchExecutable": "SomeGame.exe",
        });
        std::fs::write(
            manifests.join("SomeGame.item"),
            serde_json::to_string_pretty(&manifest_json).unwrap(),
        )
        .unwrap();

        // A second manifest whose InstallLocation does not exist must be
        // dropped (Epic occasionally leaves dangling manifests after
        // uninstall).
        let dangling = serde_json::json!({
            "InstallLocation": tmp.path().join("nonexistent").to_string_lossy(),
            "DisplayName": "Ghost",
            "CatalogItemId": "ghost",
        });
        std::fs::write(
            manifests.join("Ghost.item"),
            serde_json::to_string(&dangling).unwrap(),
        )
        .unwrap();

        // A non-`.item` file must be ignored.
        std::fs::write(manifests.join("README.txt"), "ignore me").unwrap();

        with_env("PROGRAMDATA", programdata, || {
            let apps = list_installed_epic_games(Os::Windows);
            assert_eq!(apps.len(), 1, "got {apps:?}");
            assert_eq!(apps[0].launcher, LauncherKind::Epic);
            assert_eq!(apps[0].app_id, "abc123");
            assert_eq!(apps[0].name, "Some Game");
            assert_eq!(apps[0].install_dir, install_dir);
        });
    }

    // ---- GOG -----------------------------------------------------------

    #[test]
    fn gog_returns_empty_on_non_windows() {
        let tmp = tempfile::tempdir().unwrap();
        with_env("LOCALAPPDATA", tmp.path(), || {
            assert!(list_installed_gog_games(Os::Linux).is_empty());
        });
    }

    #[test]
    fn gog_returns_empty_when_db_missing() {
        let tmp = tempfile::tempdir().unwrap();
        with_env("LOCALAPPDATA", tmp.path(), || {
            assert!(list_installed_gog_games(Os::Windows).is_empty());
        });
    }

    #[test]
    fn gog_parses_synthetic_sqlite() {
        let tmp = tempfile::tempdir().unwrap();
        let localappdata = tmp.path();
        let db_dir = localappdata.join("GOG.com").join("Galaxy").join("storage");
        std::fs::create_dir_all(&db_dir).unwrap();
        let db_path = db_dir.join("galaxy-2.0.db");

        let install_dir = tmp.path().join("Games").join("Hyperdrive");
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
                    1207659999_i64,
                    "Hyperdrive",
                    install_dir.to_string_lossy(),
                ],
            )
            .unwrap();
            // A second row pointing at a path that does not exist must be
            // dropped — GOG sometimes keeps stale install rows after a
            // delete.
            conn.execute(
                "INSERT INTO Builds (productId, productName, installationPath) VALUES (?1, ?2, ?3)",
                rusqlite::params![
                    9999_i64,
                    "Ghost",
                    tmp.path().join("nope").to_string_lossy(),
                ],
            )
            .unwrap();
        }

        with_env("LOCALAPPDATA", localappdata, || {
            let apps = list_installed_gog_games(Os::Windows);
            assert_eq!(apps.len(), 1, "got {apps:?}");
            assert_eq!(apps[0].launcher, LauncherKind::Gog);
            assert_eq!(apps[0].app_id, "1207659999");
            assert_eq!(apps[0].name, "Hyperdrive");
            assert_eq!(apps[0].install_dir, install_dir);
        });
    }

    #[test]
    fn gog_returns_empty_when_no_known_table_matches() {
        let tmp = tempfile::tempdir().unwrap();
        let localappdata = tmp.path();
        let db_dir = localappdata.join("GOG.com").join("Galaxy").join("storage");
        std::fs::create_dir_all(&db_dir).unwrap();
        let db_path = db_dir.join("galaxy-2.0.db");

        // Database opens fine but contains no table the reader knows
        // about — must collapse to vec empty without panic.
        {
            let conn = rusqlite::Connection::open(&db_path).unwrap();
            conn.execute("CREATE TABLE Unrelated (x INTEGER)", []).unwrap();
        }

        with_env("LOCALAPPDATA", localappdata, || {
            assert!(list_installed_gog_games(Os::Windows).is_empty());
        });
    }

    // ---- Microsoft Store ----------------------------------------------

    #[test]
    fn msstore_returns_empty_on_non_windows() {
        assert!(list_installed_msstore_games(Os::Linux).is_empty());
        assert!(list_installed_msstore_games(Os::Mac).is_empty());
    }

    /// On a real Windows host the user's `HKCU` may carry MS Store
    /// entries we cannot synthesise without writing to the real registry
    /// (which would pollute the user's machine). Ignored by default —
    /// run with `cargo test -- --ignored` on a Windows box that has at
    /// least one MS Store game installed to smoke-test the reader.
    #[cfg(windows)]
    #[test]
    #[ignore = "requires real HKCU MS Store packages; pollutes user registry to fixture"]
    fn msstore_returns_packages_on_windows_test_env() {
        let apps = list_installed_msstore_games(Os::Windows);
        // Cannot assert non-empty (CI machine may have zero packages),
        // but at least exercise the code path without panic.
        for app in &apps {
            assert_eq!(app.launcher, LauncherKind::MicrosoftStore);
            assert!(app.install_dir.is_dir());
        }
    }
}
