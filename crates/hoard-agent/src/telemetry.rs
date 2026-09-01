//! The contradictions: where detection got it wrong and what the human did to
//! fix it.
//!
//! This is the data that teaches something. "Paths detected correctly" is where
//! there is no problem and is what generates the most volume; what fixes the
//! pipeline is the opposite case, and until now it only arrived when somebody
//! bothered to write in on Discord.
//!
//! There is no new plumbing: these are ordinary `tracing` events on a fixed
//! `target` ([`TELEMETRY_TARGET`]), so they travel through `logship` like
//! everything else, path redaction included, and get queried with a
//! `where target = ...`. That is why they are INFO rather than DEBUG: the
//! process filter (`info` in the service) would throw a DEBUG away before any
//! layer saw it.
//!
//! Four fields per event, which is what is needed: the verdict, the game, the
//! shape of the path, and, off the line and once per batch, the app version. And
//! nothing that identifies the person: `logship::redact` replaces the profile
//! segment before the line enters the channel.

use std::collections::HashSet;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

use hoard_core::wire::TELEMETRY_TARGET;

/// Is this the first time this process has seen this contradiction?
///
/// The two that come from the engine, [`no_snapshots`] and [`rejected_root`],
/// repeat on every sweep: a folder pointed at the wrong place is still wrong ten
/// minutes later. Without this, one broken save puts a couple of thousand rows
/// into the 14 days of retention and turns the signal into the dump this module
/// exists not to be. Once per service start is exactly what is needed: the fact
/// is "this happens to this game", not how many times it was retried.
///
/// The other three are user actions and are not filtered: somebody repointing the
/// same game twice is information, not noise.
fn first_time(key: String) -> bool {
    static SEEN: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    SEEN.get_or_init(Default::default)
        .lock()
        .map(|mut seen| seen.insert(key))
        .unwrap_or(true)
}

/// The user stopped tracking a path the pipeline had proposed.
pub fn untracked(slug: &str, path: &Path) {
    tracing::info!(
        target: TELEMETRY_TARGET,
        verdict = "untracked",
        slug = %slug,
        path = %path.display(),
        "telemetry: the user untracked this folder"
    );
}

/// The user repointed a save: from where, to where. It is the richest correction
/// there is, saying both what failed and what the right answer was.
pub fn repointed(slug: &str, from: &Path, to: &Path) {
    tracing::info!(
        target: TELEMETRY_TARGET,
        verdict = "repointed",
        slug = %slug,
        path = %from.display(),
        to = %to.display(),
        "telemetry: the user re-pointed this save"
    );
}

/// The user pinned a game's folder by hand (`manual_paths`). It is the direct
/// contradiction of the heuristic: what it proposed was no good and there is a
/// right answer.
///
/// The folder goes in `to` rather than `path`, as in [`repointed`]: in both,
/// `path` is "from where" and `to` is "to where", and here what we know is the
/// destination. Putting it in `path` would make the panel draw it in the
/// bad-path column, the right data in the box that means the opposite, which is
/// worse than not having it.
pub fn manual_path(slug: &str, to: &Path) {
    tracing::info!(
        target: TELEMETRY_TARGET,
        verdict = "manual_path",
        slug = %slug,
        to = %to.display(),
        "telemetry: the user overrode detection for this game"
    );
}

/// A tracked save that has never produced a snapshot and is still empty: almost
/// always the folder is not where the game saves. Once per run and save (see
/// [`first_time`]), since the engine retries it on every sweep.
pub fn no_snapshots(slug: &str, path: &Path) {
    if !first_time(format!("no_snapshots|{slug}|{}", path.display())) {
        return;
    }
    tracing::info!(
        target: TELEMETRY_TARGET,
        verdict = "no_snapshots",
        slug = %slug,
        path = %path.display(),
        "telemetry: tracked folder has never produced a snapshot"
    );
}

/// A root `junkdirs::dangerous_sync_root` refused, with the reason. Says what the
/// pipeline is proposing that it should not. Once per run and root, for the same
/// reason as [`no_snapshots`].
pub fn rejected_root(slug: &str, path: &Path, reason: &str) {
    if !first_time(format!("rejected_root|{slug}|{}", path.display())) {
        return;
    }
    tracing::info!(
        target: TELEMETRY_TARGET,
        verdict = "rejected_root",
        slug = %slug,
        path = %path.display(),
        reason = %reason,
        "telemetry: refused an impossible sync root"
    );
}

/// An emulator's save root the walk found and refused: a container of one folder
/// per title, with no title inside it yet. Says which emulator, which is the
/// whole point, because the row is a line for the catalog to answer: a root that
/// never fills up usually means the template points at the wrong per-install
/// identifier (rpcs3's `00000001` profile is only the first one).
///
/// Once per run and root, for the same reason as [`no_snapshots`]: the walk runs
/// again every sweep and the root is still there.
pub fn emulator_root_skipped(emulator: &str, path: &Path) {
    if !first_time(format!("emulator_root|{emulator}|{}", path.display())) {
        return;
    }
    tracing::info!(
        target: TELEMETRY_TARGET,
        verdict = "emulator_root_skipped",
        slug = %emulator,
        path = %path.display(),
        "telemetry: emulator save root has no title inside it"
    );
}

/// A game we found no cover art for, down every path we know.
///
/// The only verdict here that doesn't come from detection, and it lives in this
/// module for the same reasons the others do: same target, same dedupe, same
/// query. It is emitted by the desktop (`commands::covers`), not by the engine.
///
/// What makes the row actionable is the `slug`, which is the key of `covers.json`,
/// so a row that arrives is a line to fill in. `source` says why there is
/// nothing: `none` is a game that is neither on Steam nor in our index (fixed by
/// adding it), `steam` is one that *is* on Steam yet whose CDN served neither the
/// vertical capsule nor the header, which is rare enough to be worth telling
/// apart.
///
/// Once per process and game, and upstream only on a fresh verdict: the desktop
/// writes an on-disk marker that stops it asking again for 30 days. One row per
/// machine per month, at the very most.
pub fn no_cover(slug: &str, source: &str) {
    if !first_time(format!("no_cover|{slug}")) {
        return;
    }
    tracing::info!(
        target: TELEMETRY_TARGET,
        verdict = "no_cover",
        slug = %slug,
        source = %source,
        "telemetry: no cover art for this game anywhere"
    );
}

/// P1: for a slug with several candidate folders, which one led `found_paths`
/// and why. This is the answer to "why did it pick THIS folder?"; the breakdown
/// was already computed during ranking and died there. Once per process and
/// (slug, path): every tick would repeat an identical verdict.
pub fn ranked_choice(slug: &str, chosen: &Path, reason: &str) {
    if !first_time(format!("ranked_choice|{slug}|{}", chosen.display())) {
        return;
    }
    tracing::info!(
        target: TELEMETRY_TARGET,
        verdict = "ranked_choice",
        slug = %slug,
        chosen = %chosen.display(),
        because = %reason,
        "telemetry: detection led with this folder"
    );
}

/// P9: an ALREADY-tracked folder looks like the game's own backup mirror, with
/// what looks like the real save sitting next to it. It repoints nothing, since
/// the warning is the whole act. Once per process and save, like
/// [`no_snapshots`].
pub fn tracked_mirror(slug: &str, save_id: &str, tracked: &Path, suggested: &Path) {
    if !first_time(format!("tracked_mirror|{save_id}|{}", tracked.display())) {
        return;
    }
    tracing::info!(
        target: TELEMETRY_TARGET,
        verdict = "tracked_mirror",
        slug = %slug,
        path = %tracked.display(),
        to = %suggested.display(),
        "telemetry: tracked folder looks like the game's own backup mirror"
    );
}

#[cfg(test)]
mod tests {
    use super::first_time;

    #[test]
    fn a_missing_cover_is_reported_once_per_run() {
        // The desktop asks for the cover on every Library repaint; without
        // this, opening and closing the tab would fill the table with the same
        // game over and over.
        assert!(super::first_time("no_cover|minecraft-java-edition".into()));
        assert!(!super::first_time("no_cover|minecraft-java-edition".into()));
    }

    #[test]
    fn the_engine_verdicts_only_count_once_per_run() {
        // The same misdirected save, sweep after sweep: one row, not a thousand.
        assert!(first_time("no_snapshots|furi|/x".into()));
        assert!(!first_time("no_snapshots|furi|/x".into()));
        // Another path for the same game is new data.
        assert!(first_time("no_snapshots|furi|/y".into()));
    }
}
