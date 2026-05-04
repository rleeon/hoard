//! Shared client-side logic for Hoard.
//!
//! This crate contains everything the CLI (`hoard-cli`) and the desktop app
//! (`hoard-desktop`) need in common: the HTTP API client, on-disk config and
//! state files, and the upload/download flows for snapshots.
//!
//! Higher-level features (game detection, process watching, scheduling) will
//! land here in later phases. Today the surface is intentionally small.

pub mod api;
pub mod backup;
pub mod config;
pub mod credentials;
pub mod manifest;
pub mod pathexpand;
pub mod restore;
pub mod state;

pub use api::{ApiClient, ApiError};
pub use config::CliConfig;
pub use credentials::{Credentials, TokenStorage, UserSection};
pub use state::{CliState, SaveState};
