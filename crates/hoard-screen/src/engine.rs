//! The frame engine: owns the live sources behind the current [`Scene`], ticks
//! them, and composites a frame. This is the platform-agnostic heart that both
//! the headless snapshot tool and the real windowed runtime drive, the only
//! difference between them is where the composited buffer goes (a file vs an
//! on-screen surface).

use std::collections::HashMap;

use crate::capture::{self, CaptureBackend};
use crate::compositor::compose_into;
use crate::scene::{Scene, SourceRef};
use crate::source::{Frame, Source, TestPattern};

/// Holds one live [`Source`] per panel plus its latest frame.
pub struct Engine {
    backend: Box<dyn CaptureBackend>,
    scene: Scene,
    sources: HashMap<String, Box<dyn Source>>,
    frames: HashMap<String, Frame>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            backend: capture::backend(),
            scene: Scene::default(),
            sources: HashMap::new(),
            frames: HashMap::new(),
        }
    }

    pub fn backend_name(&self) -> &'static str {
        self.backend.name()
    }

    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    /// Mutable access to the live scene for in-overlay editing (drag / resize a
    /// panel's `rect` directly). Only geometry is touched here, never a panel's
    /// `source`, so no source reconciliation is needed, unlike [`set_scene`].
    pub fn scene_mut(&mut self) -> &mut Scene {
        &mut self.scene
    }

    /// Apply a new layout, reconciling live sources: keep a source only when its
    /// panel id still exists *and* its source descriptor is unchanged, drop the
    /// rest, open new ones. Comparing the whole descriptor (not just the id)
    /// matters when a panel keeps its id but now points at a different
    /// window/image/note, the stale source must be dropped and reopened.
    pub fn set_scene(&mut self, scene: Scene) {
        // Source descriptor per panel id from the *outgoing* scene, so we can
        // tell an unchanged source from one swapped under a reused id.
        let old_src: HashMap<String, SourceRef> = self
            .scene
            .panels
            .iter()
            .map(|p| (p.id.clone(), p.source.clone()))
            .collect();
        let new_src: HashMap<&str, &SourceRef> = scene
            .panels
            .iter()
            .map(|p| (p.id.as_str(), &p.source))
            .collect();

        self.sources
            .retain(|id, _| match (new_src.get(id.as_str()), old_src.get(id)) {
                (Some(new), Some(old)) => **new == *old,
                _ => false,
            });
        self.frames.retain(|id, _| self.sources.contains_key(id));

        for panel in &scene.panels {
            if !self.sources.contains_key(&panel.id) {
                let src = self.open_source(&panel.id, &panel.source);
                self.sources.insert(panel.id.clone(), src);
            }
        }
        self.scene = scene;
    }

    /// Open a [`Source`] for a panel. Capture failures (unimplemented backend,
    /// denied permission) degrade to a labelled placeholder so the panel still
    /// shows *something* ("connecting") instead of vanishing.
    fn open_source(&self, panel_id: &str, src: &SourceRef) -> Box<dyn Source> {
        match src {
            SourceRef::Test => Box::new(TestPattern::new(panel_id.to_string(), 320, 180)),
            SourceRef::Window { id } => match self.backend.capture(id) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("hoard-screen: capture({id}) failed: {e}; placeholder");
                    Box::new(Placeholder::new(panel_id, [40, 40, 48, 255]))
                }
            },
            SourceRef::Crosshair(spec) => {
                Box::new(crate::crosshair::CrosshairSource::new(panel_id, spec))
            }
            SourceRef::Scope(spec) => Box::new(crate::scope::ScopeSource::new(panel_id, spec)),
            // Image decode and note text rendering land here; placeholder for now.
            SourceRef::Image { .. } => Box::new(Placeholder::new(panel_id, [24, 24, 28, 255])),
            SourceRef::Note { .. } => Box::new(Placeholder::new(panel_id, [18, 18, 22, 230])),
            // A kind from a newer editor: draw nothing (fully transparent) so
            // the rest of the scene stays intact.
            SourceRef::Unknown => Box::new(Placeholder::new(panel_id, [0, 0, 0, 0])),
        }
    }

    /// Pull a fresh frame from every source that has one. Returns `true` if any
    /// source produced a new frame this tick, so the windowed runtime can skip
    /// recompositing + presenting when nothing changed (a paused video, a static
    /// note, a capture running below the overlay's frame rate).
    ///
    /// Each source first learns where its panel currently sits (rect + target
    /// monitor) via [`Source::set_viewport`], so screen-sampling sources (the
    /// scope) follow their panel as it is dragged or resized.
    pub fn tick(&mut self) -> bool {
        let mut changed = false;
        for panel in &self.scene.panels {
            let Some(src) = self.sources.get_mut(&panel.id) else {
                continue;
            };
            let mon = match panel.monitor {
                crate::scene::PanelTarget::Monitor { id } => id,
                crate::scene::PanelTarget::All => 0,
            };
            src.set_viewport(panel.rect, mon);
            if let Some(f) = src.acquire() {
                self.frames.insert(panel.id.clone(), f);
                changed = true;
            }
        }
        changed
    }

    /// Tell every live source whether the overlay is in Editor mode. Cheap
    /// enough to call on every mode change; sources that don't care no-op.
    pub fn set_editing(&mut self, editing: bool) {
        for src in self.sources.values_mut() {
            src.set_editing(editing);
        }
    }

    /// Composite the current frames into `out` (RGBA8, `w*h*4` bytes).
    pub fn render(&self, out: &mut [u8], w: u32, h: u32) {
        compose_into(&self.scene, &self.frames, out, w, h);
    }

    /// Composite only the panels targeting monitor `mon_id` (or mirrored to all)
    /// into `out`. Panel rects are monitor-local, so `out` is that monitor's own
    /// surface with `(0,0)` at its top-left, the multi-monitor render path.
    pub fn render_monitor(&self, out: &mut [u8], w: u32, h: u32, mon_id: u32) {
        let panels = self
            .scene
            .panels
            .iter()
            .filter(|p| p.monitor.draws_on(mon_id))
            .cloned()
            .collect();
        let sub = Scene { panels };
        compose_into(&sub, &self.frames, out, w, h);
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

/// A solid placeholder source (used while a capture connects or for not-yet
/// implemented source kinds). Emits one frame, then `None`.
struct Placeholder {
    id: String,
    rgba: [u8; 4],
    emitted: bool,
}

impl Placeholder {
    fn new(id: &str, rgba: [u8; 4]) -> Self {
        Self {
            id: id.to_string(),
            rgba,
            emitted: false,
        }
    }
}

impl Source for Placeholder {
    fn id(&self) -> &str {
        &self.id
    }
    fn acquire(&mut self) -> Option<Frame> {
        if self.emitted {
            return None;
        }
        self.emitted = true;
        Some(Frame::solid(8, 8, self.rgba))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Crop, Panel, Rect, ScaleMode};

    fn test_panel(id: &str) -> Panel {
        Panel {
            id: id.into(),
            source: SourceRef::Test,
            rect: Rect::new(0.0, 0.0, 64.0, 64.0),
            crop: Crop::NONE,
            scale: ScaleMode::Fill,
            z: 0,
            monitor: Default::default(),
            passthrough: true,
            compat: false,
            passthrough_radius: 90.0,
        }
    }

    #[test]
    fn set_scene_opens_and_reconciles_sources() {
        let mut e = Engine::new();
        e.set_scene(Scene {
            panels: vec![test_panel("a"), test_panel("b")],
        });
        e.tick();
        assert!(e.frames.contains_key("a") && e.frames.contains_key("b"));

        // Drop "b": its source and frame are reclaimed.
        e.set_scene(Scene {
            panels: vec![test_panel("a")],
        });
        assert!(e.sources.contains_key("a"));
        assert!(!e.sources.contains_key("b"));
        assert!(!e.frames.contains_key("b"));
    }

    #[test]
    fn swapping_source_under_a_reused_id_reopens() {
        let mut e = Engine::new();
        e.set_scene(Scene {
            panels: vec![test_panel("a")],
        });
        e.tick();
        assert!(e.frames.contains_key("a"));

        // Same panel id, different source descriptor (Test -> Note): the stale
        // source and its frame must be dropped (reconcile by descriptor, not id).
        let mut p = test_panel("a");
        p.source = SourceRef::Note { text: "hi".into() };
        e.set_scene(Scene { panels: vec![p] });
        assert!(e.sources.contains_key("a")); // reopened
        assert!(!e.frames.contains_key("a")); // old frame gone, new one not ticked yet
    }

    #[test]
    fn renders_a_non_empty_frame() {
        let mut e = Engine::new();
        e.set_scene(Scene {
            panels: vec![test_panel("a")],
        });
        e.tick();
        let (w, h) = (64u32, 64u32);
        let mut out = vec![0u8; (w * h * 4) as usize];
        e.render(&mut out, w, h);
        // At least one fully-opaque pixel was drawn.
        assert!(out.chunks_exact(4).any(|p| p[3] == 255));
    }

    #[test]
    fn window_source_degrades_to_placeholder_when_backend_unimplemented() {
        let mut e = Engine::new();
        let mut p = test_panel("w");
        p.source = SourceRef::Window {
            id: "0xdead".into(),
        };
        e.set_scene(Scene { panels: vec![p] });
        e.tick();
        // Placeholder emitted a frame; the panel did not disappear.
        assert!(e.frames.contains_key("w"));
    }
}
