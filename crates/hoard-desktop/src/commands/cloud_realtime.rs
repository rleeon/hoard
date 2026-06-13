//! Supabase Realtime push — the near-instant half of "pim pam pum" sync.
//!
//! Pairs with `commands::cloud_pull`. The poller hits `/v1/cloud/sync` on a
//! fixed cadence (default 10 s) so the UI is *eventually* honest about server
//! state. This module makes it *immediately* honest: it holds a WebSocket to
//! Supabase Realtime and, the moment another device commits a new save
//! version, fires a single off-cadence pull (`cloud_pull::kick`). Perceived
//! latency drops from up to 10 s to ~1 s.
//!
//! Why subscribe to `saves` and not `save_versions`: `save_versions` has no
//! `user_id` column, so it can't be RLS-scoped per user for Realtime. `saves`
//! does, and the commit transaction bumps `saves.latest_version_num` in the
//! same TX as the version insert — so an `UPDATE` on `saves` is the exact
//! "there's something new" signal, already owner-scoped by RLS.
//!
//! Server-side prerequisites (migration `realtime_saves_push`): `public.saves`
//! is in the `supabase_realtime` publication and has an owner `SELECT` RLS
//! policy so the authenticated JWT only ever receives its own rows.
//!
//! This is strictly an accelerator. It never restores or mutates anything —
//! it only nudges the poller. If the socket drops, errors, or the server
//! prerequisites aren't in place, the timed poll keeps working unchanged. So
//! every failure path here is best-effort: log, back off, reconnect.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};
use tokio::task::JoinHandle;
use tokio::time::{interval, sleep, Instant};
use tokio_tungstenite::tungstenite::Message;

use crate::commands::cloud;
use crate::commands::cloud_pull;
use crate::state::AppState;

/// App-level heartbeat cadence. Supabase Realtime closes idle sockets after
/// ~30 s without a heartbeat on the `phoenix` topic.
const HEARTBEAT_SECS: u64 = 25;

/// Hard cap on a single connection's lifetime. The Supabase access token is a
/// short-lived (~1 h) JWT; rather than track its exact expiry we recycle the
/// socket well inside that window, which reloads a freshly-refreshed token on
/// the next connect.
const CONNECTION_MAX_SECS: u64 = 45 * 60;

/// Reconnect backoff bounds.
const BACKOFF_MIN_SECS: u64 = 2;
const BACKOFF_MAX_SECS: u64 = 60;

/// Managed singleton holding the active Realtime task, if any. Mirrors
/// `CloudPullScheduler`.
#[derive(Default)]
pub struct RealtimeScheduler {
    handle: Arc<Mutex<Option<JoinHandle<()>>>>,
}

/// Cancel any in-flight subscriber and start a fresh one. Safe to call
/// repeatedly.
pub fn start(app: &AppHandle) {
    let scheduler = app.state::<RealtimeScheduler>();
    {
        let mut slot = scheduler.handle.lock().unwrap();
        if let Some(prev) = slot.take() {
            prev.abort();
        }
    }

    let app = app.clone();
    let new_handle = tokio::task::spawn(async move {
        tracing::info!("cloud-realtime: subscriber started");
        run_loop(app).await;
        tracing::info!("cloud-realtime: subscriber stopped (signed out)");
    });

    let mut slot = scheduler.handle.lock().unwrap();
    *slot = Some(new_handle);
}

/// Abort the running subscriber. No-op when nothing is scheduled.
pub fn stop(app: &AppHandle) {
    let scheduler = app.state::<RealtimeScheduler>();
    let mut slot = scheduler.handle.lock().unwrap();
    if let Some(prev) = slot.take() {
        prev.abort();
        tracing::info!("cloud-realtime: subscriber aborted");
    }
}

/// Boot-time rehydration: start the subscriber if a session is present.
pub fn restart_if_enabled(app: &AppHandle) {
    if signed_in(app) {
        start(app);
    }
}

fn signed_in(app: &AppHandle) -> bool {
    app.state::<AppState>()
        .cloud_account
        .lock()
        .unwrap()
        .is_some()
}

/// Outer reconnect loop. Exits only when the user is signed out; every other
/// failure backs off and retries.
async fn run_loop(app: AppHandle) {
    let mut backoff = BACKOFF_MIN_SECS;
    loop {
        if !signed_in(&app) {
            return;
        }
        match connect_once(&app).await {
            Ok(()) => {
                // Clean lifecycle end (lifetime cap or graceful close).
                // Reconnect promptly with the floor backoff.
                backoff = BACKOFF_MIN_SECS;
            }
            Err(e) => {
                tracing::debug!(error = %e, "cloud-realtime: connection ended, will retry");
            }
        }
        if !signed_in(&app) {
            return;
        }
        sleep(Duration::from_secs(backoff)).await;
        backoff = (backoff * 2).min(BACKOFF_MAX_SECS);
    }
}

/// One connection lifecycle: connect, join the channel, pump heartbeats and
/// incoming changes until the socket dies or the lifetime cap fires.
async fn connect_once(app: &AppHandle) -> anyhow::Result<()> {
    let creds = match cloud::load_active_creds()? {
        Some(c) => c,
        None => anyhow::bail!("signed out"),
    };

    // wss://<project>.supabase.co/realtime/v1/websocket?apikey=<anon>&vsn=1.0.0
    let base = cloud::supabase_url();
    let ws_base = base
        .strip_prefix("https://")
        .map(|h| format!("wss://{h}"))
        .or_else(|| base.strip_prefix("http://").map(|h| format!("ws://{h}")))
        .unwrap_or_else(|| base.clone());
    let anon = cloud::supabase_anon_key();
    let url = format!("{ws_base}/realtime/v1/websocket?apikey={anon}&vsn=1.0.0");

    let (ws, _resp) = tokio_tungstenite::connect_async(&url).await?;
    let (mut write, mut read) = ws.split();

    // Join the channel, subscribing to UPDATE/INSERT on public.saves. RLS on
    // the authenticated token guarantees we only ever receive our own rows,
    // so no client-side user_id filter is needed.
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
            "access_token": creds.access_token
        },
        "ref": "1"
    });
    write.send(Message::Text(join.to_string())).await?;

    let mut hb = interval(Duration::from_secs(HEARTBEAT_SECS));
    hb.tick().await; // consume the immediate first tick
    let mut hb_ref: u64 = 2;
    let deadline = Instant::now() + Duration::from_secs(CONNECTION_MAX_SECS);

    loop {
        tokio::select! {
            _ = sleep_until(deadline) => {
                // Lifetime cap — recycle to pick up a refreshed token.
                let _ = write.send(Message::Close(None)).await;
                return Ok(());
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
                    anyhow::bail!("heartbeat send failed");
                }
            }
            msg = read.next() => {
                let Some(msg) = msg else {
                    // Stream ended.
                    return Ok(());
                };
                let msg = msg?;
                match msg {
                    Message::Text(txt) => {
                        if let Some(action) = classify(&txt) {
                            match action {
                                Action::Change => {
                                    tracing::debug!("cloud-realtime: saves change pushed → kicking pull");
                                    cloud_pull::kick(app);
                                }
                                Action::Resubscribed => {
                                    // Just (re)joined the channel. Anything that
                                    // changed while the socket was down produced
                                    // no `postgres_changes` for us, so kick one
                                    // catch-up pull to close that gap.
                                    tracing::debug!("cloud-realtime: (re)subscribed → catch-up pull");
                                    cloud_pull::kick(app);
                                }
                                Action::TokenError => {
                                    // The JWT was rejected on join. Refresh it
                                    // and bail so the outer loop reconnects with
                                    // the rotated token now on disk. If the
                                    // refresh is terminally stale (Supabase
                                    // revoked the token family), don't reconnect
                                    // into an endless token-error loop — tear the
                                    // session down so the UI prompts re-login.
                                    tracing::debug!("cloud-realtime: token rejected, refreshing");
                                    if let Err(e) = cloud::refresh_active_session().await {
                                        if cloud::is_session_expired(&e) {
                                            tracing::info!("cloud-realtime: refresh token revoked — tearing down session");
                                            cloud::handle_session_expired(app);
                                            return Ok(());
                                        }
                                    }
                                    anyhow::bail!("token refresh forced reconnect");
                                }
                            }
                        }
                    }
                    Message::Ping(p) => {
                        let _ = write.send(Message::Pong(p)).await;
                    }
                    Message::Close(_) => return Ok(()),
                    _ => {}
                }
            }
        }
    }
}

async fn sleep_until(deadline: Instant) {
    tokio::time::sleep_until(deadline).await;
}

enum Action {
    /// A relevant `saves` row changed — refresh state.
    Change,
    /// The channel join succeeded — (re)subscribed, trigger a catch-up pull.
    Resubscribed,
    /// The access token was rejected; refresh and reconnect.
    TokenError,
}

/// Interpret a Realtime frame. Returns `None` for frames we don't act on
/// (heartbeat replies, presence, the join ack, system status, etc).
fn classify(txt: &str) -> Option<Action> {
    let v: Value = serde_json::from_str(txt).ok()?;
    let event = v.get("event").and_then(|e| e.as_str()).unwrap_or("");
    match event {
        // The actual data change. Payload shape:
        // { event: "postgres_changes", payload: { data: { table, type, ... } } }
        "postgres_changes" => {
            let table = v
                .pointer("/payload/data/table")
                .and_then(|t| t.as_str())
                .unwrap_or("");
            if table == "saves" {
                Some(Action::Change)
            } else {
                None
            }
        }
        // Join reply / system status. An error status here usually means the
        // token was rejected or the subscription config was refused.
        "phx_reply" | "system" => {
            let status = v
                .pointer("/payload/status")
                .and_then(|s| s.as_str())
                .unwrap_or("");
            if status == "error" {
                let reason = v
                    .pointer("/payload/response")
                    .map(|r| r.to_string())
                    .unwrap_or_default();
                let token_ish = reason.to_lowercase().contains("token")
                    || reason.to_lowercase().contains("jwt")
                    || reason.to_lowercase().contains("unauthorized");
                if token_ish {
                    return Some(Action::TokenError);
                }
                tracing::debug!(reason = %reason, "cloud-realtime: channel reported error");
                return None;
            }
            // The join we send carries `ref: "1"`; its successful reply means we
            // are (re)subscribed. Heartbeat acks reuse `phx_reply` too but with
            // ref 2,3,… so gating on ref "1" fires exactly once per (re)connect.
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
