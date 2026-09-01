//! The sync engine's commands, as a **client of the service** rather than its
//! owner.
//!
//! The desktop used to embed the engine: `start_agent` called `agent::spawn`, kept
//! the `AgentHandle` in `AppState` and forwarded the `AgentEvent`s to the UI. That
//! tied the sync to the window (closing the app stopped the sync unless the CLI held
//! the pidfile) and demanded an arbiter between two engines. The engine lives in
//! `hoardd` now, one per user, outliving the app, and these commands are what ADR
//! 0021 asks for: send it requests over the IPC and paint what it reports.
//!
//! What changed and what did not:
//!
//! - **Unchanged**: the surface the UI sees. The same `#[tauri::command]`s, the same
//!   `agent://*` event names, the same `AgentStatus`. D.3's hard constraint is that
//!   the TS stores never learn the backend moved.
//! - **Changed**: who does the work. The watched set, `state.json`'s persistence and
//!   presence belong to the service. The client **announces** changes
//!   ([`hoard_core::ipc::Request::Reload`]); it does not send lists of saves.
//! - **Gone**: the pidfile. There is no engine to arbitrate here, and none anywhere
//!   else either: `hoard_agent::instance` is deleted and the arbiter is ownership of
//!   the service's socket.

use std::collections::HashSet;
use std::sync::OnceLock;

use hoard_agent::agent::WatchedSave;
use hoard_agent::prefs::Prefs;
use hoard_core::ipc::{AgentSlotStatus, EngineDownReason, Request, ServerSession, ServerUser};
use tauri::{AppHandle, Manager, State};

use crate::daemon::{self, AgentStatus};
use crate::state::AppState;

/// Serialises concurrent starts. The startup rehydration is fired from two places
/// (the automatic-mode scheduler and the cloud login), and with an `await` in the
/// middle both could pass the "is it up already?" check and duplicate the work. It
/// is still worth it now that starting is idempotent: it avoids two `ensure_running`
/// calls at once, which would launch two daemons (one would exit on its own, but the
/// log reads better this way).
fn agent_start_gate() -> &'static tokio::sync::Mutex<()> {
    static GATE: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    GATE.get_or_init(|| tokio::sync::Mutex::new(()))
}

/// Makes sure the service is up and returns its engine's state. Idempotent. The
/// event relay is switched on by [`attach_agent_events`].
#[tauri::command]
pub async fn start_agent(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<AgentStatus, String> {
    let _start = agent_start_gate().lock().await;

    // Brings the service up when there is none (the command connection does "spawn
    // if absent") and asks how it is. The event relay is **not** switched on here:
    // the UI does that once its listeners are in place, because this function is also
    // called by the automatic-mode scan from Rust and can beat the webview's mount.
    let status = state
        .daemon
        .status()
        .await
        .map_err(|e| format!("Couldn't reach the Hoard service: {e:#}"))?;

    // The watcher's dot: what the service says it is really watching, not what we
    // think it should be.
    let mut seen = HashSet::new();
    daemon::announce_slots(&app, &status.slots, &mut seen);

    if !status.engine.running {
        tracing::info!(
            reason = status.engine.last_error.as_deref().unwrap_or("starting"),
            "the Hoard service has no engine yet"
        );
    }

    // Self-repair: the service says it has no session and we have one in hand. That
    // means the earlier handover was lost (the service was reinstalled and started
    // from scratch, or the login happened while it was down) and waiting does not fix
    // it: with no session the engine never comes back, however much backoff it
    // serves. Handing it over again is idempotent and costs one round-trip per app
    // start, so it is done without asking.
    let status = match maybe_rehand_session(&state, &status).await {
        Some(refreshed) => refreshed,
        None => status,
    };

    // The self-hosted server-to-app push (SSE). Cloud receives it through Supabase
    // Realtime, so this only comes up with a live self-hosted session. It is decided
    // from what is already in memory: probing `/v1/health` just for this was one
    // network request on the startup path.
    let selfhosted = state
        .user
        .lock()
        .unwrap()
        .as_ref()
        .is_some_and(|u| !u.is_cloud_server);
    if selfhosted {
        crate::commands::selfhosted_events::start(&app);
    }

    let reported = AgentStatus::from_daemon(&status);
    daemon::emit_status(&app, &reported);
    Ok(reported)
}

/// Re-hands the self-hosted session over when the engine is down **for want of it**
/// and this process does have it on loan. Returns the new state when the handover
/// works; `None` when there was nothing to do or it could not be done.
///
/// It acts on [`EngineDownReason::NoSession`] only, deliberately: with an unreadable
/// keyring or an expired token, handing the same thing over again fixes nothing and
/// would only add noise to the log of a service that is already saying what is wrong
/// with it.
async fn maybe_rehand_session(
    state: &State<'_, AppState>,
    status: &hoard_core::ipc::DaemonStatus,
) -> Option<hoard_core::ipc::DaemonStatus> {
    if status.engine.running || status.engine.reason != EngineDownReason::NoSession {
        return None;
    }
    let creds = hoard_agent::credentials::lent()?;
    let session = ServerSession {
        server_url: creds.url,
        token: creds.token,
        user: creds.user.map(|u| ServerUser {
            user_id: u.user_id,
            username: u.username,
            is_admin: u.is_admin,
        }),
    };
    if let Err(err) = state.daemon.adopt_server_session(session).await {
        tracing::warn!(error = %format!("{err:#}"), "the service didn't take the session we re-handed");
        return None;
    }
    tracing::info!("re-handed our session to a service that had none");
    // The daemon restarts the engine when it adopts, so the state we just read is
    // already stale. Asking again is what makes the window paint "up" on this very
    // start rather than on the next poll.
    state.daemon.status().await.ok()
}

/// Detaches the app from the service (logout, shutdown).
///
/// **The service stays alive**: that is the whole point. Closing the app or signing
/// out must not stop the sync; `hoard sync stop` is there for that, and it is an
/// explicit order. What is released here are this window's connections and tasks.
#[tauri::command]
pub async fn stop_agent(app: AppHandle, _state: State<'_, AppState>) -> Result<(), String> {
    // The SSE subscriber stops in the same step: it has nobody to dispatch to and,
    // on a logout, the credentials it reads are about to disappear.
    crate::commands::selfhosted_events::stop(&app);
    daemon::detach(&app);
    daemon::emit_status(&app, &AgentStatus::down());
    Ok(())
}

/// The UI has its `agent://*` `listen()`s in place: start relaying the service's
/// events to it (the backlog from the cursor, plus the live push).
///
/// It is separate from `start_agent` on purpose (see [`crate::daemon`]): whoever
/// switches the relay on has to be whoever listens, or the first backlog is emitted
/// into the void.
#[tauri::command]
pub async fn attach_agent_events(app: AppHandle) -> Result<(), String> {
    daemon::attach(&app);
    Ok(())
}

/// La UI deja de escuchar (logout, recarga). Para el relevo; el servicio sigue.
#[tauri::command]
pub async fn detach_agent_events(app: AppHandle) -> Result<(), String> {
    daemon::detach(&app);
    Ok(())
}

/// What this process already knows, copied as it is: the engine's state, the
/// journal's last rows and the cloud's pulse.
///
/// **It switches nothing on.** No `attach`, no `start_agent`, no request to the
/// service: it reads three in-memory mutexes. That is the entire point, since what
/// asks for it is a surface that only looks (the Alt+H HUD), and opening a window to
/// look must have no effects.
///
/// It exists because the other road, listening, is no use to whoever arrives late:
/// the backlog is emitted once when the app starts, [`attach_agent_events`] is
/// idempotent, and `emit_status` only speaks when something changes. A window
/// created after all that can have its listeners perfectly in place and never
/// receive a single line.
///
/// Synchronous on purpose: with no `async` there is nowhere to put an `await` to the
/// service, so "this only reads" is guaranteed by the type and not by the good faith
/// of whoever edits it tomorrow.
#[tauri::command]
pub fn agent_snapshot(state: State<'_, AppState>) -> daemon::UiSnapshot {
    state.daemon.snapshot()
}

/// Forces a backup now, skipping the debounce.
#[tauri::command]
pub async fn backup_now(save_id: String, state: State<'_, AppState>) -> Result<(), String> {
    state
        .daemon
        .request(Request::BackupNow { save_id })
        .await
        .map(|_| ())
        .map_err(|e| format!("{e:#}"))
}

/// A staggered backup sweep over every tracked save, fired by the automatic-mode
/// tick. The service having no engine is not an error: the next tick will sweep.
#[tauri::command]
pub async fn sweep_backups(state: State<'_, AppState>) -> Result<(), String> {
    let window_secs = Prefs::load_default()
        .map(|(p, _)| p.automatic_backup_interval_secs)
        .unwrap_or(3600);
    state
        .daemon
        .tell("run a backup sweep", Request::SweepAll { window_secs })
        .await;
    Ok(())
}

/// A diagnostic snapshot of every watched slot. It feeds the hidden Settings panel.
/// Empty means the service has no engine, and the UI shows the agent as stopped.
#[tauri::command]
pub async fn agent_status(state: State<'_, AppState>) -> Result<Vec<AgentSlotStatus>, String> {
    match state.daemon.status().await {
        Ok(status) => Ok(status.slots),
        Err(e) => Err(format!("{e:#}")),
    }
}

// ---- pegamento con el resto de comandos --------------------------------

/// A new save starts being watched without restarting anything.
///
/// It deliberately does not send the `WatchedSave` over the wire: the service owns
/// the watched set, so the client tells it `state.json` changed and it re-hydrates
/// (D.15). Sending the save would be the client deciding what the engine watches.
pub(crate) async fn attach_save_if_running(state: &State<'_, AppState>, _save: WatchedSave) {
    state.daemon.notify_reload().await;
}

/// Un save que deja de rastrearse deja de vigilarse. Mismo aviso: el servicio
/// compara lo que vigila con lo que hay en disco.
pub(crate) async fn detach_save_if_running(state: &State<'_, AppState>, _save_id: String) {
    state.daemon.notify_reload().await;
}

/// Applies the live effect of a change to a save's settings
/// (`hoard_agent::library::set_paused`/`set_preset`/`set_local_path`). Attach,
/// detach and reseat are the same thing from here: the disk changed.
pub(crate) async fn apply_reseat(
    state: &State<'_, AppState>,
    reseat: hoard_agent::library::LiveReseat,
) {
    if matches!(reseat, hoard_agent::library::LiveReseat::Noop) {
        return;
    }
    state.daemon.notify_reload().await;
}

/// Tells the service the session on disk changed (a login, a logout, a change of
/// account) so it drops the engine and brings it up resolving credentials again.
///
/// Fire-and-forget: whoever signs out must not wait on a socket, and the daemon's
/// keeper retries on its own.
pub(crate) fn notify_session_changed(app: &AppHandle) {
    let app = app.clone();
    tokio::spawn(async move {
        app.state::<AppState>()
            .daemon
            .tell(
                "tell the service the session changed",
                Request::RestartEngine,
            )
            .await;
    });
}

/// Hands the engine the candidate folders from the last scan so it can probe the
/// process-to-write correlation. It is the one thing the client does send as a list:
/// detection lives here for now.
pub(crate) async fn set_probe_candidates(app: &AppHandle, dirs: Vec<std::path::PathBuf>) {
    let count = dirs.len();
    // The wire is JSON: a path that is not UTF-8 does not fit, and it is said here,
    // which is where we still know which one it was.
    let mut sendable = Vec::with_capacity(dirs.len());
    for dir in dirs {
        match dir.into_os_string().into_string() {
            Ok(text) => sendable.push(text),
            Err(bad) => tracing::warn!(
                path = %std::path::Path::new(&bad).display(),
                "automatic scan: dropping a probe candidate whose path isn't UTF-8"
            ),
        }
    }
    app.state::<AppState>()
        .daemon
        .tell(
            "send the probe candidates",
            Request::SetProbeCandidates { dirs: sendable },
        )
        .await;
    tracing::debug!(
        count,
        "automatic scan: probe candidates sent to the service"
    );
}

/// Pushes a preference to the engine. The prefs are already saved to disk by the
/// time this runs, so a failure here is cosmetic: the engine reads it anyway on its
/// next start.
pub(crate) async fn push_pref(state: &State<'_, AppState>, request: Request) {
    state.daemon.tell("push a preference", request).await;
}

#[cfg(test)]
mod tests {
    use hoard_agent::library::name_matches;

    #[test]
    fn name_match_kebab_vs_titlecase() {
        assert!(name_matches("Stardew Valley", "stardew-valley"));
        assert!(name_matches("Hollow Knight", "hollow-knight"));
        assert!(name_matches(
            "Subnautica: Below Zero",
            "subnautica-below-zero"
        ));
        assert!(!name_matches("Stardew Valley", "stardew-vallei"));
        assert!(!name_matches("", ""));
    }
}
