//! Window-capture backends, the "show ANY app over the game" feature.
//!
//! Every platform exposes the same shape: enumerate capturable windows
//! ([`CaptureBackend::list_windows`]) and open a live [`Source`] for one
//! ([`CaptureBackend::capture`]). The compositor never knows which OS produced
//! the pixels. [`backend`] picks the implementation at compile time.
//!
//! Each platform module documents the *exact* native API it must drive. The
//! pixel pump for each is the only code that genuinely needs a real desktop
//! session to validate, so they are written as honest, clearly-marked stubs
//! returning [`CaptureError::Unimplemented`] until finished on that OS, never
//! as code that silently pretends to work.
//!
//! ## DRM caveat (read this)
//! Protected streaming (Netflix, Prime Video, Disney+) uses HDCP / a protected
//! media path. Capturing such a window yields **black frames** on every OS,
//! this is by design and not fixable here, exactly as OBS hits it. Ordinary
//! apps, YouTube, Twitch, native players, browsers, file managers capture fine.
//! Backends surface a likely-protected window via [`WindowInfo::protected`] so
//! the editor can warn instead of showing a black panel.

use crate::source::Source;

/// Opaque, backend-specific window handle (an X11 window id, an HWND, a
/// `CGWindowID`, or a portal node id), stringified so it round-trips through
/// the JSON scene as `SourceRef::Window { id }`.
pub type WindowId = String;

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct WindowInfo {
    pub id: WindowId,
    /// Human title for the editor's picker.
    pub title: String,
    /// Owning application / class, when the backend can resolve it.
    pub app: String,
    /// Heuristic: the window likely shows DRM-protected content and will
    /// capture black. Editor should warn.
    pub protected: bool,
}

#[derive(Debug)]
pub enum CaptureError {
    /// This platform's pixel pump is not finished yet.
    Unimplemented(&'static str),
    /// The user denied the capture permission (Wayland portal, macOS TCC).
    PermissionDenied,
    /// The requested window no longer exists.
    NotFound(WindowId),
    /// Anything else, with context.
    Backend(String),
}

impl std::fmt::Display for CaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CaptureError::Unimplemented(p) => write!(f, "capture backend not implemented: {p}"),
            CaptureError::PermissionDenied => write!(f, "capture permission denied"),
            CaptureError::NotFound(id) => write!(f, "window not found: {id}"),
            CaptureError::Backend(m) => write!(f, "capture backend error: {m}"),
        }
    }
}

impl std::error::Error for CaptureError {}

pub type Result<T> = std::result::Result<T, CaptureError>;

/// A platform window-capture provider.
pub trait CaptureBackend: Send {
    /// Backend name for logs / about screen.
    fn name(&self) -> &'static str;
    /// Windows the user can pick in the editor.
    fn list_windows(&self) -> Result<Vec<WindowInfo>>;
    /// Open a live frame source for one window.
    fn capture(&self, id: &WindowId) -> Result<Box<dyn Source>>;
}

#[cfg(target_os = "linux")]
mod linux;
#[cfg(all(target_os = "macos", feature = "runtime"))]
mod macos;
pub mod screen;
#[cfg(all(target_os = "linux", feature = "wayland"))]
mod wayland;
#[cfg(all(target_os = "windows", feature = "runtime"))]
mod windows;
#[cfg(all(target_os = "linux", feature = "runtime"))]
mod x11;

/// The capture backend for the host OS.
///
/// On Linux the choice between the Wayland (portal + PipeWire) and X11
/// (XComposite) paths is made at runtime inside the linux module from the
/// session environment, because one binary runs on both.
pub fn backend() -> Box<dyn CaptureBackend> {
    #[cfg(all(target_os = "windows", feature = "runtime"))]
    {
        Box::new(windows::WindowsCapture::new())
    }
    #[cfg(all(target_os = "macos", feature = "runtime"))]
    {
        Box::new(macos::MacCapture::new())
    }
    #[cfg(target_os = "linux")]
    {
        linux::backend()
    }
    // Windows/macOS without the `runtime` feature (the testable core build)
    // have no native capture backend compiled in.
    #[cfg(not(any(
        all(target_os = "windows", feature = "runtime"),
        all(target_os = "macos", feature = "runtime"),
        target_os = "linux"
    )))]
    {
        Box::new(Unsupported)
    }
}

/// True when running under a Wayland session (used to pick the overlay surface).
#[cfg(target_os = "linux")]
pub fn wayland_session() -> bool {
    matches!(linux::detect_server(), linux::Server::Wayland)
}

/// Fallback for platforms with no backend at all.
#[allow(dead_code)]
pub struct Unsupported;

impl CaptureBackend for Unsupported {
    fn name(&self) -> &'static str {
        "unsupported"
    }
    fn list_windows(&self) -> Result<Vec<WindowInfo>> {
        Err(CaptureError::Unimplemented("unsupported platform"))
    }
    fn capture(&self, _id: &WindowId) -> Result<Box<dyn Source>> {
        Err(CaptureError::Unimplemented("unsupported platform"))
    }
}
