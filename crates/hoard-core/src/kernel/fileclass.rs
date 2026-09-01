//! What each file inside a save folder actually is.
//!
//! A save folder almost never holds only saves. `walk_source` used to take every
//! regular file it found, and that drags into the snapshot things that are not
//! the player's data but *this machine's*: a Unity `Player.log`, the analytics
//! queue carrying the install GUID, this GPU's shader info,
//! `steam_autocloud.vdf`, a `graphics.ini` with this monitor's resolution.
//!
//! Two separate kinds of damage:
//!
//! * Noise. The log is rewritten on every launch, so the cheap signature moves,
//!   the content signature confirms the bytes really did change (they did, it is
//!   a log), and a new cloud version gets cut every single time the game opens
//!   without the save being touched.
//! * Crashes. Restoring PC A's `graphics.ini` onto PC B hands the game a
//!   resolution, a GPU or a path that does not exist on that machine.
//!
//! ## The ladder, least to most destructive
//!
//! [`FileClass::Junk`] is the only thing that stops being uploaded, which is why
//! the list is short and matches exact names wherever it can: a file that is not
//! uploaded cannot be recovered, so doubt never lands here.
//!
//! [`FileClass::DeviceLocal`] is where doubt lands. It does get uploaded (if the
//! disk burns, it is there), but a restore will not write it unless the user
//! asks by hand (`--allow-ini` on the CLI, a switch that is off by default in
//! the desktop dialog). That way the most expensive misclassification possible,
//! calling config something that was the save, costs a click rather than the
//! save.
//!
//! ## Shields from the manifest
//!
//! The catalogue carries a file pattern in 20,499 of its 47,404 templates
//! (`<base>/Saves/*.sav`), and there the community does know what save data is.
//! That pattern arrives here as `shields`: whatever matches one is save data and
//! no rule below touches it. It is genuinely needed, because `.ini` is the save
//! pattern of 582 templates, `.cfg` of 98 and `.log` of 64, so without shields
//! the extension rules would mow down real saves.
//!
//! The reverse is not used: a file the manifest does not list is not thereby
//! condemned. The catalogue has enormous holes (one game is a bare directory
//! with not a single pattern), and trusting it to exclude would mean trusting a
//! hole to delete.

/// What a file inside the save folder is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileClass {
    /// The player's data. Uploaded and restored, as always.
    SaveData,
    /// This machine's data rather than the player's: config, settings, generic
    /// logs. Uploaded so it is never lost, but a restore will not write it
    /// without an explicit request.
    DeviceLocal,
    /// Neither the player's data nor config anyone wants back: OS litter,
    /// temporaries, crash dumps, engine telemetry. Never uploaded, never
    /// restored.
    Junk,
}

impl FileClass {
    /// Does it go into a new snapshot?
    pub fn is_backed_up(self) -> bool {
        !matches!(self, FileClass::Junk)
    }

    /// Does a restore write it to disk? `allow_device_local` is the switch the
    /// user turns on by hand.
    pub fn is_restored(self, allow_device_local: bool) -> bool {
        match self {
            FileClass::SaveData => true,
            FileClass::DeviceLocal => allow_device_local,
            FileClass::Junk => false,
        }
    }
}

/// OS and file-manager litter, by exact name.
const JUNK_NAMES: &[&str] = &[
    ".ds_store",
    "thumbs.db",
    "ehthumbs.db",
    "desktop.ini",
    ".directory",
    // Steam's own bookkeeping, not the game's: which files it had to sync and
    // when. Restoring it on another machine lies to the Steam client.
    "steam_autocloud.vdf",
    "remotecache.vdf",
    // Engine logs by exact name. Generic `*.log` does not land here but in
    // `DeviceLocal`, because `.log` is the save pattern of 64 catalogue
    // templates and not all of them are shielded.
    "player.log",
    "player-prev.log",
    "output_log.txt",
    "output_log_prev.txt",
    // A lock the game holds open exclusively while it runs. It carries no data,
    // and on Windows it cannot even be opened for reading with the game alive
    // (sharing violation, os error 32), which used to abort the whole backup
    // halfway through the walk.
    "session.lock",
];

/// Extensions that are never save data.
const JUNK_EXTS: &[&str] = &[
    // Crash dumps.
    "dmp",
    "mdmp",
    "stackdump", // Escrituras a medias y temporales de editores/descargas.
    "tmp",
    "temp",
    "part",
    "crdownload",
    "swp",
];

/// A path segment with this name hangs off engine telemetry, not off the save.
const JUNK_SEGMENTS: &[&str] = &[
    // Unity: shader and GPU info for *this* machine.
    "shadervariantanalytics",
    // Unreal.
    "crashreportclient",
];

/// Config extensions. Unshielded, a file with one of these uploads but never
/// gets restored over a live machine.
const CONFIG_EXTS: &[&str] = &[
    "ini",
    "cfg",
    "conf",
    "config",
    "toml",
    "yaml",
    "yml",
    "vdf",
    "properties",
    // Generic log: kept just in case, never restored.
    "log",
];

/// A stem ending in one of these is config whatever its extension. Catches
/// `GraphicsSettings.json`, `Fallout4Prefs.ini`, `UserOptions.dat`.
const CONFIG_STEM_SUFFIXES: &[&str] = &[
    "settings",
    "config",
    "configuration",
    "prefs",
    "preferences",
    "options",
];

/// A stem that is exactly one of these is config. Exact rather than
/// "contains", deliberately: `input` is config, `input_puzzle_solved` would be
/// the save.
const CONFIG_STEMS: &[&str] = &[
    "graphics",
    "graphic",
    "video",
    "audio",
    "sound",
    "display",
    "resolution",
    "input",
    "controls",
    "keybinds",
    "keybindings",
    "keyboard",
    "gamepad",
    "launcher",
    "hardware",
];

/// What a restore is allowed to write to disk.
///
/// Travels inside `RestoreOptions` and rides along with the preview, so that
/// what `--dry-run` promises and what the restore does come out of one decision
/// rather than two copies drifting apart.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RestoreGate {
    /// Manifest patterns that shield a file as save data.
    pub shields: Vec<String>,
    /// The user asked by hand for the snapshot's config to be written over this
    /// machine's (`--allow-ini`, the switch in the dialog). Off by default, and
    /// always off in auto-restore: writing PC A's config onto PC B is precisely
    /// the crash this module exists to prevent.
    pub allow_device_local: bool,
}

impl RestoreGate {
    /// Wide open, the way things were before any of this existed. For tests and
    /// for callers that already did their own filtering.
    pub fn permissive() -> Self {
        Self {
            shields: Vec::new(),
            allow_device_local: true,
        }
    }

    /// Does this snapshot file get written to disk?
    pub fn allows(&self, rel_path: &str) -> bool {
        classify(rel_path, &self.shields).is_restored(self.allow_device_local)
    }
}

/// Classifies a file by its path relative to the save root, `/`-separated, the
/// shape `walk_source` already produces.
///
/// `shields` are filename patterns lifted from the manifest (`*.sav`, `save*`).
/// Anything matching one is save data and leaves by the top door without
/// meeting another rule.
pub fn classify(rel_path: &str, shields: &[String]) -> FileClass {
    let lower = rel_path.to_ascii_lowercase();
    let name = lower.rsplit('/').next().unwrap_or(&lower);

    // 1. The manifest rules: if it says this is a save, it is a save.
    if shields.iter().any(|p| glob_match(p, name)) {
        return FileClass::SaveData;
    }

    // 2. Unambiguous litter, the only thing that stops being uploaded.
    if JUNK_NAMES.contains(&name) {
        return FileClass::Junk;
    }
    // The AppleDouble `._foo` files macOS scatters on non-HFS volumes.
    if name.starts_with("._") {
        return FileClass::Junk;
    }
    if let Some(ext) = extension_of(name) {
        if JUNK_EXTS.contains(&ext) {
            return FileClass::Junk;
        }
    }
    let segments: Vec<&str> = lower.split('/').collect();
    // Everything hanging off a telemetry directory.
    if segments
        .iter()
        .take(segments.len().saturating_sub(1))
        .any(|s| JUNK_SEGMENTS.contains(s))
    {
        return FileClass::Junk;
    }
    // The Unity Analytics event queue, `Unity/<guid>/Analytics/...`. The GUID
    // identifies the *install*, so restoring it onto another machine clones its
    // analytics identity.
    if is_under_unity_analytics(&segments) {
        return FileClass::Junk;
    }

    // 3. Config and the rest of this machine's data. Uploaded, never restored
    //    on its own.
    if let Some(ext) = extension_of(name) {
        if CONFIG_EXTS.contains(&ext) {
            return FileClass::DeviceLocal;
        }
    }
    let stem = stem_of(name);
    if CONFIG_STEMS.contains(&stem) || CONFIG_STEM_SUFFIXES.iter().any(|s| stem.ends_with(s)) {
        return FileClass::DeviceLocal;
    }

    FileClass::SaveData
}

/// `Unity/<something>/Analytics/...` at any depth. The `unity` ancestor is
/// required so a game's own `analytics` folder is not mistaken for it.
fn is_under_unity_analytics(segments: &[&str]) -> bool {
    let Some(unity_at) = segments.iter().position(|s| *s == "unity") else {
        return false;
    };
    // The file itself does not count as a containing directory.
    segments
        .iter()
        .enumerate()
        .any(|(i, s)| i > unity_at && i + 1 < segments.len() && *s == "analytics")
}

/// Lowercase extension without the dot. `None` when there is none, or when the
/// dot opens the name: `.bashrc` has no extension, that is its name.
fn extension_of(name: &str) -> Option<&str> {
    let (stem, ext) = name.rsplit_once('.')?;
    if stem.is_empty() {
        return None;
    }
    Some(ext)
}

/// The name without its extension.
fn stem_of(name: &str) -> &str {
    match name.rsplit_once('.') {
        Some((stem, _)) if !stem.is_empty() => stem,
        _ => name,
    }
}

/// Is this manifest pattern any use as a shield?
///
/// `*` and `*.*` match everything, so they would shield the whole folder and
/// leave the filter doing nothing. They do not say *what* a save is, only "there
/// are files here", and 1,519 catalogue templates are exactly that.
pub fn is_useful_shield(pattern: &str) -> bool {
    let p = pattern.trim();
    if !p.contains('*') && !p.contains('?') {
        // A literal name is informative, and then it is not acting as a
        // wildcard at all: it still works as an exact shield.
        return !p.is_empty();
    }
    !matches!(p, "*" | "*.*" | "?" | "**")
}

/// Single-segment glob: `*` is anything including empty, `?` is one character.
/// No classes and no alternatives, because the manifest does not use them in the
/// last segment.
///
/// Written here rather than reused from `pathexpand` because the kernel does not
/// depend on `hoard-agent` (ADR 0021's hard rule: the kernel imports no shells).
fn glob_match(pattern: &str, name: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let n: Vec<char> = name.chars().collect();
    let (mut pi, mut ni) = (0usize, 0usize);
    // The last `*` seen and where the name was then, so we can backtrack.
    let (mut star, mut backtrack) = (usize::MAX, 0usize);
    while ni < n.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == n[ni]) {
            pi += 1;
            ni += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = pi;
            backtrack = ni;
            pi += 1;
        } else if star != usize::MAX {
            pi = star + 1;
            backtrack += 1;
            ni = backtrack;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(path: &str) -> FileClass {
        classify(path, &[])
    }

    #[test]
    fn plain_save_files_are_save_data() {
        for p in [
            "save1.sav",
            "autosave/autosave0.sav",
            "bmonster_4_6_2026_auto_1242.pss",
            "savedGames.gd",
            "level.dat",
            "world/region/r.0.0.mca",
        ] {
            assert_eq!(c(p), FileClass::SaveData, "{p}");
        }
    }

    /// A real user's folder that currently syncs whole. This is the case that
    /// motivated the module.
    #[test]
    fn the_unity_folder_that_started_this() {
        assert_eq!(c("Player.log"), FileClass::Junk);
        assert_eq!(c("Player-prev.log"), FileClass::Junk);
        assert_eq!(c("steam_autocloud.vdf"), FileClass::Junk);
        assert_eq!(
            c("Unity/0a8833bc-a8ad-47f7-abed-f8d04a6f02f8/Analytics/values"),
            FileClass::Junk
        );
        assert_eq!(
            c("Unity/ShaderVariantAnalytics/ShaderRuntimeInfoEvent.json"),
            FileClass::Junk
        );
        // And the saves in the same folder come out untouched.
        assert_eq!(c("savedGames2.gd"), FileClass::SaveData);
        assert_eq!(c("savedGamesDeepBackup.gd.restore"), FileClass::SaveData);
    }

    #[test]
    fn os_and_temp_junk() {
        for p in [
            ".DS_Store",
            "Thumbs.db",
            "desktop.ini",
            "._save.sav",
            "crash_2026.dmp",
            "save.sav.tmp",
            "download.part",
        ] {
            assert_eq!(c(p), FileClass::Junk, "{p}");
        }
    }

    #[test]
    fn config_is_device_local_not_junk() {
        for p in [
            "graphics.ini",
            "settings.toml",
            "config.json",
            "GraphicsSettings.json",
            "Fallout4Prefs.ini",
            "UserOptions.dat",
            "keybinds.cfg",
            "video.yaml",
            "debug.log",
        ] {
            assert_eq!(c(p), FileClass::DeviceLocal, "{p}");
        }
    }

    /// The ladder's rule: anything doubtful still uploads. Only unambiguous
    /// litter stays out of the snapshot.
    #[test]
    fn only_junk_is_dropped_from_the_backup() {
        assert!(!FileClass::Junk.is_backed_up());
        assert!(FileClass::DeviceLocal.is_backed_up());
        assert!(FileClass::SaveData.is_backed_up());
    }

    #[test]
    fn device_local_needs_an_explicit_yes_to_be_restored() {
        assert!(!FileClass::DeviceLocal.is_restored(false));
        assert!(FileClass::DeviceLocal.is_restored(true));
        // Litter does not come back even on request; the switch is for config.
        assert!(!FileClass::Junk.is_restored(true));
        assert!(FileClass::SaveData.is_restored(false));
    }

    /// 582 catalogue templates use `*.ini` as their save pattern. Unshielded,
    /// the extension rule would take them all.
    #[test]
    fn the_manifest_shield_beats_every_rule_below_it() {
        let shields = vec!["*.ini".to_string()];
        assert_eq!(classify("save01.ini", &shields), FileClass::SaveData);
        assert_eq!(classify("save01.ini", &[]), FileClass::DeviceLocal);

        let log_shield = vec!["*.log".to_string()];
        assert_eq!(classify("player.log", &log_shield), FileClass::SaveData);
        assert_eq!(classify("player.log", &[]), FileClass::Junk);
    }

    #[test]
    fn shields_match_on_the_basename_at_any_depth() {
        let shields = vec!["*.bksav".to_string()];
        assert_eq!(
            classify("Saves/slot3/quick.bksav", &shields),
            FileClass::SaveData
        );
    }

    #[test]
    fn degenerate_patterns_are_not_shields() {
        // These would shield the whole folder and leave the filter doing
        // nothing.
        assert!(!is_useful_shield("*"));
        assert!(!is_useful_shield("*.*"));
        assert!(!is_useful_shield("**"));
        assert!(is_useful_shield("*.sav"));
        assert!(is_useful_shield("save*"));
        assert!(is_useful_shield("gamedata.bin"));
    }

    #[test]
    fn the_gate_is_shut_for_config_by_default() {
        let gate = RestoreGate::default();
        assert!(gate.allows("slot1.sav"));
        assert!(!gate.allows("graphics.ini"));
        assert!(!gate.allows("Player.log"));
    }

    #[test]
    fn the_gate_opens_for_config_when_asked_but_never_for_junk() {
        let gate = RestoreGate {
            shields: Vec::new(),
            allow_device_local: true,
        };
        assert!(gate.allows("graphics.ini"));
        // Litter does not come back even on request.
        assert!(!gate.allows("Player.log"));
        assert!(!gate.allows(".DS_Store"));
    }

    #[test]
    fn a_shielded_config_file_still_goes_through_a_shut_gate() {
        let gate = RestoreGate {
            shields: vec!["*.ini".to_string()],
            allow_device_local: false,
        };
        assert!(gate.allows("save01.ini"));
    }

    #[test]
    fn glob_basics() {
        assert!(glob_match("*.sav", "slot1.sav"));
        assert!(!glob_match("*.sav", "slot1.savx"));
        assert!(glob_match("save*", "save"));
        assert!(glob_match("profile?.sav", "profile1.sav"));
        assert!(!glob_match("profile?.sav", "profile12.sav"));
        assert!(glob_match("*save*.dat", "my_save_2.dat"));
    }

    /// A game's own `analytics` folder is not Unity telemetry. The `Unity/`
    /// ancestor is what condemns it.
    #[test]
    fn analytics_alone_is_not_enough() {
        assert_eq!(c("analytics/run1.sav"), FileClass::SaveData);
        assert_eq!(c("Unity/x/Analytics/run1.sav"), FileClass::Junk);
    }

    #[test]
    fn dotfiles_have_no_extension() {
        assert_eq!(extension_of(".bashrc"), None);
        assert_eq!(extension_of("save.sav"), Some("sav"));
        assert_eq!(stem_of("graphicssettings.json"), "graphicssettings");
    }
}
