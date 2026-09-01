//! Standalone file logger for the overlay sidecar.
//!
//! The sidecar runs as its own process, so its `stderr` is easy to lose. This
//! writes a dedicated `hoard-screen.log` next to the executable (on Windows that
//! is `%LOCALAPPDATA%\hoard\hoard-screen.log`) so we can debug the overlay
//! independently of the desktop app's own logs, especially the GPU/capture path
//! that can only be exercised on the user's real machine.
//!
//! Timestamps are milliseconds since process start: deltas between lines make it
//! obvious when a frame stalls (the mouse-freeze symptom) or a capture stops
//! producing frames (the Discord/Brave black-panel symptom).

use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

static FILE: OnceLock<Option<Mutex<std::fs::File>>> = OnceLock::new();
static START: OnceLock<Instant> = OnceLock::new();

fn log_path() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    Some(dir.join("hoard-screen.log"))
}

/// Open the log file (truncating, keeping one `.1` backup) and record a banner.
/// Safe to call more than once; only the first call opens the file.
pub fn init(extra: &str) {
    START.get_or_init(Instant::now);
    FILE.get_or_init(|| {
        let path = log_path()?;
        // Keep the previous run for comparison, then start fresh so the file
        // can't grow without bound across launches.
        let _ = std::fs::rename(&path, path.with_extension("log.1"));
        OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)
            .ok()
            .map(Mutex::new)
    });
    line(
        "info",
        &format!(
            "hoard-screen {} started ({}) pid={}",
            env!("CARGO_PKG_VERSION"),
            extra,
            std::process::id()
        ),
    );
}

/// Write one line at `level` with the elapsed-ms prefix. Always mirrored to
/// stderr too, so it still surfaces if the file couldn't be opened.
pub fn line(level: &str, msg: &str) {
    let ms = START.get().map(|s| s.elapsed().as_millis()).unwrap_or(0);
    let formatted = format!("{ms:>9} [{level:<5}] {msg}\n");
    if let Some(Some(m)) = FILE.get() {
        if let Ok(mut f) = m.lock() {
            let _ = f.write_all(formatted.as_bytes());
            let _ = f.flush();
        }
    }
    eprint!("{formatted}");
}

#[macro_export]
macro_rules! slog {
    ($($arg:tt)*) => { $crate::slog::line("info", &format!($($arg)*)) };
}

#[macro_export]
macro_rules! swarn {
    ($($arg:tt)*) => { $crate::slog::line("warn", &format!($($arg)*)) };
}

#[macro_export]
macro_rules! serr {
    ($($arg:tt)*) => { $crate::slog::line("error", &format!($($arg)*)) };
}
