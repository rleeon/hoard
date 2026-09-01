//! The conflict-aware half of auto-restore, with the IO taken out: the shell
//! samples the mtimes and carries out the resolution, the kernel only decides
//! who wins and what to do about it.

use std::time::{Duration, SystemTime};

/// One second covers FAT32, which rounds to two, and clock skew on network
/// shares. A near-tie must not read as "local is newer".
const MTIME_TOLERANCE: Duration = Duration::from_secs(1);

/// True only when the local mtime is more than [`MTIME_TOLERANCE`] newer than
/// the remote one. Conservative when something is missing: if either mtime is
/// unknown the remote wins, because a snapshot's authority comes from
/// timestamps committed on the server, which beat a local filesystem with
/// opinions of its own.
pub fn local_wins_on_mtime(local: Option<SystemTime>, remote: Option<SystemTime>) -> bool {
    match (local, remote) {
        (Some(l), Some(r)) => match l.duration_since(r) {
            Ok(age) => age > MTIME_TOLERANCE,
            Err(_) => false,
        },
        _ => false,
    }
}

/// What to do with a file that exists on both sides with different bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictResolution {
    KeepLocal,
    BackupThenTakeRemote,
}

/// `local_wins` comes from [`local_wins_on_mtime`]; `has_backup_dir` says
/// whether a conflict dir is configured. The hard fallback is keeping the local
/// copy: with nowhere to stash it we never destroy the user's data, however new
/// the remote looks.
pub fn resolve_conflict(local_wins: bool, has_backup_dir: bool) -> ConflictResolution {
    if local_wins || !has_backup_dir {
        ConflictResolution::KeepLocal
    } else {
        ConflictResolution::BackupThenTakeRemote
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(secs: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(secs)
    }

    #[test]
    fn local_wins_only_when_clearly_newer() {
        assert!(local_wins_on_mtime(Some(t(110)), Some(t(100))));
        // Inside the one-second tolerance a tie goes to the remote.
        assert!(!local_wins_on_mtime(Some(t(101)), Some(t(100))));
        assert!(!local_wins_on_mtime(Some(t(100)), Some(t(100))));
        assert!(!local_wins_on_mtime(Some(t(100)), Some(t(110))));
    }

    #[test]
    fn unknown_mtime_hands_it_to_remote() {
        assert!(!local_wins_on_mtime(None, Some(t(100))));
        assert!(!local_wins_on_mtime(Some(t(100)), None));
        assert!(!local_wins_on_mtime(None, None));
    }

    #[test]
    fn conflict_policy_matches_the_original_branches() {
        assert_eq!(resolve_conflict(true, true), ConflictResolution::KeepLocal);
        assert_eq!(resolve_conflict(true, false), ConflictResolution::KeepLocal);
        assert_eq!(
            resolve_conflict(false, true),
            ConflictResolution::BackupThenTakeRemote
        );
        // Remote wins but there is nowhere to stash the local copy, so keep it.
        assert_eq!(
            resolve_conflict(false, false),
            ConflictResolution::KeepLocal
        );
    }
}
