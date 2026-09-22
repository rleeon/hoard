//! Who draws the window's title bar.
//!
//! Windows and most Linux desktops paint a light caption with square corners,
//! which on a near-black app reads as a strip of tape across the top, so the
//! frontend paints its own instead (`Titlebar.svelte`) and the decoration comes
//! off entirely, on both. The frame goes with it, so the bar paints its own edge
//! and carries the invisible resize grips.
//!
//! On Linux, replacing GTK's title bar with an empty widget was tried first,
//! because GTK then keeps drawing its frame, shadow and rounded corners. It
//! cannot work: the webview is an opaque rectangle painted over the whole client
//! area, so it covers the rounded corners and the frame's border, and nothing of
//! what the trick buys is ever visible. What it does bring is a window that is
//! still decorated, and so gets no resize border from tao, whose `hit_test` runs
//! only on undecorated ones.
//!
//! Every answer this module gives is a fact, never an intention: if taking the
//! system bar off fails, [`Titlebar::own`] comes back `false`, the frontend
//! mounts nothing and the user keeps the bar the system drew. That is the half of
//! the trade that always works, so no caller has to handle an error.
//!
//! The decision is made once, before the window is first shown, and the user can
//! force the system bar from Settings (`Prefs::system_titlebar`) for the case we
//! cannot test: a desktop that renders an undecorated window badly.

use std::sync::Mutex;

use hoard_agent::prefs::Prefs;
use serde::Serialize;
use tauri::{AppHandle, Manager, WebviewWindow};

/// Env override, for trying the system bar without touching the prefs file.
const SYSTEM_BAR_ENV: &str = "HOARD_SYSTEM_TITLEBAR";

/// What the frontend needs to know, as it ended up rather than as it was asked.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Titlebar {
    /// The window has no system bar and the frontend is the one that paints it.
    pub own: bool,
    /// Whether Settings shows the switch. False where the choice isn't the
    /// user's to make: macOS has no bar of ours, and an override already decided.
    pub toggleable: bool,
}

/// The last word on this window, so the frontend and Settings read what really
/// happened instead of asking the window (with GTK the window still calls itself
/// decorated: what changed is what its decoration contains).
#[derive(Debug)]
pub struct TitlebarState(Mutex<Titlebar>);

impl Default for TitlebarState {
    fn default() -> Self {
        // Until `apply` runs, the honest answer is the system's: that is what is
        // on screen.
        Self(Mutex::new(Titlebar {
            own: false,
            toggleable: false,
        }))
    }
}

impl TitlebarState {
    pub fn get(&self) -> Titlebar {
        *self.0.lock().unwrap()
    }

    fn set(&self, titlebar: Titlebar) {
        *self.0.lock().unwrap() = titlebar;
    }
}

#[tauri::command]
pub fn window_titlebar(app: AppHandle) -> Titlebar {
    app.state::<TitlebarState>().get()
}

/// Takes the system title bar off `window` unless something says otherwise, and
/// records what came of it for [`window_titlebar`].
///
/// Called at startup (`lib.rs`) and again by `rebuild_main`, the two places a main
/// window comes into being.
pub fn apply(app: &AppHandle, window: &WebviewWindow) -> Titlebar {
    let forced = forced_system();
    let prefer_system = Prefs::load_default()
        .map(|(prefs, _)| prefs.system_titlebar)
        .unwrap_or(false);
    let mut titlebar = plan(prefer_system, forced.is_some());
    if titlebar.own {
        titlebar.own = take_system_bar(window);
    }
    if let Some(reason) = forced {
        tracing::info!(reason, "window: keeping the system title bar");
    }
    // Not about the bar, but about the same window at the same moment, and this
    // is the one place both of its births already pass through.
    #[cfg(target_os = "linux")]
    paint_resize_gap(window);
    app.state::<TitlebarState>().set(titlebar);
    titlebar
}

/// Paints the window and its webview the app's own black.
///
/// WebKitGTK renders in a process of its own, so a window being dragged wider
/// grows first and gets its new pixels up to half a second later. Whatever GTK
/// paints in the meantime is what the user sees walking out from the edge, and by
/// default that is the theme's grey: a band of it that trails the pointer and
/// catches up in steps. The gap cannot be removed (it is where the frames are
/// not yet), but painted `#09090b` it stops being something to look at.
///
/// Windows needs none of this: WebView2 has the window's `backgroundColor` from
/// `tauri.conf.json` and resizes in step.
#[cfg(target_os = "linux")]
fn paint_resize_gap(window: &WebviewWindow) {
    // The same colour as the window's `backgroundColor`; one call covers the
    // window layer and the webview's.
    if let Err(e) = window.set_background_color(Some(tauri::window::Color(9, 9, 11, 255))) {
        tracing::warn!(error = %e, "window: couldn't paint the background");
    }
}

/// Who *should* draw the bar. Pure, so the table of cases is testable; whether
/// the platform then delivers is [`take_system_bar`]'s business.
fn plan(prefer_system: bool, forced: bool) -> Titlebar {
    if cfg!(target_os = "macos") {
        // macOS integrates its own well enough that replacing it would cost more
        // than it buys, and the traffic lights are muscle memory.
        return Titlebar {
            own: false,
            toggleable: false,
        };
    }
    Titlebar {
        own: !forced && !prefer_system,
        toggleable: !forced,
    }
}

/// Reasons to leave the system bar alone whatever the user picked.
fn forced_system() -> Option<&'static str> {
    if env_says_system(std::env::var(SYSTEM_BAR_ENV).ok().as_deref()) {
        return Some(SYSTEM_BAR_ENV);
    }
    // Under gamescope (the Deck's game mode) there is no desktop to drag a window
    // around, and a close button on a bar of ours would be the only way out of a
    // session that isn't ours to end.
    if cfg!(target_os = "linux")
        && (std::env::var_os("GAMESCOPE_WAYLAND_DISPLAY").is_some()
            || std::env::var("XDG_CURRENT_DESKTOP")
                .map(|d| d.to_ascii_lowercase().contains("gamescope"))
                .unwrap_or(false))
    {
        return Some("gamescope");
    }
    None
}

/// `HOARD_SYSTEM_TITLEBAR=0` has to mean "no", or the variable could only ever be
/// turned on once per session.
fn env_says_system(value: Option<&str>) -> bool {
    matches!(
        value.map(|v| v.trim().to_ascii_lowercase()).as_deref(),
        Some("1") | Some("true") | Some("yes") | Some("on")
    )
}

#[cfg(any(windows, target_os = "linux"))]
fn take_system_bar(window: &WebviewWindow) -> bool {
    match window.set_decorations(false) {
        Ok(()) => true,
        Err(e) => {
            tracing::warn!(error = %e, "window: couldn't drop the system title bar");
            false
        }
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
fn take_system_bar(_window: &WebviewWindow) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_switch_decides_when_nothing_overrides_it() {
        let ours = plan(false, false);
        let theirs = plan(true, false);
        if cfg!(target_os = "macos") {
            assert!(!ours.own);
            assert!(!ours.toggleable);
        } else {
            assert!(ours.own);
            assert!(ours.toggleable);
            assert!(!theirs.own);
            assert!(theirs.toggleable);
        }
    }

    #[test]
    fn an_override_takes_the_choice_away() {
        let forced = plan(false, true);
        assert!(!forced.own);
        assert!(
            !forced.toggleable,
            "a switch that changes nothing is worse than none"
        );
    }

    #[test]
    fn the_env_var_reads_both_ways() {
        assert!(env_says_system(Some("1")));
        assert!(env_says_system(Some("True")));
        assert!(env_says_system(Some(" yes ")));
        assert!(!env_says_system(Some("0")));
        assert!(!env_says_system(Some("")));
        assert!(!env_says_system(None));
    }

    #[test]
    fn the_default_answer_is_the_system_bar() {
        let state = TitlebarState::default();
        assert!(!state.get().own);
    }
}
