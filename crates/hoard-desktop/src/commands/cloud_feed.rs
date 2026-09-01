//! Device presence + operator broadcasts: the fetch-and-emit half of the Eye
//! panel and the bell.
//!
//! Two feeds, same plumbing:
//!
//! - **Devices** (`hoard://devices`): GET `/v1/devices`, every machine on
//!   the account with its live state (online dot, playing what, since when).
//!   The whole response body is re-emitted verbatim as the Tauri payload; the
//!   UI filters out `this_device` and renders the rest under this machine.
//! - **Notifications** (`hoard://notification`, one event per row): GET
//!   `/v1/notifications?since=<cursor>`, operator broadcasts for the bell.
//!   The cursor (RFC3339 of the newest row ever seen) persists in the state
//!   dir so a restart never re-delivers; the UI's own localStorage dedup by
//!   id is the second belt.
//!
//! Kicked from two places, mirroring the `cloud_pull` design: the Realtime
//! subscriber (`cloud_realtime`) on every pushed `devices`/`notifications`
//! change and on (re)join, which is the "responds within a second" path, and
//! the timed poller as fallback (at most every [`FALLBACK_MIN_SECS`]) for
//! when the socket is down. Each feed runs behind a single-flight gate with a
//! short spacing floor so a burst of Realtime events (e.g. three devices'
//! keepalives landing together) collapses into one HTTP fetch.
//!
//! Cloud-only by construction: both endpoints exist only on the cloud
//! server, and every kick path (Realtime, cloud poller) already runs only
//! with a cloud session. Best-effort throughout: a failed fetch logs at
//! debug and waits for the next kick, and there is nothing to roll back.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use tokio::time::Instant;

/// Spacing floor between fetches of the same feed. Keepalive heartbeats
/// arrive every ~30s per sibling device, each one pushing a `devices` UPDATE
/// through Realtime; without a floor every beat would cost one GET. 10s
/// keeps a many-device account at 6 GETs/min or fewer, safely under the server's
/// per-device poll guard (10/min), and beats still land within one beat interval of
/// each other.
const SPACING_SECS: u64 = 10;

/// The timed poller may tick as fast as every 5s; the fallback feed refresh
/// doesn't need that, since Realtime covers immediacy. Cap it to once a minute.
pub const FALLBACK_MIN_SECS: u64 = 60;

/// Managed singleton: one gate per feed + the notifications cursor cache.
#[derive(Default)]
pub struct CloudFeed {
    devices_gate: Arc<Mutex<FeedGate>>,
    notif_gate: Arc<Mutex<FeedGate>>,
}

/// Single-flight + coalescing state, same shape as `cloud_pull::PullGate`
/// plus a spacing floor.
#[derive(Default)]
struct FeedGate {
    running: bool,
    rerun: bool,
    last_done: Option<Instant>,
}

/// The managed [`CloudFeed`], or `None` (logged) if it was never registered.
///
/// **This is the ADR 0021 D.12 panic.** `Manager::state::<T>()` panics when `T`
/// was never `.manage()`d, and `CloudFeed` never was, so every `kick_*` call
/// killed *its caller's task*, synchronously, before any of the spawn/gate
/// machinery below ran. The cloud-pull poller calls `kick_all` right after its
/// first pull, which is exactly the observed failure: two ticks in the first
/// second, then a task that no longer exists, with no `gate busy`, no `stopped`
/// and no log line at all, and the engine blind for the rest of the session. The
/// realtime socket died the same way on its (re)subscribe catch-up.
///
/// Registering it in `lib.rs` is the fix; going through `try_state` is the
/// guard, so a future mis-wiring degrades to one warning and a dead feed
/// instead of taking down whatever loop happened to call it.
fn feed_state(app: &AppHandle) -> Option<tauri::State<'_, CloudFeed>> {
    let state = app.try_state::<CloudFeed>();
    if state.is_none() {
        tracing::warn!("cloud-feed: state not managed, devices and bell feeds are inert");
    }
    state
}

/// Refresh the devices feed now (single-flight, spaced). Fire-and-forget.
pub fn kick_devices(app: &AppHandle) {
    let Some(state) = feed_state(app) else {
        return;
    };
    let gate = state.devices_gate.clone();
    let app = app.clone();
    tokio::task::spawn(async move {
        guarded(&gate, || fetch_devices_once(&app)).await;
    });
}

/// Refresh the notifications feed now (single-flight, spaced). Fire-and-forget.
pub fn kick_notifications(app: &AppHandle) {
    let Some(state) = feed_state(app) else {
        return;
    };
    let gate = state.notif_gate.clone();
    let app = app.clone();
    tokio::task::spawn(async move {
        guarded(&gate, || fetch_notifications_once(&app)).await;
    });
}

/// Both feeds. The Realtime (re)join and the poller fallback want them
/// together.
pub fn kick_all(app: &AppHandle) {
    kick_devices(app);
    kick_notifications(app);
}

/// Run `fetch` behind the gate: coalesce concurrent kicks into at most one
/// extra pass, and keep [`SPACING_SECS`] between passes.
async fn guarded<F, Fut>(gate: &Arc<Mutex<FeedGate>>, fetch: F)
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    {
        let mut g = gate.lock().unwrap();
        if g.running {
            g.rerun = true;
            return;
        }
        g.running = true;
    }
    loop {
        // Spacing floor, measured from the previous completed pass.
        let wait = {
            let g = gate.lock().unwrap();
            g.last_done
                .map(|t| Duration::from_secs(SPACING_SECS).saturating_sub(t.elapsed()))
                .unwrap_or(Duration::ZERO)
        };
        if !wait.is_zero() {
            tokio::time::sleep(wait).await;
        }

        fetch().await;

        let mut g = gate.lock().unwrap();
        g.last_done = Some(Instant::now());
        if g.rerun {
            g.rerun = false;
        } else {
            g.running = false;
            return;
        }
    }
}

/// Authenticated GET with the same 401-retry-once dance as `cloud_pull`: the
/// Supabase JWT rotates hourly and a stale one must never kill a feed. The
/// replacement token is **borrowed** from the service rather than rotated here (ADR
/// 0021: one rotator only). Returns the response body on 2xx, `None` (logged)
/// otherwise.
async fn authed_get(app: &AppHandle, path_and_query: &str) -> Option<String> {
    let creds = match crate::commands::cloud::active_creds(app).await {
        Ok(Some(c)) => c,
        Ok(None) => return None, // signed out between kicks: quiet exit
        Err(e) => {
            tracing::debug!(error = %e, "cloud-feed: couldn't get a token for the session");
            return None;
        }
    };
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent(concat!("hoard-desktop/", env!("CARGO_PKG_VERSION")))
        .build()
        .ok()?;

    // Fingerprint header so `/v1/devices` marks `this_device` for the UI.
    let fp = hoard_agent::logship::device_identity().fingerprint;
    let url = format!("{}{}", creds.server_url, path_and_query);
    let send = |token: String| {
        let client = client.clone();
        let url = url.clone();
        let fp = fp.clone();
        async move {
            client
                .get(&url)
                .bearer_auth(token)
                .header("x-hoard-device-fp", fp)
                .send()
                .await
        }
    };

    let mut resp = match send(creds.access_token.clone()).await {
        Ok(r) => r,
        Err(e) => {
            tracing::debug!(error = %e, "cloud-feed: network error");
            return None;
        }
    };
    if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
        let fresh =
            crate::commands::cloud::borrow_access_token(app, Some(creds.access_token.clone()))
                .await
                .ok()?;
        resp = match send(fresh.access_token).await {
            Ok(r) => r,
            Err(e) => {
                tracing::debug!(error = %e, "cloud-feed: network error after refresh");
                return None;
            }
        };
    }
    if !resp.status().is_success() {
        tracing::debug!(status = %resp.status(), url = %url, "cloud-feed: non-2xx");
        return None;
    }
    resp.text().await.ok()
}

async fn fetch_devices_once(app: &AppHandle) {
    let Some(body) = authed_get(app, "/v1/devices").await else {
        return;
    };
    // Re-emit the wire payload as-is; the UI owns the shape. Parsing first
    // keeps a malformed body from reaching the frontend.
    match serde_json::from_str::<serde_json::Value>(&body) {
        Ok(v) if v.get("devices").is_some() => {
            let _ = app.emit("hoard://devices", &v);
        }
        _ => tracing::debug!("cloud-feed: unexpected /v1/devices body"),
    }
}

/// Fetch the current broadcasts (the server caps at 10 non-expired, filtered
/// per-user by created_at + dismissals) and emit a `hoard://notifications-snapshot`
/// event with the FULL list so the UI can reconcile: add new rows AND remove
/// server-sourced entries the server no longer delivers (dismissed on another
/// device, expired, etc.). The server is the authority for server notifications
/// now (migration 0033 plus cloud/routes/notifications.rs), so the UI must drop
/// stale localStorage entries that the server's filtered list excludes: without
/// this snapshot a notification dismissed on machine A would persist
/// on machine B until B is restarted (localStorage resurrects it).
///
/// Individual `hoard://notification` events are NOT emitted anymore: the
/// snapshot carries the full list and the UI's `reconcileServer` handles both
/// additions and removals. Boot-time emits that race the webview mount are
/// covered by the UI pulling [`notifications_backlog`] once its listener is
/// armed (the backlog returns the same filtered list and goes through the same
/// reconcile path).
async fn fetch_notifications_once(app: &AppHandle) {
    let Some(rows) = fetch_notification_rows(app).await else {
        return;
    };
    let snapshot = serde_json::json!({ "notifications": rows });
    let _ = app.emit("hoard://notifications-snapshot", &snapshot);
}

/// GET the broadcast list, newest-first, as raw JSON rows (the wire shape
/// already matches the UI's `ServerNotification`, `created_at` included).
async fn fetch_notification_rows(app: &AppHandle) -> Option<Vec<serde_json::Value>> {
    let body = authed_get(app, "/v1/notifications").await?;
    match serde_json::from_str::<serde_json::Value>(&body) {
        Ok(mut v) => match v.get_mut("notifications").map(serde_json::Value::take) {
            Some(serde_json::Value::Array(rows)) => Some(rows),
            _ => {
                tracing::debug!("cloud-feed: unexpected /v1/notifications body");
                None
            }
        },
        Err(e) => {
            tracing::debug!(error = %e, "cloud-feed: unexpected /v1/notifications body");
            None
        }
    }
}

/// The bell's boot-time catch-up, invoked by the UI right after it arms its
/// `hoard://notification` listener. Returning the rows as the command's
/// response (instead of events) makes the handoff race-free: events emitted
/// while the webview was still loading are simply lost, which is exactly how
/// a broadcast sent while the app was closed used to vanish. Newest-first,
/// as served.
#[tauri::command]
pub async fn notifications_backlog(app: AppHandle) -> Result<Vec<serde_json::Value>, String> {
    Ok(fetch_notification_rows(&app).await.unwrap_or_default())
}

/// Companion for the Eye panel: the UI calls this after arming its
/// `hoard://devices` listener so the first snapshot lands post-mount instead
/// of racing the webview load.
#[tauri::command]
pub async fn devices_refresh(app: AppHandle) -> Result<(), String> {
    kick_devices(&app);
    Ok(())
}

/// `POST /v1/notifications/:id/dismiss`: record a per-user, cross-device
/// dismissal on the server. Thin wrapper over `cloud_account::dismiss_notification`
/// with the same 401-refresh-once dance as `cloud_entitlements`: the Supabase
/// JWT rotates hourly and a stale one must never block a dismiss. The UI calls
/// this fire-and-forget (optimistic: the localStorage tombstone hides the row
/// immediately, and a dropped network call just means the server picks the
/// dismissal up on a future dismiss/retry).
#[tauri::command]
pub async fn notification_dismiss(app: AppHandle, id: String) -> Result<(), String> {
    use crate::commands::cloud::{
        active_creds_or_msg, borrow_access_token, cloud_err_to_string, handle_session_expired,
        is_session_expired,
    };
    use hoard_agent::cloud_account::{self, CloudError};
    let creds = active_creds_or_msg(&app).await?;
    match cloud_account::dismiss_notification(&creds.server_url, &creds.access_token, &id).await {
        Ok(()) => Ok(()),
        Err(CloudError::Unauthorized) => {
            let fresh = borrow_access_token(&app, Some(creds.access_token.clone()))
                .await
                .map_err(|e| {
                    if is_session_expired(&e) {
                        handle_session_expired(&app);
                    }
                    e.to_string()
                })?;
            cloud_account::dismiss_notification(&fresh.server_url, &fresh.access_token, &id)
                .await
                .map_err(cloud_err_to_string)
        }
        Err(other) => Err(cloud_err_to_string(other)),
    }
}
