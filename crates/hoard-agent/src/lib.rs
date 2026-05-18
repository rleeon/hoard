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

#[cfg(test)]
pub(crate) mod test_lock {
    //! Single process-wide mutex guarding tests that mutate environment
    //! variables (`HOME`, `XDG_*`). Cargo runs tests in parallel by default,
    //! so two unrelated tests poking `HOME` in different threads will
    //! corrupt each other's view. Hold this lock for the entire body of
    //! any env-mutating test.
    use std::sync::Mutex;
    pub(crate) static ENV: Mutex<()> = Mutex::new(());
}

pub use api::{ApiClient, ApiError};
pub use config::CliConfig;
pub use credentials::{Credentials, TokenStorage, UserSection};
pub use state::{CliState, SaveState};
