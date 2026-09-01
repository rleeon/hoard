//! Windows on-screen overlay.
//!
//! Presentation is a GPU pipeline ([`super::win_gpu`]): one
//! `WS_EX_NOREDIRECTIONBITMAP` window per monitor, fixed at the monitor size,
//! fronted by a DirectComposition swapchain. Panels split by kind:
//!
//! * **Non-compat app-window panels are the real windows**, placed over the game
//!  , not captured pixels. The HWND is moved onto its slot, raised
//!   `HWND_TOPMOST`, region-clipped to its crop, and (in View) given
//!   `WS_EX_TRANSPARENT | WS_EX_NOACTIVATE` so it's click-through and never steals
//!   focus. Because the window stays genuinely visible on top, it is never
//!   *occluded*: browsers (Chrome/Edge/Firefox) won't trip their native
//!   occlusion detection (`CalculateNativeWinOcclusion`) and pause video render,
//!   so a backgrounded YouTube tab keeps playing instead of going black. Editor
//!   mode drops `WS_EX_TRANSPARENT` so the user can grab, move, resize and
//!   *interact* natively; leaving Editor reads the new geometry back.
//!
//! * **Compat window panels (View)** are parked off-screen (kept rendering, so
//!   Chromium/Electron won't background them) and WGC-captured straight to the
//!   GPU: the capture texture is sampled directly into the swapchain (no CPU
//!   readback, no swizzle), crop→UV, scale→quad. Presented with `Present(0)`: the
//!   run loop paces itself on the message queue (`MsgWaitForMultipleObjectsEx`)
//!   instead of blocking on vblank, which would freeze hit-testing for a frame.
//!   Interactive compat (passthrough off) forwards mouse to the parked window and
//!   gives it the foreground on click so keystrokes flow to it natively.
//!
//! * **Note / image / test panels** are composited on the CPU by the engine,
//!   then uploaded to a per-monitor texture and drawn as one quad on the same
//!   swapchain. The engine only ever sees these non-window panels.
//!
//! Caveats: needs the game in **borderless** fullscreen (exclusive fullscreen
//! lets nothing on top, and WGC capture couldn't either); cropping a real window
//! is a clip (`SetWindowRgn`), not a zoom, in outer-window coords. In View we
//! strip the window's chrome (title bar + frame) so a panel shows only content
//! and the crop maps to it; Editor restores the chrome so the user can drag and
//! resize the real window natively.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, Write};
use std::sync::mpsc::{self, Receiver};
use std::time::Instant;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, TRUE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CombineRgn, CreateEllipticRgn, CreateRectRgn, DeleteObject, RedrawWindow, ScreenToClient,
    SetWindowRgn, HGDIOBJ, HRGN, RDW_ALLCHILDREN, RDW_FRAME, RDW_INVALIDATE, RDW_UPDATENOW,
    RGN_DIFF, RGN_OR,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, RegisterHotKey, SetFocus, UnregisterHotKey, MOD_CONTROL, MOD_NOREPEAT,
    VK_LBUTTON, VK_RBUTTON,
};
use windows::Win32::UI::WindowsAndMessaging::{
    ChildWindowFromPointEx, CreateWindowExW, DefWindowProcW, DispatchMessageW, GetCursorPos,
    GetForegroundWindow, GetSystemMetrics, GetTopWindow, GetWindow, GetWindowLongW,
    GetWindowPlacement, GetWindowRect, GetWindowThreadProcessId, IsIconic, IsWindowVisible,
    IsZoomed, MsgWaitForMultipleObjectsEx, PeekMessageW, PostMessageW, RegisterClassW,
    SetForegroundWindow, SetLayeredWindowAttributes, SetWindowDisplayAffinity, SetWindowLongW,
    SetWindowPlacement, SetWindowPos, ShowWindow, TranslateMessage, CWP_SKIPINVISIBLE,
    CWP_SKIPTRANSPARENT, GWL_EXSTYLE, GWL_STYLE, GW_HWNDNEXT, HTCLIENT, HTTRANSPARENT,
    HWND_NOTOPMOST, HWND_TOPMOST, LWA_ALPHA, MSG, MWMO_INPUTAVAILABLE, PM_REMOVE, QS_ALLINPUT,
    SM_CXSCREEN, SM_CYSCREEN, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_HIDE,
    SW_RESTORE, SW_SHOWNOACTIVATE, SW_SHOWNORMAL, WDA_EXCLUDEFROMCAPTURE, WDA_NONE,
    WINDOWPLACEMENT, WM_HOTKEY, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL,
    WM_NCHITTEST, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_XBUTTONDOWN, WM_XBUTTONUP, WNDCLASSW,
    WS_CAPTION, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_NOREDIRECTIONBITMAP, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_POPUP, WS_SYSMENU,
    WS_THICKFRAME,
};

use super::win_gpu::{GpuOverlay, Quad};

use crate::capture::WindowId;
use crate::engine::Engine;
use crate::ipc::{parse_line, Message};
use crate::mode::Mode;
use crate::monitors::MonitorInfo;
use crate::scene::{Crop, Panel, PanelTarget, ScaleMode, Scene, SourceRef};

const HOTKEY_TOGGLE: i32 = 1; // Ctrl+O
const HOTKEY_ESCAPE: i32 = 2; // Esc (only registered while editing)
const VK_O: u32 = 0x4F;
const VK_ESCAPE: u32 = 0x1B;
/// Re-assert app-window z-order every N frames in View, so a game that pops
/// itself to the foreground can't bury the panels. ~0.5s at 30fps.
const REASSERT_EVERY: u64 = 15;

type SavedRect = (i32, i32, i32, i32); // left, top, width, height

fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

/// A compat panel that should receive (forwarded) mouse input: its slot in
/// virtual-desktop coords, the parked HWND we forward to, and the crop/scale so
/// we can map a cursor point back to a point in the parked window. Rebuilt every
/// View frame; read by [`wndproc`]'s `WM_NCHITTEST` (so the overlay catches the
/// click over the slot) and by [`forward_pointer`] (to translate + post it).
#[derive(Clone)]
struct CompatSlot {
    rect: SavedRect, // x, y, w, h (virtual desktop)
    hwnd: isize,
    crop: Crop,
    scale: ScaleMode,
    /// "Clics pasan al juego" ON: instead of forwarding, a circular cut-out of the
    /// overlay follows the cursor over this panel so the OS delivers clicks NATIVELY
    /// to whatever is physically behind it.
    passthrough: bool,
    /// Radius (px) of that click-through hole. Only meaningful when `passthrough`.
    hole_radius: i32,
}

thread_local! {
    static SLOTS: RefCell<Vec<CompatSlot>> = const { RefCell::new(Vec::new()) };
    /// The child HWND latched at left-button-down, so a drag keeps posting to it
    /// (capture emulation). `None` when no left-drag is in progress.
    static DRAG_CHILD: RefCell<Option<isize>> = const { RefCell::new(None) };
    /// A parked window that should be brought to the foreground. Latched in
    /// `forward_pointer` on left-button-down and consumed by the run loop, so the
    /// blocking `AttachThreadInput`/`SetForegroundWindow` never runs on the click
    /// hot path (doing it inline stalled the click → 200-800ms delay and dropped
    /// clicks while Chromium was busy loading).
    static PENDING_FG: RefCell<Option<isize>> = const { RefCell::new(None) };
    /// After a right-click is forwarded to a parked compat window, its context menu
    /// pops up at the window's REAL (off-screen, bottom-right) position. Latch the
    /// app PID + the cursor point so the run loop can find that fresh popup and move
    /// it under the cursor. `(pid, cursor, deadline)`.
    static PENDING_POPUP: RefCell<Option<(u32, isize, POINT, std::time::Instant)>> = const { RefCell::new(None) };
}

fn slot_contains(s: &CompatSlot, x: i32, y: i32) -> bool {
    let (sx, sy, sw, sh) = s.rect;
    x >= sx && x < sx + sw && y >= sy && y < sy + sh
}

const MK_LBUTTON: usize = 0x0001;
const MK_RBUTTON: usize = 0x0002;

fn make_lparam(x: i32, y: i32) -> LPARAM {
    let lo = (x as i16) as u16 as u32;
    let hi = (y as i16) as u16 as u32;
    LPARAM(((hi << 16) | lo) as i32 as isize)
}

/// Rebuild the compat slot list from the current scene: every compat panel whose
/// real window is parked (we're showing the WGC capture). The `passthrough` flag
/// decides where its clicks go, OFF forwards to the parked capture (interactive),
/// ON forwards to the window physically behind the panel ("clics pasan al juego").
unsafe fn rebuild_slots(scene: &Scene, mons: &[MonitorInfo], managed: &HashMap<isize, Managed>) {
    let mut slots = Vec::new();
    for p in scene.draw_order() {
        let Some(win) = panel_win(p) else { continue };
        if !p.compat {
            continue;
        }
        if managed.get(&win).map_or(true, |m| !m.parked) {
            continue;
        }
        let mon = target_monitor(mons, p);
        let r = p.rect.normalized();
        let rect = (
            mon.x + r.x as i32,
            mon.y + r.y as i32,
            r.w.max(1.0) as i32,
            r.h.max(1.0) as i32,
        );
        // Passthrough panels forward to the window behind; interactive ones to the
        // parked capture. Both need a slot so the overlay's click reaches a real
        // window (the composition overlay isn't click-through even with
        // WS_EX_TRANSPARENT, so we always route the click ourselves).
        slots.push(CompatSlot {
            rect,
            hwnd: win,
            crop: p.crop,
            scale: p.scale,
            passthrough: p.passthrough,
            hole_radius: (p.passthrough_radius.max(8.0)) as i32,
        });
    }
    SLOTS.with(|s| *s.borrow_mut() = slots);
}

fn clear_slots() {
    SLOTS.with(|s| s.borrow_mut().clear());
}

// DEAD CODE, old toggle-transparency architecture, replaced by per-frame region
// clipping (`apply_overlay_region` / `SetWindowRgn`) + the passthrough lens.
// Commented out (not deleted) in case the region approach needs to fall back to it.
// /// Add or drop `WS_EX_TRANSPARENT` on an overlay surface. With it (the default)
// /// the layered window is fully click-through and `WM_NCHITTEST` is never asked;
// /// without it the window hit-tests, so its opaque pixels can catch the mouse and
// /// `wndproc` decides per-point (slot -> `HTCLIENT`, else `HTTRANSPARENT`).
// unsafe fn set_surface_transparent(hwnd: HWND, transparent: bool) {
//     let mut ex = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
//     if transparent {
//         ex |= WS_EX_TRANSPARENT.0;
//     } else {
//         ex &= !WS_EX_TRANSPARENT.0;
//     }
//     let _ = SetWindowLongW(hwnd, GWL_EXSTYLE, ex as i32);
// }

/// Give the parked (off-screen) compat window real keyboard focus so the user's
/// keystrokes flow to it NATIVELY (Chromium routes typing/IME to its focused
/// control, synthetic key posting is unreliable, native focus is not).
///
/// A plain `SetForegroundWindow` from our overlay is silently refused by Windows:
/// the overlay is `WS_EX_NOACTIVATE` and never the foreground process, and only
/// the foreground process may hand the foreground away. The documented escape is
/// to attach our input queue to the current foreground thread (and the target's)
/// for the duration of the call, which lets the focus change go through.
unsafe fn force_foreground(target: HWND) {
    let fg = GetForegroundWindow();
    if fg == target {
        return;
    }
    let cur = GetCurrentThreadId();
    let fg_tid = if fg.0.is_null() {
        0
    } else {
        GetWindowThreadProcessId(fg, None)
    };
    let tgt_tid = GetWindowThreadProcessId(target, None);
    let a1 = fg_tid != 0 && fg_tid != cur && AttachThreadInput(cur, fg_tid, true).as_bool();
    let a2 = tgt_tid != 0
        && tgt_tid != cur
        && tgt_tid != fg_tid
        && AttachThreadInput(cur, tgt_tid, true).as_bool();
    let _ = SetForegroundWindow(target);
    let _ = SetFocus(target);
    if a2 {
        let _ = AttachThreadInput(cur, tgt_tid, false);
    }
    if a1 {
        let _ = AttachThreadInput(cur, fg_tid, false);
    }
}

/// Translate a mouse message that landed on the overlay (over an interactive
/// compat slot) into the equivalent point in the parked window and post it
/// there. The parked window is off-screen but still a live, hit-testable window,
/// so `PostMessageW` to the child under the mapped point drives it (Chromium's
/// render widget processes posted mouse messages without needing focus).
unsafe fn forward_pointer(message: u32, wparam: WPARAM) {
    let mut pt = POINT::default();
    if GetCursorPos(&mut pt).is_err() {
        return;
    }
    let slot = SLOTS.with(|s| {
        s.borrow()
            .iter()
            .find(|sl| slot_contains(sl, pt.x, pt.y))
            .cloned()
    });
    let Some(slot) = slot else { return };
    // Passthrough ON ("clics pasan al juego"): handled by the click-through hole,
    // a circular cut-out of the overlay follows the cursor over the panel, so the
    // OS delivers clicks NATIVELY to whatever is physically behind (real
    // double-click / drag, correct multi-monitor coords). No synthetic forwarding.
    // A click only reaches here if the hole hasn't caught up for a frame; ignore it.
    if slot.passthrough {
        return;
    }
    let target = HWND(slot.hwnd as *mut core::ffi::c_void);
    let mut wr = RECT::default();
    if GetWindowRect(target, &mut wr).is_err() {
        return;
    }
    let (vx, vy, vw, vh) = slot.rect;
    let (vw, vh) = (vw.max(1) as f64, vh.max(1) as f64);
    let win_w = (wr.right - wr.left).max(1) as f64;
    let win_h = (wr.bottom - wr.top).max(1) as f64;

    // Inverse of the compositor's crop and scale (it must match `scene::resolve_blit`
    // EXACTLY): the slot shows the cropped region, which under `Fill` is STRETCHED to
    // the whole box (dst = box, aspect not preserved) and under `Fit` is centred with
    // letterboxing. It maps the slot's local point to a fractional position (cu, cv)
    // inside the cropped region.
    let c = slot.crop.sanitized();
    let lx = (pt.x - vx) as f64;
    let ly = (pt.y - vy) as f64;
    let (cu, cv) = match slot.scale {
        // Fill: a linear stretch across the whole box, with no offset (this used to
        // be inverted as cover and the clicks landed shifted, the reported bug).
        ScaleMode::Fill => ((lx / vw).clamp(0.0, 1.0), (ly / vh).clamp(0.0, 1.0)),
        // Fit: the image centred with letterbox bars; a click on a bar hits
        // nothing.
        ScaleMode::Fit => {
            let src_w = (1.0 - c.left - c.right).max(0.001) * win_w;
            let src_h = (1.0 - c.top - c.bottom).max(0.001) * win_h;
            let s = (vw / src_w).min(vh / src_h);
            let (dw, dh) = (src_w * s, src_h * s);
            let (offx, offy) = ((vw - dw) / 2.0, (vh - dh) / 2.0);
            if lx < offx || lx > offx + dw || ly < offy || ly > offy + dh {
                return;
            }
            (
                ((lx - offx) / dw).clamp(0.0, 1.0),
                ((ly - offy) / dh).clamp(0.0, 1.0),
            )
        }
    };
    let fx = c.left + cu * (1.0 - c.left - c.right);
    let fy = c.top + cv * (1.0 - c.top - c.bottom);
    let sxp = (wr.left as f64 + fx * win_w).round() as i32;
    let syp = (wr.top as f64 + fy * win_h).round() as i32;

    // While a left-drag is in progress keep posting to the SAME child captured at
    // button-down, this mirrors `SetCapture` in a real app. Re-running the hit
    // test on every move can land on a different (or transparent-skipped) window
    // mid-drag, which breaks text selection / drag-scroll ("no puedo deslizar").
    let dragging = DRAG_CHILD.with(|d| *d.borrow());
    let child = if matches!(message, WM_MOUSEMOVE | WM_LBUTTONUP) && dragging.is_some() {
        HWND(dragging.unwrap() as *mut core::ffi::c_void)
    } else {
        // Descend to the deepest child window under the mapped point (Chromium
        // nests its render widget a couple of levels in).
        let mut child = target;
        for _ in 0..8 {
            let mut cp = POINT { x: sxp, y: syp };
            let _ = ScreenToClient(child, &mut cp);
            let next = ChildWindowFromPointEx(child, cp, CWP_SKIPINVISIBLE | CWP_SKIPTRANSPARENT);
            if next.0.is_null() || next == child {
                break;
            }
            child = next;
        }
        child
    };
    let mut cp = POINT { x: sxp, y: syp };
    let _ = ScreenToClient(child, &mut cp);
    let lp = make_lparam(cp.x, cp.y);

    let _ = match message {
        WM_MOUSEMOVE => {
            // Carry the *currently held* buttons in wParam. Posting moves with 0
            // told the app no button was down, so drag-select / drag-scroll never
            // worked ("no puedo deslizar"). Reflect the real button state so a
            // press-move-release sequence reads as a drag.
            let mut mk = 0usize;
            if (GetAsyncKeyState(VK_LBUTTON.0 as i32) as u16) & 0x8000 != 0 {
                mk |= MK_LBUTTON;
            }
            if (GetAsyncKeyState(VK_RBUTTON.0 as i32) as u16) & 0x8000 != 0 {
                mk |= MK_RBUTTON;
            }
            PostMessageW(child, WM_MOUSEMOVE, WPARAM(mk), lp)
        }
        WM_LBUTTONDOWN => {
            // Give the parked window keyboard focus so typing flows to it natively
            // (Chromium routes typing/IME to its focused control). Done OUTSIDE the
            // hook: just latch it; the run loop calls `force_foreground` (the
            // `AttachThreadInput`/`SetForegroundWindow` there can block when the app
            // is busy and would stall a low-level hook → laggy/dropped clicks).
            PENDING_FG.with(|p| *p.borrow_mut() = Some(target.0 as isize));
            // Latch this child as the drag-capture target for the moves that follow.
            DRAG_CHILD.with(|d| *d.borrow_mut() = Some(child.0 as isize));
            PostMessageW(child, WM_LBUTTONDOWN, WPARAM(MK_LBUTTON), lp)
        }
        WM_LBUTTONUP => {
            DRAG_CHILD.with(|d| *d.borrow_mut() = None);
            PostMessageW(child, WM_LBUTTONUP, WPARAM(0), lp)
        }
        WM_RBUTTONDOWN => PostMessageW(child, WM_RBUTTONDOWN, WPARAM(MK_RBUTTON), lp),
        WM_RBUTTONUP => {
            // The app will pop a context menu at its own (off-screen) location;
            // ask the run loop to relocate that popup under the real cursor.
            let mut pid = 0u32;
            GetWindowThreadProcessId(target, Some(&mut pid));
            crate::slog!(
                "rclick forwarded, watching popups pid={} cursor=({},{})",
                pid,
                pt.x,
                pt.y
            );
            if pid != 0 {
                PENDING_POPUP.with(|p| {
                    *p.borrow_mut() = Some((
                        pid,
                        target.0 as isize,
                        pt,
                        std::time::Instant::now() + std::time::Duration::from_millis(800),
                    ))
                });
            }
            PostMessageW(child, WM_RBUTTONUP, WPARAM(0), lp)
        }
        WM_XBUTTONDOWN | WM_XBUTTONUP => {
            // Side buttons (back/forward). The pressed button (XBUTTON1=1 / 2=2) is
            // in the high word of `mouseData` (passed in as `wparam` from the hook);
            // it must be echoed in the high word of the posted wParam.
            let xbtn = ((wparam.0 >> 16) & 0xffff) as usize;
            PostMessageW(child, message, WPARAM(xbtn << 16), lp)
        }
        WM_MOUSEWHEEL => {
            // Wheel lParam is in SCREEN coords; wParam high word carries the delta.
            let delta = ((wparam.0 >> 16) & 0xffff) as u32;
            PostMessageW(
                child,
                WM_MOUSEWHEEL,
                WPARAM((delta << 16) as usize),
                make_lparam(sxp, syp),
            )
        }
        _ => Ok(()),
    };
}

pub fn run(mut engine: Engine) -> Result<(), String> {
    unsafe { run_inner(&mut engine) }
}

/// One per-monitor overlay window: a fixed, full-monitor
/// `WS_EX_NOREDIRECTIONBITMAP` window shown through a DirectComposition swapchain
/// ([`GpuOverlay`]). It never resizes (so the interactive hit-test is stable and
/// the mouse isn't "lost" between frames). Compat-window video is sampled
/// straight from the WGC capture texture; note/image/test panels are uploaded as
/// one texture. App-window (non-compat) panels are still real windows.
struct OverlayWin {
    hwnd: HWND,
    mon: MonitorInfo,
    // DEAD CODE, went with the old toggle-transparency path (see
    // `set_surface_transparent`); the overlay is now region-clipped per frame.
    // Commented out (not deleted) as a fallback marker.
    // /// Whether the window currently has `WS_EX_TRANSPARENT` (fully click-through).
    // /// Dropped while an interactive compat panel sits on this monitor so the
    // /// overlay can catch + forward clicks over that slot.
    // transparent: bool,
    /// Reusable monitor-sized RGBA scratch the engine composes notes/images into
    /// before they're uploaded to the swapchain's notes texture.
    buf: Vec<u8>,
    /// Whether the window is currently visible. It is a full-monitor topmost
    /// window, so we keep it hidden whenever this monitor has nothing to draw,
    /// otherwise an empty overlay would sit over the whole screen and (any
    /// click-through gap aside) block every click. Shown only while it carries
    /// a compat capture or note/image on this monitor.
    shown: bool,
    /// The monitor-local content rects the window is currently clipped to (its
    /// `SetWindowRgn`). The overlay is a full-monitor topmost window, but we clip
    /// it to just the panel/note areas so everything OUTSIDE a panel has no window
    /// there at all, clicks fall straight through to the game/background apps
    /// (don't rely on `WS_EX_TRANSPARENT` alone, which a composition overlay does
    /// not honor reliably for routing clicks to other top-level windows).
    region: Vec<SavedRect>,
    /// The click-through hole currently subtracted from `region`: `(cx, cy, r)` in
    /// monitor-local coords, centred on the cursor while it hovers a passthrough-ON
    /// panel. `None` = no hole. Tracked so we only re-clip when it actually moves.
    hole: Option<(i32, i32, i32)>,
    /// Whether the window currently has `WS_EX_LAYERED`. Added while this monitor
    /// shows only engine content (crosshair/notes) so the window is OS-level
    /// click-through, `HTTRANSPARENT` from our wndproc does NOT route a click to
    /// another process's window, it just eats it, which left the mouse "stuck" on
    /// a centred crosshair (games lock the cursor exactly there). Dropped while a
    /// compat panel sits on this monitor: those need the wndproc hit-test to catch
    /// and forward clicks. `NOREDIRECTIONBITMAP` has no redirection surface, so
    /// layering can't bleed outside the region clip like it does on real windows.
    layered: bool,
}

/// Book-keeping for a real app window we've taken over: its original ex-style
/// and geometry (to restore on close), and whether we currently clip it.
struct Managed {
    exstyle: i32,
    style: i32,
    rect: SavedRect,
    cropped: bool,
    /// Whether we currently have the window's chrome (title bar + frame) stripped.
    borderless: bool,
    /// Compat mode: the window is parked off-screen (kept rendering for WGC
    /// capture) instead of placed over the game. See [`park_window`].
    parked: bool,
}

unsafe fn run_inner(engine: &mut Engine) -> Result<(), String> {
    let hinstance = GetModuleHandleW(None).map_err(err)?;
    let class_name = wide("HoardScreenOverlay");

    let wc = WNDCLASSW {
        lpfnWndProc: Some(wndproc),
        hInstance: hinstance.into(),
        lpszClassName: PCWSTR(class_name.as_ptr()),
        ..Default::default()
    };
    if RegisterClassW(&wc) == 0 {
        return Err("RegisterClassW failed".into());
    }

    let mut mons = crate::monitors::list_monitors();
    if mons.is_empty() {
        // Defensive (enumeration returned nothing): fall back to the primary.
        mons.push(MonitorInfo {
            id: 0,
            name: "primary".into(),
            x: 0,
            y: 0,
            w: GetSystemMetrics(SM_CXSCREEN),
            h: GetSystemMetrics(SM_CYSCREEN),
            primary: true,
        });
    }

    // GPU compositor: one shared D3D11 device used for both WGC capture and the
    // per-monitor DirectComposition swapchains. If it can't be created (very old
    // GPU/driver with no D3D11 at all, WGC itself wouldn't work either) there is
    // nothing to fall back to, so surface the error.
    let mut gpu = GpuOverlay::new()?;

    // One fixed, full-monitor `WS_EX_NOREDIRECTIONBITMAP` overlay window per
    // monitor, each fronted by a composition swapchain. It is created
    // click-through (`WS_EX_TRANSPARENT`) and never resized; that fixed geometry
    // is what keeps the interactive hit-test stable (no more "mouse lost over
    // video"). Presentation is `Present(0)` and the run loop paces itself on the
    // message queue (blocking on vblank would freeze hit-testing for a frame).
    let mut overlays: Vec<OverlayWin> = Vec::with_capacity(mons.len());
    for (idx, mon) in mons.iter().enumerate() {
        let hwnd = CreateWindowExW(
            WS_EX_NOREDIRECTIONBITMAP
                | WS_EX_TRANSPARENT
                | WS_EX_TOPMOST
                | WS_EX_TOOLWINDOW
                | WS_EX_NOACTIVATE,
            PCWSTR(class_name.as_ptr()),
            PCWSTR(wide("Hoard Screen").as_ptr()),
            WS_POPUP,
            mon.x,
            mon.y,
            mon.w,
            mon.h,
            None,
            None,
            hinstance,
            None,
        )
        .map_err(err)?;
        gpu.add_monitor(hwnd, mon.w, mon.h)?;
        // Present one transparent frame so the swapchain is clear, but keep the
        // window HIDDEN until this monitor actually has something to draw, an
        // always-visible full-screen topmost window would blanket the desktop.
        gpu.render(idx, &[])?;
        overlays.push(OverlayWin {
            hwnd,
            mon: mon.clone(),
            // transparent: true, // DEAD CODE, see OverlayWin field above
            buf: vec![0u8; (mon.w as usize) * (mon.h as usize) * 4],
            shown: false,
            region: Vec::new(),
            hole: None,
            layered: false,
        });
    }
    crate::slog!(
        "overlays ready: {} monitor(s) {:?}",
        overlays.len(),
        mons.iter()
            .map(|m| (m.x, m.y, m.w, m.h))
            .collect::<Vec<_>>()
    );

    // No low-level mouse hook: an interactive (passthrough-OFF) compat slot is
    // caught by the overlay itself (its `wndproc` returns HTCLIENT over that slot)
    // and forwarded to the parked capture from the run loop below, a SINGLE,
    // clean forward (the old LL hook also forwarded, so moves got posted twice and
    // drag-select broke). A passthrough-ON slot is handled by the circular region
    // hole that follows the cursor (see `passthrough_hole`): the OS routes clicks
    // inside the hole natively to whatever sits behind, so nothing to hook there
    // either.

    // Per-monitor flag: does the engine (notes/images/test) currently draw here,
    // so we should include its uploaded texture as a quad.
    let mut notes_present = vec![false; mons.len()];

    // The global Ctrl+O is process-wide, so one registration (on any window)
    // serves every monitor.
    let hotkey_hwnd = overlays[0].hwnd;
    let _ = RegisterHotKey(hotkey_hwnd, HOTKEY_TOGGLE, MOD_CONTROL | MOD_NOREPEAT, VK_O);

    let mut mode = Mode::View;
    // The authoritative full scene (window + non-window panels). The engine only
    // ever gets the note/image/test subset; window panels live either as real
    // placed windows (`managed`) or, compat in View, as GPU captures.
    let mut scene = Scene::default();
    let mut managed: HashMap<isize, Managed> = HashMap::new();

    let rx = spawn_stdin_reader();
    let mut frame_no: u64 = 0;
    // Set whenever the scene/mode changes so the next frame repaints even if no
    // capture produced a new frame (geometry moved, a panel appeared/vanished).
    let mut scene_dirty = true;
    // Whether our windows currently carry WDA_EXCLUDEFROMCAPTURE (scope active).
    let mut affinity_on = false;

    loop {
        let frame_start = Instant::now();
        frame_no = frame_no.wrapping_add(1);

        // Drain the message queue; act on hotkeys. `PeekMessageW(None)` covers
        // every window owned by this thread, i.e. all the per-monitor overlays.
        let mut msg = MSG::default();
        while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
            match msg.message {
                WM_MOUSEMOVE | WM_LBUTTONDOWN | WM_LBUTTONUP | WM_RBUTTONDOWN | WM_RBUTTONUP
                | WM_XBUTTONDOWN | WM_XBUTTONUP | WM_MOUSEWHEEL => {
                    // Reached the queue only because wndproc returned HTCLIENT over
                    // an interactive compat slot: translate + post to the parked
                    // window instead of dispatching it to the inert overlay.
                    forward_pointer(msg.message, msg.wParam);
                    continue;
                }
                _ => {}
            }
            if msg.message == WM_HOTKEY {
                match msg.wParam.0 as i32 {
                    HOTKEY_TOGGLE => {
                        let next = mode.toggled();
                        set_mode(
                            hotkey_hwnd,
                            &mut scene,
                            &mut managed,
                            &mut mode,
                            &mons,
                            next,
                        );
                        scene_dirty = true;
                        emit_mode(mode);
                        if mode == Mode::View {
                            emit_scene(&scene);
                        }
                    }
                    HOTKEY_ESCAPE if mode == Mode::Editor => {
                        set_mode(
                            hotkey_hwnd,
                            &mut scene,
                            &mut managed,
                            &mut mode,
                            &mons,
                            Mode::View,
                        );
                        scene_dirty = true;
                        emit_mode(mode);
                        emit_scene(&scene);
                    }
                    _ => {}
                }
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        // Hand the parked window the foreground here (not inside the mouse hook):
        // the click already posted instantly; the focus change can block when the
        // app is busy, so keep it off the hook's hot path.
        if let Some(t) = PENDING_FG.with(|p| p.borrow_mut().take()) {
            force_foreground(HWND(t as *mut core::ffi::c_void));
        }

        loop {
            match rx.try_recv() {
                Ok(Message::SetScene { scene: next }) => {
                    scene = next;
                    // Engine only ever gets note/image/test panels now; window
                    // panels are real placed windows or (compat in View) GPU
                    // captures driven directly here.
                    engine.set_scene(engine_subset(&scene));
                    apply_windows(&scene, &mons, &mut managed, mode == Mode::Editor);
                    scene_dirty = true;
                    // A scope lens samples the screen under itself; keep OUR
                    // windows out of every capture path while one exists or the
                    // lens recursively magnifies itself. (Side effect while on:
                    // the overlay also disappears from OBS/recordings.)
                    let want_affinity = scene
                        .panels
                        .iter()
                        .any(|p| matches!(p.source, SourceRef::Scope(_)));
                    if want_affinity != affinity_on {
                        for ov in overlays.iter() {
                            let _ = SetWindowDisplayAffinity(
                                ov.hwnd,
                                if want_affinity {
                                    WDA_EXCLUDEFROMCAPTURE
                                } else {
                                    WDA_NONE
                                },
                            );
                        }
                        affinity_on = want_affinity;
                    }
                }
                Ok(Message::SetEditor { editor }) => {
                    let want = if editor { Mode::Editor } else { Mode::View };
                    if want != mode {
                        set_mode(
                            hotkey_hwnd,
                            &mut scene,
                            &mut managed,
                            &mut mode,
                            &mons,
                            want,
                        );
                        scene_dirty = true;
                        if mode == Mode::View {
                            emit_scene(&scene);
                        }
                    }
                }
                Ok(Message::GetScene) => {
                    // Desktop resync: tell it what is ACTUALLY on screen.
                    emit_mode(mode);
                    emit_scene(&scene);
                }
                // Quit, or the desktop app died (stdin hit EOF, sender gone).
                // A controllerless overlay must not outlive its app: it kept
                // running forever with panels nobody could move or close.
                Ok(Message::Quit) | Err(mpsc::TryRecvError::Disconnected) => {
                    for (&win, m) in managed.iter() {
                        restore_window(HWND(win as *mut core::ffi::c_void), m);
                    }
                    managed.clear();
                    let _ = UnregisterHotKey(hotkey_hwnd, HOTKEY_TOGGLE);
                    return Ok(());
                }
                Err(mpsc::TryRecvError::Empty) => break,
            }
        }

        // Which compat windows to capture this frame (parked, in View) and which
        // overlays must drop click-through to forward interactive clicks.
        let mut wanted: Vec<isize> = Vec::new();
        if mode == Mode::View {
            // The overlay is clipped to its content by region (not permanently
            // click-through); clicks over a slot are caught by `wndproc` (HTCLIENT)
            // and forwarded from the run loop. We only need the slot list current.
            rebuild_slots(&scene, &mons, &managed);
            for p in scene.draw_order() {
                if let (Some(win), true) = (panel_win(p), p.compat) {
                    if !wanted.contains(&win) {
                        wanted.push(win);
                    }
                }
            }
        } else {
            // Editor: windows are real and natively interactive, no forwarding,
            // no captures. An empty slot list makes `wndproc` fall through
            // (HTTRANSPARENT) everywhere, so the overlay catches nothing.
            clear_slots();
        }
        gpu.reconcile_captures(&wanted);

        let caps_new = gpu.poll_captures();
        engine.set_editing(mode == Mode::Editor);
        let notes_new = engine.tick();
        let need_render = caps_new || notes_new || scene_dirty;

        if need_render {
            let refresh_notes = notes_new || scene_dirty;
            for (idx, ov) in overlays.iter_mut().enumerate() {
                if refresh_notes {
                    if engine_bbox(engine, &ov.mon).is_some() {
                        engine.render_monitor(
                            &mut ov.buf,
                            ov.mon.w as u32,
                            ov.mon.h as u32,
                            ov.mon.id,
                        );
                        gpu.upload_notes(idx, &ov.buf)?;
                        notes_present[idx] = true;
                    } else {
                        notes_present[idx] = false;
                    }
                }

                let mut quads: Vec<Quad> = Vec::new();
                if mode == Mode::View {
                    for p in scene.draw_order() {
                        let Some(win) = panel_win(p) else { continue };
                        if !p.compat || !p.monitor.draws_on(ov.mon.id) {
                            continue;
                        }
                        let Some((srv, sw, sh)) = gpu.capture_srv(win) else {
                            continue;
                        };
                        let r = p.rect.normalized();
                        let c = p.crop.sanitized();
                        let uv = (
                            c.left as f32,
                            c.top as f32,
                            (1.0 - c.right) as f32,
                            (1.0 - c.bottom) as f32,
                        );
                        // Map crop+scale to a destination rect, mirroring
                        // `scene::resolve_blit`: Fill stretches the cropped region
                        // across the whole box; Fit centers it with letterbox bars
                        // (left transparent so the game shows through).
                        let dst = match p.scale {
                            ScaleMode::Fill => (
                                r.x as f32,
                                r.y as f32,
                                r.w.max(1.0) as f32,
                                r.h.max(1.0) as f32,
                            ),
                            ScaleMode::Fit => {
                                let src_w = (1.0 - c.left - c.right).max(0.001) * sw as f64;
                                let src_h = (1.0 - c.top - c.bottom).max(0.001) * sh as f64;
                                let s = (r.w / src_w).min(r.h / src_h);
                                let (dw, dh) = (src_w * s, src_h * s);
                                (
                                    (r.x + (r.w - dw) / 2.0) as f32,
                                    (r.y + (r.h - dh) / 2.0) as f32,
                                    dw as f32,
                                    dh as f32,
                                )
                            }
                        };
                        quads.push(Quad {
                            srv,
                            dst,
                            uv,
                            opaque: true,
                        });
                    }
                }
                // Notes/images on top of the captured video.
                if notes_present[idx] {
                    if let Some(srv) = gpu.notes_srv(idx) {
                        quads.push(Quad {
                            srv,
                            dst: (0.0, 0.0, ov.mon.w as f32, ov.mon.h as f32),
                            uv: (0.0, 0.0, 1.0, 1.0),
                            opaque: false,
                        });
                    }
                }
                gpu.render(idx, &quads)?;
            }
            scene_dirty = false;
        }

        // Show each overlay only while its monitor carries something (a compat
        // capture in View, or a note/image). Empty monitors stay hidden so the
        // full-screen topmost window never blankets the desktop / blocks clicks.
        for (idx, ov) in overlays.iter_mut().enumerate() {
            let has_compat = mode == Mode::View
                && scene
                    .draw_order()
                    .iter()
                    .any(|p| p.compat && panel_win(p).is_some() && p.monitor.draws_on(ov.mon.id));
            let active = has_compat || notes_present[idx];
            // Engine-only content (crosshair/notes) is pure visual: make the
            // window OS-level click-through (`LAYERED` + the `TRANSPARENT` it
            // already has) so the mouse never sees it. Our wndproc can't route a
            // non-slot click to another process (`HTTRANSPARENT` just eats it),
            // which pinned the cursor on a centred crosshair in cursor-locking
            // games. With a compat panel here the wndproc must hit-test, so
            // layering comes off. (Known gap: compat + crosshair on the SAME
            // monitor keeps the trap over the crosshair rect.)
            let want_layered = !has_compat;
            if want_layered != ov.layered {
                let mut ex = GetWindowLongW(ov.hwnd, GWL_EXSTYLE) as u32;
                if want_layered {
                    ex |= WS_EX_LAYERED.0;
                } else {
                    ex &= !WS_EX_LAYERED.0;
                }
                let _ = SetWindowLongW(ov.hwnd, GWL_EXSTYLE, ex as i32);
                if want_layered {
                    // Attributes must be set once after adding LAYERED or the
                    // window counts as "unset" and may not display.
                    let _ = SetLayeredWindowAttributes(ov.hwnd, COLORREF(0), 255, LWA_ALPHA);
                }
                ov.layered = want_layered;
            }
            // Clip the overlay to just its content so the rest of the monitor is
            // not covered by any window (clicks reach the game/background apps).
            if active {
                let rects =
                    overlay_content_rects(&scene, engine, &ov.mon, mode, notes_present[idx]);
                // Circular click-through lens: a hole under the cursor while it
                // hovers a passthrough-ON panel, so clicks reach the game/desktop
                // behind NATIVELY (no synthetic forwarding). Recomputed every frame
                // so it tracks the cursor; re-clip only when region or hole change.
                let hole = if mode == Mode::View {
                    passthrough_hole(&ov.mon)
                } else {
                    None
                };
                if rects != ov.region || hole != ov.hole {
                    apply_overlay_region(ov.hwnd, &rects, hole);
                    ov.region = rects;
                    ov.hole = hole;
                }
            }
            if active != ov.shown {
                let _ = ShowWindow(ov.hwnd, if active { SW_SHOWNOACTIVATE } else { SW_HIDE });
                ov.shown = active;
            }
        }

        if mode == Mode::View {
            reposition_popup(&mons);
            if frame_no % REASSERT_EVERY == 0 {
                reassert_z(&scene, &managed);
                // The overlay (crosshair/scope/notes + compat quads) stays
                // visually ABOVE the placed real windows: widgets are meant to
                // float over everything, and among themselves panel `z` orders
                // them inside the swapchain. Raised after the app windows so
                // this window wins the TOPMOST band.
                for ov in overlays.iter() {
                    if ov.shown {
                        let _ = SetWindowPos(
                            ov.hwnd,
                            HWND_TOPMOST,
                            0,
                            0,
                            0,
                            0,
                            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                        );
                    }
                }
            }
        } else if frame_no % REASSERT_EVERY == 0 {
            // A window won't shrink below its own minimum size (and the user can
            // move/resize the real window), so the panel rect the editor shows can
            // drift from reality. Fold the real geometry back and push it so the
            // editor reflects the actual size. The editor ignores incoming scenes
            // mid-drag, so this only lands once the user releases.
            if readback_geometry(&mut scene, &mons, &managed) {
                emit_scene(&scene);
            }
        }

        // Pace the loop WITHOUT ever blocking the thread to vblank: a plain
        // `sleep` can't wake for an incoming `WM_NCHITTEST`, so with a full-screen
        // topmost overlay the mouse would freeze for the whole sleep. Wait on the
        // message queue instead, `MsgWaitForMultipleObjectsEx` returns the instant
        // input arrives (so hit-tests are answered immediately) or when the budget
        // elapses (so captures keep polling). Budget: tight while capturing video,
        // relaxed when only static notes are up.
        let budget = if !wanted.is_empty() {
            8u32
        } else if need_render {
            8
        } else {
            30
        };
        let elapsed = frame_start.elapsed().as_millis() as u32;
        let wait = budget.saturating_sub(elapsed);
        if wait > 0 {
            let _ = MsgWaitForMultipleObjectsEx(None, wait, QS_ALLINPUT, MWMO_INPUTAVAILABLE);
        }

        // Frame-time watchdog: anything that parks this thread long enough to
        // freeze input shows up here as an outlier in the log.
        let took = frame_start.elapsed().as_millis();
        if took > 60 {
            crate::swarn!("slow frame: {took} ms (render={need_render})");
        }

        // Periodic capture health: frame counts that stop climbing = a frozen
        // capture (Discord/Brave going black).
        if frame_no % 120 == 0 {
            for (h, frames, has) in gpu.capture_stats() {
                crate::slog!("capture hwnd={h:#x} frames={frames} has_frame={has}");
            }
        }
    }
}

/// Monitor-local bounding box `(x, y, w, h)` of every engine panel drawing on
/// `mon`, clamped to the monitor. `None` when no panel touches this monitor, so
/// the caller can hide that surface instead of presenting an empty full screen.
/// This is what lets the overlay size each window to its content (cheap
/// `UpdateLayeredWindow`) rather than blitting the whole monitor every frame.
fn engine_bbox(engine: &Engine, mon: &MonitorInfo) -> Option<SavedRect> {
    let (mut min_x, mut min_y) = (f64::MAX, f64::MAX);
    let (mut max_x, mut max_y) = (f64::MIN, f64::MIN);
    let mut any = false;
    for p in &engine.scene().panels {
        if !p.monitor.draws_on(mon.id) {
            continue;
        }
        any = true;
        min_x = min_x.min(p.rect.x);
        min_y = min_y.min(p.rect.y);
        max_x = max_x.max(p.rect.x + p.rect.w);
        max_y = max_y.max(p.rect.y + p.rect.h);
    }
    if !any {
        return None;
    }
    let x0 = (min_x.floor() as i32).clamp(0, mon.w);
    let y0 = (min_y.floor() as i32).clamp(0, mon.h);
    let x1 = (max_x.ceil() as i32).clamp(0, mon.w);
    let y1 = (max_y.ceil() as i32).clamp(0, mon.h);
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    Some((x0, y0, x1 - x0, y1 - y0))
}

/// Monitor-local content rects to clip the overlay to: every compat panel box on
/// this monitor (View) plus the note/image bounding box. Outside these the
/// overlay has no window, so clicks fall through to whatever is beneath.
fn overlay_content_rects(
    scene: &Scene,
    engine: &Engine,
    mon: &MonitorInfo,
    mode: Mode,
    notes_present: bool,
) -> Vec<SavedRect> {
    let mut rects: Vec<SavedRect> = Vec::new();
    if mode == Mode::View {
        for p in scene.draw_order() {
            if !p.compat || panel_win(p).is_none() || !p.monitor.draws_on(mon.id) {
                continue;
            }
            let r = p.rect.normalized();
            rects.push((
                r.x as i32,
                r.y as i32,
                r.w.max(1.0) as i32,
                r.h.max(1.0) as i32,
            ));
        }
    }
    if notes_present {
        if let Some(bb) = engine_bbox(engine, mon) {
            rects.push(bb);
        }
    }
    rects
}

/// Clip `hwnd` to the union of `rects` (monitor-local, i.e. relative to the
/// window's top-left), minus an optional circular `hole` `(cx, cy, r)`, the
/// click-through "lens" that follows the cursor over a passthrough-ON panel so
/// the OS delivers clicks NATIVELY to whatever sits behind it. Empty `rects` with
/// no hole clears the clip (whole window visible).
unsafe fn apply_overlay_region(hwnd: HWND, rects: &[SavedRect], hole: Option<(i32, i32, i32)>) {
    if rects.is_empty() && hole.is_none() {
        let _ = SetWindowRgn(hwnd, HRGN(std::ptr::null_mut()), TRUE);
        return;
    }
    let rgn = CreateRectRgn(0, 0, 0, 0);
    for &(x, y, w, h) in rects {
        let r = CreateRectRgn(x, y, x + w, y + h);
        CombineRgn(rgn, rgn, r, RGN_OR);
        let _ = DeleteObject(HGDIOBJ(r.0));
    }
    if let Some((cx, cy, rad)) = hole {
        let circle = CreateEllipticRgn(cx - rad, cy - rad, cx + rad, cy + rad);
        CombineRgn(rgn, rgn, circle, RGN_DIFF);
        let _ = DeleteObject(HGDIOBJ(circle.0));
    }
    // The window owns the region after this call; the system frees it.
    let _ = SetWindowRgn(hwnd, rgn, TRUE);
}

/// After a right-click was forwarded to a parked compat window, find its freshly
/// opened context-menu popup (a visible `WS_POPUP` top-level of the same PID sitting
/// in the off-screen park corner) and move it under the cursor, so menus appear
/// where the user clicked instead of bottom-right where the real window is parked.
/// No-op (cheap) when nothing is pending. Clears the latch once handled or expired.
unsafe fn reposition_popup(mons: &[MonitorInfo]) {
    let Some((pid, parked, cursor, deadline)) = PENDING_POPUP.with(|p| *p.borrow()) else {
        return;
    };
    if std::time::Instant::now() > deadline {
        PENDING_POPUP.with(|p| *p.borrow_mut() = None);
        return;
    }
    let mut cur = GetTopWindow(HWND::default()).ok();
    while let Some(w) = cur {
        if w.0.is_null() {
            break;
        }
        cur = GetWindow(w, GW_HWNDNEXT).ok();
        if w.0 as isize == parked {
            continue; // never the captured window itself
        }
        let mut wpid = 0u32;
        GetWindowThreadProcessId(w, Some(&mut wpid));
        if wpid != pid || !IsWindowVisible(w).as_bool() {
            continue;
        }
        let style = GetWindowLongW(w, GWL_STYLE) as u32;
        let mut r = RECT::default();
        if GetWindowRect(w, &mut r).is_err() {
            continue;
        }
        let (pw, ph) = (r.right - r.left, r.bottom - r.top);
        if style & WS_POPUP.0 == 0 {
            continue;
        }
        // A context menu is a small WS_POPUP. The app anchors it to its real
        // (parked) position, so the shell clamps it far from the cursor (e.g.
        // bottom-right). Move any small popup whose top-left isn't already at the
        // click point; a correctly placed menu opens with its corner at the cursor.
        let near_cursor = (r.left - cursor.x).abs() <= 40 && (r.top - cursor.y).abs() <= 40;
        if pw <= 0 || ph <= 0 || pw > 700 || near_cursor {
            continue;
        }
        // Place it under the cursor, clamped so it stays fully on the cursor's monitor.
        let mon = mons
            .iter()
            .find(|m| {
                cursor.x >= m.x && cursor.x < m.x + m.w && cursor.y >= m.y && cursor.y < m.y + m.h
            })
            .or_else(|| mons.first());
        let (mut nx, mut ny) = (cursor.x, cursor.y);
        if let Some(m) = mon {
            nx = nx.min(m.x + m.w - pw).max(m.x);
            ny = ny.min(m.y + m.h - ph).max(m.y);
        }
        let _ = SetWindowPos(w, HWND_TOPMOST, nx, ny, 0, 0, SWP_NOSIZE | SWP_NOACTIVATE);
        PENDING_POPUP.with(|p| *p.borrow_mut() = None);
        return;
    }
}

/// If the cursor currently hovers a passthrough-ON compat slot on `mon`, return
/// the monitor-local circular hole `(cx, cy, r)` to cut from the overlay (centred
/// on the cursor); otherwise `None`.
unsafe fn passthrough_hole(mon: &MonitorInfo) -> Option<(i32, i32, i32)> {
    let mut pt = POINT::default();
    if GetCursorPos(&mut pt).is_err() {
        return None;
    }
    let rad = SLOTS.with(|s| {
        s.borrow()
            .iter()
            .find(|sl| sl.passthrough && slot_contains(sl, pt.x, pt.y))
            .map(|sl| sl.hole_radius)
    })?;
    Some((pt.x - mon.x, pt.y - mon.y, rad))
}

/// The panels the compositing engine should render: only note/image/test panels.
/// Window panels never reach the engine, non-compat ones are real OS windows
/// placed over the game, and compat ones (in View) are captured straight to the
/// GPU overlay ([`GpuOverlay`]) instead of going through the CPU compositor.
fn engine_subset(scene: &Scene) -> Scene {
    Scene {
        panels: scene
            .panels
            .iter()
            .filter(|p| !matches!(&p.source, SourceRef::Window { .. }))
            .cloned()
            .collect(),
    }
}

/// The monitor a panel is drawn on (its target, or the primary for a mirror).
fn target_monitor<'a>(mons: &'a [MonitorInfo], p: &Panel) -> &'a MonitorInfo {
    let want = match p.monitor {
        PanelTarget::Monitor { id } => id,
        PanelTarget::All => 0,
    };
    mons.iter().find(|m| m.id == want).unwrap_or(&mons[0])
}

/// The monitor whose bounds contain the virtual-desktop point `(x, y)`.
fn monitor_at<'a>(mons: &'a [MonitorInfo], x: i32, y: i32) -> &'a MonitorInfo {
    mons.iter()
        .find(|m| x >= m.x && x < m.x + m.w && y >= m.y && y < m.y + m.h)
        .unwrap_or(&mons[0])
}

/// Place every window panel's real HWND over the game and restore the ones that
/// left the scene. `interactive` is true in Editor (window grabs input) and
/// false in View (click-through, no-activate).
unsafe fn apply_windows(
    scene: &Scene,
    mons: &[MonitorInfo],
    managed: &mut HashMap<isize, Managed>,
    interactive: bool,
) {
    let mut desired: HashSet<isize> = HashSet::new();
    for p in scene.draw_order() {
        let Some(win) = panel_win(p) else { continue };
        desired.insert(win);
        let target = HWND(win as *mut core::ffi::c_void);
        let m = managed
            .entry(win)
            .or_insert_with(|| unsafe { adopt(target) });
        if !interactive && p.compat {
            // View + compat: get the real window out of the way but keep it
            // rendering; the engine WGC-captures it and the compositor draws the
            // cropped pixels onto the slot. Park it at the SLOT size so WGC
            // captures at the resolution it's shown at (parking at the window's
            // original size captured low-res → blurry upscale on the slot).
            let r = p.rect.normalized();
            park_window(target, mons, m, r.w.max(1.0) as i32, r.h.max(1.0) as i32);
        } else {
            place_window(target, target_monitor(mons, p), p, interactive, m);
        }
    }
    // Restore windows whose panel is gone.
    let gone: Vec<isize> = managed
        .keys()
        .copied()
        .filter(|w| !desired.contains(w))
        .collect();
    for win in gone {
        if let Some(m) = managed.remove(&win) {
            restore_window(HWND(win as *mut core::ffi::c_void), &m);
        }
    }
}

/// Snapshot a window's original ex-style + geometry so we can restore it later.
unsafe fn adopt(hwnd: HWND) -> Managed {
    Managed {
        exstyle: GetWindowLongW(hwnd, GWL_EXSTYLE),
        style: GetWindowLongW(hwnd, GWL_STYLE),
        rect: window_rect(hwnd).unwrap_or((0, 0, 1, 1)),
        cropped: false,
        borderless: false,
        parked: false,
    }
}

/// Window-style bits that draw the title bar + resizable frame. We strip these in
/// View so an adopted window shows only its content (no caption to drag, no
/// border), and restore them in Editor / on close.
const CHROME: u32 =
    WS_CAPTION.0 | WS_THICKFRAME.0 | WS_MINIMIZEBOX.0 | WS_MAXIMIZEBOX.0 | WS_SYSMENU.0;

/// Compat mode (View): park the real window fully off every monitor, at its
/// natural size, with its own original styles restored (no clip, no chrome
/// strip, no layering). DWM keeps composing a non-minimized window even when
/// it's off-screen, and nothing occludes it there, so Chromium/Electron won't
/// background it and its video keeps playing, which is the whole point: the
/// engine WGC-captures these live pixels and the compositor draws the cropped
/// result onto the panel slot. Idempotent: once parked we leave it be.
unsafe fn park_window(hwnd: HWND, mons: &[MonitorInfo], m: &mut Managed, cap_w: i32, cap_h: i32) {
    let max_right = mons.iter().map(|mm| mm.x + mm.w).max().unwrap_or(0);
    let max_bottom = mons.iter().map(|mm| mm.y + mm.h).max().unwrap_or(0);
    let park_x = max_right - 2;
    let park_y = max_bottom - 2;
    let (cw, ch) = (cap_w.max(1), cap_h.max(1));
    if m.parked {
        // Already parked: only resize if the slot size changed, so the capture
        // resolution tracks the displayed size without re-running the full park.
        if let Some((_, _, w, h)) = window_rect(hwnd) {
            if w != cw || h != ch {
                // Reassert TOPMOST on resize (drop SWP_NOZORDER) so the 2px nub
                // stays above any app the user maximizes over it (see below).
                let _ = SetWindowPos(hwnd, HWND_TOPMOST, park_x, park_y, cw, ch, SWP_NOACTIVATE);
            }
        }
        return;
    }
    if IsIconic(hwnd).as_bool() {
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    }
    if IsZoomed(hwnd).as_bool() {
        let _ = ShowWindow(hwnd, SW_RESTORE);
    }
    // Undo anything a prior View placement applied so the window paints normally
    // while captured.
    if m.cropped {
        let _ = SetWindowRgn(hwnd, HRGN(std::ptr::null_mut()), TRUE);
        m.cropped = false;
    }
    let _ = SetWindowLongW(hwnd, GWL_EXSTYLE, m.exstyle);
    if m.borderless {
        let _ = SetWindowLongW(hwnd, GWL_STYLE, m.style);
        m.borderless = false;
    }
    // Park it so only a 2px nub at the very bottom-right corner stays on a
    // display. A window that intersects *no* monitor is treated as hidden by
    // Chromium/Electron occlusion (CalculateNativeWinOcclusion) and they throttle
    // rendering to a stop, which froze the capture at a few frames. Keeping a
    // tiny on-screen intersection makes them keep painting, while being
    // effectively invisible. Sized to the slot (`cap_w`×`cap_h`) so WGC captures
    // at the resolution it's displayed at, crisp, not an upscaled low-res frame.
    //
    // The nub must be TOPMOST (below). Occlusion is computed from the *visible*
    // on-screen region: if another window (another Brave/Discord maximised) covers
    // the nub, Chromium counts the parked window as fully occluded and freezes the
    // capture again. A topmost nub can't be covered by ordinary windows, so it
    // stays painting. (Fullscreen-exclusive games like Factorio bypass this, which
    // is why they never triggered the freeze.)
    let (w, h) = (cw, ch);
    // Force the window into a NORMAL (non-maximized) state at the park rect.
    // `SetWindowPos` alone is ignored by a maximized window, it stayed maximized
    // and fully visible behind the captured panel (the "a second, maximised Brave
    // behind it" report). `SetWindowPlacement` with `SW_SHOWNORMAL` reliably
    // un-maximizes and positions it.
    let mut wp = WINDOWPLACEMENT {
        length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
        ..Default::default()
    };
    if GetWindowPlacement(hwnd, &mut wp).is_ok() {
        wp.showCmd = SW_SHOWNORMAL.0 as u32;
        wp.rcNormalPosition = RECT {
            left: park_x,
            top: park_y,
            right: park_x + w,
            bottom: park_y + h,
        };
        let _ = SetWindowPlacement(hwnd, &wp);
    }
    let _ = SetWindowPos(
        hwnd,
        HWND_TOPMOST,
        park_x,
        park_y,
        w,
        h,
        SWP_NOACTIVATE | SWP_FRAMECHANGED,
    );
    m.parked = true;
}

/// Move/size a real window onto its panel slot, set click-through per mode, and
/// clip it to its crop. Panel rects are monitor-local, so offset by the target
/// monitor's origin (a mirror panel is placed on the primary).
unsafe fn place_window(
    hwnd: HWND,
    mon: &MonitorInfo,
    p: &Panel,
    interactive: bool,
    m: &mut Managed,
) {
    // Taking the window back from a parked (compat) state: it's a normal placed
    // window again from here on; the style/pos/crop below re-establish it.
    m.parked = false;
    // A minimized window would show nothing wherever we place it, restore it
    // first, without stealing focus from the game (guarded so we never disturb a
    // normal/maximized window).
    if IsIconic(hwnd).as_bool() {
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    }
    // A maximized window ignores our slot: `SetWindowPos` moves it but it keeps
    // the maximized state, so the app snaps it back to fullscreen on its next
    // relayout (and re-maximizes when we move/crop it). Un-maximize first. This
    // is a one-shot (once restored `IsZoomed` is false), so the single focus
    // activation it costs doesn't repeat per frame.
    if IsZoomed(hwnd).as_bool() {
        let _ = ShowWindow(hwnd, SW_RESTORE);
    }

    let r = p.rect.normalized();
    let (vx, vy) = (mon.x + r.x as i32, mon.y + r.y as i32);
    let (w, h) = (r.w.max(1.0) as i32, r.h.max(1.0) as i32);

    // Ex-style policy for the adopted (foreign) window:
    //
    // * Editor (`interactive`): restore the window's own original ex-style so the
    //   user can focus, drag, resize and interact (e.g. pause the video). Never
    //   touch click-through here.
    //
    // * View, `passthrough` (default): add `WS_EX_LAYERED | WS_EX_TRANSPARENT |
    //   WS_EX_NOACTIVATE` so the window is hit-test-transparent (mouse falls
    //   through to the game below) and never steals focus. The `LAYERED` flag is
    //   load-bearing: for a top-level FOREIGN window, `WS_EX_TRANSPARENT` ALONE
    //   does not make it click-through, Win32 only routes the click to the
    //   window below when the window is also `WS_EX_LAYERED` (multi-HWND Chromium
    //   apps like Discord/Brave otherwise keep catching the click on their child
    //   render surface). We do NOT paint the layered window ourselves (no
    //   `UpdateLayeredWindow`/alpha set), so DWM composites it at full opacity as
    //   usual; for ordinary apps this is invisible. The known exception is a
    //   window owning a hardware video swapchain (Prime/Netflix, MPO + HW-accel):
    //   layering can break its composition and show black, that's what the
    //   per-panel `passthrough` toggle is for (turn it off on that one panel).
    //
    // * View, `!passthrough`: leave the adopted window's ex-style exactly as we
    //   found it. The video then composites with no style we imposed (the safest
    //   thing for a protected/accelerated swapchain), at the cost that clicks
    //   over THIS panel's rectangle land on the browser, not the game. Every
    //   other panel keeps click-through, so gameplay is unaffected except
    //   directly over this one video. This is the per-panel escape hatch for the
    //   black-video case; the browser-side causes (Chromium occlusion + MPO) are
    //   out of our control.
    //
    // Trade-off, stated plainly: there is no single ex-style that both keeps the
    // game click-through AND guarantees a protected/HW video never goes black,
    // click-through *requires* altering the foreign window's hit-testing. So we
    // expose it per panel and default to click-through (gameplay-preserving).
    // A layered window does NOT honor `SetWindowRgn` cleanly: the DWM
    // redirection surface bleeds through outside the clip as a gray/black border
    // (seen on Explorer and any cropped panel). So we only layer an UNcropped
    // passthrough panel; a cropped one keeps a plain transparent style so its
    // region clips cleanly. The cost: click-through over a cropped Chromium panel
    // is best-effort (TRANSPARENT alone), but a clean crop matters more there.
    let c = p.crop.sanitized();
    let has_crop = !(c.left == 0.0 && c.right == 0.0 && c.top == 0.0 && c.bottom == 0.0);
    let layered = !interactive && p.passthrough && !has_crop;
    let ex = if interactive {
        m.exstyle as u32
    } else if p.passthrough {
        let base = m.exstyle as u32 | WS_EX_TRANSPARENT.0 | WS_EX_NOACTIVATE.0;
        if layered {
            base | WS_EX_LAYERED.0
        } else {
            base
        }
    } else {
        // Leave NOACTIVATE off too: if the user can click it, they may want to
        // focus/pause it. Keep the window's own style untouched.
        m.exstyle as u32
    };
    let _ = SetWindowLongW(hwnd, GWL_EXSTYLE, ex as i32);

    // Chrome policy: in View strip the title bar + frame so only the app's
    // content shows (a clean panel, no caption to drag, no border bleeding past
    // the crop, and it fixes the "the top bar and the border stayed" report). In
    // Editor restore the real style so the user can drag/resize it natively.
    // Toggling the style requires `SWP_FRAMECHANGED` for the non-client area to
    // recompute, so we only do it on transition (not every frame).
    let want_borderless = !interactive;
    let mut frame_changed = false;
    if want_borderless != m.borderless {
        let new_style = if want_borderless {
            (m.style as u32) & !CHROME
        } else {
            m.style as u32
        };
        let _ = SetWindowLongW(hwnd, GWL_STYLE, new_style as i32);
        m.borderless = want_borderless;
        frame_changed = true;
    }
    // A freshly `WS_EX_LAYERED` top-level window renders NOTHING until its layer
    // attributes are set, without this the adopted app would vanish instead of
    // becoming click-through. Alpha 255 = fully opaque: we use the layer purely
    // to route clicks through, not to fade the window. DWM keeps compositing its
    // real content (the foreign app paints normally; we never call
    // `UpdateLayeredWindow` on it). A protected/HW-video swapchain is the one
    // case this can black out, handled by turning the panel's `passthrough` off.
    if layered {
        let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_ALPHA);
    }

    let swp = if frame_changed {
        SWP_NOACTIVATE | SWP_FRAMECHANGED
    } else {
        SWP_NOACTIVATE
    };
    let _ = SetWindowPos(hwnd, HWND_TOPMOST, vx, vy, w, h, swp);
    apply_crop(hwnd, w, h, p.crop, m);
    // NB: deliberately NO `SetForegroundWindow` here. `place_window` runs on
    // EVERY `set_scene`, and the editor streams one per pointer-move (~30/s)
    // while the user drags a panel rectangle in the desktop UI. Forcing the
    // adopted window foreground each time yanked focus away from the Hoard
    // window mid-drag, a focus/z-order war that flickered the panel between its
    // old and new spot (and, when the adopted window WAS the Hoard editor
    // window, fed the pointer deltas back on themselves into an endless
    // oscillation). The adopted window is already raised `HWND_TOPMOST`, so it's
    // visible; to interact with it (pause a video) the user just clicks it and
    // Windows focuses it natively, its real ex-style is restored in Editor.
}

/// Clip a window to its crop with a window region (a clip, not a zoom). A NONE
/// crop clears any region we previously set.
unsafe fn apply_crop(hwnd: HWND, w: i32, h: i32, crop: Crop, m: &mut Managed) {
    let c = crop.sanitized();
    if c.left == 0.0 && c.right == 0.0 && c.top == 0.0 && c.bottom == 0.0 {
        if m.cropped {
            let _ = SetWindowRgn(hwnd, HRGN(std::ptr::null_mut()), TRUE);
            m.cropped = false;
        }
        return;
    }
    let l = (c.left * w as f64).round() as i32;
    let t = (c.top * h as f64).round() as i32;
    let r = ((1.0 - c.right) * w as f64).round() as i32;
    let b = ((1.0 - c.bottom) * h as f64).round() as i32;
    let rgn = CreateRectRgn(l.min(r), t.min(b), l.max(r), t.max(b));
    // SetWindowRgn takes ownership of the region on success, don't delete it.
    let _ = SetWindowRgn(hwnd, rgn, TRUE);
    m.cropped = true;
}

/// Restore a window to how we found it: drop the clip, restore ex-style, move it
/// back and out of the topmost band.
unsafe fn restore_window(hwnd: HWND, m: &Managed) {
    if m.cropped {
        let _ = SetWindowRgn(hwnd, HRGN(std::ptr::null_mut()), TRUE);
    }
    let _ = SetWindowLongW(hwnd, GWL_EXSTYLE, m.exstyle);
    // Put the chrome back (we may have stripped it in View), with FRAMECHANGED so
    // the title bar/border redraw.
    let _ = SetWindowLongW(hwnd, GWL_STYLE, m.style);
    // A window adopted while minimised (or that the app minimised while it was
    // parked) has to come back visible.
    if IsIconic(hwnd).as_bool() {
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    }
    let (x, y, w, h) = m.rect;
    let _ = SetWindowPos(
        hwnd,
        HWND_NOTOPMOST,
        x,
        y,
        w,
        h,
        SWP_NOACTIVATE | SWP_FRAMECHANGED,
    );
    // Guarantees visibility and a repaint: a window that was parked off-screen or
    // layered (Steam/CEF, Discord) could stay black or invisible when it was taken
    // out of the scene, forcing an app restart to get it back (P8/P11). Forcing a
    // show plus a redraw of the frame and its children brings it back with no
    // restart.
    let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    let _ = RedrawWindow(
        hwnd,
        None,
        HRGN(std::ptr::null_mut()),
        RDW_INVALIDATE | RDW_FRAME | RDW_ALLCHILDREN | RDW_UPDATENOW,
    );
}

/// Re-raise the managed windows to the top of the topmost band, in draw order
/// (so panel z is preserved), without moving or resizing them.
unsafe fn reassert_z(scene: &Scene, managed: &HashMap<isize, Managed>) {
    for p in scene.draw_order() {
        let Some(win) = panel_win(p) else { continue };
        // A parked (compat) window stays off-screen, never raise it topmost.
        if managed.get(&win).map_or(true, |m| m.parked) {
            continue;
        }
        let target = HWND(win as *mut core::ffi::c_void);
        let _ = SetWindowPos(
            target,
            HWND_TOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE,
        );
    }
}

unsafe fn set_mode(
    hotkey_hwnd: HWND,
    scene: &mut Scene,
    managed: &mut HashMap<isize, Managed>,
    mode: &mut Mode,
    mons: &[MonitorInfo],
    next: Mode,
) {
    if *mode == next {
        return;
    }
    *mode = next;
    if next == Mode::Editor {
        let _ = RegisterHotKey(hotkey_hwnd, HOTKEY_ESCAPE, MOD_NOREPEAT, VK_ESCAPE);
        apply_windows(scene, mons, managed, true);
    } else {
        let _ = UnregisterHotKey(hotkey_hwnd, HOTKEY_ESCAPE);
        // Read back where the user left each window, then re-apply click-through.
        readback_geometry(scene, mons, managed);
        apply_windows(scene, mons, managed, false);
    }
}

/// After an edit, fold each managed window's on-screen geometry back into its
/// panel rect (monitor-local), re-homing it onto whichever monitor it now sits
/// on, dragging a window to another screen reassigns its target. A mirror
/// ("all") panel keeps mirroring; its rect is measured against the primary.
unsafe fn readback_geometry(
    scene: &mut Scene,
    mons: &[MonitorInfo],
    managed: &HashMap<isize, Managed>,
) -> bool {
    let mut changed = false;
    for p in scene.panels.iter_mut() {
        let Some(win) = panel_win(p) else { continue };
        // Skip parked (compat) windows: their off-screen rect is not the panel.
        if managed.get(&win).map_or(true, |m| m.parked) {
            continue;
        }
        let target = HWND(win as *mut core::ffi::c_void);
        let Some((x, y, w, h)) = window_rect(target) else {
            continue;
        };
        let was_all = matches!(p.monitor, PanelTarget::All);
        let mon = if was_all {
            &mons[0]
        } else {
            monitor_at(mons, x, y)
        };
        if !was_all {
            p.monitor = PanelTarget::Monitor { id: mon.id };
        }
        let (nx, ny, nw, nh) = (
            (x - mon.x) as f64,
            (y - mon.y) as f64,
            w.max(1) as f64,
            h.max(1) as f64,
        );
        if p.rect.x != nx || p.rect.y != ny || p.rect.w != nw || p.rect.h != nh {
            changed = true;
        }
        p.rect.x = nx;
        p.rect.y = ny;
        p.rect.w = nw;
        p.rect.h = nh;
    }
    changed
}

unsafe fn window_rect(hwnd: HWND) -> Option<SavedRect> {
    let mut r = RECT::default();
    GetWindowRect(hwnd, &mut r).ok()?;
    Some((r.left, r.top, r.right - r.left, r.bottom - r.top))
}

fn panel_win(p: &Panel) -> Option<isize> {
    match &p.source {
        SourceRef::Window { id } => parse_hwnd_val(id),
        _ => None,
    }
}

fn parse_hwnd_val(id: &WindowId) -> Option<isize> {
    let s = id.trim();
    if let Some(hex) = s.strip_prefix("0x") {
        isize::from_str_radix(hex, 16).ok()
    } else {
        s.parse::<isize>().ok()
    }
}

extern "system" fn wndproc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    if msg == WM_NCHITTEST {
        // lParam is screen coords. Catch the click (HTCLIENT) over ANY compat slot
        // so the OS delivers the mouse to this overlay's queue and the run loop can
        // forward it: OFF → to the parked capture, ON → to the window physically
        // behind the panel. (HTTRANSPARENT does NOT route through a composition
        // overlay to another top-level window, clicks just get lost, so we must
        // catch and forward ourselves in both modes.) Everywhere else falls through
        // to the game (HTTRANSPARENT).
        let x = (l.0 & 0xffff) as i16 as i32;
        let y = ((l.0 >> 16) & 0xffff) as i16 as i32;
        let hit = SLOTS.with(|s| s.borrow().iter().any(|sl| slot_contains(sl, x, y)));
        return LRESULT(if hit {
            HTCLIENT as isize
        } else {
            HTTRANSPARENT as isize
        });
    }
    unsafe { DefWindowProcW(hwnd, msg, w, l) }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
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
