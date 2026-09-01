//! The system tray: the always-visible "Hoard is alive" surface.
//!
//! We render a 32×32 monochrome icon with a small status dot in the
//! bottom-right corner. The dot is colour-coded so the user can glance at the
//! menu bar / system tray and immediately know what the agent is doing:
//!
//! - **idle** (zinc): agent is up, nothing in flight.
//! - **running** (sky): the user is playing a tracked game.
//! - **uploading** (amber): a backup is in progress.
//! - **ok** (emerald): a backup just succeeded; reverts to idle after a beat.
//! - **error** (red): the last backup failed.
//!
//! The base image is the bundled app icon (embedded at compile time), so the
//! tray always matches the current branding. The status dot is composited
//! onto it at runtime in the bottom-right quarter. On macOS the icon is
//! registered as a *template* (the OS renders its alpha as a monochrome
//! silhouette that adapts to light/dark menu bars), so the dot is skipped
//! there and the state lives in the tooltip instead.
//!
//! The tray menu carries: Show Hoard, Back up everything now, Open Settings,
//! Sign out, Quit. Left-click the icon to toggle the main window.

use std::sync::Mutex;

use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{image::Image, AppHandle, Emitter, Manager};

/// State the tray icon can display. A single enum keeps the colour table in
/// one place and out of the rendering helper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayState {
    Idle,
    Running,
    Uploading,
    Ok,
    Error,
    Offline,
}

impl TrayState {
    /// RGBA tuple for the status dot. Picked to match Tailwind's 400-weight
    /// palette so the desktop chrome and the in-app pills feel consistent.
    fn dot_rgba(self) -> [u8; 4] {
        match self {
            // zinc-500
            TrayState::Idle => [113, 113, 122, 255],
            // sky-400
            TrayState::Running => [56, 189, 248, 255],
            // amber-400
            TrayState::Uploading => [251, 191, 36, 255],
            // emerald-400
            TrayState::Ok => [52, 211, 153, 255],
            // red-400
            TrayState::Error => [248, 113, 113, 255],
            // zinc-700, duller than idle so it reads as "asleep"
            TrayState::Offline => [63, 63, 70, 255],
        }
    }

    /// Tooltip text shown on hover. Kept short, since most tray UIs truncate
    /// aggressively after about 50 chars.
    fn tooltip(self) -> &'static str {
        match self {
            TrayState::Idle => "Hoard — watching",
            TrayState::Running => "Hoard — game running",
            TrayState::Uploading => "Hoard — backing up",
            TrayState::Ok => "Hoard — backup complete",
            TrayState::Error => "Hoard — backup failed",
            TrayState::Offline => "Hoard — agent offline",
        }
    }
}

/// Slot the live `TrayIcon` lives in once the app has launched. We hold it
/// behind a Mutex so the agent forwarder can mutate the icon from any thread
/// when state changes; AppHandle is `Clone`, so we don't store it here.
#[derive(Default)]
pub struct TrayController {
    inner: Mutex<Option<tauri::tray::TrayIcon>>,
}

impl TrayController {
    /// Replace the tray icon's appearance to reflect the given state.
    /// Cheap (a few hundred bytes of RGBA), so safe to call on every event.
    pub fn set_state(&self, state: TrayState) {
        let Some(tray) = self.inner.lock().unwrap().clone() else {
            // No tray yet (we're still inside `setup`, or running on a
            // platform that didn't grant us one). Drop the update silently:
            // the next `set_state` will reflect the latest value.
            return;
        };
        // macOS renders the icon as a fixed template silhouette, so pushing a
        // re-rendered bitmap is pointless (and resets the template flag on
        // some Tauri versions), so only the tooltip changes there.
        if !cfg!(target_os = "macos") {
            match render_icon(state) {
                Ok(img) => {
                    if let Err(e) = tray.set_icon(Some(img)) {
                        tracing::warn!(error = %e, "couldn't update tray icon");
                    }
                }
                Err(e) => tracing::warn!(error = %e, "tray icon render failed"),
            }
        }
        if let Err(e) = tray.set_tooltip(Some(state.tooltip())) {
            tracing::warn!(error = %e, "couldn't update tray tooltip");
        }
    }
}

/// Build the tray once on startup. The returned icon is stored on
/// `TrayController` so later state updates can mutate it. We register click
/// and menu handlers here too; everything in those closures is just a Tauri
/// event emit, with no business logic, so the rest of the app can subscribe.
pub fn install(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "tray::show", "Show Hoard", true, None::<&str>)?;
    let backup_all = MenuItem::with_id(
        app,
        "tray::backup-all",
        "Back up everything now",
        true,
        None::<&str>,
    )?;
    let settings = MenuItem::with_id(app, "tray::settings", "Settings…", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let sign_out = MenuItem::with_id(app, "tray::sign-out", "Sign out", true, None::<&str>)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "tray::quit", "Quit Hoard", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &show,
            &backup_all,
            &settings,
            &sep1,
            &sign_out,
            &sep2,
            &quit,
        ],
    )?;

    let icon = render_icon(TrayState::Offline)?;
    #[allow(unused_mut)]
    let mut builder = TrayIconBuilder::with_id("hoard-tray")
        .icon(icon)
        .tooltip(TrayState::Offline.tooltip())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(handle_menu_event)
        .on_tray_icon_event(handle_icon_event);
    // macOS: register as a template so the menu bar renders the icon's alpha
    // as a monochrome silhouette matching light/dark mode (Apple HIG); the
    // status dot is skipped there (see `render_icon`).
    #[cfg(target_os = "macos")]
    {
        builder = builder.icon_as_template(true);
    }
    let tray = builder.build(app)?;

    // Stash the handle so other modules can recolour later.
    let controller = app.state::<TrayController>();
    *controller.inner.lock().unwrap() = Some(tray);
    Ok(())
}

/// Map a menu click to a Tauri event the rest of the app can listen for.
/// Routing through events keeps Rust-side logic out of this file (we don't
/// want a tray module reaching into auth or agent state directly).
fn handle_menu_event(app: &AppHandle, event: MenuEvent) {
    match event.id().as_ref() {
        "tray::show" => show_main_window(app),
        "tray::backup-all" => {
            // The frontend listens for this and calls `backup_now` per save.
            // Doing it that way keeps the per-save iteration logic, and the
            // success and failure toasts, in one place (the dashboard).
            let _ = app.emit("tray://backup-all", ());
            show_main_window(app);
        }
        "tray::settings" => {
            // Tell the frontend to navigate. We could push a deep-link URL,
            // but emitting an event keeps the Svelte router as the single
            // source of routing truth.
            let _ = app.emit("tray://open-settings", ());
            show_main_window(app);
        }
        "tray::sign-out" => {
            // The frontend confirms and calls `signOut()` from auth.ts.
            let _ = app.emit("tray://sign-out", ());
            show_main_window(app);
        }
        "tray::quit" => {
            // Properly tear down rather than just app.exit so the agent gets
            // a chance to flush. The `WindowEvent::CloseRequested` path
            // we install elsewhere doesn't help us here: that's only for
            // window closes, not explicit quits.
            tracing::info!("user requested quit from tray");
            app.exit(0);
        }
        other => tracing::debug!(item = other, "unhandled tray menu id"),
    }
}

/// Left-click toggles visibility, matching what users reflexively expect.
/// Right-click opens the menu (handled by the runtime).
fn handle_icon_event(tray: &tauri::tray::TrayIcon, event: TrayIconEvent) {
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
    } = event
    {
        toggle_main_window(tray.app_handle());
    }
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn toggle_main_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    match window.is_visible() {
        Ok(true) => {
            // Already visible: clicking the tray on a visible window is the
            // user telling us they're done. Hide it; the agent keeps running.
            let _ = window.hide();
        }
        _ => {
            let _ = window.unminimize();
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

/// The bundled app icon, embedded at compile time. 64×64 (not 32) so HiDPI
/// trays don't upscale a soft bitmap; low-DPI trays downscale it cleanly.
/// Same asset family the window/taskbar uses, so a rebrand of `icons/`
/// updates the tray with no code change.
const ICON_PNG: &[u8] = include_bytes!("../icons/64x64.png");

/// Compose the tray icon: the app icon with a status-coloured dot in the
/// lower-right (except on macOS: see the module docs, where the template
/// silhouette would flatten any colour, so the base icon is returned untouched and
/// the tooltip carries the state).
fn render_icon(state: TrayState) -> tauri::Result<Image<'static>> {
    let base = Image::from_bytes(ICON_PNG)?;
    let (w, h) = (base.width(), base.height());
    let mut buf = base.rgba().to_vec();

    if !cfg!(target_os = "macos") {
        // Geometry scales off the icon size, keeping the proportions of the
        // old 32px design: dot radius = w/8, centred 2r from the right/bottom
        // edges, with a 1-ish px dark ring so it reads on any artwork.
        let dot = state.dot_rgba();
        let ring_rgba = [24u8, 24, 27, 255];
        let r = (w / 8) as i32;
        let ring = r + (w / 32).max(1) as i32;
        let cx = w as i32 - 2 * r;
        let cy = h as i32 - 2 * r;
        for y in (cy - ring)..=(cy + ring) {
            for x in (cx - ring)..=(cx + ring) {
                let dx = x - cx;
                let dy = y - cy;
                let d2 = dx * dx + dy * dy;
                if d2 <= r * r {
                    put_pixel(&mut buf, w, h, x, y, dot);
                } else if d2 <= ring * ring {
                    put_pixel(&mut buf, w, h, x, y, ring_rgba);
                }
            }
        }
    }

    Ok(Image::new_owned(buf, w, h))
}

#[inline]
fn put_pixel(buf: &mut [u8], w: u32, h: u32, x: i32, y: i32, rgba: [u8; 4]) {
    if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
        return;
    }
    let i = ((y as u32 * w + x as u32) * 4) as usize;
    buf[i..i + 4].copy_from_slice(&rgba);
}
