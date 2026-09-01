//! The engine inside the daemon: starting it, keeping it alive and pumping its
//! events into the journal.
//!
//! One engine per user, owned by this process. Three pieces:
//!
//! - [`Engine`]: the shared slot. The IPC server asks it for the `AgentHandle` and
//!   for the state; it never starts or stops the engine on its own.
//! - [`keeper`]: the supervised loop that **makes sure** the engine is up. It
//!   resolves the session, calls `agent::spawn`, and brings the engine back when it
//!   dies. Nothing here may die in silence (D.12): the keeper detects a finished
//!   `JoinHandle` rather than trusting a boolean, which is exactly how the poller's
//!   gate got stuck.
//! - [`pump`]: the supervised loop that consumes `AgentEvent`s, persists them into
//!   `state.json` and puts them in the journal (which is what pushes to the
//!   clients).
//!
//! ## The pidfile is dead
//!
//! While the desktop and `hoard sync` embedded `agent::spawn`, the arbiter between
//! daemon and embedded engine was a pidfile (`agent.pid`,
//! `hoard_agent::instance`): the keeper consulted it and started no engine when
//! somebody else held it. That check rotted (it accepted as a live owner any
//! process whose name contained "hoard", and every client contains it), and the
//! file is gone entirely: **the arbiter is ownership of the socket**, a mutex with
//! real liveness that the kernel releases when the process dies, not a file you
//! have to guess is lying.
//!
//! That is why [`Running`] holds no lock any more: the only thing preventing two
//! engines is that there is only one daemon, and the bind answers for that
//! (`transport::Listener`).

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use hoard_agent::agent::{self, AgentConfig, AgentEvent, AgentHandle};
use hoard_agent::api::ApiClient;
use hoard_agent::config::CliConfig;
use hoard_agent::prefs::Prefs;
use hoard_agent::presence::PresenceHandle;
use hoard_agent::state::CliState;
use hoard_agent::supervisor::Finished;
use hoard_agent::{cloud_live, library, presence};
use hoard_core::ipc::{EngineDownReason, EngineStatus, KeyringFault};
use time::OffsetDateTime;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::journal::EventLog;

/// How often the keeper checks the engine is still alive.
const KEEPER_TICK: Duration = Duration::from_secs(5);

/// The backoff after a failed engine start (no session, network down).
const START_BACKOFF_MIN: Duration = Duration::from_secs(5);
const START_BACKOFF_MAX: Duration = Duration::from_secs(5 * 60);

/// How often the Cloud push's tasks are checked to be alive.
const CLOUD_LIVE_CHECK: Duration = Duration::from_secs(15);

/// Margen que se le da al motor para atender su `shutdown` antes de abortarlo.
const SHUTDOWN_GRACE: Duration = Duration::from_secs(10);

/// Pasado esto, una transferencia "en vuelo" se da por muerta. Ver
/// [`Engine::transfers_in_flight`]: es el seguro contra un descuadre del
/// contador, no un tiempo de espera de red.
const TRANSFER_STALE: Duration = Duration::from_secs(30 * 60);

/// The longest wait for the engine to answer a status query. With no ceiling, a
/// stuck engine would hang the client that asked (and the UI behind it).
const STATUS_TIMEOUT: Duration = Duration::from_secs(5);

/// Aborts a group of tasks when dropped. Without this, restarting the Cloud push
/// would leave the old tasks running, and two pollers is exactly the failure D.12
/// documents.
struct AbortOnDrop(Vec<JoinHandle<()>>);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        for task in &self.0 {
            task.abort();
        }
    }
}

/// Lo que compone un motor vivo. Todo se tira junto.
struct Running {
    handle: AgentHandle,
    task: JoinHandle<()>,
    presence: PresenceHandle,
    /// The live engine's client. `Reload` rebuilds the watched set and for that it
    /// has to ask again which saves are archived: without this, archiving a save
    /// would have no effect until the next start. It shares its token cell with the
    /// other clones, so the JWT the refresher rotates is good here too.
    client: ApiClient,
    /// Tareas auxiliares del motor (presencia, empuje Cloud, refresher del JWT).
    aux: Vec<JoinHandle<()>>,
}

impl Drop for Running {
    fn drop(&mut self) {
        // Dropping a `JoinHandle` does **not** cancel its task: detaching them would
        // leave the old engine's poller, presence beat and token rotator running
        // alongside the new one's. Two pollers and two rotators of the same refresh
        // token is the family of bugs D.12 and Part A document, so dying completely is
        // part of this type's contract.
        for task in &self.aux {
            task.abort();
        }
        self.task.abort();
    }
}

#[derive(Default)]
struct Inner {
    running: Option<Running>,
    status: EngineStatus,
    /// We stopped on purpose (`Request::Shutdown`): the keeper must not revive it.
    stopping: bool,
    /// A client asked for the engine to be brought up from scratch (the session on
    /// disk changed), and why. The keeper serves it, being the only owner of the
    /// lifecycle.
    restart_requested: Option<String>,
    /// Backups and restores started and still without an outcome.
    ///
    /// The event pump keeps it, being where all of them pass, and the updater reads
    /// it: swapping the binaries with an upload halfway through kills the process
    /// doing it and leaves a half-committed blob on the server. It is the one brake
    /// not even the deadline lifts.
    ///
    /// It lives here and not in the engine because it has to **survive an engine
    /// restart**: were it to go with it, an engine bouncing mid-upload would leave
    /// the counter stuck at 1 for ever and the updater would never apply anything
    /// again. Starting a new engine zeroes it (see [`Engine::transfers_reset`]),
    /// which is the truth: whatever was in flight went with the previous process.
    in_flight: usize,
    /// Since when something has been in flight. What makes the counter expire.
    in_flight_since: Option<Instant>,
}

/// Ranura compartida del motor. Cheap to clone.
#[derive(Clone, Default)]
pub struct Engine {
    inner: Arc<Mutex<Inner>>,
    /// Wakes the keeper from its wait. Without this, an engine down after several
    /// failures sleeps up to [`START_BACKOFF_MAX`], and a login that has just
    /// happened would take up to five minutes to sync even with the client having
    /// said so. `Notify` stores the permit when nobody is listening, so a nudge
    /// arriving while the keeper is working is not lost.
    wake: Arc<tokio::sync::Notify>,
}

impl Engine {
    pub fn new() -> Self {
        Self::default()
    }

    /// The engine's handle, when it is up.
    pub fn handle(&self) -> Option<AgentHandle> {
        self.lock().running.as_ref().map(|r| r.handle.clone())
    }

    pub fn presence(&self) -> Option<PresenceHandle> {
        self.lock().running.as_ref().map(|r| r.presence.clone())
    }

    /// Cliente del motor vivo, si lo hay.
    pub fn client(&self) -> Option<ApiClient> {
        self.lock().running.as_ref().map(|r| r.client.clone())
    }

    pub fn status(&self) -> EngineStatus {
        self.lock().status.clone()
    }

    /// A readable reason for there being no engine, for the `IpcError::EngineDown`
    /// the client receives. A client that only sees "error" retries for ever with
    /// nothing to tell the user.
    pub fn down_reason(&self) -> String {
        let guard = self.lock();
        if guard.stopping {
            return "the daemon is shutting down".to_string();
        }
        guard
            .status
            .last_error
            .clone()
            .unwrap_or_else(|| "the engine is still starting".to_string())
    }

    pub fn set_watched(&self, count: usize) {
        self.lock().status.watched = count;
    }

    /// Starts a transfer (a backup or a restore).
    pub fn transfer_started(&self) {
        let mut guard = self.lock();
        if guard.in_flight == 0 {
            guard.in_flight_since = Some(Instant::now());
        }
        guard.in_flight += 1;
    }

    /// Ends a transfer, well or badly. It saturates at 0: an outcome with no start
    /// (the engine came up with an upload already in flight, D.8.3) must not leave
    /// the counter negative and block the updater for ever.
    pub fn transfer_finished(&self) {
        let mut guard = self.lock();
        guard.in_flight = guard.in_flight.saturating_sub(1);
        if guard.in_flight == 0 {
            guard.in_flight_since = None;
        }
    }

    /// Forgets whatever was in flight. The keeper calls it when bringing up a new
    /// engine: anything halfway through died with the previous one.
    pub fn transfers_reset(&self) {
        let mut guard = self.lock();
        guard.in_flight = 0;
        guard.in_flight_since = None;
    }

    /// Is anything halfway through right now?
    ///
    /// **It expires.** The counter is kept by pairing events, and pairing events is
    /// exactly the kind of tally that goes out of balance the moment somebody adds a
    /// terminal variant and does not count it here. Being out by one would block the
    /// updater **for ever**, in silence, the same failure as the poller's gate with
    /// no RAII (D.10), so past [`TRANSFER_STALE`] whatever it was is taken as
    /// finished. No legitimate backup lasts that long.
    pub fn transfers_in_flight(&self) -> bool {
        let guard = self.lock();
        match guard.in_flight_since {
            Some(since) if guard.in_flight > 0 => since.elapsed() < TRANSFER_STALE,
            _ => false,
        }
    }

    /// Marks that there will be no engine, and why (`--no-engine`). The reason
    /// travels to the client in `EngineDown`: an engine absent **on purpose** has to
    /// be told apart from one that will not start.
    pub fn disable(&self, reason: &str) {
        let mut guard = self.lock();
        guard.status.running = false;
        guard.status.last_error = Some(reason.to_string());
    }

    pub fn stopping(&self) -> bool {
        self.lock().stopping
    }

    /// Engine up **and** its task alive? The second half matters: a panic inside the
    /// agent's loop leaves the handle intact and the commands would be swallowed by a
    /// channel nobody reads.
    pub fn alive(&self) -> bool {
        self.lock()
            .running
            .as_ref()
            .is_some_and(|r| !r.task.is_finished())
    }

    fn install(&self, running: Running, server: String, is_cloud: bool, watched: usize) {
        let mut guard = self.lock();
        guard.status = EngineStatus {
            running: true,
            server: Some(server),
            is_cloud,
            watched,
            since: Some(OffsetDateTime::now_utc()),
            last_error: None,
            reason: EngineDownReason::Unknown,
            keyring: None,
        };
        // A previous engine (the one that died and is being replaced, say) is dropped
        // here: `Running::aux` aborts its tasks when released.
        guard.running = Some(running);
        // Lo que estuviera a medias se fue con el motor anterior. Sin esto, un
        // motor que rebota en mitad de una subida deja el contador clavado y el
        // updater no vuelve a aplicar nada.
        guard.in_flight = 0;
        guard.in_flight_since = None;
    }

    fn note_error(&self, error: String, reason: EngineDownReason, keyring: Option<KeyringFault>) {
        let mut guard = self.lock();
        guard.status.running = false;
        guard.status.last_error = Some(error);
        guard.status.reason = reason;
        // Which way the keyring failed, when that's what failed. Cleared
        // otherwise, or a machine that once hit a locked keyring would keep
        // explaining every later failure with it.
        guard.status.keyring = keyring;
        guard.running = None;
    }

    /// Drops an engine that is already dead **before** another is started: its `Drop`
    /// aborts the auxiliary tasks (token rotator, poller, presence), and two sets of
    /// those alive at once is the 401 family this design kills.
    fn forget(&self) {
        let mut guard = self.lock();
        guard.status.running = false;
        guard.running = None;
    }

    /// Waits `for_`, or until somebody asks for attention, whichever comes first.
    async fn nap(&self, for_: Duration) {
        tokio::select! {
            _ = tokio::time::sleep(for_) => {}
            _ = self.wake.notified() => {}
        }
    }

    /// Asks for the engine to be brought up from scratch with whatever session is
    /// on disk now. It answers [`hoard_core::ipc::Request::RestartEngine`].
    ///
    /// **It asks, it does not do.** The only thing that starts and stops the engine
    /// is the keeper: were the restart executed here, between dropping the old engine
    /// and finishing its shutdown the keeper would see an empty slot and start
    /// another, and during that window there would be two engines in the same
    /// process, with two rotators of the same refresh token. Leaving it as a request
    /// keeps a single owner of the lifecycle.
    ///
    /// The nudge counts even with no engine: that is the typical case (it would not
    /// start for want of a session and the user has just signed in), and waiting out
    /// the backoff would mean never finding out.
    pub fn request_restart(&self, reason: &str) {
        self.lock().restart_requested = Some(reason.to_string());
        self.wake.notify_one();
    }

    /// A session was signed out. Restart only if it was *this* engine's.
    ///
    /// The two sessions are independent (a machine can hold a Cloud one and a
    /// self-hosted one at once) but the engine runs against exactly one of
    /// them, and dropping the other changes nothing it is doing. On 2026-08-28
    /// the desktop forgot the self-hosted session five seconds after the engine
    /// had finally come up on Cloud, and the engine was torn down and rebuilt
    /// for it: a second "watching…" for every save, and a gap in the middle of
    /// a sync that had nothing to do with the session that went.
    ///
    /// A engine that is *down* is restarted either way: the session that is left
    /// may be the one it was missing.
    pub fn request_restart_if_signed_out(&self, was_cloud: bool, reason: &str) {
        let mine = {
            let guard = self.lock();
            !guard.status.running || guard.status.is_cloud == was_cloud
        };
        if mine {
            self.request_restart(reason);
        } else {
            tracing::info!(
                signed_out_cloud = was_cloud,
                "hoardd: a session was signed out, but not the one the engine runs on, leaving it alone"
            );
        }
    }

    /// The session can be read *right now*, somebody just did it. Wake an engine
    /// that is down because it couldn't.
    ///
    /// The backoff after a failed start is five minutes, which is the right
    /// pace for a keyring that keeps refusing and the wrong one for a keyring
    /// that has started answering: on 2026-08-28 the desktop opened at 05:34:48
    /// and lent a Cloud token successfully, and the engine, down since 05:31:08
    /// for not being able to read that same session, slept until 05:36:10, its
    /// backoff to the second. Eighty-two seconds of "the sync service is
    /// stopped" with the session sitting there, readable.
    ///
    /// Gated on the reason so this can't turn into a retry loop: only the three
    /// session faults are unblocked by a session that reads, and the caller only
    /// calls after a read that worked. A keyring still refusing fails the lend
    /// first and never gets here.
    pub fn wake_if_a_session_would_help(&self) {
        let reason = {
            let guard = self.lock();
            if guard.status.running {
                return;
            }
            guard.status.reason
        };
        if matches!(
            reason,
            EngineDownReason::NoSession
                | EngineDownReason::KeyringUnreadable
                | EngineDownReason::SessionExpired
        ) {
            tracing::info!(
                ?reason,
                "hoardd: the session reads again, waking the engine instead of waiting out its backoff"
            );
            self.request_restart("the session became readable");
        }
    }

    fn take_restart_request(&self) -> Option<String> {
        self.lock().restart_requested.take()
    }

    /// A clean shutdown of the live engine so it can be started again. Keeper only.
    async fn stop_for_restart(&self, reason: &str) {
        let running = {
            let mut guard = self.lock();
            let taken = guard.running.take();
            if taken.is_some() {
                guard.status.running = false;
                guard.status.last_error = Some(reason.to_string());
            }
            taken
        };
        let Some(mut running) = running else { return };
        tracing::info!(reason, "hoardd: restarting the engine");
        // One last presence beat with the old token, which is still good: it leaves
        // this machine greyed out on the other machines' panel instead of going dark
        // without a word.
        running.presence.closing().await;
        if let Err(err) = running.handle.shutdown().await {
            tracing::warn!(error = %err, "hoardd: the engine didn't acknowledge the restart");
        }
        if tokio::time::timeout(SHUTDOWN_GRACE, &mut running.task)
            .await
            .is_err()
        {
            tracing::warn!("hoardd: the engine didn't stop in time; aborting it");
        }
        // Dropping `running` here aborts its auxiliary tasks, so the next start does
        // not live alongside the previous one.
    }

    /// Para el motor. Marca `stopping` **antes** de nada para que el keeper no lo
    /// resucite mientras se apaga.
    pub async fn shutdown(&self) {
        let running = {
            let mut guard = self.lock();
            guard.stopping = true;
            guard.status.running = false;
            guard.running.take()
        };
        let Some(mut running) = running else { return };
        // One last presence beat while the token is good: it greys this machine out
        // on the other machines' panel straight away.
        running.presence.closing().await;
        if let Err(err) = running.handle.shutdown().await {
            tracing::warn!(error = %err, "hoardd: the engine didn't acknowledge shutdown");
        }
        // `shutdown` only *sends* the command: the agent's loop needs the one pass it
        // takes to serve it, or `Drop`'s `abort` would cut it off mid-cleanup.
        // Bounded, so a hung engine does not block the service's shutdown.
        if tokio::time::timeout(SHUTDOWN_GRACE, &mut running.task)
            .await
            .is_err()
        {
            tracing::warn!("hoardd: the engine didn't stop in time; aborting it");
        }
        // Al soltarse `running` se abortan sus tareas auxiliares: nada del motor
        // sobrevive al apagado del servicio.
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        // As in the journal: somebody else's panic must not leave the engine's slot
        // unreachable for ever.
        self.inner.lock().unwrap_or_else(|poisoned| {
            tracing::error!("hoardd: the engine mutex was poisoned; recovering");
            poisoned.into_inner()
        })
    }
}

/// The loop that keeps the engine up. It never returns (which is why it cannot
/// produce a [`Finished`] by accident); `supervise` restarts it on a panic and the
/// shutdown's `abort()` kills it.
pub async fn keeper(engine: Engine, events_tx: mpsc::Sender<AgentEvent>) -> Finished {
    let mut backoff = START_BACKOFF_MIN;
    loop {
        if engine.stopping() {
            // No `Finished` is returned: shutting down is `main`'s business, and it
            // aborts this task. Sleeping here avoids burning CPU in the shutdown
            // window.
            tokio::time::sleep(KEEPER_TICK).await;
            continue;
        }
        // A session change asked for by a client. It is served **before** anything
        // else: whatever live engine there is, is talking to the account that no
        // longer applies. No `continue`, so the start below happens on the same
        // pass.
        if let Some(reason) = engine.take_restart_request() {
            engine.stop_for_restart(&reason).await;
            backoff = START_BACKOFF_MIN;
        }
        if engine.alive() {
            engine.nap(KEEPER_TICK).await;
            continue;
        }
        if engine.status().running {
            // It was up and its task has died: that is an incident, not a normal
            // transition.
            tracing::error!("hoardd: the engine task is gone; restarting it");
            // Drop the corpse (and its tasks) before trying another start.
            engine.forget();
        }
        match start(events_tx.clone()).await {
            Ok(started) => {
                tracing::info!(
                    server = %started.server,
                    watched = started.watched,
                    "hoardd: engine up"
                );
                engine.install(
                    started.running,
                    started.server,
                    started.is_cloud,
                    started.watched,
                );
                backoff = START_BACKOFF_MIN;
            }
            Err(err) => {
                let text = format!("{err:#}");
                let reason = classify(&err);
                let keyring = hoard_agent::keychain::fault(&err);
                tracing::warn!(
                    error = %text,
                    ?reason,
                    keyring = keyring.map(|f| f.as_str()).unwrap_or("-"),
                    retry_in_secs = backoff.as_secs(),
                    "hoardd: couldn't start the engine"
                );
                engine.note_error(text, reason, keyring);
                engine.nap(backoff).await;
                backoff = (backoff * 2).min(START_BACKOFF_MAX);
            }
        }
    }
}

/// Why it would not start, so the window can say so.
///
/// **By downcast, never by the error's text.** A message gets rewritten without a
/// second thought (this one has been rewritten already) and with
/// `contains("no session")` the classification would break in silence, which is
/// exactly the invisible failure all of this exists to kill. Every arm hangs off a
/// type that exists precisely to be recognised here.
fn classify(err: &anyhow::Error) -> EngineDownReason {
    if err
        .downcast_ref::<hoard_agent::session::NoSession>()
        .is_some()
    {
        return EngineDownReason::NoSession;
    }
    // The keyring has two ways to fail (it does not answer, or it answers no) and
    // one piece of advice for the user: sign in again, which rewrites the item in the
    // service's name. They are separated in the log, not on screen.
    if err
        .downcast_ref::<hoard_agent::keychain::KeyringTimeout>()
        .is_some()
        || err
            .downcast_ref::<hoard_agent::keychain::KeyringUnreadable>()
            .is_some()
    {
        return EngineDownReason::KeyringUnreadable;
    }
    if hoard_agent::cloud_auth::is_session_expired(err) {
        return EngineDownReason::SessionExpired;
    }
    EngineDownReason::Other
}

struct Started {
    running: Running,
    server: String,
    is_cloud: bool,
    watched: usize,
}

/// Starts the engine: session, then saves, then `agent::spawn`, then presence, the
/// Cloud push and the JWT refresher.
async fn start(events_tx: mpsc::Sender<AgentEvent>) -> anyhow::Result<Started> {
    // `resolve_owned`: the road that rotates. It belongs to the service and to
    // nobody else; clients use `resolve_borrowed` with the token we lend them.
    let active = hoard_agent::session::resolve_owned().await?;
    // Before hydrating anything: heal the state against the server. It is the one
    // point every machine passes through (start, login, update, since the installer
    // restarts the service), so updating the app repairs anybody with rows pointing
    // at ids their server no longer knows. A failure here cannot stop the start: with
    // no network the engine still has local work to do.
    match library::reconcile_with_server(&active.client).await {
        Ok(r) if r.changed() => tracing::info!(
            relinked = r.relinked,
            dropped = r.dropped,
            "hoardd: reconciled tracked saves with the server"
        ),
        Ok(_) => {}
        Err(e) => tracing::warn!(error = %e, "hoardd: couldn't reconcile with the server"),
    }
    let (state, _path) = CliState::load_default()?;
    let archived = archived_save_ids(&active.client).await;
    // With no tracked saves the engine starts anyway (unlike `hoard sync`, which is
    // a command and may abort): a resident service has to be there when the user
    // tracks their first one, and `Request::Reload` picks it up.
    let saves = library::watched_saves_from_state(&state, &archived);
    let watched = saves.len();
    let config = engine_config();

    let (presence_handle, presence_task) = presence::spawn(active.client.clone());
    // Two clones before `agent::spawn` consumes the client. `ApiClient` shares its
    // token cell across clones, so the JWT the refresher rotates reaches the engine
    // and the Cloud push too.
    let live_client = active.client.clone();
    let refresh_client = active.client.clone();
    let reload_client = active.client.clone();
    let global_sync = config.global_sync;
    let (handle, task) = agent::spawn(active.client, config, saves, events_tx);

    let mut aux = vec![presence_task];
    // The low-latency Cloud push (Realtime plus a backup poll). Cloud only, and only
    // with global sync: `backup_only` never writes.
    if active.is_cloud && global_sync {
        aux.push(spawn_cloud_live(live_client, handle.clone()));
    }
    // One rotator of the refresh token: this one. `owned()` is `Some` only because
    // we resolved as owners; a client never even receives the refresh token.
    if let Some(session) = active.cloud.as_ref().and_then(|c| c.owned()) {
        let shared = Arc::new(tokio::sync::Mutex::new(session));
        aux.push(tokio::spawn(hoard_agent::supervisor::supervise(
            "hoardd cloud refresh",
            move || hoard_agent::session::refresh_loop(refresh_client.clone(), shared.clone()),
        )));
    }

    Ok(Started {
        running: Running {
            handle,
            task,
            presence: presence_handle,
            client: reload_client,
            aux,
        },
        server: active.server,
        is_cloud: active.is_cloud,
        watched,
    })
}

/// The saves frozen in the server's black box, so they can be left out of the
/// watched set.
///
/// **It never fails upwards.** A server that does not answer, a self-hosted one with
/// no black box, an old version without that endpoint: here they all mean "I know of
/// none archived", which is the long-standing behaviour. Returning an error instead
/// would leave the engine unstarted over an incidental query, and returning a partial
/// set would stop watching saves that are perfectly alive. Of the two possible
/// mistakes, watching too much is the cheap one: an archived save that slips through
/// is stopped by the 403, as it always was.
async fn archived_save_ids(client: &ApiClient) -> HashSet<String> {
    if !client.is_cloud().await {
        return HashSet::new();
    }
    match client.cloud_archived_save_ids().await {
        Ok(ids) => {
            if !ids.is_empty() {
                tracing::info!(count = ids.len(), "hoardd: saves archived on the server");
            }
            ids
        }
        Err(err) => {
            tracing::warn!(
                error = %format!("{err:#}"),
                "hoardd: couldn't ask which saves are archived; watching them all"
            );
            HashSet::new()
        }
    }
}

/// The engine's config, built from the user's preferences.
///
/// The same prefs the desktop reads (`prefs.json` belongs to the user, not to the
/// frontend), with one deliberate exception: when there is **no** prefs file, the
/// machine has never seen the desktop app and this is the headless case, which is
/// what `hoard sync` serves today with global sync and auto-restore on. A home
/// server that only has the CLI must not end up in "upload only" from reading
/// defaults meant for the GUI.
fn engine_config() -> AgentConfig {
    let (prefs, path) = Prefs::load_default()
        .map(|(p, path)| (p, Some(path)))
        .unwrap_or_else(|err| {
            tracing::warn!(error = %err, "hoardd: couldn't read prefs; using defaults");
            (Prefs::default(), None)
        });
    let headless = path.map(|p| !p.exists()).unwrap_or(false);
    AgentConfig {
        auto_restore: prefs.auto_restore || headless,
        global_sync: prefs.global_sync || headless,
        conflict_retention_days: prefs.conflict_retention_days,
        // `data_saving` deliberately does NOT feed the floor any more. Its
        // slider left the UI on 2026-06-14 ("the backend keeps its defaults"),
        // but the pref already written to disk stayed at whatever the user had
        // last dragged it to, and the engine kept honouring it. On one machine
        // that was 1.0, a ten-minute floor between uploads that nothing could
        // show or change: edits were picked up in two seconds and then sat in
        // the queue, which reads as "it doesn't detect my changes". Worse, a
        // restore marks the next backup urgent and skips the floor, so changes
        // arriving from the other machine synced instantly while your own
        // waited, and the two halves looked unrelated.
        //
        // Per-save pacing is still reachable where it is visible: the
        // `data_saver` preset sets its own 600s floor through
        // `SavePolicy::min_snapshot_interval_secs`, and that one the user picks
        // per game and can see.
        min_snapshot_interval_secs: 0,
        // Parks the local copy before letting a newer remote one overwrite it (it
        // never destroys data in silence).
        conflict_root: CliConfig::state_dir().ok().map(|d| d.join("conflicts")),
        ..AgentConfig::default()
    }
}

/// The Cloud push, supervised. `cloud_live::spawn` sets up two loose `tokio::spawn`
/// tasks (poll and Realtime) that survive errors but not a panic, so the keeper
/// covers it from outside: when either task finishes, the pair is dropped and rearmed.
///
/// This is its **only** caller, so the supervision could move inside `cloud_live`
/// without breaking anybody. It is left for the single cloud client work, which will
/// rewrite that function entirely; wrapping it from outside already satisfies D.12.
fn spawn_cloud_live(client: ApiClient, handle: AgentHandle) -> JoinHandle<()> {
    tokio::spawn(hoard_agent::supervisor::supervise(
        "hoardd cloud-live",
        move || {
            let client = client.clone();
            let handle = handle.clone();
            async move {
                let mut tasks = AbortOnDrop(spawn_cloud_live_pair(&client, &handle));
                loop {
                    tokio::time::sleep(CLOUD_LIVE_CHECK).await;
                    if tasks.0.iter().any(|t| t.is_finished()) {
                        tracing::warn!("hoardd: a cloud-live task ended; restarting the pair");
                        // The assignment drops the previous group, which aborts
                        // whatever was still alive. Never two pollers.
                        tasks = AbortOnDrop(spawn_cloud_live_pair(&client, &handle));
                    }
                }
            }
        },
    ))
}

fn spawn_cloud_live_pair(client: &ApiClient, handle: &AgentHandle) -> Vec<JoinHandle<()>> {
    cloud_live::spawn(
        client.clone(),
        handle.clone(),
        cloud_live::Config {
            poll_interval: Duration::from_secs(hoard_agent::prefs::CLOUD_POLL_INTERVAL_SECS as u64),
            global_sync: true,
        },
    )
}

/// Estado de los slots vigilados, con tope de espera. Lo usa el `Status` del IPC.
pub async fn slot_status(engine: &Engine) -> Vec<hoard_core::ipc::AgentSlotStatus> {
    let Some(handle) = engine.handle() else {
        return Vec::new();
    };
    match tokio::time::timeout(STATUS_TIMEOUT, handle.status()).await {
        Ok(Ok(slots)) => slots,
        Ok(Err(err)) => {
            tracing::warn!(error = %err, "hoardd: the engine didn't answer a status query");
            Vec::new()
        }
        Err(_) => {
            tracing::warn!("hoardd: the engine took too long to answer a status query");
            Vec::new()
        }
    }
}

/// Pumps the engine's events: presence, `state.json`, the journal and the native
/// notification.
///
/// The channel belongs to **the daemon**, not to the engine: it is created once and
/// every engine start gets a clone of the sender. That way this loop can restart
/// under `supervise` without losing the receiver, and a restarted engine keeps
/// writing to the same journal (the clients' cursors do not break because the engine
/// bounced).
///
/// The native notifications go out from here and not from each action's executor
/// because this is the **only** place all the engine's events pass through: one
/// notice hanging off the backup branch and another off the restore branch is
/// exactly how the 429 ended up handled on one road and not the other (D.7).
pub async fn pump(
    engine: Engine,
    log: Arc<EventLog>,
    notifier: Arc<crate::notify::Notifier>,
    events_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<AgentEvent>>>,
) -> Finished {
    let mut rx = events_rx.lock().await;
    while let Some(event) = rx.recv().await {
        // Presencia (panel Eye): espeja las transiciones de juego.
        match &event {
            AgentEvent::GameStarted { game_slug, .. } => {
                if let Some(p) = engine.presence() {
                    p.game_started(game_slug.clone());
                }
            }
            AgentEvent::GameStopped { game_slug, .. } => {
                if let Some(p) = engine.presence() {
                    p.game_stopped(game_slug.clone());
                }
            }
            _ => {}
        }
        // What the updater needs to know so it does not swap the binaries mid-upload.
        // It is counted here, the one place **all** the transfers pass through, for
        // the same reason the notifying happens here and not in each branch (D.7).
        match &event {
            AgentEvent::BackupStarted { .. } => engine.transfer_started(),
            AgentEvent::BackupSuccess { .. }
            | AgentEvent::BackupFailed { .. }
            | AgentEvent::BackupThrottled { .. }
            | AgentEvent::BackupTooLarge { .. }
            | AgentEvent::BackupQuotaFull { .. }
            | AgentEvent::BackupSkippedEmpty { .. }
            | AgentEvent::SaveAutoRestored { .. }
            | AgentEvent::SaveAutoRestoreFailed { .. } => engine.transfer_finished(),
            _ => {}
        }
        persist(&event);
        // Antes de meterlo en el journal: el aviso es del evento **vivo**, y un
        // colapso (una racha del mismo reposo) no debe cambiar si suena o no.
        notifier.consider(&event).await;
        log.record(OffsetDateTime::now_utc(), event);
    }
    // The channel only closes when no sender is left, and the daemon keeps one alive
    // while it runs. Reaching here is the shutdown.
    tracing::info!("hoardd: the event channel closed");
    Finished
}

/// Persists into `state.json` what the engine only holds in memory: the version
/// cursor and the anti-reupload signature. Without this, every daemon restart would
/// re-upload identical snapshots and re-download to diff them.
fn persist(event: &AgentEvent) {
    let (save_id, version, set_hash) = match event {
        AgentEvent::BackupSuccess {
            save_id,
            version_num,
            set_hash,
            ..
        } => (save_id, Some(*version_num), set_hash.clone()),
        // After a restore the slot is synced to that version: remembering it is what
        // makes the version gate survive a restart.
        AgentEvent::SaveAutoRestored {
            save_id,
            version_num,
            ..
        } => (save_id, Some(*version_num), None),
        _ => return,
    };

    let (mut state, path) = match CliState::load_default() {
        Ok(v) => v,
        Err(err) => {
            tracing::warn!(error = %err, "hoardd: couldn't load state.json to persist an event");
            return;
        }
    };
    let Some(entry) = state.saves.get_mut(save_id) else {
        // Un save de la nube respaldado antes de adoptarlo no tiene fila local.
        return;
    };
    if let Some(v) = version {
        entry.last_version_num = Some(v);
    }
    if let Some(hash) = set_hash {
        entry.set_hash = Some(hash);
    }
    if matches!(event, AgentEvent::BackupSuccess { .. }) {
        entry.last_backup_at = Some(OffsetDateTime::now_utc());
    }
    if let Err(err) = state.save(&path) {
        tracing::warn!(error = %err, "hoardd: couldn't write state.json");
    }
}

/// Re-hydrates the watched save set from `state.json` and hands the engine the
/// difference. It is what answers [`hoard_core::ipc::Request::Reload`]: the client
/// says the set changed and the daemon, which owns the state, decides what to
/// watch.
pub async fn reload(engine: &Engine) -> anyhow::Result<usize> {
    let Some(handle) = engine.handle() else {
        anyhow::bail!("the engine isn't running");
    };
    let (state, _path) = CliState::load_default()?;
    let archived = match engine.client() {
        Some(client) => archived_save_ids(&client).await,
        None => HashSet::new(),
    };
    let desired = library::watched_saves_from_state(&state, &archived);
    let current: std::collections::HashSet<String> =
        tokio::time::timeout(STATUS_TIMEOUT, handle.status())
            .await
            .map_err(|_| anyhow::anyhow!("the engine didn't answer in {STATUS_TIMEOUT:?}"))??
            .into_iter()
            .map(|s| s.save_id)
            .collect();
    let desired_ids: std::collections::HashSet<String> =
        desired.iter().map(|s| s.save_id.clone()).collect();

    for save in desired
        .into_iter()
        .filter(|s| !current.contains(&s.save_id))
    {
        handle.add_save(save).await?;
    }
    for gone in current.difference(&desired_ids) {
        handle.remove_save(gone.clone()).await?;
    }
    let watched = desired_ids.len();
    engine.set_watched(watched);
    Ok(watched)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An engine down with a reason and no `Running`, which is how a `note_error`
    /// leaves it. Enough for the two policies below, which only look at the state.
    fn down_with(reason: EngineDownReason, is_cloud: bool) -> Engine {
        let engine = Engine::new();
        {
            let mut guard = engine.lock();
            guard.status.running = false;
            guard.status.reason = reason;
            guard.status.is_cloud = is_cloud;
        }
        engine
    }

    fn up_on(is_cloud: bool) -> Engine {
        let engine = Engine::new();
        {
            let mut guard = engine.lock();
            guard.status.running = true;
            guard.status.is_cloud = is_cloud;
        }
        engine
    }

    fn restart_asked(engine: &Engine) -> bool {
        engine.lock().restart_requested.is_some()
    }

    /// Lending the token proves the session reads. An engine down for not being able
    /// to read it has to retry now, not burn five minutes of backoff next to a session
    /// that already works: the 82 s of 2026-08-28.
    #[test]
    fn a_readable_session_wakes_an_engine_that_was_missing_one() {
        for reason in [
            EngineDownReason::NoSession,
            EngineDownReason::KeyringUnreadable,
            EngineDownReason::SessionExpired,
        ] {
            let engine = down_with(reason, true);
            engine.wake_if_a_session_would_help();
            assert!(
                restart_asked(&engine),
                "{reason:?} is unblocked by a session"
            );
        }
    }

    /// And only those. An engine that went down for something else is not fixed by
    /// somebody managing to read the session, and waking it on every token loan would
    /// turn the backoff into a loop.
    #[test]
    fn a_readable_session_doesnt_wake_an_engine_that_failed_for_another_reason() {
        for reason in [EngineDownReason::Other, EngineDownReason::Unknown] {
            let engine = down_with(reason, true);
            engine.wake_if_a_session_would_help();
            assert!(
                !restart_asked(&engine),
                "{reason:?} is not fixed by a session"
            );
        }
    }

    /// A live engine is left alone: the token is lent constantly, and restarting on
    /// every loan would cut the sync off every few minutes.
    #[test]
    fn a_live_engine_is_never_woken() {
        let engine = up_on(true);
        engine.wake_if_a_session_would_help();
        assert!(!restart_asked(&engine));
    }

    /// Las dos sesiones son independientes y el motor corre contra una sola:
    /// tirarlo porque se fue la otra es un corte gratis y una segunda tanda de
    /// "vigilando" por cada save.
    #[test]
    fn signing_out_of_the_other_session_leaves_the_engine_alone() {
        let engine = up_on(true);
        engine.request_restart_if_signed_out(false, "self-hosted signed out");
        assert!(!restart_asked(&engine), "el motor va con Cloud");

        let engine = up_on(false);
        engine.request_restart_if_signed_out(true, "cloud signed out");
        assert!(!restart_asked(&engine), "el motor va con el self-hosted");
    }

    /// Its own does tear it down: it is talking to a server whose session no longer
    /// exists.
    #[test]
    fn signing_out_of_its_own_session_restarts_the_engine() {
        let engine = up_on(true);
        engine.request_restart_if_signed_out(true, "cloud signed out");
        assert!(restart_asked(&engine));
    }

    /// And an engine that is down restarts whichever session went: the one left may
    /// be exactly the one it was missing.
    #[test]
    fn a_down_engine_restarts_on_either_sign_out() {
        let engine = down_with(EngineDownReason::NoSession, true);
        engine.request_restart_if_signed_out(false, "self-hosted signed out");
        assert!(restart_asked(&engine));
    }

    /// The reason has to survive the context layers the real road puts on it:
    /// `resolve_owned` wraps the error a couple of times before it gets here. If the
    /// classification only looked at the outer layer, the most important case (there
    /// is no session) would come out as `Other` and the window would show the generic
    /// banner again.
    #[test]
    fn no_session_survives_the_context_layers() {
        let err = anyhow::Error::new(hoard_agent::session::NoSession)
            .context("resolving the service session")
            .context("arrancando el motor");
        assert_eq!(classify(&err), EngineDownReason::NoSession);
    }

    /// Las dos formas de fallar del llavero comparten motivo: el consejo al
    /// usuario es el mismo.
    #[test]
    fn both_keyring_failures_read_as_unreadable() {
        let stuck = anyhow::Error::new(hoard_agent::keychain::KeyringTimeout {
            doing: "reading the self-hosted session",
            after: std::time::Duration::from_secs(5),
        })
        .context("reading the session");
        assert_eq!(classify(&stuck), EngineDownReason::KeyringUnreadable);

        let refused =
            anyhow::anyhow!("access denied").context(hoard_agent::keychain::KeyringUnreadable {
                doing: "reading the self-hosted session",
            });
        assert_eq!(classify(&refused), EngineDownReason::KeyringUnreadable);
    }

    /// And what we do not recognise is said not to be recognised, rather than
    /// dressed up as the last reason we can think of: `last_error` carries the detail
    /// and the banner falls back to the generic text, which for an unknown failure is
    /// honest.
    #[test]
    fn anything_else_stays_other() {
        let err = anyhow::anyhow!("the server hung up").context("arrancando el motor");
        assert_eq!(classify(&err), EngineDownReason::Other);
    }
}
