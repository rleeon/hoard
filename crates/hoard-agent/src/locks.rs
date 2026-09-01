//! Is the game writing the save RIGHT NOW?
//!
//! A filesystem probe, independent of the process table: we try to open each of
//! the save's files read-only, and if the OS says another process holds it
//! exclusively, something is writing to it.
//!
//! It is worth having precisely because it does not depend on recognising the
//! game. All of `agent::process_poll`'s session detection starts from matching a
//! process to the save (name, install folder, handles, correlation), and a game
//! that matches nothing shows up as "stopped" while it saves, at which point a
//! backup copies a half-written file and a restore walks over it. This covers
//! that without knowing anything about the game.
//!
//! Only Windows can assert it. On POSIX a read `open()` does not fail because
//! another process is writing (there is no mandatory locking), so there the probe
//! returns `false` and the usual guards decide. On Linux those guards are strong:
//! `agent.rs` already matches through `/proc/<pid>/fd`, the route Windows does not
//! have. Both platforms end up covered, each its own way.

use std::path::Path;

/// How many files get probed at most. The probe runs on every poll tick with a
/// live game, and a folder of 4000 saves (the `swarm` case in the test bench)
/// cannot turn into 4000 `open()` calls every two seconds. One locked file
/// answers the question, and the one the game holds is usually near the front.
const MAX_PROBED_FILES: usize = 64;

/// Maximum depth when looking for files to probe.
const MAX_DEPTH: usize = 3;

/// `true` when some file under `path` is held open exclusively by another
/// process. `path` may be a single file or a folder.
///
/// Conservative when in doubt: any error that is not a declared lock (missing,
/// no permission, deleted mid-probe) counts as NOT locked. Treating "no
/// permission" as a lock is what left the wait loop spinning forever in the
/// original version of this idea.
pub fn any_file_locked(path: &Path) -> bool {
    let mut budget = MAX_PROBED_FILES;
    probe(path, MAX_DEPTH, &mut budget)
}

fn probe(path: &Path, depth: usize, budget: &mut usize) -> bool {
    if *budget == 0 {
        return false;
    }
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return false;
    };
    if meta.is_file() {
        *budget -= 1;
        return is_file_locked(path);
    }
    if !meta.is_dir() || depth == 0 {
        return false;
    }
    let Ok(read) = std::fs::read_dir(path) else {
        return false;
    };
    for entry in read.flatten() {
        if *budget == 0 {
            return false;
        }
        if probe(&entry.path(), depth - 1, budget) {
            return true;
        }
    }
    false
}

/// Windows: open read-only and look at the error.
///
/// Read-only on purpose: saves are never written during a backup, so asking for
/// write access would give false positives on any read-only file. And only the
/// two errors that really mean "another process holds it" count: an ordinary
/// permission denial is NOT a lock, and treating it as one would freeze the save
/// forever.
#[cfg(windows)]
fn is_file_locked(path: &Path) -> bool {
    /// `ERROR_SHARING_VIOLATION`
    const SHARING_VIOLATION: i32 = 32;
    /// `ERROR_LOCK_VIOLATION`
    const LOCK_VIOLATION: i32 = 33;
    match std::fs::File::open(path) {
        Ok(_) => false,
        Err(e) => matches!(
            e.raw_os_error(),
            Some(SHARING_VIOLATION) | Some(LOCK_VIOLATION)
        ),
    }
}

/// POSIX: there is no mandatory locking, so a read `open()` cannot answer the
/// question. It returns `false` rather than inventing an answer.
#[cfg(not(windows))]
fn is_file_locked(_path: &Path) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_quiet_save_folder_is_not_locked() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("slot1.sav"), b"x").unwrap();
        std::fs::create_dir(tmp.path().join("autosave")).unwrap();
        std::fs::write(tmp.path().join("autosave/a.sav"), b"x").unwrap();
        assert!(!any_file_locked(tmp.path()));
    }

    #[test]
    fn a_missing_path_is_not_locked() {
        assert!(!any_file_locked(Path::new("/definitely/not/here")));
    }

    /// A large tree must not turn into thousands of `open()` calls per tick.
    #[test]
    fn the_probe_is_bounded() {
        let tmp = tempfile::tempdir().unwrap();
        for i in 0..500 {
            std::fs::write(tmp.path().join(format!("s{i}.sav")), b"x").unwrap();
        }
        let mut budget = MAX_PROBED_FILES;
        probe(tmp.path(), MAX_DEPTH, &mut budget);
        assert_eq!(budget, 0, "the budget should be spent, no more than that");
    }

    /// On POSIX the probe can assert nothing, and that is right: it must never
    /// brake a backup over a file that merely happens to be open.
    #[cfg(unix)]
    #[test]
    fn posix_never_reports_locked() {
        let tmp = tempfile::tempdir().unwrap();
        let f = tmp.path().join("held.sav");
        std::fs::write(&f, b"x").unwrap();
        let _held = std::fs::OpenOptions::new().write(true).open(&f).unwrap();
        assert!(!any_file_locked(tmp.path()));
    }
}
