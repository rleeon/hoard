//! The main window: when it gets shown.
//!
//! The window is declared `"visible": false` in `tauri.conf.json`. Tauri would
//! create it visible the moment the Rust side finished building it, but the webview
//! still has to start its process and parse the bundle, so the user saw a white
//! rectangle (the webview's default background, not our `bg-zinc-950`) for that
//! whole gap. Born hidden, the window appears already drawn: the frontend is what
//! asks to show it, through [`ui_ready`], right after the first paint.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tauri::{AppHandle, Manager};

/// How long we wait for the frontend before showing the window ourselves.
///
/// With the window hidden by default, a frontend that never mounts (a throw in the
/// bootstrap, like the v1.2.1 i18n bug) no longer leaves a white window: it leaves
/// an **invisible** app, which from outside looks far too much like "it doesn't
/// start". This deadline guarantees there is always something on screen, even if it
/// is the broken page, which is what the user can report.
const FALLBACK_SHOW_AFTER: Duration = Duration::from_secs(8);

/// Decides whether the window should be shown on this start.
///
/// Starting silently (autostart with `--silent` plus `start_minimised`) is the only
/// legitimate reason to stay hidden: there the app lives in the tray until the user
/// opens it. It is resolved once in `setup()` because it depends on the process's
/// arguments, not on the UI's state.
#[derive(Debug, Default)]
pub struct StartHidden(AtomicBool);

impl StartHidden {
    pub fn set(&self, hidden: bool) {
        self.0.store(hidden, Ordering::Relaxed);
    }

    pub fn get(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// Muestra la ventana principal salvo que este arranque sea silencioso.
fn show_main(app: &AppHandle) {
    if app.state::<StartHidden>().get() {
        return;
    }
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        // Without this the window appears behind on some Linux compositors when the
        // start was slow: another window took the focus while we were still hidden.
        let _ = w.set_focus();
    }
}

/// The frontend has painted its first frame and the window can be shown.
///
/// Idempotent: `show()` on an already visible window does nothing, so it does not
/// matter if the fallback got there first.
#[tauri::command]
pub fn ui_ready(app: AppHandle) {
    show_main(&app);
}

/// Red de seguridad: si el frontend no ha llamado a [`ui_ready`] dentro de
/// [`FALLBACK_SHOW_AFTER`], mostramos la ventana igualmente.
pub fn spawn_fallback_show(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(FALLBACK_SHOW_AFTER).await;
        let already_visible = app
            .get_webview_window("main")
            .and_then(|w| w.is_visible().ok())
            .unwrap_or(false);
        if already_visible || app.state::<StartHidden>().get() {
            return;
        }
        tracing::warn!(
            "the frontend never signalled ui_ready; showing the window anyway \
             (the UI is probably broken)"
        );
        show_main(&app);
    });
}

/// Marca este arranque como silencioso: la ventana se queda oculta hasta que
/// el usuario la invoque desde la bandeja.
pub fn mark_start_hidden(app: &AppHandle, hidden: bool) {
    app.state::<StartHidden>().set(hidden);
}

#[cfg(test)]
mod tests {
    use super::StartHidden;

    #[test]
    fn start_hidden_defaults_to_showing_the_window() {
        assert!(!StartHidden::default().get());
    }

    #[test]
    fn start_hidden_round_trips() {
        let flag = StartHidden::default();
        flag.set(true);
        assert!(flag.get());
        flag.set(false);
        assert!(!flag.get());
    }
}
