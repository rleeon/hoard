use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use time::OffsetDateTime;

/// Per-save local metadata: which directory on disk maps to which remote save.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveState {
    pub local_path: PathBuf,
    pub game_slug: String,
    pub label: String,
    #[serde(with = "time::serde::rfc3339::option", default)]
    pub last_backup_at: Option<OffsetDateTime>,
    pub last_version_num: Option<i64>,
    /// User-toggled pause. When true the agent skips this save (no process
    /// matching, no FS watch) but the row stays in `state.json` so flipping
    /// it back on doesn't lose the path mapping. `default` lets us read
    /// older state files without migration.
    #[serde(default)]
    pub paused: bool,
    /// Sync preset id for this save (see [`crate::presets`]). Resolves into a
    /// [`crate::presets::SavePolicy`] of overrides layered on the global
    /// config. `None`/absent = the implicit `standard` preset (inherit
    /// everything). Auto-assigned from [`crate::presets::builtin_preset_for`]
    /// on track for known-quirky games; user-overridable. `default` keeps
    /// older `state.json` files loading without migration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
    /// Skip-by-set-hash cache (ADR 0019). A cheap signature over the save's
    /// `(relative_path, size, mtime)` set as of the last successful upload.
    /// Before backing up, the agent recomputes the signature; if it's
    /// unchanged the watcher fired on a settle that touched nothing, so the
    /// upload is a no-op and skipped. `default` keeps older state files
    /// loading without migration.
    #[serde(default)]
    pub set_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CliState {
    /// keyed by save_id (UUID string)
    #[serde(default)]
    pub saves: HashMap<String, SaveState>,
    /// User-supplied save-path overrides, keyed by game slug. When detection
    /// runs, any entry here wins over every heuristic (filesystem, Steam,
    /// Proton prefix, refinement). The detection pipeline tags the resulting
    /// row with `DetectionSource::ManualOverride` so the UI can show "manual"
    /// in the source badge. Set via [`Self::set_manual_path`], cleared via
    /// [`Self::clear_manual_path`]. `default` lets older `state.json` files
    /// load without migration.
    #[serde(default)]
    pub manual_paths: HashMap<String, PathBuf>,
    /// Slugs the user has explicitly blacklisted from the Library page. The
    /// detection pipeline runs to completion as usual; the filter happens at
    /// the edge of `list_detected_games` so the walker still benefits from
    /// install dirs we'd otherwise miss. Reactivatable from
    /// Settings → "Juegos ignorados". `default` keeps older `state.json`
    /// files loading without migration.
    #[serde(default)]
    pub ignored_slugs: HashSet<String>,
}

impl CliState {
    pub fn default_path() -> Result<PathBuf> {
        let dir = crate::config::CliConfig::state_dir()?;
        Ok(dir.join("state.json"))
    }

    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        match serde_json::from_str::<CliState>(&text) {
            Ok(st) => Ok(st),
            Err(e) => {
                // A corrupt state.json (half-written on a crash, disk gremlin,
                // a hand-edit gone wrong) used to abort startup with a bare
                // "parsing <path>" error and brick the app. But this file is a
                // rebuildable cache — every tracked save re-adopts on the next
                // detection pass — so we self-heal instead: move the bad file
                // aside for forensics and start from a clean default.
                let backup = path.with_extension(format!(
                    "json.corrupt-{}",
                    OffsetDateTime::now_utc().unix_timestamp()
                ));
                match std::fs::rename(path, &backup) {
                    Ok(()) => tracing::warn!(
                        error = %e,
                        backup = %backup.display(),
                        "state.json was corrupt; backed it up and started fresh"
                    ),
                    Err(re) => tracing::warn!(
                        error = %re,
                        path = %path.display(),
                        "state.json is corrupt and couldn't be moved aside; ignoring it"
                    ),
                }
                Ok(Self::default())
            }
        }
    }

    pub fn load_default() -> Result<(Self, PathBuf)> {
        let path = Self::default_path()?;
        Ok((Self::load(&path)?, path))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        let text = serde_json::to_string_pretty(self).context("serializing state")?;
        std::fs::write(path, text).with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }

    /// Record a manual save-folder override for `slug`. Subsequent calls to
    /// [`crate::detection::detect_all`] return a row whose `found_paths` is
    /// exactly `[path]` and whose source is `ManualOverride`, regardless of
    /// what the heuristics produced.
    pub fn set_manual_path(&mut self, slug: &str, path: PathBuf) {
        self.manual_paths.insert(slug.to_string(), path);
    }

    /// Drop the manual override for `slug` (if any). After this the next
    /// detect_all pass returns whatever the heuristics find.
    pub fn clear_manual_path(&mut self, slug: &str) {
        self.manual_paths.remove(slug);
    }

    /// True when `slug` has been blacklisted via
    /// [`Self::add_ignored_slug`]. The Library page filters detected games
    /// against this set so they stop reappearing in the grid until the user
    /// reactivates them from Settings.
    pub fn is_ignored(&self, slug: &str) -> bool {
        self.ignored_slugs.contains(slug)
    }

    /// Persistently blacklist a detected slug. After this call any
    /// `list_detected_games` invocation drops the row before returning it to
    /// the UI. Idempotent: re-adding an existing slug is a no-op.
    pub fn add_ignored_slug(&mut self, slug: String) {
        self.ignored_slugs.insert(slug);
    }

    /// Drop the blacklist entry for `slug` so the next detection pass
    /// re-surfaces it. Mirrors `add_ignored_slug`. Idempotent.
    pub fn remove_ignored_slug(&mut self, slug: &str) {
        self.ignored_slugs.remove(slug);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `manual_paths` survives a save → load round-trip and a missing field
    /// in older `state.json` files deserialises as an empty map.
    #[test]
    fn manual_paths_round_trip_to_disk() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("state.json");

        let mut state = CliState::default();
        state.set_manual_path("stellaris", PathBuf::from("/home/x/Stellaris/save games"));
        state.set_manual_path("ck3", PathBuf::from("/data/ck3"));
        state.save(&path).unwrap();

        let loaded = CliState::load(&path).unwrap();
        assert_eq!(loaded.manual_paths.len(), 2);
        assert_eq!(
            loaded.manual_paths.get("stellaris"),
            Some(&PathBuf::from("/home/x/Stellaris/save games")),
        );
        assert_eq!(
            loaded.manual_paths.get("ck3"),
            Some(&PathBuf::from("/data/ck3")),
        );
    }

    /// Pre-1.5 state files have no `manual_paths` key. Loading them must not
    /// fail and must default to an empty map (no serde migration step).
    #[test]
    fn manual_paths_default_when_missing_from_disk() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("state.json");
        std::fs::write(&path, "{\"saves\":{}}").unwrap();

        let loaded = CliState::load(&path).unwrap();
        assert!(loaded.manual_paths.is_empty());
    }

    /// `clear_manual_path` removes the entry; subsequent saves no longer
    /// emit the slug.
    #[test]
    fn clear_manual_path_removes_entry() {
        let mut state = CliState::default();
        state.set_manual_path("stardew-valley", PathBuf::from("/x"));
        assert_eq!(state.manual_paths.len(), 1);
        state.clear_manual_path("stardew-valley");
        assert!(state.manual_paths.is_empty());
        // Idempotent: clearing an unknown slug doesn't panic.
        state.clear_manual_path("not-there");
    }

    /// Default `CliState` has no blacklisted slugs — the field is purely
    /// opt-in.
    #[test]
    fn ignored_slugs_default_empty() {
        assert!(CliState::default().ignored_slugs.is_empty());
    }

    /// Round-trip the blacklist API: add a slug, see it via `is_ignored`,
    /// drop it, see it gone. Idempotent on both ends.
    #[test]
    fn add_and_remove_ignored_slug() {
        let mut state = CliState::default();
        assert!(!state.is_ignored("lethal-company"));

        state.add_ignored_slug("lethal-company".to_string());
        assert!(state.is_ignored("lethal-company"));
        assert_eq!(state.ignored_slugs.len(), 1);

        // Idempotent: re-adding doesn't grow the set.
        state.add_ignored_slug("lethal-company".to_string());
        assert_eq!(state.ignored_slugs.len(), 1);

        state.remove_ignored_slug("lethal-company");
        assert!(!state.is_ignored("lethal-company"));
        assert!(state.ignored_slugs.is_empty());

        // Idempotent: removing an unknown slug doesn't panic.
        state.remove_ignored_slug("not-there");
    }

    /// `ignored_slugs` survives a JSON round-trip and pre-1.5.3 state files
    /// (no `ignored_slugs` key) deserialise as an empty set.
    #[test]
    fn serialize_with_empty_ignored_does_not_emit_field_explicitly_or_does_emit_consistently() {
        // Round-trip with a populated set: every slug survives.
        let mut state = CliState::default();
        state.add_ignored_slug("lethal-company".to_string());
        state.add_ignored_slug("terraforming-mars".to_string());
        let json = serde_json::to_string(&state).unwrap();
        let parsed: CliState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.ignored_slugs.len(), 2);
        assert!(parsed.is_ignored("lethal-company"));
        assert!(parsed.is_ignored("terraforming-mars"));

        // Pre-1.5.3 files without the key load with an empty set thanks to
        // `#[serde(default)]`.
        let legacy: CliState = serde_json::from_str("{\"saves\":{}}").unwrap();
        assert!(legacy.ignored_slugs.is_empty());

        // Empty set round-trips back to empty.
        let empty = CliState::default();
        let empty_json = serde_json::to_string(&empty).unwrap();
        let parsed_empty: CliState = serde_json::from_str(&empty_json).unwrap();
        assert!(parsed_empty.ignored_slugs.is_empty());
    }
}
