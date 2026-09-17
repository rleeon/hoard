//! The HUD over the game: a second, nearly transparent window carrying the sync
//! service's live log.
//!
//! It is **the normal app**, not Hoard-Screen. Hoard-Screen is a separate process
//! that composes native panels; this is one more Tauri window, with the same
//! frontend bundle, told apart by its label (`OVERLAY_LABEL`). The frontend looks at
//! that label on start and mounts the HUD instead of the whole application (see
//! `main.ts`).
//!
//! Why it is created here and not from JS: the window has to be born undecorated,
//! transparent, always on top and out of the taskbar, and those are construction-time
//! properties. Creating it from the webview would also mean widening the capabilities
//! to allow arbitrary window creation, which is exactly what should stay shut.
//!
//! **Ordering against Hoard-Screen**: the Pro overlay is an independent process that
//! also puts itself always on top, and between two "always on top" windows the
//! compositor's activation order decides, so this window is shown **without stealing
//! focus** (`focused = false`) so it does not jump over it.

use std::sync::atomic::Ordering;

use tauri::utils::config::Color;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::commands::window::{background, foreground, WindowLife};

/// The window's label. The frontend compares against it to decide what to mount, so
/// changing it here means changing it in `main.ts`.
pub const OVERLAY_LABEL: &str = "overlay";

/// Creates the window when it does not exist. It is born hidden, and
/// [`overlay_set_visible`] is what shows it, so the first press of the shortcut does
/// not show a white rectangle while the webview starts.
fn ensure(app: &AppHandle) -> Result<tauri::WebviewWindow, String> {
    if let Some(w) = app.get_webview_window(OVERLAY_LABEL) {
        return Ok(w);
    }
    WebviewWindowBuilder::new(app, OVERLAY_LABEL, WebviewUrl::default())
        .title("Hoard")
        .transparent(true)
        // A fully transparent window background. Without it the window keeps the
        // system's (opaque white on most platforms) and **shows it through any pixel
        // the HUD does not cover**: typically a light one- or two-pixel line along an
        // edge. The main window sets its own in `tauri.conf.json`; this one was
        // created with none.
        .background_color(Color(0, 0, 0, 0))
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .maximized(true)
        .visible(false)
        // No shadow: on Windows an undecorated window's shadow is drawn anyway and
        // leaves a grey halo over the game.
        .shadow(false)
        .build()
        .map_err(|e| format!("no se pudo crear el overlay: {e}"))
}

/// Muestra u oculta el HUD. Devuelve el estado en que queda.
#[tauri::command]
pub async fn overlay_set_visible(app: AppHandle, visible: bool) -> Result<bool, String> {
    if !visible {
        // Hiding a HUD that doesn't exist is nothing to do. Building it just to hide
        // it put a second webview in memory on every start with the HUD turned off.
        if let Some(w) = app.get_webview_window(OVERLAY_LABEL) {
            let _ = w.hide();
            background(&app, &w, OVERLAY_LABEL);
        }
        return Ok(false);
    }
    let w = ensure(&app)?;
    foreground(&app, &w, OVERLAY_LABEL);
    let _ = w.show();
    // Focus is asked for on purpose: the HUD has a close button and responds to
    // Escape, so it needs the keyboard. It is what the Steam overlay does too.
    let _ = w.set_focus();
    Ok(true)
}

/// Sets the shortcut that toggles the HUD, `None` for none. Registered from here
/// and not from the page: the page is the main window's, and on Linux that window
/// is dropped while hidden, which is exactly when the shortcut is needed (the game
/// is in front).
#[tauri::command]
pub fn overlay_bind(app: AppHandle, accel: Option<String>) -> Result<(), String> {
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
    let life = app.state::<WindowLife>();
    let mut current = life.overlay_accel.lock().unwrap();
    if let Some(old) = current.take() {
        let _ = app.global_shortcut().unregister(old.as_str());
    }
    let Some(accel) = accel else {
        return Ok(());
    };
    app.global_shortcut()
        .on_shortcut(accel.as_str(), |app, _, event| {
            if event.state == ShortcutState::Pressed {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = overlay_toggle(app).await;
                });
            }
        })
        .map_err(|e| e.to_string())?;
    *current = Some(accel);
    Ok(())
}

/// A game just started: have the HUD loaded and waiting, so the shortcut shows it
/// at once instead of starting a webview in front of the game. Only when there is
/// a shortcut to call it with.
pub fn game_started(app: &AppHandle) {
    let life = app.state::<WindowLife>();
    life.games.fetch_add(1, Ordering::SeqCst);
    if life.overlay_accel.lock().unwrap().is_none()
        || app.get_webview_window(OVERLAY_LABEL).is_some()
    {
        return;
    }
    match ensure(app) {
        Ok(w) => background(app, &w, OVERLAY_LABEL),
        Err(e) => tracing::warn!(error = %e, "overlay: couldn't preload the HUD"),
    }
}

/// A game ended. With none left, a hidden HUD may go once it has been hidden a while.
pub fn game_stopped(app: &AppHandle) {
    let life = app.state::<WindowLife>();
    let _ = life
        .games
        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| {
            Some(n.saturating_sub(1))
        });
    if life.games.load(Ordering::SeqCst) > 0 {
        return;
    }
    if let Some(w) = app.get_webview_window(OVERLAY_LABEL) {
        if !w.is_visible().unwrap_or(true) {
            background(app, &w, OVERLAY_LABEL);
        }
    }
}

/// Alterna el HUD. Es lo que llama el atajo global.
#[tauri::command]
pub async fn overlay_toggle(app: AppHandle) -> Result<bool, String> {
    let w = ensure(&app)?;
    let showing = w.is_visible().unwrap_or(false);
    overlay_set_visible(app, !showing).await
}

/// Is the HUD on screen? The Settings page uses it to paint its state.
#[tauri::command]
pub async fn overlay_is_visible(app: AppHandle) -> Result<bool, String> {
    Ok(app
        .get_webview_window(OVERLAY_LABEL)
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false))
}
