//! Tauri `#[command]` handlers exposed to the frontend via `invoke()`.
//!
//! Each submodule groups commands by concern. The frontend never talks to
//! `hoard-agent` or `hoard-core` directly — it goes through these handlers,
//! which translate library errors into messages suitable for end users.

pub mod auth;
pub mod misc;
