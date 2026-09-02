//! X11 on-screen overlay.
//!
//! Two layers, by panel kind, the same model as the Windows runtime:
//!
//! * **App-window panels are the real windows**, placed over the game, not
//!   captured pixels. Each window panel's X window is raised
//!   (`_NET_WM_STATE_ABOVE`) onto its slot (`_NET_MOVERESIZE_WINDOW`) and, in
//!   View, made **click-through** by emptying its SHAPE *input* region so pointer
//!   events fall through to the game. Because the real window stays visible on
//!   top it is never fully obscured, so a browser won't throttle/blank a
//!   backgrounded video tab, the same reason we place the real window instead of
//!   `GetImage`-capturing it (you can't force a foreign app to keep rendering
//!   while it's covered, so we never cover it; it also drops the per-frame
//!   full-window `GetImage` round-trip). Editor mode restores the window's normal
//!   input shape and focuses it so the user can move/resize/interact natively;
//!   leaving Editor reads the new geometry back into the scene.
//!
//! * **Note / image / test panels** are composited the old way into a
//!   screen-sized, ARGB32, override-redirect overlay (needs a running X
//!   compositor for the alpha). To keep that overlay from covering the real
//!   window panels, override-redirect z-order vs. WM `_ABOVE` windows is not
//!   guaranteed, its SHAPE *bounding* region is punched with a hole for each
//!   window panel, so the real windows always show through regardless of stack
//!   order. With no note/image panels the overlay is simply unmapped.
//!
//! Ctrl+O (always) and Escape (while editing) are grabbed on the root so they
//! toggle the mode even with the overlay unmapped. Scene updates arrive on stdin
//! (newline-JSON); mode + edited-scene changes are emitted on stdout so the
//! desktop app stays in sync.
//!
//! Caveats: cropping a real window is a clip (BOUNDING shape), not a zoom;
//! emptying/clipping a foreign window's shape is intrusive but reversible (reset
//! to no shape on restore); without a compositor the notes overlay won't be
//! translucent, though the real-window panels still work.

use std::collections::{HashMap, HashSet};
use std::io::{BufRead, Write};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use x11rb::connection::Connection;
use x11rb::protocol::shape::{self, ConnectionExt as _, SK, SO};
use x11rb::protocol::xproto::*;
use x11rb::protocol::Event;
use x11rb::wrapper::ConnectionExt as _;

use crate::engine::Engine;
use crate::ipc::{parse_line, Message};
use crate::mode::Mode;
use crate::scene::{Crop, Panel, Scene, SourceRef};

/// Re-assert app-window z-order every N frames in View, so a game that pops
/// itself above can't bury the panels. ~0.5s at 30fps.
const REASSERT_EVERY: u64 = 15;

fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

/// Interned EWMH atoms used to drive the window manager.
struct Ewmh {
    moveresize: Atom,
    wm_state: Atom,
    state_above: Atom,
    active: Atom,
}

/// Absolute root-space geometry: top-left of the content + size.
type Saved = (i32, i32, u32, u32);

/// Book-keeping for a real app window we've taken over: where to put it back,
/// and which of its shapes we changed (so we only reset what we touched).
struct Managed {
    saved: Saved,
    cropped: bool,      // we set a BOUNDING shape on it
    shaped_input: bool, // we emptied its INPUT shape (click-through)
}

pub fn run(mut engine: Engine) -> Result<(), String> {
    let (conn, screen_num) = x11rb::connect(None).map_err(err)?;
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;
    let (sw, sh) = (screen.width_in_pixels, screen.height_in_pixels);

    let visual = find_argb_visual(screen)
        .ok_or("no 32-bit TrueColor visual (need a compositing X server)")?;
    let colormap = conn.generate_id().map_err(err)?;
    conn.create_colormap(ColormapAlloc::NONE, colormap, root, visual)
        .map_err(err)?;

    let window = conn.generate_id().map_err(err)?;
    let aux = CreateWindowAux::new()
        .background_pixel(0)
        .border_pixel(0)
        .colormap(colormap)
        .override_redirect(1)
        .event_mask(EventMask::EXPOSURE | EventMask::KEY_PRESS | EventMask::STRUCTURE_NOTIFY);
    conn.create_window(
        32,
        window,
        root,
        0,
        0,
        sw,
        sh,
        0,
        WindowClass::INPUT_OUTPUT,
        visual,
        &aux,
    )
    .map_err(err)?;

    set_window_type_overlay(&conn, window)?;
    let ewmh = intern_ewmh(&conn)?;

    let gc = conn.generate_id().map_err(err)?;
    conn.create_gc(gc, window, &CreateGCAux::new())
        .map_err(err)?;
    let _ = shape::ConnectionExt::shape_query_version(&conn);

    let o_keycode = keysym_to_keycode(&conn, 0x6f); // XK_o
    if let Some(kc) = o_keycode {
        conn.grab_key(
            true,
            root,
            ModMask::CONTROL,
            kc,
            GrabMode::ASYNC,
            GrabMode::ASYNC,
        )
        .map_err(err)?;
    }
    let esc_keycode = keysym_to_keycode(&conn, 0xff1b); // XK_Escape

    // The overlay is only mapped when there are note/image panels to show; it
    // starts unmapped (the global Ctrl+O grab is on the root, so it toggles
    // regardless). Window panels are real windows handled separately.
    conn.flush().map_err(err)?;

    let mut mode = Mode::View;
    // The authoritative full scene (window + non-window panels). The engine only
    // ever gets the non-window subset; window panels live as real windows.
    let mut scene = Scene::default();
    let mut managed: HashMap<u32, Managed> = HashMap::new();
    let mut overlay_mapped = false;
    let mut shape_dirty = true;

    let rx = spawn_stdin_reader();
    let frame_dt = Duration::from_millis(33); // ~30 fps
    let mut buf = vec![0u8; (sw as usize) * (sh as usize) * 4];
    let mut bgrx = vec![0u8; buf.len()];
    let mut frame_no: u64 = 0;

    loop {
        let frame_start = Instant::now();
        frame_no = frame_no.wrapping_add(1);

        while let Some(event) = conn.poll_for_event().map_err(err)? {
            match event {
                Event::KeyPress(ev) => {
                    let toggle =
                        Some(ev.detail) == o_keycode && ev.state.contains(KeyButMask::CONTROL);
                    let escape = Some(ev.detail) == esc_keycode && mode == Mode::Editor;
                    if toggle || escape {
                        let next = if escape { Mode::View } else { mode.toggled() };
                        set_mode(
                            &conn,
                            root,
                            &ewmh,
                            sw,
                            sh,
                            esc_keycode,
                            &mut scene,
                            &mut managed,
                            &mut mode,
                            next,
                        )?;
                        shape_dirty = true;
                        emit_mode(mode);
                        if mode == Mode::View {
                            emit_scene(&scene);
                        }
                    }
                }
                Event::Error(e) => eprintln!("hoard-screen: x11 error: {e:?}"),
                _ => {}
            }
        }

        loop {
            let msg = match rx.try_recv() {
                Ok(m) => m,
                Err(mpsc::TryRecvError::Empty) => break,
                // Desktop app died (stdin EOF): don't outlive our controller.
                Err(mpsc::TryRecvError::Disconnected) => Message::Quit,
            };
            match msg {
                Message::GetScene => {
                    emit_mode(mode);
                    emit_scene(&scene);
                }
                Message::SetScene { scene: next } => {
                    scene = next;
                    // The engine never sees window panels (it would GetImage them);
                    // they become real windows over the game.
                    engine.set_scene(engine_subset(&scene));
                    apply_windows(
                        &conn,
                        root,
                        &ewmh,
                        &scene,
                        &mut managed,
                        mode == Mode::Editor,
                    )?;
                    shape_dirty = true;
                }
                Message::SetEditor { editor } => {
                    let want = if editor { Mode::Editor } else { Mode::View };
                    if want != mode {
                        set_mode(
                            &conn,
                            root,
                            &ewmh,
                            sw,
                            sh,
                            esc_keycode,
                            &mut scene,
                            &mut managed,
                            &mut mode,
                            want,
                        )?;
                        shape_dirty = true;
                        if mode == Mode::View {
                            emit_scene(&scene);
                        }
                    }
                }
                Message::Quit => {
                    restore_all(&conn, root, &ewmh, &mut managed);
                    conn.flush().map_err(err)?;
                    return Ok(());
                }
            }
        }

        // The overlay carries only note/image panels; map it only when there are
        // some (and in View). Window panels are real windows, always on top.
        let want_overlay = mode == Mode::View && has_overlay_panels(&scene);
        if want_overlay && !overlay_mapped {
            conn.map_window(window).map_err(err)?;
            restack(&conn, window)?;
            overlay_mapped = true;
            shape_dirty = true;
        } else if !want_overlay && overlay_mapped {
            conn.unmap_window(window).map_err(err)?;
            overlay_mapped = false;
        }

        if want_overlay {
            if shape_dirty {
                overlay_shape(&conn, window, sw, sh, &scene)?;
                shape_dirty = false;
            }
            // The scope can be bound to a button and hide itself; in the editor it
            // has to be visible anyway so it can be placed. It is told before every
            // tick: setting a bool on a handful of sources is not worth tracking
            // whether it changed.
            engine.set_editing(mode == Mode::Editor);
            engine.tick();
            engine.render(&mut buf, sw as u32, sh as u32);
            rgba_to_bgrx(&buf, &mut bgrx);
            present(&conn, window, gc, &bgrx, sw, sh)?;
            if frame_no.is_multiple_of(REASSERT_EVERY) {
                restack(&conn, window)?;
            }
        }

        if mode == Mode::View && frame_no.is_multiple_of(REASSERT_EVERY) {
            reassert_above(&conn, root, &ewmh, &scene, &managed);
        }

        conn.flush().map_err(err)?;
        if let Some(rem) = frame_dt.checked_sub(frame_start.elapsed()) {
            std::thread::sleep(rem);
        }
    }
}

/// The non-window panels, for the compositing engine. Window panels are real OS
/// windows and must not reach the engine (it would open a capture for them).
fn engine_subset(scene: &Scene) -> Scene {
    Scene {
        panels: scene
            .panels
            .iter()
            .filter(|p| !matches!(p.source, SourceRef::Window { .. }))
            .cloned()
            .collect(),
    }
}

/// True when the scene has at least one note/image/test panel (the overlay's job).
fn has_overlay_panels(scene: &Scene) -> bool {
    scene
        .panels
        .iter()
        .any(|p| !matches!(p.source, SourceRef::Window { .. }))
}

// --- real-window placement --------------------------------------------------

/// Place every window panel's real window over the game and restore the ones
/// that left the scene. `interactive` is true in Editor (window takes input,
/// gets focus) and false in View (click-through).
fn apply_windows(
    conn: &impl Connection,
    root: Window,
    ewmh: &Ewmh,
    scene: &Scene,
    managed: &mut HashMap<u32, Managed>,
    interactive: bool,
) -> Result<(), String> {
    let mut desired: HashSet<u32> = HashSet::new();
    for p in scene.draw_order() {
        let Some(win) = panel_win(p) else { continue };
        desired.insert(win);
        // The geometry is read once, the first time we take a window over, so
        // that what we restore later is where the window actually was.
        let m = managed.entry(win).or_insert_with(|| Managed {
            saved: abs_geom(conn, root, win).unwrap_or((0, 0, 1, 1)),
            cropped: false,
            shaped_input: false,
        });
        place_panel(conn, root, ewmh, win, p, interactive, m);
    }
    // Restore windows whose panel is gone.
    let gone: Vec<u32> = managed
        .keys()
        .copied()
        .filter(|w| !desired.contains(w))
        .collect();
    for win in gone {
        if let Some(m) = managed.remove(&win) {
            restore_window(conn, root, ewmh, win, &m);
        }
    }
    conn.flush().map_err(err)?;
    Ok(())
}

/// Raise a real window onto its panel slot, set click-through per mode, and clip
/// it to its crop. Panel rects are already in root/screen coords on X11.
fn place_panel(
    conn: &impl Connection,
    root: Window,
    ewmh: &Ewmh,
    win: u32,
    p: &Panel,
    interactive: bool,
    m: &mut Managed,
) {
    let r = p.rect.normalized();
    let _ = client_msg(
        conn,
        root,
        win,
        ewmh.wm_state,
        [1, ewmh.state_above, 0, 1, 0],
    );
    if interactive {
        let _ = client_msg(conn, root, win, ewmh.active, [2, 0, 0, 0, 0]);
    }
    move_resize(
        conn,
        root,
        ewmh,
        win,
        (r.x as i32, r.y as i32, r.w as i32, r.h as i32),
    );
    set_clickthrough(conn, win, !interactive, m);
    apply_crop(conn, win, r.w as i32, r.h as i32, p.crop, m);
}

/// Empty a foreign window's INPUT shape (events fall through) in View; reset it
/// to the default (whole window) in Editor. Only resets what we set.
fn set_clickthrough(conn: &impl Connection, win: u32, on: bool, m: &mut Managed) {
    if on {
        let _ = conn.shape_rectangles(SO::SET, SK::INPUT, ClipOrdering::UNSORTED, win, 0, 0, &[]);
        m.shaped_input = true;
    } else if m.shaped_input {
        let _ = conn.shape_mask(SO::SET, SK::INPUT, win, 0, 0, 0u32);
        m.shaped_input = false;
    }
}

/// Clip a real window to its crop with a BOUNDING shape (a clip, not a zoom). A
/// NONE crop clears any shape we previously set.
fn apply_crop(conn: &impl Connection, win: u32, w: i32, h: i32, crop: Crop, m: &mut Managed) {
    let c = crop.sanitized();
    if c.left == 0.0 && c.right == 0.0 && c.top == 0.0 && c.bottom == 0.0 {
        if m.cropped {
            let _ = conn.shape_mask(SO::SET, SK::BOUNDING, win, 0, 0, 0u32);
            m.cropped = false;
        }
        return;
    }
    let l = (c.left * w as f64).round() as i32;
    let t = (c.top * h as f64).round() as i32;
    let r = ((1.0 - c.right) * w as f64).round() as i32;
    let b = ((1.0 - c.bottom) * h as f64).round() as i32;
    let (x0, y0) = (l.min(r), t.min(b));
    let rect = Rectangle {
        x: x0 as i16,
        y: y0 as i16,
        width: (l.max(r) - x0).max(1) as u16,
        height: (b.max(t) - y0).max(1) as u16,
    };
    let _ = conn.shape_rectangles(
        SO::SET,
        SK::BOUNDING,
        ClipOrdering::UNSORTED,
        win,
        0,
        0,
        &[rect],
    );
    m.cropped = true;
}

/// Restore a window to how we found it: reset any shapes we set, drop ABOVE, and
/// move it back to its saved geometry.
fn restore_window(conn: &impl Connection, root: Window, ewmh: &Ewmh, win: u32, m: &Managed) {
    if m.shaped_input {
        let _ = conn.shape_mask(SO::SET, SK::INPUT, win, 0, 0, 0u32);
    }
    if m.cropped {
        let _ = conn.shape_mask(SO::SET, SK::BOUNDING, win, 0, 0, 0u32);
    }
    let _ = client_msg(
        conn,
        root,
        win,
        ewmh.wm_state,
        [0, ewmh.state_above, 0, 1, 0],
    );
    let (x, y, w, h) = m.saved;
    move_resize(conn, root, ewmh, win, (x, y, w as i32, h as i32));
}

/// Restore every relocated window to its saved state (used on quit).
fn restore_all(
    conn: &impl Connection,
    root: Window,
    ewmh: &Ewmh,
    managed: &mut HashMap<u32, Managed>,
) {
    for (&win, m) in managed.iter() {
        restore_window(conn, root, ewmh, win, m);
    }
    managed.clear();
}

/// Re-assert each managed window's ABOVE state, so a game that raises itself
/// can't bury the panels. Does not steal focus (no `_NET_ACTIVE_WINDOW`).
fn reassert_above(
    conn: &impl Connection,
    root: Window,
    ewmh: &Ewmh,
    scene: &Scene,
    managed: &HashMap<u32, Managed>,
) {
    for p in scene.draw_order() {
        let Some(win) = panel_win(p) else { continue };
        if managed.contains_key(&win) {
            let _ = client_msg(
                conn,
                root,
                win,
                ewmh.wm_state,
                [1, ewmh.state_above, 0, 1, 0],
            );
        }
    }
}

// --- mode transition --------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn set_mode(
    conn: &impl Connection,
    root: Window,
    ewmh: &Ewmh,
    sw: u16,
    sh: u16,
    esc_kc: Option<u8>,
    scene: &mut Scene,
    managed: &mut HashMap<u32, Managed>,
    mode: &mut Mode,
    next: Mode,
) -> Result<(), String> {
    if *mode == next {
        return Ok(());
    }
    *mode = next;
    set_escape_grab(conn, root, esc_kc, next == Mode::Editor);

    if next == Mode::Editor {
        // Hand the floor to the real windows: interactive (input restored, raised
        // and focused). The overlay is unmapped by the main loop (not View).
        apply_windows(conn, root, ewmh, scene, managed, true)?;
    } else {
        // Read back where the user left each window, then re-apply click-through.
        readback_geometry(conn, root, sw, sh, scene, managed);
        apply_windows(conn, root, ewmh, scene, managed, false)?;
    }
    conn.flush().map_err(err)?;
    Ok(())
}

/// After an edit, fold each managed window's on-screen geometry back into its
/// panel rect so the layout persists.
fn readback_geometry(
    conn: &impl Connection,
    root: Window,
    sw: u16,
    sh: u16,
    scene: &mut Scene,
    managed: &HashMap<u32, Managed>,
) {
    for p in scene.panels.iter_mut() {
        let Some(win) = panel_win(p) else { continue };
        if !managed.contains_key(&win) {
            continue;
        }
        if let Some((x, y, w, h)) = abs_geom(conn, root, win) {
            let nx = x.clamp(0, sw as i32) as f64;
            let ny = y.clamp(0, sh as i32) as f64;
            p.rect.x = nx;
            p.rect.y = ny;
            p.rect.w = (w as f64).min(sw as f64 - nx).max(1.0);
            p.rect.h = (h as f64).min(sh as f64 - ny).max(1.0);
        }
    }
}

// --- window ids / geometry --------------------------------------------------

fn panel_win(p: &Panel) -> Option<u32> {
    match &p.source {
        SourceRef::Window { id } => parse_win(id),
        _ => None,
    }
}

fn parse_win(id: &str) -> Option<u32> {
    let s = id.trim().trim_start_matches("0x");
    u32::from_str_radix(s, 16).ok()
}

/// Absolute root-space geometry of a window (top-left of its content + size).
fn abs_geom(conn: &impl Connection, root: Window, win: u32) -> Option<Saved> {
    let g = conn.get_geometry(win).ok()?.reply().ok()?;
    let t = conn
        .translate_coordinates(win, root, 0, 0)
        .ok()?
        .reply()
        .ok()?;
    Some((
        t.dst_x as i32,
        t.dst_y as i32,
        g.width as u32,
        g.height as u32,
    ))
}

// --- EWMH / WM cooperation --------------------------------------------------

fn intern_ewmh(conn: &impl Connection) -> Result<Ewmh, String> {
    let intern = |name: &str| -> Result<Atom, String> {
        Ok(conn
            .intern_atom(false, name.as_bytes())
            .map_err(err)?
            .reply()
            .map_err(err)?
            .atom)
    };
    Ok(Ewmh {
        moveresize: intern("_NET_MOVERESIZE_WINDOW")?,
        wm_state: intern("_NET_WM_STATE")?,
        state_above: intern("_NET_WM_STATE_ABOVE")?,
        active: intern("_NET_ACTIVE_WINDOW")?,
    })
}

fn client_msg(
    conn: &impl Connection,
    root: Window,
    window: u32,
    type_: Atom,
    data: [u32; 5],
) -> Result<(), String> {
    let ev = ClientMessageEvent::new(32, window, type_, data);
    conn.send_event(
        false,
        root,
        EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
        ev,
    )
    .map_err(err)?;
    Ok(())
}

/// Ask the WM to move+resize a window via `_NET_MOVERESIZE_WINDOW`.
fn move_resize(
    conn: &impl Connection,
    root: Window,
    ewmh: &Ewmh,
    win: u32,
    rect: (i32, i32, i32, i32),
) {
    // flags: gravity 0 | x,y,w,h present (bits 8..11) | source = application (bit 12).
    let flags: u32 = (1 << 8) | (1 << 9) | (1 << 10) | (1 << 11) | (1 << 12);
    let (x, y, w, h) = rect;
    let _ = client_msg(
        conn,
        root,
        win,
        ewmh.moveresize,
        [flags, x as u32, y as u32, w.max(1) as u32, h.max(1) as u32],
    );
}

// --- overlay layer ----------------------------------------------------------

fn restack(conn: &impl Connection, window: Window) -> Result<(), String> {
    conn.configure_window(
        window,
        &ConfigureWindowAux::new().stack_mode(StackMode::ABOVE),
    )
    .map_err(err)?;
    Ok(())
}

fn set_escape_grab(conn: &impl Connection, root: Window, esc_kc: Option<u8>, on: bool) {
    if let Some(kc) = esc_kc {
        let none = ModMask::from(0u16);
        if on {
            let _ = conn.grab_key(true, root, none, kc, GrabMode::ASYNC, GrabMode::ASYNC);
        } else {
            let _ = conn.ungrab_key(kc, root, none);
        }
    }
}

/// Overlay BOUNDING = full screen minus a hole per window panel (so real windows
/// always show through, whatever the stack order), INPUT empty (click-through).
fn overlay_shape(
    conn: &impl Connection,
    window: Window,
    sw: u16,
    sh: u16,
    scene: &Scene,
) -> Result<(), String> {
    let full = Rectangle {
        x: 0,
        y: 0,
        width: sw,
        height: sh,
    };
    conn.shape_rectangles(
        SO::SET,
        SK::BOUNDING,
        ClipOrdering::UNSORTED,
        window,
        0,
        0,
        &[full],
    )
    .map_err(err)?;
    let holes: Vec<Rectangle> = scene
        .panels
        .iter()
        .filter(|p| matches!(p.source, SourceRef::Window { .. }))
        .map(|p| {
            let r = p.rect.normalized();
            Rectangle {
                x: r.x as i16,
                y: r.y as i16,
                width: r.w.max(1.0) as u16,
                height: r.h.max(1.0) as u16,
            }
        })
        .collect();
    if !holes.is_empty() {
        conn.shape_rectangles(
            SO::SUBTRACT,
            SK::BOUNDING,
            ClipOrdering::UNSORTED,
            window,
            0,
            0,
            &holes,
        )
        .map_err(err)?;
    }
    conn.shape_rectangles(
        SO::SET,
        SK::INPUT,
        ClipOrdering::UNSORTED,
        window,
        0,
        0,
        &[],
    )
    .map_err(err)?;
    Ok(())
}

// --- the rest (unchanged plumbing) ------------------------------------------

fn find_argb_visual(screen: &Screen) -> Option<Visualid> {
    for depth in &screen.allowed_depths {
        if depth.depth == 32 {
            for v in &depth.visuals {
                if v.class == VisualClass::TRUE_COLOR {
                    return Some(v.visual_id);
                }
            }
        }
    }
    None
}

fn set_window_type_overlay(conn: &impl Connection, window: Window) -> Result<(), String> {
    let intern = |name: &str| -> Result<Atom, String> {
        Ok(conn
            .intern_atom(false, name.as_bytes())
            .map_err(err)?
            .reply()
            .map_err(err)?
            .atom)
    };
    if let (Ok(wt), Ok(notif)) = (
        intern("_NET_WM_WINDOW_TYPE"),
        intern("_NET_WM_WINDOW_TYPE_NOTIFICATION"),
    ) {
        let _ = conn.change_property32(PropMode::REPLACE, window, wt, AtomEnum::ATOM, &[notif]);
    }
    Ok(())
}

fn keysym_to_keycode(conn: &impl Connection, keysym: u32) -> Option<u8> {
    let setup = conn.setup();
    let min = setup.min_keycode;
    let max = setup.max_keycode;
    let count = max - min + 1;
    let reply = conn.get_keyboard_mapping(min, count).ok()?.reply().ok()?;
    let per = reply.keysyms_per_keycode as usize;
    if per == 0 {
        return None;
    }
    for (i, chunk) in reply.keysyms.chunks(per).enumerate() {
        if chunk.contains(&keysym) {
            return Some(min + i as u8);
        }
    }
    None
}

fn emit_mode(mode: Mode) {
    println!("{{\"type\":\"mode\",\"editor\":{}}}", mode == Mode::Editor);
    let _ = std::io::stdout().flush();
}

fn emit_scene(scene: &Scene) {
    if let Ok(s) = serde_json::to_string(scene) {
        println!("{{\"type\":\"scene\",\"scene\":{s}}}");
        let _ = std::io::stdout().flush();
    }
}

fn present(
    conn: &impl Connection,
    window: Window,
    gc: Gcontext,
    bgrx: &[u8],
    sw: u16,
    sh: u16,
) -> Result<(), String> {
    let row_bytes = sw as usize * 4;
    let max_bytes = (conn.maximum_request_bytes()).saturating_sub(256);
    let rows_per = (max_bytes / row_bytes).clamp(1, sh as usize);
    let mut y = 0u16;
    while y < sh {
        let rows = rows_per.min((sh - y) as usize) as u16;
        let start = y as usize * row_bytes;
        let end = start + rows as usize * row_bytes;
        conn.put_image(
            ImageFormat::Z_PIXMAP,
            window,
            gc,
            sw,
            rows,
            0,
            y as i16,
            0,
            32,
            &bgrx[start..end],
        )
        .map_err(err)?;
        y += rows;
    }
    Ok(())
}

fn rgba_to_bgrx(rgba: &[u8], out: &mut [u8]) {
    for (s, d) in rgba.chunks_exact(4).zip(out.chunks_exact_mut(4)) {
        d[0] = s[2];
        d[1] = s[1];
        d[2] = s[0];
        d[3] = s[3];
    }
}

fn spawn_stdin_reader() -> Receiver<Message> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            let Ok(line) = line else { break };
            match parse_line(&line) {
                Ok(Some(msg)) => {
                    if tx.send(msg).is_err() {
                        break;
                    }
                }
                Ok(None) => {}
                Err(e) => eprintln!("hoard-screen: bad ipc: {e}"),
            }
        }
    });
    rx
}
