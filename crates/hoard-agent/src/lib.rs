//! Shared client-side logic for Hoard.
//!
//! This crate contains everything the CLI (`hoard-cli`) and the desktop app
//! (`hoard-desktop`) need in common: the HTTP API client, on-disk config and
//! state files, and the upload/download flows for snapshots.
//!
//! Higher-level features (game detection, process watching, scheduling) will
//! land here in later phases. Today the surface is intentionally small.

pub mod agent;
pub mod api;
pub mod autodetect;
pub mod backup;
pub mod config;
pub mod credentials;
pub mod detection;
pub mod manifest;
pub mod pathexpand;
pub mod prefs;
pub mod restore;
pub mod state;
pub mod steam;

pub use api::{ApiClient, ApiError};
pub use config::CliConfig;
pub use credentials::{Credentials, TokenStorage, UserSection};
pub use state::{CliState, SaveState};

/// Look up the `processes` list for a game slug from the embedded
/// manifest catalogue. Returns an empty `Vec` if the slug is unknown —
/// the agent treats an empty list as "fall back to install-dir match".
///
/// Convenience wrapper so callers (e.g. `hoard-desktop`) don't need a
/// direct dependency on `hoard-manifest`.
pub fn hoard_manifest_processes(slug: &str) -> Vec<String> {
    hoard_manifest::lookup(slug)
        .map(|m| m.processes.clone())
        .unwrap_or_default()
}
