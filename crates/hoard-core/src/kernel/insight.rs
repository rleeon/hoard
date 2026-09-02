//! What a history row says about a version when nobody knows the game.
//!
//! The timeline used to label every row `save_v47 · 2026-08-06 04:16`, which is
//! the one thing the user already knows: that it is a backup, and when. What it
//! never said is *what changed*: which of the 70 Factorio worlds in that folder
//! moved, and by how much. This module derives that from data the server already
//! stores (the per-version file manifests) with no per-game knowledge at all.
//!
//! ## The protagonist
//!
//! A version is a whole folder, but a row can only lead with one thing. The
//! protagonist is the file that best answers "what did you play": among the
//! files that actually changed in this version, the most recently written one
//! that is real player data ([`FileClass::SaveData`]).
//!
//! Its name is the title, because in practice the save's *name* almost never
//! lives inside the binary. It is the file or the folder: `adwdaw.zip`,
//! `SavedGame0/sav.dat`, `Farm_123456/SaveGameInfo`. That makes the generic
//! layer right far more often than it has any business being, and it is the
//! fallback a per-game probe falls back *to* when its parser gives up.
//!
//! Autosaves are pushed down but never excluded: `_autosave1.zip` is genuinely
//! the newest file most of the time, but it names a rotating slot, not a world.
//! When something with a real name changed in the same version, that wins.
//!
//! ## What this deliberately does not do
//!
//! No IO, no game catalogue, no heuristics that need to know which game this
//! is. Everything here is a pure function of the manifest, so it can be
//! computed server-side for versions uploaded years ago by a client that never
//! heard of any of this.

use serde::{Deserialize, Serialize};

use super::fileclass::{classify, FileClass};

/// Current shape of the serialised insight. Bumped when a field changes
/// meaning; readers that see a higher number than they know render what they
/// recognise and ignore the rest.
///
/// A stored insight below this is served as-is but queued to be recomputed:
/// the rules that derive it get better, and a label computed by an older
/// version of them should not outlive the improvement.
///
/// * 2, display names drop the game's own bookkeeping (`murray
///   heath_31852938(m)` → `murray heath`).
pub const SCHEMA: u8 = 2;

/// How many distinct save entries a folder may hold before we stop counting.
/// The count only feeds a "and N more" suffix, so an exact answer past this is
/// worth nothing and a save with one file per map chunk would make it expensive.
const MAX_ENTRIES_COUNTED: usize = 4096;

/// One file of a version, as the manifest knows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileFacts {
    /// Path relative to the save root, `/`-separated.
    pub relative_path: String,
    pub size_bytes: i64,
    /// Source mtime in unix seconds. `None` when the filesystem didn't report
    /// one. Those files can still be picked, they just lose every tie.
    pub modified_at: Option<i64>,
    /// Did this file appear or change content in this version? `false` for
    /// every file when there is no previous version to compare against, which
    /// is why an empty changed-set falls back to considering everything.
    pub changed: bool,
}

/// What kind of value a field holds, so one renderer can draw every game
/// without a component per game. The UI decides the formatting; the value
/// travels as a string so a probe can't be limited by our numeric types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldKind {
    Text,
    Number,
    /// Seconds, rendered as a duration.
    Duration,
    /// Unix seconds, rendered as a date.
    Date,
    Money,
    Badge,
}

/// One labelled fact about the version, filled by a per-game probe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InsightField {
    pub kind: FieldKind,
    /// i18n key when the probe uses a known one, else literal text.
    pub label: String,
    pub value: String,
}

/// Everything a history row can show beyond version number, date and size.
///
/// Serialised into one column, so the field names are short on purpose: this is
/// stored once per version and the biggest accounts hold hundreds of thousands
/// of them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionInsight {
    #[serde(rename = "v")]
    pub schema: u8,
    /// The save's display name, from the protagonist.
    #[serde(rename = "t", default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Free line under the title. Never set by the generic layer.
    #[serde(rename = "s", default, skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    /// Manifest path of the protagonist, so a probe (or a lazy backfill) knows
    /// which blob to fetch without walking the manifest again.
    #[serde(rename = "p", default, skip_serializing_if = "Option::is_none")]
    pub primary_path: Option<String>,
    /// Distinct save entries in the folder: worlds, characters, slots. `1`
    /// means the version is about the one thing the title names.
    #[serde(rename = "n", default, skip_serializing_if = "is_zero_u32")]
    pub entries: u32,
    /// Files added or rewritten since the previous version.
    #[serde(rename = "c", default, skip_serializing_if = "is_zero_u32")]
    pub changed_files: u32,
    /// Files the previous version had and this one doesn't.
    #[serde(rename = "r", default, skip_serializing_if = "is_zero_u32")]
    pub removed_files: u32,
    /// Signed size delta against the previous version, in bytes.
    #[serde(rename = "d", default, skip_serializing_if = "is_zero_i64")]
    pub delta_bytes: i64,
    /// sha256 of the thumbnail blob, once there are thumbnails.
    #[serde(rename = "th", default, skip_serializing_if = "Option::is_none")]
    pub thumb_sha: Option<String>,
    #[serde(rename = "f", default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<InsightField>,
    /// `generic`, or the name of the probe that filled this in.
    #[serde(rename = "src")]
    pub source: String,
}

fn is_zero_u32(n: &u32) -> bool {
    *n == 0
}

fn is_zero_i64(n: &i64) -> bool {
    *n == 0
}

impl VersionInsight {
    /// An insight that knows nothing. Used as the base a probe writes over.
    pub fn empty() -> Self {
        Self {
            schema: SCHEMA,
            title: None,
            subtitle: None,
            primary_path: None,
            entries: 0,
            changed_files: 0,
            removed_files: 0,
            delta_bytes: 0,
            thumb_sha: None,
            fields: Vec::new(),
            source: "generic".into(),
        }
    }

    /// Is there anything here worth storing? A version whose whole insight is
    /// "schema 1, source generic" is a row of JSON that says nothing.
    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.subtitle.is_none()
            && self.entries == 0
            && self.changed_files == 0
            && self.removed_files == 0
            && self.delta_bytes == 0
            && self.thumb_sha.is_none()
            && self.fields.is_empty()
    }
}

/// The file a row leads with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Protagonist {
    pub relative_path: String,
    pub display_name: String,
}

/// Build the generic insight for a version.
///
/// `files` is the version's whole manifest with `changed` already resolved
/// against the previous version; `removed` and `delta_bytes` are the parts of
/// the diff that don't survive in the manifest and so have to be passed in.
/// `shields` are the manifest's save-file patterns, same as [`classify`] takes
/// Empty is fine and only makes the classifier slightly more suspicious.
pub fn generic_insight(
    files: &[FileFacts],
    shields: &[String],
    removed_files: u32,
    delta_bytes: i64,
) -> VersionInsight {
    let mut out = VersionInsight::empty();
    out.changed_files = files.iter().filter(|f| f.changed).count() as u32;
    out.removed_files = removed_files;
    out.delta_bytes = delta_bytes;
    out.entries = count_entries(files, shields);
    if let Some(p) = pick_protagonist(files, shields) {
        out.title = Some(p.display_name);
        out.primary_path = Some(p.relative_path);
    }
    out
}

/// One file as a stored manifest knows it: no `changed` flag, because that is
/// what comparing two of these is for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestFile {
    pub relative_path: String,
    pub sha256: String,
    pub size_bytes: i64,
    pub modified_at: Option<i64>,
}

/// Derive a version's insight from its manifest and the previous version's.
///
/// This is the whole generic layer in one call, and the reason it takes
/// manifests rather than a diff: the server holds both, `save_version_files`
/// being exactly this, so it can compute the insight for a version uploaded long
/// before any of this existed, without asking a client for anything.
///
/// `prev` empty means there is no previous version: nothing is "changed", the
/// diff counters stay at zero, and the protagonist is picked from the whole
/// folder.
pub fn insight_from_manifests(
    cur: &[ManifestFile],
    prev: &[ManifestFile],
    shields: &[String],
) -> VersionInsight {
    let mut before: Vec<(&str, &str, i64)> = prev
        .iter()
        .map(|f| {
            (
                f.relative_path.as_str(),
                f.sha256.as_str(),
                f.size_bytes.max(0),
            )
        })
        .collect();
    before.sort_unstable_by(|a, b| a.0.cmp(b.0));
    let find = |path: &str| {
        before
            .binary_search_by(|probe| probe.0.cmp(path))
            .ok()
            .map(|i| before[i])
    };

    let has_prev = !prev.is_empty();
    let facts: Vec<FileFacts> = cur
        .iter()
        .map(|f| FileFacts {
            relative_path: f.relative_path.clone(),
            size_bytes: f.size_bytes,
            modified_at: f.modified_at,
            // A file with no digest on either side (legacy archive manifests)
            // counts as unchanged: "we don't know" must not read as "rewritten".
            changed: has_prev
                && match find(&f.relative_path) {
                    Some((_, sha, _)) => sha != f.sha256 && !sha.is_empty() && !f.sha256.is_empty(),
                    None => true,
                },
        })
        .collect();

    let removed = if has_prev {
        let mut now: Vec<&str> = cur.iter().map(|f| f.relative_path.as_str()).collect();
        now.sort_unstable();
        before
            .iter()
            .filter(|(path, _, _)| now.binary_search(path).is_err())
            .count() as u32
    } else {
        0
    };

    let delta = if has_prev {
        let after: i64 = cur.iter().map(|f| f.size_bytes.max(0)).sum();
        let before_bytes: i64 = before.iter().map(|(_, _, size)| *size).sum();
        after - before_bytes
    } else {
        0
    };

    generic_insight(&facts, shields, removed, delta)
}

/// Pick the file the row leads with. `None` when the version holds no player
/// data at all (an all-config, all-junk folder).
pub fn pick_protagonist(files: &[FileFacts], shields: &[String]) -> Option<Protagonist> {
    let saves: Vec<&FileFacts> = files
        .iter()
        .filter(|f| classify(&f.relative_path, shields) == FileClass::SaveData)
        .collect();
    if saves.is_empty() {
        return None;
    }

    // Only what moved, when anything did. A version where nothing changed
    // (the first one, or a re-upload) still deserves a title, so there the
    // whole folder is the pool.
    let changed: Vec<&FileFacts> = saves.iter().copied().filter(|f| f.changed).collect();
    let pool = if changed.is_empty() { &saves } else { &changed };

    let best = pool.iter().copied().max_by(|a, b| {
        rank(a)
            .cmp(&rank(b))
            // Ties go to the path that sorts first, so the same manifest always
            // produces the same row no matter what order it arrived in.
            .then_with(|| b.relative_path.cmp(&a.relative_path))
    })?;

    Some(Protagonist {
        relative_path: best.relative_path.clone(),
        display_name: display_name(&best.relative_path),
    })
}

/// Sort key, highest wins: a real name beats a rotating slot, then recency,
/// then size.
fn rank(f: &FileFacts) -> (bool, i64, i64) {
    (
        !is_rotating_slot(file_name(&f.relative_path)),
        f.modified_at.unwrap_or(i64::MIN),
        f.size_bytes,
    )
}

/// How many distinct saves live in this folder.
///
/// The grouping is the first path segment: a folder per world (Cyberpunk,
/// Minecraft) groups by that folder, and loose files (Factorio's `.zip`s,
/// Skyrim's `.ess`) each count as their own. Both are what a player would
/// count.
fn count_entries(files: &[FileFacts], shields: &[String]) -> u32 {
    let mut seen: Vec<&str> = Vec::new();
    for f in files {
        if classify(&f.relative_path, shields) != FileClass::SaveData {
            continue;
        }
        let head = f.relative_path.split('/').next().unwrap_or("");
        if head.is_empty() {
            continue;
        }
        if !seen.contains(&head) {
            seen.push(head);
            if seen.len() >= MAX_ENTRIES_COUNTED {
                break;
            }
        }
    }
    seen.len() as u32
}

fn file_name(rel_path: &str) -> &str {
    rel_path.rsplit('/').next().unwrap_or(rel_path)
}

fn stem_of(name: &str) -> &str {
    match name.rsplit_once('.') {
        Some((stem, _)) if !stem.is_empty() => stem,
        _ => name,
    }
}

/// The name to show for a save file.
///
/// The last path component without its extension, unless that name says nothing
/// about which save this is (`sav.dat`, `level.dat`, `save1`), in which case the
/// folder holding it is the name, which is where Cyberpunk, Stardew and
/// Minecraft keep it.
fn display_name(rel_path: &str) -> String {
    let stem = stem_of(file_name(rel_path));
    let folder = rel_path
        .rsplit_once('/')
        .map(|(parent, _)| file_name(parent))
        .filter(|f| !f.is_empty());
    let picked = match folder {
        // The folder wins ties because it is what groups the save: with
        // `SavedGame0/sav.dat` the player's answer to "which one" is the
        // folder, and `sav.dat` is the same in all of them.
        Some(folder) if name_quality(folder) >= name_quality(stem) => folder,
        _ => stem,
    };
    tidy_name(picked)
}

/// Strip the bookkeeping a game staples onto the name the player chose.
///
/// The Universim writes `murray heath_31852938(m)`, Stardew writes
/// `<farm name>_150130751`: an id and a marker that mean something to the game
/// and nothing to the person reading a row. What is left, `murray heath`,
/// is what they actually named it.
///
/// Two saves can tidy down to the same name. That is fine for a label: the row
/// carries the full path in its tooltip, and a name that repeats is still more
/// use than a number that never meant anything.
fn tidy_name(name: &str) -> String {
    let trimmed = name.trim();
    // A short parenthesised suffix: `(m)`/`(a)` for manual and auto, `(1)` for
    // a copy. Long ones are left alone, since they may be the name.
    let without_marker = match trimmed.strip_suffix(')').and_then(|s| s.rsplit_once('(')) {
        Some((head, marker)) if marker.len() <= 3 && !head.trim().is_empty() => head.trim(),
        _ => trimmed,
    };
    // A trailing run of digits behind a separator, six or more of them: an id
    // or a timestamp. Below six it is a number the player can read and may have
    // chosen (`mipartida-12379`, `world2`, `save01`) and it stays.
    let cleaned = match without_marker.rsplit_once(['_', '-', ' ']) {
        Some((head, tail))
            if tail.len() >= 6 && tail.chars().all(|c| c.is_ascii_digit()) && !head.is_empty() =>
        {
            head
        }
        _ => without_marker,
    };
    let cleaned = cleaned.trim_end_matches([' ', '_', '-']).trim();
    if cleaned.is_empty() {
        trimmed.to_string()
    } else {
        cleaned.to_string()
    }
}

/// How much a name tells you about *which* save this is.
///
/// `2` a name someone chose, `1` a numbered slot (`SavedGame0`, `world2`:
/// generic, but it still picks one out of the folder), `0` a name that only
/// says what kind of file it is and would read the same for every save.
fn name_quality(name: &str) -> u8 {
    if !is_generic_stem(name) {
        return 2;
    }
    if name.chars().any(|c| c.is_ascii_digit()) {
        return 1;
    }
    0
}

/// Names that describe the *kind* of file rather than the save it belongs to.
/// Trailing digits are dropped first, so `save1`, `slot03` and `SaveGame2` all
/// land here.
fn is_generic_stem(stem: &str) -> bool {
    const GENERIC: &[&str] = &[
        "sav",
        "save",
        "saves",
        "savegame",
        "savedgame",
        "savefile",
        "savedata",
        "savegameinfo",
        "data",
        "game",
        "gamedata",
        "gamestate",
        "level",
        "world",
        "player",
        "playerdata",
        "profile",
        "slot",
        "continue",
        "checkpoint",
        "progress",
        "state",
        "main",
        "current",
        "latest",
        "backup",
        "autosave",
        "auto",
        "quicksave",
        "quick",
        "manualsave",
        "manual",
        "exitsave",
        "index",
        "metadata",
        "meta",
        "header",
        "info",
    ];
    let lower = stem.to_ascii_lowercase();
    let trimmed = lower.trim_end_matches(|c: char| c.is_ascii_digit() || c == '_' || c == '-');
    let trimmed = if trimmed.is_empty() { &lower } else { trimmed };
    GENERIC.contains(&trimmed)
}

/// Does this name describe a slot the game overwrites on a timer rather than a
/// save the player named?
fn is_rotating_slot(name: &str) -> bool {
    let stem = stem_of(name).to_ascii_lowercase();
    let squashed: String = stem
        .chars()
        .filter(|c| !c.is_ascii_digit() && *c != '_' && *c != '-' && *c != ' ')
        .collect();
    matches!(
        squashed.as_str(),
        "autosave"
            | "autosav"
            | "auto"
            | "quicksave"
            | "quicksav"
            | "quick"
            | "backup"
            | "bak"
            | "temp"
            | "tmp"
            | "exitsave"
            | "crashsave"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(path: &str, size: i64, mtime: i64, changed: bool) -> FileFacts {
        FileFacts {
            relative_path: path.into(),
            size_bytes: size,
            modified_at: Some(mtime),
            changed,
        }
    }

    #[test]
    fn the_newest_changed_save_leads_the_row() {
        let files = vec![
            f("adwdaw.zip", 8_199_644, 1_000, false),
            f("s21.zip", 8_844_199, 3_000, true),
            f("d2.zip", 8_257_317, 2_000, false),
        ];
        let p = pick_protagonist(&files, &[]).expect("a save is present");
        assert_eq!(p.relative_path, "s21.zip");
        assert_eq!(p.display_name, "s21");
    }

    #[test]
    fn a_named_save_beats_a_newer_autosave() {
        // Factorio rewrites `_autosave1.zip` every minute; it is nearly always
        // the newest file in the folder and it names a slot, not a world.
        let files = vec![
            f("_autosave1.zip", 8_740_833, 9_000, true),
            f("adwdaw.zip", 8_199_644, 8_000, true),
        ];
        let p = pick_protagonist(&files, &[]).expect("a save is present");
        assert_eq!(p.display_name, "adwdaw");
    }

    #[test]
    fn an_autosave_still_leads_when_it_is_all_that_moved() {
        let files = vec![
            f("_autosave1.zip", 8_740_833, 9_000, true),
            f("adwdaw.zip", 8_199_644, 8_000, false),
        ];
        let p = pick_protagonist(&files, &[]).expect("a save is present");
        assert_eq!(p.display_name, "_autosave1");
    }

    #[test]
    fn with_nothing_changed_the_whole_folder_is_the_pool() {
        let files = vec![
            f("old.zip", 10, 1_000, false),
            f("newer.zip", 10, 2_000, false),
        ];
        let p = pick_protagonist(&files, &[]).expect("a save is present");
        assert_eq!(p.display_name, "newer");
    }

    #[test]
    fn a_generic_file_name_takes_the_folders_name() {
        let files = vec![f("SavedGame0/sav.dat", 4_000, 5_000, true)];
        let p = pick_protagonist(&files, &[]).expect("a save is present");
        assert_eq!(p.display_name, "SavedGame0");

        // And the id Stardew hangs off the end is not part of the name.
        let files = vec![f("Farm_123456/SaveGameInfo", 4_000, 5_000, true)];
        let p = pick_protagonist(&files, &[]).expect("a save is present");
        assert_eq!(p.display_name, "Farm");
    }

    #[test]
    fn config_and_junk_never_lead() {
        // The .ini is newer than the save, and it is exactly the file we must
        // not name the row after: it is this machine's, not the player's.
        let files = vec![
            f("graphics.ini", 400, 9_999, true),
            f("Thumbs.db", 4_000, 9_998, true),
            f("mygame.sav", 1_000, 1, true),
        ];
        let p = pick_protagonist(&files, &[]).expect("a save is present");
        assert_eq!(p.display_name, "mygame");
    }

    #[test]
    fn the_games_own_bookkeeping_is_not_part_of_the_name() {
        // The Universim: id + a marker for manual vs auto.
        assert_eq!(tidy_name("murray heath_31852938(m)"), "murray heath");
        // Stardew: the farm's name is what the player typed; the id is not.
        assert_eq!(tidy_name("Roble_150130751"), "Roble");
        // Short numbers are readable and may well be the name.
        assert_eq!(tidy_name("mipartida-12379"), "mipartida-12379");
        assert_eq!(tidy_name("world2"), "world2");
        assert_eq!(tidy_name("_autosave1"), "_autosave1");
        // Nothing to strip.
        assert_eq!(tidy_name("adwdaw"), "adwdaw");
        // Stripping everything would leave the row nameless, so it doesn't.
        assert_eq!(tidy_name("31852938"), "31852938");
        assert_eq!(tidy_name("(m)"), "(m)");
    }

    #[test]
    fn a_folder_with_no_player_data_has_no_protagonist() {
        let files = vec![
            f("settings.ini", 400, 1, true),
            f("Player.log", 40, 2, true),
        ];
        assert!(pick_protagonist(&files, &[]).is_none());
    }

    #[test]
    fn a_shielded_pattern_rescues_a_config_looking_save() {
        // 582 catalogue templates say `.ini` IS the save data.
        let files = vec![f("profile.ini", 400, 1, true)];
        assert!(pick_protagonist(&files, &[]).is_none());
        let shields = vec!["*.ini".to_string()];
        let p = pick_protagonist(&files, &shields).expect("the shield makes it save data");
        assert_eq!(p.display_name, "profile");
    }

    #[test]
    fn entries_count_worlds_not_files() {
        let files = vec![
            f("world1/level.dat", 10, 1, true),
            f("world1/region/r.0.0.mca", 10, 1, true),
            f("world2/level.dat", 10, 1, true),
            f("loose.sav", 10, 1, true),
        ];
        assert_eq!(count_entries(&files, &[]), 3);
    }

    #[test]
    fn the_same_manifest_always_picks_the_same_row() {
        let mut files = vec![
            f("a.sav", 10, 1_000, true),
            f("b.sav", 10, 1_000, true),
            f("c.sav", 10, 1_000, true),
        ];
        let first = pick_protagonist(&files, &[]).unwrap();
        files.reverse();
        let second = pick_protagonist(&files, &[]).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn an_insight_that_says_nothing_knows_it() {
        assert!(VersionInsight::empty().is_empty());
        let out = generic_insight(&[f("a.sav", 10, 1, true)], &[], 0, 0);
        assert!(!out.is_empty());
        assert_eq!(out.title.as_deref(), Some("a"));
        assert_eq!(out.changed_files, 1);
        assert_eq!(out.entries, 1);
    }

    fn m(path: &str, sha: &str, size: i64, mtime: i64) -> ManifestFile {
        ManifestFile {
            relative_path: path.into(),
            sha256: sha.into(),
            size_bytes: size,
            modified_at: Some(mtime),
        }
    }

    #[test]
    fn two_manifests_make_the_whole_row() {
        let prev = vec![
            m("adwdaw.zip", "aaa", 8_000_000, 1_000),
            m("gone.zip", "ccc", 1_000_000, 500),
        ];
        let cur = vec![
            m("adwdaw.zip", "bbb", 8_200_000, 2_000),
            m("_autosave1.zip", "ddd", 500_000, 3_000),
        ];
        let out = insight_from_manifests(&cur, &prev, &[]);
        assert_eq!(out.title.as_deref(), Some("adwdaw"));
        assert_eq!(out.changed_files, 2);
        assert_eq!(out.removed_files, 1);
        assert_eq!(out.delta_bytes, 8_700_000 - 9_000_000);
        assert_eq!(out.entries, 2);
    }

    #[test]
    fn the_first_version_has_no_diff_to_show() {
        let cur = vec![m("adwdaw.zip", "aaa", 8_000_000, 1_000)];
        let out = insight_from_manifests(&cur, &[], &[]);
        assert_eq!(out.title.as_deref(), Some("adwdaw"));
        assert_eq!(out.changed_files, 0);
        assert_eq!(out.removed_files, 0);
        assert_eq!(out.delta_bytes, 0);
    }

    #[test]
    fn a_missing_digest_is_not_a_rewrite() {
        // Legacy whole-archive manifests carry no per-file sha; an empty one on
        // either side must not read as "this file changed".
        let prev = vec![m("a.sav", "", 10, 1)];
        let cur = vec![m("a.sav", "", 10, 2)];
        assert_eq!(insight_from_manifests(&cur, &prev, &[]).changed_files, 0);
    }

    #[test]
    fn the_json_keeps_only_what_it_knows() {
        let mut i = VersionInsight::empty();
        i.title = Some("adwdaw".into());
        i.changed_files = 3;
        let json = serde_json::to_string(&i).unwrap();
        assert_eq!(json, r#"{"v":2,"t":"adwdaw","c":3,"src":"generic"}"#);
        let back: VersionInsight = serde_json::from_str(&json).unwrap();
        assert_eq!(back, i);
    }
}
