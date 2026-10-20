//! How WebKitGTK reaches the screen on Linux, decided before GTK starts.
//!
//! WebKitGTK hands every frame to the compositor as a GPU buffer (DMA-BUF). On
//! some driver and compositor pairs the compositor rejects it and drops the
//! connection; GDK prints `Error 71 (Protocol error) dispatching to Wayland
//! display` and calls `_exit(1)`, so the window shows up and is gone 165 ms
//! later. First report: 16-10-2026, a KONKR Pocket FIT (Snapdragon 8 Gen 3,
//! KDE on Wayland) on the SteamOS ARM port, fixed there by `GDK_BACKEND=x11`.
//! Nothing about it is ARM: the same message is the usual NVIDIA report.
//!
//! The AppImage used to hide it. Tauri's GTK hook exported `GDK_BACKEND=x11`
//! for years, and the copy the 1.2.2 build fetched has that line commented
//! out, so every AppImage went native Wayland without a change on our side.
//!
//! Forcing X11 on everyone would make machines that work today pay for the few
//! that don't (blurry text under fractional scaling, for one). So the default
//! stays native, and a GLib log writer waits for that one message: before
//! `_exit` runs it records a step down [`render`], and the next start applies
//! it. One failed launch, then the app opens for good. Deleting the
//! `render-fallback` file in the state dir goes back to native.

use std::path::{Path, PathBuf};

const FILE: &str = "render-fallback";
const TOP: u8 = 3;

/// The variables this module would set. If the user exported any of them, the
/// choice is theirs: nothing is applied and nothing is recorded.
const USER_VARS: [&str; 3] = [
    "GDK_BACKEND",
    "WEBKIT_DISABLE_DMABUF_RENDERER",
    "WEBKIT_DISABLE_COMPOSITING_MODE",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Render {
    x11: bool,
    no_dmabuf: bool,
    no_compositing: bool,
}

pub enum Choice {
    User,
    Native,
    Fallback(u8, Render),
}

/// What step `level` means on this machine. X11 comes first: it is what fixed
/// the first report and what the AppImage did all along. Without XWayland
/// there is no X11 to go to, so the first step is turning DMA-BUF off, which
/// paints through shared memory and costs CPU. Compositing off is the last
/// resort.
fn render(level: u8, xwayland: bool) -> Render {
    Render {
        x11: level >= 1 && xwayland,
        no_dmabuf: level >= 2 || (level >= 1 && !xwayland),
        no_compositing: level >= 3,
    }
}

/// The next level that changes something here, so a crash never buys a step
/// that would crash the same way.
fn next(level: u8, xwayland: bool) -> u8 {
    let now = render(level, xwayland);
    (level + 1..=TOP)
        .find(|&l| render(l, xwayland) != now)
        .unwrap_or(TOP)
}

/// Only errno 71. A compositor that exits (logout, a KWin crash) gives
/// `Error 32 (Broken pipe)` on the same line, and that must not cost anyone
/// the native path. The number, not the text: `g_strerror` is translated.
fn is_protocol_error(message: &str) -> bool {
    message.starts_with("Error 71 ") && message.contains("Wayland display")
}

/// Sets environment variables, so it must run before any thread exists.
pub fn prepare() -> Choice {
    if USER_VARS.iter().any(|v| std::env::var_os(v).is_some()) {
        return Choice::User;
    }
    let Ok(path) = hoard_agent::config::CliConfig::state_dir().map(|d| d.join(FILE)) else {
        return Choice::Native;
    };
    let xwayland = std::env::var_os("DISPLAY").is_some_and(|d| !d.is_empty());
    let level = read_level(&path);
    watch(path, level, xwayland);
    if level == 0 {
        return Choice::Native;
    }
    let r = render(level, xwayland);
    if r.x11 {
        std::env::set_var("GDK_BACKEND", "x11");
    }
    if r.no_dmabuf {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
    if r.no_compositing {
        std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
    }
    Choice::Fallback(level, r)
}

pub fn log(choice: &Choice) {
    match choice {
        Choice::User => tracing::info!("graphics: left to the user's environment"),
        Choice::Native => {}
        Choice::Fallback(level, r) => tracing::warn!(
            level,
            x11 = r.x11,
            dmabuf_off = r.no_dmabuf,
            compositing_off = r.no_compositing,
            "graphics: a previous run lost its Wayland connection (Error 71), starting on a fallback"
        ),
    }
}

fn read_level(path: &Path) -> u8 {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse::<u8>().ok())
        .map_or(0, |l| l.min(TOP))
}

/// GDK reports the failure with `g_log_structured` and calls `_exit(1)` right
/// after, so the writer is the last code of ours that runs. The file is
/// written synchronously: the tracing appender is non-blocking and would lose
/// the line. Everything else goes on to GLib's default writer untouched.
fn watch(path: PathBuf, level: u8, xwayland: bool) {
    glib::log_set_writer_func(move |log_level, fields| {
        let hit = fields
            .iter()
            .any(|f| f.key() == "MESSAGE" && f.value_str().is_some_and(is_protocol_error));
        if hit && level < TOP {
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(&path, next(level, xwayland).to_string());
        }
        glib::log_writer_default(log_level, fields)
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_xwayland_the_first_step_is_x11() {
        assert_eq!(next(0, true), 1);
        assert_eq!(
            render(1, true),
            Render {
                x11: true,
                no_dmabuf: false,
                no_compositing: false
            }
        );
        assert_eq!(next(1, true), 2);
        assert_eq!(next(2, true), 3);
    }

    #[test]
    fn without_xwayland_no_step_repeats_the_last_one() {
        assert_eq!(next(0, false), 1);
        assert!(render(1, false).no_dmabuf);
        assert!(!render(1, false).x11);
        // Level 2 would be level 1 again here.
        assert_eq!(next(1, false), 3);
        assert!(render(3, false).no_compositing);
    }

    #[test]
    fn the_ladder_ends() {
        assert_eq!(next(TOP, true), TOP);
        assert_eq!(next(TOP, false), TOP);
    }

    #[test]
    fn only_a_protocol_error_counts() {
        assert!(is_protocol_error(
            "Error 71 (Protocol error) dispatching to Wayland display."
        ));
        assert!(is_protocol_error(
            "Error 71 (Error de protocolo) dispatching to Wayland display."
        ));
        assert!(!is_protocol_error(
            "Error 32 (Broken pipe) dispatching to Wayland display."
        ));
        assert!(!is_protocol_error("Error 71 (Protocol error) something else"));
    }

    /// The exact call GDK makes before `_exit(1)`, through the real GLib: if a
    /// GLib stops routing it to the writer, this fails here and not on someone's
    /// handheld. Only one test may install the writer per process.
    #[test]
    fn the_writer_records_the_next_step() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE);
        watch(path.clone(), 0, true);
        unsafe {
            glib::ffi::g_log_structured_standard(
                c"Gdk".as_ptr(),
                glib::ffi::G_LOG_LEVEL_MESSAGE,
                c"gdkeventsource.c".as_ptr(),
                c"1".as_ptr(),
                c"_gdk_wayland_display_queue_events".as_ptr(),
                c"Error %d (%s) dispatching to Wayland display.".as_ptr(),
                71 as std::ffi::c_int,
                c"Protocol error".as_ptr(),
            );
        }
        assert_eq!(read_level(&path), 1);
    }
}
