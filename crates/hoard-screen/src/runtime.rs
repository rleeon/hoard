//! Windowed runtime, the on-screen overlay surface.
//!
//! Implemented natively per OS; all share the [`crate::engine::Engine`] +
//! [`crate::compositor`] core, only the surface and click-through differ:
//! - **X11** (`feature = "runtime"`): ARGB32 override-redirect always-on-top
//!   window, SHAPE input region for click-through, `XGrabKey` for the global
//!   Ctrl+O. *(done)*
//! - **Wayland** (`feature = "wayland"`): `wlr-layer-shell` overlay surface +
//!   `wl_shm` rendering; input region for click-through. Covers wlroots / KDE /
//!   Cosmic; GNOME / universal via **gamescope**. Global Ctrl+O comes over IPC
//!   (`set_editor`) since Wayland grants no app-level global key grab. *(done)*
//! - **Windows** (`feature = "runtime"`): `WS_EX_LAYERED | WS_EX_TRANSPARENT`
//!   topmost layered window presented with `UpdateLayeredWindow`; `RegisterHotKey`
//!   for the global Ctrl+O.
//! - **macOS** (`feature = "runtime"`): borderless `NSWindow`, high window level,
//!   `ignoresMouseEvents` for click-through; editor toggle over IPC.

use crate::engine::Engine;

#[cfg(all(target_os = "macos", feature = "runtime"))]
pub mod macos;
#[cfg(all(target_os = "linux", feature = "wayland"))]
pub mod wayland;
#[cfg(all(target_os = "windows", feature = "runtime"))]
pub mod win_gpu;
#[cfg(all(target_os = "windows", feature = "runtime"))]
pub mod windows;
#[cfg(all(target_os = "linux", feature = "runtime"))]
pub mod x11;

/// Run the overlay: own the [`Engine`], present each frame, toggle View/Editor.
/// Blocks until the overlay quits.
///
/// On Linux the Wayland vs X11 surface is chosen at runtime from the session,
/// when the binary was built with both backends.
#[cfg(all(target_os = "linux", any(feature = "runtime", feature = "wayland")))]
pub fn run(engine: Engine) -> Result<(), String> {
    #[cfg(feature = "wayland")]
    {
        if crate::capture::wayland_session() {
            return wayland::run(engine);
        }
    }
    #[cfg(feature = "runtime")]
    {
        x11::run(engine)
    }
    #[cfg(not(feature = "runtime"))]
    {
        // Wayland-only build on an X11 session: nothing to present.
        let _ = &engine;
        Err(
            "hoard-screen: built with the Wayland backend only, but this is an \
             X11 session; rebuild with --features runtime"
                .into(),
        )
    }
}

#[cfg(all(target_os = "windows", feature = "runtime"))]
pub fn run(engine: Engine) -> Result<(), String> {
    windows::run(engine)
}

#[cfg(all(target_os = "macos", feature = "runtime"))]
pub fn run(engine: Engine) -> Result<(), String> {
    macos::run(engine)
}

#[cfg(not(any(
    all(target_os = "linux", any(feature = "runtime", feature = "wayland")),
    all(target_os = "windows", feature = "runtime"),
    all(target_os = "macos", feature = "runtime"),
)))]
pub fn run(_engine: Engine) -> Result<(), String> {
    Err(
        "hoard-screen: on-screen runtime for this OS is not built in this \
         configuration; see src/runtime.rs for the per-platform plan"
            .into(),
    )
}
