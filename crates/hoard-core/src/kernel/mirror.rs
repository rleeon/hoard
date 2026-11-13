//! A save kept in more than one folder on the same machine.
//!
//! One game played in two emulators writes the same files to two directories.
//! The save has one folder that syncs with the server (the primary) and any
//! number of mirrors that are kept equal to it by copying locally.
//!
//! There is no version number between two folders on one disk, so the only
//! honest way to know which one moved is to remember what each looked like the
//! last time they were equal. File dates alone are not that: an emulator that
//! rewrites its old save on start-up stamps stale progress with a fresh date.
//! With a remembered signature per folder, "only this one changed" is a fact,
//! and "both changed" is a question for the user rather than a guess.

/// The two ends of a pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Primary,
    Mirror,
}

impl Side {
    pub fn other(self) -> Self {
        match self {
            Side::Primary => Side::Mirror,
            Side::Mirror => Side::Primary,
        }
    }
}

/// What one pair looks like on this pass. A signature is whatever the shell
/// computes over a folder's files, compared only for equality; `None` in a
/// `now_*` field means the folder holds no save file at all.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Pair<'a> {
    /// The primary's signature the last time the two were equal.
    pub base_primary: Option<&'a str>,
    /// The mirror's, from that same moment.
    pub base_mirror: Option<&'a str>,
    pub now_primary: Option<&'a str>,
    pub now_mirror: Option<&'a str>,
    /// The user's answer to a conflict: this side's content replaces the other.
    pub keep: Option<Side>,
}

/// What to do with a pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Both folders are as they were when last made equal.
    InSync,
    /// Only this side changed, or the other one is empty: copy it over.
    CopyFrom(Side),
    /// Nothing says which side is ahead. The shell compares the bytes: equal
    /// folders only need their signatures remembered, different ones are a
    /// conflict the user settles.
    Compare,
    /// The primary has nothing in it. Filling it is the sync engine's call (it
    /// may be about to restore from the server), so the pair waits.
    WaitForPrimary,
}

pub fn decide(pair: Pair<'_>) -> Verdict {
    let (Some(primary), mirror) = (pair.now_primary, pair.now_mirror) else {
        return match pair.keep {
            // The user pointed at the mirror as the good copy; an empty primary
            // is exactly what that answer fills.
            Some(Side::Mirror) if pair.now_mirror.is_some() => Verdict::CopyFrom(Side::Mirror),
            _ if pair.now_mirror.is_none() => Verdict::InSync,
            _ => Verdict::WaitForPrimary,
        };
    };
    let Some(mirror) = mirror else {
        // An empty mirror has nothing to lose, whatever it held before.
        return Verdict::CopyFrom(Side::Primary);
    };
    if let Some(side) = pair.keep {
        return Verdict::CopyFrom(side);
    }
    let (Some(base_primary), Some(base_mirror)) = (pair.base_primary, pair.base_mirror) else {
        return Verdict::Compare;
    };
    match (primary != base_primary, mirror != base_mirror) {
        (false, false) => Verdict::InSync,
        (true, false) => Verdict::CopyFrom(Side::Primary),
        (false, true) => Verdict::CopyFrom(Side::Mirror),
        (true, true) => Verdict::Compare,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settled<'a>(now_primary: &'a str, now_mirror: &'a str) -> Pair<'a> {
        Pair {
            base_primary: Some("p0"),
            base_mirror: Some("m0"),
            now_primary: Some(now_primary),
            now_mirror: Some(now_mirror),
            keep: None,
        }
    }

    #[test]
    fn untouched_pair_is_in_sync() {
        assert_eq!(decide(settled("p0", "m0")), Verdict::InSync);
    }

    #[test]
    fn the_side_that_changed_is_the_source() {
        assert_eq!(
            decide(settled("p1", "m0")),
            Verdict::CopyFrom(Side::Primary)
        );
        assert_eq!(decide(settled("p0", "m1")), Verdict::CopyFrom(Side::Mirror));
    }

    /// The case file dates get wrong: played in one emulator, then the other
    /// one rewrote its old save. Both moved, so nothing is copied on a guess.
    #[test]
    fn both_changed_is_never_decided_here() {
        assert_eq!(decide(settled("p1", "m1")), Verdict::Compare);
    }

    #[test]
    fn a_pair_never_compared_is_compared_first() {
        let pair = Pair {
            now_primary: Some("p"),
            now_mirror: Some("m"),
            ..Pair::default()
        };
        assert_eq!(decide(pair), Verdict::Compare);
        // Half a memory is no memory.
        let pair = Pair {
            base_primary: Some("p"),
            ..pair
        };
        assert_eq!(decide(pair), Verdict::Compare);
    }

    #[test]
    fn an_empty_mirror_is_filled() {
        let pair = Pair {
            now_primary: Some("p"),
            ..Pair::default()
        };
        assert_eq!(decide(pair), Verdict::CopyFrom(Side::Primary));
        // Also one that was full before: an emulator that rebuilt its profile
        // gets the save back, and an empty folder has nothing to overwrite.
        let pair = Pair {
            base_primary: Some("p"),
            base_mirror: Some("m"),
            ..pair
        };
        assert_eq!(decide(pair), Verdict::CopyFrom(Side::Primary));
    }

    #[test]
    fn an_empty_primary_waits_for_the_engine() {
        let pair = Pair {
            base_primary: Some("p0"),
            base_mirror: Some("m0"),
            now_primary: None,
            now_mirror: Some("m0"),
            keep: None,
        };
        assert_eq!(decide(pair), Verdict::WaitForPrimary);
        assert_eq!(decide(Pair::default()), Verdict::InSync);
    }

    #[test]
    fn the_users_answer_outranks_the_signatures() {
        let pair = Pair {
            keep: Some(Side::Mirror),
            ..settled("p1", "m1")
        };
        assert_eq!(decide(pair), Verdict::CopyFrom(Side::Mirror));
        let pair = Pair {
            keep: Some(Side::Primary),
            ..settled("p1", "m1")
        };
        assert_eq!(decide(pair), Verdict::CopyFrom(Side::Primary));
        // Keeping the mirror fills an empty primary; keeping an empty side
        // copies nothing.
        let pair = Pair {
            now_primary: None,
            now_mirror: Some("m"),
            keep: Some(Side::Mirror),
            ..Pair::default()
        };
        assert_eq!(decide(pair), Verdict::CopyFrom(Side::Mirror));
        let pair = Pair {
            now_primary: None,
            now_mirror: Some("m"),
            keep: Some(Side::Primary),
            ..Pair::default()
        };
        assert_eq!(decide(pair), Verdict::WaitForPrimary);
    }

    #[test]
    fn sides_flip() {
        assert_eq!(Side::Primary.other(), Side::Mirror);
        assert_eq!(Side::Mirror.other(), Side::Primary);
    }
}
