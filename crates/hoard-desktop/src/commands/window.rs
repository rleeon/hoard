//! The main window: when it gets shown, and what it costs while it isn't.
//!
//! The window is declared `"visible": false` in `tauri.conf.json`. Tauri would
//! create it visible the moment the Rust side finished building it, but the webview
//! still has to start its process and parse the bundle, so the user saw a white
//! rectangle (the webview's default background, not our `bg-zinc-950`) for that
//! whole gap. Born hidden, the window appears already drawn: the frontend is what
//! asks to show it, through [`ui_ready`], right after the first paint.
//!
//! Hidden in the tray is where the app spends most of its life, and a hidden
//! webview costs what a visible one does: 201 MB on WebView2 and 194 MB on
//! WebKitGTK for the same page, measured on 13-09. What it can give back depends
//! on the engine:
//!
//! - **WebView2** has a knob for exactly this. Asked for a low memory target while
//!   hidden it came down to 60 MB and was back on screen in 5 ms with everything in
//!   place, so on Windows a window is only ever hidden.
//! - **WebKitGTK** has nothing like it: the only way to get the memory back is to
//!   drop the webview (7 MB stay, the network process). So on Linux a window hidden
//!   for [`RELEASE_AFTER`] is destroyed, and rebuilt the next time it is asked for,
//!   which is a cold start (~0.6 s) instead of 5 ms. What the screens paint from
//!   lives on this side (the session, the service's journal, the notifications), so
//!   the rebuilt UI comes up with it and lands back on the page the user left.
//!
//! macOS keeps the old behaviour: neither path has been tried there.

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

use crate::commands::overlay::OVERLAY_LABEL;

#[cfg(windows)]
use webview2_com::Microsoft::Web::WebView2::Win32::{
    ICoreWebView2ProcessFailedEventArgs, COREWEBVIEW2_PROCESS_FAILED_KIND,
    COREWEBVIEW2_PROCESS_FAILED_REASON,
};

pub const MAIN_LABEL: &str = "main";

/// How long we wait for the frontend before showing the window ourselves.
///
/// With the window hidden by default, a frontend that never mounts (a throw in the
/// bootstrap, like the v1.2.1 i18n bug) no longer leaves a white window: it leaves
/// an **invisible** app, which from outside looks far too much like "it doesn't
/// start". This deadline guarantees there is always something on screen, even if it
/// is the broken page, which is what the user can report.
const FALLBACK_SHOW_AFTER: Duration = Duration::from_secs(8);

/// How long a hidden window keeps its webview on Linux. Long enough that closing
/// the window to glance at a game and opening it again never pays a rebuild.
#[cfg(target_os = "linux")]
const RELEASE_AFTER: Duration = Duration::from_secs(10 * 60);

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

/// What outlives the windows themselves.
#[derive(Debug, Default)]
pub struct WindowLife {
    /// Bumped whenever a window is shown or hidden: a release armed by an older
    /// hide finds a different number and leaves the window alone.
    main_epoch: AtomicU64,
    overlay_epoch: AtomicU64,
    /// Set when the main window is rebuilt. The UI reads it once, through
    /// [`ui_ready`], to land back where the user was instead of on the start page.
    reopened: AtomicBool,
    /// Set just before this module drops a window, so the "last window closed"
    /// that follows is not taken for the user quitting.
    releasing: AtomicBool,
    /// Games running right now, per the service. The HUD is kept while any is.
    pub(crate) games: AtomicUsize,
    /// The HUD's shortcut, when one is registered (`overlay_bind`).
    pub(crate) overlay_accel: Mutex<Option<String>>,
    /// A tray action for a UI that had to be rebuilt first, and so had no listener
    /// yet when it was sent.
    pending_intent: Mutex<Option<String>>,
}

impl WindowLife {
    fn epoch(&self, label: &str) -> &AtomicU64 {
        if label == OVERLAY_LABEL {
            &self.overlay_epoch
        } else {
            &self.main_epoch
        }
    }

    fn bump(&self, label: &str) -> u64 {
        self.epoch(label).fetch_add(1, Ordering::SeqCst) + 1
    }
}

/// Muestra la ventana principal salvo que este arranque sea silencioso.
fn show_main(app: &AppHandle) {
    if app.state::<StartHidden>().get() {
        return;
    }
    if let Some(w) = app.get_webview_window(MAIN_LABEL) {
        let _ = w.show();
        // Without this the window appears behind on some Linux compositors when the
        // start was slow: another window took the focus while we were still hidden.
        let _ = w.set_focus();
    }
}

/// The frontend has painted its first frame and the window can be shown.
///
/// Idempotent: `show()` on an already visible window does nothing, so it does not
/// matter if the fallback got there first. Answers whether this window was rebuilt
/// after being released, so the UI knows to go back to the page it was on.
#[tauri::command]
pub fn ui_ready(app: AppHandle) -> bool {
    show_main(&app);
    app.state::<WindowLife>().reopened.swap(false, Ordering::SeqCst)
}

/// Red de seguridad: si el frontend no ha llamado a [`ui_ready`] dentro de
/// [`FALLBACK_SHOW_AFTER`], mostramos la ventana igualmente.
pub fn spawn_fallback_show(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(FALLBACK_SHOW_AFTER).await;
        let already_visible = app
            .get_webview_window(MAIN_LABEL)
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
/// el usuario la invoque desde la bandeja. A window that starts in the tray is a
/// hidden window from its first second, and gets treated as one.
pub fn mark_start_hidden(app: &AppHandle, hidden: bool) {
    app.state::<StartHidden>().set(hidden);
    if hidden {
        if let Some(window) = app.get_webview_window(MAIN_LABEL) {
            background(app, &window, MAIN_LABEL);
        }
    }
}

/// Shows the main window, rebuilding it first if it was released while hidden.
/// Every way of bringing the window back goes through here: the tray, a second
/// launch, a deep link, the login loopback.
pub fn reveal_main(app: &AppHandle) {
    // Whoever asks for the window wants it, silent start or not.
    app.state::<StartHidden>().set(false);
    let window = match app.get_webview_window(MAIN_LABEL) {
        Some(w) => w,
        None => match rebuild_main(app) {
            Ok(w) => w,
            Err(e) => {
                tracing::error!(error = %e, "window: couldn't rebuild the main window");
                return;
            }
        },
    };
    foreground(app, &window, MAIN_LABEL);
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
}

/// Hides the main window and lets its webview give back what it can.
pub fn stash_main(app: &AppHandle) {
    let Some(window) = app.get_webview_window(MAIN_LABEL) else {
        return;
    };
    let _ = window.hide();
    background(app, &window, MAIN_LABEL);
}

/// A tray action for the main UI (`tray://<intent>`), bringing the window up. When
/// the window had to be rebuilt the UI isn't listening yet, so the action waits in
/// [`window_take_intent`] instead of being emitted into nothing.
pub fn send_intent(app: &AppHandle, intent: &str) {
    let rebuilding = app.get_webview_window(MAIN_LABEL).is_none();
    reveal_main(app);
    if rebuilding {
        *app.state::<WindowLife>().pending_intent.lock().unwrap() = Some(intent.to_string());
    } else {
        let _ = app.emit(&format!("tray://{intent}"), ());
    }
}

#[tauri::command]
pub fn window_take_intent(app: AppHandle) -> Option<String> {
    app.state::<WindowLife>().pending_intent.lock().unwrap().take()
}

/// `true` when the app must keep running although its last window just closed:
/// this module dropped it to save memory, the user didn't close anything.
pub fn keep_running_after_last_window(app: &AppHandle) -> bool {
    app.state::<WindowLife>().releasing.swap(false, Ordering::SeqCst)
}

/// A window has come back on screen, or is about to.
pub(crate) fn foreground(app: &AppHandle, window: &WebviewWindow, label: &str) {
    app.state::<WindowLife>().bump(label);
    set_backgrounded(window, false);
}

/// A window has just been hidden: what it does with its memory, per engine (see
/// the module docs).
pub(crate) fn background(app: &AppHandle, window: &WebviewWindow, label: &'static str) {
    let _epoch = app.state::<WindowLife>().bump(label);
    set_backgrounded(window, true);
    #[cfg(target_os = "linux")]
    release_later(app.clone(), label, _epoch);
}

/// Builds the main window again from its `tauri.conf.json` entry, the one it was
/// born from at startup.
fn rebuild_main(app: &AppHandle) -> Result<WebviewWindow, String> {
    let config = app
        .config()
        .app
        .windows
        .iter()
        .find(|w| w.label == MAIN_LABEL)
        .cloned()
        .ok_or("no main window in tauri.conf.json")?;
    let window = tauri::WebviewWindowBuilder::from_config(app, &config)
        .and_then(|b| b.build())
        .map_err(|e| e.to_string())?;
    // As at startup (`lib.rs`): on Windows the frontend paints its own title bar.
    #[cfg(windows)]
    let _ = window.set_decorations(false);
    #[cfg(windows)]
    watch_engine(&window);
    app.state::<WindowLife>().reopened.store(true, Ordering::SeqCst);
    tracing::info!("window: main window rebuilt");
    Ok(window)
}

#[cfg(target_os = "linux")]
fn release_after() -> Duration {
    // For trying the release by hand without waiting ten minutes.
    std::env::var("HOARD_RELEASE_HIDDEN_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .map(Duration::from_secs)
        .unwrap_or(RELEASE_AFTER)
}

#[cfg(target_os = "linux")]
fn release_later(app: AppHandle, label: &'static str, epoch: u64) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(release_after()).await;
        let life = app.state::<WindowLife>();
        if life.epoch(label).load(Ordering::SeqCst) != epoch {
            return;
        }
        // The HUD stays while a game is on, so the next Alt+H is instant;
        // `overlay::game_stopped` arms this again when the last one ends.
        if label == OVERLAY_LABEL && life.games.load(Ordering::SeqCst) > 0 {
            return;
        }
        let Some(window) = app.get_webview_window(label) else {
            return;
        };
        if window.is_visible().unwrap_or(true) {
            return;
        }
        tracing::info!(label, "window: dropping a webview that has been hidden for a while");
        life.releasing.store(true, Ordering::SeqCst);
        if window.destroy().is_err() {
            life.releasing.store(false, Ordering::SeqCst);
        }
    });
}

#[cfg(windows)]
fn set_backgrounded(window: &WebviewWindow, background: bool) {
    let _ = window.with_webview(move |webview| unsafe {
        use webview2_com::Microsoft::Web::WebView2::Win32::{
            ICoreWebView2_19, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW,
            COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL,
        };
        use windows_core::Interface;
        let controller = webview.controller();
        let _ = controller.SetIsVisible(!background);
        let level = if background {
            COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW
        } else {
            COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL
        };
        // A runtime too old for `_19` doesn't know the knob and keeps its memory.
        if let Ok(core) = controller
            .CoreWebView2()
            .and_then(|c| c.cast::<ICoreWebView2_19>())
        {
            let _ = core.SetMemoryUsageTargetLevel(level);
        }
    });
}

#[cfg(not(windows))]
fn set_backgrounded(_window: &WebviewWindow, _background: bool) {}

/// Logs every WebView2 process that dies under `window`, with what the engine says
/// about it, and one line when the engine comes up.
///
/// On 14-09 the browser process went two seconds after a start and the window stayed
/// black until the user quit. All the log had was a `0x8007139F` for each emit that
/// followed, and Crashpad had already uploaded its dump and deleted it: there was no
/// way left to tell which process died, or why.
#[cfg(windows)]
pub(crate) fn watch_engine(window: &WebviewWindow) {
    let label = window.label().to_string();
    let watched = window.with_webview(move |webview| unsafe {
        let core = match webview.controller().CoreWebView2() {
            Ok(core) => core,
            Err(e) => {
                tracing::warn!(window = %label, error = %e, "webview2: no engine to watch");
                return;
            }
        };
        let mut browser_pid = 0u32;
        let _ = core.BrowserProcessId(&mut browser_pid);
        let mut version = windows_core::PWSTR::null();
        let version = match webview.environment().BrowserVersionString(&mut version) {
            Ok(()) => webview2_com::take_pwstr(version),
            Err(_) => String::new(),
        };
        tracing::info!(window = %label, browser_pid, %version, "webview2: engine up");

        let failed = label.clone();
        let handler = webview2_com::ProcessFailedEventHandler::create(Box::new(move |_, args| {
            if let Some(args) = args {
                log_process_failed(&failed, &args);
            }
            Ok(())
        }));
        let mut token = 0i64;
        if let Err(e) = core.add_ProcessFailed(&handler, &mut token) {
            tracing::warn!(
                window = %label, error = %e,
                "webview2: couldn't watch the engine's processes"
            );
        }
    });
    if let Err(e) = watched {
        tracing::warn!(error = %e, "webview2: couldn't reach the webview to watch it");
    }
}

#[cfg(windows)]
fn log_process_failed(label: &str, args: &ICoreWebView2ProcessFailedEventArgs) {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        ICoreWebView2ProcessFailedEventArgs2, ICoreWebView2ProcessFailedEventArgs3,
        COREWEBVIEW2_PROCESS_FAILED_KIND_BROWSER_PROCESS_EXITED,
        COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_EXITED,
    };
    use windows_core::Interface;

    let mut kind = COREWEBVIEW2_PROCESS_FAILED_KIND::default();
    // Reason, exit code and module came with later runtimes; an old one only gives
    // the kind, and -1 keeps it from reading as a real reason.
    let mut reason = COREWEBVIEW2_PROCESS_FAILED_REASON(-1);
    let mut exit_code = 0i32;
    let mut process = String::new();
    let mut module = String::new();
    unsafe {
        let _ = args.ProcessFailedKind(&mut kind);
        if let Ok(args) = args.cast::<ICoreWebView2ProcessFailedEventArgs2>() {
            let _ = args.Reason(&mut reason);
            let _ = args.ExitCode(&mut exit_code);
            let mut text = windows_core::PWSTR::null();
            if args.ProcessDescription(&mut text).is_ok() {
                process = webview2_com::take_pwstr(text);
            }
        }
        if let Ok(args) = args.cast::<ICoreWebView2ProcessFailedEventArgs3>() {
            let mut text = windows_core::PWSTR::null();
            if args.FailureSourceModulePath(&mut text).is_ok() {
                module = webview2_com::take_pwstr(text);
            }
        }
    }
    // The browser and the page's renderer take the window down with them; GPU,
    // utility and the rest the engine restarts on its own.
    let takes_the_window = kind == COREWEBVIEW2_PROCESS_FAILED_KIND_BROWSER_PROCESS_EXITED
        || kind == COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_EXITED;
    let exit_code = format!("0x{:08X}", exit_code as u32);
    let (kind, reason) = (kind_name(kind), reason_name(reason));
    if takes_the_window {
        tracing::error!(
            window = label, kind, reason, %exit_code, %process, %module,
            "webview2: an engine process died"
        );
    } else {
        tracing::warn!(
            window = label, kind, reason, %exit_code, %process, %module,
            "webview2: an engine process died"
        );
    }
}

#[cfg(windows)]
fn kind_name(kind: COREWEBVIEW2_PROCESS_FAILED_KIND) -> &'static str {
    use webview2_com::Microsoft::Web::WebView2::Win32::*;
    match kind {
        COREWEBVIEW2_PROCESS_FAILED_KIND_BROWSER_PROCESS_EXITED => "browser",
        COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_EXITED => "renderer",
        COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_UNRESPONSIVE => "renderer unresponsive",
        COREWEBVIEW2_PROCESS_FAILED_KIND_FRAME_RENDER_PROCESS_EXITED => "frame renderer",
        COREWEBVIEW2_PROCESS_FAILED_KIND_GPU_PROCESS_EXITED => "gpu",
        COREWEBVIEW2_PROCESS_FAILED_KIND_UTILITY_PROCESS_EXITED => "utility",
        COREWEBVIEW2_PROCESS_FAILED_KIND_SANDBOX_HELPER_PROCESS_EXITED => "sandbox helper",
        _ => "other",
    }
}

#[cfg(windows)]
fn reason_name(reason: COREWEBVIEW2_PROCESS_FAILED_REASON) -> &'static str {
    use webview2_com::Microsoft::Web::WebView2::Win32::*;
    match reason {
        COREWEBVIEW2_PROCESS_FAILED_REASON_UNEXPECTED => "unexpected",
        COREWEBVIEW2_PROCESS_FAILED_REASON_UNRESPONSIVE => "unresponsive",
        COREWEBVIEW2_PROCESS_FAILED_REASON_TERMINATED => "terminated",
        COREWEBVIEW2_PROCESS_FAILED_REASON_CRASHED => "crashed",
        COREWEBVIEW2_PROCESS_FAILED_REASON_LAUNCH_FAILED => "launch failed",
        COREWEBVIEW2_PROCESS_FAILED_REASON_OUT_OF_MEMORY => "out of memory",
        COREWEBVIEW2_PROCESS_FAILED_REASON_PROFILE_DELETED => "profile deleted",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::{StartHidden, WindowLife, MAIN_LABEL};
    use crate::commands::overlay::OVERLAY_LABEL;

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

    #[test]
    fn a_release_armed_before_the_window_was_used_again_is_stale() {
        let life = WindowLife::default();
        let armed = life.bump(MAIN_LABEL);
        // Shown and hidden again before the timer fired: a newer epoch.
        life.bump(MAIN_LABEL);
        assert_ne!(
            life.epoch(MAIN_LABEL).load(std::sync::atomic::Ordering::SeqCst),
            armed
        );
        // The HUD counts on its own.
        assert_eq!(
            life.epoch(OVERLAY_LABEL).load(std::sync::atomic::Ordering::SeqCst),
            0
        );
    }
}
