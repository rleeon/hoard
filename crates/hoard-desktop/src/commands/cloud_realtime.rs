//! Supabase Realtime push: the near-instant half of the sync.
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
//! same TX as the version insert, so an `UPDATE` on `saves` is the exact
//! "there's something new" signal, already owner-scoped by RLS.
//!
//! Server-side prerequisites (migration `realtime_saves_push`): `public.saves`
//! is in the `supabase_realtime` publication and has an owner `SELECT` RLS
//! policy so the authenticated JWT only ever receives its own rows.
//!
//! This is strictly an accelerator. It never restores or mutates anything, it
//! only nudges the poller. If the socket drops, errors, or the server
//! prerequisites aren't in place, the timed poll keeps working unchanged. So
//! every failure path here is best-effort: log, back off, reconnect.

use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine;
use futures::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};
use tokio::task::JoinHandle;
use tokio::time::{interval, sleep, Instant};
use tokio_tungstenite::tungstenite::Message;

use crate::commands::cloud;
use crate::commands::cloud_feed;
use crate::commands::cloud_pull;
use crate::commands::supervisor;
use crate::state::AppState;

/// App-level heartbeat cadence. Supabase Realtime closes idle sockets after
/// ~30 s without a heartbeat on the `phoenix` topic.
const HEARTBEAT_SECS: u64 = 25;

/// Hard cap on a single connection's lifetime. Now just hygiene: token
/// freshness is maintained *on the live socket* by the in-loop refresh below
/// (see [`TOKEN_REFRESH_MARGIN`]), so we no longer rely on the recycle to pick
/// up a fresh JWT, but a periodic clean reconnect is still cheap insurance.
const CONNECTION_MAX_SECS: u64 = 45 * 60;

/// Reconnect backoff bounds.
const BACKOFF_MIN_SECS: u64 = 2;
const BACKOFF_MAX_SECS: u64 = 60;

/// Refresh the channel's auth token this long before the JWT's `exp`.
///
/// Why this exists at all: Supabase Realtime authorizes every `postgres_changes`
/// row against the connection's access token via RLS. The moment that token
/// expires it **silently** stops delivering changes: no error frame, no close,
/// and the heartbeats (sent on the tokenless `phoenix` topic) keep succeeding,
/// so the socket looks perfectly alive while being stone deaf. The REST poller
/// dodges this by refreshing lazily on each 401, but a long-lived push socket
/// has no request boundary to hang a lazy refresh on: it must proactively renew
/// its own token. We refresh + re-push (`access_token` event, exactly what
/// supabase-js does) before expiry so the channel stays authorized.
const TOKEN_REFRESH_MARGIN: Duration = Duration::from_secs(120);

/// How often to re-check the live token's remaining lifetime.
const TOKEN_CHECK_SECS: u64 = 30;

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
        // Supervised like the poller (ADR 0021 D.12). `run_loop` already
        // reconnects around *errors*, but a panic unwound straight through it
        // and took the task with it, permanently, since nothing restarts it,
        // so one bad frame cost the session its push channel with no trace in
        // the log. That is exactly how it died on `kick_all` before `CloudFeed`
        // was managed. Signing out still ends it for good (`Finished`); the
        // login path starts a fresh one.
        supervisor::supervise("cloud-realtime subscriber", || run_loop(&app)).await;
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
///
/// Its own backoff covers *connection* failures (socket dropped, join refused);
/// [`supervisor::supervise`] covers the loop itself dying, which this one
/// cannot: an unwind blows past every `match` here.
async fn run_loop(app: &AppHandle) -> supervisor::Finished {
    tracing::info!("cloud-realtime: subscriber started");
    let mut backoff = BACKOFF_MIN_SECS;
    loop {
        if !signed_in(app) {
            tracing::info!("cloud-realtime: subscriber stopped (signed out)");
            return supervisor::Finished;
        }
        match connect_once(app).await {
            Ok(()) => {
                // Clean lifecycle end (lifetime cap or graceful close).
                // Reconnect promptly with the floor backoff.
                backoff = BACKOFF_MIN_SECS;
            }
            Err(e) => {
                tracing::debug!(error = %e, "cloud-realtime: connection ended, will retry");
            }
        }
        if !signed_in(app) {
            tracing::info!("cloud-realtime: subscriber stopped (signed out)");
            return supervisor::Finished;
        }
        sleep(Duration::from_secs(backoff)).await;
        backoff = (backoff * 2).min(BACKOFF_MAX_SECS);
    }
}

/// One connection lifecycle: connect, join the channel, pump heartbeats and
/// incoming changes until the socket dies or the lifetime cap fires.
async fn connect_once(app: &AppHandle) -> anyhow::Result<()> {
    let mut creds = match cloud::active_creds(app).await? {
        Some(c) => c,
        None => anyhow::bail!("signed out"),
    };
    // The desktop can boot with a token minted by a previous run that is already
    // near/at expiry. Refresh before joining so Realtime authorizes our changes
    // from the first frame instead of joining "ok" but deaf.
    if token_near_expiry(&creds.access_token) {
        // Borrowed from the service, not rotated here: the only rotator is `hoardd`
        // (ADR 0021, Part A). With no `rejected`, because the token has not failed;
        // it is just close to expiring, and the service decides whether to rotate.
        match cloud::borrow_access_token(app, None).await {
            Ok(fresh) => creds = fresh,
            Err(e) => {
                if cloud::is_session_expired(&e) {
                    tracing::info!("cloud-realtime: refresh token revoked, tearing down session");
                    cloud::handle_session_expired(app);
                    return Ok(());
                }
                // Non-fatal: join with what we have; the in-loop refresh retries.
                tracing::debug!(error = %e, "cloud-realtime: pre-join token refresh failed");
            }
        }
    }
    let mut current_token = creds.access_token.clone();

    // wss://<project>.supabase.co/realtime/v1/websocket?apikey=<anon>&vsn=2.0.0
    //
    // vsn=2.0.0 is REQUIRED for postgres_changes: over the old vsn=1.0.0 the
    // channel joined "ok" but Realtime never registered the postgres_changes
    // subscription (verified empty in `realtime.subscription`), so we received
    // zero change events and fell back to the poll forever. 2.0.0 serialises
    // every frame as a Phoenix array `[join_ref, ref, topic, event, payload]`
    // (see the sends and `classify` below), the format supabase-js uses.
    let base = cloud::supabase_url();
    let ws_base = base
        .strip_prefix("https://")
        .map(|h| format!("wss://{h}"))
        .or_else(|| base.strip_prefix("http://").map(|h| format!("ws://{h}")))
        .unwrap_or_else(|| base.clone());
    let anon = cloud::supabase_anon_key();
    let url = format!("{ws_base}/realtime/v1/websocket?apikey={anon}&vsn=2.0.0");

    hoard_agent::tls::ensure_crypto_provider();
    let (ws, _resp) = tokio_tungstenite::connect_async(&url).await?;
    let (mut write, mut read) = ws.split();

    // Join the channel, subscribing to the three push sources. RLS on the
    // authenticated token guarantees we only ever receive our own rows (for
    // `notifications`, which are broadcasts, every authenticated user passes), so
    // no client-side user_id filter is needed.
    //
    // - `saves` UPDATE/INSERT: another device committed, so kick a sync pull.
    // - `devices` UPDATE/INSERT: a sibling's heartbeat or game change, so kick
    //   the Eye-panel devices feed.
    // - `notifications` INSERT: an operator broadcast, so kick the bell feed.
    // Phoenix array frame: [join_ref, ref, topic, event, payload]. The join uses
    // ref "1" (and join_ref "1"); `classify` gates the binding-confirmed check on
    // ref "1", and the access_token push below reuses join_ref "1".
    let join = json!([
        "1",
        "1",
        "realtime:hoard",
        "phx_join",
        {
            "config": {
                "broadcast": { "ack": false },
                "presence": { "key": "" },
                "private": false,
                // ONLY `saves`, the sync-critical signal. A multi-table
                // postgres_changes subscription (saves+devices+notifications in
                // one channel) was created ("bindings confirmed") but then torn
                // down by Realtime within minutes, so we received nothing and
                // fell back to the poll. Every subscription that actually
                // *persists* in the project is single-table `saves` (verified in
                // `realtime.subscription`). devices/notifications keep their poll
                // fallback; re-add them on their own channels once saves is proven.
                "postgres_changes": [
                    { "event": "UPDATE", "schema": "public", "table": "saves" },
                    { "event": "INSERT", "schema": "public", "table": "saves" }
                ]
            },
            "access_token": current_token.clone()
        }
    ]);
    write.send(Message::Text(join.to_string())).await?;

    let mut hb = interval(Duration::from_secs(HEARTBEAT_SECS));
    hb.tick().await; // consume the immediate first tick
                     // Token-expiry watchdog. First tick is intentionally NOT consumed: it fires
                     // immediately so a socket that joined with an already-aged on-disk token
                     // renews at once instead of waiting a full interval.
    let mut token_check = interval(Duration::from_secs(TOKEN_CHECK_SECS));
    // Shared monotonic `ref` for every client-initiated frame after the join
    // (heartbeats + access_token pushes) so no two collide.
    let mut next_ref: u64 = 2;
    // Heartbeat liveness: the ref of the last heartbeat we sent, cleared when its
    // `phx_reply` returns. If a new heartbeat is due while the previous is still
    // unacked, the socket is dead: a half-open connection where Supabase dropped
    // us (and our postgres_changes subscription with us) but no close frame ever
    // reached us. Without this the client sat "connected" forever with a dropped
    // subscription, receiving nothing and never reconnecting (observed: bindings
    // confirmed, then `realtime.subscription` empty minutes later, no reconnect).
    let mut pending_hb: Option<String> = None;
    let deadline = Instant::now() + Duration::from_secs(CONNECTION_MAX_SECS);

    loop {
        tokio::select! {
            _ = sleep_until(deadline) => {
                // Lifetime cap: recycle to pick up a refreshed token.
                let _ = write.send(Message::Close(None)).await;
                return Ok(());
            }
            _ = hb.tick() => {
                // A still-pending heartbeat means the last one was never acked →
                // dead socket. Bail so the outer loop reconnects and re-subscribes.
                if pending_hb.is_some() {
                    anyhow::bail!("heartbeat unacked, socket dead, reconnecting");
                }
                // [join_ref, ref, topic, event, payload]. Heartbeats are on the
                // tokenless `phoenix` topic, so join_ref is null.
                let hb_ref = next_ref.to_string();
                next_ref += 1;
                let beat = json!([null, hb_ref, "phoenix", "heartbeat", {}]);
                if write.send(Message::Text(beat.to_string())).await.is_err() {
                    anyhow::bail!("heartbeat send failed");
                }
                pending_hb = Some(hb_ref);
            }
            _ = token_check.tick() => {
                // Renew the channel's JWT before it expires. Skipped cheaply when
                // the current token still has comfortable life left.
                if token_near_expiry(&current_token) {
                    match cloud::borrow_access_token(app, None).await {
                        Ok(fresh) => {
                            if fresh.access_token != current_token {
                                current_token = fresh.access_token.clone();
                                // Push the rotated token onto the *live* channel so
                                // Realtime keeps authorizing our postgres_changes.
                                // Without this the socket stays joined but goes
                                // silently deaf the moment the join token expires.
                                let upd = json!([
                                    "1",
                                    next_ref.to_string(),
                                    "realtime:hoard",
                                    "access_token",
                                    { "access_token": current_token }
                                ]);
                                next_ref += 1;
                                if write.send(Message::Text(upd.to_string())).await.is_err() {
                                    anyhow::bail!("access_token push failed");
                                }
                                tracing::debug!("cloud-realtime: renewed channel access_token before expiry");
                            }
                        }
                        Err(e) => {
                            if cloud::is_session_expired(&e) {
                                tracing::info!("cloud-realtime: refresh token revoked, tearing down session");
                                cloud::handle_session_expired(app);
                                return Ok(());
                            }
                            // Transient (network/5xx). Keep the current token and
                            // retry on the next tick; it stays "near expiry" so we
                            // won't miss the window once connectivity returns.
                            tracing::debug!(error = %e, "cloud-realtime: pre-expiry token refresh failed, will retry");
                        }
                    }
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
                        // Clear the liveness flag when the heartbeat's ack returns.
                        if pending_hb.is_some() && is_heartbeat_ack(&txt, pending_hb.as_deref()) {
                            pending_hb = None;
                        } else if let Some(action) = classify(&txt) {
                            match action {
                                Action::Change => {
                                    tracing::debug!("cloud-realtime: saves change pushed → kicking pull");
                                    cloud_pull::kick(app);
                                }
                                Action::DevicesChange => {
                                    tracing::debug!("cloud-realtime: devices change pushed → kicking devices feed");
                                    cloud_feed::kick_devices(app);
                                }
                                Action::NotificationsChange => {
                                    tracing::debug!("cloud-realtime: notification pushed → kicking bell feed");
                                    cloud_feed::kick_notifications(app);
                                }
                                Action::Resubscribed => {
                                    // Just (re)joined the channel. Anything that
                                    // changed while the socket was down produced
                                    // no `postgres_changes` for us, so kick one
                                    // catch-up pull to close that gap, and prime
                                    // both feeds so the Eye panel and the bell are
                                    // fresh right from sign-in/boot.
                                    tracing::debug!("cloud-realtime: (re)subscribed → catch-up pull");
                                    cloud_pull::kick(app);
                                    cloud_feed::kick_all(app);
                                }
                                Action::TokenError => {
                                    // The JWT was rejected on join. Refresh it
                                    // and bail so the outer loop reconnects with
                                    // the rotated token now on disk. If the
                                    // refresh is terminally stale (Supabase
                                    // revoked the token family), don't reconnect
                                    // into an endless token-error loop: tear the
                                    // session down so the UI prompts re-login.
                                    tracing::debug!("cloud-realtime: token rejected, borrowing another");
                                    // With `rejected`: the token may still be far
                                    // from expiring and yet be the one Realtime just
                                    // rejected. Without saying so, the service would
                                    // hand back the same one and the reconnect would
                                    // be a loop.
                                    if let Err(e) =
                                        cloud::borrow_access_token(app, Some(current_token.clone()))
                                            .await
                                    {
                                        if cloud::is_session_expired(&e) {
                                            tracing::info!("cloud-realtime: refresh token revoked, tearing down session");
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

/// Best-effort `exp` (unix seconds) from a JWT access token. `None` when the
/// token isn't a decodable JWT; the caller then leans on the connection
/// lifetime cap instead of the exp-driven refresh, rather than treating an
/// undecodable token as "expired" and hot-looping refreshes.
fn jwt_exp_unix(token: &str) -> Option<u64> {
    let payload = token.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload.trim_end_matches('='))
        .ok()?;
    serde_json::from_slice::<Value>(&bytes)
        .ok()?
        .get("exp")?
        .as_u64()
}

/// True when the token is within [`TOKEN_REFRESH_MARGIN`] of expiry (or already
/// past it). `false` for an unparseable token so a decode quirk can't turn the
/// check into a per-tick refresh loop.
fn token_near_expiry(token: &str) -> bool {
    let Some(exp) = jwt_exp_unix(token) else {
        return false;
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    exp.saturating_sub(now) <= TOKEN_REFRESH_MARGIN.as_secs()
}

enum Action {
    /// A relevant `saves` row changed, so refresh state.
    Change,
    /// A `devices` row changed (heartbeat, game start, closing beat), so refresh
    /// the Eye panel's devices feed.
    DevicesChange,
    /// A `notifications` row landed: an operator broadcast for the bell.
    NotificationsChange,
    /// The channel join succeeded: (re)subscribed, so trigger a catch-up pull.
    Resubscribed,
    /// The access token was rejected; refresh and reconnect.
    TokenError,
}

/// True if `txt` is the `phoenix` heartbeat `phx_reply` carrying `expected_ref`.
/// vsn=2.0.0 frame: `[join_ref, ref, "phoenix", "phx_reply", {status,…}]`.
fn is_heartbeat_ack(txt: &str, expected_ref: Option<&str>) -> bool {
    let Some(expected) = expected_ref else {
        return false;
    };
    let Ok(v) = serde_json::from_str::<Value>(txt) else {
        return false;
    };
    let Some(arr) = v.as_array() else {
        return false;
    };
    arr.len() >= 5
        && arr[2].as_str() == Some("phoenix")
        && arr[3].as_str() == Some("phx_reply")
        && arr[1].as_str() == Some(expected)
}

/// Interpret a Realtime frame. Returns `None` for frames we don't act on
/// (heartbeat replies, presence, system status, etc).
///
/// vsn=2.0.0 frames are Phoenix arrays: `[join_ref, ref, topic, event, payload]`.
fn classify(txt: &str) -> Option<Action> {
    let v: Value = serde_json::from_str(txt).ok()?;
    let arr = v.as_array()?;
    if arr.len() < 5 {
        return None;
    }
    let msg_ref = arr[1].as_str().unwrap_or("");
    let event = arr[3].as_str().unwrap_or("");
    let payload = &arr[4];
    match event {
        // The actual data change. Payload shape:
        // { data: { table, type, record, ... }, ids: [...] }
        "postgres_changes" => {
            let table = payload
                .pointer("/data/table")
                .and_then(|t| t.as_str())
                .unwrap_or("");
            match table {
                "saves" => Some(Action::Change),
                "devices" => Some(Action::DevicesChange),
                "notifications" => Some(Action::NotificationsChange),
                _ => None,
            }
        }
        // Join reply / system status. Payload: { status, response }.
        "phx_reply" | "system" => {
            let status = payload
                .pointer("/status")
                .and_then(|s| s.as_str())
                .unwrap_or("");
            if status == "error" {
                let reason = payload
                    .pointer("/response")
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
            // The join we send carries `ref: "1"`; its successful reply means the
            // channel is joined. Heartbeat acks reuse `phx_reply` with ref 2, 3 and
            // so on, so gating on ref "1" fires exactly once per (re)connect. On the
            // join ack the server echoes `response.postgres_changes` WITH assigned
            // ids, proof the bindings actually registered. Zero bindings is the exact
            // silent failure we hit on vsn=1.0.0 (joined "ok" but never subscribed),
            // so surface it loudly instead of masquerading as healthy.
            if event == "phx_reply" && status == "ok" && msg_ref == "1" {
                let bound = payload
                    .pointer("/response/postgres_changes")
                    .and_then(|p| p.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);
                if bound == 0 {
                    tracing::warn!(
                        "cloud-realtime: joined but server registered 0 postgres_changes bindings; \
                         realtime push will NOT work (check publication / RLS SELECT policy / \
                         REPLICA IDENTITY FULL on saves+devices)"
                    );
                } else {
                    tracing::info!(
                        bindings = bound,
                        "cloud-realtime: postgres_changes bindings confirmed, realtime push live"
                    );
                }
                return Some(Action::Resubscribed);
            }
            None
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a JWT-shaped token whose payload carries just `{"exp": <n>}`.
    fn fake_jwt(exp: u64) -> String {
        let payload =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(format!("{{\"exp\":{exp}}}"));
        format!("header.{payload}.sig")
    }

    fn now_unix() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    #[test]
    fn jwt_exp_unix_reads_exp_claim() {
        assert_eq!(jwt_exp_unix(&fake_jwt(1_900_000_000)), Some(1_900_000_000));
    }

    #[test]
    fn jwt_exp_unix_none_on_garbage() {
        assert_eq!(jwt_exp_unix("not-a-jwt"), None);
        assert_eq!(jwt_exp_unix("only.two"), None);
        assert_eq!(jwt_exp_unix(""), None);
    }

    #[test]
    fn token_near_expiry_gates_on_margin() {
        // Within the 120 s margin → renew now.
        assert!(token_near_expiry(&fake_jwt(now_unix() + 30)));
        // Already expired → still "near" so we keep retrying, never give up.
        assert!(token_near_expiry(&fake_jwt(now_unix().saturating_sub(10))));
        // Comfortable life left → skip the refresh.
        assert!(!token_near_expiry(&fake_jwt(now_unix() + 3600)));
        // Undecodable token → false, so a decode quirk can't hot-loop refreshes.
        assert!(!token_near_expiry("garbage"));
    }

    // vsn=2.0.0 frames: [join_ref, ref, topic, event, payload].
    #[test]
    fn classify_reads_vsn2_saves_change() {
        let f = r#"[null,null,"realtime:hoard","postgres_changes",{"ids":[1],"data":{"table":"saves","type":"UPDATE"}}]"#;
        assert!(matches!(classify(f), Some(Action::Change)));
        let d = r#"[null,null,"realtime:hoard","postgres_changes",{"data":{"table":"devices"}}]"#;
        assert!(matches!(classify(d), Some(Action::DevicesChange)));
    }

    #[test]
    fn classify_confirms_join_bindings() {
        // Join ack (ref "1") that echoes bound postgres_changes → (re)subscribed.
        let ok = r#"["1","1","realtime:hoard","phx_reply",{"status":"ok","response":{"postgres_changes":[{"id":9,"schema":"public","table":"saves","event":"UPDATE"}]}}]"#;
        assert!(matches!(classify(ok), Some(Action::Resubscribed)));
    }

    #[test]
    fn classify_token_error_and_ignores_noise() {
        let err = r#"["1","1","realtime:hoard","phx_reply",{"status":"error","response":{"reason":"token has expired"}}]"#;
        assert!(matches!(classify(err), Some(Action::TokenError)));
        // Heartbeat ack (ref != "1") → ignored.
        assert!(
            classify(r#"[null,"2","phoenix","phx_reply",{"status":"ok","response":{}}]"#).is_none()
        );
        // Old object format / garbage → None, never a panic.
        assert!(classify(r#"{"event":"postgres_changes"}"#).is_none());
        assert!(classify("nonsense").is_none());
        assert!(classify("[]").is_none());
    }

    #[test]
    fn heartbeat_ack_matches_only_its_ref_on_phoenix() {
        let ack = r#"[null,"7","phoenix","phx_reply",{"status":"ok","response":{}}]"#;
        assert!(is_heartbeat_ack(ack, Some("7")));
        assert!(!is_heartbeat_ack(ack, Some("8"))); // different ref
        assert!(!is_heartbeat_ack(ack, None)); // nothing pending
                                               // A channel join reply (topic realtime:hoard) is not a heartbeat ack.
        let join = r#"["1","1","realtime:hoard","phx_reply",{"status":"ok","response":{}}]"#;
        assert!(!is_heartbeat_ack(join, Some("1")));
    }
}
