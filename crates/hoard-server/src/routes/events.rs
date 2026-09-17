//! Server→app push (Server-Sent Events).
//!
//! Without it the only "another device uploaded" path is the agent's
//! reconciliation sweep, up to a cooldown of latency. Clients open a
//! long-lived `GET /v1/events` SSE stream and the commit path publishes a
//! [`SaveEvent`] the instant a new version lands, so the listening device
//! pulls within ~1s. Self-hosted serves it from here; cloud's handler is
//! `cloud::routes::events`, over the same bus and frames.
//!
//! Reverse-proxy note: SSE needs response buffering disabled. Nginx works with
//! `proxy_buffering off;` + `proxy_set_header X-Accel-Buffering no;` and a high
//! `proxy_read_timeout` on the `/v1/events` location; Caddy streams correctly
//! out of the box. The 25 s keep-alive comment below also keeps idle
//! connections from being reaped.

use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::{Extension, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use futures::Stream;
use serde::Serialize;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::routes::health::ServerState;

/// One "save version landed" notification. The client only needs to know which
/// save advanced; the version number lets it short-circuit if it already has
/// that version (e.g. it was this very device's own upload echoed back).
#[derive(Clone, Debug, Serialize)]
pub struct SaveEvent {
    pub save_id: String,
    pub version_num: i64,
}

/// Bounded backlog per user channel. A device that falls this far behind gets a
/// `lagged` hint and does a full catch-up pull, so a small buffer is plenty.
const CHANNEL_CAPACITY: usize = 64;

/// Per-user fan-out of [`SaveEvent`]s. A user's `broadcast` channel is created
/// lazily on their first `/v1/events` subscribe; the snapshot-commit path
/// publishes into it. Lives in [`ServerState`]; uses interior mutability so it
/// can sit behind the shared `Arc<ServerState>` without its own lock dance at
/// the call sites. Cloud keeps its own in `CloudState`.
#[derive(Default)]
pub struct EventBus {
    inner: Mutex<HashMap<Uuid, broadcast::Sender<SaveEvent>>>,
}

impl EventBus {
    /// Subscribe `user` to their event stream, creating the channel on first
    /// use. The returned receiver keeps the channel alive for as long as the
    /// SSE connection holds it.
    pub fn subscribe(&self, user: Uuid) -> broadcast::Receiver<SaveEvent> {
        // A poisoned lock only means another thread panicked mid-insert; the
        // map is still usable, so keep serving instead of panicking the request.
        let mut map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        map.entry(user)
            .or_insert_with(|| broadcast::channel(CHANNEL_CAPACITY).0)
            .subscribe()
    }

    /// End every open stream. A graceful shutdown waits for responses in
    /// flight, and an SSE response never finishes by itself: without this each
    /// restart sat out the whole drain grace before exiting. Dropping the
    /// senders closes the channels, the streams return `None`, the clients
    /// reconnect to whatever comes up next.
    pub fn close_all(&self) {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }

    /// Publish `ev` to every device of `user` currently listening. A cheap
    /// no-op when nobody is connected. `broadcast::Sender::send` errors only
    /// when there are zero live receivers, so that case doubles as our cue to
    /// drop the channel and keep the map from growing without bound.
    pub fn publish(&self, user: Uuid, ev: SaveEvent) {
        let mut map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(tx) = map.get(&user) {
            if tx.send(ev).is_err() {
                map.remove(&user);
            }
        }
    }
}

/// `GET /v1/events`: long-lived SSE stream of the authenticated user's save
/// changes. Emits `event: save` frames carrying the JSON [`SaveEvent`], and an
/// `event: lagged` hint if the client fell behind (it should respond with a
/// full catch-up pull). A 25 s keep-alive comment keeps idle proxies from
/// cutting the connection.
pub async fn stream(
    State(state): State<Arc<ServerState>>,
    Extension(user): Extension<AuthUser>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    sse(state.events.subscribe(user.user_id))
}

/// The frames both deployments send for one subscription. Cloud's handler
/// lives in `cloud::routes::events`, behind its own auth.
pub fn sse(
    rx: broadcast::Receiver<SaveEvent>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = futures::stream::unfold(rx, |mut rx| async move {
        match rx.recv().await {
            Ok(ev) => {
                let frame = Event::default()
                    .event("save")
                    .json_data(&ev)
                    .unwrap_or_else(|_| Event::default().comment("encode error"));
                Some((Ok(frame), rx))
            }
            // Buffer overflow: we dropped some events. Tell the client to
            // reconcile from scratch rather than trust the stream.
            Err(broadcast::error::RecvError::Lagged(_)) => {
                Some((Ok(Event::default().event("lagged").data("")), rx))
            }
            // Sender gone, which means the server is shutting down
            // (`close_all`): end the stream so the client reconnects.
            Err(broadcast::error::RecvError::Closed) => None,
        }
    });
    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(25)).text(""))
}
