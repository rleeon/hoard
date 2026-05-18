use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
        let st: CliState =
            serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        Ok(st)
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
}
