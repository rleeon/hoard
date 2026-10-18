//! Per-file SHA-256 remembered between passes, trusted only while `stat` still
//! says the same thing about the file.
//!
//! A backup used to read the whole save twice, plus once more per changed file,
//! however little had changed: 957 GB hashed in 30 days of oct-2026 to upload
//! 114.5 GB of new content. One small file per save folder under the cache dir.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A digest from a file modified this close to its hashing isn't kept across
/// runs: a second write inside the same timestamp tick would look identical
/// (the "racy git" problem; FAT/exFAT tick every 2 s).
const RACY_NS: i64 = 2_000_000_000;

/// Past this, every file is hashed again whatever the cache says.
const REVERIFY_AFTER_SECS: i64 = 24 * 60 * 60;

/// Cache files untouched this long belong to saves gone or long unplayed.
const FORGET_AFTER: std::time::Duration = std::time::Duration::from_secs(30 * 24 * 60 * 60);

/// What `stat` says. A cached digest is reused only while all of it holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Stamp {
    len: u64,
    mtime_ns: i64,
    /// Unix: ctime, which userland can't set back. Windows: creation time.
    change_ns: i64,
    /// Unix: inode. Windows: 0.
    ino: u64,
}

impl Stamp {
    pub(crate) fn of(meta: &std::fs::Metadata) -> Self {
        let mtime_ns = meta.modified().map(unix_ns).unwrap_or(0);
        #[cfg(unix)]
        let (change_ns, ino) = {
            use std::os::unix::fs::MetadataExt;
            (meta.ctime() * 1_000_000_000 + meta.ctime_nsec(), meta.ino())
        };
        #[cfg(not(unix))]
        let (change_ns, ino) = (meta.created().map(unix_ns).unwrap_or(0), 0);
        Self {
            len: meta.len(),
            mtime_ns,
            change_ns,
            ino,
        }
    }
}

fn unix_ns(t: SystemTime) -> i64 {
    t.duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0)
}

fn now_ns() -> i64 {
    unix_ns(SystemTime::now())
}

/// `(len, mtime_ns, change_ns, ino, sha)`: a tuple keeps 15k-file saves small.
type DiskEntry = (u64, i64, i64, u64, String);

#[derive(Serialize, Deserialize, Default)]
struct OnDisk {
    verified_at: i64,
    entries: HashMap<String, DiskEntry>,
}

pub(crate) struct HashCache {
    path: Option<PathBuf>,
    verified_at: i64,
    /// What survives to the next run.
    entries: HashMap<String, (Stamp, String)>,
    /// This run only: racy ones too, so a file hashed for the signature isn't
    /// read again for the manifest a moment later.
    session: HashMap<String, (Stamp, String)>,
    dirty: bool,
}

impl HashCache {
    /// The cache for the files under `root`. Never fails: no cache, or a bad
    /// one, is just an empty cache.
    pub(crate) fn open(root: &Path) -> Self {
        Self::open_at(cache_path(root))
    }

    fn open_at(path: Option<PathBuf>) -> Self {
        let disk: OnDisk = path
            .as_ref()
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        let now = now_ns() / 1_000_000_000;
        let fresh = now - disk.verified_at < REVERIFY_AFTER_SECS;
        if !fresh {
            if let Some(dir) = path.as_ref().and_then(|p| p.parent()) {
                forget_stale(dir);
            }
        }
        let entries = if fresh {
            disk.entries
                .into_iter()
                .map(|(k, (len, mtime_ns, change_ns, ino, sha))| {
                    let stamp = Stamp {
                        len,
                        mtime_ns,
                        change_ns,
                        ino,
                    };
                    (k, (stamp, sha))
                })
                .collect()
        } else {
            HashMap::new()
        };
        Self {
            path,
            verified_at: if fresh { disk.verified_at } else { now },
            entries,
            session: HashMap::new(),
            dirty: !fresh,
        }
    }

    #[cfg(test)]
    pub(crate) fn empty() -> Self {
        Self::open_at(None)
    }

    pub(crate) fn get(&self, rel: &str, stamp: Stamp) -> Option<String> {
        self.session
            .get(rel)
            .or_else(|| self.entries.get(rel))
            .filter(|(s, _)| *s == stamp)
            .map(|(_, sha)| sha.clone())
    }

    /// Records a digest taken between two equal stamps.
    pub(crate) fn put(&mut self, rel: &str, stamp: Stamp, sha: &str) {
        self.session
            .insert(rel.to_string(), (stamp, sha.to_string()));
        if now_ns() - stamp.mtime_ns > RACY_NS {
            self.entries
                .insert(rel.to_string(), (stamp, sha.to_string()));
            self.dirty = true;
        }
    }

    /// Drops what the walk no longer saw (rotated autosaves, deleted slots).
    pub(crate) fn retain(&mut self, keep: impl Fn(&str) -> bool) {
        let before = self.entries.len();
        self.entries.retain(|k, _| keep(k));
        self.dirty |= self.entries.len() != before;
    }

    /// Best effort: a cache that fails to write costs a re-read next time.
    pub(crate) fn save(&self) {
        let Some(path) = &self.path else { return };
        if !self.dirty || (self.entries.is_empty() && !path.exists()) {
            return;
        }
        let disk = OnDisk {
            verified_at: self.verified_at,
            entries: self
                .entries
                .iter()
                .map(|(k, (s, sha))| {
                    (
                        k.clone(),
                        (s.len, s.mtime_ns, s.change_ns, s.ino, sha.clone()),
                    )
                })
                .collect(),
        };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(bytes) = serde_json::to_vec(&disk) {
            if let Err(e) = crate::atomic_write::write_atomic(path, &bytes) {
                tracing::debug!(error = %format!("{e:#}"), "hash cache not saved");
            }
        }
    }
}

fn forget_stale(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let old = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age > FORGET_AFTER);
        if old {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

fn cache_path(root: &Path) -> Option<PathBuf> {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let key = hex::encode(Sha256::digest(root.to_string_lossy().as_bytes()));
    Some(
        crate::config::CliConfig::cache_dir()
            .ok()?
            .join("hashes")
            .join(format!("{}.json", &key[..32])),
    )
}

/// SHA-256 of `abs`, from the cache while its stamp holds; otherwise read,
/// and remembered only if the file didn't move while it was being read.
pub(crate) async fn sha256(cache: &Mutex<HashCache>, rel: &str, abs: &Path) -> Result<String> {
    let before = tokio::fs::metadata(abs).await.ok().map(|m| Stamp::of(&m));
    if let Some(stamp) = before {
        if let Some(sha) = cache.lock().ok().and_then(|c| c.get(rel, stamp)) {
            return Ok(sha);
        }
    }
    let sha = crate::backup::hash_file(abs).await?;
    remember(cache, rel, abs, before, &sha).await;
    Ok(sha)
}

/// The second half of [`sha256`], for callers that read the bytes themselves.
pub(crate) async fn remember(
    cache: &Mutex<HashCache>,
    rel: &str,
    abs: &Path,
    before: Option<Stamp>,
    sha: &str,
) {
    let after = tokio::fs::metadata(abs).await.ok().map(|m| Stamp::of(&m));
    if let (Some(b), Some(a)) = (before, after) {
        if b == a {
            if let Ok(mut c) = cache.lock() {
                c.put(rel, a, sha);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cache() -> HashCache {
        HashCache {
            path: None,
            verified_at: now_ns() / 1_000_000_000,
            entries: HashMap::new(),
            session: HashMap::new(),
            dirty: false,
        }
    }

    fn old_file(dir: &Path, name: &str, body: &[u8]) -> PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, body).unwrap();
        let past = SystemTime::now() - std::time::Duration::from_secs(60);
        let f = std::fs::File::options().write(true).open(&p).unwrap();
        f.set_modified(past).unwrap();
        p
    }

    #[tokio::test]
    async fn a_hit_needs_the_same_stamp() {
        let dir = tempfile::tempdir().unwrap();
        let p = old_file(dir.path(), "a.sav", b"one");
        let c = Mutex::new(cache());
        let first = sha256(&c, "a.sav", &p).await.unwrap();
        assert_eq!(first, crate::backup::hash_file(&p).await.unwrap());
        assert_eq!(c.lock().unwrap().entries.len(), 1, "an old file is kept");

        // Same size, different bytes: the stamp moves (mtime/ctime), so it's read.
        std::fs::write(&p, b"two").unwrap();
        let second = sha256(&c, "a.sav", &p).await.unwrap();
        assert_ne!(first, second);
        assert_eq!(second, crate::backup::hash_file(&p).await.unwrap());
    }

    #[tokio::test]
    async fn a_just_written_file_lives_only_in_the_session() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("hot.sav");
        std::fs::write(&p, b"fresh").unwrap();
        let c = Mutex::new(cache());
        let sha = sha256(&c, "hot.sav", &p).await.unwrap();
        let guard = c.lock().unwrap();
        assert!(guard.entries.is_empty(), "racy: not persisted");
        let stamp = Stamp::of(&std::fs::metadata(&p).unwrap());
        assert_eq!(
            guard.get("hot.sav", stamp),
            Some(sha),
            "but reused this run"
        );
    }

    #[test]
    fn an_expired_cache_starts_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("c.json");
        let stamp = Stamp {
            len: 1,
            mtime_ns: 1,
            change_ns: 1,
            ino: 1,
        };
        let entries = HashMap::from([("a".to_string(), (1, 1, 1, 1, "x".to_string()))]);
        let write = |verified_at: i64| {
            let disk = OnDisk {
                verified_at,
                entries: entries.clone(),
            };
            std::fs::write(&path, serde_json::to_vec(&disk).unwrap()).unwrap();
        };
        write(now_ns() / 1_000_000_000);
        assert_eq!(
            HashCache::open_at(Some(path.clone()))
                .get("a", stamp)
                .as_deref(),
            Some("x")
        );
        write(0);
        assert_eq!(HashCache::open_at(Some(path.clone())).get("a", stamp), None);
    }
}
