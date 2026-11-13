//! A save kept in more than one folder on this machine.
//!
//! The same game played in two emulators writes one set of files to two
//! directories. The save keeps its folder (the primary, the one that talks to
//! the server) and gains any number of second folders that are held equal to
//! it by copying locally: no upload, no download, no quota.
//!
//! Which way to copy is [`hoard_core::kernel::mirror::decide`]'s call, from a
//! signature of each folder remembered the last time the two were equal. When
//! both moved since then nothing is copied: the pair is marked as a conflict
//! and waits for the user to say which folder to keep.
//!
//! Every file a copy replaces is first saved under the conflicts folder, the
//! same one the automatic restore uses, so a wrong call can be undone for as
//! long as that folder keeps it.

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};
use hoard_core::kernel::mirror::{decide, Pair, Side, Verdict};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::backup::{compute_set_signature, walk_source, SourceFilter, UploadFile};
use crate::state::CliState;

/// A folder written this recently is taken to be in use, by an emulator that is
/// saving right now. The pair is left alone and looked at again on a later pass.
const QUIET: Duration = Duration::from_secs(5);

/// Where a second folder stands, for the UI.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// Added, and the engine has not looked at it yet.
    #[default]
    Pending,
    InSync,
    /// Both folders changed and they differ. Nothing is copied until the user
    /// picks one ([`keep`]).
    Conflict,
    /// The folder is not there: a drive that is unplugged, an emulator that was
    /// removed. It is never created; it counts again when it comes back.
    Missing,
}

/// Which folder of a pair the user wants kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Keep {
    Primary,
    Mirror,
}

impl From<Keep> for Side {
    fn from(k: Keep) -> Self {
        match k {
            Keep::Primary => Side::Primary,
            Keep::Mirror => Side::Mirror,
        }
    }
}

/// What the two folders hold, so the conflict dialog can show something to
/// choose by.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sides {
    pub primary_files: usize,
    pub mirror_files: usize,
    #[serde(with = "time::serde::rfc3339::option", default)]
    pub primary_newest: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option", default)]
    pub mirror_newest: Option<OffsetDateTime>,
}

/// One second folder of a save.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Folder {
    pub path: PathBuf,
    #[serde(default)]
    pub status: Status,
    /// The primary's signature and this folder's, both taken when the two were
    /// last equal. One per folder and not one for the pair: copying keeps the
    /// file dates, but two drives do not round them alike, so equal content
    /// can sign differently on each side.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_sig: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mirror_sig: Option<String>,
    /// The answer to a conflict, waiting for the engine to carry it out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keep: Option<Keep>,
    /// Filled in while [`Status::Conflict`] stands.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sides: Option<Sides>,
    /// When the engine last copied between the two.
    #[serde(
        with = "time::serde::rfc3339::option",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub copied_at: Option<OffsetDateTime>,
}

impl Folder {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            status: Status::Pending,
            primary_sig: None,
            mirror_sig: None,
            keep: None,
            sides: None,
            copied_at: None,
        }
    }
}

/// On-disk shape of `mirrors/<context>.json`, keyed by save id.
///
/// Its own file and not a field of the saves file: that one is rewritten whole
/// by the app and by the service, and this one is written by the engine after
/// every copy. Keeping them apart means neither can drop the other's change.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Store {
    #[serde(default)]
    saves: HashMap<String, Vec<Folder>>,
}

/// The store for the active account or server.
pub fn store_path() -> Result<PathBuf> {
    Ok(crate::config::CliConfig::state_dir()?
        .join("mirrors")
        .join(format!("{}.json", crate::state::current_context_id())))
}

fn load(path: &Path) -> Store {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Store::default();
    };
    match serde_json::from_str(&text) {
        Ok(store) => store,
        Err(e) => {
            // Starting empty forgets the links, and a forgotten link only stops
            // copying. The folders themselves are untouched.
            tracing::warn!(error = %e, path = %path.display(), "mirror: store is unreadable; ignoring it");
            Store::default()
        }
    }
}

fn save(path: &Path, store: &Store) -> Result<()> {
    let text = serde_json::to_string_pretty(store).context("serializing mirror store")?;
    crate::atomic_write::write_atomic(path, text.as_bytes())
}

/// The store's own date, which is how the engine notices that the app or the
/// command line changed it.
pub(crate) fn store_stamp() -> Option<SystemTime> {
    let path = store_path().ok()?;
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

// ---- what the app and the command line call

/// Every second folder on this machine, by save. Folders of a save that is no
/// longer tracked are left out.
pub fn list() -> Result<HashMap<String, Vec<Folder>>> {
    let (state, _) = CliState::load_default()?;
    let mut store = load(&store_path()?);
    store
        .saves
        .retain(|id, folders| state.saves.contains_key(id) && !folders.is_empty());
    Ok(store.saves)
}

/// Gives `save_id` a second folder. The engine picks it up on its next tick and
/// makes the two equal, or marks the pair as a conflict when they differ.
pub fn add(save_id: &str, folder: &Path) -> Result<Vec<Folder>> {
    let (state, _) = CliState::load_default()?;
    let path = store_path()?;
    let mut store = load(&path);
    store.saves.retain(|id, _| state.saves.contains_key(id));
    check_new_folder(&state, &store, save_id, folder)?;
    let folders = store.saves.entry(save_id.to_string()).or_default();
    folders.push(Folder::new(folder.to_path_buf()));
    let out = folders.clone();
    save(&path, &store)?;
    tracing::info!(save_id, folder = %folder.display(), "mirror: second folder added");
    Ok(out)
}

/// Stops keeping `folder` equal to the save. The folder and its files stay.
pub fn remove(save_id: &str, folder: &Path) -> Result<Vec<Folder>> {
    let path = store_path()?;
    let mut store = load(&path);
    let folders = store.saves.entry(save_id.to_string()).or_default();
    let before = folders.len();
    folders.retain(|f| !same_folder(&f.path, folder));
    if folders.len() == before {
        anyhow::bail!("{} isn't a second folder of this save.", folder.display());
    }
    let out = folders.clone();
    if out.is_empty() {
        store.saves.remove(save_id);
    }
    save(&path, &store)?;
    tracing::info!(save_id, folder = %folder.display(), "mirror: second folder removed");
    Ok(out)
}

/// Settles a conflict: `keep`'s content replaces the other folder's, and what
/// it replaces goes to the conflicts folder. The engine does the copy.
pub fn keep(save_id: &str, folder: &Path, keep: Keep) -> Result<Vec<Folder>> {
    let path = store_path()?;
    let mut store = load(&path);
    let folders = store
        .saves
        .get_mut(save_id)
        .context("This save has no second folder.")?;
    let entry = folders
        .iter_mut()
        .find(|f| same_folder(&f.path, folder))
        .with_context(|| format!("{} isn't a second folder of this save.", folder.display()))?;
    entry.keep = Some(keep);
    let out = folders.clone();
    save(&path, &store)?;
    tracing::info!(save_id, folder = %folder.display(), ?keep, "mirror: conflict answered");
    Ok(out)
}

/// Forgets what the folders of `save_id` looked like when they were last equal.
/// For when the save's own folder is pointed somewhere else: the remembered
/// signature describes a folder that is no longer the primary, and trusting it
/// would read the new one as "the primary changed" and copy it over the rest.
pub(crate) fn forget_signatures(save_id: &str) {
    let Ok(path) = store_path() else { return };
    let mut store = load(&path);
    let Some(folders) = store.saves.get_mut(save_id) else {
        return;
    };
    for f in folders.iter_mut() {
        f.primary_sig = None;
        f.mirror_sig = None;
        f.status = Status::Pending;
    }
    if let Err(e) = save(&path, &store) {
        tracing::warn!(save_id, error = %format!("{e:#}"), "mirror: couldn't reset the signatures");
    }
}

/// The save that keeps `folder` (or something overlapping it) as a second
/// folder, for "one folder, one game": a second folder is taken, and tracking
/// it as a game of its own would back the same bytes up twice.
pub(crate) fn owner_of(state: &CliState, folder: &Path) -> Option<(String, PathBuf)> {
    let store = load(&store_path().ok()?);
    owner_in(state, &store, folder)
}

fn owner_in(state: &CliState, store: &Store, folder: &Path) -> Option<(String, PathBuf)> {
    store
        .saves
        .iter()
        .filter(|(id, _)| state.saves.contains_key(*id))
        .flat_map(|(id, folders)| folders.iter().map(move |f| (id, f)))
        .find(|(_, f)| crate::detection::paths_overlap(&f.path, folder))
        .map(|(id, f)| (id.clone(), f.path.clone()))
}

/// Every second folder, for the lists of "folders this machine already tracks"
/// that detection and automatic mode filter by.
pub fn claimed_folders() -> Vec<PathBuf> {
    list()
        .map(|by_save| by_save.into_values().flatten().map(|f| f.path).collect())
        .unwrap_or_default()
}

/// What the engine reads of one save's entry in the store.
#[derive(Debug, Default)]
pub(crate) struct Watched {
    pub folders: Vec<PathBuf>,
    /// One of them is waiting for a pass: just added, its signatures
    /// forgotten, or a conflict answered. What a pass writes back never reads
    /// as this, so the engine can tell its own change to the store from one
    /// made by the app or the command line without comparing dates.
    pub asks: bool,
}

/// The folders the engine watches per save. Empty when the store cannot be
/// read: a link that is not seen only stops copying.
pub(crate) fn folders_by_save() -> HashMap<String, Watched> {
    let Ok(path) = store_path() else {
        return HashMap::new();
    };
    load(&path)
        .saves
        .into_iter()
        .map(|(id, folders)| {
            let asks = folders
                .iter()
                .any(|f| f.status == Status::Pending || f.keep.is_some());
            let folders = folders.into_iter().map(|f| f.path).collect();
            (id, Watched { folders, asks })
        })
        .collect()
}

fn same_folder(a: &Path, b: &Path) -> bool {
    crate::detection::path_is_inside(a, b) && crate::detection::path_is_inside(b, a)
}

fn check_new_folder(state: &CliState, store: &Store, save_id: &str, folder: &Path) -> Result<()> {
    let save = state
        .saves
        .get(save_id)
        .context("This save isn't tracked on this machine; link a local folder first.")?;
    if save.local_path.is_file() {
        anyhow::bail!("A save that is a single file can't have a second folder.");
    }
    if !folder.is_dir() {
        anyhow::bail!("{} isn't a folder.", folder.display());
    }
    crate::library::validate_path_shape(folder)?;
    if crate::detection::paths_overlap(&save.local_path, folder) {
        anyhow::bail!(
            "{} is this save's own folder, or sits inside it.",
            folder.display()
        );
    }
    if let Some((_, other)) = state.saves.iter().find(|(id, st)| {
        id.as_str() != save_id && crate::detection::paths_overlap(&st.local_path, folder)
    }) {
        anyhow::bail!(
            "'{}' already tracks {}. Stop tracking it there first, then add the folder here.",
            other.game_slug,
            other.local_path.display()
        );
    }
    if let Some((owner, taken)) = owner_in(state, store, folder) {
        let slug = state
            .saves
            .get(&owner)
            .map(|s| s.game_slug.as_str())
            .unwrap_or("another save");
        anyhow::bail!(
            "{} is already a second folder of '{slug}'.",
            taken.display()
        );
    }
    Ok(())
}

// ---- the engine's pass

/// What one pass over a save's folders did.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Report {
    /// Files were written into the save's own folder, so there is something new
    /// for the sync engine to upload.
    pub wrote_primary: bool,
    pub copied: usize,
    /// Replaced files saved under the conflicts folder before being overwritten.
    pub backed_up: usize,
    pub conflicts: usize,
    /// A folder was in use or a copy failed: look again soon.
    pub retry: bool,
}

/// Everything a pass needs, with nothing read from the process: the store, the
/// filter and the clock come in, so a test runs it on two temp folders.
pub(crate) struct Pass<'a> {
    pub store: &'a Path,
    pub save_id: &'a str,
    pub primary: &'a Path,
    pub filter: &'a SourceFilter,
    /// `<root>/<save id>/<timestamp>/` receives what a copy replaces. Without a
    /// root nothing is ever overwritten: a pair that needs it stays pending.
    pub conflict_root: Option<&'a Path>,
    pub now: SystemTime,
}

/// Brings the second folders of one save level with its primary. Blocking: it
/// walks folders and copies files.
pub(crate) fn run_pass(pass: &Pass<'_>) -> Report {
    let mut report = Report::default();
    let folders = load(pass.store)
        .saves
        .remove(pass.save_id)
        .unwrap_or_default();
    if folders.is_empty() {
        return report;
    }
    let mut done: Vec<Folder> = Vec::with_capacity(folders.len());
    for before in &folders {
        let mut after = before.clone();
        if let Err(e) = level_pair(pass, &mut after, &mut report) {
            tracing::warn!(
                save_id = pass.save_id,
                folder = %before.path.display(),
                error = %format!("{e:#}"),
                "mirror: couldn't bring this folder level; trying again later"
            );
            report.retry = true;
            after = before.clone();
        }
        done.push(after);
    }
    commit(pass, &folders, &done);
    report
}

/// Writes back only what this pass changed, over a fresh read of the store, so
/// a folder the user added or removed while the pass ran is neither lost nor
/// resurrected.
fn commit(pass: &Pass<'_>, before: &[Folder], after: &[Folder]) {
    if before == after {
        return;
    }
    let mut store = load(pass.store);
    let Some(current) = store.saves.get_mut(pass.save_id) else {
        return;
    };
    for (old, new) in before.iter().zip(after) {
        let Some(entry) = current.iter_mut().find(|f| same_folder(&f.path, &old.path)) else {
            continue;
        };
        // The signatures were reset while the pass ran: the save's folder was
        // pointed somewhere else, or this one was removed and added again.
        // What the pass found is about a pair that is no longer this one, and
        // the entry stays pending for the next.
        if entry.primary_sig != old.primary_sig || entry.mirror_sig != old.mirror_sig {
            continue;
        }
        // An answer given while the pass ran is for the next one.
        let keep = if entry.keep == old.keep {
            new.keep
        } else {
            entry.keep
        };
        *entry = Folder {
            keep,
            ..new.clone()
        };
    }
    if let Err(e) = save(pass.store, &store) {
        tracing::warn!(save_id = pass.save_id, error = %format!("{e:#}"), "mirror: couldn't save the pass");
    }
}

fn level_pair(pass: &Pass<'_>, entry: &mut Folder, report: &mut Report) -> Result<()> {
    if !entry.path.is_dir() {
        entry.status = Status::Missing;
        return Ok(());
    }
    // A primary that is not there is a drive that is not mounted, or a folder
    // the engine is about to restore. Either way it is not this pass's to fill.
    if !pass.primary.is_dir() {
        return Ok(());
    }
    if crate::locks::any_file_locked(pass.primary) || crate::locks::any_file_locked(&entry.path) {
        report.retry = true;
        return Ok(());
    }
    let primary = walk_source(pass.primary, pass.filter)?;
    let mirror = walk_source(&entry.path, pass.filter)?;
    if written_recently(&primary, pass.now) || written_recently(&mirror, pass.now) {
        report.retry = true;
        return Ok(());
    }
    let (now_primary, now_mirror) = (signature(&primary), signature(&mirror));
    let verdict = decide(Pair {
        base_primary: entry.primary_sig.as_deref(),
        base_mirror: entry.mirror_sig.as_deref(),
        now_primary: now_primary.as_deref(),
        now_mirror: now_mirror.as_deref(),
        keep: entry.keep.map(Side::from),
    });
    match verdict {
        Verdict::InSync => {
            entry.status = Status::InSync;
            entry.sides = None;
        }
        Verdict::WaitForPrimary => entry.status = Status::Pending,
        Verdict::Compare => {
            if same_content(&primary, &mirror)? {
                entry.primary_sig = now_primary;
                entry.mirror_sig = now_mirror;
                entry.status = Status::InSync;
                entry.sides = None;
            } else {
                if entry.status != Status::Conflict {
                    tracing::warn!(
                        save_id = pass.save_id,
                        folder = %entry.path.display(),
                        "mirror: both folders changed and they differ; waiting for the user"
                    );
                }
                entry.status = Status::Conflict;
                entry.sides = Some(Sides {
                    primary_files: primary.len(),
                    mirror_files: mirror.len(),
                    primary_newest: newest(&primary).map(OffsetDateTime::from),
                    mirror_newest: newest(&mirror).map(OffsetDateTime::from),
                });
                report.conflicts += 1;
            }
        }
        Verdict::CopyFrom(side) => {
            let Some(root) = pass.conflict_root else {
                tracing::warn!(
                    save_id = pass.save_id,
                    "mirror: no conflicts folder configured; not copying"
                );
                return Ok(());
            };
            let (source, target_root) = match side {
                Side::Primary => (&primary, entry.path.as_path()),
                Side::Mirror => (&mirror, pass.primary),
            };
            let backup_dir = root.join(pass.save_id).join(timestamp(pass.now));
            let stats = copy_over(source, target_root, &backup_dir)?;
            // The source's signature is the one from before the copy: if the
            // game wrote to it while the copy ran, it no longer matches and the
            // next pass copies again. Re-reading it here would record that
            // half-seen state as "already equal" and the write would never
            // reach the other folder.
            let target_now = signature(&walk_source(target_root, pass.filter)?);
            match side {
                Side::Primary => {
                    entry.primary_sig = now_primary;
                    entry.mirror_sig = target_now;
                }
                Side::Mirror => {
                    entry.primary_sig = target_now;
                    entry.mirror_sig = now_mirror;
                    report.wrote_primary |= stats.copied > 0;
                }
            }
            entry.status = Status::InSync;
            entry.sides = None;
            entry.keep = None;
            if stats.copied > 0 {
                entry.copied_at = Some(OffsetDateTime::from(pass.now));
                tracing::info!(
                    save_id = pass.save_id,
                    from = ?side,
                    folder = %entry.path.display(),
                    copied = stats.copied,
                    replaced = stats.replaced,
                    bytes = stats.bytes,
                    "mirror: folders brought level"
                );
            }
            report.copied += stats.copied;
            report.backed_up += stats.replaced;
        }
    }
    Ok(())
}

fn signature(files: &[UploadFile]) -> Option<String> {
    (!files.is_empty()).then(|| compute_set_signature(files))
}

fn newest(files: &[UploadFile]) -> Option<SystemTime> {
    files.iter().filter_map(|f| f.modified).max()
}

/// A date in the future is a clock that was wrong when the file was written.
/// It is not "just now", or the pair would wait for ever.
fn written_recently(files: &[UploadFile], now: SystemTime) -> bool {
    newest(files)
        .and_then(|t| now.duration_since(t).ok())
        .is_some_and(|age| age < QUIET)
}

/// Colons are not legal in a Windows path. Same shape as the automatic
/// restore's folders, which the retention sweep already knows how to expire.
fn timestamp(now: SystemTime) -> String {
    OffsetDateTime::from(now)
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "unknown-ts".to_string())
        .replace(':', "-")
}

fn rel_key(rel: &str) -> String {
    if cfg!(windows) {
        rel.to_lowercase()
    } else {
        rel.to_string()
    }
}

/// The two walks hold the same files with the same bytes.
fn same_content(a: &[UploadFile], b: &[UploadFile]) -> Result<bool> {
    if a.len() != b.len() {
        return Ok(false);
    }
    let by_rel: HashMap<String, &UploadFile> =
        b.iter().map(|f| (rel_key(&f.relative_path), f)).collect();
    for f in a {
        let Some(other) = by_rel.get(&rel_key(&f.relative_path)) else {
            return Ok(false);
        };
        if !same_file(f, &other.absolute_path)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Two files of the same size are read and compared, always. Same size and
/// same date is not taken as the same file here, though the backup's skip
/// check makes that bet: emulators write saves of a fixed size (a memory card
/// image), and on a drive that rounds dates to two seconds two different ones
/// can carry the same stamp. Calling those equal would record two folders that
/// differ as level, and the difference would never be seen again.
///
/// In chunks, so a save with large files is never held in memory twice over.
fn same_file(source: &UploadFile, dest: &Path) -> Result<bool> {
    let meta = std::fs::metadata(dest).with_context(|| format!("reading {}", dest.display()))?;
    if meta.len() != source.size_bytes {
        return Ok(false);
    }
    let mut a = std::io::BufReader::new(
        std::fs::File::open(&source.absolute_path)
            .with_context(|| format!("opening {}", source.absolute_path.display()))?,
    );
    let mut b = std::io::BufReader::new(
        std::fs::File::open(dest).with_context(|| format!("opening {}", dest.display()))?,
    );
    let (mut buf_a, mut buf_b) = (vec![0u8; 64 * 1024], vec![0u8; 64 * 1024]);
    loop {
        let n = read_full(&mut a, &mut buf_a)?;
        let m = read_full(&mut b, &mut buf_b)?;
        if n != m || buf_a[..n] != buf_b[..m] {
            return Ok(false);
        }
        if n == 0 {
            return Ok(true);
        }
    }
}

fn read_full(reader: &mut impl Read, buf: &mut [u8]) -> Result<usize> {
    let mut filled = 0;
    while filled < buf.len() {
        match reader.read(&mut buf[filled..])? {
            0 => break,
            n => filled += n,
        }
    }
    Ok(filled)
}

#[derive(Debug, Default)]
struct CopyStats {
    copied: usize,
    replaced: usize,
    bytes: u64,
}

/// Writes every file of `source` into `target_root`. Nothing is deleted there:
/// a file only the target has stays, as it does when a restore merges.
///
/// A file about to be overwritten is copied to `backup_dir` first, and copied
/// rather than moved, so the target never goes without it. If the pass dies
/// halfway, the files already replaced are in the backup and the pair comes
/// back as a conflict: both folders moved since they were last equal.
fn copy_over(source: &[UploadFile], target_root: &Path, backup_dir: &Path) -> Result<CopyStats> {
    let in_prefix = crate::restore::in_wine_prefix(target_root);
    let mut stats = CopyStats::default();
    for file in source {
        let rel = Path::new(&file.relative_path);
        let dest = crate::restore::dest_in(target_root, rel, in_prefix);
        if dest.is_file() {
            if same_file(file, &dest)? {
                continue;
            }
            let saved = backup_dir.join(rel);
            if let Some(parent) = saved.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("creating {}", parent.display()))?;
            }
            copy_synced(&dest, &saved)
                .with_context(|| format!("saving {} before replacing it", dest.display()))?;
            stats.replaced += 1;
        } else if dest.exists() {
            anyhow::bail!(
                "{} is a folder where a save file should go.",
                dest.display()
            );
        } else if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        stats.bytes += place_copy(&file.absolute_path, &dest).with_context(|| {
            format!(
                "copying {} to {}",
                file.absolute_path.display(),
                dest.display()
            )
        })?;
        stats.copied += 1;
    }
    Ok(stats)
}

/// Copies `from` over `to` through a temp file beside it, renamed into place
/// once its bytes are on disk. An emulator that opens the save while the copy
/// runs reads the old file or the new one, never half of each, and a power cut
/// cannot leave an empty file under a name the store already calls level.
fn place_copy(from: &Path, to: &Path) -> std::io::Result<u64> {
    let tmp = crate::restore::temp_beside(to);
    let placed = copy_synced(from, &tmp)
        .and_then(|bytes| crate::restore::rename_over(&tmp, to).map(|()| bytes));
    if placed.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    placed
}

/// By hand rather than `fs::copy`, which would carry a read-only mode over
/// before the file could be flushed. The date travels too: a game that offers
/// "continue" by the newest file would otherwise see every copied save as
/// brand new.
fn copy_synced(from: &Path, to: &Path) -> std::io::Result<u64> {
    let mut src = std::fs::File::open(from)?;
    let mut dst = std::fs::File::create(to)?;
    let bytes = std::io::copy(&mut src, &mut dst)?;
    crate::restore::sync_handle(&dst)?;
    drop(dst);
    let meta = src.metadata()?;
    if let Ok(mtime) = meta.modified() {
        if let Err(e) = filetime::set_file_mtime(to, filetime::FileTime::from_system_time(mtime)) {
            tracing::debug!(to = %to.display(), error = %e, "mirror: couldn't carry the file date over");
        }
    }
    std::fs::set_permissions(to, meta.permissions())?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::SaveState;

    struct Rig {
        _tmp: tempfile::TempDir,
        store: PathBuf,
        primary: PathBuf,
        mirror: PathBuf,
        conflicts: PathBuf,
        filter: SourceFilter,
        now: SystemTime,
    }

    const ID: &str = "6b1f6a52-3f0e-4d55-9a57-0c7d3d2f1a10";

    impl Rig {
        fn new() -> Self {
            let tmp = tempfile::tempdir().unwrap();
            let root = tmp.path().to_path_buf();
            let rig = Self {
                store: root.join("mirrors.json"),
                primary: root.join("emu-a").join("save"),
                mirror: root.join("emu-b").join("save"),
                conflicts: root.join("conflicts"),
                filter: SourceFilter::default(),
                now: SystemTime::now(),
                _tmp: tmp,
            };
            std::fs::create_dir_all(&rig.primary).unwrap();
            std::fs::create_dir_all(&rig.mirror).unwrap();
            let mut store = Store::default();
            store
                .saves
                .insert(ID.to_string(), vec![Folder::new(rig.mirror.clone())]);
            save(&rig.store, &store).unwrap();
            rig
        }

        /// Writes a file dated `mins_ago`, so the pass does not take it for a
        /// save in progress and two writes never share a date.
        fn write(&self, root: &Path, rel: &str, bytes: &[u8], mins_ago: u64) {
            let path = root.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, bytes).unwrap();
            let when = self.now - Duration::from_secs(mins_ago * 60);
            filetime::set_file_mtime(&path, filetime::FileTime::from_system_time(when)).unwrap();
        }

        fn pass(&self) -> Report {
            run_pass(&Pass {
                store: &self.store,
                save_id: ID,
                primary: &self.primary,
                filter: &self.filter,
                conflict_root: Some(&self.conflicts),
                now: self.now,
            })
        }

        fn entry(&self) -> Folder {
            load(&self.store).saves.remove(ID).unwrap().remove(0)
        }

        fn read(&self, root: &Path, rel: &str) -> Vec<u8> {
            std::fs::read(root.join(rel)).unwrap()
        }

        fn backed_up(&self) -> Vec<PathBuf> {
            let mut out = Vec::new();
            let mut stack = vec![self.conflicts.clone()];
            while let Some(dir) = stack.pop() {
                for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
                    if e.path().is_dir() {
                        stack.push(e.path());
                    } else {
                        out.push(e.path());
                    }
                }
            }
            out
        }
    }

    #[test]
    fn equal_folders_are_linked_without_copying() {
        let rig = Rig::new();
        rig.write(&rig.primary, "slot1.sav", b"same", 60);
        rig.write(&rig.mirror, "slot1.sav", b"same", 50);
        let report = rig.pass();
        assert_eq!(report, Report::default());
        let entry = rig.entry();
        assert_eq!(entry.status, Status::InSync);
        assert!(entry.primary_sig.is_some() && entry.mirror_sig.is_some());
        assert!(rig.backed_up().is_empty());
    }

    #[test]
    fn an_empty_second_folder_is_filled() {
        let rig = Rig::new();
        rig.write(&rig.primary, "slot1.sav", b"progress", 60);
        rig.write(&rig.primary, "sub/slot2.sav", b"more", 60);
        let report = rig.pass();
        assert_eq!(report.copied, 2);
        assert!(!report.wrote_primary);
        assert_eq!(rig.read(&rig.mirror, "sub/slot2.sav"), b"more");
        assert_eq!(rig.entry().status, Status::InSync);
    }

    #[test]
    fn a_change_in_one_folder_reaches_the_other_and_the_old_file_is_kept() {
        let rig = Rig::new();
        rig.write(&rig.primary, "slot1.sav", b"v1", 60);
        rig.write(&rig.mirror, "slot1.sav", b"v1", 60);
        rig.pass();

        // Played in the second emulator.
        rig.write(&rig.mirror, "slot1.sav", b"v2 from b", 10);
        let report = rig.pass();
        assert!(report.wrote_primary);
        assert_eq!((report.copied, report.backed_up), (1, 1));
        assert_eq!(rig.read(&rig.primary, "slot1.sav"), b"v2 from b");
        let saved = rig.backed_up();
        assert_eq!(saved.len(), 1);
        assert_eq!(std::fs::read(&saved[0]).unwrap(), b"v1");

        // Then in the first one.
        rig.write(&rig.primary, "slot1.sav", b"v3 from a", 5);
        let report = rig.pass();
        assert!(!report.wrote_primary);
        assert_eq!(rig.read(&rig.mirror, "slot1.sav"), b"v3 from a");
    }

    /// The copy's own writes must not look like a change to copy back.
    #[test]
    fn a_pass_after_a_copy_does_nothing() {
        let rig = Rig::new();
        rig.write(&rig.primary, "slot1.sav", b"v1", 60);
        rig.pass();
        let before = rig.entry();
        assert_eq!(rig.pass(), Report::default());
        assert_eq!(rig.pass(), Report::default());
        assert_eq!(rig.entry(), before);
    }

    /// What file dates get wrong. The second emulator rewrites its old save
    /// after the first one made progress: its file is the newest and its
    /// content the oldest. Nothing may be overwritten on that evidence.
    #[test]
    fn both_changed_is_a_conflict_and_touches_nothing() {
        let rig = Rig::new();
        rig.write(&rig.primary, "slot1.sav", b"v1", 60);
        rig.write(&rig.mirror, "slot1.sav", b"v1", 60);
        rig.pass();

        rig.write(&rig.primary, "slot1.sav", b"real progress", 20);
        rig.write(&rig.mirror, "slot1.sav", b"stale rewrite", 1);
        let report = rig.pass();
        assert_eq!(report.conflicts, 1);
        assert_eq!(report.copied, 0);
        assert_eq!(rig.read(&rig.primary, "slot1.sav"), b"real progress");
        assert_eq!(rig.read(&rig.mirror, "slot1.sav"), b"stale rewrite");
        let entry = rig.entry();
        assert_eq!(entry.status, Status::Conflict);
        assert_eq!(entry.sides.as_ref().map(|s| s.primary_files), Some(1));
        assert!(rig.backed_up().is_empty());

        // It stays a conflict however many times it is looked at.
        assert_eq!(rig.pass().copied, 0);
        assert_eq!(rig.entry().status, Status::Conflict);
    }

    /// Same size, same date, different bytes: a fixed-size save written twice
    /// inside one tick of a coarse clock. Found by running the service on two
    /// real folders, where it came out as "in sync".
    #[test]
    fn same_size_and_date_is_not_the_same_file() {
        let rig = Rig::new();
        rig.write(&rig.primary, "card.bin", b"v1v1v1v1", 60);
        rig.write(&rig.mirror, "card.bin", b"v1v1v1v1", 60);
        rig.pass();
        rig.write(&rig.primary, "card.bin", b"AAAAAAAA", 10);
        rig.write(&rig.mirror, "card.bin", b"BBBBBBBB", 10);
        assert_eq!(rig.pass().conflicts, 1);
        assert_eq!(rig.entry().status, Status::Conflict);

        // And a copy does not skip the file on that evidence either.
        let mut store = load(&rig.store);
        store.saves.get_mut(ID).unwrap()[0].keep = Some(Keep::Primary);
        save(&rig.store, &store).unwrap();
        assert_eq!(rig.pass().copied, 1);
        assert_eq!(rig.read(&rig.mirror, "card.bin"), b"AAAAAAAA");
    }

    #[test]
    fn the_users_answer_settles_a_conflict() {
        let rig = Rig::new();
        rig.write(&rig.primary, "slot1.sav", b"real progress", 20);
        rig.write(&rig.mirror, "slot1.sav", b"stale rewrite", 1);
        assert_eq!(rig.pass().conflicts, 1);

        let mut store = load(&rig.store);
        store.saves.get_mut(ID).unwrap()[0].keep = Some(Keep::Primary);
        save(&rig.store, &store).unwrap();
        let report = rig.pass();
        assert_eq!((report.copied, report.backed_up), (1, 1));
        assert_eq!(rig.read(&rig.mirror, "slot1.sav"), b"real progress");
        let entry = rig.entry();
        assert_eq!(entry.status, Status::InSync);
        assert_eq!(entry.keep, None);
        assert_eq!(entry.sides, None);
        assert_eq!(
            std::fs::read(&rig.backed_up()[0]).unwrap(),
            b"stale rewrite"
        );
    }

    #[test]
    fn folders_made_equal_by_hand_stop_being_a_conflict() {
        let rig = Rig::new();
        rig.write(&rig.primary, "slot1.sav", b"one", 20);
        rig.write(&rig.mirror, "slot1.sav", b"two", 10);
        assert_eq!(rig.pass().conflicts, 1);
        rig.write(&rig.mirror, "slot1.sav", b"one", 8);
        assert_eq!(rig.pass().conflicts, 0);
        assert_eq!(rig.entry().status, Status::InSync);
    }

    #[test]
    fn a_folder_being_written_is_left_for_later() {
        let rig = Rig::new();
        rig.write(&rig.primary, "slot1.sav", b"v1", 60);
        rig.write(&rig.mirror, "slot1.sav", b"v1", 60);
        rig.pass();
        rig.write(&rig.mirror, "slot1.sav", b"saving right now", 0);
        let report = rig.pass();
        assert!(report.retry);
        assert_eq!(report.copied, 0);
        assert_eq!(rig.read(&rig.primary, "slot1.sav"), b"v1");
    }

    #[test]
    fn a_missing_folder_is_reported_and_never_created() {
        let rig = Rig::new();
        rig.write(&rig.primary, "slot1.sav", b"v1", 60);
        std::fs::remove_dir_all(&rig.mirror).unwrap();
        assert_eq!(rig.pass(), Report::default());
        assert_eq!(rig.entry().status, Status::Missing);
        assert!(!rig.mirror.exists());
    }

    #[test]
    fn an_empty_primary_is_not_filled_from_here() {
        let rig = Rig::new();
        rig.write(&rig.mirror, "slot1.sav", b"v1", 60);
        assert_eq!(rig.pass(), Report::default());
        assert!(std::fs::read_dir(&rig.primary).unwrap().next().is_none());
        assert_eq!(rig.entry().status, Status::Pending);
    }

    #[test]
    fn without_a_conflicts_folder_nothing_is_overwritten() {
        let rig = Rig::new();
        rig.write(&rig.primary, "slot1.sav", b"v1", 60);
        rig.write(&rig.mirror, "slot1.sav", b"v1", 60);
        rig.pass();
        rig.write(&rig.primary, "slot1.sav", b"v2", 10);
        let report = run_pass(&Pass {
            store: &rig.store,
            save_id: ID,
            primary: &rig.primary,
            filter: &rig.filter,
            conflict_root: None,
            now: rig.now,
        });
        assert_eq!(report.copied, 0);
        assert_eq!(rig.read(&rig.mirror, "slot1.sav"), b"v1");
    }

    #[test]
    fn a_file_deleted_in_one_folder_stays_in_the_other() {
        let rig = Rig::new();
        for root in [&rig.primary, &rig.mirror] {
            rig.write(root, "slot1.sav", b"v1", 60);
            rig.write(root, "slot2.sav", b"old run", 60);
        }
        rig.pass();
        std::fs::remove_file(rig.primary.join("slot2.sav")).unwrap();
        rig.write(&rig.primary, "slot3.sav", b"new run", 10);
        let report = rig.pass();
        assert_eq!(report.copied, 1);
        assert_eq!(rig.read(&rig.mirror, "slot3.sav"), b"new run");
        assert_eq!(rig.read(&rig.mirror, "slot2.sav"), b"old run");
        assert_eq!(rig.entry().status, Status::InSync);
    }

    #[test]
    fn an_answer_given_during_a_pass_survives_it() {
        let rig = Rig::new();
        rig.write(&rig.primary, "slot1.sav", b"one", 20);
        rig.write(&rig.mirror, "slot1.sav", b"two", 10);
        let before = vec![rig.entry()];
        let mut after = before.clone();
        after[0].status = Status::Conflict;
        // The user answers between the pass reading the store and writing it.
        let mut store = load(&rig.store);
        store.saves.get_mut(ID).unwrap()[0].keep = Some(Keep::Mirror);
        save(&rig.store, &store).unwrap();
        commit(
            &Pass {
                store: &rig.store,
                save_id: ID,
                primary: &rig.primary,
                filter: &rig.filter,
                conflict_root: Some(&rig.conflicts),
                now: rig.now,
            },
            &before,
            &after,
        );
        let entry = rig.entry();
        assert_eq!(entry.keep, Some(Keep::Mirror));
        assert_eq!(entry.status, Status::Conflict);
    }

    #[test]
    fn signatures_reset_during_a_pass_are_not_brought_back() {
        let rig = Rig::new();
        rig.write(&rig.primary, "slot1.sav", b"v1", 60);
        rig.write(&rig.mirror, "slot1.sav", b"v1", 60);
        rig.pass();
        let before = vec![rig.entry()];
        let mut after = before.clone();
        after[0].copied_at = Some(OffsetDateTime::from(rig.now));
        // The save's own folder is pointed elsewhere while the pass runs.
        let mut store = load(&rig.store);
        let entry = &mut store.saves.get_mut(ID).unwrap()[0];
        (entry.primary_sig, entry.mirror_sig, entry.status) = (None, None, Status::Pending);
        save(&rig.store, &store).unwrap();
        commit(
            &Pass {
                store: &rig.store,
                save_id: ID,
                primary: &rig.primary,
                filter: &rig.filter,
                conflict_root: Some(&rig.conflicts),
                now: rig.now,
            },
            &before,
            &after,
        );
        let entry = rig.entry();
        assert_eq!(entry.primary_sig, None);
        assert_eq!(entry.status, Status::Pending);
    }

    #[test]
    fn a_copy_replaces_a_read_only_file_and_leaves_no_temp_behind() {
        let rig = Rig::new();
        rig.write(&rig.primary, "slot1.sav", b"v1", 60);
        rig.write(&rig.mirror, "slot1.sav", b"v1", 60);
        rig.pass();
        let target = rig.mirror.join("slot1.sav");
        let mut perms = std::fs::metadata(&target).unwrap().permissions();
        perms.set_readonly(true);
        std::fs::set_permissions(&target, perms).unwrap();
        // The mark is not a change: the signature does not read it.
        rig.write(&rig.primary, "slot1.sav", b"v2 longer", 10);
        let report = rig.pass();
        assert_eq!((report.copied, report.backed_up), (1, 1));
        assert_eq!(rig.read(&rig.mirror, "slot1.sav"), b"v2 longer");
        let names: Vec<_> = std::fs::read_dir(&rig.mirror)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, ["slot1.sav"]);
    }

    fn tracked(path: &Path, slug: &str) -> SaveState {
        serde_json::from_value(serde_json::json!({
            "local_path": path,
            "game_slug": slug,
            "label": "main",
        }))
        .unwrap()
    }

    #[test]
    fn a_second_folder_has_to_be_free() {
        let tmp = tempfile::tempdir().unwrap();
        let (a, b, c) = (
            tmp.path().join("a"),
            tmp.path().join("b"),
            tmp.path().join("c"),
        );
        for d in [&a, &b, &c] {
            std::fs::create_dir_all(d.join("inner")).unwrap();
        }
        let mut state = CliState::default();
        state.saves.insert("one".into(), tracked(&a, "emu-a-title"));
        state.saves.insert("two".into(), tracked(&b, "emu-b-title"));
        let mut store = Store::default();

        let err = |r: Result<()>| format!("{:#}", r.unwrap_err());
        assert!(err(check_new_folder(&state, &store, "gone", &c)).contains("isn't tracked"));
        assert!(err(check_new_folder(&state, &store, "one", &a)).contains("own folder"));
        assert!(
            err(check_new_folder(&state, &store, "one", &a.join("inner"))).contains("own folder")
        );
        assert!(err(check_new_folder(&state, &store, "one", &b)).contains("already tracks"));
        assert!(err(check_new_folder(
            &state,
            &store,
            "one",
            &tmp.path().join("nope")
        ))
        .contains("isn't a folder"));
        check_new_folder(&state, &store, "one", &c).unwrap();

        store
            .saves
            .insert("one".into(), vec![Folder::new(c.clone())]);
        assert!(
            err(check_new_folder(&state, &store, "two", &c)).contains("already a second folder")
        );
        assert!(
            err(check_new_folder(&state, &store, "two", &c.join("inner")))
                .contains("already a second folder")
        );
        assert_eq!(
            owner_in(&state, &store, &c).map(|(id, _)| id),
            Some("one".to_string())
        );

        // A save that is no longer tracked holds nothing.
        state.saves.remove("one");
        assert_eq!(owner_in(&state, &store, &c), None);
    }
}
