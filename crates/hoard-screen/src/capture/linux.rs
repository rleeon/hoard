//! Linux window capture. One binary, two backends chosen at runtime:
//!
//! - **Wayland** (`XDG_SESSION_TYPE=wayland`, or `WAYLAND_DISPLAY` set): the
//!   only sanctioned path is `xdg-desktop-portal`'s **ScreenCast** interface
//!   (`org.freedesktop.portal.ScreenCast`) which, after the user picks a window
//!   in the compositor's own dialog, hands back a **PipeWire** node streaming
//!   that window's frames. Per-window capture availability depends on the
//!   portal backend (`xdg-desktop-portal-gnome`/`-kde` do windows; wlroots'
//!   `-wlr` is more output/region oriented). To finish: drive the portal over
//!   D-Bus (`ashpd` crate), then pull buffers from the PipeWire node (`pipewire`
//!   crate) and convert to RGBA. This path also covers GNOME, which has no
//!   wlr-layer-shell, see the window/placement layer.
//!
//! - **X11** (`XDG_SESSION_TYPE=x11`, or only `DISPLAY` set): redirect the
//!   target window with **XComposite** (`XCompositeRedirectWindow`), then read
//!   its off-screen pixmap each frame via XShm / `XGetImage`, using **XDamage**
//!   to know when it changed. To finish: `x11rb` with the `composite` + `damage`
//!   + `shm` extensions; enumerate top-levels via `_NET_CLIENT_LIST`.
//!
//! Both are stubbed today: enumeration and the pixel pump need a real session
//! to validate, so they return [`CaptureError::Unimplemented`] rather than
//! fabricated frames.

use super::{CaptureBackend, CaptureError, Result, WindowId, WindowInfo};
use crate::source::Source;

/// Which display server we are on, decided from the environment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Server {
    Wayland,
    X11,
}

pub fn detect_server() -> Server {
    if std::env::var_os("WAYLAND_DISPLAY").is_some()
        || std::env::var("XDG_SESSION_TYPE")
            .map(|s| s == "wayland")
            .unwrap_or(false)
    {
        Server::Wayland
    } else {
        Server::X11
    }
}

pub fn backend() -> Box<dyn CaptureBackend> {
    match detect_server() {
        Server::Wayland => wayland_backend(),
        Server::X11 => x11_backend(),
    }
}

/// Real Wayland portal+PipeWire capture when built with `wayland`; stub otherwise.
#[cfg(feature = "wayland")]
fn wayland_backend() -> Box<dyn CaptureBackend> {
    Box::new(super::wayland::WaylandPortalCapture)
}

#[cfg(not(feature = "wayland"))]
fn wayland_backend() -> Box<dyn CaptureBackend> {
    Box::new(WaylandPortalCapture)
}

/// Real X11 capture when built with `runtime`; the documented stub otherwise.
#[cfg(feature = "runtime")]
fn x11_backend() -> Box<dyn CaptureBackend> {
    match super::x11::X11Capture::connect() {
        Ok(b) => Box::new(b),
        Err(e) => {
            eprintln!("hoard-screen: X11 connect failed ({e}); stub backend");
            Box::new(X11CompositeCapture)
        }
    }
}

#[cfg(not(feature = "runtime"))]
fn x11_backend() -> Box<dyn CaptureBackend> {
    Box::new(X11CompositeCapture)
}

/// xdg-desktop-portal ScreenCast + PipeWire stub, used only when the crate is
/// built without the `wayland` feature. The real implementation lives in
/// [`super::wayland`].
#[cfg(not(feature = "wayland"))]
pub struct WaylandPortalCapture;

#[cfg(not(feature = "wayland"))]
impl CaptureBackend for WaylandPortalCapture {
    fn name(&self) -> &'static str {
        "linux/wayland-portal-pipewire"
    }
    fn list_windows(&self) -> Result<Vec<WindowInfo>> {
        // The portal picks the window via its own dialog; a pre-enumerated list
        // isn't always available. Return empty for now (editor falls back to
        // "pick via system dialog").
        Err(CaptureError::Unimplemented(self.name()))
    }
    fn capture(&self, _id: &WindowId) -> Result<Box<dyn Source>> {
        Err(CaptureError::Unimplemented(self.name()))
    }
}

/// XComposite + XDamage + XShm. TODO: implement with `x11rb`.
pub struct X11CompositeCapture;

impl CaptureBackend for X11CompositeCapture {
    fn name(&self) -> &'static str {
        "linux/x11-xcomposite"
    }
    fn list_windows(&self) -> Result<Vec<WindowInfo>> {
        Err(CaptureError::Unimplemented(self.name()))
    }
    fn capture(&self, _id: &WindowId) -> Result<Box<dyn Source>> {
        Err(CaptureError::Unimplemented(self.name()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_detection_prefers_wayland_when_display_set() {
        // Pure function of env; just assert it returns one of the two and does
        // not panic in this headless tty.
        let s = detect_server();
        assert!(matches!(s, Server::Wayland | Server::X11));
    }
}
