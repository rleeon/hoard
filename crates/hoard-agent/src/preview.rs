//! What will happen to the folder if this version is restored.
//!
//! Restoring is the scariest operation in the app, and until now it was confirmed
//! blind: the user picked a date and accepted without knowing whether that touched
//! one file or eight hundred. This module answers the question before anything is
//! written (how many files change, which appear, which exist only on disk) without
//! downloading a single byte.
//!
//! It comes free because both halves already existed: the server publishes each
//! version's per-file manifest (path, sha256 and size) and
//! [`crate::backup::walk_source`] is the same walk the backup uses, already
//! filtered of symlinks and of the transient locks an open game leaves. All that
//! is needed is to cross them.
//!
//! The cross runs in two passes so nothing is read twice over: first by path and
//! size, which settles for free everything that appears, disappears or changes
//! size; and only what matches in both path *and* size gets hashed, because that
//! is the only thing that can be either "the same file" or "different content,
//! same size", which is exactly the common case for a fixed-size save.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::Result;
use hoard_core::kernel::fileclass::RestoreGate;
use serde::Serialize;

/// A file in the remote version, in the least it takes to compare. It is built
/// the same way from Cloud's manifest as from the self-hosted listing, which carry
/// the same three facts under different names.
#[derive(Debug, Clone)]
pub struct RemoteFile {
    pub relative_path: String,
    pub size_bytes: u64,
    /// `None` means unknown, not "bad hash". The legacy whole-archive versions
    /// have no per-file digest, and that gap is propagated all the way to the UI
    /// rather than faking a comparison that never happened.
    pub sha256: Option<String>,
}

/// A file already in the destination folder.
#[derive(Debug, Clone)]
pub struct LocalFile {
    pub relative_path: String,
    pub size_bytes: u64,
}

/// What the restore will do to the folder.
#[derive(Debug, Clone, Default, Serialize)]
pub struct RestorePreview {
    /// Files the version brings identical to what is already there: untouched.
    pub unchanged: usize,
    /// Present on both sides with different content: they get overwritten.
    ///
    /// Listed up to [`MAX_LISTED`]; the real total is in [`Self::modified_count`].
    /// Counting by `len()` said "200 files" of an eight-hundred-file save, and it
    /// said it in exactly the sentence the user reads before overwriting their
    /// saves.
    pub modified: Vec<String>,
    /// How many get overwritten in total, listed or not.
    #[serde(default)]
    pub modified_count: usize,
    /// Only in the version: they get created. Listed up to [`MAX_LISTED`].
    pub added: Vec<String>,
    /// How many get created in total, listed or not.
    #[serde(default)]
    pub added_count: usize,
    /// Only on disk. They are not deleted, since a restore writes over rather
    /// than mirroring, but the user deserves to see them: they are the saves made
    /// after the version about to be brought back. Listed up to [`MAX_LISTED`].
    pub local_only: Vec<String>,
    /// How many exist only on disk in total, listed or not.
    #[serde(default)]
    pub local_only_count: usize,
    /// Bytes that have to be written (modified plus added).
    pub bytes_to_write: u64,
    /// `false` when the version publishes no per-file hashes (the legacy
    /// whole-archive ones). Then `modified` and `unchanged` cannot be told apart
    /// and the UI has to say it cannot preview, rather than showing an empty diff
    /// that would read as "nothing changes".
    pub comparable: bool,
}

/// How many paths get listed at most in each list. A save with eight hundred
/// files does not fit in a dialog, and the count already travels in the totals.
const MAX_LISTED: usize = 200;

/// The cross, without touching the disk: it takes both sides already read plus a
/// per-file equality verdict, and decides.
///
/// `same_bytes` is only consulted for paths present on both sides *and* of the
/// same size; for the rest the answer is already settled and nothing needs
/// hashing.
pub fn diff(
    remote: &[RemoteFile],
    local: &[LocalFile],
    mut same_bytes: impl FnMut(&str) -> bool,
) -> RestorePreview {
    let local_by_path: HashMap<&str, &LocalFile> = local
        .iter()
        .map(|f| (f.relative_path.as_str(), f))
        .collect();
    let remote_paths: HashSet<&str> = remote.iter().map(|f| f.relative_path.as_str()).collect();

    let comparable = remote.iter().all(|f| f.sha256.is_some());
    let mut out = RestorePreview {
        comparable,
        ..Default::default()
    };

    for r in remote {
        match local_by_path.get(r.relative_path.as_str()) {
            None => {
                out.bytes_to_write += r.size_bytes;
                out.added_count += 1;
                push_capped(&mut out.added, &r.relative_path);
            }
            Some(l) if l.size_bytes != r.size_bytes => {
                out.bytes_to_write += r.size_bytes;
                out.modified_count += 1;
                push_capped(&mut out.modified, &r.relative_path);
            }
            Some(_) => {
                // Same size, so the content has to be looked at. With no
                // published hash there is no claiming they are equal, so it
                // counts as overwritten, which is the safe side, and
                // `comparable` already warns the count is an upper bound.
                if comparable && same_bytes(&r.relative_path) {
                    out.unchanged += 1;
                } else {
                    out.bytes_to_write += r.size_bytes;
                    out.modified_count += 1;
                    push_capped(&mut out.modified, &r.relative_path);
                }
            }
        }
    }

    for l in local {
        if !remote_paths.contains(l.relative_path.as_str()) {
            out.local_only_count += 1;
            push_capped(&mut out.local_only, &l.relative_path);
        }
    }

    out.modified.sort();
    out.added.sort();
    out.local_only.sort();
    out
}

/// Appends to the list up to the cap. Past the cap it stops listing; the total is
/// carried by the `*_count` fields, which the caller always increments.
fn push_capped(list: &mut Vec<String>, path: &str) {
    if list.len() < MAX_LISTED {
        list.push(path.to_string());
    }
}

/// Reads the destination folder and crosses it with the already-downloaded
/// manifest.
///
/// It hashes only the paths matching the version in both name and size; the rest
/// is settled without opening the file. In the worst case it reads as much as the
/// save occupies, which is the same budget a real restore's dedup against disk
/// already spends.
pub async fn against_disk(
    remote: &[RemoteFile],
    dest: &Path,
    gate: &RestoreGate,
) -> Result<RestorePreview> {
    // What the gate does not let through is not going to be written, so it cannot
    // appear in the preview as though it were. The same decision the restore
    // makes, the single-file save exception included, rather than a copy of it
    // that drifts away.
    let names: Vec<&str> = remote.iter().map(|f| f.relative_path.as_str()).collect();
    let filtered: Vec<RemoteFile>;
    let remote = if crate::restore::is_single_file_snapshot(dest, &names) {
        remote
    } else {
        filtered = remote
            .iter()
            .filter(|f| gate.allows(&f.relative_path))
            .cloned()
            .collect();
        &filtered[..]
    };
    let local: Vec<LocalFile> = match crate::backup::walk_source(dest, &gate.shields) {
        Ok(files) => files
            .into_iter()
            .map(|f| LocalFile {
                relative_path: f.relative_path,
                size_bytes: f.size_bytes,
            })
            .collect(),
        // A folder that does not exist yet (a new machine): everything is an
        // addition. Not an error, but the commonest case of a restore.
        Err(_) => Vec::new(),
    };

    // Only the ambiguous candidates: same path, same size.
    let local_sizes: HashMap<&str, u64> = local
        .iter()
        .map(|f| (f.relative_path.as_str(), f.size_bytes))
        .collect();
    let mut equal: HashSet<String> = HashSet::new();
    for r in remote {
        let Some(sha) = r.sha256.as_deref() else {
            continue;
        };
        if local_sizes.get(r.relative_path.as_str()) != Some(&r.size_bytes) {
            continue;
        }
        let path = dest.join(&r.relative_path);
        // A file that cannot be read (an open game's lock, permissions) counts as
        // different: the preview would rather be over-cautious than promise
        // something will not be touched.
        if let Ok(actual) = crate::backup::hash_file(&path).await {
            if actual.eq_ignore_ascii_case(sha) {
                equal.insert(r.relative_path.clone());
            }
        }
    }

    Ok(diff(remote, &local, |p| equal.contains(p)))
}

/// A version's manifest, wherever it comes from.
///
/// Both halves of the product publish the same three per-file facts under
/// different names, so they are normalised here and the rest of the module never
/// learns which server it is talking to. `presign = false`: this is a query, not a
/// download, and it must not spend bandwidth quota.
pub async fn remote_files(
    client: &crate::api::ApiClient,
    save_id: &str,
    version: i64,
) -> Result<Vec<RemoteFile>> {
    if client.is_cloud().await {
        let manifest = client
            .cloud_version_manifest(save_id, version, false)
            .await?;
        if !manifest.content_addressed {
            // A legacy whole-archive version: there is no per-file listing. It
            // comes back empty and `diff` will mark it not comparable.
            return Ok(Vec::new());
        }
        return Ok(manifest
            .files
            .into_iter()
            .map(|f| RemoteFile {
                relative_path: f.relative_path,
                size_bytes: f.size_bytes.max(0) as u64,
                sha256: (!f.sha256.is_empty()).then_some(f.sha256),
            })
            .collect());
    }
    let detail = client.snapshot_detail(save_id, version).await?;
    Ok(detail
        .files
        .into_iter()
        .map(|f| RemoteFile {
            relative_path: f.relative_path,
            size_bytes: f.size_bytes.max(0) as u64,
            sha256: f.sha256.map(|s| s.to_string()),
        })
        .collect())
}

/// The full preview: it fetches the manifest and crosses it with the disk.
///
/// An empty manifest (a legacy version, or a version with no files) comes out as
/// not comparable, never as "nothing changes".
pub async fn restore_preview(
    client: &crate::api::ApiClient,
    save_id: &str,
    version: i64,
    dest: &Path,
    gate: &RestoreGate,
) -> Result<RestorePreview> {
    let remote = remote_files(client, save_id, version).await?;
    if remote.is_empty() {
        return Ok(RestorePreview {
            comparable: false,
            ..Default::default()
        });
    }
    against_disk(&remote, dest, gate).await
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Past the cap the lists stop growing, so `len()` under-reports. The
    /// counts are what the "N files overwritten" line must use: saying 200 of
    /// a 250-file save, right before overwriting someone's saves, is the one
    /// place a rounded number is not acceptable.
    #[test]
    fn counts_survive_the_listing_cap() {
        let remote: Vec<RemoteFile> = (0..MAX_LISTED + 50)
            .map(|i| RemoteFile {
                relative_path: format!("save{i:04}.dat"),
                size_bytes: 10,
                sha256: Some(format!("{i:064x}")),
            })
            .collect();
        let p = diff(&remote, &[], |_| false);
        assert_eq!(p.added.len(), MAX_LISTED, "the listing is still capped");
        assert_eq!(p.added_count, MAX_LISTED + 50, "the count is the real one");
    }

    fn r(path: &str, size: u64, sha: Option<&str>) -> RemoteFile {
        RemoteFile {
            relative_path: path.to_string(),
            size_bytes: size,
            sha256: sha.map(str::to_string),
        }
    }
    fn l(path: &str, size: u64) -> LocalFile {
        LocalFile {
            relative_path: path.to_string(),
            size_bytes: size,
        }
    }

    #[test]
    fn a_size_change_needs_no_hash_at_all() {
        let remote = vec![r("save.dat", 200, Some("aa"))];
        let local = vec![l("save.dat", 100)];
        let out = diff(&remote, &local, |_| panic!("no debería hashear"));
        assert_eq!(out.modified, vec!["save.dat"]);
        assert_eq!(out.bytes_to_write, 200);
    }

    #[test]
    fn same_size_different_bytes_is_a_modification() {
        // The case that forces a hash: a fixed-size save whose content changes
        // without its size changing.
        let remote = vec![r("slot1.sav", 4096, Some("aa"))];
        let local = vec![l("slot1.sav", 4096)];

        let same = diff(&remote, &local, |_| true);
        assert_eq!(same.unchanged, 1);
        assert!(same.modified.is_empty());
        assert_eq!(same.bytes_to_write, 0);

        let differs = diff(&remote, &local, |_| false);
        assert_eq!(differs.modified, vec!["slot1.sav"]);
        assert_eq!(differs.bytes_to_write, 4096);
    }

    #[test]
    fn files_only_on_disk_are_reported_but_never_counted_as_writes() {
        let remote = vec![r("a.sav", 10, Some("aa"))];
        let local = vec![l("a.sav", 10), l("b.sav", 99)];
        let out = diff(&remote, &local, |_| true);
        assert_eq!(out.local_only, vec!["b.sav"]);
        assert_eq!(out.bytes_to_write, 0);
    }

    #[test]
    fn a_version_without_per_file_hashes_says_so_instead_of_showing_no_changes() {
        // A legacy whole-archive version: with no digests there is no claiming
        // nothing changed, and saying "0 changes" would lie in the worse
        // direction.
        let remote = vec![r("save.dat", 10, None)];
        let local = vec![l("save.dat", 10)];
        let out = diff(&remote, &local, |_| true);
        assert!(!out.comparable);
        assert_eq!(out.unchanged, 0);
        assert_eq!(out.modified, vec!["save.dat"]);
    }

    #[test]
    fn an_empty_destination_makes_everything_an_addition() {
        let remote = vec![r("a.sav", 10, Some("aa")), r("b.sav", 20, Some("bb"))];
        let out = diff(&remote, &[], |_| true);
        assert_eq!(out.added.len(), 2);
        assert_eq!(out.bytes_to_write, 30);
        assert!(out.local_only.is_empty());
    }

    #[tokio::test]
    async fn against_disk_reads_the_folder_and_hashes_only_the_ambiguous_ones() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("igual.sav"), b"hola").unwrap();
        std::fs::write(dir.path().join("distinto.sav"), b"adio").unwrap();
        std::fs::write(dir.path().join("sobra.sav"), b"xxxx").unwrap();

        // sha256("hola")
        let sha_hola = "b221d9dbb083a7f33428d7c2a3c3198ae925614d70210e28716ccaa7cd4ddb79";
        let remote = vec![
            r("igual.sav", 4, Some(sha_hola)),
            r("distinto.sav", 4, Some(sha_hola)),
            r("nuevo.sav", 7, Some("cc")),
        ];

        let out = against_disk(&remote, dir.path(), &RestoreGate::permissive())
            .await
            .unwrap();
        assert_eq!(out.unchanged, 1);
        assert_eq!(out.modified, vec!["distinto.sav"]);
        assert_eq!(out.added, vec!["nuevo.sav"]);
        assert_eq!(out.local_only, vec!["sobra.sav"]);
        assert_eq!(out.bytes_to_write, 4 + 7);
        assert!(out.comparable);
    }
}
