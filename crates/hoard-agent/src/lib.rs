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
pub mod atomic_write;
pub mod backup;
pub mod catalog;
pub mod cloud_account;
pub mod cloud_auth;
pub mod cloud_live;
pub mod config;
pub mod correlation;
pub mod credentials;
pub mod detection;
pub mod device_slot;
pub mod doctor;
pub mod emulators;
pub mod install;
pub mod junkdirs;
pub mod keychain;
pub mod launchers;
pub mod library;
pub mod locks;
pub mod logship;
pub mod manifest;
pub mod pathexpand;
pub mod playtime;
pub mod playtime_catalog;
pub mod playtime_index;
pub mod prefs;
pub mod presence;
pub mod presets;
pub mod preview;
pub mod proclist;
pub mod restore;
pub mod roots;
pub mod savefilter;
pub mod scoring;
pub mod serverclass;
pub mod session;
pub mod state;
pub mod steam;
pub mod supervisor;
pub mod telemetry;
pub mod tls;
pub mod update;
pub mod wine_prefixes;
pub mod wrappers;

pub use api::{ApiClient, ApiError};
pub use config::CliConfig;
pub use credentials::{Credentials, TokenStorage, UserSection};
pub use state::{CliState, SaveState};

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
