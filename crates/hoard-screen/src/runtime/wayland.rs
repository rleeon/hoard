//! Wayland overlay placement via **wlr-layer-shell** (`feature = "wayland"`).
//!
//! Creates a fullscreen surface on the `Overlay` layer, above normal windows,
//! over the game, and paints the composed [`Engine`] frame into a `wl_shm`
//! buffer each vsync. Click-through is the surface's **input region**: empty in
//! View (all input falls through to the game), full in Editor.
//!
//! ## Compositor coverage
//! `wlr-layer-shell` is implemented by wlroots (Hyprland, sway), KWin (KDE) and
//! Cosmic, so this path is the native overlay there. **GNOME/Mutter refuses
//! layer-shell**, and Wayland grants no app-level "always on top over a
//! fullscreen client" otherwise, so the universal/GNOME route is to run the
//! game (or this overlay) under **gamescope**, a nested compositor that *does*
//! speak layer-shell. See [`gamescope_hint`].
//!
//! ## The Ctrl+O problem
//! Wayland has no global key grab: a click-through surface never holds keyboard
//! focus, so it cannot see Ctrl+O in View mode. The toggle therefore arrives
//! over IPC (`set_editor`, sent by the desktop app, which owns the real global
//! shortcut via the GlobalShortcuts portal). While the surface *does* hold focus
//! in Editor mode we also honor Ctrl+O / Esc locally to drop back to View.

use std::collections::VecDeque;
use std::io::BufRead;
use std::sync::{Arc, Mutex};

use smithay_client_toolkit::reexports::client::{
    globals::registry_queue_init,
    protocol::{wl_keyboard, wl_output, wl_seat, wl_shm, wl_surface},
    Connection, QueueHandle,
};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState, Region},
    delegate_compositor, delegate_keyboard, delegate_layer, delegate_output, delegate_registry,
    delegate_seat, delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers},
        Capability, SeatHandler, SeatState,
    },
    shell::{
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
        WaylandSurface,
    },
    shm::{slot::SlotPool, Shm, ShmHandler},
};

use crate::engine::Engine;
use crate::ipc::{parse_line, Message};
use crate::mode::Mode;

/// One line for the logs when layer-shell is missing, pointing at the fix.
pub fn gamescope_hint() -> &'static str {
    "wlr-layer-shell unavailable (likely GNOME/Mutter). Run under gamescope, \
     e.g. `gamescope -- <game>` and launch hoard-screen inside it."
}

/// Run the Wayland overlay. Blocks until the surface is closed or `quit`.
pub fn run(engine: Engine) -> Result<(), String> {
    let conn = Connection::connect_to_env()
        .map_err(|e| format!("wayland connect: {e} (no WAYLAND_DISPLAY?)"))?;
    let (globals, mut event_queue) =
        registry_queue_init(&conn).map_err(|e| format!("wayland registry: {e}"))?;
    let qh = event_queue.handle();

    let compositor =
        CompositorState::bind(&globals, &qh).map_err(|e| format!("wl_compositor: {e}"))?;
    let layer_shell =
        LayerShell::bind(&globals, &qh).map_err(|e| format!("{}: {e}", gamescope_hint()))?;
    let shm = Shm::bind(&globals, &qh).map_err(|e| format!("wl_shm: {e}"))?;

    let surface = compositor.create_surface(&qh);
    let layer =
        layer_shell.create_layer_surface(&qh, surface, Layer::Overlay, Some("hoard-screen"), None);
    // Cover the whole output; -1 exclusive zone = ignore other panels' margins
    // and fill the screen edge-to-edge.
    layer.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
    layer.set_exclusive_zone(-1);
    layer.set_keyboard_interactivity(KeyboardInteractivity::OnDemand);
    layer.set_size(0, 0);
    layer.commit();

    let pool = SlotPool::new(256 * 256 * 4, &shm).map_err(|e| format!("shm pool: {e}"))?;

    let mut state = Overlay {
        registry_state: RegistryState::new(&globals),
        seat_state: SeatState::new(&globals, &qh),
        output_state: OutputState::new(&globals, &qh),
        shm,
        compositor,
        pool,
        layer,
        keyboard: None,
        modifiers: Modifiers::default(),
        width: 0,
        height: 0,
        configured: false,
        exit: false,
        mode: Mode::View,
        engine,
        ipc: spawn_stdin_reader(),
    };

    loop {
        event_queue
            .blocking_dispatch(&mut state)
            .map_err(|e| format!("wayland dispatch: {e}"))?;
        if state.exit {
            return Ok(());
        }
    }
}

struct Overlay {
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    shm: Shm,
    compositor: CompositorState,
    pool: SlotPool,
    layer: LayerSurface,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    modifiers: Modifiers,
    width: u32,
    height: u32,
    configured: bool,
    exit: bool,
    mode: Mode,
    engine: Engine,
    ipc: Arc<Mutex<VecDeque<Message>>>,
}

impl Overlay {
    /// Drain pending IPC and apply it. Returns true if a redraw is warranted.
    fn pump_ipc(&mut self) {
        let drained: Vec<Message> = {
            // Recover a poisoned lock: the queue is a plain VecDeque with no
            // broken invariant if a holder panicked, so don't let it kill IPC.
            let mut q = self.ipc.lock().unwrap_or_else(|e| e.into_inner());
            q.drain(..).collect()
        };
        for msg in drained {
            match msg {
                Message::SetScene { scene } => self.engine.set_scene(scene),
                Message::SetEditor { editor } => {
                    let want = if editor { Mode::Editor } else { Mode::View };
                    if want != self.mode {
                        self.mode = want;
                        self.apply_input_region();
                    }
                }
                Message::GetScene => {
                    // Desktop resync: what the overlay is actually showing.
                    if let Ok(s) = serde_json::to_string(self.engine.scene()) {
                        println!("{{\"type\":\"scene\",\"scene\":{s}}}");
                        use std::io::Write as _;
                        let _ = std::io::stdout().flush();
                    }
                }
                Message::Quit => self.exit = true,
            }
        }
    }

    /// Empty input region in View (click-through); full surface in Editor.
    fn apply_input_region(&self) {
        let surface = self.layer.wl_surface();
        match self.mode.routing().interactive {
            false => {
                // An empty region: nothing is interactive, all input falls through.
                if let Ok(region) = Region::new(&self.compositor) {
                    surface.set_input_region(Some(region.wl_region()));
                }
            }
            true => surface.set_input_region(None),
        }
        surface.commit();
    }

    fn draw(&mut self, qh: &QueueHandle<Self>) {
        self.pump_ipc();
        if self.exit || self.width == 0 || self.height == 0 {
            return;
        }
        let editing = self.mode == Mode::Editor;
        self.engine.set_editing(editing);
        self.engine.tick();

        let (w, h) = (self.width, self.height);
        let stride = w as i32 * 4;
        let (buffer, canvas) =
            match self
                .pool
                .create_buffer(w as i32, h as i32, stride, wl_shm::Format::Argb8888)
            {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("hoard-screen: shm buffer: {e}");
                    return;
                }
            };

        // Engine renders RGBA; shm Argb8888 is little-endian => bytes B,G,R,A.
        self.engine.render(canvas, w, h);
        for px in canvas.chunks_exact_mut(4) {
            px.swap(0, 2); // R <-> B, leaving G and A in place
        }

        let surface = self.layer.wl_surface();
        surface.damage_buffer(0, 0, w as i32, h as i32);
        surface.frame(qh, surface.clone());
        if let Err(e) = buffer.attach_to(surface) {
            eprintln!("hoard-screen: attach: {e}");
            return;
        }
        self.layer.commit();
    }
}

impl CompositorHandler for Overlay {
    fn scale_factor_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: i32,
    ) {
    }
    fn transform_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: wl_output::Transform,
    ) {
    }
    fn frame(&mut self, _: &Connection, qh: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: u32) {
        self.draw(qh);
    }
    fn surface_enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
    fn surface_leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
}

impl LayerShellHandler for Overlay {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &LayerSurface) {
        self.exit = true;
    }
    fn configure(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        _: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        self.width = configure.new_size.0;
        self.height = configure.new_size.1;
        if !self.configured {
            self.configured = true;
            self.apply_input_region();
            self.draw(qh);
        }
    }
}

impl SeatHandler for Overlay {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }
    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
    fn new_capability(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            if let Ok(kb) = self.seat_state.get_keyboard(qh, &seat, None) {
                self.keyboard = Some(kb);
            }
        }
    }
    fn remove_capability(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard {
            if let Some(kb) = self.keyboard.take() {
                kb.release();
            }
        }
    }
    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl KeyboardHandler for Overlay {
    fn enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: &wl_surface::WlSurface,
        _: u32,
        _: &[u32],
        _: &[Keysym],
    ) {
    }
    fn leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: &wl_surface::WlSurface,
        _: u32,
    ) {
    }
    fn press_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        // While focused (Editor mode), Ctrl+O or Esc drops back to View.
        let ctrl_o = self.modifiers.ctrl && event.keysym == Keysym::o;
        if ctrl_o || event.keysym == Keysym::Escape {
            if self.mode != Mode::View {
                self.mode = Mode::View;
                self.apply_input_region();
            }
        }
    }
    fn release_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        _: KeyEvent,
    ) {
    }
    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        modifiers: Modifiers,
        _: u32,
    ) {
        self.modifiers = modifiers;
    }
}

impl ShmHandler for Overlay {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl OutputHandler for Overlay {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }
    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
}

impl ProvidesRegistryState for Overlay {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState];
}

delegate_compositor!(Overlay);
delegate_output!(Overlay);
delegate_shm!(Overlay);
delegate_seat!(Overlay);
delegate_keyboard!(Overlay);
delegate_layer!(Overlay);
delegate_registry!(Overlay);

/// Background thread reading newline-JSON IPC from stdin into a shared queue.
fn spawn_stdin_reader() -> Arc<Mutex<VecDeque<Message>>> {
    let queue = Arc::new(Mutex::new(VecDeque::new()));
    let sink = Arc::clone(&queue);
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            let Ok(line) = line else { break };
            match parse_line(&line) {
                Ok(Some(msg)) => sink
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .push_back(msg),
                Ok(None) => {}
                Err(e) => eprintln!("hoard-screen: bad ipc: {e}"),
            }
        }
    });
    queue
}
