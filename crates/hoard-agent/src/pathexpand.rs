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

use std::path::PathBuf;

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
            // No placeholder at the start: template is literal and probably
            // useless to us (Ludusavi only emits absolute literals on rare
            // entries). Return as-is.
            let trimmed = template.trim_start_matches('/');
            return vec![PathBuf::from(trimmed)];
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
    match (os, name) {
        // -------- Cross-platform basics
        (_, "home") => home_dir().into_iter().collect(),
        (_, "root") => vec![PathBuf::from(if cfg!(windows) { "C:\\" } else { "/" })],

        // -------- Windows
        (Os::Windows, "winAppData") => env_dir("APPDATA"),
        (Os::Windows, "winLocalAppData") => env_dir("LOCALAPPDATA"),
        (Os::Windows, "winDocuments") => {
            // Best-effort: %USERPROFILE%\Documents. The real Documents path
            // can be redirected via Known Folders, but the env-based fallback
            // is what 99% of installs use.
            home_dir().map(|h| vec![h.join("Documents")]).unwrap_or_default()
        }
        (Os::Windows, "winPublic") => env_dir("PUBLIC"),
        (Os::Windows, "winProgramData") => env_dir("PROGRAMDATA"),
        (Os::Windows, "winDir") => env_dir("WINDIR"),

        // -------- Linux / XDG
        (Os::Linux, "xdgData") => xdg_or(home_dir().map(|h| h.join(".local/share")), "XDG_DATA_HOME"),
        (Os::Linux, "xdgConfig") => xdg_or(home_dir().map(|h| h.join(".config")), "XDG_CONFIG_HOME"),
        (Os::Linux, "xdgState") => xdg_or(home_dir().map(|h| h.join(".local/state")), "XDG_STATE_HOME"),
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
            // These are per-install identifiers we don't know yet at
            // expansion time. Returning an empty list drops the template;
            // detection.rs handles the wildcard case separately.
            Vec::new()
        }

        _ => Vec::new(),
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

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Mutex;

    // Tests in this module mutate the process environment, which is shared
    // across threads. cargo test runs them in parallel by default, so we
    // serialise with a single mutex instead of asking users to remember
    // `--test-threads=1`.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_env<F: FnOnce()>(pairs: &[(&str, Option<&str>)], f: F) {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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
        assert_eq!(out, vec![PathBuf::from("etc/games/foo")]);
    }

    #[test]
    fn placeholder_with_no_tail() {
        with_env(&[("HOME", Some("/home/test"))], || {
            let out = expand_path("<home>", Os::Linux);
            assert_eq!(out, vec![PathBuf::from("/home/test")]);
        });
    }
}
