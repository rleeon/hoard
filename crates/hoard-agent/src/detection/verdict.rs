//! Detection's own account of what it offered and what it left out, and why.
//!
//! Local only: plain `info` under its own target, below the upload floor of
//! `logship`, so it lands in the service's log and nowhere else. The question
//! it answers is support's first one ("why didn't it find X?", "why does it
//! call my RetroArch folder Stygian?"), which until now needed a debugger or a
//! lucky `debug!` line: the offer was logged sometimes, the refusals almost
//! never.
//!
//! Every verdict is written once per process. Detection runs every few minutes
//! and repeats itself; a verdict that changes is a new line.

use std::collections::HashSet;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

const TARGET: &str = "hoard::detect";

fn first_time(key: String) -> bool {
    static SEEN: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    SEEN.get_or_init(Default::default)
        .lock()
        .map(|mut seen| seen.insert(key))
        .unwrap_or(true)
}

/// A folder detection puts forward for `slug`.
pub fn offered(slug: &str, path: &Path, why: &str) {
    if !first_time(format!("yes|{slug}|{}|{why}", path.display())) {
        return;
    }
    tracing::info!(
        target: TARGET,
        slug = %slug,
        path = %path.display(),
        why = %why,
        "detect: offered as a save"
    );
}

/// A folder detection looked at and did not offer. `slug` is empty when the
/// folder was dropped before anything named it.
pub fn not_offered(slug: &str, path: &Path, why: &str) {
    if !first_time(format!("no|{slug}|{}|{why}", path.display())) {
        return;
    }
    tracing::info!(
        target: TARGET,
        slug = %slug,
        path = %path.display(),
        why = %why,
        "detect: not offered as a save"
    );
}

/// What automatic mode did with a game detection offered.
pub fn auto_track(slug: &str, path: &Path, verdict: &str) {
    if !first_time(format!("auto|{slug}|{}|{verdict}", path.display())) {
        return;
    }
    tracing::info!(
        target: TARGET,
        slug = %slug,
        path = %path.display(),
        verdict = %verdict,
        "detect: automatic mode"
    );
}
