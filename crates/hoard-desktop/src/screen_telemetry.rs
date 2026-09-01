//! **Hoard Screen**'s telemetry: does anybody open the overlay, how long do they
//! keep it up, and what do they put inside it?
//!
//! That is the question that decides whether Screen deserves more work, and until
//! now there was no way to answer it: 116 people have had Pro for seven days and
//! there is not one data point on whether they ever launched it once. Polishing a
//! feature nobody may be discovering is the most expensive work there is.
//!
//! There is no new pipe. These are `tracing` events with a fixed `target`
//! ([`SCREEN_TARGET`]), so they travel through `logship` like everything else, with
//! its path redaction and its opt-in, and they are queried with a `where target =
//! ...`. They go at INFO because the process filter (`info`) would drop a DEBUG
//! before any layer saw it, and `wire::ships_at` exempts them from Cloud's minimum
//! (WARN) just as it does the detection verdicts.
//!
//! ## What is NOT sent
//!
//! Not one window title, application name or thumbnail. The overlay mirrors
//! arbitrary windows on somebody else's desktop: whatever is there is not our
//! business. What goes out is the panel's **type** (window, crosshair, scope) and
//! how many there are, which is what answers the question, and nothing else.
//!
//! ## One session is one open row and one close row
//!
//! [`Session`] accumulates in memory while the overlay lives and releases the
//! summary on close. If the app dies outright with the overlay up, that session
//! loses its close: that is why the panel shows opens and closes separately instead
//! of trusting them to match. A visible gap beats a mean silently skewed towards
//! short sessions.

use std::sync::Mutex;
use std::time::Instant;

use hoard_core::wire::SCREEN_TARGET;

/// Why the session ended. Telling them apart matters: `Crashed` mixed into `User`
/// turns a failure into "the user wasn't interested".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndReason {
    /// The user closed the overlay from the app.
    User,
    /// The process ended on its own with code 0 (Esc, or an internal quit).
    SelfQuit,
    /// The process died with a non-zero code or on a signal.
    Crashed,
}

impl EndReason {
    fn as_str(self) -> &'static str {
        match self {
            EndReason::User => "user",
            EndReason::SelfQuit => "self_quit",
            EndReason::Crashed => "crashed",
        }
    }

    /// Translates the sidecar's exit code. `None` (a signal, or our own kill after
    /// asking it to quit) counts as a clean close: when we close it we have already
    /// emitted the event and this road is never walked.
    pub fn from_exit_code(code: Option<i32>) -> Self {
        match code {
            Some(0) | None => EndReason::SelfQuit,
            Some(_) => EndReason::Crashed,
        }
    }
}

/// What people put inside the overlay. It is the product question: if everybody
/// places crosshairs and nobody mirrors a window, Screen is something other than
/// what we think it is.
#[derive(Clone, Copy, Debug, Default)]
struct ByKind {
    window: u32,
    crosshair: u32,
    scope: u32,
    other: u32,
}

impl ByKind {
    fn bump(&mut self, kind: &str) {
        match kind {
            "window" => self.window += 1,
            "crosshair" => self.crosshair += 1,
            "scope" => self.scope += 1,
            _ => self.other += 1,
        }
    }

    fn total(self) -> u32 {
        self.window + self.crosshair + self.scope + self.other
    }
}

/// An overlay session in progress.
pub struct Session {
    started: Instant,
    /// Since when it has been in editor mode, when it is.
    editor_since: Option<Instant>,
    /// Time accumulated in editor mode, in seconds.
    editor_secs: f64,
    /// How many times editor mode has been entered.
    editor_flips: u32,
    added: ByKind,
    removed: ByKind,
    /// The peak of panels alive at once. The final count will not do: somebody who
    /// places four and removes them before closing has used Screen, not the
    /// opposite.
    peak_panels: u32,
    live_panels: i64,
    /// Escenas empujadas al overlay: mide el trasteo (mover, redimensionar,
    /// recortar) sin instrumentar cada arrastre.
    edits: u32,
    /// Botones asignados a un visor, por modo (`toggle` / `hold` / `timed`).
    bindings: u32,
    monitors: u32,
}

impl Session {
    fn new(monitors: u32) -> Self {
        Self {
            started: Instant::now(),
            editor_since: None,
            editor_secs: 0.0,
            editor_flips: 0,
            added: ByKind::default(),
            removed: ByKind::default(),
            peak_panels: 0,
            live_panels: 0,
            edits: 0,
            bindings: 0,
            monitors,
        }
    }

    /// Closes the open editing stretch, if any, and returns the total.
    fn editor_total(&mut self) -> f64 {
        if let Some(since) = self.editor_since.take() {
            self.editor_secs += since.elapsed().as_secs_f64();
        }
        self.editor_secs
    }
}

/// Tauri state: the live session, when there is one. Registered in `lib.rs` with
/// `.manage(ScreenTelemetry::default())`.
#[derive(Default)]
pub struct ScreenTelemetry(pub Mutex<Option<Session>>);

impl ScreenTelemetry {
    /// El overlay acaba de arrancar.
    pub fn opened(&self, monitors: u32) {
        let mut guard = self.0.lock().unwrap();
        // An open with a live session should not happen (`screen_open` is
        // idempotent), but if it does the old one is lost with no close, and that
        // showing up in the panel beats inventing a duration for it.
        *guard = Some(Session::new(monitors));
        tracing::info!(
            target: SCREEN_TARGET,
            event = "open",
            monitors = monitors,
            "screen: overlay opened"
        );
    }

    /// The overlay is gone. Releases the session's summary.
    ///
    /// Idempotent: with no session left (the user's close followed by the process's
    /// `Terminated`) it emits nothing, so nothing gets counted twice.
    pub fn closed(&self, reason: EndReason) {
        let Some(mut s) = self.0.lock().unwrap().take() else {
            return;
        };
        let secs = s.started.elapsed().as_secs_f64();
        let editor_secs = s.editor_total();
        tracing::info!(
            target: SCREEN_TARGET,
            event = "close",
            reason = reason.as_str(),
            secs = secs.round() as u64,
            editor_secs = editor_secs.round() as u64,
            editor_flips = s.editor_flips,
            monitors = s.monitors,
            peak_panels = s.peak_panels,
            added = s.added.total(),
            added_window = s.added.window,
            added_crosshair = s.added.crosshair,
            added_scope = s.added.scope,
            removed = s.removed.total(),
            edits = s.edits,
            bindings = s.bindings,
            "screen: overlay closed"
        );
    }

    /// Editor mode on or off. It comes from the overlay itself over stdout, so it
    /// covers both roads: the app's button and the global Ctrl+O.
    pub fn editor(&self, on: bool) {
        let mut guard = self.0.lock().unwrap();
        let Some(s) = guard.as_mut() else { return };
        if on {
            if s.editor_since.is_none() {
                s.editor_since = Some(Instant::now());
                s.editor_flips += 1;
            }
        } else if let Some(since) = s.editor_since.take() {
            s.editor_secs += since.elapsed().as_secs_f64();
        }
    }

    /// Something the user did inside the editor. `kind` only means anything for
    /// `panel_add` and `panel_remove` (`window`, `crosshair`, `scope`) and for
    /// `binding` (`toggle`, `hold`, `timed`).
    ///
    /// On top of accumulating into the session it emits a row of its own: the
    /// summary says "this session placed two scopes", and the loose rows answer the
    /// other question, the funnel's, of how many people ever got to use each
    /// piece.
    pub fn action(&self, action: &str, kind: Option<&str>) {
        {
            let mut guard = self.0.lock().unwrap();
            if let Some(s) = guard.as_mut() {
                let k = kind.unwrap_or("");
                match action {
                    "panel_add" => {
                        s.added.bump(k);
                        s.live_panels += 1;
                        s.peak_panels = s.peak_panels.max(s.live_panels.max(0) as u32);
                    }
                    "panel_remove" => {
                        s.removed.bump(k);
                        s.live_panels = (s.live_panels - 1).max(0);
                    }
                    "edit" => s.edits += 1,
                    "binding" => s.bindings += 1,
                    _ => {}
                }
            }
        }
        // `edit` fires on every scene nudge (dragging a panel is many of them): it
        // accumulates into the session but produces no row of its own, or we would be
        // the ones building the landfill.
        if action == "edit" {
            return;
        }
        tracing::info!(
            target: SCREEN_TARGET,
            event = "action",
            action = action,
            kind = kind.unwrap_or("-"),
            "screen: {action}"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_time_accumulates_across_flips() {
        let t = ScreenTelemetry::default();
        t.opened(1);
        t.editor(true);
        std::thread::sleep(std::time::Duration::from_millis(20));
        t.editor(false);
        let acc = {
            let mut g = t.0.lock().unwrap();
            g.as_mut().unwrap().editor_total()
        };
        assert!(acc >= 0.02, "the editing stretch was lost: {acc}");
        // Turning it on twice in a row starts no second stopwatch and counts no
        // second entry: the overlay re-emits its mode when it resyncs
        // (`get_scene`).
        t.editor(true);
        t.editor(true);
        let g = t.0.lock().unwrap();
        assert_eq!(g.as_ref().unwrap().editor_flips, 2);
    }

    #[test]
    fn peak_panels_survives_removing_them_before_closing() {
        let t = ScreenTelemetry::default();
        t.opened(2);
        for kind in ["window", "crosshair", "scope"] {
            t.action("panel_add", Some(kind));
        }
        t.action("panel_remove", Some("window"));
        t.action("panel_remove", Some("scope"));
        let g = t.0.lock().unwrap();
        let s = g.as_ref().unwrap();
        assert_eq!(s.peak_panels, 3);
        assert_eq!(s.added.total(), 3);
        assert_eq!(s.removed.total(), 2);
        assert_eq!(s.added.scope, 1);
    }

    /// Removing more than there is (a resynced scene, a panel deleted twice) must
    /// not leave the live counter negative and skew the next peak.
    #[test]
    fn live_panels_never_goes_negative() {
        let t = ScreenTelemetry::default();
        t.opened(1);
        t.action("panel_remove", Some("window"));
        t.action("panel_remove", Some("window"));
        t.action("panel_add", Some("window"));
        let g = t.0.lock().unwrap();
        assert_eq!(g.as_ref().unwrap().peak_panels, 1);
    }

    #[test]
    fn closing_twice_only_reports_once() {
        let t = ScreenTelemetry::default();
        t.opened(1);
        t.closed(EndReason::User);
        assert!(t.0.lock().unwrap().is_none());
        // The process's `Terminated` arrives after the user's close: it must emit no
        // second session and must not panic.
        t.closed(EndReason::SelfQuit);
    }

    #[test]
    fn an_exit_code_tells_a_crash_from_a_clean_quit() {
        assert_eq!(EndReason::from_exit_code(Some(0)), EndReason::SelfQuit);
        assert_eq!(EndReason::from_exit_code(None), EndReason::SelfQuit);
        assert_eq!(EndReason::from_exit_code(Some(1)), EndReason::Crashed);
    }
}
