//! Background scheduler that backs the sidebar's "Modo Automático" toggle.
//!
//! When the user flips the toggle on, we persist `prefs.automatic_mode = true`
//! and start a Tokio task that fires every `automatic_scan_interval_hours`.
//! Each tick emits a `automatic-tick` Tauri event the frontend listens to and
//! runs the JS-side `runAutomaticSetup()` (formerly `runMagicSetup`). We
//! deliberately keep the heavy lifting in the UI layer — the magic flow
//! reads the catalog cache, dispatches toasts, and boots the live agent
//! through stores that don't exist on the Rust side. Re-implementing all of
//! that here would mean two implementations of the same logic drifting in
//! lockstep.
//!
//! Lifecycle:
//! * `start(app, hours)` aborts any previous handle and spawns a fresh
//!   task. We emit `automatic-tick` **immediately** so flipping the toggle
//!   on (or saving a new interval in Settings) kicks off a scan right
//!   away — users expect "Modo Automático" to do something visible the
//!   moment it's enabled. After that we drive a `tokio::time::interval`
//!   normally; its built-in zero-delay first tick is consumed before the
//!   loop so subsequent emits land on full-interval boundaries.
//! * `stop(app)` aborts the handle and clears the slot.
//! * `restart_if_enabled(app)` is called from Tauri setup so a user who
//!   left the toggle on across reboots gets their scheduler back without
//!   touching the UI.
//!
//! The scheduler lives as a `tauri::State` singleton so the JoinHandle
//! survives between Tauri commands. Aborting a `JoinHandle` is cooperative
//! — the task wakes on its next `await` and notices the cancel; for an
//! interval-driven loop that's at most `interval_hours` later, but in
//! practice we abort + replace immediately so the cancelled task drops on
//! its very next sleep.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use tokio::task::JoinHandle;
use tokio::time::interval;

use hoard_agent::prefs::Prefs;

/// Managed singleton holding the currently-active scheduler task, if any.
/// Registered with `app.manage(AutomaticScheduler::default())` during Tauri
/// setup. A single instance lives for the whole app lifetime; we mutate the
/// inner `Option<JoinHandle>` to swap tasks on toggle changes.
#[derive(Default)]
pub struct AutomaticScheduler {
    handle: Arc<Mutex<Option<JoinHandle<()>>>>,
}

/// Cancel any in-flight scheduler task and start a new one ticking every
/// `interval_hours`. Safe to call repeatedly — each call cleanly replaces
/// the previous handle.
///
/// On every tick we emit a `automatic-tick` Tauri event. The frontend's
/// `initAutomaticListener()` (in `lib/stores/automatic.ts`) listens once
/// per app load and calls `runAutomaticSetup()` in response.
pub fn start(app: &AppHandle, interval_hours: u32) {
    let scheduler = app.state::<AutomaticScheduler>();
    {
        let mut slot = scheduler.handle.lock().unwrap();
        if let Some(prev) = slot.take() {
            prev.abort();
        }
    }

    let hours = interval_hours.max(1) as u64;
    let period = Duration::from_secs(hours * 3600);
    let app = app.clone();
    let new_handle = tokio::task::spawn(async move {
        // Fire one tick immediately so toggling Modo Automático on (or
        // changing the interval from Settings) produces a visible scan
        // right away instead of waiting up to `interval_hours`. The UI
        // is idempotent — if it already ran a scan synchronously on
        // toggle-on it will just no-op the duplicate one.
        tracing::info!(
            interval_hours = hours,
            "automatic mode: immediate scan on (re)start"
        );
        if let Err(e) = app.emit("automatic-tick", ()) {
            tracing::warn!(error = %e, "automatic mode: couldn't emit initial tick event");
        }

        let mut ticker = interval(period);
        // Consume the zero-delay first tick from `tokio::time::interval`
        // so the loop below waits a full `period` before the next emit
        // (we just fired manually above).
        ticker.tick().await;
        loop {
            ticker.tick().await;
            tracing::info!(
                interval_hours = hours,
                "automatic mode: scheduled scan starting"
            );
            // Best-effort emit — if no window is listening (rare; the
            // listener registers in App.svelte::onMount) we silently
            // drop the tick rather than queue events.
            if let Err(e) = app.emit("automatic-tick", ()) {
                tracing::warn!(error = %e, "automatic mode: couldn't emit tick event");
            }
        }
    });

    let mut slot = scheduler.handle.lock().unwrap();
    *slot = Some(new_handle);
}

/// Abort the running scheduler, if any. No-op when nothing is scheduled —
/// safe to call from any code path that wants to wind the loop down
/// (toggle off, app shutdown).
pub fn stop(app: &AppHandle) {
    let scheduler = app.state::<AutomaticScheduler>();
    let mut slot = scheduler.handle.lock().unwrap();
    if let Some(prev) = slot.take() {
        prev.abort();
        tracing::info!("automatic mode: scheduler stopped");
    }
}

/// Re-arm the scheduler if the user had Modo Automático enabled before the
/// app last closed. Called from the Tauri `setup` closure once the
/// `AutomaticScheduler` state is managed. Errors are non-fatal — the
/// toggle still shows the persisted value and the user can re-trigger by
/// flipping it off and on again.
pub async fn restart_if_enabled(app: &AppHandle) -> anyhow::Result<()> {
    let (prefs, _) = Prefs::load_default()?;
    if prefs.automatic_mode {
        tracing::info!(
            interval_hours = prefs.automatic_scan_interval_hours,
            "automatic mode: rehydrating scheduler from prefs"
        );
        start(app, prefs.automatic_scan_interval_hours);
    }
    Ok(())
}
