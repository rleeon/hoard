//! Crash-safe whole-file replacement.
//!
//! `std::fs::write` truncates the destination *before* it writes anything, so a
//! process that dies inside that window leaves a 0-byte file behind. For a
//! rebuildable cache that's a nuisance; for the files that record what the user
//! is tracking it's data loss: [`crate::state::load_json`] treats an unparseable
//! file as corrupt, moves it aside and starts from `Default`, i.e. an empty save
//! list.
//!
//! Both failure modes are real: production telemetry carries 917 rows of
//! "prefs.json was corrupt; resetting to defaults" from a single user, every one
//! of them with serde's "EOF while parsing a value at line 1 column 0", the
//! signature of a zero-length file.
//!
//! The write here never truncates the destination. The new contents land in a
//! sibling temp file, are flushed to the platter, and only then replace the
//! target with a rename. A reader always sees either the old file or the new
//! one, whole.

use anyhow::{Context, Result};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Disambiguates temp files written by the same process at the same moment. The
/// pid covers the interesting case, since desktop, daemon and CLI all write
/// these files, and this covers two threads racing inside one of them.
static SEQ: AtomicU64 = AtomicU64::new(0);

/// Replace `path` with `bytes`, atomically. The parent directory is created if
/// it isn't there yet, so a first-run write lands before anything else has
/// touched the state dir.
///
/// The temp file is a *sibling* of the target on purpose: `rename` is only
/// atomic within a single filesystem, and the system temp dir routinely isn't
/// the same one as the user's state dir.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let dir = parent_dir(path);
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;

    let tmp = temp_sibling(path, &dir);
    // Scoped so the handle is closed before the rename: Windows refuses to
    // replace a file that still has an open handle.
    let written = (|| -> std::io::Result<()> {
        let mut f = File::create(&tmp)?;
        f.write_all(bytes)?;
        // Without this the rename can land while the contents are still only in
        // the page cache: the same 0-byte file, with extra steps.
        f.sync_all()
    })();
    if let Err(e) = written {
        let _ = std::fs::remove_file(&tmp);
        return Err(e).with_context(|| format!("writing {}", tmp.display()));
    }

    if let Err(e) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e).with_context(|| format!("renaming {} -> {}", tmp.display(), path.display()));
    }

    // Best-effort: fsync the directory so the rename itself survives a power
    // cut, not just the bytes it points at. Not a thing on Windows, and failing
    // here still leaves us strictly ahead of the truncate-in-place we replaced.
    #[cfg(unix)]
    {
        let _ = File::open(&dir).and_then(|d| d.sync_all());
    }

    Ok(())
}

/// The directory the file lives in. A bare filename has no parent (or an empty
/// one) and `create_dir_all("")` is an error, so both answer ".".
fn parent_dir(path: &Path) -> PathBuf {
    match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => PathBuf::from("."),
    }
}

/// `.<filename>.tmp-<pid>-<seq>`, next to the target.
///
/// Built from the whole file name rather than `with_extension` so that two
/// files differing only in extension can't collide on one temp name. The
/// leading dot plus the non-`.json` suffix keeps a leftover, one per crash
/// between `create` and `rename`, out of every directory listing and
/// extension filter that walks the state dir.
fn temp_sibling(path: &Path, dir: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "hoard".to_string());
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    dir.join(format!(".{name}.tmp-{}-{seq}", std::process::id()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Leftovers would be harmless but they'd also mean the rename never ran,
    /// so every test below asserts on this.
    fn siblings(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn writes_the_file_and_leaves_no_temp_behind() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("prefs.json");

        write_atomic(&path, b"{\"a\":1}").unwrap();

        assert_eq!(std::fs::read(&path).unwrap(), b"{\"a\":1}");
        assert_eq!(siblings(dir.path()), vec!["prefs.json".to_string()]);
    }

    /// The regression this module exists for: the destination is never left
    /// shorter than what we asked for, and a second write replaces the first
    /// one whole instead of overwriting it in place.
    #[test]
    fn replacing_a_longer_file_leaves_no_tail_of_the_old_one() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");

        write_atomic(&path, b"aaaaaaaaaaaaaaaaaaaaaaaa").unwrap();
        write_atomic(&path, b"bb").unwrap();

        assert_eq!(std::fs::read(&path).unwrap(), b"bb");
        assert_eq!(siblings(dir.path()), vec!["state.json".to_string()]);
    }

    /// A 0-byte file on disk is exactly what the old `fs::write` left behind on
    /// a crash, and it has to be recoverable by the next save.
    #[test]
    fn overwrites_a_zero_byte_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("prefs.json");
        std::fs::write(&path, b"").unwrap();

        write_atomic(&path, b"{}").unwrap();

        assert_eq!(std::fs::read(&path).unwrap(), b"{}");
    }

    #[test]
    fn creates_the_parent_directory_on_first_run() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("contexts").join("cloud-abc.json");

        write_atomic(&path, b"{}").unwrap();

        assert_eq!(std::fs::read(&path).unwrap(), b"{}");
    }

    /// Two files whose names differ only in extension must not share a temp
    /// name; `with_extension` would have collapsed them onto one.
    #[test]
    fn temp_names_are_unique_per_target_and_per_call() {
        let dir = Path::new("/state");
        let a = temp_sibling(Path::new("/state/device.json"), dir);
        let b = temp_sibling(Path::new("/state/device.toml"), dir);
        let c = temp_sibling(Path::new("/state/device.json"), dir);

        assert_ne!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.parent().unwrap(), dir);
    }

    #[test]
    fn a_bare_filename_writes_into_the_current_directory() {
        assert_eq!(parent_dir(Path::new("prefs.json")), PathBuf::from("."));
        assert_eq!(
            parent_dir(Path::new("/state/prefs.json")),
            PathBuf::from("/state")
        );
    }
}
