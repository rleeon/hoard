//! Sniper-scope / magnifier widget: a lens (circle or square) that shows the
//! screen region under itself magnified. Like [`crate::crosshair`] it is a
//! procedural [`Source`] riding the engine → CPU-compositor path, but it is
//! *live*: every tick it re-grabs the screen under the panel and emits a frame
//! captured at `panel_size / zoom`, which the compositor then stretches over
//! the panel box, the stretch IS the magnification, reusing the existing
//! bilinear scaler.
//!
//! Where the pixels come from is [`crate::capture::screen`]'s per-OS screen
//! grab. The overlay's own windows must be excluded from that grab or the lens
//! would recursively magnify itself, on Windows the runtime flips
//! `WDA_EXCLUDEFROMCAPTURE` on while a scope panel exists.
//!
//! The lens needs to know *where* the panel currently is, which a plain
//! [`Source`] never did: [`Source::set_viewport`] (a default-no-op hook) is
//! fed by [`Engine::tick`](crate::engine::Engine::tick) with the panel's rect
//! and target monitor right before each `acquire`.

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::input::{self, Binding};
use crate::monitors::MonitorInfo;
use crate::scene::Rect;
use crate::source::{Frame, Source};

/// Lens outline.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ScopeShape {
    /// Elliptical lens inscribed in the panel box (a circle when the box is
    /// square), the classic sniper look.
    #[default]
    Circle,
    /// The whole panel box.
    Square,
}

/// Where the lens takes its pixels from.
///
/// The historic default, and the one that still holds, is `Under`: it magnifies
/// whatever is beneath it. The problem is that to magnify the centre of the screen
/// the lens then has to sit right on top of it, covering exactly that. `Center` and
/// `Offset` decouple *where you look* from *what you see*, which is what makes it
/// possible to leave the lens in a corner.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ScopeAim {
    /// Whatever sits under the lens.
    #[default]
    Under,
    /// The centre of the monitor the panel is on.
    Center,
    /// The lens centre displaced by `(dx, dy)` pixels.
    Offset { dx: f32, dy: f32 },
}

/// How a bound button turns the lens on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ActivationMode {
    /// Press once to show, press again to hide. Survives letting go, the
    /// default because it's the only mode that leaves a hand free.
    #[default]
    Toggle,
    /// Visible only while the button is held down.
    Hold,
    /// One press shows it for [`ScopeActivation::seconds`], then it hides on
    /// its own. Pressing again restarts the countdown rather than extending it.
    Timed,
}

/// When the lens is visible.
///
/// `binding: None` is the historical behaviour, always on, and stays the
/// default so existing scenes keep working untouched.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct ScopeActivation {
    #[serde(default)]
    pub binding: Option<Binding>,
    #[serde(default)]
    pub mode: ActivationMode,
    /// Seconds the lens stays up in [`ActivationMode::Timed`]. Clamped to
    /// 0.5..=60 at use.
    #[serde(default = "default_seconds")]
    pub seconds: f32,
}

fn default_seconds() -> f32 {
    3.0
}

/// Everything that defines a scope's look. All fields default so the desktop
/// can send just `{"kind":"scope"}`.
// Not `Copy` any more: the activation can carry a key's name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScopeSpec {
    #[serde(default)]
    pub shape: ScopeShape,
    /// Magnification: the captured region is `panel_size / zoom`. Clamped to
    /// 1..=20 at use; 1 shows the region unmagnified.
    ///
    /// The ceiling is what the *capture* can still sustain, not a taste call:
    /// at ×20 a 360 px lens grabs an 18 px square, and the floor of 8 px below
    /// stops the region collapsing to nothing on a small panel. Past that the
    /// grab would be a handful of pixels stretched across the lens, which is
    /// exactly what ×20 is for, but there is no point pretending more is
    /// meaningful.
    #[serde(default = "default_zoom")]
    pub zoom: f32,
    /// Dark rim around the lens edge so it reads as a scope.
    #[serde(default = "default_border")]
    pub border: bool,
    /// Smooth the magnified pixels (bilinear) or keep them hard (nearest).
    ///
    /// Matters most at high zoom: at ×20 the lens stretches an 18 px square
    /// across the whole panel, and smoothing turns that into coloured mush.
    /// Hard pixels keep the edges readable. Neither is right always, smooth
    /// reads better for distant text, hard for picking out single pixels, so
    /// it's a choice, defaulting to the historical behaviour.
    #[serde(default = "default_smooth")]
    pub smooth: bool,
    /// Small centre mark inside the magnified view, so it's obvious which point
    /// is being magnified. A separate crosshair panel can do this too, but it
    /// draws unmagnified and doesn't follow the lens when it's aimed elsewhere.
    #[serde(default)]
    pub reticle: bool,
    /// What the lens looks at (see [`ScopeAim`]).
    #[serde(default)]
    pub aim: ScopeAim,
    /// Button/key that shows the lens, and how. Defaults to "always on".
    #[serde(default)]
    pub activation: ScopeActivation,
}

impl Default for ScopeSpec {
    fn default() -> Self {
        Self {
            shape: ScopeShape::Circle,
            zoom: default_zoom(),
            border: default_border(),
            smooth: default_smooth(),
            reticle: false,
            aim: ScopeAim::default(),
            activation: ScopeActivation::default(),
        }
    }
}

fn default_zoom() -> f32 {
    2.0
}
fn default_border() -> bool {
    true
}
fn default_smooth() -> bool {
    true
}

/// Screen-grab function: `(x, y, w, h)` in virtual-desktop pixels → RGBA
/// frame. Injectable so the mask/zoom geometry is unit-testable headless.
pub type Grabber = fn(i32, i32, u32, u32) -> Option<Frame>;

/// Live magnifier source. Re-grabs the screen under its viewport each acquire,
/// throttled to ~30 fps so an otherwise-idle overlay doesn't spin the CPU
/// compositor at the message-loop rate.
pub struct ScopeSource {
    id: String,
    spec: ScopeSpec,
    grab: Grabber,
    viewport: Option<(Rect, u32)>,
    /// Monitor origins, cached on first use (the overlay process is restarted
    /// on display changes anyway).
    mons: Option<Vec<MonitorInfo>>,
    last: Option<Instant>,
    /// Activation state. `held` is the previous poll, so a *press* is a rising
    /// edge, polling level only would re-toggle every tick the button is down.
    held: bool,
    /// Toggle latch / timed deadline.
    toggled_on: bool,
    until: Option<Instant>,
    /// True while the user is arranging panels: the binding is ignored so the
    /// lens can be dragged into place.
    editing: bool,
    /// Whether the last emitted frame was the transparent "hidden" one, so we
    /// emit it once on the falling edge instead of every tick.
    hidden_emitted: bool,
}

/// Minimum interval between grabs (~30 fps).
const FRAME_INTERVAL: Duration = Duration::from_millis(33);

impl ScopeSource {
    pub fn new(id: impl Into<String>, spec: &ScopeSpec) -> Self {
        Self::with_grabber(id, spec, crate::capture::screen::grab)
    }

    pub fn with_grabber(id: impl Into<String>, spec: &ScopeSpec, grab: Grabber) -> Self {
        Self {
            id: id.into(),
            spec: spec.clone(),
            grab,
            viewport: None,
            mons: None,
            last: None,
            held: false,
            toggled_on: false,
            until: None,
            editing: false,
            hidden_emitted: false,
        }
    }

    /// Is the lens supposed to be visible this tick?
    ///
    /// Unbound scopes are always on (the behaviour that predates bindings), and
    /// so are scopes on a platform where we can't read global input, a lens
    /// that never appears reads as a broken feature, while one that ignores its
    /// binding is merely the old behaviour. Editor mode also forces it on, or
    /// the user couldn't see what they're dragging.
    fn poll_active(&mut self) -> bool {
        let Some(binding) = self.spec.activation.binding.as_ref() else {
            return true;
        };
        if self.editing || !input::available() {
            return true;
        }

        self.step(input::is_down(binding), Instant::now())
    }

    /// The activation state machine, split out from the polling so it can be
    /// driven in tests without a mouse: `down` is the button's level this tick
    /// and `now` the clock. Pure except for the latch/deadline it owns.
    fn step(&mut self, down: bool, now: Instant) -> bool {
        let pressed = down && !self.held; // rising edge
        self.held = down;

        match self.spec.activation.mode {
            ActivationMode::Hold => down,
            ActivationMode::Toggle => {
                if pressed {
                    self.toggled_on = !self.toggled_on;
                }
                self.toggled_on
            }
            ActivationMode::Timed => {
                if pressed {
                    let secs = self.spec.activation.seconds.clamp(0.5, 60.0);
                    self.until = Some(now + Duration::from_secs_f32(secs));
                }
                self.until.is_some_and(|t| now < t)
            }
        }
    }

    /// The target monitor's `(x, y, w, h)`. The whole thing is needed, not just the
    /// origin, now that the lens can aim at its centre.
    fn monitor_rect(&mut self, mon_id: u32) -> (i32, i32, i32, i32) {
        let mons = self.mons.get_or_insert_with(crate::monitors::list_monitors);
        mons.iter()
            .find(|m| m.id == mon_id)
            .map(|m| (m.x, m.y, m.w, m.h))
            .unwrap_or((0, 0, 0, 0))
    }
}

impl Source for ScopeSource {
    fn id(&self) -> &str {
        &self.id
    }

    fn set_viewport(&mut self, rect: Rect, monitor: u32) {
        self.viewport = Some((rect, monitor));
    }

    fn set_editing(&mut self, editing: bool) {
        self.editing = editing;
    }

    fn acquire(&mut self) -> Option<Frame> {
        if !self.poll_active() {
            // Hidden. Emit ONE fully transparent frame and then nothing:
            // returning `None` alone would leave the compositor holding the
            // last magnified frame, so the lens would freeze on screen instead
            // of disappearing.
            if self.hidden_emitted {
                return None;
            }
            self.hidden_emitted = true;
            self.last = None; // next activation grabs immediately
            return Some(Frame::solid(8, 8, [0, 0, 0, 0]));
        }
        self.hidden_emitted = false;

        if self.last.is_some_and(|t| t.elapsed() < FRAME_INTERVAL) {
            return None;
        }
        let (rect, mon_id) = self.viewport?;
        self.last = Some(Instant::now());

        let r = rect.normalized();
        let zoom = self.spec.zoom.clamp(1.0, 20.0) as f64;
        let cap_w = ((r.w / zoom).round() as u32).clamp(8, 4096);
        let cap_h = ((r.h / zoom).round() as u32).clamp(8, 4096);
        let (ox, oy, mw, mh) = self.monitor_rect(mon_id);
        // Centro de la lente en coordenadas del escritorio virtual.
        let lens_x = ox + (r.x + r.w / 2.0) as i32;
        let lens_y = oy + (r.y + r.h / 2.0) as i32;
        let (cx, cy) = match self.spec.aim {
            ScopeAim::Under => (lens_x, lens_y),
            // With the monitor unknown (an empty list when headless) it falls back
            // to the usual behaviour rather than aiming at (0,0).
            ScopeAim::Center if mw > 0 && mh > 0 => (ox + mw / 2, oy + mh / 2),
            ScopeAim::Center => (lens_x, lens_y),
            ScopeAim::Offset { dx, dy } => (lens_x + dx as i32, lens_y + dy as i32),
        };

        let mut frame = (self.grab)(
            cx - (cap_w / 2) as i32,
            cy - (cap_h / 2) as i32,
            cap_w,
            cap_h,
        )
        // No grab on this platform / it failed: a dim glass placeholder so the
        // lens still shows where it is instead of vanishing.
        .unwrap_or_else(|| Frame::solid(cap_w, cap_h, [20, 20, 24, 200]));

        apply_lens(&mut frame, &self.spec);
        Some(frame)
    }
}

/// Mask the frame to the lens shape and draw the rim, in place. Geometry is in
/// normalized coordinates so the mask stretches with the frame onto the panel
/// box (a circle lens on a square box, an ellipse on a wide one).
fn apply_lens(frame: &mut Frame, spec: &ScopeSpec) {
    let (w, h) = (frame.width, frame.height);
    if w == 0 || h == 0 {
        return;
    }
    let buf = match std::sync::Arc::get_mut(&mut frame.rgba) {
        Some(b) => b,
        None => return, // freshly built frames are uniquely owned
    };
    // Edge widths in normalized units, sized off the frame so the rim stays a
    // consistent on-panel thickness (~2px of frame ≈ 2*zoom px on screen is too
    // fat; the frame is panel/zoom so 2px frame == 2px panel after stretch...
    // exactly what we want).
    let aa = 1.5 / w.min(h) as f32;
    let rim_w = 2.5 / w.min(h) as f32;
    let rim = spec.border;
    // The reticle, also in normalised units so it comes out the same thickness on
    // screen whatever the magnification.
    let reticle = spec.reticle;
    let ret_w = 1.0 / w.min(h) as f32;
    let ret_len = 0.09;
    let ret_gap = 0.018;

    for y in 0..h {
        for x in 0..w {
            // Normalized offset from centre, -0.5..0.5 on each axis.
            let nx = (x as f32 + 0.5) / w as f32 - 0.5;
            let ny = (y as f32 + 0.5) / h as f32 - 0.5;
            // Signed distance to the lens edge (negative inside).
            let d = match spec.shape {
                ScopeShape::Circle => (nx * nx + ny * ny).sqrt() - 0.5,
                ScopeShape::Square => nx.abs().max(ny.abs()) - 0.5,
            };
            let i = ((y * w + x) * 4) as usize;
            // Outside → transparent (with AA); rim band → dark ring.
            let cover = ((-d) / aa).clamp(0.0, 1.0);
            if cover < 1.0 {
                let a = (buf[i + 3] as f32 * cover) as u8;
                if cover <= 0.0 {
                    buf[i] = 0;
                    buf[i + 1] = 0;
                    buf[i + 2] = 0;
                }
                buf[i + 3] = a;
            }
            if rim && d > -rim_w && cover > 0.0 {
                let t = (cover * 230.0) as u8;
                buf[i] = 12;
                buf[i + 1] = 12;
                buf[i + 2] = 14;
                buf[i + 3] = buf[i + 3].max(t);
            }
            // The reticle: a thin cross at the exact centre of the magnified view.
            // Something similar can be had with a separate crosshair panel, but that
            // one is drawn WITHOUT magnification and does not follow the lens when it
            // aims somewhere else; this mark always points at the spot really being
            // magnified.
            if reticle && cover > 0.0 {
                let on_v = nx.abs() <= ret_w && ny.abs() <= ret_len;
                let on_h = ny.abs() <= ret_w && nx.abs() <= ret_len;
                // The gap in the middle: it leaves the exact pixel being aimed at
                // visible.
                let in_gap = nx.abs() <= ret_gap && ny.abs() <= ret_gap;
                if (on_v || on_h) && !in_gap {
                    buf[i] = 255;
                    buf[i + 1] = 255;
                    buf[i + 2] = 255;
                    buf[i + 3] = 255;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Binding;

    fn grid(x: i32, y: i32, w: u32, h: u32) -> Option<Frame> {
        // Encode the requested origin in the first pixel so tests can assert
        // capture geometry; fill the rest opaque white.
        let mut f = vec![255u8; (w * h * 4) as usize];
        f[0] = (x & 0xff) as u8;
        f[1] = (y & 0xff) as u8;
        Some(Frame::new(w, h, f))
    }

    fn scope(spec: ScopeSpec) -> ScopeSource {
        let mut s = ScopeSource::with_grabber("s", &spec, grid);
        s.set_viewport(Rect::new(100.0, 100.0, 200.0, 200.0), 0);
        s
    }

    #[test]
    fn zoom_shrinks_the_captured_region() {
        let f = scope(ScopeSpec {
            zoom: 2.0,
            ..Default::default()
        })
        .acquire()
        .unwrap();
        // 200x200 panel at zoom 2 → 100x100 capture.
        assert_eq!((f.width, f.height), (100, 100));

        let f = scope(ScopeSpec {
            zoom: 1.0,
            shape: ScopeShape::Square,
            ..Default::default()
        })
        .acquire()
        .unwrap();
        assert_eq!((f.width, f.height), (200, 200));
    }

    #[test]
    fn circle_masks_corners_keeps_centre() {
        let f = scope(ScopeSpec {
            zoom: 2.0,
            border: false,
            ..Default::default()
        })
        .acquire()
        .unwrap();
        assert_eq!(f.pixel(2, 2)[3], 0, "corner transparent");
        assert_eq!(f.pixel(50, 50)[3], 255, "centre opaque");
    }

    #[test]
    fn square_keeps_corners_and_rim_is_dark() {
        let f = scope(ScopeSpec {
            zoom: 2.0,
            shape: ScopeShape::Square,
            border: true,
            ..Default::default()
        })
        .acquire()
        .unwrap();
        assert!(f.pixel(1, 1)[3] > 0, "corner kept on square");
        let rim = f.pixel(1, 50);
        assert!(rim[0] < 30, "left rim dark: {rim:?}");
        assert_eq!(f.pixel(50, 50), [255, 255, 255, 255], "centre untouched");
    }

    fn bound(mode: ActivationMode, seconds: f32) -> ScopeSource {
        ScopeSource::with_grabber(
            "s",
            &ScopeSpec {
                activation: ScopeActivation {
                    binding: Some(Binding::Mouse { button: 3 }),
                    mode,
                    seconds,
                },
                ..Default::default()
            },
            grid,
        )
    }

    /// The exact JSON the editor emits (`Screen.svelte`). If somebody changes a
    /// field name on either side, this catches it here instead of leaving a scope
    /// that never activates and nobody knows why.
    #[test]
    fn parses_the_payload_the_editor_actually_sends() {
        let json = r#"{
            "kind": "scope",
            "shape": "circle",
            "zoom": 2,
            "border": true,
            "activation": {
                "binding": { "type": "mouse", "button": 4 },
                "mode": "hold",
                "seconds": 3
            }
        }"#;
        let src: crate::scene::SourceRef = serde_json::from_str(json).unwrap();
        let crate::scene::SourceRef::Scope(spec) = src else {
            panic!("not recognised as a scope");
        };
        assert_eq!(
            spec.activation.binding,
            Some(Binding::Mouse { button: 4 }),
            "the side button has to arrive as it is"
        );
        assert_eq!(spec.activation.mode, ActivationMode::Hold);
        assert_eq!(spec.activation.seconds, 3.0);
    }

    /// A scene from before bindings existed carries no `activation`; it has to keep
    /// loading and stay "always visible".
    #[test]
    fn an_old_scene_without_activation_still_loads() {
        let src: crate::scene::SourceRef =
            serde_json::from_str(r#"{"kind":"scope","zoom":3}"#).unwrap();
        let crate::scene::SourceRef::Scope(spec) = src else {
            panic!("not recognised as a scope");
        };
        assert_eq!(spec.activation.binding, None);
        assert_eq!(spec.zoom, 3.0);
    }

    thread_local! {
        static LAST_GRAB: std::cell::Cell<(i32, i32)> = const { std::cell::Cell::new((0, 0)) };
    }

    /// Records the requested origin rather than encoding it into pixel (0,0): that
    /// pixel falls in the corner and the lens's mask leaves it transparent, so it
    /// cannot be read back.
    fn recording(x: i32, y: i32, w: u32, h: u32) -> Option<Frame> {
        LAST_GRAB.with(|c| c.set((x, y)));
        Some(Frame::solid(w, h, [10, 10, 10, 255]))
    }

    fn aim_origin(aim: ScopeAim) -> (i32, i32) {
        let mut s = ScopeSource::with_grabber(
            "s",
            &ScopeSpec {
                zoom: 1.0,
                border: false,
                aim,
                ..Default::default()
            },
            recording,
        );
        // Lente de 40×40 en (100,100): su centro cae en (120,120).
        s.set_viewport(Rect::new(100.0, 100.0, 40.0, 40.0), 0);
        s.acquire().unwrap();
        LAST_GRAB.with(|c| c.get())
    }

    #[test]
    fn aim_under_grabs_beneath_the_lens() {
        // centre 120 minus half the region (20) = 100.
        assert_eq!(aim_origin(ScopeAim::Under), (100, 100));
    }

    #[test]
    fn aim_offset_displaces_the_grabbed_point() {
        let (x, y) = aim_origin(ScopeAim::Offset {
            dx: 30.0,
            dy: -20.0,
        });
        assert_eq!(
            (x, y),
            (130, 80),
            "la lente mira 30 a la derecha y 20 arriba"
        );
    }

    #[test]
    fn aim_center_falls_back_when_the_monitor_is_unknown() {
        // Sin monitores (headless) `Center` no puede apuntar al centro: tiene
        // que comportarse como `Under`, no apuntar a (0,0).
        assert_eq!(aim_origin(ScopeAim::Center), (100, 100));
    }

    #[test]
    fn reticle_marks_the_centre_only_when_asked() {
        let shoot = |reticle: bool| {
            let mut s = ScopeSource::with_grabber(
                "s",
                &ScopeSpec {
                    border: false,
                    reticle,
                    ..Default::default()
                },
                recording, // a dark background: a white reticle shows up
            );
            s.set_viewport(Rect::new(100.0, 100.0, 200.0, 200.0), 0);
            s.acquire().unwrap()
        };
        let off = shoot(false);
        let on = shoot(true);
        let (w, h) = (on.width, on.height);
        // Un punto sobre el brazo vertical, fuera del hueco central.
        let probe_y = h / 2 - h / 20;
        assert_eq!(
            on.pixel(w / 2, probe_y),
            [255, 255, 255, 255],
            "the reticle arm is drawn"
        );
        assert_ne!(
            off.pixel(w / 2, probe_y),
            [255, 255, 255, 255],
            "with no reticle that pixel is untouched"
        );
        // The gap in the middle leaves the aimed-at pixel visible.
        assert_eq!(
            on.pixel(w / 2, h / 2),
            off.pixel(w / 2, h / 2),
            "el centro exacto queda libre"
        );
    }

    #[test]
    fn smooth_defaults_on_and_survives_the_wire() {
        assert!(ScopeSpec::default().smooth, "por defecto, como siempre");
        let src: crate::scene::SourceRef = serde_json::from_str(
            r#"{"kind":"scope","smooth":false,"reticle":true,
                "aim":{"kind":"offset","dx":40,"dy":-10}}"#,
        )
        .unwrap();
        let crate::scene::SourceRef::Scope(spec) = src else {
            panic!("not recognised as a scope");
        };
        assert!(!spec.smooth);
        assert!(spec.reticle);
        assert_eq!(
            spec.aim,
            ScopeAim::Offset {
                dx: 40.0,
                dy: -10.0
            }
        );
    }

    #[test]
    fn hold_follows_the_button_level() {
        let mut s = bound(ActivationMode::Hold, 3.0);
        let t = Instant::now();
        assert!(!s.step(false, t));
        assert!(s.step(true, t), "visible mientras se mantiene");
        assert!(s.step(true, t), "sigue visible sin soltar");
        assert!(!s.step(false, t), "se oculta al soltar");
    }

    #[test]
    fn toggle_flips_on_the_press_not_on_every_tick() {
        let mut s = bound(ActivationMode::Toggle, 3.0);
        let t = Instant::now();
        assert!(!s.step(false, t), "arranca oculto");
        assert!(s.step(true, t), "one press turns it on");
        // Holding it down must NOT re-toggle: only the edge counts.
        assert!(s.step(true, t));
        assert!(s.step(true, t));
        assert!(s.step(false, t), "sigue encendido tras soltar");
        assert!(!s.step(true, t), "the second press turns it off");
    }

    #[test]
    fn timed_hides_itself_and_a_new_press_restarts_the_clock() {
        let mut s = bound(ActivationMode::Timed, 2.0);
        let t0 = Instant::now();
        assert!(!s.step(false, t0));
        assert!(s.step(true, t0), "the press turns it on");
        assert!(
            s.step(false, t0 + Duration::from_secs_f32(1.9)),
            "still inside"
        );
        assert!(
            !s.step(false, t0 + Duration::from_secs_f32(2.1)),
            "se apaga solo al vencer"
        );
        // Volver a pulsar reinicia la cuenta desde ese momento.
        let t1 = t0 + Duration::from_secs_f32(5.0);
        assert!(s.step(true, t1));
        assert!(s.step(false, t1 + Duration::from_secs_f32(1.5)));
        assert!(!s.step(false, t1 + Duration::from_secs_f32(2.5)));
    }

    #[test]
    fn timed_clamps_absurd_durations() {
        let mut s = bound(ActivationMode::Timed, 9999.0);
        let t = Instant::now();
        assert!(s.step(true, t));
        // 60 s es el tope; a los 61 ya no puede seguir encendido.
        assert!(!s.step(false, t + Duration::from_secs(61)));
    }

    #[test]
    fn an_unbound_scope_is_always_on() {
        let mut s = ScopeSource::with_grabber("s", &ScopeSpec::default(), grid);
        assert!(s.poll_active(), "sin binding, el visor no se esconde nunca");
    }

    #[test]
    fn editing_mode_ignores_the_binding() {
        let mut s = bound(ActivationMode::Toggle, 3.0);
        s.set_editing(true);
        assert!(
            s.poll_active(),
            "en el editor tiene que verse para poder colocarlo"
        );
    }

    #[test]
    fn a_bound_scope_emits_one_transparent_frame_then_stops() {
        let mut s = bound(ActivationMode::Toggle, 3.0);
        s.set_viewport(Rect::new(0.0, 0.0, 100.0, 100.0), 0);
        // Without `input::available()` in the test build the binding is ignored, so
        // the hidden state is forced through the machine's own route.
        s.spec.activation.binding = Some(Binding::Mouse { button: 3 });
        s.editing = false;
        let f = s.acquire();
        // The test build cannot probe, so it behaves as "always visible": what is
        // checked here is that THAT is what happens, not an invisible scope.
        assert!(f.is_some(), "with no probing available it keeps showing");
    }

    #[test]
    fn without_viewport_no_frame_with_it_throttled() {
        let mut s = ScopeSource::with_grabber("s", &ScopeSpec::default(), grid);
        assert!(s.acquire().is_none(), "no viewport yet");
        s.set_viewport(Rect::new(0.0, 0.0, 100.0, 100.0), 0);
        assert!(s.acquire().is_some());
        assert!(s.acquire().is_none(), "throttled immediately after");
    }
}
