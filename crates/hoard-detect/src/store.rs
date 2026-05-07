//! Storefront scanners.
//!
//! Each scanner enumerates installed games for one storefront and
//! cross-references them against the manifest catalogue. Games not in
//! the catalogue are skipped silently — we only surface games we know
//! how to back up.
//!
//! v0.3 ships **Steam only**. GOG / Epic / Xbox stubs are filled in
//! across the v0.3.x patch line.

use std::path::{Path, PathBuf};

use crate::{DetectError, DetectedGame, GameSource};

pub mod steam;

/// Scan every supported store and return a deduplicated list of detected
/// games. If the same game is found via multiple stores (e.g. on Steam
/// and GOG), the first one wins.
///
/// Errors from individual scanners are logged via `tracing` and the
/// scanner is skipped — a missing Epic install must not break Steam
/// detection.
pub fn scan_all() -> Vec<DetectedGame> {
    let mut out: Vec<DetectedGame> = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    match steam::scan() {
        Ok(games) => {
            for g in games {
                if seen_ids.insert(g.manifest_id.clone()) {
                    out.push(g);
                }
            }
        }
        Err(e) => {
            tracing::info!("Steam scan skipped: {e}");
        }
    }

    // GOG / Epic / Xbox land in v0.3.x.
    out
}

/// Helper used by all scanners: read a file or return a wrapped IO error
/// tagged with the path so users can see which store choked.
pub(crate) fn read_to_string(path: &Path) -> Result<String, DetectError> {
    std::fs::read_to_string(path).map_err(|e| DetectError::Io {
        path: path.display().to_string(),
        source: e,
    })
}

/// Convenience constructor for a [`DetectedGame`] from a manifest lookup.
pub(crate) fn from_manifest(
    m: &hoard_manifest::GameManifest,
    install_dir: Option<PathBuf>,
    source: GameSource,
) -> DetectedGame {
    DetectedGame {
        manifest_id: m.id.clone(),
        title: m.title.clone(),
        install_dir,
        source,
    }
}
