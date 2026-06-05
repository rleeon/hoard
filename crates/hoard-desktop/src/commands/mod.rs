//! Tauri `#[command]` handlers exposed to the frontend via `invoke()`.
//!
//! Each submodule groups commands by concern. The frontend never talks to
//! `hoard-agent` or `hoard-core` directly — it goes through these handlers,
//! which translate library errors into messages suitable for end users.

pub mod agent;
pub mod auth;
pub mod automatic;
pub mod catalog;
pub mod cloud;
pub mod cloud_pull;
pub mod cloud_realtime;
pub mod covers;
pub mod error;
pub mod history;
pub mod library;
pub mod loopback;
pub mod misc;
pub mod prefs;
pub mod updates;
