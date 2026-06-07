//! Per-save sync presets.
//!
//! A preset is a named bundle of overrides layered on top of the global
//! [`crate::agent::AgentConfig`] for a single save. They let us special-case
//! games whose save folders behave oddly — R.E.P.O. wipes its own directory at
//! the menu and plays in short bursts — without the user hand-tuning raw knobs,
//! and let the user pin a manual preset when a game "goes weird".
//!
//! Flow: the desktop resolves a preset *name* (chosen by the user, or
//! auto-assigned from [`builtin_preset_for`]) into a [`SavePolicy`], stores the
//! name in `state.json`, and carries the resolved policy on the
//! [`crate::agent::WatchedSave`]. The agent loop reads
//! `slot.save.policy.<field>.unwrap_or(config.<field>)` at each decision point,
//! so a `None` field simply inherits the global setting.

use serde::{Deserialize, Serialize};

/// Overrides applied to one save. Every field is optional: `None` means
/// "inherit the global setting". Serialised on [`crate::agent::WatchedSave`]
/// (and indirectly persisted via the preset name in `state.json`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SavePolicy {
    /// Override auto-restore for this save. `Some(false)` = backup-only (never
    /// pull the cloud copy down over local); `Some(true)` = always restore an
    /// empty/missing folder; `None` = follow the global preference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_restore: Option<bool>,
    /// Override the minimum snapshot interval (seconds). Lower = more frequent
    /// backups, good for short-session games; higher = data saving. `None`
    /// inherits the value derived from the global `data_saving` knob.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_snapshot_interval_secs: Option<u64>,
    /// Override the debounce window (seconds) a settled write waits before it
    /// backs up. `None` inherits `AgentConfig::debounce_secs`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub debounce_secs: Option<u64>,
}

impl SavePolicy {
    /// Resolve a preset name into its policy. An unknown or `None` name yields
    /// the empty (all-inherit) policy.
    pub fn from_preset(name: Option<&str>) -> Self {
        name.map(policy_for_preset).unwrap_or_default()
    }
}

/// Stable preset identifiers. Stored verbatim in `state.json` and matched in
/// [`policy_for_preset`]; keep them stable across releases.
pub const PRESET_STANDARD: &str = "standard";
pub const PRESET_SHORT_SESSION: &str = "short_session";
pub const PRESET_BACKUP_ONLY: &str = "backup_only";
pub const PRESET_DATA_SAVER: &str = "data_saver";

/// Every selectable preset id. The UI renders a localized label for each;
/// `standard` is the implicit default (empty policy).
pub const ALL_PRESETS: &[&str] = &[
    PRESET_STANDARD,
    PRESET_SHORT_SESSION,
    PRESET_BACKUP_ONLY,
    PRESET_DATA_SAVER,
];

/// Map a preset id to its concrete overrides.
pub fn policy_for_preset(name: &str) -> SavePolicy {
    match name {
        // Short, bursty co-op rounds where the game rewrites/clears its own
        // save folder (R.E.P.O.): back up eagerly so a round's progress lands
        // before the next wipe, and keep restore on so the save reappears.
        PRESET_SHORT_SESSION => SavePolicy {
            auto_restore: Some(true),
            min_snapshot_interval_secs: Some(30),
            debounce_secs: Some(5),
        },
        // Never overwrite local with the cloud copy. Pure one-way upload.
        PRESET_BACKUP_ONLY => SavePolicy {
            auto_restore: Some(false),
            ..Default::default()
        },
        // Stretch the gap between backups to the max to spare bandwidth.
        PRESET_DATA_SAVER => SavePolicy {
            min_snapshot_interval_secs: Some(600),
            ..Default::default()
        },
        // `standard` and anything unknown inherit every global setting.
        _ => SavePolicy::default(),
    }
}

/// Built-in preset auto-assigned to a known-quirky game on track, unless the
/// user has already pinned one. Keyed by game slug. This is "our own list of
/// games that behave differently".
pub fn builtin_preset_for(slug: &str) -> Option<&'static str> {
    match slug {
        "r-e-p-o" => Some(PRESET_SHORT_SESSION),
        _ => None,
    }
}

/// Process executable names for games the storefront/manifest can't supply on
/// its own — non-Steam launchers (TLauncher Minecraft → `javaw.exe`) and games
/// whose binary the catalog doesn't map. The agent's process poll matches these
/// (case-insensitive) to fire `GameStarted` / `GameStopped`, so "is the game
/// running" works even without a Steam install dir. Linux-native and Windows
/// variants are both listed because the match is exact on the process name.
///
/// This is the same curated "games that behave differently" list, extended to
/// the detection side: without it Minecraft and Factorio never register as
/// "playing" and the running-game guards never engage.
pub fn builtin_processes_for(slug: &str) -> &'static [&'static str] {
    match slug {
        "minecraft" => &["javaw.exe", "java.exe", "minecraft.exe", "minecraft"],
        "factorio" => &["factorio.exe", "factorio"],
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_and_standard_inherit_everything() {
        assert_eq!(policy_for_preset(PRESET_STANDARD), SavePolicy::default());
        assert_eq!(policy_for_preset("nope"), SavePolicy::default());
        assert_eq!(SavePolicy::from_preset(None), SavePolicy::default());
    }

    #[test]
    fn short_session_backs_up_eagerly_and_restores() {
        let p = policy_for_preset(PRESET_SHORT_SESSION);
        assert_eq!(p.auto_restore, Some(true));
        assert_eq!(p.min_snapshot_interval_secs, Some(30));
    }

    #[test]
    fn backup_only_disables_restore() {
        assert_eq!(policy_for_preset(PRESET_BACKUP_ONLY).auto_restore, Some(false));
    }

    #[test]
    fn repo_is_in_the_quirky_list() {
        assert_eq!(builtin_preset_for("r-e-p-o"), Some(PRESET_SHORT_SESSION));
        assert_eq!(builtin_preset_for("factorio"), None);
    }

    #[test]
    fn known_games_carry_process_names() {
        assert!(builtin_processes_for("minecraft").contains(&"javaw.exe"));
        assert!(builtin_processes_for("factorio").contains(&"factorio.exe"));
        assert!(builtin_processes_for("unknown-game").is_empty());
    }

    #[test]
    fn policy_omits_none_fields_when_serialized() {
        let json = serde_json::to_string(&SavePolicy::default()).unwrap();
        assert_eq!(json, "{}");
    }
}
