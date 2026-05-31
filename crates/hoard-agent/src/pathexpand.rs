//! Expand Ludusavi-style path templates into concrete filesystem paths.
//!
//! Ludusavi save-path entries use placeholders like `<winAppData>`,
//! `<xdgData>` and `<home>`. This module knows how to turn each of those
//! into one or more real directories on the host.
//!
//! Some placeholders fan out to multiple candidates (Steam libraries on
//! several disks, both XDG and `~/.config`, etc.) — hence the `Vec<PathBuf>`
//! return type. Callers use the *existing* paths from the result to decide
//! whether the game has been played on this machine.
//!
//! Detection is intentionally non-destructive: nothing here touches disks
//! beyond reading environment variables.

use std::path::{Path, PathBuf};

use crate::manifest::Os;

/// Expand a single template into zero or more concrete paths. Unknown
/// placeholders cause the template to be dropped (returning `vec![]`) so the
/// caller never tries to stat literal `"<winAppData>"`.
pub fn expand_path(template: &str, os: Os) -> Vec<PathBuf> {
    // Steam libraries are the only fan-out we currently support; everything
    // else maps to a single base directory. The trailing tail of the
    // template gets joined onto each base.
    let (placeholder, tail) = match split_placeholder(template) {
        Some(parts) => parts,
        None => {
            // No placeholder at the start: template is a literal path
            // (Ludusavi only emits absolute literals on rare entries). Return
            // it verbatim — stripping the leading '/' would turn an absolute
            // path into a bogus relative one that never stats.
            return vec![PathBuf::from(template)];
        }
    };

    let bases = expand_placeholder(&placeholder, os);
    if bases.is_empty() {
        return Vec::new();
    }

    let tail_clean = tail.trim_start_matches(['/', '\\']);
    bases
        .into_iter()
        .map(|b| {
            if tail_clean.is_empty() {
                b
            } else {
                b.join(tail_clean)
            }
        })
        .collect()
}

/// Expand a Ludusavi Windows template against a Proton/Wine prefix root.
///
/// `prefix` points at the `pfx/` directory of one Steam compatdata entry
/// (e.g. `~/.steam/steam/steamapps/compatdata/413150/pfx`). Windows
/// placeholders (`<winAppData>`, `<home>`, `<root>`, …) are mapped onto
/// the prefix's `drive_c/` layout, mirroring how Wine exposes a Windows
/// user environment.
///
/// Returns an empty `Vec` if the template doesn't start with a placeholder
/// we know how to map to a Wine path — Linux/Mac-only placeholders
/// (`<xdgData>`, `<macAppSupport>`, …) and unknown tokens both yield
/// `vec![]` so the caller doesn't accidentally stat a meaningless path
/// under the prefix.
pub fn expand_path_in_prefix(template: &str, prefix: &Path) -> Vec<PathBuf> {
    let Some((placeholder, tail)) = split_placeholder(template) else {
        // Literal templates don't apply to a prefix — they're absolute
        // host paths, not Wine paths. Drop them.
        return Vec::new();
    };
    let Some(base) = expand_placeholder_in_prefix(&placeholder, prefix) else {
        return Vec::new();
    };
    let tail_clean = tail.trim_start_matches(['/', '\\']);
    if tail_clean.is_empty() {
        vec![base]
    } else {
        vec![base.join(tail_clean)]
    }
}

/// Map one Ludusavi placeholder onto the corresponding directory inside a
/// Wine prefix. Returns `None` for placeholders that don't apply (Linux/Mac
/// tokens, per-install identifiers, unknown names).
fn expand_placeholder_in_prefix(name: &str, prefix: &Path) -> Option<PathBuf> {
    let drive_c = prefix.join("drive_c");
    let steamuser = drive_c.join("users/steamuser");
    let mapped = match name {
        "winAppData" => steamuser.join("AppData/Roaming"),
        "winLocalAppData" => steamuser.join("AppData/Local"),
        "winLocalAppDataLow" => steamuser.join("AppData/LocalLow"),
        "winDocuments" => steamuser.join("Documents"),
        "winPublic" => drive_c.join("users/Public"),
        "winProgramData" => drive_c.join("ProgramData"),
        "winDir" => drive_c.join("windows"),
        "home" => steamuser,
        "root" => drive_c,
        _ => return None,
    };
    Some(mapped)
}

/// Split `<name>tail` into `("name", "tail")`. Returns `None` if the template
/// doesn't start with a `<…>` placeholder.
fn split_placeholder(template: &str) -> Option<(String, String)> {
    let rest = template.strip_prefix('<')?;
    let end = rest.find('>')?;
    let name = rest[..end].to_string();
    let tail = rest[end + 1..].to_string();
    Some((name, tail))
}

fn expand_placeholder(name: &str, os: Os) -> Vec<PathBuf> {
    // On Windows, prefer the OneDrive-aware Known Folder lookup before
    // falling back to env vars. Modern installs frequently have Documents
    // (and sometimes Pictures/Desktop) redirected to a OneDrive subtree;
    // `%USERPROFILE%\Documents` then points at an empty stub the game
    // never writes to. `windows_known_folder` reads
    // `HKCU\...\User Shell Folders` so we follow the redirect. Returns
    // `None` on non-Windows or when the registry value is missing — the
    // existing env-based match below kicks in as the fallback.
    if matches!(os, Os::Windows) {
        if let Some(p) = windows_known_folder(name) {
            return vec![p];
        }
    }

    match (os, name) {
        // -------- Cross-platform basics
        (_, "home") => home_dir().into_iter().collect(),
        (_, "root") => vec![PathBuf::from(if cfg!(windows) { "C:\\" } else { "/" })],

        // -------- Windows
        (Os::Windows, "winAppData") => env_dir("APPDATA"),
        (Os::Windows, "winLocalAppData") => env_dir("LOCALAPPDATA"),
        // `<winLocalAppDataLow>` — `%USERPROFILE%\AppData\LocalLow`. Not
        // present in `User Shell Folders`, so we synthesise it from
        // `%USERPROFILE%` directly. Ludusavi uses it for a handful of
        // games that write to the IE-sandboxed LocalLow tree.
        (Os::Windows, "winLocalAppDataLow") => home_dir()
            .map(|h| vec![h.join("AppData").join("LocalLow")])
            .unwrap_or_default(),
        (Os::Windows, "winDocuments") => {
            // Fallback only: `windows_known_folder` already ran above and
            // resolves the OneDrive-redirected Documents path when present.
            // We only reach here when the registry lookup failed, so use the
            // plain %USERPROFILE%\Documents stub.
            home_dir()
                .map(|h| vec![h.join("Documents")])
                .unwrap_or_default()
        }
        (Os::Windows, "winPublic") => env_dir("PUBLIC"),
        (Os::Windows, "winProgramData") => env_dir("PROGRAMDATA"),
        (Os::Windows, "winDir") => env_dir("WINDIR"),
        // `<winSavedGames>` — `%USERPROFILE%\Saved Games`, the Vista+
        // canonical save folder. Modern titles increasingly target it;
        // on non-Windows it returns an empty `Vec` so callers skip it.
        (Os::Windows, "winSavedGames") => home_dir()
            .map(|h| vec![h.join("Saved Games")])
            .unwrap_or_default(),

        // -------- Linux / XDG
        (Os::Linux, "xdgData") => {
            xdg_or(home_dir().map(|h| h.join(".local/share")), "XDG_DATA_HOME")
        }
        (Os::Linux, "xdgConfig") => {
            xdg_or(home_dir().map(|h| h.join(".config")), "XDG_CONFIG_HOME")
        }
        (Os::Linux, "xdgState") => {
            xdg_or(home_dir().map(|h| h.join(".local/state")), "XDG_STATE_HOME")
        }
        (Os::Linux, "xdgCache") => xdg_or(home_dir().map(|h| h.join(".cache")), "XDG_CACHE_HOME"),

        // -------- macOS
        (Os::Mac, "xdgData") | (Os::Mac, "macAppSupport") => home_dir()
            .map(|h| vec![h.join("Library/Application Support")])
            .unwrap_or_default(),
        (Os::Mac, "macPreferences") => home_dir()
            .map(|h| vec![h.join("Library/Preferences")])
            .unwrap_or_default(),

        // -------- Steam
        (_, "storeUserId") | (_, "gameId") => {
            // These are per-install Steam identifiers we can't resolve from a
            // template alone (we'd need the live `userdata/<id>` and the app's
            // numeric id). Dropping the template here is correct: the Steam
            // Cloud stage in `detection.rs::detect_all` (ADR 0019) walks
            // `userdata/<storeUserId>/<appid>/remote/` directly for every
            // installed app and merges any hit, which covers this case without
            // threading live Steam state through this pure expander.
            Vec::new()
        }

        (_, other) => {
            // An unknown placeholder is almost always a real bug — either
            // the manifest grew a new token that we haven't taught
            // pathexpand about, or the user is on an OS we don't handle.
            // Log it (sampled — `trace`, not `warn`) so we can spot gaps
            // without spamming the terminal during a full scan.
            tracing::trace!(
                token = other,
                ?os,
                "unknown path placeholder; dropping template"
            );
            Vec::new()
        }
    }
}

fn env_dir(name: &str) -> Vec<PathBuf> {
    std::env::var_os(name)
        .map(|v| vec![PathBuf::from(v)])
        .unwrap_or_default()
}

fn home_dir() -> Option<PathBuf> {
    // We call this often enough that going through `directories` would feel
    // heavy; std + a Windows fallback covers the cases we hit.
    if let Some(h) = std::env::var_os("HOME") {
        return Some(PathBuf::from(h));
    }
    if cfg!(windows) {
        if let Some(h) = std::env::var_os("USERPROFILE") {
            return Some(PathBuf::from(h));
        }
    }
    None
}

/// Helper: read `env_var`, falling back to the canonical XDG default.
fn xdg_or(default: Option<PathBuf>, env_var: &str) -> Vec<PathBuf> {
    if let Some(v) = std::env::var_os(env_var) {
        let p = PathBuf::from(v);
        if !p.as_os_str().is_empty() {
            return vec![p];
        }
    }
    default.into_iter().collect()
}

/// Resolve a Ludusavi Windows token via the per-user `User Shell Folders`
/// registry mapping, returning the *real* path even when OneDrive has
/// redirected Documents/AppData/etc. away from `%USERPROFILE%`.
///
/// Returns `None` on non-Windows hosts, for tokens we don't map, or when
/// the registry value is missing/unreadable — callers then fall back to
/// the env-var-based match in `expand_placeholder`.
///
/// We only ship a Windows implementation; the no-op stub keeps the call
/// site (`expand_placeholder`) free of `cfg` gating.
#[cfg(windows)]
fn windows_known_folder(token: &str) -> Option<PathBuf> {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    use winreg::RegKey;

    // Map the Ludusavi token to (hive, subkey, value name). The per-user
    // tokens live under HKCU User Shell Folders; ProgramData is machine-
    // wide so we look it up in HKLM Shell Folders instead.
    let (hive, subkey, value_name): (winreg::HKEY, &str, &str) = match token {
        "winDocuments" => (
            HKEY_CURRENT_USER,
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\User Shell Folders",
            "Personal",
        ),
        "winAppData" => (
            HKEY_CURRENT_USER,
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\User Shell Folders",
            "AppData",
        ),
        "winLocalAppData" => (
            HKEY_CURRENT_USER,
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\User Shell Folders",
            "Local AppData",
        ),
        "winPublic" => (
            HKEY_LOCAL_MACHINE,
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\Shell Folders",
            "Common Documents",
        ),
        "winProgramData" => (
            HKEY_LOCAL_MACHINE,
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\Shell Folders",
            "Common AppData",
        ),
        // No registry mapping for LocalLow or SavedGames — both are
        // synthesised from %USERPROFILE% in `expand_placeholder`.
        _ => return None,
    };

    let key = RegKey::predef(hive).open_subkey(subkey).ok()?;
    let raw: String = key.get_value(value_name).ok()?;
    if raw.is_empty() {
        return None;
    }
    Some(PathBuf::from(expand_windows_env_vars(&raw)))
}

#[cfg(not(windows))]
fn windows_known_folder(_token: &str) -> Option<PathBuf> {
    None
}

/// Expand `%FOO%`-style references inside a Windows registry string.
///
/// The `User Shell Folders` keys store paths like
/// `%USERPROFILE%\Documents`; the equivalent `Shell Folders` snapshot is
/// already expanded. We accept either by walking the string and replacing
/// any `%NAME%` segment with the corresponding env var (case-insensitive
/// lookup, since Windows env names are case-insensitive). Unknown
/// variables are left in place so callers can still spot the failure.
#[cfg(windows)]
fn expand_windows_env_vars(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        if let Some(end) = after.find('%') {
            let name = &after[..end];
            // Case-insensitive env var lookup — Windows treats `%appdata%`
            // and `%APPDATA%` identically.
            let value = std::env::vars_os()
                .find(|(k, _)| k.to_string_lossy().eq_ignore_ascii_case(name))
                .map(|(_, v)| v.to_string_lossy().into_owned());
            match value {
                Some(v) => out.push_str(&v),
                None => {
                    // Unknown variable: keep the literal so misconfig is
                    // visible upstream.
                    out.push('%');
                    out.push_str(name);
                    out.push('%');
                }
            }
            rest = &after[end + 1..];
        } else {
            // Trailing `%` with no closer: append the remainder verbatim.
            out.push('%');
            out.push_str(after);
            rest = "";
            break;
        }
    }
    out.push_str(rest);
    out
}

/// Resolve a Ludusavi `RegistryPath` to one or more filesystem paths by
/// reading the named value (or the subkey's default value) on Windows.
///
/// `reg.key` carries the full path with the hive prefix, using either
/// `/` or `\` as separator (Ludusavi emits `/`). Both `HKEY_CURRENT_USER`
/// and `HKEY_LOCAL_MACHINE` are supported; other hives are rejected.
/// `reg.value` is the named value to read; `None` means the subkey's
/// default value.
///
/// The value is treated as a string. Absolute paths are returned as-is.
/// Strings containing `<…>` Ludusavi placeholders are recursively expanded
/// via `expand_path` (Windows OS). Missing keys/values, non-string values
/// and parse errors all collapse to `Vec::new()` so a bad registry entry
/// silently drops out of detection rather than aborting the scan.
///
/// Non-Windows builds always return an empty `Vec` so callers don't need
/// to `#[cfg]` around the call site.
#[cfg(windows)]
pub fn expand_registry_path(reg: &hoard_manifest::ludusavi::RegistryPath) -> Vec<PathBuf> {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    use winreg::RegKey;

    // Normalise the separator: winreg accepts both, but `\` matches what
    // every other Windows-facing API uses.
    let key_path = reg.key.replace('/', "\\");
    let mut parts = key_path.splitn(2, '\\');
    let hive_str = parts.next().unwrap_or("");
    let subkey = parts.next().unwrap_or("");
    let hive = match hive_str {
        "HKEY_CURRENT_USER" | "HKCU" => HKEY_CURRENT_USER,
        "HKEY_LOCAL_MACHINE" | "HKLM" => HKEY_LOCAL_MACHINE,
        _ => return Vec::new(),
    };

    let key = match RegKey::predef(hive).open_subkey(subkey) {
        Ok(k) => k,
        Err(_) => return Vec::new(),
    };
    let value_name: &str = reg.value.as_deref().unwrap_or("");
    let raw: String = match key.get_value(value_name) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let expanded = expand_windows_env_vars(&raw);

    // If the value still contains Ludusavi `<token>` placeholders, recurse
    // through the regular expander so we honour OneDrive redirection and
    // env var fallbacks; otherwise pass the literal back as a single path.
    if expanded.contains('<') {
        expand_path(&expanded, Os::Windows)
    } else if expanded.is_empty() {
        Vec::new()
    } else {
        vec![PathBuf::from(expanded)]
    }
}

#[cfg(not(windows))]
pub fn expand_registry_path(_reg: &hoard_manifest::ludusavi::RegistryPath) -> Vec<PathBuf> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests in this module mutate the process environment. Use the
    // crate-wide lock so tests in *other* modules that also poke `HOME`
    // can't interleave with us.
    fn with_env<F: FnOnce()>(pairs: &[(&str, Option<&str>)], f: F) {
        let _guard = crate::test_lock::ENV
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let prev: Vec<_> = pairs
            .iter()
            .map(|(name, _)| (*name, std::env::var_os(name)))
            .collect();
        for (name, value) in pairs {
            match value {
                Some(v) => std::env::set_var(name, v),
                None => std::env::remove_var(name),
            }
        }
        f();
        for (name, prev_value) in prev {
            match prev_value {
                Some(v) => std::env::set_var(name, v),
                None => std::env::remove_var(name),
            }
        }
    }

    #[test]
    fn expands_home() {
        with_env(&[("HOME", Some("/home/test"))], || {
            let out = expand_path("<home>/.savegames/foo", Os::Linux);
            assert_eq!(out, vec![PathBuf::from("/home/test/.savegames/foo")]);
        });
    }

    #[test]
    fn expands_xdg_with_default() {
        with_env(
            &[("HOME", Some("/home/test")), ("XDG_DATA_HOME", None)],
            || {
                let out = expand_path("<xdgData>/Stardew/Saves", Os::Linux);
                assert_eq!(
                    out,
                    vec![PathBuf::from("/home/test/.local/share/Stardew/Saves")]
                );
            },
        );
    }

    #[test]
    fn expands_xdg_with_override() {
        with_env(
            &[
                ("HOME", Some("/home/test")),
                ("XDG_DATA_HOME", Some("/tmp/xdg")),
            ],
            || {
                let out = expand_path("<xdgData>/Stardew", Os::Linux);
                assert_eq!(out, vec![PathBuf::from("/tmp/xdg/Stardew")]);
            },
        );
    }

    #[test]
    fn windows_appdata() {
        with_env(
            &[("APPDATA", Some("C:\\Users\\T\\AppData\\Roaming"))],
            || {
                let out = expand_path("<winAppData>/Stardew", Os::Windows);
                assert_eq!(out.len(), 1);
                assert!(out[0].ends_with("Stardew"));
            },
        );
    }

    #[test]
    fn unknown_placeholder_drops_template() {
        let out = expand_path("<frobnicate>/foo", Os::Linux);
        assert!(out.is_empty());
    }

    #[test]
    fn literal_path_passes_through() {
        let out = expand_path("/etc/games/foo", Os::Linux);
        assert_eq!(out, vec![PathBuf::from("/etc/games/foo")]);
    }

    #[test]
    fn placeholder_with_no_tail() {
        with_env(&[("HOME", Some("/home/test"))], || {
            let out = expand_path("<home>", Os::Linux);
            assert_eq!(out, vec![PathBuf::from("/home/test")]);
        });
    }

    #[test]
    fn expands_winappdata_against_prefix() {
        let prefix = PathBuf::from("/tmp/fake-prefix");
        let out = expand_path_in_prefix("<winAppData>/Game", &prefix);
        assert_eq!(
            out,
            vec![PathBuf::from(
                "/tmp/fake-prefix/drive_c/users/steamuser/AppData/Roaming/Game"
            )]
        );
    }

    #[test]
    fn expands_all_known_windows_tokens_against_prefix() {
        let prefix = PathBuf::from("/p");
        let cases: &[(&str, &str)] = &[
            ("<winAppData>/X", "/p/drive_c/users/steamuser/AppData/Roaming/X"),
            ("<winLocalAppData>/X", "/p/drive_c/users/steamuser/AppData/Local/X"),
            (
                "<winLocalAppDataLow>/X",
                "/p/drive_c/users/steamuser/AppData/LocalLow/X",
            ),
            ("<winDocuments>/X", "/p/drive_c/users/steamuser/Documents/X"),
            ("<winPublic>/X", "/p/drive_c/users/Public/X"),
            ("<winProgramData>/X", "/p/drive_c/ProgramData/X"),
            ("<winDir>/X", "/p/drive_c/windows/X"),
            ("<home>/X", "/p/drive_c/users/steamuser/X"),
            ("<root>/X", "/p/drive_c/X"),
        ];
        for (template, expected) in cases {
            let out = expand_path_in_prefix(template, &prefix);
            assert_eq!(
                out,
                vec![PathBuf::from(expected)],
                "template {template} mismatched"
            );
        }
    }

    #[test]
    fn prefix_expand_drops_linux_and_unknown_tokens() {
        let prefix = PathBuf::from("/p");
        assert!(expand_path_in_prefix("<xdgData>/X", &prefix).is_empty());
        assert!(expand_path_in_prefix("<xdgConfig>/X", &prefix).is_empty());
        assert!(expand_path_in_prefix("<macAppSupport>/X", &prefix).is_empty());
        assert!(expand_path_in_prefix("<frobnicate>/X", &prefix).is_empty());
    }

    #[test]
    fn prefix_expand_drops_literal_templates() {
        let prefix = PathBuf::from("/p");
        assert!(expand_path_in_prefix("/etc/games/foo", &prefix).is_empty());
    }

    #[test]
    fn prefix_expand_placeholder_no_tail() {
        let prefix = PathBuf::from("/p");
        let out = expand_path_in_prefix("<winAppData>", &prefix);
        assert_eq!(
            out,
            vec![PathBuf::from("/p/drive_c/users/steamuser/AppData/Roaming")]
        );
    }

    /// `<winSavedGames>` is keyed on `(Os::Windows, …)` in the match, so
    /// asking for it under `Os::Linux` drops the template. That's the
    /// real-world case on a Linux host: detection passes `Os::Linux` and
    /// the token is irrelevant.
    #[test]
    fn winsavedgames_dropped_under_os_linux() {
        with_env(&[("HOME", Some("/home/test"))], || {
            let out = expand_path("<winSavedGames>/MyGame", Os::Linux);
            assert!(out.is_empty(), "got {out:?}");
            let out = expand_path("<winSavedGames>/MyGame", Os::Mac);
            assert!(out.is_empty(), "got {out:?}");
        });
    }

    /// When the caller asks for `<winSavedGames>` under `Os::Windows`
    /// the token resolves to `<home>/Saved Games`, whatever `home_dir`
    /// returns. Validates the synthesis works without poking the
    /// registry (the OneDrive-aware path covers only Documents/AppData).
    #[test]
    fn winsavedgames_synthesises_from_home_under_os_windows() {
        with_env(
            &[
                ("HOME", Some("/home/test")),
                // Force the Known-Folder helper to return None on
                // Windows hosts running the test suite — we don't want
                // a real HKCU lookup to interfere.
                ("USERPROFILE", Some("/home/test")),
            ],
            || {
                let out = expand_path("<winSavedGames>/MyGame", Os::Windows);
                assert_eq!(out.len(), 1, "got {out:?}");
                assert!(
                    out[0].ends_with("Saved Games/MyGame")
                        || out[0].ends_with("Saved Games\\MyGame"),
                    "got {out:?}"
                );
            },
        );
    }

    /// `expand_registry_path` is a Windows-only feature; on every other
    /// host it must collapse to an empty vec so callers don't need to
    /// `#[cfg]` around the call.
    #[cfg(not(windows))]
    #[test]
    fn expand_registry_returns_empty_on_unix() {
        let reg = hoard_manifest::ludusavi::RegistryPath {
            key: "HKEY_CURRENT_USER/Software/Acme/Game".to_string(),
            value: None,
        };
        assert!(expand_registry_path(&reg).is_empty());
        let reg = hoard_manifest::ludusavi::RegistryPath {
            key: "HKEY_LOCAL_MACHINE/Software/Acme/Game".to_string(),
            value: Some("SavePath".to_string()),
        };
        assert!(expand_registry_path(&reg).is_empty());
    }

    /// Reads a value the test itself writes to HKCU. Marked `#[ignore]`
    /// because it touches the live user hive; run by hand on a Windows
    /// box when validating the registry expander.
    #[cfg(windows)]
    #[test]
    #[ignore]
    fn expand_registry_reads_value_from_hkcu() {
        use winreg::enums::{HKEY_CURRENT_USER, KEY_ALL_ACCESS};
        use winreg::RegKey;

        let subkey = r"Software\Hoard\PathexpandTest";
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (key, _) = hkcu
            .create_subkey_with_flags(subkey, KEY_ALL_ACCESS)
            .expect("create subkey");
        key.set_value("SavePath", &"C:\\Users\\Tester\\Saves\\Game")
            .expect("set value");

        let reg = hoard_manifest::ludusavi::RegistryPath {
            key: format!("HKEY_CURRENT_USER/{}", subkey.replace('\\', "/")),
            value: Some("SavePath".to_string()),
        };
        let out = expand_registry_path(&reg);
        assert_eq!(out, vec![PathBuf::from("C:\\Users\\Tester\\Saves\\Game")]);

        let _ = hkcu.delete_subkey_all(subkey);
    }
}
