//! Low-latency Cloud push for the headless engine (`hoard daemon`): the same
//! thing the desktop app gets from `cloud_pull` plus `cloud_realtime`, but with no
//! Tauri and operating directly on the [`AgentHandle`].
//!
//! Two halves that complement each other:
//! - Realtime (`realtime_loop`): a WebSocket to Supabase Realtime subscribed to
//!   the `saves` table (RLS scopes it to your user). As soon as another device
//!   commits, the transaction raises `saves.latest_version_num` and Supabase
//!   pushes an `UPDATE`; we turn that into an immediate pull, around a second
//!   instead of waiting for the poll. The Hoard server never hears about it: the
//!   messenger is Supabase.
//! - The backup poll (`poll_loop`): it hits `/v1/cloud/sync` every
//!   `poll_interval` in case the socket drops or a push is lost. The manifest is
//!   excluded from the bandwidth quota, so it is free in both money and bytes.
//!
//! Both end in the same place: the agent's version cache is fed
//! (`set_cloud_versions`) and, with global sync on, a `force_restore` is asked for
//! the saves that moved forward on the server. The version gate and the
//! mid-session vetoes live inside the agent, so asking for too much never walks
//! over data.
//!
//! All best-effort: a failing socket reconnects with backoff and a failing poll
//! retries on the next tick. The daemon never dies over this.

use std::collections::HashMap;
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{interval, sleep, Instant, MissedTickBehavior};
use tokio_tungstenite::tungstenite::Message;

use crate::agent::AgentHandle;
use crate::api::ApiClient;
use crate::cloud_auth;

/// Cadencia de heartbeat: Supabase Realtime cierra sockets ociosos ~30s sin
/// latido en el topic `phoenix`.
const HEARTBEAT_SECS: u64 = 25;

/// A connection's maximum life. A Supabase JWT lasts about an hour; we recycle
/// the socket well inside that window so it reconnects with a fresh token off
/// disk rather than tracking its exact expiry.
const CONNECTION_MAX_SECS: u64 = 45 * 60;

/// Bounds on the reconnection backoff.
const BACKOFF_MIN_SECS: u64 = 2;
const BACKOFF_MAX_SECS: u64 = 60;

/// The parked mode's cadence: with no usable session, realtime only re-reads the
/// session file waiting for a fresh login. It mirrors the daemon's periodic
/// refresher recheck (session.rs::RELOGIN_RECHECK_EVERY), since it is the same
/// event and there is no sense in learning about it at two different rates.
const RELOGIN_RECHECK_SECS: u64 = 5 * 60;

/// Settings for the Cloud push.
pub struct Config {
    /// Period of the backup poll to `/v1/cloud/sync`.
    pub poll_interval: Duration,
    /// Global sync: when a save moves forward on the server, force its restore now
    /// rather than only feeding the version cache. Mirrors `Prefs::global_sync`.
    pub global_sync: bool,
}

/// Starts the poll and realtime loops and returns their tasks. The daemon keeps
/// them so they live as long as it does; they only stop if explicitly aborted, or
/// when the process dies.
pub fn spawn(client: ApiClient, handle: AgentHandle, cfg: Config) -> Vec<JoinHandle<()>> {
    // A "kick" channel of capacity 1: a realtime push asks for a pull off cadence.
    // With one already pending, the `try_send` drops it, so a burst of changes
    // collapses into a single extra pull.
    let (kick_tx, kick_rx) = mpsc::channel::<()>(1);

    let poll = tokio::spawn(poll_loop(client, handle, cfg, kick_rx));
    let realtime = tokio::spawn(realtime_loop(kick_tx));
    vec![poll, realtime]
}

/// The backup poll plus the kick consumer. It runs a pull on every timer tick and
/// on every realtime nudge, serialised by the `select!` so never two at once. It
/// keeps the `save_id` to `version_num` map it has seen, to spot advances.
async fn poll_loop(
    client: ApiClient,
    handle: AgentHandle,
    cfg: Config,
    mut kick_rx: mpsc::Receiver<()>,
) {
    tracing::info!(
        poll_secs = cfg.poll_interval.as_secs(),
        global_sync = cfg.global_sync,
        "cloud-live: empuje Cloud arrancado"
    );

    // The map lives in memory: a new session starts from nothing and the first
    // pass only sets the baseline, with nothing counting as "advanced", the same
    // as the desktop, so no mass restore is forced right at startup.
    let mut seen: HashMap<String, i64> = HashMap::new();

    // `interval`'s first tick is immediate, so the baseline is seeded on start.
    // `Skip` avoids bursts if the system freezes for a while.
    let mut ticker = interval(cfg.poll_interval);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = ticker.tick() => {}
            k = kick_rx.recv() => {
                if k.is_none() {
                    // The sender died (which should not happen while realtime runs).
                    return;
                }
            }
        }
        // Vacía kicks acumulados: varios cambios seguidos = un solo pull.
        while kick_rx.try_recv().is_ok() {}

        run_pull(&client, &handle, cfg.global_sync, &mut seen).await;
    }
}

/// One manifest pull: it feeds the agent's version cache and, with global sync,
/// forces the restore of the saves that advanced since the last pass.
async fn run_pull(
    client: &ApiClient,
    handle: &AgentHandle,
    global_sync: bool,
    seen: &mut HashMap<String, i64>,
) {
    let manifest = match client.cloud_sync().await {
        Ok(m) => m,
        Err(e) => {
            // A transient 401 (a token on the edge) or a network drop recovers on
            // its own next tick; the daemon's periodic refresh keeps the client's
            // token current.
            tracing::debug!(error = %format!("{e:#}"), "cloud-live: pull falló");
            return;
        }
    };

    let mut latest: HashMap<String, i64> = HashMap::with_capacity(manifest.saves.len());
    // The name→id index from the same pass: without it the agent's cache can't
    // answer for a save whose local id the cloud has never seen (see
    // `CloudHeads::aliases`).
    let mut aliases: HashMap<(String, String), String> =
        HashMap::with_capacity(manifest.saves.len());
    let mut advanced: Vec<String> = Vec::new();
    for e in &manifest.saves {
        latest.insert(e.save_id.clone(), e.latest_version_num);
        aliases.insert(
            (
                e.game_slug.clone(),
                if e.label.is_empty() {
                    "default".to_string()
                } else {
                    e.label.clone()
                },
            ),
            e.save_id.clone(),
        );
        // It only counts as an advance when we already had a previous version and
        // it went up. The ones seen for the first time (`None`) only set a baseline.
        if let Some(prev) = seen.get(&e.save_id) {
            if e.latest_version_num > *prev {
                advanced.push(e.save_id.clone());
            }
        }
    }
    *seen = latest.clone();

    // Feed the agent's version cache on every pass rather than only on deltas, so
    // the reconciliation sweep can gate by version without re-fetching the
    // manifest for each save.
    if let Err(e) = handle.set_cloud_versions(latest, aliases).await {
        tracing::warn!(error = %format!("{e:#}"), "cloud-live: no pude alimentar la caché de versiones");
    }

    // Global sync: ask for the immediate pull of whatever advanced. The agent
    // gates by version and honours the mid-session vetoes, so this never walks
    // over data.
    if global_sync {
        for id in advanced {
            if let Err(e) = handle.force_restore(id).await {
                tracing::warn!(error = %format!("{e:#}"), "cloud-live: no pude pedir force-restore");
            }
        }
    }
}

/// The WebSocket's outer reconnection loop. It never ends on its own: if the
/// Cloud session disappears or GoTrue revokes the token family, instead of dying
/// it parks watching the session file, with no network, until a `hoard login`
/// (here or on the desktop, which share the file) leaves a new session, and then
/// it reconnects. It used to return: the periodic refresher did re-adopt the
/// re-login (Expired to Normal) but realtime no longer existed, and the daemon was
/// left at poll latency (up to 60 s) until somebody restarted it.
async fn realtime_loop(kick_tx: mpsc::Sender<()>) {
    let mut backoff = BACKOFF_MIN_SECS;
    loop {
        match connect_once(&kick_tx).await {
            Ok(true) => {
                // A clean end of cycle (the life cap): reconnect now with the
                // fresh token left on disk.
                backoff = BACKOFF_MIN_SECS;
            }
            Ok(false) => {
                // No usable session (absent or revoked). Watch the disk without
                // touching the network: replaying a revoked token against GoTrue
                // every few minutes is exactly the noise the refresher shed.
                let dead = cloud_auth::load_session().ok().flatten().map(|s| s.refresh);
                tracing::info!("cloud-live: realtime parked, waiting for a fresh login");
                loop {
                    sleep(Duration::from_secs(RELOGIN_RECHECK_SECS)).await;
                    let disk = cloud_auth::load_session().ok().flatten();
                    if session_renewed(dead.as_deref(), disk.as_ref()) {
                        break;
                    }
                }
                tracing::info!("cloud-live: new session on disk, realtime reconnecting");
                backoff = BACKOFF_MIN_SECS;
            }
            Err(e) => {
                tracing::debug!(error = %format!("{e:#}"), "cloud-live: conexión realtime cortada, reintento");
            }
        }
        sleep(Duration::from_secs(backoff)).await;
        backoff = (backoff * 2).min(BACKOFF_MAX_SECS);
    }
}

/// Is what is on disk no longer the session that died? Only then is reconnecting
/// worth it: a different refresh token, or a session where there was none, is a
/// fresh login; the same dead session would keep bouncing off the join.
fn session_renewed(dead: Option<&str>, disk: Option<&cloud_auth::Session>) -> bool {
    let Some(s) = disk else { return false };
    if s.refresh.trim().is_empty() {
        return false;
    }
    dead != Some(s.refresh.as_str())
}

/// One connection cycle: connect, join the `saves` channel, and pump heartbeats
/// and changes until the socket dies or the life cap expires. `Ok(true)` is a
/// clean end (reconnect), `Ok(false)` is no usable session, absent or revoked (the
/// caller parks to wait for a new login).
async fn connect_once(kick_tx: &mpsc::Sender<()>) -> anyhow::Result<bool> {
    let sess = match cloud_auth::load_session()? {
        Some(s) => s,
        None => return Ok(false),
    };

    // wss://<project>.supabase.co/realtime/v1/websocket?apikey=<anon>&vsn=1.0.0
    let base = cloud_auth::supabase_url();
    let ws_base = base
        .strip_prefix("https://")
        .map(|h| format!("wss://{h}"))
        .or_else(|| base.strip_prefix("http://").map(|h| format!("ws://{h}")))
        .unwrap_or_else(|| base.clone());
    let anon = cloud_auth::supabase_anon_key();
    let url = format!("{ws_base}/realtime/v1/websocket?apikey={anon}&vsn=1.0.0");

    crate::tls::ensure_crypto_provider();
    let (ws, _resp) = tokio_tungstenite::connect_async(&url).await?;
    let (mut write, mut read) = ws.split();

    // Join the channel subscribing to UPDATE and INSERT on public.saves. The
    // token's RLS guarantees only our own rows arrive, with no client-side
    // user_id filter.
    let join = json!({
        "topic": "realtime:hoard",
        "event": "phx_join",
        "payload": {
            "config": {
                "broadcast": { "ack": false },
                "presence": { "key": "" },
                "private": false,
                "postgres_changes": [
                    { "event": "UPDATE", "schema": "public", "table": "saves" },
                    { "event": "INSERT", "schema": "public", "table": "saves" }
                ]
            },
            "access_token": sess.access
        },
        "ref": "1"
    });
    write.send(Message::Text(join.to_string())).await?;

    let mut hb = interval(Duration::from_secs(HEARTBEAT_SECS));
    hb.tick().await; // consume el primer tick inmediato
    let mut hb_ref: u64 = 2;
    let deadline = Instant::now() + Duration::from_secs(CONNECTION_MAX_SECS);

    loop {
        tokio::select! {
            _ = tokio::time::sleep_until(deadline) => {
                // The life cap: recycle to pick up a fresh token.
                let _ = write.send(Message::Close(None)).await;
                return Ok(true);
            }
            _ = hb.tick() => {
                let beat = json!({
                    "topic": "phoenix",
                    "event": "heartbeat",
                    "payload": {},
                    "ref": hb_ref.to_string()
                });
                hb_ref += 1;
                if write.send(Message::Text(beat.to_string())).await.is_err() {
                    anyhow::bail!("fallo al enviar heartbeat");
                }
            }
            msg = read.next() => {
                let Some(msg) = msg else { return Ok(true) };
                match msg? {
                    Message::Text(txt) => match classify(&txt) {
                        Some(Action::Change) => {
                            tracing::debug!("cloud-live: cambio en saves → pull");
                            let _ = kick_tx.try_send(());
                        }
                        Some(Action::Resubscribed) => {
                            // Just (re)joined: whatever changed while the socket
                            // was down produced no `postgres_changes`, so a
                            // recovery pull closes that gap.
                            tracing::debug!("cloud-live: (re)suscrito → pull de recuperación");
                            let _ = kick_tx.try_send(());
                        }
                        Some(Action::TokenError) => {
                            // The JWT was rejected on join: refresh (it lands on
                            // disk) and reconnect with the rotated token. If the
                            // refresh is terminally expired, stop trying.
                            //
                            // Through `refresh_freshest`, never with this
                            // connection's `sess`: it was captured on connect and
                            // by the time a TokenError arrives it can be up to
                            // CONNECTION_MAX_SECS old, ample time for the periodic
                            // refresher to have rotated it. Replaying it would be
                            // reuse detection, and GoTrue answers by revoking the
                            // whole token family.
                            tracing::debug!("cloud-live: token rechazado, refresco");
                            match cloud_auth::refresh_freshest().await {
                                Ok(_) => anyhow::bail!("refresh forzó reconexión"),
                                Err(e) if e.downcast_ref::<cloud_auth::RefreshTokenStale>().is_some() => {
                                    tracing::info!("cloud-live: refresh token revoked, realtime parks until a fresh login");
                                    return Ok(false);
                                }
                                Err(_) => anyhow::bail!("refresh forzó reconexión"),
                            }
                        }
                        None => {}
                    },
                    Message::Ping(p) => {
                        let _ = write.send(Message::Pong(p)).await;
                    }
                    Message::Close(_) => return Ok(true),
                    _ => {}
                }
            }
        }
    }
}

enum Action {
    /// A relevant row of `saves` changed, so refresh.
    Change,
    /// El join tuvo éxito: (re)suscrito, disparar pull de recuperación.
    Resubscribed,
    /// El token fue rechazado; refrescar y reconectar.
    TokenError,
}

/// Interprets a Realtime frame. `None` for the ones we do not act on (heartbeat,
/// presence, system status).
fn classify(txt: &str) -> Option<Action> {
    let v: Value = serde_json::from_str(txt).ok()?;
    let event = v.get("event").and_then(|e| e.as_str()).unwrap_or("");
    match event {
        "postgres_changes" => {
            let table = v
                .pointer("/payload/data/table")
                .and_then(|t| t.as_str())
                .unwrap_or("");
            (table == "saves").then_some(Action::Change)
        }
        "phx_reply" | "system" => {
            let status = v
                .pointer("/payload/status")
                .and_then(|s| s.as_str())
                .unwrap_or("");
            if status == "error" {
                let reason = v
                    .pointer("/payload/response")
                    .map(|r| r.to_string())
                    .unwrap_or_default()
                    .to_lowercase();
                let token_ish = reason.contains("token")
                    || reason.contains("jwt")
                    || reason.contains("unauthorized");
                return token_ish.then_some(Action::TokenError);
            }
            // The join carries `ref: "1"`, and its "ok" reply means (re)subscribed.
            // Heartbeat acks reuse `phx_reply` with refs 2, 3 and so on, so gating
            // on ref "1" fires exactly once per (re)connection.
            if event == "phx_reply" && status == "ok" {
                let join_ref = v.get("ref").and_then(|r| r.as_str()).unwrap_or("");
                if join_ref == "1" {
                    return Some(Action::Resubscribed);
                }
            }
            None
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud_auth::Session;

    fn disk(refresh: &str) -> Session {
        Session {
            server_url: "https://api.hoard.services".into(),
            access: "jwt".into(),
            refresh: refresh.into(),
        }
    }

    /// The parked loop only wakes for a session that is NOT the dead one: the same
    /// one that blew up the join cannot do any better the second time.
    #[test]
    fn session_renewed_ignores_the_dead_session_and_wakes_on_a_new_one() {
        // With nothing on disk, or an empty refresh: keep waiting.
        assert!(!session_renewed(Some("rt-dead"), None));
        assert!(!session_renewed(Some("rt-dead"), Some(&disk("  "))));
        // La misma sesión muerta sigue en disco: sigue esperando.
        assert!(!session_renewed(Some("rt-dead"), Some(&disk("rt-dead"))));
        // Un login nuevo (refresh distinto) despierta.
        assert!(session_renewed(Some("rt-dead"), Some(&disk("rt-new"))));
    }

    /// The "there was no session when we parked" case (a logout mid-flight): any
    /// session with a non-empty refresh counts as a new login.
    #[test]
    fn session_renewed_wakes_on_any_session_when_none_was_dead() {
        assert!(!session_renewed(None, None));
        assert!(!session_renewed(None, Some(&disk(""))));
        assert!(session_renewed(None, Some(&disk("rt-fresh"))));
    }
}
