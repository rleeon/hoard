//! CPU compositor: paints a [`Scene`] into an RGBA8 framebuffer.
//!
//! Deliberately backend-free, it writes pixels into a plain `&mut [u8]`, so
//! the whole compositing path (crop, scale, alpha, z-order) is unit-testable
//! with no GPU or window. The windowed runtime ([`crate::runtime`], behind the
//! `runtime` feature) just hands this its surface buffer and presents it.
//!
//! Scaling is bilinear by default. A panel can ask for nearest-neighbour
//! instead (`smooth: false` on a scope): at high magnification the lens
//! stretches a handful of pixels across the whole panel, and interpolating
//! that turns it into coloured mush, hard pixel edges read better. A GPU path
//! (sampled textures) can replace this later without touching the
//! scene/geometry model, that is the point of the seam.

use std::collections::HashMap;

use crate::scene::{resolve_blit, Panel, Scene, SourceRef};
use crate::source::Frame;

/// Whether this panel's pixels should be interpolated when scaled. Only the
/// scope opts out today; everything else keeps the smooth path it always had.
fn panel_smooth(panel: &Panel) -> bool {
    match &panel.source {
        SourceRef::Scope(spec) => spec.smooth,
        _ => true,
    }
}

/// Clear `out` (RGBA8, `out_w*out_h*4` bytes) to fully transparent, then draw
/// every panel of `scene` back-to-front. `frames` holds each panel's latest
/// frame, keyed by `panel.id`; a panel with no frame yet is skipped.
pub fn compose_into(
    scene: &Scene,
    frames: &HashMap<String, Frame>,
    out: &mut [u8],
    out_w: u32,
    out_h: u32,
) {
    debug_assert_eq!(out.len(), (out_w as usize) * (out_h as usize) * 4);
    out.fill(0);

    for panel in scene.draw_order() {
        let Some(frame) = frames.get(&panel.id) else {
            continue;
        };
        if frame.width == 0 || frame.height == 0 {
            continue;
        }
        let blit = resolve_blit(panel, frame.width, frame.height);
        draw_blit(frame, &blit, out, out_w, out_h, panel_smooth(panel));
    }
}

fn draw_blit(
    frame: &Frame,
    blit: &crate::scene::Blit,
    out: &mut [u8],
    out_w: u32,
    out_h: u32,
    smooth: bool,
) {
    let dst = blit.dst;
    // Destination pixel bounds, clipped to the surface.
    let x0 = dst.x.floor().max(0.0) as i64;
    let y0 = dst.y.floor().max(0.0) as i64;
    let x1 = (dst.x + dst.w).ceil().min(out_w as f64) as i64;
    let y1 = (dst.y + dst.h).ceil().min(out_h as f64) as i64;
    if x1 <= x0 || y1 <= y0 {
        return;
    }

    let (sx, sy) = (blit.src.x as f64, blit.src.y as f64);
    let (sw, sh) = (blit.src.w.max(1) as f64, blit.src.h.max(1) as f64);
    // Valid source sample window (the crop sub-rect), clamped to the frame.
    let sx_max = (sx + sw - 1.0).min((frame.width - 1) as f64).max(sx);
    let sy_max = (sy + sh - 1.0).min((frame.height - 1) as f64).max(sy);
    // Source units per destination pixel, stepped incrementally below so the
    // inner loop avoids a divide per pixel.
    let step_x = sw / dst.w;
    let step_y = sh / dst.h;

    for dy in y0..y1 {
        // Sample centre of the destination pixel, map to source space. The
        // `-0.5` puts the coordinate in source pixel-centre space so that a 1:1
        // blit reproduces pixels exactly and a scaled one interpolates between
        // the right neighbours (bilinear) instead of snapping (nearest), which
        // is what made captured/scaled panels look blocky and "off-colour".
        let syf = (sy + (dy as f64 + 0.5 - dst.y) * step_y - 0.5).clamp(sy, sy_max);
        let iy0 = syf.floor();
        let y0s = iy0 as u32;
        let y1s = (iy0 + 1.0).min(sy_max) as u32;
        // Vertical blend weight in 1/256ths (fixed point), so the per-channel
        // blend below is integer-only, the old per-pixel `f64` math was the
        // compositor's hot spot for a scaled video panel.
        let ty = ((syf - iy0) * 256.0) as u32;
        let ity = 256 - ty;
        for dx in x0..x1 {
            let sxf = (sx + (dx as f64 + 0.5 - dst.x) * step_x - 0.5).clamp(sx, sx_max);
            let ix0 = sxf.floor();
            let x0s = ix0 as u32;
            let x1s = (ix0 + 1.0).min(sx_max) as u32;
            let tx = ((sxf - ix0) * 256.0) as u32;

            // Exact hit (1:1 region or integer-aligned sample): take the texel
            // directly, skipping three lookups and the blend. Nearest-neighbour
            // takes the same shortcut on purpose, `sxf`/`syf` already sit in
            // pixel-centre space, so rounding them picks the texel the
            // destination pixel actually lands on.
            let s = if !smooth {
                frame.pixel(
                    (sxf.round() as u32).min(sx_max as u32),
                    (syf.round() as u32).min(sy_max as u32),
                )
            } else if tx == 0 && ty == 0 {
                frame.pixel(x0s, y0s)
            } else {
                let itx = 256 - tx;
                let p00 = frame.pixel(x0s, y0s);
                let p10 = frame.pixel(x1s, y0s);
                let p01 = frame.pixel(x0s, y1s);
                let p11 = frame.pixel(x1s, y1s);
                let mut s = [0u8; 4];
                for c in 0..4 {
                    let top = p00[c] as u32 * itx + p10[c] as u32 * tx;
                    let bot = p01[c] as u32 * itx + p11[c] as u32 * tx;
                    s[c] = ((top * ity + bot * ty) >> 16) as u8;
                }
                s
            };
            let a = s[3] as u32;
            if a == 0 {
                continue;
            }
            let oi = (((dy as u32) * out_w + dx as u32) * 4) as usize;
            if a == 255 {
                out[oi] = s[0];
                out[oi + 1] = s[1];
                out[oi + 2] = s[2];
                out[oi + 3] = 255;
            } else {
                // straight alpha-over
                let ia = 255 - a;
                for c in 0..3 {
                    out[oi + c] = ((s[c] as u32 * a + out[oi + c] as u32 * ia) / 255) as u8;
                }
                out[oi + 3] = (a + (out[oi + 3] as u32) * ia / 255) as u8;
            }
        }
    }
}

/// Pack an RGBA8 buffer into the `0xAARRGGBB` u32 layout `softbuffer` expects.
pub fn rgba_to_argb_u32(rgba: &[u8], out: &mut [u32]) {
    debug_assert_eq!(rgba.len(), out.len() * 4);
    for (px, o) in rgba.as_chunks::<4>().0.iter().zip(out.iter_mut()) {
        *o = (px[3] as u32) << 24 | (px[0] as u32) << 16 | (px[1] as u32) << 8 | (px[2] as u32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Crop, Panel, Rect, ScaleMode, SourceRef};

    fn surface(w: u32, h: u32) -> (Vec<u8>, u32, u32) {
        (vec![0u8; (w * h * 4) as usize], w, h)
    }

    fn px(out: &[u8], w: u32, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * w + x) * 4) as usize;
        [out[i], out[i + 1], out[i + 2], out[i + 3]]
    }

    #[test]
    fn fills_panel_rect_and_leaves_rest_transparent() {
        let (mut out, w, h) = surface(20, 20);
        let scene = Scene {
            panels: vec![Panel {
                id: "p".into(),
                source: SourceRef::Test,
                rect: Rect::new(5.0, 5.0, 10.0, 10.0),
                crop: Crop::NONE,
                scale: ScaleMode::Fill,
                z: 0,
                monitor: Default::default(),
                passthrough: true,
                compat: false,
                passthrough_radius: 90.0,
            }],
        };
        let mut frames = HashMap::new();
        frames.insert("p".to_string(), Frame::solid(4, 4, [200, 0, 0, 255]));
        compose_into(&scene, &frames, &mut out, w, h);

        assert_eq!(px(&out, w, 10, 10), [200, 0, 0, 255]); // inside panel
        assert_eq!(px(&out, w, 0, 0), [0, 0, 0, 0]); // outside -> transparent
        assert_eq!(px(&out, w, 14, 14), [200, 0, 0, 255]); // last panel px
        assert_eq!(px(&out, w, 15, 15), [0, 0, 0, 0]); // just past panel
    }

    #[test]
    fn alpha_over_blends_with_lower_panel() {
        let (mut out, w, h) = surface(4, 4);
        let scene = Scene {
            panels: vec![
                Panel {
                    id: "bg".into(),
                    source: SourceRef::Test,
                    rect: Rect::new(0.0, 0.0, 4.0, 4.0),
                    crop: Crop::NONE,
                    scale: ScaleMode::Fill,
                    z: 0,
                    monitor: Default::default(),
                    passthrough: true,
                    compat: false,
                    passthrough_radius: 90.0,
                },
                Panel {
                    id: "fg".into(),
                    source: SourceRef::Test,
                    rect: Rect::new(0.0, 0.0, 4.0, 4.0),
                    crop: Crop::NONE,
                    scale: ScaleMode::Fill,
                    z: 1,
                    monitor: Default::default(),
                    passthrough: true,
                    compat: false,
                    passthrough_radius: 90.0,
                },
            ],
        };
        let mut frames = HashMap::new();
        frames.insert("bg".to_string(), Frame::solid(1, 1, [0, 0, 200, 255]));
        frames.insert("fg".to_string(), Frame::solid(1, 1, [200, 0, 0, 128]));
        compose_into(&scene, &frames, &mut out, w, h);
        let p = px(&out, w, 2, 2);
        assert!(p[0] > 90 && p[0] < 110, "red ~half: {p:?}");
        assert!(p[2] > 90 && p[2] < 110, "blue ~half: {p:?}");
    }

    #[test]
    fn upscaled_gradient_interpolates_monotonically() {
        // A 2px horizontal black→white frame stretched across the panel must
        // ramp smoothly (the fixed-point bilinear path), not snap between two
        // values like nearest-neighbour would.
        let (mut out, w, h) = surface(8, 1);
        let scene = Scene {
            panels: vec![Panel {
                id: "p".into(),
                source: SourceRef::Test,
                rect: Rect::new(0.0, 0.0, 8.0, 1.0),
                crop: Crop::NONE,
                scale: ScaleMode::Fill,
                z: 0,
                monitor: Default::default(),
                passthrough: true,
                compat: false,
                passthrough_radius: 90.0,
            }],
        };
        let mut frames = HashMap::new();
        frames.insert(
            "p".to_string(),
            Frame::new(2, 1, vec![0, 0, 0, 255, 254, 254, 254, 255]),
        );
        compose_into(&scene, &frames, &mut out, w, h);

        let lum: Vec<u8> = (0..8).map(|x| px(&out, w, x, 0)[0]).collect();
        assert_eq!(lum[0], 0, "left edge stays black");
        assert_eq!(lum[7], 254, "right edge stays white");
        for win in lum.windows(2) {
            assert!(win[1] >= win[0], "must be non-decreasing: {lum:?}");
        }
        assert!(
            lum[4] > 100 && lum[4] < 200,
            "midpoint interpolated: {lum:?}"
        );
    }

    #[test]
    fn argb_packing() {
        let mut out = [0u32; 1];
        rgba_to_argb_u32(&[0x12, 0x34, 0x56, 0x78], &mut out);
        assert_eq!(out[0], 0x78123456);
    }
}
