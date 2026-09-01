//! Expand Ludusavi-style path templates into concrete filesystem paths.
//!
//! Ludusavi save-path entries use placeholders like `<winAppData>`,
//! `<xdgData>` and `<home>`. This module knows how to turn each of those
//! into one or more real directories on the host.
//!
//! Some placeholders fan out to multiple candidates (Steam libraries on
//! several disks, both XDG and `~/.config`, and so on), hence the `Vec<PathBuf>`
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
            // it verbatim: stripping the leading '/' would turn an absolute
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

/// The pieces of a template that can only be resolved against **live** host
/// state, so [`expand_path`] (which is pure) can't do it alone.
///
/// Two thirds of the manifest's templates are useless without this:
/// `<base>` alone leads 15.8k of them ("the game's own install folder"),
/// and dropping those templates is why a third of the catalog produced no
/// path at all on a machine that had the game installed.
#[derive(Debug, Default, Clone)]
pub struct PathScope {
    /// Where this specific game is installed; resolves `<base>`. Several
    /// candidates are normal: the same folder name can exist in more than
    /// one Steam library.
    pub install_dirs: Vec<PathBuf>,
    /// Storefront roots; resolves `<root>`. Steam's library roots plus any
    /// other storefront installed on this host ([`NON_STEAM_STORE_ROOTS`]).
    pub store_roots: Vec<PathBuf>,
}

impl PathScope {
    pub fn is_empty(&self) -> bool {
        self.install_dirs.is_empty() && self.store_roots.is_empty()
    }
}

/// Where one storefront keeps its own directory, which is what `<root>` names for the
/// templates constrained to that store.
///
/// Steam's roots come from live state (its library folders move), so they are
/// resolved separately; a store with a fixed layout only needs this. One table
/// serves both the native lookup ([`crate::roots::other_store_roots`]) and the
/// inside-a-prefix one, so the two can't drift.
pub struct StoreRootLayout {
    /// Path under `Program Files` / `Program Files (x86)`.
    pub program_files: &'static str,
    /// Path under `%LOCALAPPDATA%`, for a store that also keeps one there.
    pub local_appdata: Option<&'static str>,
}

/// Storefronts other than Steam that `<root>` can mean. **Adding one is a
/// row**; nothing else in the expander knows any store by name.
///
/// Today there is one, because the catalog says so: of the 3.1k `<root>`
/// templates, 2.9k are Steam's `userdata/...` and 219 are the Ubisoft
/// launcher's `savegames/<storeUserId>/<gameId>`, the only save path 63 games
/// declare, the whole Assassin's Creed / Far Cry / Watch Dogs line among them.
/// With Steam as the sole `<root>` those expanded to `…/Steam/savegames/…` and
/// found nothing, so the games came back with no save folder at all.
pub const NON_STEAM_STORE_ROOTS: &[StoreRootLayout] = &[StoreRootLayout {
    program_files: "Ubisoft/Ubisoft Game Launcher",
    local_appdata: Some("Ubisoft Game Launcher"),
}];

/// Expand a template that may reference the game's install dir.
///
/// Superset of [`expand_path_globbed`]:
///
/// * `<base>` → each of `scope.install_dirs`
/// * `<root>` → each of `scope.store_roots`
/// * `<storeUserId>` → `*` (globbed): the Steam account id isn't knowable
///   from a template, but the glob machinery resolves it against the real
///   directory. 5.8k templates carry it mid-path and used to expand to a
///   literal `<storeUserId>` segment that could never exist.
/// * `<osUserName>` → the current user name.
///
/// Anything else falls through to [`expand_path_globbed`] unchanged. A
/// template whose scope placeholder has no candidates yields nothing; the
/// game isn't installed here, so there is nothing to stat.
pub fn expand_path_scoped(template: &str, os: Os, scope: &PathScope) -> Vec<PathBuf> {
    let substituted = substitute_inline(template);
    let Some((placeholder, tail)) = split_placeholder(&substituted) else {
        // A literal that isn't absolute would resolve against the process
        // CWD, never a save location.
        if !is_absolute_literal(&substituted) {
            return Vec::new();
        }
        return expand_path_globbed(&substituted, os);
    };

    let scoped_bases: Option<&[PathBuf]> = match placeholder.as_str() {
        "base" => Some(&scope.install_dirs),
        "root" => Some(&scope.store_roots),
        _ => None,
    };
    let Some(bases) = scoped_bases else {
        return expand_path_globbed(&substituted, os);
    };

    let tail_clean = tail.trim_start_matches(['/', '\\']);
    let mut out = Vec::new();
    for base in bases {
        if tail_clean.is_empty() {
            // `<base>` on its own is the install directory: never a save
            // location, and offering it would snapshot the whole game.
            continue;
        }
        if has_glob(tail_clean) {
            expand_glob_tail(base, tail_clean, &mut out);
        } else {
            out.push(base.join(tail_clean));
        }
    }
    out
}

/// Replace the placeholders that resolve the same way wherever they appear
/// in a template (not just at the front).
fn substitute_inline(template: &str) -> String {
    let mut s = template.to_string();
    if s.contains("<storeUserId>") {
        // Not knowable from the template; the glob resolves it on disk.
        s = s.replace("<storeUserId>", "*");
    }
    if s.contains("<osUserName>") {
        if let Some(user) = os_user_name() {
            s = s.replace("<osUserName>", &user);
        }
    }
    s
}

fn os_user_name() -> Option<String> {
    for key in ["USER", "USERNAME", "LOGNAME"] {
        if let Some(v) = std::env::var_os(key) {
            let v = v.to_string_lossy().to_string();
            if !v.is_empty() {
                return Some(v);
            }
        }
    }
    home_dir()?
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
}

/// True for a path we can stat without a placeholder: rooted on Unix, or
/// carrying a drive letter / UNC prefix on Windows.
fn is_absolute_literal(s: &str) -> bool {
    let b = s.as_bytes();
    s.starts_with('/')
        || s.starts_with("\\\\")
        || (b.len() >= 3
            && b[0].is_ascii_alphabetic()
            && b[1] == b':'
            && (b[2] == b'\\' || b[2] == b'/'))
}

/// Maximum number of directory entries to consider per glob segment.
/// Caps the fan-out so a wildcard in a path like `<home>/Downloads/*/Saves`
/// doesn't enumerate thousands of entries on a busy folder.
const MAX_GLOB_FANOUT: usize = 64;

/// True if `s` contains a glob wildcard (`*` or `?`).
fn has_glob(s: &str) -> bool {
    s.contains('*') || s.contains('?')
}

/// Match a single path segment against a simple glob pattern (`*` = any
/// run of chars, `?` = exactly one char). No brace expansion, no `**`.
fn glob_match(pattern: &str, name: &str) -> bool {
    let p = pattern.as_bytes();
    let n = name.as_bytes();
    let (mut pi, mut ni) = (0, 0);
    let (mut star_pi, mut star_ni): (Option<usize>, usize) = (None, 0);
    while ni < n.len() {
        if pi < p.len() && (p[pi] == b'?' || p[pi] == n[ni]) {
            pi += 1;
            ni += 1;
        } else if pi < p.len() && p[pi] == b'*' {
            star_pi = Some(pi);
            star_ni = ni;
            pi += 1;
        } else if let Some(sp) = star_pi {
            pi = sp + 1;
            star_ni += 1;
            ni = star_ni;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == b'*' {
        pi += 1;
    }
    pi == p.len()
}

/// Like [`expand_path`] but resolves `*`/`?` wildcards in the tail against
/// the filesystem. Used by the catalog scan so templates like
/// `<home>/AppData/*.savegame` (Ludusavi emits them for games whose save
/// filenames aren't fixed ??? e.g. Twelve Minutes) produce real hits instead
/// of a literal `*` that never stats true.
///
/// Returns **directory** candidates (the save folders to watch), never
/// individual files:
/// - Glob in the **last** segment (e.g. `.../Twelve Minutes/*.savegame`):
///   the parent directory is returned, confirmed by a `read_dir` that at
///   least one entry matches the pattern.
/// - Glob in an **intermediate** segment (e.g. `.../Steam/*Saves/slot`):
///   that segment is expanded via `read_dir` and the remaining tail is
///   joined onto each match. Fan-out is capped at [`MAX_GLOB_FANOUT`].
///
/// When the tail has no glob the result is identical to [`expand_path`] ???
/// no filesystem access, pure join.
pub fn expand_path_globbed(template: &str, os: Os) -> Vec<PathBuf> {
    let (placeholder, tail) = match split_placeholder(template) {
        Some(parts) => parts,
        None => {
            if has_glob(template) {
                // Literal path with a glob: use the filesystem root as base
                // and the rest as tail so expand_glob_tail walks the segments.
                let mut out = Vec::new();
                let base = if template.starts_with('/') {
                    PathBuf::from("/")
                } else {
                    PathBuf::from("")
                };
                let tail = template.trim_start_matches(['/', '\\']);
                expand_glob_tail(&base, tail, &mut out);
                return out;
            }
            return vec![PathBuf::from(template)];
        }
    };
    let bases = expand_placeholder(&placeholder, os);
    if bases.is_empty() {
        return Vec::new();
    }
    let tail_clean = tail.trim_start_matches(['/', '\\']);
    if !has_glob(tail_clean) {
        return bases
            .into_iter()
            .map(|b| {
                if tail_clean.is_empty() {
                    b
                } else {
                    b.join(tail_clean)
                }
            })
            .collect();
    }
    let mut out = Vec::new();
    for base in bases {
        expand_glob_tail(&base, tail_clean, &mut out);
    }
    out
}

/// Walk `tail` (a relative path with possible glob segments) starting from
/// `base`, appending resolved directory candidates to `out`.
fn expand_glob_tail(base: &Path, tail: &str, out: &mut Vec<PathBuf>) {
    let tail = tail.trim_start_matches(['/', '\\']);
    if tail.is_empty() {
        out.push(base.to_path_buf());
        return;
    }
    let (first, rest) = match tail.find(['/', '\\']) {
        Some(i) => (&tail[..i], &tail[i + 1..]),
        None => (tail, ""),
    };
    let rest = rest.trim_start_matches(['/', '\\']);

    if !has_glob(first) {
        let next = base.join(first);
        if rest.is_empty() {
            out.push(next);
        } else {
            expand_glob_tail(&next, rest, out);
        }
        return;
    }

    if rest.is_empty() {
        // Glob in the LAST segment: return the parent dir (`base`), but
        // only if at least one entry in `base` matches the pattern.
        if !base.is_dir() {
            return;
        }
        if let Ok(rd) = std::fs::read_dir(base) {
            for e in rd.take(MAX_GLOB_FANOUT).flatten() {
                if let Some(name) = e.file_name().to_str() {
                    if glob_match(first, name) {
                        out.push(base.to_path_buf());
                        return;
                    }
                }
            }
        }
        return;
    }

    // Glob in an INTERMEDIATE segment: expand via read_dir, continue on
    // each match. Capped at MAX_GLOB_FANOUT entries per segment.
    if !base.is_dir() {
        return;
    }
    if let Ok(rd) = std::fs::read_dir(base) {
        let mut count = 0;
        for entry in rd {
            if count >= MAX_GLOB_FANOUT {
                break;
            }
            let Ok(e) = entry else { continue };
            let fname = e.file_name();
            let Some(name) = fname.to_str() else { continue };
            if !glob_match(first, name) {
                continue;
            }
            count += 1;
            let next = base.join(name);
            expand_glob_tail(&next, rest, out);
        }
    }
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
/// we know how to map to a Wine path; Linux and Mac-only placeholders
/// (`<xdgData>`, `<macAppSupport>`, …) and unknown tokens both yield
/// `vec![]` so the caller doesn't accidentally stat a meaningless path
/// under the prefix.
pub fn expand_path_in_prefix(template: &str, prefix: &Path) -> Vec<PathBuf> {
    // Proton always names its single Windows user `steamuser`; keep that as
    // the back-compatible default so existing Steam callers don't change.
    expand_path_in_prefix_as_user(template, prefix, "steamuser")
}

/// Like [`expand_path_in_prefix`] but for a specific Windows user inside the
/// prefix. Generic Wine prefixes (plain `wine`, PlayOnLinux, `.desktop`
/// launchers) name their user after the host login (`$USER`), not
/// `steamuser`, so the caller must pass the real user-dir name.
pub fn expand_path_in_prefix_as_user(template: &str, prefix: &Path, user: &str) -> Vec<PathBuf> {
    // Same inline substitution the native path does, with the prefix's own
    // Windows user instead of the host login. Without it `<storeUserId>` stayed
    // literal inside a prefix, so every template that carries one mid-path (the
    // whole Ubisoft launcher family, `savegames/<storeUserId>/<gameId>`)
    // expanded to a directory that can't exist.
    let substituted = template
        .replace("<storeUserId>", "*")
        .replace("<osUserName>", user);
    let Some((placeholder, tail)) = split_placeholder(&substituted) else {
        // Literal templates don't apply to a prefix; they're absolute
        // host paths, not Wine paths. Drop them.
        return Vec::new();
    };
    let bases = expand_placeholder_in_prefix(&placeholder, prefix, user);
    if bases.is_empty() {
        return Vec::new();
    }
    let tail_clean = tail.trim_start_matches(['/', '\\']);
    let mut out = Vec::new();
    for base in bases {
        if tail_clean.is_empty() {
            out.push(base);
        } else if has_glob(tail_clean) {
            expand_glob_tail(&base, tail_clean, &mut out);
        } else {
            out.push(base.join(tail_clean));
        }
    }
    out
}

/// Map one Ludusavi placeholder onto the corresponding directories inside a
/// Wine prefix for the given Windows user. Empty for placeholders that don't
/// apply (Linux/Mac tokens, per-install identifiers, unknown names); more than
/// one for `<root>`, which has as many candidates as there are storefronts
/// inside the prefix.
fn expand_placeholder_in_prefix(name: &str, prefix: &Path, user: &str) -> Vec<PathBuf> {
    let drive_c = prefix.join("drive_c");
    let userhome = drive_c.join("users").join(user);
    // `<root>` is the STOREFRONT root, and a prefix can hold more than one:
    // `drive_c` is the layout it always meant, and every storefront in the
    // table adds its own. Which one a template meant is decided by the store
    // constraint the catalog carries; expanding all of them and letting the
    // caller keep what exists is cheaper than threading that constraint down
    // here, and it can't misfire: two storefronts never share a tree.
    if name == "root" {
        let mut out = vec![drive_c.clone()];
        for store in NON_STEAM_STORE_ROOTS {
            out.push(
                drive_c
                    .join("Program Files (x86)")
                    .join(store.program_files),
            );
            out.push(drive_c.join("Program Files").join(store.program_files));
            if let Some(local) = store.local_appdata {
                out.push(userhome.join("AppData/Local").join(local));
            }
        }
        return out;
    }
    let mapped = match name {
        "winAppData" => userhome.join("AppData/Roaming"),
        "winLocalAppData" => userhome.join("AppData/Local"),
        "winLocalAppDataLow" => userhome.join("AppData/LocalLow"),
        "winDocuments" => userhome.join("Documents"),
        // `%USERPROFILE%\Saved Games` inside the prefix. Without this, games
        // that target `<winSavedGames>` (Planet S, plenty of modern titles)
        // were never searched under a Proton or Wine prefix on Linux, so detection
        // fell back to the low-confidence Steam Cloud stub instead.
        "winSavedGames" => userhome.join("Saved Games"),
        "winPublic" => drive_c.join("users/Public"),
        "winProgramData" => drive_c.join("ProgramData"),
        "winDir" => drive_c.join("windows"),
        "home" => userhome,
        _ => return Vec::new(),
    };
    vec![mapped]
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
    // `None` on non-Windows or when the registry value is missing; the
    // existing env-based match below kicks in as the fallback.
    if matches!(os, Os::Windows) {
        if let Some(p) = windows_known_folder(name) {
            return vec![p];
        }
    }

    match (os, name) {
        // -------- Cross-platform basics
        (_, "home") => home_dir().into_iter().collect(),
        // `<root>` is the STOREFRONT root (the Steam install dir), not the
        // filesystem root: 2.7k of its 3.1k uses are
        // `<root>/userdata/<storeUserId>/<appid>/remote`. Mapping it to `/`
        // produced `/userdata/...`, which never exists, so those templates
        // were dead weight either way. It needs live Steam state, so it is
        // resolved by `expand_path_scoped`; here there is nothing to give.
        (_, "root") => Vec::new(),

        // -------- Windows
        (Os::Windows, "winAppData") => env_dir("APPDATA"),
        (Os::Windows, "winLocalAppData") => env_dir("LOCALAPPDATA"),
        // `<winLocalAppDataLow>`, meaning `%USERPROFILE%\AppData\LocalLow`. Not
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
        // `<winSavedGames>`, meaning `%USERPROFILE%\Saved Games`, the Vista+
        // canonical save folder. Modern titles increasingly target it;
        // on non-Windows it returns an empty `Vec` so callers skip it.
        (Os::Windows, "winSavedGames") => home_dir()
            .map(|h| vec![h.join("Saved Games")])
            .unwrap_or_default(),

        // `<winSavedGames>` on native Linux. Plenty of cross-platform games
        // (Unity/Unreal and several indies, Planet S among them) keep the Windows
        // `~/Saved Games` convention in their Linux build, outside any Wine prefix.
        // Without this the catalogue did not resolve that path on the native one and
        // fell back to the install-dir's `saves` (often a Steam Cloud stub). Generic:
        // if the folder is missing or holds no saves, refinement discards it like any
        // other candidate.
        (Os::Linux, "winSavedGames") => home_dir()
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
            // An unknown placeholder is almost always a real bug: either
            // the manifest grew a new token that we haven't taught
            // pathexpand about, or the user is on an OS we don't handle.
            // Log it (sampled: `trace`, not `warn`) so we can spot gaps
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
    // heavy; std + a per-platform fallback covers the cases we hit.
    //
    // `USERPROFILE` is asked first on Windows, and the order is the whole
    // point. `HOME` is not a Windows variable; when it is set at all it was
    // set by something ported from Unix, Git Bash and MSYS above all, and it
    // frequently holds that shell's idea of the home rather than the account's
    // (`/c/Users/name`, or a drive that doesn't exist outside the shell).
    // Preferring it meant every `<home>` template, the blocked roots among them,
    // resolved somewhere the user's saves have never been, on machines
    // whose only sin was having Git installed.
    if cfg!(windows) {
        if let Some(h) = std::env::var_os("USERPROFILE") {
            return Some(PathBuf::from(h));
        }
    }
    if let Some(h) = std::env::var_os("HOME") {
        return Some(PathBuf::from(h));
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
/// the registry value is missing or unreadable, and callers then fall back to
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
        // No registry mapping for LocalLow or SavedGames: both are
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
            // Case-insensitive env var lookup, since Windows treats `%appdata%`
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
        // A test that pins HOME means "the home directory is this", and on
        // Windows that takes both variables: `home_dir` asks USERPROFILE first
        // there (see its comment), so leaving it alone would have the test read
        // the runner's real profile and assert against a path it never chose.
        let mut pairs = pairs.to_vec();
        if let Some((_, home)) = pairs.iter().find(|(name, _)| *name == "HOME").copied() {
            if !pairs.iter().any(|(name, _)| *name == "USERPROFILE") {
                pairs.push(("USERPROFILE", home));
            }
        }
        let pairs = &pairs[..];
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
            (
                "<winAppData>/X",
                "/p/drive_c/users/steamuser/AppData/Roaming/X",
            ),
            (
                "<winLocalAppData>/X",
                "/p/drive_c/users/steamuser/AppData/Local/X",
            ),
            (
                "<winLocalAppDataLow>/X",
                "/p/drive_c/users/steamuser/AppData/LocalLow/X",
            ),
            ("<winDocuments>/X", "/p/drive_c/users/steamuser/Documents/X"),
            ("<winPublic>/X", "/p/drive_c/users/Public/X"),
            ("<winProgramData>/X", "/p/drive_c/ProgramData/X"),
            ("<winDir>/X", "/p/drive_c/windows/X"),
            ("<home>/X", "/p/drive_c/users/steamuser/X"),
        ];
        for (template, expected) in cases {
            let out = expand_path_in_prefix(template, &prefix);
            assert_eq!(
                out,
                vec![PathBuf::from(expected)],
                "template {template} mismatched"
            );
        }
        // `<root>` is the odd one out: a prefix can hold more than one
        // storefront, so it expands to every candidate. `drive_c` stays first
        // (the Steam-in-prefix layout it always meant).
        let root = expand_path_in_prefix("<root>/X", &prefix);
        assert_eq!(root.first(), Some(&PathBuf::from("/p/drive_c/X")));
        assert!(root.contains(&PathBuf::from(
            "/p/drive_c/Program Files (x86)/Ubisoft/Ubisoft Game Launcher/X"
        )));
    }

    /// The reason `<root>` grew a list, and the reason the prefix expander
    /// substitutes inline placeholders: every modern Assassin's Creed (and the
    /// rest of the Ubisoft line) declares exactly one save path,
    /// `<root>/savegames/<storeUserId>/<gameId>`. Under Proton that used to
    /// expand to `drive_c/savegames/<storeUserId>/…`, a literal
    /// `<storeUserId>` under a folder that doesn't exist, so the game came
    /// back with no save folder and the user had to find it by hand.
    #[test]
    fn a_ubisoft_save_resolves_inside_a_proton_prefix() {
        let tmp = tempfile::tempdir().unwrap();
        let prefix = tmp.path().join("pfx");
        let save = prefix
            .join("drive_c/Program Files (x86)/Ubisoft/Ubisoft Game Launcher/savegames")
            .join("1234567")
            .join("5092");
        std::fs::create_dir_all(&save).unwrap();

        let out = expand_path_in_prefix("<root>/savegames/<storeUserId>/5092", &prefix);
        assert!(
            out.contains(&save),
            "the launcher's save inside the prefix didn't resolve: {out:?}"
        );
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

    /// `<winSavedGames>` resolves to `~/Saved Games` on native Linux too:
    /// the Linux builds of many cross-platform games keep that same layout, so
    /// the token still resolves there. On a native Linux-only game there is no
    /// such convention, and the token keeps falling through.
    #[test]
    fn winsavedgames_resolves_under_os_linux_drops_on_mac() {
        with_env(&[("HOME", Some("/home/test"))], || {
            let out = expand_path("<winSavedGames>/MyGame", Os::Linux);
            assert_eq!(out, vec![PathBuf::from("/home/test/Saved Games/MyGame")]);
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
                // Windows hosts running the test suite, and we don't want
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

    #[test]
    fn glob_last_segment_returns_parent_dir() {
        // Template like Twelve Minutes: `<home>/AppData/Nomada/Twelve Minutes/*.savegame`
        // The candidate must be the parent dir, confirmed by a matching file.
        let _guard = crate::test_lock::ENV
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let tmp = std::env::temp_dir().join(format!("hoard-glob-last-{}", std::process::id()));
        let save_dir = tmp.join("AppData/Nomada/Twelve Minutes");
        std::fs::create_dir_all(&save_dir).unwrap();
        std::fs::write(save_dir.join("slot1.savegame"), b"x").unwrap();
        let prev = std::env::var_os("HOME");
        let prev_profile = std::env::var_os("USERPROFILE");
        std::env::set_var("HOME", &tmp);
        // Windows resolves `<home>` from USERPROFILE first, so pinning HOME
        // alone would leave this reading the runner's real profile.
        std::env::set_var("USERPROFILE", &tmp);
        let out = expand_path_globbed("<home>/AppData/Nomada/Twelve Minutes/*.savegame", Os::Linux);
        assert_eq!(out, vec![save_dir.clone()]);
        // No matching file -> no hit.
        std::fs::remove_file(save_dir.join("slot1.savegame")).unwrap();
        let out2 =
            expand_path_globbed("<home>/AppData/Nomada/Twelve Minutes/*.savegame", Os::Linux);
        assert!(out2.is_empty(), "should be empty with no matching file");
        match prev {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
        match prev_profile {
            Some(v) => std::env::set_var("USERPROFILE", v),
            None => std::env::remove_var("USERPROFILE"),
        }
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn glob_intermediate_segment_expands() {
        // Template like `<home>/Steam/*Saves/slot` ??? the `*Saves` segment
        // is a glob in an intermediate position. Both `remoteSaves` and
        // `cloudSaves` should expand; `other` should not.
        let _guard = crate::test_lock::ENV
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let tmp = std::env::temp_dir().join(format!("hoard-glob-mid-{}", std::process::id()));
        for d in [
            "Steam/remoteSaves/slot",
            "Steam/cloudSaves/slot",
            "Steam/other/slot",
        ] {
            std::fs::create_dir_all(tmp.join(d)).unwrap();
        }
        let prev = std::env::var_os("HOME");
        let prev_profile = std::env::var_os("USERPROFILE");
        std::env::set_var("HOME", &tmp);
        // Windows resolves `<home>` from USERPROFILE first, so pinning HOME
        // alone would leave this reading the runner's real profile.
        std::env::set_var("USERPROFILE", &tmp);
        let out = expand_path_globbed("<home>/Steam/*Saves/slot", Os::Linux);
        assert!(
            out.contains(&tmp.join("Steam/remoteSaves/slot")),
            "got {out:?}"
        );
        assert!(
            out.contains(&tmp.join("Steam/cloudSaves/slot")),
            "got {out:?}"
        );
        assert!(!out.contains(&tmp.join("Steam/other/slot")), "got {out:?}");
        match prev {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
        match prev_profile {
            Some(v) => std::env::set_var("USERPROFILE", v),
            None => std::env::remove_var("USERPROFILE"),
        }
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn glob_no_wildcard_delegates_to_expand_path() {
        // No glob -> identical to expand_path, no FS access.
        with_env(&[("HOME", Some("/home/test"))], || {
            let a = expand_path("<home>/Saves/foo", Os::Linux);
            let b = expand_path_globbed("<home>/Saves/foo", Os::Linux);
            assert_eq!(a, b);
        });
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

    // ---- <base> / <root> ------------------------------------------------

    #[test]
    fn base_resolves_against_each_install_dir() {
        let scope = PathScope {
            install_dirs: vec![
                PathBuf::from("/lib1/common/Game"),
                PathBuf::from("/lib2/Game"),
            ],
            store_roots: Vec::new(),
        };
        assert_eq!(
            expand_path_scoped("<base>/saves", Os::Linux, &scope),
            vec![
                PathBuf::from("/lib1/common/Game/saves"),
                PathBuf::from("/lib2/Game/saves"),
            ]
        );
    }

    #[test]
    fn base_without_install_dirs_yields_nothing() {
        // The game isn't installed here: there is nothing to stat, and
        // guessing a path would be worse than reporting no candidates.
        let scope = PathScope::default();
        assert!(expand_path_scoped("<base>/saves", Os::Linux, &scope).is_empty());
    }

    #[test]
    fn bare_base_is_never_a_save_location() {
        // `<base>` alone is the install directory. Offering it would
        // snapshot the whole multi-GB game.
        let scope = PathScope {
            install_dirs: vec![PathBuf::from("/lib/Game")],
            store_roots: Vec::new(),
        };
        assert!(expand_path_scoped("<base>", Os::Linux, &scope).is_empty());
        assert!(expand_path_scoped("<base>/", Os::Linux, &scope).is_empty());
    }

    #[test]
    fn root_resolves_to_the_store_root_not_the_filesystem_root() {
        // The old mapping turned `<root>/userdata/...` into `/userdata/...`.
        let scope = PathScope {
            install_dirs: Vec::new(),
            store_roots: vec![PathBuf::from("/home/u/.steam/steam")],
        };
        assert_eq!(
            expand_path_scoped("<root>/config", Os::Linux, &scope),
            vec![PathBuf::from("/home/u/.steam/steam/config")]
        );
        // And with no Steam on the box, nothing at all.
        assert!(expand_path_scoped("<root>/config", Os::Linux, &PathScope::default()).is_empty());
    }

    #[test]
    fn store_user_id_becomes_a_glob() {
        // 5.8k templates carry `<storeUserId>` mid-path; expanding it
        // literally produced a segment that can never exist on disk.
        let tmp = tempfile::tempdir().unwrap();
        let real = tmp
            .path()
            .join("userdata")
            .join("76561198000000000")
            .join("413150");
        std::fs::create_dir_all(&real).unwrap();
        let scope = PathScope {
            install_dirs: Vec::new(),
            store_roots: vec![tmp.path().to_path_buf()],
        };
        let out = expand_path_scoped("<root>/userdata/<storeUserId>/413150", Os::Linux, &scope);
        assert_eq!(out, vec![real]);
    }

    #[test]
    fn a_relative_literal_is_dropped_rather_than_resolved_against_the_cwd() {
        assert!(expand_path_scoped("saves/slot1", Os::Linux, &PathScope::default()).is_empty());
    }

    #[test]
    fn scoped_expansion_falls_through_for_ordinary_placeholders() {
        with_env(
            &[
                ("HOME", Some("/home/tester")),
                ("USERPROFILE", Some("/home/tester")),
            ],
            || {
                assert_eq!(
                    expand_path_scoped(
                        "<home>/.local/share/Game",
                        Os::Linux,
                        &PathScope::default()
                    ),
                    vec![PathBuf::from("/home/tester/.local/share/Game")]
                );
            },
        );
    }
}
