//! Scene model, the layout the overlay renders, pushed from the desktop app.
//!
//! A [`Scene`] is a flat list of [`Panel`]s. Each panel points at a *source*
//! (an app-window capture, an image, a note) and says where on the overlay it
//! is drawn ([`Rect`]), how the source is cropped ([`Crop`]) and how it fills
//! the panel box ([`ScaleMode`]).
//!
//! The whole model is serde-serialisable: the desktop (Tauri) app owns the
//! editor and ships scene updates as JSON over [`crate::ipc`]. The overlay
//! process is a dumb renderer of whatever scene it last received.
//!
//! Coordinates are **logical overlay pixels** (the overlay surface spans the
//! game's monitor). A panel never escapes its [`Rect`]: "maximise"/"amplify" a
//! video means growing its `Rect` or switching to [`ScaleMode::Fill`], not the
//! OS notion of fullscreen, there is no toplevel to fullscreen, the content is
//! composited into the quad we draw. That is the containment guarantee.

use serde::{Deserialize, Serialize};

/// A rectangle in logical overlay pixels.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub const fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self { x, y, w, h }
    }

    /// Width/height clamped to be non-negative and at least 1px so a degenerate
    /// panel never produces a zero-area blit (which some blitters choke on).
    pub fn normalized(self) -> Self {
        Self {
            x: self.x,
            y: self.y,
            w: self.w.max(1.0),
            h: self.h.max(1.0),
        }
    }
}

/// Per-edge crop, expressed as a **fraction** (0.0..1.0) of the *source*
/// content, so it is resolution-independent: a 30% left crop is 30% whatever
/// the captured window's pixel size is at that instant (window capture sources
/// change size as the user resizes the real window). The visible source region
/// is `1 - left - right` wide and `1 - top - bottom` tall.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Crop {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

impl Crop {
    pub const NONE: Crop = Crop {
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
        left: 0.0,
    };

    /// Clamp each edge to 0..1 and guarantee the kept span on each axis is at
    /// least 1% (so a fully-collapsed crop can't yield a zero-size source rect).
    pub fn sanitized(self) -> Self {
        let c = |v: f64| v.clamp(0.0, 1.0);
        let (mut top, mut bottom) = (c(self.top), c(self.bottom));
        let (mut left, mut right) = (c(self.left), c(self.right));
        if top + bottom > 0.99 {
            let s = 0.99 / (top + bottom);
            top *= s;
            bottom *= s;
        }
        if left + right > 0.99 {
            let s = 0.99 / (left + right);
            left *= s;
            right *= s;
        }
        Self {
            top,
            right,
            bottom,
            left,
        }
    }
}

/// Which monitor a panel is drawn on. Its [`Rect`] is in **monitor-local**
/// overlay pixels, `(0,0)` is the top-left of the target monitor, not the
/// virtual desktop, so a layout stays put regardless of how the OS arranges
/// monitors in virtual space.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum PanelTarget {
    /// Draw only on the monitor with this id (index from `--list-monitors`,
    /// `0` = primary).
    Monitor { id: u32 },
    /// Mirror: draw the same panel on every monitor (each in its own local
    /// space, so it lands at the same relative spot on each screen).
    All,
}

impl Default for PanelTarget {
    fn default() -> Self {
        PanelTarget::Monitor { id: 0 }
    }
}

impl PanelTarget {
    /// True when this panel should be drawn on the monitor with `mon_id`.
    pub fn draws_on(self, mon_id: u32) -> bool {
        match self {
            PanelTarget::Monitor { id } => id == mon_id,
            PanelTarget::All => true,
        }
    }
}

/// How the cropped source fills the panel box.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ScaleMode {
    /// Fill the whole panel box, ignoring aspect ratio. This is the "blow it up to
    /// fill whatever window the overlay gives it" behaviour.
    #[default]
    Fill,
    /// Preserve aspect ratio, letterboxed inside the panel box.
    Fit,
}

/// What a panel draws.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum SourceRef {
    /// Live capture of an OS window, addressed by the capture backend's id.
    Window { id: String },
    /// A still image already on disk (cache from R2).
    Image { path: String },
    /// A text note rendered by the overlay.
    Note { text: String },
    /// A procedurally-drawn crosshair / reticle (see [`crate::crosshair`]).
    /// Static: the overlay rasterises the spec once and re-renders only when
    /// the descriptor changes.
    Crosshair(crate::crosshair::CrosshairSpec),
    /// A live magnifier lens over the screen region under the panel (see
    /// [`crate::scope`]).
    Scope(crate::scope::ScopeSpec),
    /// Built-in animated test pattern, lets the full pipeline (window +
    /// compositor + crop) be driven on a real desktop before any capture
    /// backend is finished.
    Test,
    /// Forward-compat safety net: any `kind` this build doesn't know. Renders
    /// as an invisible placeholder instead of failing the whole `set_scene`
    /// line, an older overlay paired with a newer editor must degrade to
    /// "that one panel is missing", never to "the entire scene was dropped"
    /// (which also made the next mode-toggle echo the stale scene back and
    /// erase the new panel from the editor).
    #[serde(other)]
    Unknown,
}

/// One element of the overlay.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Panel {
    pub id: String,
    pub source: SourceRef,
    pub rect: Rect,
    #[serde(default)]
    pub crop: Crop,
    #[serde(default)]
    pub scale: ScaleMode,
    /// Draw order; higher is on top.
    #[serde(default)]
    pub z: i32,
    /// Which monitor(s) this panel is drawn on. Defaults to the primary monitor
    /// so older scenes (no `monitor` field) keep their single-screen behaviour.
    #[serde(default)]
    pub monitor: PanelTarget,
    /// Windows-only: in View mode, whether the adopted real window is made
    /// click-through (`WS_EX_TRANSPARENT`) so mouse input falls to the game
    /// below. Defaults to `true` (the historical behaviour: gameplay clicks pass
    /// through every panel).
    ///
    /// Set to `false` for a panel whose source is a hardware-accelerated /
    /// DRM-protected video (Prime Video, Netflix in a browser) that renders
    /// **black** when click-through is enabled. Applying `WS_EX_TRANSPARENT` to a
    /// foreign window backed by a video swapchain can disturb its composition;
    /// turning passthrough off for *that* panel leaves the browser window with
    /// its own ex-style untouched (so the video composites normally) at the cost
    /// that clicks on that one panel land on the browser instead of the game.
    /// All other panels keep click-through, so gameplay is unaffected except
    /// directly over the video rectangle.
    #[serde(default = "default_passthrough")]
    pub passthrough: bool,
    /// Windows-only, View mode: route this **window** panel through the WGC
    /// capture + compositor path instead of placing the real window over the
    /// game. The crop is then applied to captured pixels, so it is
    /// pixel-perfect even on Chromium/Electron windows (Brave, Discord), whose
    /// DirectComposition surface ignores `SetWindowRgn` and otherwise leaves a
    /// gray border. The real window is parked off-screen (not minimized, not
    /// occluded) so it keeps rendering and its video doesn't go black. Defaults
    /// to `false` (place the real window, the historical behaviour). See
    /// `docs/hoard-screen-chromium-crop.md`.
    #[serde(default)]
    pub compat: bool,
    /// Windows-only, View mode, compat + passthrough ON: radius (px) of the circular
    /// click-through "lens" that follows the cursor over this panel. Inside it the
    /// overlay is cut away so the OS delivers clicks NATIVELY to whatever sits
    /// behind the panel (real double-click / drag); outside it the panel is shown
    /// normally. Adjustable from the Hoard panel UI.
    #[serde(default = "default_passthrough_radius")]
    pub passthrough_radius: f32,
}

/// Panels are click-through by default (preserves pre-`passthrough` scenes).
fn default_passthrough() -> bool {
    true
}

/// Default click-through lens radius (px).
fn default_passthrough_radius() -> f32 {
    90.0
}

/// The full overlay layout.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Scene {
    pub panels: Vec<Panel>,
}

impl Scene {
    /// Panels in back-to-front draw order (stable within equal `z`).
    pub fn draw_order(&self) -> Vec<&Panel> {
        let mut v: Vec<&Panel> = self.panels.iter().collect();
        v.sort_by_key(|p| p.z);
        v
    }
}

/// A source-pixel rectangle (the region of a captured frame that is visible).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PixRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// A resolved copy operation: take `src` pixels from the source frame and draw
/// them into `dst` on the overlay. Produced by [`resolve_blit`]; consumed by
/// the compositor. Keeping it a plain value makes the geometry unit-testable
/// with no GPU or window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Blit {
    pub src: PixRect,
    pub dst: Rect,
}

/// Resolve where a panel's cropped, scaled source lands on the overlay, given
/// the source frame's current pixel size.
///
/// `Fill` makes `dst` exactly the panel box (content stretches to fill, the
/// containment behaviour). `Fit` centres an aspect-correct rect inside the box.
pub fn resolve_blit(panel: &Panel, src_w: u32, src_h: u32) -> Blit {
    let crop = panel.crop.sanitized();
    let box_ = panel.rect.normalized();
    let (sw, sh) = (src_w.max(1) as f64, src_h.max(1) as f64);

    // Visible source rectangle in source pixels.
    let sx = (crop.left * sw).round();
    let sy = (crop.top * sh).round();
    let svw = ((1.0 - crop.left - crop.right) * sw).round().max(1.0);
    let svh = ((1.0 - crop.top - crop.bottom) * sh).round().max(1.0);
    let src = PixRect {
        x: sx as u32,
        y: sy as u32,
        w: svw as u32,
        h: svh as u32,
    };

    let dst = match panel.scale {
        ScaleMode::Fill => box_,
        ScaleMode::Fit => {
            let ar = svw / svh;
            let box_ar = box_.w / box_.h;
            let (w, h) = if ar > box_ar {
                (box_.w, box_.w / ar)
            } else {
                (box_.h * ar, box_.h)
            };
            Rect::new(
                box_.x + (box_.w - w) / 2.0,
                box_.y + (box_.h - h) / 2.0,
                w,
                h,
            )
        }
    };
    Blit { src, dst }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn panel(scale: ScaleMode, crop: Crop) -> Panel {
        Panel {
            id: "p".into(),
            source: SourceRef::Test,
            rect: Rect::new(100.0, 50.0, 800.0, 600.0),
            crop,
            scale,
            z: 0,
            monitor: PanelTarget::default(),
            passthrough: true,
            compat: false,
            passthrough_radius: 90.0,
        }
    }

    #[test]
    fn fill_uses_whole_panel_box_never_more() {
        let b = resolve_blit(&panel(ScaleMode::Fill, Crop::NONE), 1920, 1080);
        // dst is exactly the panel box: the content can never exceed it.
        assert_eq!(b.dst, Rect::new(100.0, 50.0, 800.0, 600.0));
        assert_eq!(
            b.src,
            PixRect {
                x: 0,
                y: 0,
                w: 1920,
                h: 1080
            }
        );
    }

    #[test]
    fn crop_selects_a_subrect_of_the_source() {
        let c = Crop {
            top: 0.25,
            right: 0.0,
            bottom: 0.25,
            left: 0.5,
        };
        let b = resolve_blit(&panel(ScaleMode::Fill, c), 1000, 800);
        assert_eq!(
            b.src,
            PixRect {
                x: 500,
                y: 200,
                w: 500,
                h: 400
            }
        );
    }

    #[test]
    fn fit_preserves_aspect_inside_box() {
        // 2:1 source into an 800x600 (4:3) box -> width-bound, letterboxed.
        let b = resolve_blit(&panel(ScaleMode::Fit, Crop::NONE), 2000, 1000);
        assert!((b.dst.w - 800.0).abs() < 0.01);
        assert!((b.dst.h - 400.0).abs() < 0.01);
        // centred vertically inside the 600-tall box.
        assert!((b.dst.y - (50.0 + 100.0)).abs() < 0.01);
    }

    #[test]
    fn collapsed_crop_cannot_produce_zero_area() {
        let c = Crop {
            top: 0.9,
            right: 0.9,
            bottom: 0.9,
            left: 0.9,
        };
        let b = resolve_blit(&panel(ScaleMode::Fill, c), 100, 100);
        assert!(b.src.w >= 1 && b.src.h >= 1);
    }

    #[test]
    fn scene_draw_order_is_by_z() {
        let mk = |id: &str, z| Panel {
            id: id.into(),
            source: SourceRef::Test,
            rect: Rect::new(0.0, 0.0, 1.0, 1.0),
            crop: Crop::NONE,
            scale: ScaleMode::Fill,
            z,
            monitor: PanelTarget::default(),
            passthrough: true,
            compat: false,
            passthrough_radius: 90.0,
        };
        let s = Scene {
            panels: vec![mk("a", 5), mk("b", -1), mk("c", 2)],
        };
        let order: Vec<_> = s.draw_order().into_iter().map(|p| p.id.as_str()).collect();
        assert_eq!(order, ["b", "c", "a"]);
    }
}
