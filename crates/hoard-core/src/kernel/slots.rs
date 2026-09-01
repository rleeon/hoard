//! Which folder of a game is which, and the same one across machines.
//!
//! A game almost never keeps everything in one place. Factorio has the saves in
//! `Factorio/saves` and the settings in `Factorio/config`; a Paradox game splits
//! saves and mods; an emulator separates memory cards from BIOS files. Tracking
//! a single folder per title forces a choice, and making that choice by hand,
//! by pointing at the second folder, is what left the card showing only that one
//! with the real folder nowhere in sight until aug-2026.
//!
//! Here a title stops having *one* folder and gets a numbered list. The number
//! is all Hoard needs to know:
//!
//! * **Slot 1 is always the saved games.** It is what detection proposes and
//!   what counts as "this game is synced".
//! * **From 2 up it is everything else**, and Hoard does not try to guess what.
//!
//! Past that, the number changes *nothing* about how a folder is treated. Every
//! slot backs up and restores down the same path slot 1 does, because "attach
//! several folders so they all sync" is the whole request, and a folder that
//! only ever uploads is not synced, it is a one-way copy.
//!
//! Slots 2+ were briefly made backup-only, out of a worry about one machine's
//! config landing on another's. That worry is real and it is already handled one
//! layer down and far more precisely: [`crate::kernel::fileclass`] marks the
//! device-local files (`graphics.ini`, the settings carrying this monitor's
//! resolution) and a restore does not write *those*, whatever folder they live
//! in. A blanket per-slot rule was a coarser copy of a guard that already
//! existed, and it cost the feature its point: an empty slot 2 on the second
//! machine sat empty while the first uploaded happily into it.
//!
//! ## Why a number and not the folder's name
//!
//! The number is the **identity across machines**, so it has to be something
//! both of them can work out without talking to each other. The path won't do:
//! Factorio's config lives in `%APPDATA%\Factorio\config` on Windows and in
//! `~/.factorio/config` on Linux, so pairing by path pairs nothing. Neither
//! will a name, which needs someone to type the same thing twice.
//!
//! With a number, machine B can see the title has a slot 2 in the cloud that it
//! doesn't have locally, and say "this folder here is my 2" with one click.
//! Hoard never decides what goes with what; it just carries whatever is in each
//! number.
//!
//! And the number has to be **the user's to pick**, not assigned in arrival
//! order. Auto-numbering was tried first and fails at exactly the job the slots
//! exist for: the same folder added on two machines came out as 2 on Windows and
//! 3 on Linux, because by then Linux could already see Windows' 2 taken, so the
//! two never paired up.
//!
//! ## The number, and the name the user gives it
//!
//! Both live in the save's `label`, in the shape `<key>` or `<key> · <name>`:
//! `"2"`, `"2 · Mods"`, `"main"`, `"main · Ironman"`.
//!
//! One field because `label` is the row's identity server-side
//! (`UNIQUE(user_id, game_slug, label)`) *and* the thing the two machines match
//! on, and splitting the name into a second column would mean a migration on
//! every deployment out there for something the label can carry.
//!
//! It does mean the name is **not free text the user types into the key**. The
//! UI edits the name half only and this module composes the label; letting
//! somebody type `"2 - Mods"` by hand is what broke it the first time round,
//! because [`slot_of`] then read no number at all and the folder quietly stopped
//! pairing with the other machine's 2.
//!
//! Because it is one field, the name travels: renaming on one machine patches
//! the row, and the other machine picks it up the next time it lists. The catch
//! that comes with that is in [`slot_of`]'s callers: a client that keeps its own
//! stale copy of the label will upload under the old one and fork the row in two,
//! so local state has to follow the server's label, never the reverse.
//!
//! Slot 1's key is `"main"`, not `"1"`, because `"main"` is what every save
//! tracked so far already carries in the cloud, and renaming those would move
//! their history for no reason at all. The asymmetry is ugly and cheap: both
//! machines compute the same key for the same slot, which is the only thing that
//! has to hold for them to recognise each other.

/// The saved-games slot: what detection proposes, what restores on its own, and
/// what decides whether a game counts as synced.
pub const SAVES: u32 = 1;

/// Historical labels that mean slot 1. `"main"` is what the client has always
/// written; `"default"` is what the server fills in when an upload arrives
/// without one (see the `unwrap_or_else` in `/v1/snapshots`).
const LEGACY_SAVES_LABELS: [&str; 2] = ["main", "default"];

/// Separator between the slot key and the name. Picked because it is not on
/// anybody's keyboard by accident, so a name can never be mistaken for one.
const SEP: &str = " · ";

/// The key half of a slot's label. See the module docs for why slot 1 is
/// `"main"`.
pub fn key_for(slot: u32) -> String {
    if slot == SAVES {
        LEGACY_SAVES_LABELS[0].to_string()
    } else {
        slot.to_string()
    }
}

/// The full `label` for a slot, with the user's name for it when there is one.
///
/// The name is sanitised, not rejected: the separator is stripped out of it so a
/// name can never introduce a second one and make the label ambiguous.
pub fn label_for(slot: u32, name: Option<&str>) -> String {
    let key = key_for(slot);
    match name.map(sanitise_name).filter(|n| !n.is_empty()) {
        Some(n) => format!("{key}{SEP}{n}"),
        None => key,
    }
}

/// A name safe to put in a label: no separator, no leading or trailing space,
/// and no interior runs of whitespace that would render as a gap.
pub fn sanitise_name(name: &str) -> String {
    name.replace(SEP, " ")
        .replace('·', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// The name the user gave this slot, if any.
///
/// Lenient in the same way [`slot_of`] is: whatever separator ended up between
/// the number and the name, the canonical `" · "` or the `" - "` somebody typed
/// by hand, the name is what follows it.
pub fn name_of(label: &str) -> Option<&str> {
    let t = label.trim();
    slot_of(t)?;
    let key_len = if t.starts_with(|c: char| c.is_ascii_digit()) {
        t.chars().take_while(char::is_ascii_digit).count()
    } else {
        LEGACY_SAVES_LABELS
            .iter()
            .find(|l| starts_with_word(t, l))
            .map(|l| l.len())?
    };
    let name = t[key_len..].trim_matches(|c: char| c.is_whitespace() || "-–—·:_|".contains(c));
    (!name.is_empty()).then_some(name)
}

/// Which slot a `label` is, or `None` for one of the older free-form labels.
///
/// `None` is not an error: the label used to be whatever text the user wanted
/// (and the "track this path" button went as far as stuffing the whole path in
/// there). Those rows still exist, still sync, and still render with their own
/// text; they just have no number until someone gives them one.
pub fn slot_of(label: &str) -> Option<u32> {
    let t = label.trim();
    if LEGACY_SAVES_LABELS.iter().any(|l| starts_with_word(t, l)) {
        return Some(SAVES);
    }
    // Read leniently, write canonically. The digits at the front are the slot
    // whatever follows them: `"2"`, `"2 · Mods"`, and the `"2 - shit"` a user
    // typed into the old free-text rename box all mean slot 2. Being strict here
    // cost a real user their pairing: the hand-typed label parsed as no slot at
    // all, the folder dropped out of 2 in silence, and the other machine went on
    // uploading to a row this one no longer recognised as its own.
    let digits: String = t.chars().take_while(char::is_ascii_digit).collect();
    // A name has to be separated from the number, or `"2000AD"` becomes slot
    // 2000. The whole label being digits is the only case that needs no gap.
    let rest = t[digits.len()..].trim_start();
    if digits.is_empty() || (!rest.is_empty() && t.as_bytes()[digits.len()].is_ascii_alphanumeric())
    {
        return None;
    }
    digits.parse::<u32>().ok().filter(|n| *n >= SAVES)
}

/// Does `label` open with the whole word `word`? A word, so `"maintenance"` is
/// not slot 1 while `"main"`, `"main · Ironman"` and the `"main - ironman"` of a
/// hand-typed rename all are.
fn starts_with_word(label: &str, word: &str) -> bool {
    let Some(rest) = label.get(..word.len()) else {
        return false;
    };
    rest.eq_ignore_ascii_case(word)
        && label[word.len()..]
            .chars()
            .next()
            .is_none_or(|c| !c.is_alphanumeric())
}

/// The lowest free number, given the slots a title already occupies.
///
/// Lowest rather than "last + 1" so that deleting slot 2 and adding another
/// folder hands back a 2, instead of leaving a hole and counting on from 4. The
/// holes matter: the number is what the other machine sees, and a list reading
/// `1, 4, 7` tells nobody anything.
pub fn next_free(taken: impl IntoIterator<Item = u32>) -> u32 {
    let mut taken: Vec<u32> = taken.into_iter().collect();
    taken.sort_unstable();
    let mut next = SAVES;
    for n in taken {
        if n == next {
            next += 1;
        } else if n > next {
            break;
        }
    }
    next
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_slot_round_trips_through_the_legacy_label() {
        assert_eq!(label_for(SAVES, None), "main");
        assert_eq!(slot_of("main"), Some(1));
        assert_eq!(slot_of("default"), Some(1));
        assert_eq!(slot_of("1"), Some(1));
    }

    #[test]
    fn extra_slots_are_their_own_number() {
        for n in [2u32, 3, 17] {
            assert_eq!(slot_of(&label_for(n, None)), Some(n), "slot {n}");
        }
    }

    /// Naming a slot must not cost it its number. That is the bug this shape
    /// exists to make impossible.
    #[test]
    fn a_named_slot_keeps_its_number() {
        let label = label_for(2, Some("Mods"));
        assert_eq!(label, "2 · Mods");
        assert_eq!(slot_of(&label), Some(2));
        assert_eq!(name_of(&label), Some("Mods"));

        let named_saves = label_for(SAVES, Some("Ironman"));
        assert_eq!(named_saves, "main · Ironman");
        assert_eq!(slot_of(&named_saves), Some(SAVES));
        assert_eq!(name_of(&named_saves), Some("Ironman"));
    }

    /// A name can't smuggle in a second separator and leave the label
    /// ambiguous, and an empty one is the same as having none.
    #[test]
    fn names_are_sanitised_not_rejected() {
        assert_eq!(label_for(2, Some("Mods · v2")), "2 · Mods v2");
        assert_eq!(label_for(2, Some("  spaced   out ")), "2 · spaced out");
        assert_eq!(label_for(2, Some("   ")), "2");
        assert_eq!(label_for(2, Some("")), "2");
        assert_eq!(name_of("2"), None);
    }

    /// The older free-form labels neither break nor get handed a number.
    #[test]
    fn free_labels_have_no_slot() {
        for label in [
            "ironman",
            "",
            "  ",
            r"C:\Users\rl261\Desktop\saves",
            "0",
            "2000AD",
            "99999999999999999999",
        ] {
            assert_eq!(slot_of(label), None, "{label:?} is not a slot");
        }
    }

    /// Whatever a user typed into the old free-text box, the number in front of
    /// it still names the slot. A real label from aug-2026 was `"2 - shit"`:
    /// read strictly it had no slot, so the folder stopped pairing with the
    /// other machine's 2 and nothing said why.
    #[test]
    fn a_hand_typed_label_keeps_the_number_in_front() {
        for label in ["2 - shit", "2 shit", "2- shit", "2 · shit", "2·shit"] {
            assert_eq!(slot_of(label), Some(2), "{label:?}");
        }
        assert_eq!(slot_of("main - ironman"), Some(SAVES));
        // And the name survives whichever separator got typed.
        for label in ["2 - shit", "2 · shit", "2_shit", "2: shit"] {
            assert_eq!(name_of(label), Some("shit"), "{label:?}");
        }
        assert_eq!(name_of("2"), None);
        assert_eq!(name_of("main"), None);
        assert_eq!(name_of("ironman"), None, "no slot, no name to split off");
    }

    #[test]
    fn next_free_fills_the_lowest_gap() {
        assert_eq!(next_free([]), 1);
        assert_eq!(next_free([1]), 2);
        assert_eq!(next_free([1, 2, 3]), 4);
        assert_eq!(next_free([1, 3]), 2, "fills the hole before growing");
        assert_eq!(next_free([2, 3]), 1);
        assert_eq!(next_free([3, 1, 2]), 4, "input order is irrelevant");
    }
}
