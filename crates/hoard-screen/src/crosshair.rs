//! Procedural crosshair (reticle) rendering, the first non-capture overlay
//! widget. A [`CrosshairSpec`] travels inside
//! [`SourceRef::Crosshair`](crate::scene::SourceRef) and is rasterised here
//! into a single static [`Frame`], so it rides the existing engine →
//! CPU-compositor path on every platform with no per-OS code. Editing any
//! knob changes the source descriptor, which makes
//! [`Engine::set_scene`](crate::engine::Engine::set_scene) reopen the source
//! and re-render, no invalidation protocol needed.
//!
//! Shapes are drawn as signed-distance fields with ~1px analytic
//! anti-aliasing; the optional outline is the same field dilated 1px and
//! painted black underneath, so the reticle stays readable over both bright
//! and dark game footage. The frame is `size × size` and the editor keeps the
//! panel rect the same size, so the blit is 1:1 and the compositor's exact
//! texel fast-path keeps edges crisp.

use serde::{Deserialize, Serialize};

use crate::source::{Frame, Source};

/// Reticle shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CrosshairStyle {
    /// Four axis-aligned arms (`+`).
    #[default]
    Cross,
    /// Four diagonal arms (`×`).
    X,
    /// A single filled dot.
    Dot,
    /// A ring.
    Circle,
}

/// Everything that defines a crosshair's look. All fields default so the
/// desktop can send just `{"kind":"crosshair"}` and get a sane reticle.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CrosshairSpec {
    #[serde(default)]
    pub style: CrosshairStyle,
    /// Rendered frame is `size × size` px (clamped to 8..=512 at render).
    #[serde(default = "default_size")]
    pub size: u32,
    /// Stroke width in px (arm width, ring width, or dot radius for
    /// [`CrosshairStyle::Dot`]).
    #[serde(default = "default_thickness")]
    pub thickness: f32,
    /// Empty radius around the centre before the arms start (cross/x only).
    #[serde(default = "default_gap")]
    pub gap: f32,
    /// Straight-alpha RGBA; the alpha channel is the whole reticle's opacity.
    #[serde(default = "default_color")]
    pub color: [u8; 4],
    /// Extra filled dot at the exact centre (redundant for `Dot`).
    #[serde(default)]
    pub dot: bool,
    /// 1px dark rim around every stroke so the shape reads on any background.
    #[serde(default = "default_outline")]
    pub outline: bool,
}

impl Default for CrosshairSpec {
    fn default() -> Self {
        Self {
            style: CrosshairStyle::Cross,
            size: default_size(),
            thickness: default_thickness(),
            gap: default_gap(),
            color: default_color(),
            dot: false,
            outline: default_outline(),
        }
    }
}

fn default_size() -> u32 {
    48
}
fn default_thickness() -> f32 {
    3.0
}
fn default_gap() -> f32 {
    6.0
}
/// Emerald, matching the app accent, near-opaque.
fn default_color() -> [u8; 4] {
    [52, 211, 153, 240]
}
fn default_outline() -> bool {
    true
}

/// Distance from `p` to the segment `a..b` (all centre-relative px).
fn sd_segment(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (px, py) = (p.0 - a.0, p.1 - a.1);
    let (bx, by) = (b.0 - a.0, b.1 - a.1);
    let len2 = bx * bx + by * by;
    let t = if len2 > 0.0 {
        ((px * bx + py * by) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (dx, dy) = (px - bx * t, py - by * t);
    (dx * dx + dy * dy).sqrt()
}

/// Signed distance from `p` (centre-relative px) to the spec's shape edge,
/// negative inside a stroke. This is the single source of truth both the fill
/// and the outline sample.
fn distance(spec: &CrosshairSpec, half: f32, p: (f32, f32)) -> f32 {
    let half_t = (spec.thickness.max(1.0) / 2.0).min(half);
    // Strokes stop 1.5px short of the frame edge: 1px of outline + AA
    // headroom so nothing clips against the panel border.
    let reach = half - 1.5;
    let gap = spec.gap.clamp(0.0, reach);
    let arms = |dirs: [(f32, f32); 4]| {
        dirs.iter()
            .map(|d| sd_segment(p, (d.0 * gap, d.1 * gap), (d.0 * reach, d.1 * reach)) - half_t)
            .fold(f32::INFINITY, f32::min)
    };
    let r = (p.0 * p.0 + p.1 * p.1).sqrt();
    let d = match spec.style {
        CrosshairStyle::Cross => arms([(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)]),
        CrosshairStyle::X => {
            const D: f32 = std::f32::consts::FRAC_1_SQRT_2;
            arms([(D, D), (-D, D), (D, -D), (-D, -D)])
        }
        CrosshairStyle::Dot => r - spec.thickness.max(1.5),
        CrosshairStyle::Circle => (r - (reach - half_t)).abs() - half_t,
    };
    if spec.dot {
        d.min(r - (half_t + 1.0))
    } else {
        d
    }
}

/// Rasterise the spec into a straight-alpha RGBA frame.
pub fn render(spec: &CrosshairSpec) -> Frame {
    let size = spec.size.clamp(8, 512);
    let half = size as f32 / 2.0;
    let [cr, cg, cb, ca] = spec.color;
    let ca = ca as f32 / 255.0;

    let mut buf = vec![0u8; (size as usize) * (size as usize) * 4];
    for y in 0..size {
        for x in 0..size {
            let p = (x as f32 + 0.5 - half, y as f32 + 0.5 - half);
            let d = distance(spec, half, p);
            // ~1px analytic AA on the stroke; the outline is the same field
            // dilated by 1px, drawn black underneath.
            let a_fill = (0.5 - d).clamp(0.0, 1.0) * ca;
            let a_rim = if spec.outline {
                (1.5 - d).clamp(0.0, 1.0) * ca
            } else {
                a_fill
            };
            // fill (colour) OVER rim (black), straight alpha.
            let a_out = a_fill + a_rim * (1.0 - a_fill);
            if a_out <= 0.0 {
                continue;
            }
            let scale = a_fill / a_out;
            let i = ((y * size + x) * 4) as usize;
            buf[i] = (cr as f32 * scale) as u8;
            buf[i + 1] = (cg as f32 * scale) as u8;
            buf[i + 2] = (cb as f32 * scale) as u8;
            buf[i + 3] = (a_out * 255.0) as u8;
        }
    }
    Frame::new(size, size, buf)
}

/// [`Source`] wrapper: emits the rendered frame once, then `None` (the engine
/// keeps showing the last frame). A spec change arrives as a new descriptor,
/// so the engine reopens the source and this renders fresh.
pub struct CrosshairSource {
    id: String,
    frame: Option<Frame>,
}

impl CrosshairSource {
    pub fn new(id: impl Into<String>, spec: &CrosshairSpec) -> Self {
        Self {
            id: id.into(),
            frame: Some(render(spec)),
        }
    }
}

impl Source for CrosshairSource {
    fn id(&self) -> &str {
        &self.id
    }
    fn acquire(&mut self) -> Option<Frame> {
        self.frame.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alpha(f: &Frame, x: u32, y: u32) -> u8 {
        f.pixel(x, y)[3]
    }

    #[test]
    fn cross_arms_hit_axes_and_gap_stays_empty() {
        let spec = CrosshairSpec {
            gap: 8.0,
            dot: false,
            ..Default::default()
        };
        let f = render(&spec);
        let c = spec.size / 2;
        assert_eq!(alpha(&f, c, c), 0, "gap centre must be transparent");
        assert!(alpha(&f, c + 14, c) > 200, "right arm on the axis");
        assert!(alpha(&f, c, c - 14) > 200, "top arm on the axis");
        assert_eq!(alpha(&f, c + 14, c + 14), 0, "diagonal is empty on a cross");
    }

    #[test]
    fn x_arms_hit_diagonals_not_axes() {
        let spec = CrosshairSpec {
            style: CrosshairStyle::X,
            gap: 8.0,
            ..Default::default()
        };
        let f = render(&spec);
        let c = spec.size / 2;
        assert!(alpha(&f, c + 10, c + 10) > 200, "diagonal arm");
        assert_eq!(alpha(&f, c + 14, c), 0, "axis is empty on an x");
    }

    #[test]
    fn dot_and_circle_shapes() {
        let dot = render(&CrosshairSpec {
            style: CrosshairStyle::Dot,
            ..Default::default()
        });
        let c = dot.width / 2;
        assert!(alpha(&dot, c, c) > 200, "dot centre filled");
        assert_eq!(alpha(&dot, c + 10, c), 0, "dot is small");

        let spec = CrosshairSpec {
            style: CrosshairStyle::Circle,
            dot: false,
            ..Default::default()
        };
        let ring = render(&spec);
        assert_eq!(alpha(&ring, c, c), 0, "ring centre empty without dot");
        // Ring midline: reach - half_t from centre, along +x.
        let r = (spec.size as f32 / 2.0 - 1.5 - spec.thickness / 2.0) as u32;
        assert!(alpha(&ring, c + r, c) > 200, "on the ring");
    }

    #[test]
    fn center_dot_option_fills_the_gap() {
        let f = render(&CrosshairSpec {
            gap: 8.0,
            dot: true,
            ..Default::default()
        });
        let c = f.width / 2;
        assert!(alpha(&f, c, c) > 200, "centre dot present");
    }

    #[test]
    fn outline_is_dark_and_omitted_when_off() {
        let on = render(&CrosshairSpec::default());
        let c = on.width / 2;
        // Just past the arm's edge (half_t = 1.5; pixel centre 2.5px off-axis
        // → d = 1.0, inside the 1px-dilated rim but outside the fill).
        let rim = on.pixel(c + 14, c + 2);
        assert!(rim[3] > 60, "rim has coverage: {rim:?}");
        assert!(rim[0] < 30 && rim[1] < 30, "rim is dark: {rim:?}");

        let off = render(&CrosshairSpec {
            outline: false,
            ..Default::default()
        });
        assert_eq!(alpha(&off, c + 14, c + 2), 0, "no rim when outline off");
    }

    #[test]
    fn frame_size_is_clamped() {
        let f = render(&CrosshairSpec {
            size: 4,
            ..Default::default()
        });
        assert_eq!((f.width, f.height), (8, 8));
    }

    #[test]
    fn source_emits_once() {
        let mut s = CrosshairSource::new("ch", &CrosshairSpec::default());
        assert!(s.acquire().is_some());
        assert!(s.acquire().is_none(), "static: one frame, then keep-last");
    }
}
