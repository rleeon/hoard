//! Frame sources, anything that can hand the compositor RGBA pixels.
//!
//! A window capture, a decoded image and a rendered note all implement
//! [`Source`]. The compositor only ever asks a source for its latest [`Frame`];
//! it does not care where the pixels came from. This is the seam the
//! per-platform capture backends ([`crate::capture`]) plug into.

/// A frame of tightly-packed RGBA8 pixels (`rgba.len() == width*height*4`),
/// top-row first. Alpha is honoured by the compositor so a source can be
/// partially transparent.
///
/// `rgba` is an `Arc<[u8]>` so handing the latest frame to the compositor
/// (`Source::acquire` → `Engine::tick`) is a pointer clone, not a multi-MB
/// memcpy every tick, a 1080p frame is ~8 MB and the old `Vec` clone showed up
/// directly in the overlay's per-frame CPU cost.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub rgba: std::sync::Arc<[u8]>,
}

impl Frame {
    pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> Self {
        debug_assert_eq!(rgba.len(), (width as usize) * (height as usize) * 4);
        Self {
            width,
            height,
            rgba: rgba.into(),
        }
    }

    /// A solid-colour frame; handy for tests and as a placeholder while a real
    /// capture is still connecting.
    pub fn solid(width: u32, height: u32, rgba: [u8; 4]) -> Self {
        let mut buf = Vec::with_capacity((width * height * 4) as usize);
        for _ in 0..(width * height) {
            buf.extend_from_slice(&rgba);
        }
        Self::new(width, height, buf)
    }

    #[inline]
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * self.width + x) * 4) as usize;
        [
            self.rgba[i],
            self.rgba[i + 1],
            self.rgba[i + 2],
            self.rgba[i + 3],
        ]
    }
}

/// A live producer of frames. `acquire` returns the newest frame if one is
/// ready, or `None` to mean "no new frame, keep showing the last one", so a
/// 24fps capture doesn't force the overlay to redraw at the source's rate.
pub trait Source: Send {
    fn id(&self) -> &str;
    fn acquire(&mut self) -> Option<Frame>;
    /// Where this source's panel currently sits (monitor-local rect + target
    /// monitor id). Fed by the engine right before each `acquire`, so a source
    /// that samples the *screen* (the scope/magnifier) tracks its panel as the
    /// user drags it. Most sources don't care, default is a no-op.
    fn set_viewport(&mut self, _rect: crate::scene::Rect, _monitor: u32) {}
    /// Whether the overlay is in Editor mode. Fed by the engine when the mode
    /// changes. A source that hides itself behind a binding (the scope) must
    /// ignore that binding while the user is arranging panels, otherwise the
    /// lens is invisible exactly when it needs to be dragged into place.
    /// Default is a no-op; most sources don't care.
    fn set_editing(&mut self, _editing: bool) {}
}

/// Animated test pattern: a moving diagonal gradient. Drives the entire
/// pipeline (window + compositor + crop) on a real desktop with no capture
/// backend, so the GPU/window path can be validated before any OS capture work.
pub struct TestPattern {
    id: String,
    width: u32,
    height: u32,
    tick: u32,
}

impl TestPattern {
    pub fn new(id: impl Into<String>, width: u32, height: u32) -> Self {
        Self {
            id: id.into(),
            width,
            height,
            tick: 0,
        }
    }
}

impl Source for TestPattern {
    fn id(&self) -> &str {
        &self.id
    }

    fn acquire(&mut self) -> Option<Frame> {
        self.tick = self.tick.wrapping_add(4);
        let (w, h, t) = (self.width, self.height, self.tick);
        let mut buf = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
                buf[i] = ((x + t) & 0xff) as u8;
                buf[i + 1] = ((y + t) & 0xff) as u8;
                buf[i + 2] = ((x + y) & 0xff) as u8;
                buf[i + 3] = 0xff;
            }
        }
        Some(Frame::new(w, h, buf))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solid_frame_is_uniform() {
        let f = Frame::solid(3, 2, [10, 20, 30, 255]);
        assert_eq!(f.rgba.len(), 3 * 2 * 4);
        assert_eq!(f.pixel(2, 1), [10, 20, 30, 255]);
    }

    #[test]
    fn test_pattern_produces_full_frames() {
        let mut s = TestPattern::new("t", 16, 8);
        let f = s.acquire().unwrap();
        assert_eq!((f.width, f.height), (16, 8));
        assert_eq!(f.rgba.len(), 16 * 8 * 4);
    }
}
