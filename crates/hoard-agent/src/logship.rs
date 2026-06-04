//! Ship `tracing` events to the connected server for centralized diagnostics.
//!
//! A [`LogShipLayer`] is added to the process's `tracing` subscriber (desktop
//! and CLI). Each event is serialized and pushed onto a bounded channel with
//! **drop-on-full** semantics: a slow or dead network must never block or
//! crash the app, exactly like the non-blocking local file appender.
//!
//! A dedicated background thread (own current-thread Tokio runtime, so it
//! works regardless of the host's runtime) drains the channel in batches and
//! POSTs them to the server. It only ships when a valid session exists
//! (`credentials::load`) and the server advertises a log-ingest level via
//! `/v1/health`; otherwise events are discarded. The server dictates the
//! minimum level (self-hosted: DEBUG, cloud: INFO), so the client filters at
//! source and never sends below it.

use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TrySendError};
use std::time::Duration;

use serde::Serialize;
use tracing::field::{Field, Visit};
use tracing_subscriber::layer::{Context, Layer};

use crate::api::Health;
use crate::credentials;

/// Channel capacity. Bursty startup logging can momentarily exceed the drain
/// rate; past this we drop, which is fine for diagnostics.
const CHANNEL_CAPACITY: usize = 2048;
/// Max entries per POST (mirrors the server cap).
const MAX_BATCH: usize = 500;
/// How long to accumulate before flushing a non-empty batch.
const FLUSH_INTERVAL: Duration = Duration::from_secs(5);

/// A single captured event, pre-serialized into wire shape.
#[derive(Debug, Clone, Serialize)]
struct WireEntry {
    level: String,
    target: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    fields: Option<serde_json::Value>,
    ts: String,
}

#[derive(Debug, Clone, Serialize)]
struct DeviceMeta {
    name: Option<String>,
    os: Option<String>,
    fingerprint: Option<String>,
    app_version: Option<String>,
}

#[derive(Debug, Serialize)]
struct Batch<'a> {
    device: &'a DeviceMeta,
    entries: Vec<WireEntry>,
}

/// `tracing` layer that forwards events onto the ship channel.
pub struct LogShipLayer {
    tx: SyncSender<WireEntry>,
}

/// Build the layer and spawn the background shipper thread. Returns the layer
/// to be `.with(...)`-ed onto the subscriber registry. Cheap and infallible —
/// if the thread can't spawn, the layer simply drops everything.
pub fn start() -> LogShipLayer {
    let (tx, rx) = sync_channel::<WireEntry>(CHANNEL_CAPACITY);
    let _ = std::thread::Builder::new()
        .name("hoard-logship".into())
        .spawn(move || drain_loop(rx));
    LogShipLayer { tx }
}

impl<S> Layer<S> for LogShipLayer
where
    S: tracing::Subscriber,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let meta = event.metadata();
        let target = meta.target();
        // Never ship our own shipper logs — that would feed back into the
        // channel and (worse) loop network errors into more events.
        if target.starts_with("hoard_agent::logship") {
            return;
        }

        let mut visitor = FieldVisitor::default();
        event.record(&mut visitor);

        let fields = if visitor.fields.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(visitor.fields))
        };

        let entry = WireEntry {
            level: meta.level().as_str().to_ascii_lowercase(),
            target: target.to_string(),
            message: visitor.message.unwrap_or_default(),
            fields,
            ts: time::OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default(),
        };

        // Drop-on-full: diagnostics must never block the app.
        match self.tx.try_send(entry) {
            Ok(()) | Err(TrySendError::Full(_)) => {}
            Err(TrySendError::Disconnected(_)) => {}
        }
    }
}

/// Collects an event's fields into a JSON object, pulling out the special
/// `message` field that `tracing` uses for the format string.
#[derive(Default)]
struct FieldVisitor {
    message: Option<String>,
    fields: serde_json::Map<String, serde_json::Value>,
}

impl Visit for FieldVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        let rendered = format!("{value:?}");
        if field.name() == "message" {
            self.message = Some(rendered);
        } else {
            self.fields.insert(
                field.name().to_string(),
                serde_json::Value::String(rendered),
            );
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_string());
        } else {
            self.fields.insert(
                field.name().to_string(),
                serde_json::Value::String(value.to_string()),
            );
        }
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.fields
            .insert(field.name().to_string(), serde_json::Value::from(value));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.fields
            .insert(field.name().to_string(), serde_json::Value::from(value));
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.fields
            .insert(field.name().to_string(), serde_json::Value::Bool(value));
    }
}

// ---- background shipper -------------------------------------------------

fn level_rank(level: &str) -> u8 {
    match level {
        "trace" => 0,
        "debug" => 1,
        "warn" => 3,
        "error" => 4,
        _ => 2, // info + unknown
    }
}

/// Resolved server policy for the current session.
struct Policy {
    /// Full ingest URL (already joined with base_url).
    url: String,
    token: String,
    min_rank: u8,
}

fn drain_loop(rx: Receiver<WireEntry>) {
    // One current-thread runtime for all network I/O on this thread.
    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(_) => {
            // No runtime → just drain and discard forever so senders don't
            // wedge on a full channel.
            while rx.recv().is_ok() {}
            return;
        }
    };

    let device = device_meta();
    let client = reqwest::Client::builder()
        .user_agent(concat!("hoard-agent/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(30))
        .build()
        .ok();

    loop {
        // 1. Wait for a usable session + a server that accepts logs.
        let policy = match client.as_ref().and_then(|c| rt.block_on(resolve_policy(c))) {
            Some(p) => p,
            None => {
                // No session/endpoint yet. Discard whatever queued so we don't
                // hold stale lines or wedge senders, then back off.
                discard_available(&rx);
                std::thread::sleep(Duration::from_secs(15));
                continue;
            }
        };
        let client = client.as_ref().unwrap();

        // 2. Ship until the session changes or the channel closes.
        loop {
            let batch = collect_batch(&rx, policy.min_rank);
            match batch {
                BatchResult::Closed => return,
                BatchResult::Empty => {}
                BatchResult::Entries(entries) => {
                    let body = Batch {
                        device: &device,
                        entries,
                    };
                    let _ = rt.block_on(post_batch(client, &policy, &body));
                }
            }

            // Re-validate the session roughly every loop; if it vanished or the
            // token rotated, drop back to the outer loop to re-resolve.
            if !session_matches(&policy) {
                break;
            }
        }
    }
}

enum BatchResult {
    Entries(Vec<WireEntry>),
    Empty,
    Closed,
}

/// Block up to `FLUSH_INTERVAL` for the first entry, then greedily drain up to
/// `MAX_BATCH`, filtering by level.
fn collect_batch(rx: &Receiver<WireEntry>, min_rank: u8) -> BatchResult {
    let first = match rx.recv_timeout(FLUSH_INTERVAL) {
        Ok(e) => e,
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => return BatchResult::Empty,
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return BatchResult::Closed,
    };

    let mut out = Vec::new();
    if level_rank(&first.level) >= min_rank {
        out.push(first);
    }
    while out.len() < MAX_BATCH {
        match rx.try_recv() {
            Ok(e) => {
                if level_rank(&e.level) >= min_rank {
                    out.push(e);
                }
            }
            Err(_) => break,
        }
    }

    if out.is_empty() {
        BatchResult::Empty
    } else {
        BatchResult::Entries(out)
    }
}

fn discard_available(rx: &Receiver<WireEntry>) {
    while rx.try_recv().is_ok() {}
}

/// Read the session, probe `/v1/health`, and decide the ingest endpoint +
/// minimum level. Returns `None` when there's no session or the server can't
/// receive logs.
async fn resolve_policy(client: &reqwest::Client) -> Option<Policy> {
    let creds = credentials::load().ok().flatten()?;
    let base = creds.url.trim_end_matches('/').to_string();

    let health: Health = client
        .get(format!("{base}/v1/health"))
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;

    // Server doesn't advertise ingest → unsupported, don't ship.
    let min_level = health.log_min_level.as_deref()?;
    let min_rank = level_rank(&min_level.to_ascii_lowercase());

    let path = if health.mode.as_deref() == Some("cloud") {
        "/v1/cloud/logs"
    } else {
        "/v1/logs"
    };

    Some(Policy {
        url: format!("{base}{path}"),
        token: creds.token,
        min_rank,
    })
}

/// Cheap re-check: is there still a session whose token matches the policy we
/// resolved? Avoids a full health round-trip on every batch.
fn session_matches(policy: &Policy) -> bool {
    matches!(credentials::load(), Ok(Some(c)) if c.token == policy.token)
}

async fn post_batch(
    client: &reqwest::Client,
    policy: &Policy,
    body: &Batch<'_>,
) -> Result<(), reqwest::Error> {
    client
        .post(&policy.url)
        .header("authorization", format!("Bearer {}", policy.token))
        .json(body)
        .send()
        .await?;
    Ok(())
}

fn device_meta() -> DeviceMeta {
    let id = device_identity();
    DeviceMeta {
        name: id.name,
        os: Some(id.os),
        fingerprint: Some(id.fingerprint),
        app_version: Some(env!("CARGO_PKG_VERSION").to_string()),
    }
}

/// Stable identity of this machine, shared by log shipping and cloud device
/// registration (the `Dispositivos N/M` counter on the account page). Keeping
/// one source of truth means the fingerprint a log row carries matches the one
/// the device-list upsert keys on.
pub struct DeviceIdentity {
    pub name: Option<String>,
    pub os: String,
    pub fingerprint: String,
}

pub fn device_identity() -> DeviceIdentity {
    let hostname = sysinfo::System::host_name();
    DeviceIdentity {
        fingerprint: fingerprint(hostname.as_deref()),
        os: std::env::consts::OS.to_string(),
        name: hostname,
    }
}

/// Stable per-machine id: hash of `/etc/machine-id` (Linux) plus hostname,
/// falling back to the hostname alone when the machine-id is unreadable.
fn fingerprint(hostname: Option<&str>) -> String {
    use sha2::{Digest, Sha256};
    let machine_id = std::fs::read_to_string("/etc/machine-id")
        .ok()
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(machine_id.as_bytes());
    hasher.update(b"|");
    hasher.update(hostname.unwrap_or("unknown").as_bytes());
    hex::encode(hasher.finalize())
}
