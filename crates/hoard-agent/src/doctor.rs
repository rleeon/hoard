//! `hoard doctor` — the deterministic half of "is anything wrong with my
//! saves?".
//!
//! Every rule here fires on a mistake we have actually shipped into someone's
//! machine: a save pointing at a game's install directory instead of its save
//! folder, a tracked folder that is really a backup mirror, a row named after
//! an installer. They are heuristics over data the engine already holds — no
//! model, no key, no network — and each one carries the command that fixes it,
//! so a caller (the desktop, the CLI, or an assistant driving the CLI) proposes
//! rather than guesses.
//!
//! Offline by design: state plus the filesystem plus the detection cache if one
//! is already on disk. `doctor` never triggers a scan, because a diagnosis the
//! user waits two minutes for is one they run once.
//!
//! Findings are advice, not failure: a caller reports them and exits 0.

use serde::Serialize;
use std::path::{Path, PathBuf};

use crate::junkdirs;
use crate::library;
use crate::state::{CliState, SaveState};

/// How much a finding should worry the reader.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// This save cannot be syncing correctly right now.
    Error,
    /// Probably wrong, worth a look.
    Warning,
    /// Odd, but the user set it up this way on purpose. Worth mentioning once,
    /// never worth "fixing" behind their back.
    Notice,
}

/// What kind of problem this is. A stable vocabulary: callers branch on `code`,
/// never on the wording of `detail`.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FindingCode {
    /// The tracked folder is gone (uninstalled game, unmounted drive, moved
    /// install).
    MissingFolder,
    /// Tracked, exists, and holds nothing — there is no save here to upload.
    EmptyFolder,
    /// The folder is a backup copy of another one, by name (`Saves.bak`).
    BackupSuffix,
    /// The save is named after an installer, so the row was born from a bad
    /// correlation rather than from the game.
    InstallerNamed,
    /// The path is one we refuse to sync (a dangerous root, or Hoard's own
    /// data folder).
    DangerousRoot,
    /// Detection knows a different folder for this game than the one being
    /// tracked — the shape of the install-dir mistrack.
    AlternatePathKnown,
}

/// One problem, with the way out.
#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub save_id: String,
    pub game_slug: String,
    pub label: String,
    pub code: FindingCode,
    pub severity: Severity,
    /// The user put this folder here by hand — a manual path override, or a
    /// numbered slot they created. Deliberate oddities still get reported, but
    /// they are reported as theirs: "you set this up this way" reads very
    /// differently from "this is broken", and only one of them is true.
    pub manual: bool,
    /// The tracked folder this is about.
    pub path: String,
    /// One sentence, for a human to read.
    pub detail: String,
    /// A better folder, when a rule found one.
    pub suggested_path: Option<String>,
    /// The exact command that would act on this, ready to run after the user
    /// approves it. Spelled out so a caller never has to assemble one — an
    /// invented `save_id` or a wrong flag is how a diagnosis becomes damage.
    pub command: String,
}

/// Run every rule over what this machine tracks.
pub fn diagnose(state: &CliState) -> Vec<Finding> {
    // Read the cache; absent is fine, it only gates the last rule.
    let cached = library::load_detection_from_disk();
    diagnose_with(state, cached.as_ref().map(|c| &c.report))
}

/// [`diagnose`] with the detection report handed in, so the rule that depends
/// on it can be tested without a scan on disk.
pub fn diagnose_with(
    state: &CliState,
    report: Option<&crate::detection::DetectionReport>,
) -> Vec<Finding> {
    let mut out = Vec::new();

    let mut saves: Vec<(&String, &SaveState)> = state.saves.iter().collect();
    // Deterministic order: same input, same report.
    saves.sort_by(|(_, a), (_, b)| {
        a.game_slug
            .cmp(&b.game_slug)
            .then_with(|| a.label.cmp(&b.label))
    });

    for (save_id, s) in saves {
        let path = &s.local_path;
        let manual = is_manual(state, s);
        let base = |code, severity, detail: String, suggested: Option<PathBuf>, command| Finding {
            save_id: save_id.clone(),
            game_slug: s.game_slug.clone(),
            label: s.label.clone(),
            code,
            severity,
            manual,
            path: path.display().to_string(),
            detail,
            suggested_path: suggested.map(|p: PathBuf| p.display().to_string()),
            command,
        };

        // A path we would refuse today. Older rows predate the guard, so this
        // catches what got in before it existed.
        if let Err(e) = library::validate_path_shape(path) {
            out.push(base(
                FindingCode::DangerousRoot,
                Severity::Error,
                format!("{e}"),
                None,
                format!("hoard save untrack {save_id}"),
            ));
            continue;
        }

        if !path.exists() {
            out.push(base(
                FindingCode::MissingFolder,
                Severity::Error,
                "The tracked folder doesn't exist. The game may be uninstalled, \
                 the drive unmounted, or the install moved."
                    .to_string(),
                None,
                format!("hoard save path {save_id} <new folder>"),
            ));
            continue;
        }

        // A single tracked file is a supported shape; "empty" doesn't apply.
        //
        // What to say depends on whether this save ever produced a version. One
        // that never did is a folder nothing was ever written to; one that has
        // versions and is empty *now* is a folder that lost its contents, and
        // telling that user to untrack would throw away the history that is the
        // only remaining copy.
        if path.is_dir() && is_empty_dir(path) {
            let (detail, command) = match s.last_version_num {
                None => (
                    "The folder is tracked but empty, and this save has never \
                     produced a version: nothing here has ever been backed up."
                        .to_string(),
                    format!("hoard save untrack {save_id}"),
                ),
                Some(v) => (
                    format!(
                        "The folder is empty now, but this save has {v} stored \
                         version(s). The game may be uninstalled or the files moved \
                         — the cloud copy is currently the only one."
                    ),
                    format!("hoard restore {save_id} --dry-run"),
                ),
            };
            out.push(base(
                FindingCode::EmptyFolder,
                Severity::Warning,
                detail,
                None,
                command,
            ));
        }

        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if junkdirs::ends_with_backup_suffix(name) {
                out.push(base(
                    FindingCode::BackupSuffix,
                    Severity::Warning,
                    format!(
                        "\"{name}\" is named like a backup copy. Tracking a mirror \
                         means the history follows the copy, not the save the game writes."
                    ),
                    None,
                    format!("hoard save path {save_id} <the game's own save folder>"),
                ));
            }
        }

        // `is_installer_like` matches lowercase tokens: its caller in
        // correlation feeds it process names already normalised, and a label
        // typed as "…Setup" would slip past otherwise.
        if crate::correlation::is_installer_like(&s.label.to_lowercase())
            || crate::correlation::is_installer_like(&s.game_slug.to_lowercase())
        {
            out.push(base(
                FindingCode::InstallerNamed,
                Severity::Warning,
                "This save is named after an installer, which means it was created \
                 from a bad match rather than from the game."
                    .to_string(),
                None,
                format!("hoard save untrack {save_id}"),
            ));
        }

        // Detection knows somewhere else for this game. The Planet S shape: the
        // row points at the Steam install dir while the real saves live under
        // the user's profile.
        //
        // Narrow on purpose, because the first version of this rule told a real
        // user to repoint a 454 MB Factorio save with 284 versions at a Steam
        // emulator's `remote/` folder — advice that would have pointed the
        // history at the wrong files. Two guards:
        //
        // - A save that has uploaded a version is watching files the game
        //   really writes. Whatever else detection found, this folder works.
        // - Several rows for one game are deliberate slots (the number lives in
        //   the label), and each one's "different folder" is just its sibling.
        // Whether this row is one of several the user keeps for one game — the
        // saves in one, the config or a debug folder in another.
        let sibling_slots = state
            .saves
            .values()
            .filter(|o| o.game_slug == s.game_slug)
            .count()
            > 1;
        let working = s.last_version_num.is_some();

        if let Some(report) = report {
            let known = library::detected_paths_in(report, &s.game_slug);
            if let Some(other) = known
                .iter()
                .map(|d| &d.path)
                .find(|p| !paths_equal(p, path))
            {
                // Deliberate, or already producing versions, or one of several
                // slots: say so, and do **not** hand over a repoint command.
                // The first version of this rule offered to repoint a 454 MB
                // save with 284 versions at a Steam emulator's state folder,
                // and the command it printed was ready to run.
                let deliberate = manual || sibling_slots || working;
                if deliberate {
                    let why = if manual {
                        "you set this folder yourself"
                    } else if sibling_slots {
                        "you keep more than one folder for this game"
                    } else {
                        "this save is already storing versions from it"
                    };
                    out.push(base(
                        FindingCode::AlternatePathKnown,
                        Severity::Notice,
                        format!(
                            "Detection also found {} for {}, which is not the tracked \
                             folder — but {why}, so this is probably how you want it.",
                            other.display(),
                            s.game_slug
                        ),
                        Some(other.clone()),
                        format!("hoard save show {save_id}"),
                    ));
                } else {
                    out.push(base(
                        FindingCode::AlternatePathKnown,
                        Severity::Warning,
                        format!(
                            "This save has never stored a version, and detection found a \
                             different save folder for {}. The tracked one may be the \
                             install directory.",
                            s.game_slug
                        ),
                        Some(other.clone()),
                        format!("hoard save path {save_id} \"{}\"", other.display()),
                    ));
                }
            }
        }
    }

    out
}

/// Did the user put this folder here on purpose?
///
/// Two independent signals, either one is enough:
/// - a manual path override for this game in `state.manual_paths` (the user
///   overruled detection by hand), pointing at this very folder;
/// - a numbered slot in the label, which only exists because someone chose to
///   keep more than one folder for the game.
fn is_manual(state: &CliState, s: &SaveState) -> bool {
    let overridden = state
        .manual_paths
        .get(&s.game_slug)
        .is_some_and(|p| paths_equal(p, &s.local_path));
    overridden || hoard_core::kernel::slots::slot_of(&s.label).is_some_and(|n| n > 1)
}

fn is_empty_dir(p: &Path) -> bool {
    // Unreadable is not empty: saying "nothing to back up" about a folder we
    // failed to open would send the user to untrack a save that is fine.
    std::fs::read_dir(p)
        .map(|mut d| d.next().is_none())
        .unwrap_or(false)
}

fn paths_equal(a: &Path, b: &Path) -> bool {
    // Compare canonically when both resolve (symlinks, `..`, case on Windows);
    // fall back to the literal path when they don't.
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Built through serde so the three required fields are all this test has
    /// to know: every other field of `SaveState` carries `#[serde(default)]`,
    /// and one added later shouldn't drag every test with it.
    fn state_with(path: PathBuf, slug: &str, label: &str) -> CliState {
        let save: SaveState = serde_json::from_value(serde_json::json!({
            "local_path": path,
            "game_slug": slug,
            "label": label,
        }))
        .expect("SaveState needs a field this test doesn't set");
        let mut saves = HashMap::new();
        saves.insert("11111111-1111-1111-1111-111111111111".to_string(), save);
        CliState {
            saves,
            ..Default::default()
        }
    }

    /// A detection report carrying one game and its found paths. Built through
    /// serde for the same reason as `state_with`: the report has many fields
    /// with defaults and this test cares about two of them.
    fn report_with(slug: &str, paths: &[PathBuf]) -> crate::detection::DetectionReport {
        serde_json::from_value(serde_json::json!({
            "games": [{
                "slug": slug,
                "display_name": slug,
                "found_paths": paths,
                "confidence": "high",
                "source": "filesystem_heuristic",
            }],
            "catalog_size": 1,
            "steam_apps_found": 0,
            "scanned_at_ms": 0,
        }))
        .expect("DetectionReport needs a field this test doesn't set")
    }

    #[test]
    fn a_missing_folder_is_an_error() {
        let st = state_with("/nonexistent/hoard/doctor".into(), "some-game", "main");
        let f = diagnose(&st);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].code, FindingCode::MissingFolder);
        assert_eq!(f[0].severity, Severity::Error);
        // The command must name the save it is about.
        assert!(f[0].command.contains(&f[0].save_id));
    }

    #[test]
    fn an_empty_tracked_folder_warns() {
        let dir = tempfile::tempdir().unwrap();
        let st = state_with(dir.path().to_path_buf(), "some-game", "main");
        let codes: Vec<_> = diagnose(&st).into_iter().map(|f| f.code).collect();
        assert!(codes.contains(&FindingCode::EmptyFolder));
    }

    #[test]
    fn a_populated_folder_is_quiet() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("save.dat"), b"x").unwrap();
        let st = state_with(dir.path().to_path_buf(), "some-game", "main");
        assert!(diagnose(&st).is_empty(), "{:?}", diagnose(&st));
    }

    #[test]
    fn a_backup_named_folder_warns() {
        let dir = tempfile::tempdir().unwrap();
        let mirror = dir.path().join("Saves.bak");
        std::fs::create_dir(&mirror).unwrap();
        std::fs::write(mirror.join("save.dat"), b"x").unwrap();
        let st = state_with(mirror, "some-game", "main");
        let codes: Vec<_> = diagnose(&st).into_iter().map(|f| f.code).collect();
        assert!(codes.contains(&FindingCode::BackupSuffix));
    }

    /// Two saves for one game are deliberate slots — one for the saves, one
    /// for the config or a debug folder — and the number lives in the label.
    /// Each one's "detection knows another folder" is just its sibling, so the
    /// rule has to stay quiet.
    #[test]
    fn slots_of_the_same_game_are_reported_as_the_user_s_own() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("saves");
        let debug_dir = dir.path().join("debug");
        for d in [&saves_dir, &debug_dir] {
            std::fs::create_dir_all(d).unwrap();
            std::fs::write(d.join("f.dat"), b"x").unwrap();
        }
        let mut st = state_with(saves_dir.clone(), "factorio", "main");
        st.saves.insert(
            "22222222-2222-2222-2222-222222222222".to_string(),
            serde_json::from_value(serde_json::json!({
                "local_path": debug_dir,
                "game_slug": "factorio",
                "label": "2 · debug",
            }))
            .unwrap(),
        );
        let report = report_with("factorio", &[dir.path().join("somewhere-else")]);
        let f: Vec<_> = diagnose_with(&st, Some(&report))
            .into_iter()
            .filter(|f| f.code == FindingCode::AlternatePathKnown)
            .collect();
        assert!(!f.is_empty(), "the user still gets told about it");
        for finding in &f {
            assert_eq!(
                finding.severity,
                Severity::Notice,
                "a second slot is deliberate, not a fault"
            );
            assert!(
                !finding.command.starts_with("hoard save path"),
                "never offer to repoint a deliberate folder: {}",
                finding.command
            );
        }
        // The debug slot is flagged as the user's own doing.
        assert!(
            f.iter().any(|x| x.manual),
            "a numbered slot is the user's own choice"
        );
    }

    /// The regression that made this rule dangerous: a 454 MB Factorio save
    /// with 284 uploaded versions was told to repoint at a Steam emulator's
    /// state folder. A save that is producing versions is watching the right
    /// files, whatever else detection turned up.
    #[test]
    fn a_save_that_uploads_versions_is_never_told_to_repoint() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("save.zip"), b"x").unwrap();
        let mut st = state_with(dir.path().to_path_buf(), "factorio", "main");
        for save in st.saves.values_mut() {
            save.last_version_num = Some(284);
        }
        let report = report_with("factorio", &[dir.path().join("RUNE/427520/remote")]);
        let f: Vec<_> = diagnose_with(&st, Some(&report))
            .into_iter()
            .filter(|f| f.code == FindingCode::AlternatePathKnown)
            .collect();
        assert_eq!(f.len(), 1, "it is still mentioned");
        assert_eq!(f[0].severity, Severity::Notice);
        assert!(
            !f[0].command.contains("save path"),
            "a save storing versions must never be handed a repoint command: {}",
            f[0].command
        );
    }

    /// A path the user pinned by hand is theirs, and is labelled as such.
    #[test]
    fn a_hand_pinned_path_is_marked_manual() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("save.dat"), b"x").unwrap();
        let mut st = state_with(dir.path().to_path_buf(), "planet-s", "main");
        st.manual_paths
            .insert("planet-s".to_string(), dir.path().to_path_buf());
        let report = report_with("planet-s", &[dir.path().join("elsewhere")]);
        let f: Vec<_> = diagnose_with(&st, Some(&report))
            .into_iter()
            .filter(|f| f.code == FindingCode::AlternatePathKnown)
            .collect();
        assert_eq!(f.len(), 1);
        assert!(f[0].manual, "the override must be recognised as the user's");
        assert_eq!(f[0].severity, Severity::Notice);
    }

    /// The shape it does exist for: never uploaded anything, and detection
    /// knows a different folder.
    #[test]
    fn a_save_that_never_uploaded_is_flagged_against_the_detected_folder() {
        let dir = tempfile::tempdir().unwrap();
        let tracked = dir.path().join("install-dir");
        let real = dir.path().join("Saved Games/the-game");
        std::fs::create_dir_all(&tracked).unwrap();
        std::fs::create_dir_all(&real).unwrap();
        std::fs::write(tracked.join("game.exe"), b"x").unwrap();
        let st = state_with(tracked, "planet-s", "main");
        let report = report_with("planet-s", std::slice::from_ref(&real));
        let f: Vec<_> = diagnose_with(&st, Some(&report))
            .into_iter()
            .filter(|f| f.code == FindingCode::AlternatePathKnown)
            .collect();
        assert_eq!(f.len(), 1, "the mistrack shape must still be caught");
        assert_eq!(
            f[0].suggested_path.as_deref(),
            Some(real.display().to_string().as_str())
        );
    }

    #[test]
    fn an_installer_named_save_warns() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("save.dat"), b"x").unwrap();
        let st = state_with(dir.path().to_path_buf(), "some-game", "Codex Sandbox Setup");
        let codes: Vec<_> = diagnose(&st).into_iter().map(|f| f.code).collect();
        assert!(codes.contains(&FindingCode::InstallerNamed));
    }
}
