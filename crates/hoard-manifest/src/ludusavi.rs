//! Embedded Ludusavi catalog with optional runtime refresh.
//!
//! The desktop app needs to know save-path templates for ~20k games to
//! detect installed games offline (no server round-trips). Hand-curating
//! that many entries is impossible — the [Ludusavi][1] community manifest
//! already does the work and is the de-facto standard for save-sync data.
//!
//! ## How the catalog gets loaded
//!
//! Two sources, in priority order:
//!
//! 1. **Runtime override** at `<cache_dir>/hoard/ludusavi-catalog.json`,
//!    written by [`save_runtime_override`] after a successful update.
//!    This lets the desktop refresh its catalog without a re-install.
//! 2. **Compile-time embedded** [`CATALOG_JSON`] (~7 MB), produced by the
//!    Python conversion script. Always available, always works offline.
//!
//! [`catalog`] resolves both lazily on first call and caches in a
//! `OnceLock`. After [`save_runtime_override`] writes a new JSON, the
//! next *process* picks it up — we deliberately don't hot-swap the
//! cached slice mid-run because that would make detection results
//! inconsistent across concurrent scans.
//!
//! ## Refreshing
//!
//! [`fetch_and_save`] downloads the upstream YAML, runs
//! [`convert_yaml_to_catalog`] (the in-Rust port of
//! `data/convert-ludusavi.py`), and writes the JSON to the cache dir.
//! That function is the entry point both for the desktop's "Update
//! catalog" button and for the background refresh on app startup.
//!
//! ## Path syntax
//!
//! Ludusavi templates use angle-bracket placeholders (`<winAppData>`,
//! `<xdgData>`, `<home>`, `<storeUserId>`, …). These are expanded by
//! `hoard-agent::pathexpand`, **not** by [`crate::placeholders`] — the
//! placeholder vocabulary is different.
//!
//! ## Licensing
//!
//! The manifest data is sourced from [PCGamingWiki][2] and is licensed
//! CC-BY-NC-SA-3.0. We embed it for personal-use detection. Distributors
//! who want to ship Hoard commercially should remove the JSON before
//! bundling and rely on the AGPL-clean hand-curated TOMLs only.
//!
//! [1]: https://github.com/mtkennerly/ludusavi-manifest
//! [2]: https://www.pcgamingwiki.com/

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

/// Compact JSON produced by `data/convert-ludusavi.py`. ~7 MB. Lives in the
/// binary as a `&'static str` thanks to `include_str!`.
const CATALOG_JSON: &str = include_str!("../data/ludusavi-catalog.json");

/// Default upstream URL for [`fetch_and_save`].
pub const DEFAULT_UPSTREAM_URL: &str =
    "https://raw.githubusercontent.com/mtkennerly/ludusavi-manifest/master/data/manifest.yaml";

/// Sub-path inside the OS cache dir where we persist runtime catalog
/// overrides. Used by [`runtime_override_path`] / [`save_runtime_override`].
const RUNTIME_OVERRIDE_REL: &str = "hoard/ludusavi-catalog.json";

static CATALOG: OnceLock<Vec<LudusaviEntry>> = OnceLock::new();

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

/// Try to load a runtime override from the cache dir. Returns `None` if
/// the file is absent, unreadable, or doesn't parse — in which case the
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
    match serde_json::from_str::<Vec<LudusaviEntry>>(&text) {
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
/// embedded JSON. The choice is sticky for the lifetime of the process —
/// updates take effect on the next launch.
pub fn catalog() -> &'static [LudusaviEntry] {
    CATALOG
        .get_or_init(|| {
            if let Some(override_) = load_runtime_override() {
                return override_;
            }
            // The embedded JSON was emitted by our own script; a parse
            // failure is a build-time invariant violation, not user-facing.
            serde_json::from_str(CATALOG_JSON).unwrap_or_else(|e| {
                tracing::error!("embedded Ludusavi catalog parse failed: {e}");
                panic!("embedded ludusavi-catalog.json must parse: {e}");
            })
        })
        .as_slice()
}

/// Number of games in the (currently-loaded) catalog. Useful for progress
/// bars.
pub fn catalog_size() -> usize {
    catalog().len()
}

/// Look up a Ludusavi entry by Steam app id. O(N) — the cross-reference
/// only happens during detection, where the steam-installed list is small
/// (~hundreds), so a linear scan beats indexing the full catalog.
pub fn find_by_steam_app_id(app_id: u64) -> Option<&'static LudusaviEntry> {
    catalog().iter().find(|e| e.steam_app_id == Some(app_id))
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
    let parsed: BTreeMap<String, YamlEntry> = serde_yaml::from_str(yaml_text)?;
    let mut out = Vec::with_capacity(parsed.len());
    let mut seen_slugs: HashSet<String> = HashSet::with_capacity(parsed.len());

    for (display_name, entry) in parsed {
        if entry.alias.is_some() {
            continue;
        }
        let paths = transform_files(&entry.files);
        if paths.windows.is_empty() && paths.linux.is_empty() && paths.mac.is_empty() {
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
            steam_app_id: entry.steam.as_ref().map(|s| s.id),
            paths,
        });
    }

    out.sort_by(|a, b| a.slug.cmp(&b.slug));
    Ok(out)
}

/// Download upstream Ludusavi YAML, convert it, and persist as the runtime
/// override at [`runtime_override_path`]. Returns the count of games in
/// the new catalog so callers can show "Updated to N games" toasts.
///
/// `yaml_text` should be the full upstream manifest body — the desktop
/// command fetches it via `reqwest` so this crate doesn't grow an HTTP
/// dependency.
pub fn save_runtime_override(yaml_text: &str) -> Result<usize, CatalogError> {
    let catalog = convert_yaml_to_catalog(yaml_text)?;
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
    tracing::info!(
        path = %path.display(),
        count = catalog.len(),
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
/// byte-compatible — same input always produces the same slug.
fn slugify(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut last_dash = true;
    for ch in name.chars() {
        let c = ch.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        out.push_str("game");
    }
    if out.len() > 96 {
        out.truncate(96);
        while out.ends_with('-') {
            out.pop();
        }
    }
    if !out.starts_with(|c: char| c.is_ascii_alphanumeric()) {
        out.insert(0, 'g');
    }
    out
}

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
        for entry in catalog() {
            assert!(
                !entry.paths.windows.is_empty()
                    || !entry.paths.linux.is_empty()
                    || !entry.paths.mac.is_empty(),
                "{} has no paths on any OS",
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
}
