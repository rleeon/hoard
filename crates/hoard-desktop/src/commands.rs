//! Tauri `#[command]` handlers exposed to the frontend via `invoke()`.
//!
//! For phase 0 we only have a dummy greet command to prove the IPC bridge
//! works. Real commands (login, scan, backup, …) land in later phases.

/// Round-trips a name through the Rust backend so the UI can prove the
/// `invoke()` plumbing works end-to-end.
#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello from Rust, {name}! 🪙")
}
