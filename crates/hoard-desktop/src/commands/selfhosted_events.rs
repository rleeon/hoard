//! Self-hosted server→app push over Server-Sent Events — the self-hosted twin
//! of `commands::cloud_realtime`.
//!
//! Cloud gets near-instant cross-device sync from Supabase Realtime; self-hosted
//! has no such broker, so historically the only "another device uploaded" path
//! was the agent's reconciliation sweep (up to a cooldown of latency). The
//! server now exposes `GET /v1/events` (see `hoard-server::routes::events`): a
//! long-lived SSE stream that emits one frame the instant a new save version
//! commits. This module holds that stream open and, on each `save` frame,
//! force-restores the advanced save when "sync global" is on — exactly what the
//! cloud poller does on a Realtime push.
//!
//! Like its cloud sibling this is strictly an accelerator. It never restores
//! anything on its own beyond the explicit `force_restore` under sync global;
//! plain auto-restore stays the agent sweep's job (the airbag). Every failure
//! path is best-effort: log, back off, reconnect. If the stream never connects
//! (old server, proxy buffering, whatever) the sweep keeps working unchanged.
//!
//! Why its own `reqwest::Client` and not the agent's `ApiClient`: the latter
//! sets a 60 s total request timeout, which would guillotine an idle-but-healthy
//! SSE stream every minute. Here we use no total timeout and instead treat a
//! gap longer than the server's 25 s keep-alive as a dead socket worth
//! reconnecting.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::StreamExt;
use serde::Deserialize;
use tauri::{AppHandle, Manager};
use tokio::task::JoinHandle;
use tokio::time::sleep;

use crate::state::AppState;

/// Reconnect backoff bounds. Mirrors `cloud_realtime`.
const BACKOFF_MIN_SECS: u64 = 2;
const BACKOFF_MAX_SECS: u64 = 60;

/// How long to wait for *any* byte (data or keep-alive) before declaring the
/// socket dead and reconnecting. The server sends a keep-alive every 25 s, so
/// a full minute of silence means the connection is gone.
const READ_IDLE_SECS: u64 = 60;

/// Fail fast on the initial TCP/TLS handshake; the body read itself is
/// unbounded (SSE is long-lived).
const CONNECT_TIMEOUT_SECS: u64 = 15;

/// Server `event: save` payload. `version_num` is merged into the engine's
/// head cache so `cloud_ahead` can fire — a bare force-restore tick is a
/// no-op when that cache is empty (self-hosted has no cloud manifest).
#[derive(Debug, Deserialize)]
struct SaveEvent {
    save_id: String,
    version_num: i64,
}

/// Managed singleton holding the active SSE task, if any. Mirrors
/// `RealtimeScheduler`.
#[derive(Default)]
pub struct SelfHostedEventsScheduler {
    handle: Arc<Mutex<Option<JoinHandle<()>>>>,
}

/// Cancel any in-flight subscriber and start a fresh one. Safe to call
/// repeatedly.
pub fn start(app: &AppHandle) {
    let scheduler = app.state::<SelfHostedEventsScheduler>();
    {
        let mut slot = scheduler.handle.lock().unwrap();
        if let Some(prev) = slot.take() {
            prev.abort();
        }
    }

    let app = app.clone();
    let new_handle = tokio::task::spawn(async move {
        tracing::info!("selfhosted-events: subscriber started");
        run_loop(app).await;
        tracing::info!("selfhosted-events: subscriber stopped (signed out)");
    });

    let mut slot = scheduler.handle.lock().unwrap();
    *slot = Some(new_handle);
}

/// Abort the running subscriber. No-op when nothing is scheduled.
pub fn stop(app: &AppHandle) {
    let scheduler = app.state::<SelfHostedEventsScheduler>();
    let mut slot = scheduler.handle.lock().unwrap();
    if let Some(prev) = slot.take() {
        prev.abort();
        tracing::info!("selfhosted-events: subscriber aborted");
    }
}

fn signed_in(app: &AppHandle) -> bool {
    app.state::<AppState>().user.lock().unwrap().is_some()
}

/// Self-hosted session creds: `(base_url, bearer_token)`. `None` when signed
/// out — o cuando el servicio, que es quien guarda el token (D.20), no está para
/// prestarlo.
async fn session(app: &AppHandle) -> Option<(String, String)> {
    let base = app
        .state::<AppState>()
        .user
        .lock()
        .unwrap()
        .as_ref()
        .map(|u| u.server_url.clone())?;
    let token = crate::commands::auth::server_session(app)
        .await
        .ok()??
        .token;
    Some((base, token))
}

/// Outer reconnect loop. Exits only when the user is signed out; every other
/// failure backs off and retries.
async fn run_loop(app: AppHandle) {
    // One client for the whole loop: reqwest pools connections internally and a
    // fresh socket is established per `connect_once` anyway.
    let client = match reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(CONNECT_TIMEOUT_SECS))
        .user_agent(concat!("hoard-desktop/", env!("CARGO_PKG_VERSION")))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "selfhosted-events: HTTP client build failed");
            return;
        }
    };

    let mut backoff = BACKOFF_MIN_SECS;
    loop {
        if !signed_in(&app) {
            return;
        }
        match connect_once(&app, &client).await {
            Ok(()) => backoff = BACKOFF_MIN_SECS,
            Err(e) => {
                tracing::debug!(error = %e, "selfhosted-events: connection ended, will retry");
            }
        }
        if !signed_in(&app) {
            return;
        }
        sleep(Duration::from_secs(backoff)).await;
        backoff = (backoff * 2).min(BACKOFF_MAX_SECS);
    }
}

/// One connection lifecycle: open the stream and pump frames until the socket
/// dies, the keep-alive gap is exceeded, or the user signs out.
async fn connect_once(app: &AppHandle, client: &reqwest::Client) -> anyhow::Result<()> {
    let (base, token) = session(app)
        .await
        .ok_or_else(|| anyhow::anyhow!("signed out"))?;
    let url = format!("{}/v1/events", base.trim_end_matches('/'));

    let resp = client
        .get(&url)
        .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
        .header(reqwest::header::ACCEPT, "text/event-stream")
        .send()
        .await?;
    if !resp.status().is_success() {
        anyhow::bail!("/v1/events returned {}", resp.status());
    }
    tracing::debug!("selfhosted-events: stream open");

    // SSE framing: events are separated by a blank line; within an event,
    // `event:` sets the type and `data:` lines accumulate the body. We dispatch
    // on the blank-line boundary. Lines starting with `:` are comments
    // (keep-alives) and ignored.
    let mut stream = resp.bytes_stream();
    let mut buf = String::new();
    let mut ev_type = String::new();
    let mut ev_data = String::new();

    loop {
        let next = tokio::time::timeout(Duration::from_secs(READ_IDLE_SECS), stream.next()).await;
        let chunk = match next {
            Err(_) => anyhow::bail!("idle timeout — no keep-alive, socket presumed dead"),
            Ok(None) => return Ok(()), // server closed the stream cleanly
            Ok(Some(Err(e))) => return Err(e.into()),
            Ok(Some(Ok(c))) => c,
        };
        buf.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(pos) = buf.find('\n') {
            let line = buf[..pos].trim_end_matches('\r').to_string();
            buf.drain(..=pos);

            if line.is_empty() {
                if !ev_type.is_empty() || !ev_data.is_empty() {
                    handle_event(app, &ev_type, &ev_data).await;
                }
                ev_type.clear();
                ev_data.clear();
            } else if let Some(rest) = line.strip_prefix("event:") {
                ev_type = rest.trim().to_string();
            } else if let Some(rest) = line.strip_prefix("data:") {
                if !ev_data.is_empty() {
                    ev_data.push('\n');
                }
                ev_data.push_str(rest.strip_prefix(' ').unwrap_or(rest));
            }
            // `:`-prefixed comments and unknown fields fall through.
        }
    }
}

/// Act on one decoded SSE event. `save` → force-restore the advanced save when
/// sync global is on (the low-latency complement to the agent sweep). `lagged`
/// means we missed events: there's no manifest to diff on self-hosted, so we
/// leave reconciliation to the sweep rather than blindly restoring.
async fn handle_event(app: &AppHandle, ev_type: &str, data: &str) {
    match ev_type {
        "save" => {
            let Some(ev) = serde_json::from_str::<SaveEvent>(data).ok() else {
                tracing::debug!(data = %data, "selfhosted-events: unparseable save frame");
                return;
            };
            let save_id = ev.save_id;
            let version_num = ev.version_num;
            // Mirror the cloud poller: only the explicit "sync global" path
            // restores in-the-moment. Plain auto-restore is the sweep's job.
            let global_sync = hoard_agent::prefs::Prefs::load_default()
                .map(|(p, _)| p.global_sync)
                .unwrap_or(false);
            if !global_sync {
                return;
            }
            // El motor está en el servicio desde el Slice 4b, así que esto va
            // por IPC. Sigue siendo el único empuje servidor→cliente que tiene
            // el self-hosted: `hoardd` monta Realtime sólo para Cloud, y la SSE
            // necesita las credenciales self-hosted que vive aquí.
            let Some(state) = app.try_state::<AppState>() else {
                return;
            };
            tracing::debug!(
                save_id = %save_id,
                version_num,
                "selfhosted-events: push → force-restore"
            );
            state
                .daemon
                .tell(
                    "force-restore a save the server just advanced",
                    hoard_core::ipc::Request::ForceRestore {
                        save_id,
                        version_num: Some(version_num),
                    },
                )
                .await;
        }
        "lagged" => {
            tracing::debug!("selfhosted-events: lagged — relying on agent sweep to reconcile");
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_event_keeps_version_num() {
        let ev: SaveEvent = serde_json::from_str(
            r#"{"save_id":"c9a456a6-0c24-4af3-9b16-2e7a2ccaa1c5","version_num":4}"#,
        )
        .unwrap();
        assert_eq!(ev.save_id, "c9a456a6-0c24-4af3-9b16-2e7a2ccaa1c5");
        assert_eq!(ev.version_num, 4);
    }
}
