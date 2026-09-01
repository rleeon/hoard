//! Embedded Ludusavi catalog with optional runtime refresh.
//!
//! The desktop app needs to know save-path templates for ~20k games to
//! detect installed games offline (no server round-trips). Hand-curating
//! that many entries is impossible; the [Ludusavi][1] community manifest
//! already does the work and is the de-facto standard for save-sync data.
//!
//! ## Two datasets
//!
//! - [`catalog`]: the games we can back up, with save paths plus what it takes
//!   to resolve them (`install_dirs` for `<base>`) and to recognise the game
//!   while it runs (`launch_exes`).
//! - [`titles`]: the two thirds of the manifest with **no** save path. They
//!   can't be tracked, so keeping them in the catalog would only slow every
//!   scan down; they exist to put a name on a process or an appid.
//!
//! ## How they get loaded
//!
//! Two sources, in priority order:
//!
//! 1. **Runtime override** at `<cache_dir>/hoard/ludusavi-{catalog,titles}.json`,
//!    written by [`save_runtime_override`] after a successful update.
//!    This lets the desktop refresh without a re-install. An override written
//!    before `launch:`/`installDir:` existed is detected by shape and ignored,
//!    so an old file can't silently switch those features off.
//! 2. **Compile-time embedded** blobs (~1.7 MB of zstd for ~11.6 MB of JSON).
//!    Always available, always works offline.
//!
//! Both resolve lazily on first call and cache in a `OnceLock`. After
//! [`save_runtime_override`] writes a new file, the next *process* picks it
//! up; we deliberately don't hot-swap the cached slice mid-run because that
//! would make detection results inconsistent across concurrent scans.
//!
//! ## Refreshing
//!
//! The desktop downloads the upstream YAML and hands it to
//! [`save_runtime_override`], which runs [`convert_yaml`] and writes both
//! files to the cache dir. That is the entry point both for the "Update
//! catalog" button and for the background refresh on app startup, and it is
//! the *same* conversion the embedded blobs are generated with (see
//! `data/README.md`), so shipped and refreshed data can't drift apart.
//!
//! ## Path syntax
//!
//! Ludusavi templates use angle-bracket placeholders (`<winAppData>`,
//! `<xdgData>`, `<home>`, `<storeUserId>` and so on). These are expanded by
//! `hoard-agent::pathexpand`, **not** by [`crate::placeholders`]: the
//! placeholder vocabulary is different.
//!
//! ## Licensing
//!
//! The manifest data is sourced from [PCGamingWiki][2] and is licensed
//! **CC BY-NC-SA 3.0**. Three obligations come with it and all three are
//! ours: attribution (see `NOTICE`, the Terms page and the app's About
//! screen), share-alike on anything derived from it, and **NonCommercial**.
//!
//! That last one is the load-bearing part. "Primarily intended for or
//! directed toward commercial advantage" is not obviously satisfied by a
//! build distributed next to a paid subscription, whoever is doing the
//! distributing. The old note here told *other* distributors to strip the
//! JSON, which quietly assumed this one wasn't commercial.
//!
//! The `bundled-catalog` feature (on by default) is the lever: turn it off
//! and the binary ships with no catalogue at all, fetching one at first run
//! into the user's own cache via [`save_runtime_override`]. That is the
//! honest build to ship the day the NC clause has to be respected rather
//! than argued about.
//!
//! [1]: https://github.com/mtkennerly/ludusavi-manifest
//! [2]: https://www.pcgamingwiki.com/

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

/// Compact JSON produced by the `regenerate_embedded_catalog` generator,
/// zstd-compressed: ~10 MB of JSON becomes ~1 MB of binary. Decompressed
/// once, lazily, on the first [`catalog`] call.
#[cfg(feature = "bundled-catalog")]
const CATALOG_ZST: &[u8] = include_bytes!("../data/ludusavi-catalog.json.zst");

/// Default upstream URL the desktop fetches before calling
/// [`save_runtime_override`].
pub const DEFAULT_UPSTREAM_URL: &str =
    "https://raw.githubusercontent.com/mtkennerly/ludusavi-manifest/master/data/manifest.yaml";

/// Sub-path inside the OS cache dir where we persist runtime catalog
/// overrides. Used by [`runtime_override_path`] / [`save_runtime_override`].
const RUNTIME_OVERRIDE_REL: &str = "hoard/ludusavi-catalog.json";

/// Same idea for the title-only index, kept in a sibling file so a refresh
/// updates both together.
const TITLES_OVERRIDE_REL: &str = "hoard/ludusavi-titles.json";

/// Compact `name / appid / exes` index for the manifest games that carry no
/// save path. Names processes and appids; never used to detect a save.
#[cfg(feature = "bundled-catalog")]
const TITLES_ZST: &[u8] = include_bytes!("../data/ludusavi-titles.json.zst");

/// The two data files, tied to the same generation. See [`manifest_data`] for why
/// they are not two loose `OnceLock`s.
static MANIFEST_DATA: OnceLock<(Vec<LudusaviEntry>, Vec<TitleEntry>)> = OnceLock::new();

/// One game from the Ludusavi catalog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LudusaviEntry {
    /// Stable slug derived from `display_name`. Used as the catalog key.
    pub slug: String,
    /// The pretty title (the YAML's top-level key).
    pub display_name: String,
    /// Steam application id, if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub steam_app_id: Option<u64>,
    /// Per-OS save-path templates with optional store/tag constraints.
    pub paths: LudusaviPaths,
    /// Windows registry locations where the game stores save data. Each
    /// entry is a `HKEY_*` key (and optionally a value name); expansion to
    /// a filesystem path happens at detection time and is a no-op on
    /// non-Windows hosts. See ADR 0011.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub registry: Vec<RegistryPath>,
    /// Folder names the game installs into, from the manifest's
    /// `installDir:` block. These are what `<base>` and `<game>` in a save
    /// template resolve to: `<base>/saves` means "the `saves` folder inside
    /// wherever this game is installed". Without them, ~15.7k templates in
    /// the manifest expand to nothing at all.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub install_dirs: Vec<String>,
    /// Executable **basenames** (lowercased) taken from the manifest's
    /// `launch:` block, so `<base>/Binaries/Win64/EYE.exe` contributes
    /// `eye.exe`. This is the community-maintained answer to "which process
    /// means the user is playing this game", which the agent otherwise has
    /// to guess from the slug. Ambiguous names shared by several games are
    /// vetoed at lookup time, not here (see `exe_index`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub launch_exes: Vec<String>,
    /// Additional Steam appids the same game ships under: regional SKUs,
    /// demos, dev builds (`id.steamExtra` upstream). An installed app whose
    /// id only matches here is still this game, and without them the appid
    /// cross-reference silently misses it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steam_extra_ids: Vec<u64>,
    /// Lutris game slug (`id.lutris` upstream). Lutris names its prefix
    /// directory after this, so it resolves a prefix to a game exactly
    /// instead of by slugifying the folder name and hoping.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lutris_slug: Option<String>,
    /// The game declares Steam Cloud support (`cloud.steam` upstream).
    /// Purely informational, surfaced to the user as "Steam already syncs
    /// this one". It must **never** change detection confidence, ordering,
    /// or auto-track priority.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub cloud_steam: bool,
}

/// A manifest game we know the **name** of but not where it saves.
///
/// Two thirds of the upstream manifest is like this: a title, usually a
/// Steam appid, and a `launch:` block, but no save path. They can't be
/// tracked, so they don't belong in [`LudusaviEntry`], but they answer
/// "what game is this process / appid?", which is what phase-4 attribution
/// and the untracked-process notice need in order to stop naming a save
/// after whatever happened to be running.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TitleEntry {
    #[serde(rename = "n")]
    pub display_name: String,
    #[serde(default, rename = "s", skip_serializing_if = "Option::is_none")]
    pub steam_app_id: Option<u64>,
    #[serde(default, rename = "e", skip_serializing_if = "Vec::is_empty")]
    pub launch_exes: Vec<String>,
}

/// A single Windows registry location from the Ludusavi catalog.
///
/// `key` is the full key path including the hive prefix
/// (`HKEY_CURRENT_USER/Software/Foo/Bar`). `value` is the optional named
/// value inside that key; `None` means the consumer should read the
/// subkey's default value (Ludusavi's convention).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryPath {
    /// Full registry key path, hive prefix included.
    pub key: String,
    /// Specific value name to read inside `key`. `None` means default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// Per-OS bundle of save paths in the Ludusavi catalog.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LudusaviPaths {
    #[serde(default)]
    pub windows: Vec<LudusaviSavePath>,
    #[serde(default)]
    pub linux: Vec<LudusaviSavePath>,
    #[serde(default)]
    pub mac: Vec<LudusaviSavePath>,
}

/// One save-path template inside [`LudusaviPaths`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LudusaviSavePath {
    /// Template using Ludusavi placeholders: `<winAppData>/Game/Saves`,
    /// `<home>/.config/...`, etc. Expand with `hoard-agent::pathexpand`.
    pub path: String,
    /// `when` clauses from Ludusavi: which storefront(s) the path applies
    /// to. An empty `store` field means "any store".
    #[serde(default)]
    pub constraints: Vec<LudusaviConstraint>,
    /// Ludusavi tags (`save`, `config`, `screenshots`, …). The conversion
    /// script keeps only entries tagged `save` (or untagged), so this is
    /// almost always `["save"]` in the embedded catalog.
    #[serde(default)]
    pub tags: Vec<String>,
}

/// One `when` clause from a Ludusavi save-path entry.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LudusaviConstraint {
    /// Ludusavi storefront (`steam`, `gog`, `epic`, `microsoft`, …).
    /// `None` means the path applies to every store.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub store: Option<String>,
}

/// Errors from runtime override / refresh operations.
#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    #[error("could not determine the OS cache directory")]
    NoCacheDir,
    #[error("I/O error at {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("could not parse Ludusavi YAML: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("could not serialize JSON catalog: {0}")]
    Json(#[from] serde_json::Error),
}

/// Path where runtime catalog overrides are written. `None` if we can't
/// determine the OS cache dir on this system (extremely rare).
pub fn runtime_override_path() -> Option<PathBuf> {
    let dirs = directories::BaseDirs::new()?;
    Some(dirs.cache_dir().join(RUNTIME_OVERRIDE_REL))
}

/// `true` when a loaded override carries no `launch:`/`installDir:` data at
/// all, which for a real manifest is impossible (~18k of ~21k entries have
/// one or the other) and therefore means the file was written by a build
/// from before those fields existed. Checking a bounded prefix keeps this
/// O(1) on a 20k-entry catalog.
fn is_outdated_override(entries: &[LudusaviEntry]) -> bool {
    !entries.is_empty()
        && !entries
            .iter()
            .take(2000)
            .any(|e| !e.launch_exes.is_empty() || !e.install_dirs.is_empty())
}

/// Sibling of [`runtime_override_path`] for the title-only index.
pub fn titles_override_path() -> Option<PathBuf> {
    let dirs = directories::BaseDirs::new()?;
    Some(dirs.cache_dir().join(TITLES_OVERRIDE_REL))
}

/// Load the title index override, or `None` to fall back to the embedded
/// one. A missing file is normal: an override written by an older build
/// updated the catalog only.
fn load_titles_override() -> Option<Vec<TitleEntry>> {
    let path = titles_override_path()?;
    let text = std::fs::read_to_string(&path).ok()?;
    match serde_json::from_str::<Vec<TitleEntry>>(&text) {
        Ok(entries) => Some(entries),
        Err(e) => {
            tracing::warn!(path = %path.display(), error = %e,
                "runtime title index malformed; falling back to embedded");
            None
        }
    }
}

/// Try to load a runtime override from the cache dir. Returns `None` if
/// the file is absent, unreadable, or doesn't parse, in which case the
/// caller falls back to the embedded catalog.
fn load_runtime_override() -> Option<Vec<LudusaviEntry>> {
    let path = runtime_override_path()?;
    if !path.exists() {
        return None;
    }
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!(path = %path.display(), error = %e,
                "runtime catalog override unreadable; falling back to embedded");
            return None;
        }
    };
    // An override written before the catalog grew `launch_exes` /
    // `install_dirs` still deserializes cleanly (every new field has a
    // default) and would silently switch off process matching and every
    // `<base>` template until the next refresh. Detect it by shape and
    // ignore it: the embedded catalog is complete, and the daily refresh
    // rewrites the override in the new form.
    match serde_json::from_str::<Vec<LudusaviEntry>>(&text) {
        Ok(entries) if is_outdated_override(&entries) => {
            tracing::info!(
                path = %path.display(),
                "catalog override predates launch/installDir data; using the embedded catalog until the next refresh"
            );
            None
        }
        Ok(entries) => {
            tracing::info!(
                path = %path.display(),
                count = entries.len(),
                "loaded Ludusavi catalog runtime override"
            );
            Some(entries)
        }
        Err(e) => {
            tracing::warn!(path = %path.display(), error = %e,
                "runtime catalog override malformed; falling back to embedded");
            None
        }
    }
}

/// Parse the embedded catalog on first call, then return the cached slice.
///
/// Resolves the *runtime override* (cache dir) first; falls back to the
/// embedded JSON. The choice is sticky for the lifetime of the process; updates
/// take effect on the next launch.
pub fn catalog() -> &'static [LudusaviEntry] {
    &manifest_data().0
}

/// The catalog and the titles **at once**, from the same generation of data.
///
/// These used to be two loose `OnceLock`s, and there was a real mixed state there:
/// each resolves the first time anybody touches it, and between those two moments
/// the override file can appear ([`save_runtime_override`] writes it with the
/// process alive) or the `XDG_CACHE_HOME` that locates it can change. The result was
/// a catalog from one generation with titles from another, and with them indexes
/// that say things neither of the two says: a `mars.exe` that two games claim in the
/// old catalog and only one claims in the new titles starts resolving to that one,
/// and a save ends up named after a different game.
///
/// Resolving them in the same `get_or_init` ties them to a single instant. The
/// override still lands on the next process, which is the contract above; what it
/// can no longer do is land halfway.
fn manifest_data() -> &'static (Vec<LudusaviEntry>, Vec<TitleEntry>) {
    MANIFEST_DATA.get_or_init(|| (load_catalog(), load_titles()))
}

fn load_catalog() -> Vec<LudusaviEntry> {
    if let Some(override_) = load_runtime_override() {
        return override_;
    }
    embedded_catalog()
}

#[cfg(feature = "bundled-catalog")]
fn embedded_catalog() -> Vec<LudusaviEntry> {
    // The embedded blob was emitted by our own generator; a decode
    // failure is a build-time invariant violation, not user-facing.
    let json = zstd::decode_all(CATALOG_ZST)
        .unwrap_or_else(|e| panic!("embedded ludusavi catalog must decompress: {e}"));
    serde_json::from_slice(&json).unwrap_or_else(|e| {
        tracing::error!("embedded Ludusavi catalog parse failed: {e}");
        panic!("embedded ludusavi-catalog.json must parse: {e}");
    })
}

/// With no embedded catalog there is nothing to load until somebody downloads one:
/// it returns empty and does **not** panic. An empty catalog degrades detection to
/// what the system can deduce on its own (Steam, processes, folders pointed at by
/// hand), which is little but works; aborting the start would turn a licensing
/// decision into an app that does not open.
#[cfg(not(feature = "bundled-catalog"))]
fn embedded_catalog() -> Vec<LudusaviEntry> {
    tracing::warn!(
        "built without the bundled catalogue: detection stays thin until one is downloaded"
    );
    Vec::new()
}

fn load_titles() -> Vec<TitleEntry> {
    if let Some(override_) = load_titles_override() {
        return override_;
    }
    embedded_titles()
}

#[cfg(feature = "bundled-catalog")]
fn embedded_titles() -> Vec<TitleEntry> {
    // Softer failure than the catalog: without the title index we
    // fall back to naming things after the process, which is worse
    // but not broken.
    zstd::decode_all(TITLES_ZST)
        .ok()
        .and_then(|json| serde_json::from_slice(&json).ok())
        .unwrap_or_else(|| {
            tracing::error!("embedded Ludusavi title index unreadable");
            Vec::new()
        })
}

#[cfg(not(feature = "bundled-catalog"))]
fn embedded_titles() -> Vec<TitleEntry> {
    Vec::new()
}

/// Number of games in the (currently-loaded) catalog. Useful for progress
/// bars.
pub fn catalog_size() -> usize {
    catalog().len()
}

/// The title-only index: manifest games with a name but no save path.
pub fn titles() -> &'static [TitleEntry] {
    &manifest_data().1
}

/// Lazily-built lookup tables over [`catalog`] and [`titles`].
struct Indexes {
    by_app_id: HashMap<u64, usize>,
    by_slug: HashMap<&'static str, usize>,
    /// Lutris game slug → catalog index, for naming a Lutris prefix.
    by_lutris: HashMap<&'static str, usize>,
    /// Canonical name (letters+digits only) → catalog index. Recognises the
    /// same game across the spellings a *folder* uses: `StardewValley`,
    /// `stardew-valley` and `Stardew Valley` all collapse to one key, which
    /// `slugify` alone can't do (it never splits CamelCase).
    by_canon: HashMap<String, usize>,
    /// Executable basename → catalog index. Only names owned by exactly one
    /// game are present (see [`exe_owner`] for why).
    exe_to_slug: HashMap<&'static str, usize>,
    /// Executable basename → display name, across catalog **and** titles.
    /// Same uniqueness rule.
    exe_to_title: HashMap<&'static str, &'static str>,
    /// Steam appid → display name, across catalog and titles.
    app_id_to_title: HashMap<u64, &'static str>,
    /// Canonical name to display name, across catalog **and** titles. The twin of
    /// [`Indexes::by_canon`] for *naming*: a game with no save path is not in the
    /// catalog, so `by_canon` does not find it and a folder named after it ended up
    /// as a phantom game with a raw name.
    canon_to_title: HashMap<String, &'static str>,
}

static INDEXES: OnceLock<Indexes> = OnceLock::new();

/// Build the lookup tables once. The uniqueness rule for executables is
/// load-bearing: 692 names in the manifest (`game.exe` ×730, `launcher.exe`,
/// `nw.exe`, `dosbox.exe`, `scummvm.exe` and friends) are shared by several games,
/// and
/// treating one of those as "you are playing X" would attribute sessions,
/// playtime and save folders to an arbitrary title. A name owned by more than
/// one game is therefore dropped from the index entirely rather than resolved
/// to a guess, the same rule the agent's correlation filter already applies.
fn indexes() -> &'static Indexes {
    INDEXES.get_or_init(|| {
        let cat = catalog();
        let tit = titles();

        let mut by_app_id = HashMap::with_capacity(cat.len());
        let mut by_slug = HashMap::with_capacity(cat.len());
        let mut by_lutris = HashMap::new();
        let mut by_canon: HashMap<String, usize> = HashMap::with_capacity(cat.len());
        for (i, e) in cat.iter().enumerate() {
            if let Some(id) = e.steam_app_id {
                by_app_id.entry(id).or_insert(i);
            }
            by_slug.entry(e.slug.as_str()).or_insert(i);
            if let Some(l) = e.lutris_slug.as_deref() {
                by_lutris.entry(l).or_insert(i);
            }
            let canon = hoard_core::ids::canon_token(&e.display_name);
            // Demasiado corto ⇒ colisiona con cualquier carpeta ("go", "if").
            if canon.len() >= hoard_core::ids::MIN_IDENTITY_TOKEN_LEN {
                by_canon.entry(canon).or_insert(i);
            }
        }
        // Secondary appids in a second pass: a primary `steam.id` must always
        // win over another game's `steamExtra` listing the same number.
        for (i, e) in cat.iter().enumerate() {
            for id in &e.steam_extra_ids {
                by_app_id.entry(*id).or_insert(i);
            }
        }

        // Count owners before inserting, so an ambiguous name is never kept.
        let mut exe_owners: HashMap<&str, u32> = HashMap::new();
        for e in cat {
            for x in &e.launch_exes {
                *exe_owners.entry(x.as_str()).or_default() += 1;
            }
        }
        let mut exe_to_slug = HashMap::new();
        for (i, e) in cat.iter().enumerate() {
            for x in &e.launch_exes {
                if exe_owners.get(x.as_str()) == Some(&1) {
                    exe_to_slug.insert(x.as_str(), i);
                }
            }
        }

        // The title index widens the same map: a name is unique only if no
        // *other* game, catalog or title-only, also claims it.
        let mut title_owners: HashMap<&str, u32> = exe_owners;
        for t in tit {
            for x in &t.launch_exes {
                *title_owners.entry(x.as_str()).or_default() += 1;
            }
        }
        let mut exe_to_title = HashMap::new();
        let mut app_id_to_title = HashMap::new();
        for (name, exes, app) in cat
            .iter()
            .map(|e| (e.display_name.as_str(), &e.launch_exes, e.steam_app_id))
            .chain(
                tit.iter()
                    .map(|t| (t.display_name.as_str(), &t.launch_exes, t.steam_app_id)),
            )
        {
            for x in exes {
                if title_owners.get(x.as_str()) == Some(&1) {
                    exe_to_title.insert(x.as_str(), name);
                }
            }
            if let Some(id) = app {
                app_id_to_title.entry(id).or_insert(name);
            }
        }

        // And the canonical name, with the same rule: the catalog first, and the
        // title-only entries fill the gaps it does not cover.
        let mut canon_to_title: HashMap<String, &'static str> =
            HashMap::with_capacity(cat.len() + tit.len());
        for (name, canon) in cat
            .iter()
            .map(|e| e.display_name.as_str())
            .chain(tit.iter().map(|t| t.display_name.as_str()))
            .map(|name| (name, hoard_core::ids::canon_token(name)))
        {
            if canon.len() >= hoard_core::ids::MIN_IDENTITY_TOKEN_LEN {
                canon_to_title.entry(canon).or_insert(name);
            }
        }

        Indexes {
            by_app_id,
            by_slug,
            by_lutris,
            by_canon,
            exe_to_slug,
            exe_to_title,
            app_id_to_title,
            canon_to_title,
        }
    })
}

/// Look up a catalog entry by a **folder-ish** name: case, spacing and
/// punctuation are ignored, so `StardewValley` finds "Stardew Valley".
///
/// Exact on the canonical form, never fuzzy: this names a folder that is
/// about to become a tracked game, and a near-miss would file someone's save
/// under the wrong title.
pub fn find_by_canon_name(name: &str) -> Option<&'static LudusaviEntry> {
    let canon = hoard_core::ids::canon_token(name);
    if canon.len() < hoard_core::ids::MIN_IDENTITY_TOKEN_LEN {
        return None;
    }
    indexes().by_canon.get(&canon).map(|i| &catalog()[*i])
}

/// Display name for a **folder-ish** name, from the catalog or the title-only
/// index. Wider than [`find_by_canon_name`]: it also names games we cannot track.
///
/// This is the one to use for *naming* a folder. `find_by_canon_name` returns a
/// catalog entry, with its paths, and therefore only sees games with a save path; a
/// folder named after a game with no path (a new edition, an online game) was left
/// without a title and ended up in the library under the raw name of the process or
/// the directory.
pub fn title_for_canon_name(name: &str) -> Option<&'static str> {
    let canon = hoard_core::ids::canon_token(name);
    if canon.len() < hoard_core::ids::MIN_IDENTITY_TOKEN_LEN {
        return None;
    }
    indexes().canon_to_title.get(&canon).copied()
}

/// Look up a catalog entry by its Lutris slug (the prefix directory name
/// Lutris creates), so a Lutris prefix resolves to a game exactly.
pub fn find_by_lutris_slug(slug: &str) -> Option<&'static LudusaviEntry> {
    indexes()
        .by_lutris
        .get(slug.to_ascii_lowercase().as_str())
        .map(|i| &catalog()[*i])
}

/// Look up a Ludusavi entry by Steam app id.
pub fn find_by_steam_app_id(app_id: u64) -> Option<&'static LudusaviEntry> {
    indexes().by_app_id.get(&app_id).map(|i| &catalog()[*i])
}

/// Look up a Ludusavi entry by its exact slug.
pub fn find_by_slug(slug: &str) -> Option<&'static LudusaviEntry> {
    indexes().by_slug.get(slug).map(|i| &catalog()[*i])
}

/// The catalog game this executable belongs to, when exactly one claims it.
///
/// `exe` is matched on the basename, case-insensitively (`"EldenRing.exe"`,
/// `"eldenring.exe"` and a full path ending in either all resolve the same).
///
/// Returns `None` for a name shared by several games. 692 names in the
/// manifest are (`game.exe` ×730, `launcher.exe`, `nw.exe`, `dosbox.exe`,
/// `scummvm.exe`, …) and resolving one of those to a guess would attribute a
/// session, its playtime and its save folder to an arbitrary title, so an
/// ambiguous name resolves to nothing at all.
pub fn find_by_exe(exe: &str) -> Option<&'static LudusaviEntry> {
    let leaf = exe_leaf(exe);
    indexes()
        .exe_to_slug
        .get(leaf.as_str())
        .map(|i| &catalog()[*i])
}

/// Display name for an executable, from the catalog or the title-only
/// index. Wider than [`find_by_exe`]: it also names games we can't track.
pub fn title_for_exe(exe: &str) -> Option<&'static str> {
    let leaf = exe_leaf(exe);
    indexes().exe_to_title.get(leaf.as_str()).copied()
}

/// Display name for a Steam appid, from the catalog or the title index.
pub fn title_for_app_id(app_id: u64) -> Option<&'static str> {
    indexes().app_id_to_title.get(&app_id).copied()
}

/// Normalise an executable reference to the lowercase basename the index
/// is keyed on. Accepts a bare name or a full path, either separator.
fn exe_leaf(exe: &str) -> String {
    exe.replace('\\', "/")
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
}

/// Fuzzy lookup by display name using normalised Levenshtein over slugs.
///
/// Used as a last-resort fallback in detection when neither `find_by_steam_app_id`
/// nor an exact slug match against the catalog resolves a Steam app. Slugifies
/// `name` (so casing/punctuation differences don't count as edits), then scans
/// every catalog slug and keeps the entry with the lowest normalised distance,
/// `levenshtein / max(len_a, len_b)`, provided it stays strictly below
/// `threshold`. The recommended default is `0.15` (≈ one edit per 7 characters).
///
/// The threshold alone can't tell sequels apart: "civilization-v" vs
/// "civilization-vi" is ≈ 0.07, comfortably inside 0.15. A numeral veto
/// closes that hole: candidates whose numeric or roman tokens differ from the
/// query's are rejected outright (see [`numeral_signature`]), no matter how
/// small the edit distance.
///
/// Tie-break: when two entries share the lowest distance, the one with a
/// populated `steam_app_id` wins (the appid is a strictly stronger structural
/// signal). The implementation is O(N) per call, but the catalog is fixed at
/// ~20k entries and this only fires once per unmatched Steam app per scan, so
/// it stays well under the cost of the surrounding pipeline.
pub fn find_by_fuzzy_name(name: &str, threshold: f32) -> Option<&'static LudusaviEntry> {
    fuzzy_match_in(catalog(), name, threshold)
}

/// Look up a catalog entry whose slug is a **token prefix** of `name`, longest
/// first, so `"Surviving Mars Relaunched"` resolves to `Surviving Mars`.
///
/// This is the case neither exact nor fuzzy matching can reach. Save folders
/// routinely carry a qualifier the catalog title doesn't have (an edition like
/// `Relaunched` or `Definitive Edition`, a store suffix, a mod-loader tag) and
/// the extra word is far more than the ~1-edit-per-7-chars `find_by_fuzzy_name`
/// tolerates, so today the folder just becomes its own phantom game.
///
/// Two guards keep it honest:
///
/// * **Token boundary.** The query must continue with `-` after the match, so
///   `civilization-v` never claims `civilization-vi-saves`.
/// * **At least two tokens.** A one-word title is too generic to swallow a
///   longer name: `Fallout` must not claim `Fallout New Vegas`. With the
///   longest-match rule, a real two-token prefix still wins over a shorter one.
pub fn find_by_name_prefix(name: &str) -> Option<&'static LudusaviEntry> {
    let query = slugify(name);
    if query.is_empty() {
        return None;
    }
    let mut best: Option<&'static LudusaviEntry> = None;
    for entry in catalog() {
        if entry.slug.len() >= query.len() || !entry.slug.contains('-') {
            continue;
        }
        // `starts_with` FIRST: only then is `entry.slug.len()` known to be a
        // char boundary of `query`, and slicing at it safe.
        if !query.starts_with(&entry.slug) || !query[entry.slug.len()..].starts_with('-') {
            continue;
        }
        if best.is_none_or(|b| entry.slug.len() > b.slug.len()) {
            best = Some(entry);
        }
    }
    best
}

/// Catalog-agnostic core of [`find_by_fuzzy_name`]. Exposed `pub(crate)` so the
/// unit tests can drive it against a fixed in-memory slice instead of the
/// embedded ~20k-entry global catalog, which keeps tie-break behaviour
/// deterministic and the test fast.
pub(crate) fn fuzzy_match_in<'a>(
    catalog: &'a [LudusaviEntry],
    name: &str,
    threshold: f32,
) -> Option<&'a LudusaviEntry> {
    let query = slugify(name);
    if query.is_empty() {
        return None;
    }
    let query_numerals = numeral_signature(&query);
    let mut best: Option<(&'a LudusaviEntry, f32)> = None;
    for entry in catalog {
        let max_len = query.len().max(entry.slug.len());
        if max_len == 0 {
            continue;
        }
        let distance = strsim::levenshtein(&query, &entry.slug) as f32 / max_len as f32;
        if distance >= threshold {
            continue;
        }
        // Sequel veto: "dark-souls-ii" and "dark-souls-iii" are one edit
        // apart, far inside any useful threshold, but never the same game.
        // A numeral mismatch disqualifies the candidate outright.
        if numeral_signature(&entry.slug) != query_numerals {
            continue;
        }
        match best {
            None => best = Some((entry, distance)),
            Some((current, current_dist)) => {
                if distance < current_dist {
                    best = Some((entry, distance));
                } else if (distance - current_dist).abs() < f32::EPSILON
                    && entry.steam_app_id.is_some()
                    && current.steam_app_id.is_none()
                {
                    // Tie: prefer the entry carrying a Steam appid.
                    best = Some((entry, distance));
                }
            }
        }
    }
    best.map(|(entry, _)| entry)
}

/// Ordered sequence of numeric tokens in a slug, arabic and roman unified:
/// `"final-fantasy-x-2"` gives `[10, 2]`, and `"hitman-2"` == `"hitman-ii"` gives
/// `[2]`. Roman numerals come from a fixed i to xx table: game sequels don't go
/// higher, and a full parser would happily read words like "mix" as numbers.
/// Single-letter tokens ("i", "v", "x") can also be genuine words; the veto
/// only fires when the *signatures* differ, and a same-game pair almost
/// always carries the same token on both sides, so the false-veto risk stays
/// negligible next to the cross-sequel mismatch it prevents.
fn numeral_signature(slug: &str) -> Vec<u64> {
    const ROMAN: &[(&str, u64)] = &[
        ("i", 1),
        ("ii", 2),
        ("iii", 3),
        ("iv", 4),
        ("v", 5),
        ("vi", 6),
        ("vii", 7),
        ("viii", 8),
        ("ix", 9),
        ("x", 10),
        ("xi", 11),
        ("xii", 12),
        ("xiii", 13),
        ("xiv", 14),
        ("xv", 15),
        ("xvi", 16),
        ("xvii", 17),
        ("xviii", 18),
        ("xix", 19),
        ("xx", 20),
    ];
    slug.split('-')
        .filter_map(|tok| {
            if !tok.is_empty() && tok.chars().all(|c| c.is_ascii_digit()) {
                tok.parse().ok()
            } else {
                ROMAN.iter().find(|(r, _)| *r == tok).map(|(_, n)| *n)
            }
        })
        .collect()
}

// ----- YAML → catalog conversion ----------------------------------------

/// Subset of the Ludusavi YAML schema we care about. Mirrors the
/// `hoard-admin manifest import` parser so behaviour stays in sync.
#[derive(Debug, Default, Deserialize)]
struct YamlEntry {
    #[serde(default)]
    alias: Option<String>,
    #[serde(default)]
    files: BTreeMap<String, YamlPath>,
    #[serde(default)]
    steam: Option<YamlSteamRef>,
    /// Ludusavi `registry:` block. Keys are full registry paths
    /// (`HKEY_CURRENT_USER/Software/...`); we ignore the per-entry
    /// `tags`/`when` payload here and just keep the key.
    #[serde(default)]
    registry: BTreeMap<String, serde::de::IgnoredAny>,
    /// `installDir:`, a map whose *keys* are the install folder names.
    #[serde(default, rename = "installDir")]
    install_dir: BTreeMap<String, serde::de::IgnoredAny>,
    /// `launch:`, a map whose *keys* are executable paths, usually
    /// `<base>`-relative. Only the basename is useful to us.
    #[serde(default)]
    launch: BTreeMap<String, serde::de::IgnoredAny>,
    #[serde(default)]
    cloud: Option<YamlCloud>,
    #[serde(default)]
    id: Option<YamlIds>,
}

/// `cloud:`, which storefronts' cloud sync the game supports. Only Steam
/// is actionable for us (it's the one that overlaps with what Hoard does).
#[derive(Debug, Default, Deserialize)]
struct YamlCloud {
    #[serde(default)]
    steam: bool,
}

/// `id:`, extra store identifiers beyond the primary `steam.id`.
#[derive(Debug, Default, Deserialize)]
struct YamlIds {
    #[serde(default, rename = "steamExtra")]
    steam_extra: Vec<u64>,
    #[serde(default)]
    lutris: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct YamlPath {
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    when: Vec<YamlWhen>,
}

#[derive(Debug, Default, Deserialize)]
struct YamlWhen {
    #[serde(default)]
    os: Option<String>,
    #[serde(default)]
    store: Option<String>,
}

#[derive(Debug, Deserialize)]
struct YamlSteamRef {
    id: u64,
}

/// Parse Ludusavi YAML and emit our compact [`LudusaviEntry`] catalog.
///
/// This is the in-Rust port of `data/convert-ludusavi.py` so the desktop
/// can refresh the catalog at runtime without shelling out to Python.
/// Behaviour matches the Python script byte-for-byte: same skip rules
/// (alias, no-paths, non-save tags), same OS expansion of empty
/// `when:` clauses, same slug rules.
pub fn convert_yaml_to_catalog(yaml_text: &str) -> Result<Vec<LudusaviEntry>, CatalogError> {
    Ok(convert_yaml(yaml_text)?.0)
}

/// Full conversion: the save-path catalog **and** the title-only index.
///
/// The manifest describes ~53k games but only ~21k of them have a save path
/// we could ever back up; the rest are entries that carry nothing but a
/// title, a Steam appid and a `launch:` block. Those are useless for
/// detecting *saves*, which is why the catalog drops them, but they are
/// exactly what's needed to put a **name** on a running process or an appid
/// (see [`TitleEntry`]). Keeping them out of the catalog proper matters:
/// every detection pass iterates the catalog, so tripling its length to
/// carry names would slow down the scan for no detection benefit.
pub fn convert_yaml(
    yaml_text: &str,
) -> Result<(Vec<LudusaviEntry>, Vec<TitleEntry>), CatalogError> {
    let parsed: BTreeMap<String, YamlEntry> = serde_yaml::from_str(yaml_text)?;
    let mut out = Vec::with_capacity(parsed.len());
    let mut titles = Vec::new();
    let mut seen_slugs: HashSet<String> = HashSet::with_capacity(parsed.len());

    for (display_name, entry) in parsed {
        if entry.alias.is_some() {
            continue;
        }
        let paths = transform_files(&entry.files);
        let registry = transform_registry(&entry.registry);
        let launch_exes = launch_basenames(&entry.launch);
        let steam_app_id = entry.steam.as_ref().map(|s| s.id);
        if paths.windows.is_empty()
            && paths.linux.is_empty()
            && paths.mac.is_empty()
            && registry.is_empty()
        {
            // No save path anywhere: keep the name only, and only when it
            // can actually be looked up by something.
            if steam_app_id.is_some() || !launch_exes.is_empty() {
                titles.push(TitleEntry {
                    display_name,
                    steam_app_id,
                    launch_exes,
                });
            }
            continue;
        }

        let mut slug = slugify(&display_name);
        if !seen_slugs.insert(slug.clone()) {
            // Disambiguate the rare collision so the catalog stays a
            // proper map keyed on slug.
            let mut i = 2u32;
            loop {
                let candidate = format!("{slug}-{i}");
                if seen_slugs.insert(candidate.clone()) {
                    slug = candidate;
                    break;
                }
                i += 1;
            }
        }

        out.push(LudusaviEntry {
            slug,
            display_name,
            steam_app_id,
            paths,
            registry,
            install_dirs: entry.install_dir.keys().cloned().collect(),
            launch_exes,
            steam_extra_ids: entry
                .id
                .as_ref()
                .map(|i| i.steam_extra.clone())
                .unwrap_or_default(),
            lutris_slug: entry.id.as_ref().and_then(|i| i.lutris.clone()),
            cloud_steam: entry.cloud.as_ref().is_some_and(|c| c.steam),
        });
    }

    out.sort_by(|a, b| a.slug.cmp(&b.slug));
    titles.sort_by(|a: &TitleEntry, b: &TitleEntry| a.display_name.cmp(&b.display_name));
    Ok((out, titles))
}

/// Executable basenames from a `launch:` block, lowercased and deduped.
///
/// Keys look like `<base>/Binaries/Win64/Game.exe` or `<base>/run.sh`; only
/// the leaf identifies the process. Anything that isn't plausibly an
/// executable leaf is dropped: a launch key can carry a `<base>`-only entry
/// or a directory, and those would match every process in that folder.
fn launch_basenames(launch: &BTreeMap<String, serde::de::IgnoredAny>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for key in launch.keys() {
        let leaf = key
            .replace('\\', "/")
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        // A leftover placeholder or an empty leaf is not an executable.
        if leaf.is_empty() || leaf.contains('<') || leaf.len() < 3 {
            continue;
        }
        if !out.contains(&leaf) {
            out.push(leaf);
        }
    }
    out
}

/// Download upstream Ludusavi YAML, convert it, and persist as the runtime
/// override at [`runtime_override_path`]. Returns the count of games in
/// the new catalog so callers can show "Updated to N games" toasts.
///
/// `yaml_text` should be the full upstream manifest body; the desktop
/// command fetches it via `reqwest` so this crate doesn't grow an HTTP
/// dependency.
pub fn save_runtime_override(yaml_text: &str) -> Result<usize, CatalogError> {
    let (catalog, titles) = convert_yaml(yaml_text)?;
    let path = runtime_override_path().ok_or(CatalogError::NoCacheDir)?;
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| CatalogError::Io {
                path: parent.display().to_string(),
                source: e,
            })?;
        }
    }
    write_atomic(&path, &serde_json::to_vec(&catalog)?)?;
    // Best-effort: a failure here costs nicer names on unknown processes,
    // never detection itself, so it must not fail the catalog update.
    if let Some(tp) = titles_override_path() {
        if let Err(e) = write_atomic(&tp, &serde_json::to_vec(&titles)?) {
            tracing::warn!(error = %e, "couldn't write the title index override");
        }
    }
    tracing::info!(
        path = %path.display(),
        count = catalog.len(),
        titles = titles.len(),
        "wrote Ludusavi catalog runtime override"
    );
    Ok(catalog.len())
}

/// Write `bytes` to `path` via a same-directory temp file + rename so a
/// crash mid-write can't corrupt the catalog.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), CatalogError> {
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, bytes).map_err(|e| CatalogError::Io {
        path: tmp.display().to_string(),
        source: e,
    })?;
    std::fs::rename(&tmp, path).map_err(|e| CatalogError::Io {
        path: path.display().to_string(),
        source: e,
    })
}

/// Convert a Ludusavi `files:` block into per-OS `LudusaviSavePath`s.
fn transform_files(files: &BTreeMap<String, YamlPath>) -> LudusaviPaths {
    let mut out = LudusaviPaths::default();

    for (path_template, p) in files {
        // Keep only save paths. Many entries have no tags at all -- we
        // keep those too (Ludusavi convention is "if no tags, it's a save").
        if !p.tags.is_empty() && !p.tags.iter().any(|t| t == "save") {
            continue;
        }

        let oses: Vec<&'static str> = if p.when.is_empty() {
            vec!["windows", "linux", "mac"]
        } else {
            let mut v: Vec<&'static str> = p
                .when
                .iter()
                .filter_map(|w| w.os.as_deref().and_then(normalise_os))
                .collect();
            v.sort();
            v.dedup();
            if v.is_empty() {
                v = vec!["windows", "linux", "mac"];
            }
            v
        };

        let constraints: Vec<LudusaviConstraint> = if p.when.is_empty() {
            vec![LudusaviConstraint { store: None }]
        } else {
            p.when
                .iter()
                .map(|w| LudusaviConstraint {
                    store: w.store.clone(),
                })
                .collect()
        };

        let tags = if p.tags.is_empty() {
            vec!["save".into()]
        } else {
            p.tags.clone()
        };

        let entry = LudusaviSavePath {
            path: path_template.clone(),
            constraints,
            tags,
        };

        for os in oses {
            match os {
                "windows" => out.windows.push(entry.clone()),
                "linux" => out.linux.push(entry.clone()),
                "mac" => out.mac.push(entry.clone()),
                _ => {}
            }
        }
    }

    out
}

/// Convert a Ludusavi `registry:` block into a flat `Vec<RegistryPath>`.
///
/// Each map key becomes the `RegistryPath::key`. We never populate
/// `value` here: Ludusavi's upstream YAML doesn't carry per-value
/// metadata, so callers default to reading the subkey's default value
/// (handled by `pathexpand::expand_registry_path` on Windows). Order is
/// stable thanks to `BTreeMap`.
fn transform_registry(registry: &BTreeMap<String, serde::de::IgnoredAny>) -> Vec<RegistryPath> {
    registry
        .keys()
        .map(|k| RegistryPath {
            key: k.clone(),
            value: None,
        })
        .collect()
}

fn normalise_os(s: &str) -> Option<&'static str> {
    match s.to_ascii_lowercase().as_str() {
        "windows" | "win" => Some("windows"),
        "linux" => Some("linux"),
        "mac" | "macos" | "osx" | "darwin" => Some("mac"),
        _ => None,
    }
}

/// Lower-kebab slug. Mirrors `slugify` in `data/convert-ludusavi.py` and
/// in `hoard-admin::commands::manifest::slugify` so all three stay
/// byte-compatible, so the same input always produces the same slug.
///
/// Public so detection code can slugify a Steam app's display name and
/// fall back to a slug-based catalog lookup when the catalog entry lacks
/// `steam_app_id`. **Never duplicate this algorithm**: divergence here
/// silently breaks the cross-reference.
///
/// The implementation moved to `hoard_core::ids::slugify` with the newtype
/// gate (ADR 0021 C.3): `GameSlug::repair` re-derives a poisoned slug with
/// exactly this function, so the two must be the same code, not two copies
/// that agree today. This stays as the crate's public name.
pub use hoard_core::ids::slugify;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_parses_and_is_nonempty() {
        let cat = catalog();
        assert!(
            cat.len() > 1000,
            "expected the bundled Ludusavi catalog to have thousands of \
             entries; got {}",
            cat.len()
        );
    }

    #[test]
    fn catalog_entries_have_at_least_one_path() {
        // Mirror of the converter's skip rule: an entry earns its slot with
        // a save path on some OS **or** a registry key (registry-only
        // entries feed detection's registry-expand stage on Windows).
        for entry in catalog() {
            assert!(
                !entry.paths.windows.is_empty()
                    || !entry.paths.linux.is_empty()
                    || !entry.paths.mac.is_empty()
                    || !entry.registry.is_empty(),
                "{} has no paths on any OS and no registry keys",
                entry.slug
            );
        }
    }

    #[test]
    fn catalog_slugs_are_unique() {
        let cat = catalog();
        let mut seen = std::collections::HashSet::with_capacity(cat.len());
        for e in cat {
            assert!(
                seen.insert(&e.slug),
                "duplicate slug in catalog: {}",
                e.slug
            );
        }
    }

    #[test]
    fn slugify_examples() {
        assert_eq!(slugify("Stardew Valley"), "stardew-valley");
        assert_eq!(
            slugify("The Witcher 3: Wild Hunt"),
            "the-witcher-3-wild-hunt"
        );
        assert_eq!(slugify("---"), "game");
        assert_eq!(slugify(""), "game");
        // Leading non-alnum: prefixed with `g`.
        assert_eq!(slugify("123"), "123");
    }

    #[test]
    fn convert_minimal_yaml_smoke() {
        let yaml = "Stardew Valley:\n  files:\n    \"<winAppData>/StardewValley/Saves\":\n      tags: [save]\n  steam:\n    id: 413150\n";
        let cat = convert_yaml_to_catalog(yaml).unwrap();
        assert_eq!(cat.len(), 1);
        assert_eq!(cat[0].slug, "stardew-valley");
        assert_eq!(cat[0].steam_app_id, Some(413150));
        // No `when:` → applies to every OS.
        assert_eq!(cat[0].paths.windows.len(), 1);
        assert_eq!(cat[0].paths.linux.len(), 1);
        assert_eq!(cat[0].paths.mac.len(), 1);
    }

    #[test]
    fn convert_skips_alias_and_no_paths() {
        let yaml = "\
Real Game:
  files:
    \"<winAppData>/X\":
      tags: [save]
Aliased:
  alias: Real Game
Empty Game:
  files: {}
";
        let cat = convert_yaml_to_catalog(yaml).unwrap();
        assert_eq!(cat.len(), 1);
        assert_eq!(cat[0].slug, "real-game");
    }

    #[test]
    fn convert_drops_non_save_tags() {
        let yaml = "\
Game X:
  files:
    \"<winAppData>/X\":
      tags: [config]
";
        let cat = convert_yaml_to_catalog(yaml).unwrap();
        // No paths kept → entry skipped.
        assert!(cat.is_empty());
    }

    /// Builds a minimal [`LudusaviEntry`] for fuzzy-match tests so the
    /// behaviour is deterministic (no dependency on the embedded ~20k catalog).
    fn entry(slug: &str, steam_app_id: Option<u64>) -> LudusaviEntry {
        LudusaviEntry {
            slug: slug.to_string(),
            display_name: slug.to_string(),
            steam_app_id,
            paths: LudusaviPaths::default(),
            registry: Vec::new(),
            install_dirs: Vec::new(),
            launch_exes: Vec::new(),
            steam_extra_ids: Vec::new(),
            lutris_slug: None,
            cloud_steam: false,
        }
    }

    #[test]
    fn fuzzy_matches_minor_typo() {
        let cat = vec![
            entry("stardew-valley", Some(413150)),
            entry("celeste", Some(504230)),
        ];
        let hit = fuzzy_match_in(&cat, "stardew vally", 0.15).expect("fuzzy hit");
        assert_eq!(hit.slug, "stardew-valley");
    }

    #[test]
    fn fuzzy_rejects_distant_names() {
        let cat = vec![
            entry("civilization-v", Some(8930)),
            entry("destiny-2", Some(1085660)),
        ];
        // "civilization" sits within edit distance of "civilization-v"
        // (0.14 < 0.15) but carries no numeral, so the sequel veto rejects it
        // instead of guessing which entry in the series was meant.
        assert!(fuzzy_match_in(&cat, "civilization", 0.15).is_none());
        // And "destiny" must not match the unrelated "civilization-v".
        let hit = fuzzy_match_in(&cat, "destiny", 0.15);
        assert!(
            hit.is_none_or(|e| e.slug != "civilization-v"),
            "destiny should not collapse onto civilization-v: got {:?}",
            hit.map(|e| &e.slug)
        );
    }

    #[test]
    fn fuzzy_prefers_steam_id() {
        // Two slugs at identical distance from the query; only one has a
        // steam_app_id, and that one must win. Iteration order: the no-id entry
        // comes first so the tie-break has to actively replace it.
        let cat = vec![entry("aaa", None), entry("aab", Some(42))];
        // "aac" is exactly 1 edit from each slug ⇒ same normalised distance.
        let hit = fuzzy_match_in(&cat, "aac", 0.5).expect("fuzzy hit");
        assert_eq!(hit.slug, "aab");
        assert_eq!(hit.steam_app_id, Some(42));
    }

    #[test]
    fn fuzzy_returns_none_below_threshold() {
        let cat = vec![entry("portal-2", Some(620))];
        // "minecraft" vs "portal-2": normalised distance ≈ 1.0, far above 0.15.
        assert!(fuzzy_match_in(&cat, "minecraft", 0.15).is_none());
    }

    #[test]
    fn fuzzy_vetoes_sequel_numeral_mismatch() {
        // "dark-souls-ii" vs "dark-souls-iii" is one edit over 14 chars
        // (about 0.071 < 0.15): inside the threshold, but a different game.
        let cat = vec![entry("dark-souls-iii", Some(374320))];
        assert!(fuzzy_match_in(&cat, "Dark Souls II", 0.15).is_none());
        // Extra numeral token: "final-fantasy-x-2" vs "final-fantasy-x"
        // (≈ 0.118 < 0.15) must also be vetoed.
        let cat = vec![entry("final-fantasy-x", Some(359870))];
        assert!(fuzzy_match_in(&cat, "Final Fantasy X-2", 0.15).is_none());
    }

    #[test]
    fn fuzzy_accepts_equivalent_numeral_spellings() {
        // Arabic vs roman spellings of the same number are the same
        // signature, so the veto must not fire and only the distance decides.
        let cat = vec![entry("hitman-2", Some(863550))];
        let hit = fuzzy_match_in(&cat, "Hitman II", 0.5).expect("fuzzy hit");
        assert_eq!(hit.slug, "hitman-2");
    }

    #[test]
    fn parses_registry_field_from_entry() {
        // Deserialise a synthetic catalog entry whose JSON includes the
        // new `registry` field. Verifies the schema accepts the field
        // and turns it into a `Vec<RegistryPath>` with a single key and
        // `value: None` (the Ludusavi default).
        let json = r#"{
            "slug": "skyrim",
            "display_name": "Skyrim",
            "paths": { "windows": [], "linux": [], "mac": [] },
            "registry": [
                { "key": "HKEY_CURRENT_USER/Software/Bethesda Softworks/Skyrim" }
            ]
        }"#;
        let entry: LudusaviEntry = serde_json::from_str(json).expect("entry parses");
        assert_eq!(entry.registry.len(), 1);
        assert_eq!(
            entry.registry[0].key,
            "HKEY_CURRENT_USER/Software/Bethesda Softworks/Skyrim"
        );
        assert!(entry.registry[0].value.is_none());
    }

    #[test]
    fn entry_without_registry_defaults_to_empty() {
        // Older catalog snapshots omit the field entirely; the `#[serde(default)]`
        // attribute must give us an empty vec instead of a parse error.
        let json = r#"{
            "slug": "stardew-valley",
            "display_name": "Stardew Valley",
            "paths": { "windows": [], "linux": [], "mac": [] }
        }"#;
        let entry: LudusaviEntry = serde_json::from_str(json).expect("entry parses");
        assert!(entry.registry.is_empty());
    }

    #[test]
    fn convert_yaml_extracts_registry_keys() {
        // The upstream YAML's `registry:` block must surface as
        // `RegistryPath`s on the resulting catalog entry, even when the
        // entry also has `files:`.
        let yaml = "\
Skyrim:
  files:
    \"<winDocuments>/My Games/Skyrim\":
      tags: [save]
  registry:
    \"HKEY_CURRENT_USER/Software/Bethesda Softworks/Skyrim\":
      tags: [save]
      when:
        - os: windows
";
        let cat = convert_yaml_to_catalog(yaml).unwrap();
        assert_eq!(cat.len(), 1);
        assert_eq!(cat[0].registry.len(), 1);
        assert_eq!(
            cat[0].registry[0].key,
            "HKEY_CURRENT_USER/Software/Bethesda Softworks/Skyrim"
        );
        assert!(cat[0].registry[0].value.is_none());
    }

    #[test]
    fn convert_yaml_keeps_registry_only_entries() {
        // A game with registry-only saves (no `files:`) must not be
        // discarded as "no paths on any OS"; the registry block is a
        // valid path source.
        let yaml = "\
Registry Only Game:
  registry:
    \"HKEY_CURRENT_USER/Software/Acme/RegOnly\":
      tags: [save]
";
        let cat = convert_yaml_to_catalog(yaml).unwrap();
        assert_eq!(cat.len(), 1);
        assert_eq!(cat[0].slug, "registry-only-game");
        assert_eq!(cat[0].registry.len(), 1);
    }

    #[test]
    fn convert_yaml_keeps_launch_installdir_flatpak_and_cloud() {
        let yaml = "\
Elden Ring:
  files:
    \"<winAppData>/EldenRing\":
      tags: [save]
  installDir:
    ELDEN RING: {}
  launch:
    \"<base>/Game/eldenring.exe\": []
    \"<base>/start_protected_game.exe\": []
  cloud:
    steam: true
  id:
    steamExtra: [1245621, 1245622]
    lutris: elden-ring
  steam:
    id: 1245620
";
        let cat = convert_yaml_to_catalog(yaml).unwrap();
        assert_eq!(cat.len(), 1);
        let e = &cat[0];
        assert_eq!(e.install_dirs, ["ELDEN RING"]);
        // Basenames only, lowercased, in key order.
        assert_eq!(e.launch_exes, ["eldenring.exe", "start_protected_game.exe"]);
        assert_eq!(e.steam_extra_ids, [1245621, 1245622]);
        assert_eq!(e.lutris_slug.as_deref(), Some("elden-ring"));
        assert!(e.cloud_steam);
    }

    #[test]
    fn a_game_without_save_paths_becomes_a_title_not_a_catalog_entry() {
        // Two thirds of the manifest looks like this. It must name a
        // process without ever being offered as something to back up.
        let yaml = "\
War Selection:
  launch:
    \"<base>/WarSelection.exe\": []
  steam:
    id: 1013740
Nameless:
  installDir:
    Nameless: {}
";
        let (cat, titles) = convert_yaml(yaml).unwrap();
        assert!(cat.is_empty(), "no save path ⇒ nothing to track");
        // "Nameless" has neither appid nor launch: nothing to look it up by.
        assert_eq!(titles.len(), 1);
        assert_eq!(titles[0].display_name, "War Selection");
        assert_eq!(titles[0].launch_exes, ["warselection.exe"]);
        assert_eq!(titles[0].steam_app_id, Some(1013740));
    }

    #[test]
    fn launch_basenames_drops_placeholders_and_stubs() {
        let mut launch: BTreeMap<String, serde::de::IgnoredAny> = BTreeMap::new();
        for k in ["<base>", "<base>/", "<base>/a", "<base>/dir/Game.EXE"] {
            launch.insert(k.to_string(), serde_yaml::from_str("~").unwrap());
        }
        // Only the real executable survives: a bare `<base>`, a trailing
        // slash and a 1-char leaf would each match far too much.
        assert_eq!(launch_basenames(&launch), ["game.exe"]);
    }

    #[test]
    fn exe_index_resolves_a_unique_name_and_vetoes_a_shared_one() {
        // Against the real embedded catalog: an executable that belongs to
        // exactly one game resolves, and one shared by hundreds must not.
        assert_eq!(
            find_by_exe("factorio.exe").map(|e| e.slug.as_str()),
            Some("factorio"),
            "a unique executable should resolve to its game"
        );
        // Path form and casing are normalised to the same key.
        assert_eq!(
            find_by_exe("/home/u/.factorio/bin/x64/Factorio.exe").map(|e| e.slug.as_str()),
            Some("factorio"),
        );
        // `game.exe` is claimed by ~730 games; resolving it to any one of
        // them would hand a save folder (and its playtime) to a coin flip.
        assert!(
            find_by_exe("game.exe").is_none(),
            "ambiguous exe must be vetoed"
        );
        assert!(title_for_exe("game.exe").is_none());
        for shared in ["launcher.exe", "nw.exe", "dosbox.exe", "scummvm.exe"] {
            assert!(
                find_by_exe(shared).is_none(),
                "{shared} should be ambiguous"
            );
        }
    }

    #[test]
    fn titles_name_games_that_have_no_save_path() {
        // The whole point of the title index: a game with no trackable save
        // still gets a name, and never shows up as something to back up.
        assert!(!titles().is_empty());
        assert!(
            title_for_app_id(1245620).is_some(),
            "appid should name a game"
        );
    }

    #[test]
    fn steam_extra_ids_resolve_to_the_same_game() {
        // A regional/demo appid must find the game rather than nothing. Pick
        // a real one from the catalog so the test tracks upstream data.
        let with_extra = catalog()
            .iter()
            .find(|e| !e.steam_extra_ids.is_empty())
            .expect("catalog should carry steamExtra ids");
        let extra = with_extra.steam_extra_ids[0];
        assert!(
            find_by_steam_app_id(extra).is_some(),
            "secondary appid {extra} should resolve"
        );
    }

    /// Regenerates the embedded catalog + title index from a manifest YAML.
    /// Not a test but a generator, skipped unless asked:
    ///
    /// ```sh
    /// GEN_CATALOG=/path/to/manifest.yaml cargo test -p hoard-manifest -- --ignored regenerate
    /// ```
    ///
    /// Using the real converter (instead of the Python script) is the point:
    /// what ships can't drift from what a runtime refresh would produce.
    #[test]
    #[ignore = "generator; set GEN_CATALOG=<manifest.yaml>"]
    fn regenerate_embedded_catalog() {
        let Ok(src) = std::env::var("GEN_CATALOG") else {
            panic!("set GEN_CATALOG=<path to manifest.yaml>");
        };
        let yaml = std::fs::read_to_string(&src).expect("reading manifest");
        let (cat, titles) = convert_yaml(&yaml).expect("converting manifest");
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
        for (name, bytes) in [
            (
                "ludusavi-catalog.json.zst",
                serde_json::to_vec(&cat).unwrap(),
            ),
            (
                "ludusavi-titles.json.zst",
                serde_json::to_vec(&titles).unwrap(),
            ),
        ] {
            let raw = bytes.len();
            let packed = zstd::encode_all(bytes.as_slice(), 12).unwrap();
            println!("{name}: {raw} → {} bytes", packed.len());
            std::fs::write(dir.join(name), packed).unwrap();
        }
        println!("catalog: {} entries, titles: {}", cat.len(), titles.len());
    }
}
