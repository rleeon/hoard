//! Live presence for the Eye panel: heartbeats to
//! `/v1/presence/heartbeat`.
//!
//! One implementation shared by both frontends (desktop and CLI daemon): it is
//! spawned alongside the agent, the `AgentEvent::GameStarted` and `GameStopped`
//! their event loops already consume get forwarded to it, and:
//!
//! - An immediate beat when the reported game changes, so starting a game on this
//!   machine shows up in every other machine's Eye panel in a second or two (the
//!   server pushes the `devices` UPDATE over Supabase Realtime).
//! - A keepalive every [`KEEPALIVE_SECS`], because the server ages a device out
//!   after 90 s with no beat (three missed), so a crash expires on its own.
//! - A final `closing` beat on an orderly shutdown, so the dot goes out at once.
//!
//! All best-effort: a failed beat is logged at debug and the next tick retries.
//! The gate is the capability the server advertises, so nothing is emitted
//! against an older one.
//!
//! On self-hosted this talks to neither Hoard nor anybody outside: the beats go
//! to the user's own server and only they see them.

use std::collections::HashMap;
use std::time::Duration;

use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio::time::{interval, Instant, MissedTickBehavior};

use crate::api::ApiClient;

/// Keepalive cadence. The server-side "online" threshold is 90 s, so this leaves
/// room for missed beats.
/// perdidos; si tocas uno, toca el otro.
const KEEPALIVE_SECS: u64 = 30;

/// The longest a `closing()` waits for its beat before giving up: quitting the
/// app must never hang on a dead network.
const CLOSING_TIMEOUT_SECS: u64 = 3;

enum Cmd {
    Started { slug: String },
    Stopped { slug: String },
    Closing { done: oneshot::Sender<()> },
}

/// A cheap handle to clone. The frontends call it from their event loops.
#[derive(Clone)]
pub struct PresenceHandle {
    tx: mpsc::Sender<Cmd>,
}

impl PresenceHandle {
    /// A tracked game started. `try_send` on purpose: if the channel is full we
    /// lose a beat rather than ever blocking the frontend's event loop.
    pub fn game_started(&self, slug: impl Into<String>) {
        let _ = self.tx.try_send(Cmd::Started { slug: slug.into() });
    }

    /// A tracked game stopped.
    pub fn game_stopped(&self, slug: impl Into<String>) {
        let _ = self.tx.try_send(Cmd::Stopped { slug: slug.into() });
    }

    /// Latido final en el shutdown ordenado: marca este device offline ya.
    /// A bounded wait ([`CLOSING_TIMEOUT_SECS`]), callable from the quit path.
    /// quit sin miedo a colgarlo.
    pub async fn closing(&self) {
        let (done_tx, done_rx) = oneshot::channel();
        if self.tx.send(Cmd::Closing { done: done_tx }).await.is_ok() {
            let _ = tokio::time::timeout(Duration::from_secs(CLOSING_TIMEOUT_SECS), done_rx).await;
        }
    }
}

/// Starts the presence task on the agent's own `ApiClient` (the desktop rotates
/// the JWT). The task dies on its own once every handle is dropped, or
/// inmediatamente tras el beat de un `closing()`.
pub fn spawn(api: ApiClient) -> (PresenceHandle, JoinHandle<()>) {
    let (tx, rx) = mpsc::channel(64);
    let task = tokio::spawn(run(api, rx));
    (PresenceHandle { tx }, task)
}

async fn run(api: ApiClient, mut rx: mpsc::Receiver<Cmd>) {
    // slug to (refcount, start). A refcount because two tracked saves of the same
    // game must not double-count, and the entry only goes when both drop. The
    // FULL list is reported on every beat, so playing two games at once draws two
    // rows in the other machines' Eye panels.
    let mut running: HashMap<String, (u32, Instant)> = HashMap::new();

    let mut tick = interval(Duration::from_secs(KEEPALIVE_SECS));
    tick.set_missed_tick_behavior(MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            cmd = rx.recv() => match cmd {
                None => break,
                Some(Cmd::Started { slug }) => {
                    let entry = running.entry(slug).or_insert((0, Instant::now()));
                    entry.0 += 1;
                    // Latido inmediato solo cuando el SET visible cambia (el
                    // the game has just appeared); the same game's second save
                    // changes nothing anybody can see.
                    if entry.0 == 1 {
                        beat(&api, &running, false).await;
                    }
                }
                Some(Cmd::Stopped { slug }) => {
                    if let Some(entry) = running.get_mut(&slug) {
                        entry.0 = entry.0.saturating_sub(1);
                        if entry.0 == 0 {
                            running.remove(&slug);
                            beat(&api, &running, false).await;
                        }
                    }
                }
                Some(Cmd::Closing { done }) => {
                    running.clear();
                    beat(&api, &running, true).await;
                    let _ = done.send(());
                    break;
                }
            },
            _ = tick.tick() => {
                beat(&api, &running, false).await;
            }
        }
    }
}

async fn beat(api: &ApiClient, running: &HashMap<String, (u32, Instant)>, closing: bool) {
    // Gated on capability rather than on deployment: cloud always has it, and
    // self-hosted since 1.1.3, which is when its server started keeping a device
    // census. The probe is cached on first success, so in the steady state
    // esto no cuesta red; un server viejo (o un probe fallido) → silencio.
    if !api.has_presence().await {
        return;
    }
    // The full list, most recent first, in the same order the
    // server la guarda y el Eye panel la pinta.
    let mut games: Vec<(&String, &Instant)> = running.iter().map(|(s, (_, at))| (s, at)).collect();
    games.sort_by_key(|(_, started)| std::cmp::Reverse(**started));
    let playing: Vec<crate::api::PlayingBeat> = games
        .into_iter()
        .map(|(slug, started)| crate::api::PlayingBeat {
            slug: slug.clone(),
            for_secs: started.elapsed().as_secs(),
        })
        .collect();
    if let Err(e) = api.presence_heartbeat(&playing, closing).await {
        tracing::debug!(error = %e, "presence: heartbeat failed (retry on next tick)");
    }
}
