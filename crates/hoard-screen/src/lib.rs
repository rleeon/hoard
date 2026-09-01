//! Hoard-Screen, native, cross-platform overlay rendered over running games.
//!
//! Pro-only. Linked into `hoard-desktop` at build time behind the `pro`
//! feature (optional git dependency). Community builds compile a stub in the
//! public repo instead and never see this code.
//!
//! # Architecture
//! The overlay is a **window-capture compositor** (think OBS, but presenting
//! over the game instead of recording). It runs as its own process; the Tauri
//! desktop app owns the editor UI and pushes layout to it over [`ipc`].
//!
//! - [`scene`], the layout model (panels, crop, scale) and the crop/scale
//!   geometry. A panel never escapes its rect, so "amplify" fills the panel box
//!   and never the monitor.
//! - [`source`], the `Source` trait: anything that yields RGBA frames.
//! - [`crosshair`], procedural reticle widgets (cross/x/dot/circle), the
//!   first non-capture source kind.
//! - [`scope`], live magnifier lens (sniper view) sampling the screen under
//!   its panel via [`capture::screen`].
//! - [`engine`], owns live sources behind a scene, ticks and composites.
//! - [`capture`], per-OS window-capture backends behind one trait
//!   (Windows.Graphics.Capture / ScreenCaptureKit / portal+PipeWire / X11
//!   XComposite). The "show ANY app" feature.
//! - [`compositor`], backend-free CPU compositing of a scene into a pixel
//!   buffer (unit-testable, GPU-swappable later).
//! - [`mode`], the View/Editor state and the Ctrl+O semantics.
//! - [`ipc`], newline-JSON protocol with the desktop app.
//! - [`runtime`], the windowed event loop (winit + softbuffer); behind the
//!   `runtime` feature so the testable core builds without a display.

#[macro_use]
pub mod slog;

pub mod capture;
pub mod compositor;
pub mod crosshair;
pub mod engine;
pub mod input;
pub mod ipc;
pub mod mode;
pub mod monitors;
pub mod scene;
pub mod scope;
pub mod source;

#[cfg(any(feature = "runtime", feature = "wayland"))]
pub mod runtime;

/// Crate identifier, surfaced in the desktop "about" panel of signed Pro builds
/// so we can confirm at a glance whether the Pro layer was actually linked.
pub const FEATURE: &str = "screen";

/// Build version of the linked Pro overlay crate.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    #[test]
    fn feature_id_is_stable() {
        // Must match the server `Feature::Screen.as_str()` and the desktop
        // `FeatureKey` so entitlement checks line up across repos.
        assert_eq!(super::FEATURE, "screen");
    }
}
