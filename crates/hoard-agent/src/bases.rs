//! Whole copies of recent blobs, zstd-compressed (zips raw: zstd can't shrink
//! them), for the next upload to patch against and for a restore to apply a
//! patch to. Kept by sha under the cache
//! dir, with a small index per save folder of which copy each file last went
//! up (or came down) as. A copy is checked against its sha every time it's
//! used: a patch made against a damaged base can't be applied anywhere else.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Files worth a patch: below this a whole zstd blob is already small, and
/// above it base and file together get heavy to hold in memory.
pub(crate) const PATCH_MIN_BYTES: u64 = 1 << 20;
pub(crate) const PATCH_MAX_BYTES: u64 = 512 << 20;

/// The least recently used copies go past this.
const STORE_CAP_BYTES: u64 = 2 << 30;

#[derive(Clone)]
pub(crate) struct BaseStore {
    dir: PathBuf,
}

impl BaseStore {
    pub(crate) fn open() -> Option<Self> {
        let dir = crate::config::CliConfig::cache_dir().ok()?.join("bases");
        std::fs::create_dir_all(&dir).ok()?;
        Some(Self { dir })
    }

    fn file(&self, sha: &str) -> PathBuf {
        self.dir.join(format!("{sha}.zst"))
    }

    fn raw_file(&self, sha: &str) -> PathBuf {
        self.dir.join(format!("{sha}.raw"))
    }

    pub(crate) fn has(&self, sha: &str) -> bool {
        self.file(sha).is_file() || self.raw_file(sha).is_file()
    }

    /// A fresh path in the store to write a compressed copy of `sha` into,
    /// for [`Self::adopt`] once it checks out.
    pub(crate) fn incoming(&self, sha: &str) -> PathBuf {
        self.dir
            .join(format!("{sha}.{}.zst.part", uuid::Uuid::new_v4().simple()))
    }

    /// Take in an already compressed copy of `sha`'s content.
    pub(crate) fn adopt(&self, sha: &str, zst: &Path) {
        let dest = self.file(sha);
        if std::fs::rename(zst, &dest).is_err() {
            let _ = std::fs::copy(zst, &dest);
            let _ = std::fs::remove_file(zst);
        }
    }

    /// The raw content of `sha`, if a copy is here and still hashes to it.
    pub(crate) fn load(&self, sha: &str) -> Option<Vec<u8>> {
        let (path, raw) = match std::fs::read(self.file(sha)) {
            Ok(packed) => (self.file(sha), zstd::decode_all(&packed[..]).ok()),
            Err(_) => (self.raw_file(sha), std::fs::read(self.raw_file(sha)).ok()),
        };
        if raw.is_none() && !path.exists() {
            return None;
        }
        match raw {
            Some(raw) if hex::encode(Sha256::digest(&raw)) == sha => {
                // Touched, so the cap evicts what nobody used.
                let _ = std::fs::File::options()
                    .write(true)
                    .open(&path)
                    .and_then(|f| f.set_modified(SystemTime::now()));
                Some(raw)
            }
            _ => {
                tracing::warn!(sha, "base copy unreadable or damaged, dropped");
                let _ = std::fs::remove_file(&path);
                None
            }
        }
    }

    /// Keep `zst`, a compressed copy of `sha`'s content held in memory.
    pub(crate) fn keep_bytes(&self, sha: &str, zst: &[u8]) {
        let tmp = self.dir.join(format!("{sha}.zst.part"));
        if std::fs::write(&tmp, zst).is_ok() {
            self.adopt(sha, &tmp);
        }
    }

    /// Keep `raw`, `sha`'s content as it is.
    pub(crate) fn keep_raw(&self, sha: &str, raw: &[u8]) {
        let tmp = self.dir.join(format!("{sha}.raw.part"));
        if std::fs::write(&tmp, raw).is_ok() && std::fs::rename(&tmp, self.raw_file(sha)).is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
    }

    /// Copy in a file that went up or came down raw. Not checked here: `load`
    /// checks every copy before it's used.
    pub(crate) async fn keep_file(&self, sha: &str, path: &Path) {
        let tmp = self.dir.join(format!("{sha}.raw.part"));
        if tokio::fs::copy(path, &tmp).await.is_ok()
            && tokio::fs::rename(&tmp, self.raw_file(sha)).await.is_err()
        {
            let _ = tokio::fs::remove_file(&tmp).await;
        }
    }

    pub(crate) fn forget(&self, sha: &str) {
        let _ = std::fs::remove_file(self.file(sha));
        let _ = std::fs::remove_file(self.raw_file(sha));
    }

    /// Evict the least recently used copies past the cap.
    pub(crate) fn prune(&self) {
        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return;
        };
        let mut files: Vec<(SystemTime, u64, PathBuf)> = entries
            .flatten()
            .filter_map(|e| {
                let m = e.metadata().ok()?;
                m.is_file()
                    .then(|| (m.modified().unwrap_or(UNIX_EPOCH), m.len(), e.path()))
            })
            .collect();
        let mut total: u64 = files.iter().map(|f| f.1).sum();
        files.sort();
        for (_, len, path) in files {
            if total <= STORE_CAP_BYTES {
                break;
            }
            let _ = std::fs::remove_file(path);
            total -= len;
        }
    }
}

/// Which stored copy each file of one save folder last went up or came down
/// as, and when.
#[derive(Default, Serialize, Deserialize)]
pub(crate) struct Keyframes {
    #[serde(skip)]
    path: Option<PathBuf>,
    by_path: HashMap<String, (String, i64)>,
    #[serde(skip)]
    dirty: bool,
}

impl Keyframes {
    pub(crate) fn open(root: &Path) -> Self {
        let path = index_path(root);
        let mut kf: Self = path
            .as_ref()
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        kf.path = path;
        kf
    }

    /// What `rel` should patch against: its own last copy, else the newest one
    /// with the same extension (autosaves rotate names), if the store has it.
    pub(crate) fn candidate(&self, rel: &str, store: &BaseStore) -> Option<String> {
        if let Some((sha, _)) = self.by_path.get(rel).filter(|(s, _)| store.has(s)) {
            return Some(sha.clone());
        }
        let ext = extension(rel)?;
        self.by_path
            .iter()
            .filter(|(p, (s, _))| extension(p) == Some(ext) && store.has(s))
            .max_by_key(|(_, (_, at))| *at)
            .map(|(_, (sha, _))| sha.clone())
    }

    pub(crate) fn set(&mut self, rel: &str, sha: &str) {
        let at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        self.by_path.insert(rel.to_string(), (sha.to_string(), at));
        self.dirty = true;
    }

    pub(crate) fn forget_sha(&mut self, sha: &str) {
        let before = self.by_path.len();
        self.by_path.retain(|_, (s, _)| s != sha);
        self.dirty |= self.by_path.len() != before;
    }

    pub(crate) fn save(&self) {
        let Some(path) = &self.path else { return };
        if !self.dirty {
            return;
        }
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(bytes) = serde_json::to_vec(self) {
            let _ = crate::atomic_write::write_atomic(path, &bytes);
        }
    }
}

/// A zip goes up raw, and only patches once inflated.
pub(crate) fn is_zip(path: &Path) -> bool {
    use std::io::Read;
    let mut head = [0u8; 4];
    std::fs::File::open(path)
        .and_then(|mut f| f.read_exact(&mut head))
        .is_ok()
        && &head == b"PK\x03\x04"
}

fn extension(rel: &str) -> Option<&str> {
    let name = rel.rsplit('/').next()?;
    name.rsplit_once('.')
        .map(|(_, e)| e)
        .filter(|e| !e.is_empty())
}

fn index_path(root: &Path) -> Option<PathBuf> {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let key = hex::encode(Sha256::digest(root.to_string_lossy().as_bytes()));
    Some(
        crate::config::CliConfig::cache_dir()
            .ok()?
            .join("bases")
            .join("index")
            .join(format!("{}.json", &key[..32])),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (tempfile::TempDir, BaseStore) {
        let dir = tempfile::tempdir().unwrap();
        let s = BaseStore {
            dir: dir.path().to_path_buf(),
        };
        (dir, s)
    }

    fn keep(s: &BaseStore, content: &[u8]) -> String {
        let sha = hex::encode(Sha256::digest(content));
        let tmp = s.dir.join("incoming");
        std::fs::write(&tmp, zstd::encode_all(content, 3).unwrap()).unwrap();
        s.adopt(&sha, &tmp);
        sha
    }

    #[test]
    fn a_copy_comes_back_only_if_it_still_hashes() {
        let (_d, s) = store();
        let sha = keep(&s, b"the old save");
        assert_eq!(s.load(&sha).as_deref(), Some(&b"the old save"[..]));
        std::fs::write(s.file(&sha), zstd::encode_all(&b"tampered"[..], 3).unwrap()).unwrap();
        assert_eq!(s.load(&sha), None);
        assert!(!s.has(&sha), "a damaged copy is dropped");
    }

    #[tokio::test]
    async fn a_raw_copy_is_checked_like_a_compressed_one() {
        let (d, s) = store();
        let src = d.path().join("save.zip");
        std::fs::write(&src, b"PK\x03\x04 a zip").unwrap();
        let sha = hex::encode(Sha256::digest(b"PK\x03\x04 a zip"));
        s.keep_file(&sha, &src).await;
        assert!(is_zip(&src));
        assert_eq!(s.load(&sha).as_deref(), Some(&b"PK\x03\x04 a zip"[..]));
        std::fs::write(s.raw_file(&sha), b"changed since").unwrap();
        assert_eq!(s.load(&sha), None);
        assert!(!s.has(&sha));
        assert!(src.exists(), "the original is never touched");
    }

    #[test]
    fn a_file_patches_against_its_own_copy_first() {
        let (_d, s) = store();
        let own = keep(&s, b"own");
        let sibling = keep(&s, b"sibling");
        let mut kf = Keyframes::default();
        kf.set("saves/Autosave1.fos", &sibling);
        kf.set("saves/Quicksave.fos", &own);
        assert_eq!(kf.candidate("saves/Quicksave.fos", &s), Some(own.clone()));
        // A new name falls back to the newest copy with its extension.
        assert!(kf.candidate("saves/Autosave2.fos", &s).is_some());
        assert_eq!(kf.candidate("saves/notes.txt", &s), None);
        kf.forget_sha(&own);
        assert_ne!(kf.candidate("saves/Quicksave.fos", &s), Some(own));
    }
}
