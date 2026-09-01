//! The desktop's link to `hoardd` (ADR 0021, Part A).
//!
//! The desktop used to **embed** the engine (`agent::spawn` inside
//! `start_agent`), with the pidfile in `hoard_agent::instance` arbitrating against
//! the CLI's, and its three design faults: a startup race, no reclaim, and the
//! sync's lifetime tied to a window. The desktop **has no engine** now: it is a
//! client of the service, sends it commands over the local socket and paints what
//! the service reports. Closing the app can no longer stop the sync, which is the
//! whole point.
//!
//! ## Two connections, deliberately
//!
//! - **Commands** ([`DaemonLink::request`]): one lazy connection under a mutex.
//!   Every `#[tauri::command]` that used to touch the `AgentHandle` sends its
//!   request here.
//! - **Events** ([`pump`]): a dedicated connection that only listens.
//!
//! It is not one for convenience: `read_frame` reads header and body in two steps,
//! so it is **not cancel-safe**. A single connection would force a `select!`
//! between "wait for a push" and "send a request", and cancelling the read halfway
//! would leave the stream out of step. Two connections cost the daemon one extra
//! task per client and give us reads that are never cancelled.
//!
//! ## Journal plus push (D.14.2)
//!
//! On connect it asks for the backlog from the cursor and then listens live. The
//! cursor lives **in memory**: a new run of the app starts with an empty UI, so
//! asking for the whole ring is exactly what rebuilds the history it did not see.
//! Within one run, the cursor avoids repeating what was already painted when it
//! reconnects.
//!
//! And when continuity cannot be asserted (the ring lost rows, `gap`; the daemon
//! restarted, a different `epoch`; the push channel left us behind, `Resync`) the
//! UI is told to **resync** instead of stitching it in silence. Faking continuity
//! there is the mute-bell bug all over again.
//!
//! ## Who switches the relay on: the UI, and not before
//!
//! The store calls [`attach`] once it **already** has its `listen()`s in place,
//! and `start_agent` does not. That is deliberate: `start_agent` is also called by
//! the automatic-mode scan, which runs in Rust and can beat the webview's mount,
//! and a backlog emitted before the listener exists is a history lost in silence,
//! exactly the bug the journal exists not to have. With the relay tied to the
//! subscription, the first emission cannot arrive early. (The command connection is
//! independent: the scan brings the service up anyway, only with nobody listening
//! yet.)

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use hoard_agent::agent::{AgentConfig, AgentEvent};
use hoard_agent::state::CliState;
use hoard_agent::supervisor::{self, Finished};
use hoard_core::ipc::{
    AdoptedSession, AgentSlotStatus, CloudToken, DaemonStatus, EngineDownReason, IpcError,
    KeyringFault, Payload, Request, ServerSession, UpdateState,
};
use hoardd::client::{Client, Push};
use hoardd::endpoint::Endpoint;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::task::JoinHandle;

use crate::state::AppState;

/// How we introduce ourselves in the daemon's log.
fn client_name(role: &str) -> String {
    format!("hoard-desktop {} ({role})", env!("CARGO_PKG_VERSION"))
}

/// The wait between reconnects of the event pump. The normal case is a daemon that
/// stays alive and never reaches this; it covers the service restarting (an update)
/// without leaving the UI mute for ever.
const RECONNECT_DELAY: Duration = Duration::from_secs(3);

/// The wait between retries when the service was stopped **on purpose**. We no
/// longer relaunch it, so retrying fast only fills the log; it keeps retrying so it
/// attaches on its own if somebody starts it.
const STOPPED_RETRY_DELAY: Duration = Duration::from_secs(30);

/// The ceiling on an already-connected request. Generous for the real worst case
/// (the `Status` asks the engine, which may be hashing), but finite: without it a
/// stuck service would hang the UI's command and, behind it, everything else waiting
/// on the command connection.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

/// How often the engine's state is asked for again.
///
/// The engine being up or down is **not a journal event** (`AgentEvent` has no such
/// variant), so without this the UI would be stuck with the state of the instant it
/// connected: an engine that starts 20 s later (the normal case, since the daemon
/// resolves the session first) would leave the icon on "stopped" for the whole
/// session. It is one round-trip over a local socket; the push is still what brings
/// the events.
const STATUS_EVERY: Duration = Duration::from_secs(20);

/// What the UI knows as `AgentStatus`. Its shape is a contract with the stores
/// (`ui/src/lib/stores/agent.ts`), which must not notice that behind it there is a
/// service rather than an embedded engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AgentStatus {
    pub running: bool,
    pub watched_count: usize,
    /// The service sends the OS's native notifications itself (ADR 0021 D.14.1),
    /// so the UI must **not** send its own or the user would see the notice twice
    /// with the app open. On a platform where the service cannot notify yet
    /// (Windows, macOS) this arrives as `false` and the notice stays the
    /// frontend's.
    pub service_notifies: bool,
    /// **Why** there is no engine, when there is none. This used to die here: the
    /// daemon had it typed, this struct threw it away, and the window could only say
    /// "the service is disconnected", which left two self-hosted users days without
    /// backups and no way to know that what they were missing was the session. What
    /// the UI paints comes from here.
    pub reason: EngineDownReason,
    /// The last failure's raw text, for the detail view and so the user can copy it
    /// into a report. The translated sentence comes from `reason`.
    pub last_error: Option<String>,
    /// Which way the keyring failed, when `reason` is `KeyringUnreadable`. One
    /// reason, four next steps: a machine with no secret-service daemon is not a
    /// locked one, and telling that user to unlock their login keyring sends them
    /// after something that isn't installed.
    pub keyring: Option<KeyringFault>,
}

impl AgentStatus {
    /// What the UI should paint when we know nothing about the engine.
    ///
    /// `service_notifies: false` is the safe default: with no service no events
    /// arrive either, so there is nothing to duplicate, and the worst possible case
    /// is a repeated notice, never a lost one.
    pub fn down() -> Self {
        Self {
            running: false,
            watched_count: 0,
            service_notifies: false,
            // We never reached the service, so we don't know whether the engine
            // is up either: `Unreachable` says that and nothing more. This used
            // to be `Unknown`, which in the window is the sentence "the sync
            // service is stopped": a claim we have no grounds for, and one
            // that on 2026-08-28 was simply false: the service had been up for
            // thirteen hours.
            reason: EngineDownReason::Unreachable,
            last_error: None,
            keyring: None,
        }
    }

    /// Translates the daemon's state into what the UI knows. One place only: there
    /// were two hand-rolled constructions and the state loop's forgot half the new
    /// fields.
    pub fn from_daemon(status: &hoard_core::ipc::DaemonStatus) -> Self {
        Self {
            running: status.engine.running,
            watched_count: status.slots.len().max(status.engine.watched),
            service_notifies: status.notifications,
            reason: status.engine.reason,
            // Only when something is broken: the last error of an engine that is
            // already up is noise the window must not show.
            last_error: (!status.engine.running)
                .then(|| status.engine.last_error.clone())
                .flatten(),
            keyring: (!status.engine.running)
                .then_some(status.engine.keyring)
                .flatten(),
        }
    }
}

/// Una fila del journal camino de la UI.
#[derive(Debug, Clone, Serialize)]
pub struct BacklogRow {
    /// The row's identity within this run of the daemon. The main window does not
    /// need it (it stitches the feed event by event) but any surface that **re-reads**
    /// the whole snapshot does: with no stable key, every re-read would be a new list
    /// as far as Svelte is concerned.
    pub seq: u64,
    /// When it happened, in epoch milliseconds. It is here because **the time
    /// matters** when replaying: a `game_started` from two hours ago has to paint two
    /// hours of session, not start the counter from zero. It is the last occurrence
    /// (`last_at`), which on a collapsed row is the one still true.
    pub at: i64,
    pub event: AgentEvent,
}

/// Backlog del journal, tal como lo recibe la UI.
#[derive(Debug, Clone, Serialize)]
struct BacklogPayload {
    /// In chronological order (oldest first), as it came out of the journal.
    rows: Vec<BacklogRow>,
    /// There is no continuity to respect: the client should rebuild its state from
    /// this backlog rather than patching the one it had.
    resync: bool,
}

impl From<hoard_core::ipc::JournalEntry> for BacklogRow {
    fn from(entry: hoard_core::ipc::JournalEntry) -> Self {
        Self {
            seq: entry.seq,
            at: entry.last_at.unix_timestamp() * 1000,
            event: entry.event,
        }
    }
}

/// How many journal rows are kept here for whoever arrives late.
///
/// The daemon's ring holds 1024; this is only the local mirror of what has been
/// relayed, and whoever reads it trims to `MAX_FEED_ENTRIES` (80). A hundred and
/// twenty leave room for the rows that are not feed rows without fattening the
/// snapshot that crosses the webview bridge whole on every read.
const JOURNAL_MIRROR: usize = 120;

/// Estado del bucle de nube tal como lo ve la UI. Es el mismo vocabulario que
/// `CloudStatus` en `stores/live.ts`: quien lo pinta no traduce nada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CloudPulse {
    /// No pass has finished yet.
    #[default]
    Unknown,
    Online,
    Offline,
    Throttled,
}

/// What the UI knows **right now**, without asking anybody.
///
/// It exists for the surfaces that only read. The main window builds its state by
/// listening: it switches the relay on, receives the backlog once, and stitches the
/// events that arrive. A window born later, the HUD, cannot do that: the backlog has
/// already been emitted, [`attach`] is idempotent and [`emit_status`] only speaks
/// when something changes, so listening would give it a blank panel and a red header
/// with the service alive.
///
/// So it does not listen: it reads this. It is a copy of what this process already
/// had in memory (no I/O, no network, no requests to the service), and that is why
/// opening the HUD cannot start, wake or alter anything.
#[derive(Debug, Clone, Serialize)]
pub struct UiSnapshot {
    pub status: AgentStatus,
    /// What the service says about each watched save: whether the game is running
    /// and when its next backup is due.
    ///
    /// It is kept apart from the journal on purpose. Who is playing and which backup
    /// is coming are **state**, not history: rebuilding them by replaying events means
    /// keeping the `game_started` for ever, and the day it falls out of the ring the
    /// HUD will say nobody is playing with the game right there. The service already
    /// keeps the tally; this copies it.
    pub slots: Vec<AgentSlotStatus>,
    /// In chronological order, oldest first (like the backlog).
    pub rows: Vec<BacklogRow>,
    pub cloud: CloudPulse,
    /// Segundos hasta que se levante el freno, cuando `cloud == Throttled`.
    pub cloud_retry_in: Option<u32>,
}

/// The journal's cursor within **one** run of the daemon.
#[derive(Debug, Clone)]
struct Cursor {
    /// The daemon run's identity. A `seq` is only comparable within the same epoch:
    /// when it changes, the daemon restarted and the cursor is worthless.
    epoch: String,
    seq: u64,
}

/// Estado del enlace, vivo mientras viva la app.
#[derive(Default)]
pub struct DaemonLink {
    /// The command connection. Lazy: it is not opened until there is something to
    /// ask for, and it reopens on its own when the daemon restarts.
    cmd: tokio::sync::Mutex<Option<Client>>,
    /// The event pump and the status refresh. Non-empty while the UI is listening
    /// (between [`attach`] and [`detach`]).
    tasks: Mutex<Vec<JoinHandle<()>>>,
    cursor: Arc<Mutex<Option<Cursor>>>,
    /// The last state published to the UI. It lives here and not inside the state
    /// loop because there are **two** emitters (the loop, and the pump, which knows
    /// before anybody else that the socket dropped) and with one memory per emitter
    /// the second never learns what the first published: the pump paints "stopped",
    /// the loop still believes it published "up", and the UI stays stopped until the
    /// engine changes state on its own.
    last_status: Mutex<Option<AgentStatus>>,
    /// A mirror of the last relayed rows, for whoever arrives late ([`UiSnapshot`]).
    /// It is written in the same gesture that emits them: what is here is exactly
    /// what the main window received, no more and no less.
    journal: Mutex<std::collections::VecDeque<BacklogRow>>,
    /// The last slots the service reported, from the same `Status`
    /// [`Self::last_status`] comes from.
    slots: Mutex<Vec<AgentSlotStatus>>,
    /// The cloud loop's last pulse; it lives in `commands::cloud_pull` and emits on
    /// its own. The same reason as the journal: its events are momentary and whoever
    /// was not listening cannot get them back.
    cloud: Mutex<(CloudPulse, Option<u32>)>,
}

impl DaemonLink {
    /// The user's endpoint (or `HOARDD_SOCKET`'s). Resolved on every use: it is one
    /// environment read and a path join, and that way an override is never cached
    /// from an earlier start.
    fn endpoint() -> Result<Endpoint> {
        Endpoint::resolve().context("resolving the hoardd endpoint")
    }

    /// Sends a request to the daemon, bringing it up when there is none.
    ///
    /// A **transport** failure drops the connection and retries once: the real case
    /// is a service that updated and restarted between two commands, and whoever
    /// pressed the button has no reason to find out.
    ///
    /// An [`IpcError`] is **not** retried: that is a healthy daemon answering "I
    /// cannot, and here is why". Retrying it would reconnect on every `EngineDown`,
    /// two connections and two log lines per command while there is no engine, only
    /// to receive exactly the same answer.
    pub async fn request(&self, request: Request) -> Result<Payload> {
        match self.request_once(request.clone()).await {
            Ok(payload) => Ok(payload),
            Err(err) if err.downcast_ref::<IpcError>().is_some() => Err(err),
            Err(err) => {
                tracing::debug!(error = %format!("{err:#}"), "daemon: retrying on a fresh connection");
                *self.cmd.lock().await = None;
                self.request_once(request).await
            }
        }
    }

    async fn request_once(&self, request: Request) -> Result<Payload> {
        let mut guard = self.cmd.lock().await;
        if guard.is_none() {
            let endpoint = Self::endpoint()?;
            // `ensure_running` does not check "is there a daemon?" before
            // launching: that is a TOCTOU and produces two engines. It launches and
            // reconnects; if two clients do it at once, one wins the bind and the
            // other exits.
            *guard = Some(
                Client::ensure_running(&endpoint, &client_name("commands"))
                    .await
                    .with_context(|| format!("connecting to the Hoard service at {endpoint}"))?,
            );
        }
        let client = guard.as_mut().expect("just connected");
        // With a ceiling: a service that accepts the connection and then does not
        // answer would hang the button that fired it **for ever**, and with it every
        // command waiting on this mutex. Cutting the read halfway puts the stream out
        // of step, so the connection is dropped in the same gesture.
        let result = match tokio::time::timeout(REQUEST_TIMEOUT, client.request(request)).await {
            Ok(result) => result,
            Err(_) => Err(anyhow!(
                "the Hoard service didn't answer in {}s",
                REQUEST_TIMEOUT.as_secs()
            )),
        };
        if result
            .as_ref()
            .err()
            .is_some_and(|err| err.downcast_ref::<IpcError>().is_none())
        {
            // A failure that is not the protocol's is almost always the connection,
            // so let the next request start by reconnecting. An `IpcError` is not one
            // (the connection worked and brought back an answer) and dropping it
            // would mean reconnecting for every command that arrives with the engine
            // down.
            *guard = None;
        }
        result
    }

    /// Estado del daemon: motor, slots y cursor.
    pub async fn status(&self) -> Result<DaemonStatus> {
        match self.request(Request::Status).await? {
            Payload::Status(status) => Ok(status),
            other => Err(anyhow!("unexpected answer to status: {other:?}")),
        }
    }

    /// How the update is going, according to the service.
    ///
    /// The window does not look at GitHub for this: **the updater belongs to the
    /// service** (it is the only thing that is always there, so it is the only thing
    /// that can update a machine whose app has been closed for two weeks). What
    /// happens here is reading what it already knows and painting it.
    pub async fn update_state(&self) -> Result<UpdateState> {
        match self.request(Request::UpdateStatus).await? {
            Payload::Update(state) => Ok(state),
            other => Err(anyhow!("unexpected answer to update status: {other:?}")),
        }
    }

    /// Tells the service to apply whatever it has downloaded, now.
    ///
    /// The window asks because **there is somebody in front of it**, and that is the
    /// whole difference: with a human at the keyboard the service can open the
    /// privilege dialog its background cycle cannot. It returns straight away;
    /// applying carries on and is followed with `update_state`.
    pub async fn apply_update(&self, version: Option<String>) -> Result<UpdateState> {
        match self.request(Request::ApplyUpdate { version }).await? {
            Payload::Update(state) => Ok(state),
            other => Err(anyhow!("unexpected answer to apply update: {other:?}")),
        }
    }

    /// "Not now", for `hours`. It does not move the deadline.
    pub async fn snooze_update(&self, hours: u32) -> Result<UpdateState> {
        match self.request(Request::SnoozeUpdate { hours }).await? {
            Payload::Update(state) => Ok(state),
            other => Err(anyhow!("unexpected answer to snooze update: {other:?}")),
        }
    }

    /// Borrows a valid Cloud token from the service.
    ///
    /// **The desktop no longer rotates.** The service is the only thing that touches
    /// `cloud.toml`'s refresh token (ADR 0021, Part A: "one rotator only"), so here a
    /// token is borrowed and used. Two processes rotating the same refresh token is
    /// the root cause of the 401 and mute-realtime family: GoTrue revokes the whole
    /// family when it detects the reuse, and that does not recover even on a restart.
    ///
    /// `rejected` is the token we just took a 401 with: without it, a token revoked
    /// server-side but still "fresh" would come back again and again.
    pub async fn cloud_token(&self, rejected: Option<String>) -> Result<CloudToken> {
        match self.request(Request::CloudToken { rejected }).await? {
            Payload::CloudToken(token) => {
                // While we are at it, it is left in place for the log shipper, which
                // runs on its own thread and cannot ask for anything over IPC. This is
                // the only place in the desktop a fresh Cloud token comes in.
                hoard_agent::credentials::set_lent_cloud(Some(
                    hoard_agent::credentials::CloudLease {
                        url: token.server_url.clone(),
                        token: token.access_token.clone(),
                    },
                ));
                Ok(token)
            }
            other => Err(anyhow!("unexpected answer to cloud_token: {other:?}")),
        }
    }

    /// Hands the service the Cloud session the OAuth has just minted.
    ///
    /// The desktop does not write it: the secret's owner is the service. On macOS
    /// that is the difference between an app that works and one that asks for the
    /// keyring password every few seconds: the item is only authorised for the binary
    /// that created it, and what reads it (the engine) lives in `hoardd`.
    pub async fn adopt_session(&self, session: AdoptedSession) -> Result<()> {
        match self.request(Request::AdoptSession { session }).await? {
            Payload::Ack => Ok(()),
            other => Err(anyhow!("unexpected answer to adopt_session: {other:?}")),
        }
    }

    /// Tells the service to forget the Cloud session (logout).
    pub async fn forget_session(&self) -> Result<()> {
        match self.request(Request::ForgetSession).await? {
            Payload::Ack => Ok(()),
            other => Err(anyhow!("unexpected answer to forget_session: {other:?}")),
        }
    }

    /// Hands the service the self-hosted session the app has just validated.
    pub async fn adopt_server_session(&self, session: ServerSession) -> Result<()> {
        match self
            .request(Request::AdoptServerSession { session })
            .await?
        {
            Payload::Ack => Ok(()),
            other => Err(anyhow!(
                "unexpected answer to adopt_server_session: {other:?}"
            )),
        }
    }

    /// Tells the service to forget the self-hosted session (logout).
    pub async fn forget_server_session(&self) -> Result<()> {
        match self.request(Request::ForgetServerSession).await? {
            Payload::Ack => Ok(()),
            other => Err(anyhow!(
                "unexpected answer to forget_server_session: {other:?}"
            )),
        }
    }

    /// Borrows the own-server session: URL, token and who you are.
    ///
    /// A `hoard_v1_` token is static (it neither expires nor rotates), so this is
    /// asked for once and kept in `hoard_agent::credentials`' slot so the log shipper
    /// sees it too, since it cannot ask for anything over IPC.
    pub async fn server_session(&self) -> Result<ServerSession> {
        match self.request(Request::ServerToken).await? {
            Payload::ServerSession(session) => Ok(session),
            other => Err(anyhow!("unexpected answer to server_token: {other:?}")),
        }
    }

    /// A best-effort request: it logs and carries on. For the places where a failure
    /// must not abort what the user asked for (persisting a preference, re-hydrating
    /// after adding a game).
    pub async fn tell(&self, what: &'static str, request: Request) {
        if let Err(err) = self.request(request).await {
            tracing::warn!(error = %format!("{err:#}"), "daemon: couldn't {what}");
        }
    }

    /// The watched save set changed on disk, so the daemon should re-read it. The
    /// client **announces**, it does not send the list: the service owns the state.
    pub async fn notify_reload(&self) {
        self.tell("ask the service to reload its watch list", Request::Reload)
            .await;
    }

    fn cursor(&self) -> Option<Cursor> {
        self.cursor.lock().unwrap().clone()
    }

    fn set_cursor(&self, epoch: &str, seq: u64) {
        *self.cursor.lock().unwrap() = Some(Cursor {
            epoch: epoch.to_string(),
            seq,
        });
    }

    /// Stores an already-relayed row. `resync` treats it as what it says it is:
    /// there is no continuity to respect, so what came before is dropped rather than
    /// stitched onto the new, exactly as the stores do.
    fn remember(&self, rows: &[BacklogRow], resync: bool) {
        let mut journal = self.journal.lock().unwrap();
        if resync {
            journal.clear();
        }
        for row in rows {
            journal.push_back(row.clone());
        }
        while journal.len() > JOURNAL_MIRROR {
            journal.pop_front();
        }
    }

    /// Records the cloud loop's pulse. `commands::cloud_pull` calls it in the same
    /// gesture it emits, so the mirror cannot tell a different story from the one the
    /// main window heard.
    pub fn note_cloud(&self, pulse: CloudPulse, retry_in: Option<u32>) {
        *self.cloud.lock().unwrap() = (pulse, retry_in);
    }

    /// Records the last `Status`'s slots. With an empty list when there is no
    /// service: not knowing is data, and pretending the last ones we saw still hold
    /// would paint "playing" over an engine that is watching nothing.
    fn note_slots(&self, slots: &[AgentSlotStatus]) {
        *self.slots.lock().unwrap() = slots.to_vec();
    }

    /// Todo lo que este proceso sabe, copiado. Sin E/S y sin tocar el servicio:
    /// ver [`UiSnapshot`].
    pub fn snapshot(&self) -> UiSnapshot {
        let (cloud, cloud_retry_in) = *self.cloud.lock().unwrap();
        UiSnapshot {
            status: self
                .last_status
                .lock()
                .unwrap()
                .clone()
                .unwrap_or_else(AgentStatus::down),
            slots: self.slots.lock().unwrap().clone(),
            rows: self.journal.lock().unwrap().iter().cloned().collect(),
            cloud,
            cloud_retry_in,
        }
    }
}

/// Starts relaying the service's events to the UI. The store calls it once its
/// listeners are in place. Idempotent.
pub fn attach(app: &AppHandle) {
    let link = app.state::<AppState>();
    let mut tasks = link.daemon.tasks.lock().unwrap();
    if !tasks.is_empty() {
        return;
    }
    // D.12's rule: if it outlives a request, it runs supervised. A panic here would
    // leave the UI mute with not one line of log, which is exactly the invisible
    // failure that cost two sessions.
    tasks.push(tokio::spawn({
        let app = app.clone();
        supervisor::supervise("desktop daemon event pump", move || pump(app.clone()))
    }));
    tasks.push(tokio::spawn({
        let app = app.clone();
        supervisor::supervise("desktop daemon status", move || status_loop(app.clone()))
    }));
}

/// Stops relaying events: it stops the tasks and closes the event connection.
///
/// It does **not** send `Shutdown` and does not touch the engine. The desktop being
/// able to stop the service would be going back to a lifetime tied to a window;
/// stopping the sync is an explicit user order (`hoard sync stop`), not a side
/// effect of signing out or closing the app.
///
/// The cursor is **kept**: if the UI subscribes again within the same run, asking
/// from it avoids repeating what it already painted.
pub fn detach(app: &AppHandle) {
    let state = app.state::<AppState>();
    let tasks: Vec<JoinHandle<()>> = std::mem::take(&mut *state.daemon.tasks.lock().unwrap());
    for task in tasks {
        task.abort();
    }
}

/// Connects, asks for the backlog and forwards the live push. It never returns: a
/// connection that drops is retried, because the service can restart (an update)
/// with the app having no other way to find out.
async fn pump(app: AppHandle) -> Finished {
    loop {
        if let Err(err) = pump_once(&app).await {
            tracing::warn!(error = %format!("{err:#}"), "daemon: the event stream ended");
        }
        // The UI must not be left believing the engine is still there: if we lost
        // the socket, we know nothing about it.
        app.state::<AppState>().daemon.note_slots(&[]);
        emit_status(&app, &AgentStatus::down());
        tokio::time::sleep(reconnect_delay()).await;
    }
}

/// How long to wait before trying again. With the service stopped on purpose there
/// is no hurry: nobody will answer until somebody starts it, and we no longer start
/// it. Probing every 3 s would only fill the log.
fn reconnect_delay() -> Duration {
    if hoardd::client::stopped_on_purpose() {
        STOPPED_RETRY_DELAY
    } else {
        RECONNECT_DELAY
    }
}

async fn pump_once(app: &AppHandle) -> Result<()> {
    let endpoint = DaemonLink::endpoint()?;
    let mut client = Client::ensure_running(&endpoint, &client_name("events"))
        .await
        .with_context(|| format!("connecting to the Hoard service at {endpoint}"))?;
    let epoch = client.welcome().epoch.clone();
    tracing::info!(
        pid = client.welcome().pid,
        version = %client.welcome().daemon_version,
        "desktop: attached to the Hoard service"
    );

    let state = app.state::<AppState>();
    // A cursor from another daemon run is not a cursor: asking from it would leave
    // the UI waiting for events that already happened.
    let since = state
        .daemon
        .cursor()
        .filter(|c| c.epoch == epoch)
        .map(|c| c.seq);
    let fresh = since.is_none();
    // With a ceiling, like the commands: if the service accepts and then goes quiet,
    // the UI would be left with no backlog and no push, and no line saying so. On
    // failure the loop above drops this connection and reconnects.
    let backlog = tokio::time::timeout(REQUEST_TIMEOUT, client.subscribe(since))
        .await
        .map_err(|_| anyhow!("the Hoard service didn't answer the subscribe"))??;
    state.daemon.set_cursor(&epoch, backlog.cursor);
    let resync = fresh || backlog.gap;
    if backlog.gap {
        tracing::warn!(
            requested = since.unwrap_or(0),
            cursor = backlog.cursor,
            "desktop: the service no longer has the journal rows we asked for; resyncing"
        );
    }
    emit_backlog(
        app,
        backlog.entries.into_iter().map(BacklogRow::from).collect(),
        resync,
    );

    while let Some(push) = client.next_push().await? {
        match push {
            Push::Event(entry) => {
                state.daemon.set_cursor(&epoch, entry.seq);
                // Store and emit in the same gesture, as the daemon's journal does:
                // separate them and one day one of the two gets forgotten, and the
                // surface reading the snapshot ends up telling a different story from
                // the one the main window heard.
                let row = BacklogRow::from(entry);
                state.daemon.remember(std::slice::from_ref(&row), false);
                emit_event(app, &row.event);
            }
            // We lagged and the channel dropped rows. The daemon confesses instead
            // of leaving the gap invisible, and we ask again from our cursor.
            Push::Resync { cursor, dropped } => {
                tracing::warn!(
                    dropped,
                    cursor,
                    "desktop: fell behind the service's event push; re-reading the journal"
                );
                let since = state.daemon.cursor().map(|c| c.seq);
                let backlog = tokio::time::timeout(REQUEST_TIMEOUT, client.subscribe(since))
                    .await
                    .map_err(|_| anyhow!("the Hoard service didn't answer the re-subscribe"))??;
                state.daemon.set_cursor(&epoch, backlog.cursor);
                emit_backlog(
                    app,
                    backlog.entries.into_iter().map(BacklogRow::from).collect(),
                    true,
                );
            }
            // It was stopped on purpose (`hoard sync stop`, `systemctl --user
            // stop`). The client has already noted the farewell, so the retry below
            // will only look for it coming back: closing this window cannot stop the
            // sync, but opening it cannot undo an order to stop it either.
            Push::Goodbye { reason } => {
                tracing::info!(reason, "desktop: the Hoard service was stopped on purpose");
                return Ok(());
            }
        }
    }
    Ok(())
}

/// Asks for the state again and publishes it when it changes. The first pass runs
/// without waiting: it is what puts the watcher's dot in the right place right after
/// the UI subscribes.
async fn status_loop(app: AppHandle) -> Finished {
    let mut armed: HashSet<String> = HashSet::new();
    loop {
        let state = app.state::<AppState>();
        match state.daemon.status().await {
            Ok(status) => {
                announce_slots(&app, &status.slots, &mut armed);
                state.daemon.note_slots(&status.slots);
                let now = AgentStatus::from_daemon(&status);
                if !now.running {
                    tracing::debug!(
                        reason = status.engine.last_error.as_deref().unwrap_or("starting"),
                        "desktop: the service has no engine"
                    );
                }
                emit_status(&app, &now);
            }
            // With no service we know nothing about the engine, so the UI must not
            // believe it is still there: the icon cannot stay green out of inertia.
            Err(err) => {
                tracing::debug!(error = %format!("{err:#}"), "desktop: couldn't read the service status");
                state.daemon.note_slots(&[]);
                emit_status(&app, &AgentStatus::down());
            }
        }
        tokio::time::sleep(STATUS_EVERY).await;
    }
}

/// Publishes the engine's state in the shape the UI already knows, **when it
/// changed**. Repeating it every 20 s would wake the webview to say nothing.
pub fn emit_status(app: &AppHandle, status: &AgentStatus) {
    let state = app.state::<AppState>();
    {
        let mut last = state.daemon.last_status.lock().unwrap();
        if last.as_ref() == Some(status) {
            return;
        }
        *last = Some(status.clone());
    }
    let _ = app.emit("agent://daemon-status", status);
}

/// Announces the slots the service says it is watching.
///
/// The client resolves the slug against `state.json` because it is presentation: the
/// daemon reports by `save_id`, which is their real identity.
pub fn announce_slots(app: &AppHandle, slots: &[AgentSlotStatus], seen: &mut HashSet<String>) {
    let fresh: Vec<&AgentSlotStatus> = slots
        .iter()
        .filter(|s| !seen.contains(&s.save_id))
        .collect();
    if fresh.is_empty() {
        return;
    }
    let slugs = CliState::load_default()
        .map(|(state, _)| state)
        .ok()
        .map(|state| {
            state
                .saves
                .into_iter()
                .map(|(id, save)| (id, save.game_slug))
                .collect::<std::collections::HashMap<_, _>>()
        })
        .unwrap_or_default();
    for slot in fresh {
        seen.insert(slot.save_id.clone());
        let _ = app.emit(
            "agent://watcher-armed",
            WatcherArmed {
                save_id: slot.save_id.clone(),
                game_slug: slugs
                    .get(&slot.save_id)
                    .cloned()
                    .unwrap_or_else(|| slot.display_name.clone()),
            },
        );
    }
}

#[derive(Debug, Clone, Serialize)]
struct WatcherArmed {
    save_id: String,
    game_slug: String,
}

fn emit_backlog(app: &AppHandle, rows: Vec<BacklogRow>, resync: bool) {
    // El espejo se escribe **antes** del early-return y aunque no haya filas: un
    // `resync` sin nada que traer sigue siendo la orden de tirar lo de antes.
    app.state::<AppState>().daemon.remember(&rows, resync);
    if rows.is_empty() && !resync {
        return;
    }
    tracing::debug!(
        count = rows.len(),
        resync,
        "desktop: seeding the UI from the service's journal"
    );
    let _ = app.emit("agent://backlog", BacklogPayload { rows, resync });
}

/// Forwards an engine event to the UI over its usual Tauri channel.
///
/// This mapping is the contract with the stores: the backend changes (an embedded
/// engine becomes a service) without the screens finding out, which is D.3's hard
/// constraint. Only the **live** events come through here; the backlog goes down its
/// own channel so a recovered history fires no toasts and no scans.
fn emit_event(app: &AppHandle, ev: &AgentEvent) {
    let topic = match ev {
        AgentEvent::GameStarted { .. } => "agent://game-started",
        AgentEvent::GameStopped { .. } => "agent://game-stopped",
        AgentEvent::BackupScheduled { .. } => "agent://backup-scheduled",
        AgentEvent::BackupStarted { .. } => "agent://backup-started",
        AgentEvent::BackupSuccess { .. } => "agent://backup-success",
        AgentEvent::BackupFailed { .. } => "agent://backup-failed",
        AgentEvent::BackupThrottled { .. } => "agent://backup-throttled",
        AgentEvent::BackupTooLarge { .. } => "agent://backup-too-large",
        AgentEvent::BackupQuotaFull { .. } => "agent://backup-quota-full",
        AgentEvent::BackupTrimmed { .. } => "agent://backup-trimmed",
        AgentEvent::BackupFilesUnreadable { .. } => "agent://backup-files-unreadable",
        AgentEvent::BackupNeedsAttention { .. } => "agent://backup-needs-attention",
        AgentEvent::BackupAttentionCleared { .. } => "agent://backup-attention-cleared",
        AgentEvent::SaveAutoRestored { .. } => "agent://save-auto-restored",
        AgentEvent::SaveAutoRestoreFailed { .. } => "agent://save-auto-restore-failed",
        AgentEvent::BackupSkippedEmpty { .. } => "agent://backup-skipped-empty",
        AgentEvent::SaveConflictsBackedUp { .. } => "agent://save-conflicts-backed-up",
        AgentEvent::HeavyProcessDetected { .. } => "agent://heavy-process-detected",
        AgentEvent::RestoreDeferred { .. } => "agent://restore-deferred",
        AgentEvent::SaveAutoRestoreStuck { .. } => "agent://save-auto-restore-stuck",
        AgentEvent::SaveAutoRestoreRecovered { .. } => "agent://save-auto-restore-recovered",
    };
    let _ = app.emit(topic, ev);

    // A heavy untracked game has just appeared, so bring the scan forward instead of
    // waiting for the timer. `request_scan` does nothing when automatic mode is off,
    // and it groups bursts.
    if let AgentEvent::HeavyProcessDetected { name } = ev {
        tracing::info!(process = %name, "desktop: heavy untracked game suspected; requesting immediate scan");
        crate::commands::automatic::request_scan(app.clone());
    }

    // Aliases with semantic names for the LiveStatus and ActivityFeed surface. The
    // same payload on a more readable channel; the original channels stay alive.
    match ev {
        AgentEvent::BackupStarted { .. } => {
            let _ = app.emit("agent://upload-started", ev);
        }
        AgentEvent::BackupSuccess { .. } => {
            let _ = app.emit("agent://upload-completed", ev);
        }
        AgentEvent::BackupScheduled {
            reason: hoard_agent::agent::BackupReason::FilesystemSettled,
            delay_ms,
            ..
        } if *delay_ms > debounce_ms() => {
            // Only when the min-interval pushed the upload past the debounce is
            // there a real wait worth showing; the routine debounce of every autosave
            // is not "queued, waiting".
            //
            // This arm was dead until Aug 2026: the only `BackupScheduled` emitted
            // came from the debounce timer and its `delay_ms` was exactly the
            // debounce, so it never exceeded it. The engine now also announces the
            // floor's wait (`agent::announce_backup_wait`, 60 s minimum), which is
            // the one that really has to be shown.
            let _ = app.emit("agent://throttled", ev);
        }
        _ => {}
    }
}

/// The debounce the service's engine runs with. The daemon builds its `AgentConfig`
/// with `..AgentConfig::default()` for this field, so the default **is** the live
/// value; if it ever becomes configurable, this has to come from the `Status`
/// instead of being computed here.
fn debounce_ms() -> u64 {
    AgentConfig::default().debounce_secs.saturating_mul(1000)
}
