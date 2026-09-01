//! Ship `tracing` events to the connected server for centralized diagnostics.
//!
//! A [`LogShipLayer`] is added to the process's `tracing` subscriber (desktop and
//! CLI). Each event is serialized and pushed onto a bounded channel with
//! drop-on-full semantics: a slow or dead network must never block or crash the
//! app, exactly like the non-blocking local file appender.
//!
//! Everything entering the channel is redacted: the profile segment of any path is
//! replaced with `<user>` in [`Layer::on_event`], before the entry even exists. It
//! happens there rather than at send time so an unredacted batch never gets to
//! exist in this process. The local file log does not pass through here and stays
//! whole, which is what makes it useful for debugging on the user's machine.
//!
//! A dedicated background thread (its own current-thread Tokio runtime, so it
//! works regardless of the host's runtime) drains the channel in batches and POSTs
//! them to the server. It only ships when the user has opted in
//! (`prefs.anonymous_telemetry`), a session exists, and the server advertises a
//! log-ingest level via `/v1/health`; otherwise events are discarded. The opt-in is
//! read fresh each cycle, so toggling it off stops shipping within a few seconds
//! without a restart. The server dictates the minimum level (self-hosted: DEBUG,
//! cloud: WARN), so the client filters at source and never sends below it, with one
//! exception: the detection contradictions ([`TELEMETRY_TARGET`]) travel whatever
//! their level.
//!
//! ## Where the session comes from, and why this used to ship nothing
//!
//! [`current_session`] looks in two slots, and that is the fix: the self-hosted
//! session lives in `credentials` and the Cloud one in `cloud_auth`, two disjoint
//! stores. This reader only looked at the first, so for a machine signed into
//! Cloud, meaning the entire cloud population, it resolved `None` on every pass and
//! `client_logs` has carried zero rows since it existed. Cloud now comes in through
//! `credentials::lent_cloud`, the slot filled by whoever holds a fresh JWT (the
//! service on each rotation, a client when it borrows one); we still cannot ask for
//! anything over IPC from this thread.

use std::borrow::Cow;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TrySendError};
use std::sync::OnceLock;
use std::time::Duration;

use tracing::field::{Field, Visit};
use tracing_subscriber::layer::{Context, Layer};

use crate::api::Health;
use crate::credentials;
use hoard_core::ids::MachineId;

/// Channel capacity. Bursty startup logging can momentarily exceed the drain
/// rate; past this we drop, which is fine for diagnostics.
const CHANNEL_CAPACITY: usize = 2048;
/// Max entries per POST (mirrors the server cap).
const MAX_BATCH: usize = 500;
/// How long to accumulate before flushing a non-empty batch.
const FLUSH_INTERVAL: Duration = Duration::from_secs(5);
/// The wait after a credential rejection. The service's rotator renews every 45
/// minutes or so; asking once a minute is cheap and does not hammer the server with
/// a token we already know is dead.
const REJECTED_BACKOFF: Duration = Duration::from_secs(60);

// The batch body (`LogEntry`, `DeviceMeta`, `LogBatch`) lives in
// `hoard_core::wire`, shared with `hoard_server::routes::logs` (ADR 0021 C.6). This
// pair was real drift: here `target` and `ts` were required and on the server they
// were `Option`, so the "correct" shape depended on which side you looked at.
use hoard_core::wire::{level_rank, ships_at, DeviceMeta, LogBatch, LogEntry};

/// `tracing` layer that forwards events onto the ship channel.
pub struct LogShipLayer {
    tx: SyncSender<LogEntry>,
}

/// Build the layer and spawn the background shipper thread. Returns the layer to
/// be `.with(...)`-ed onto the subscriber registry. Cheap and infallible: if the
/// thread can't spawn, the layer simply drops everything.
pub fn start() -> LogShipLayer {
    let (tx, rx) = sync_channel::<LogEntry>(CHANNEL_CAPACITY);
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
        // Never ship our own shipper logs: that would feed back into the channel
        // and, worse, loop network errors into more events.
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

        let entry = LogEntry {
            level: meta.level().as_str().to_ascii_lowercase(),
            target: Some(target.to_string()),
            message: visitor.message.unwrap_or_default(),
            fields,
            ts: time::OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .ok(),
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
        let rendered = redact(&format!("{value:?}")).into_owned();
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
        let value = redact(value);
        if field.name() == "message" {
            self.message = Some(value.into_owned());
        } else {
            self.fields.insert(
                field.name().to_string(),
                serde_json::Value::String(value.into_owned()),
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

// ---- redaction

/// What replaces the profile segment. The shape of the path is kept, which is what
/// is useful for fixing detection, and the person's name is thrown away, which is
/// useful for nothing.
const PROFILE_TOKEN: &str = "<user>";

/// The folders the person's name comes after: `/home/x`, `C:\Users\x`, and
/// macOS's `/Users/x`.
const PROFILE_DIRS: [&str; 2] = ["home", "users"];

fn is_sep(b: u8) -> bool {
    b == b'/' || b == b'\\'
}

/// The end of the run of separators starting at `from`.
///
/// A run rather than a separator because the text does not always carry the path
/// as-is: `record_debug` renders with `{:?}`, and a string's `Debug` escapes
/// backslashes, so a Windows path arrives as `C:\\Users\\angel`. Looking for a
/// bare `\Users\` would not match that and the name would leave the machine,
/// which is exactly what this exists to prevent. As a side effect it also absorbs
/// `//` and paths with mixed separators.
fn sep_run_end(bytes: &[u8], from: usize) -> usize {
    let mut i = from;
    while i < bytes.len() && is_sep(bytes[i]) {
        i += 1;
    }
    i
}

/// The end of the segment starting at `from`: the next separator, or the end.
fn segment_end(bytes: &[u8], from: usize) -> usize {
    bytes[from..]
        .iter()
        .position(|b| is_sep(*b))
        .map_or(bytes.len(), |p| from + p)
}

fn is_profile_dir(segment: &str) -> bool {
    PROFILE_DIRS
        .iter()
        .any(|dir| segment.eq_ignore_ascii_case(dir))
}

/// Strips the person's name out of any path in the text.
///
/// It returns `Cow::Borrowed` when there is nothing to redact, which is the normal
/// case: this runs in `on_event`, meaning on every log line in the process.
fn redact(input: &str) -> Cow<'_, str> {
    let shaped = redact_markers(input);
    match home_override() {
        Some((home, replacement)) if shaped.contains(home.as_str()) => {
            Cow::Owned(shaped.replace(home.as_str(), replacement))
        }
        _ => shaped,
    }
}

/// The profile-folder pass: `/home/angel/x` becomes `/home/<user>/x`.
///
/// It walks bytes rather than characters: everything compared, separators and
/// folder names, is ASCII, so the indices coming out always land on a character
/// boundary and the slices are safe even when the name carries accents.
fn redact_markers(input: &str) -> Cow<'_, str> {
    let bytes = input.as_bytes();
    let mut out: Option<String> = None;
    // How much of the original has already been flushed into the output buffer.
    let mut copied = 0usize;
    let mut i = 0usize;

    while i < bytes.len() {
        if !is_sep(bytes[i]) {
            i += 1;
            continue;
        }
        let after_sep = sep_run_end(bytes, i);
        // `home` or `users`, with a separator behind it? With no separator behind
        // it there is no profile segment to strip (a path ending in `/home`).
        let Some(dir_len) = PROFILE_DIRS.iter().find_map(|dir| {
            let end = after_sep + dir.len();
            (end < bytes.len()
                && bytes[after_sep..end].eq_ignore_ascii_case(dir.as_bytes())
                && is_sep(bytes[end]))
            .then_some(dir.len())
        }) else {
            i = after_sep;
            continue;
        };

        let mut seg_start = sep_run_end(bytes, after_sep + dir_len);
        let mut seg_end = segment_end(bytes, seg_start);

        // `/home/users/angel`: what follows `home` is another containing folder,
        // not the person. Without this, `users` would be redacted and the name
        // would come out intact in the next segment, the worst of both worlds.
        // Only when there is path left behind it: in `/home/users` the person is
        // called that, and it has to go.
        while seg_end < bytes.len() && is_profile_dir(&input[seg_start..seg_end]) {
            seg_start = sep_run_end(bytes, seg_end);
            seg_end = segment_end(bytes, seg_start);
        }

        // Nothing between separators, or something already redacted: no name to
        // strip.
        if seg_end == seg_start || &input[seg_start..seg_end] == PROFILE_TOKEN {
            i = seg_end.max(after_sep);
            continue;
        }

        let buf = out.get_or_insert_with(String::new);
        buf.push_str(&input[copied..seg_start]);
        buf.push_str(PROFILE_TOKEN);
        copied = seg_end;
        i = seg_end;
    }

    match out {
        Some(mut buf) => {
            buf.push_str(&input[copied..]);
            Cow::Owned(buf)
        }
        None => Cow::Borrowed(input),
    }
}

/// This process's real home, for the layouts that hit no marker (Silverblue's
/// `/var/home/<user>`, a custom `$HOME`). Resolved once: it does not change while
/// the process lives.
///
/// Returns the pair (home, home with its last segment redacted), or `None` when the
/// home is already covered by the markers and passing it again would be work for
/// nothing.
fn home_override() -> Option<&'static (String, String)> {
    static HOME: OnceLock<Option<(String, String)>> = OnceLock::new();
    HOME.get_or_init(|| {
        let base = directories::BaseDirs::new()?;
        let home = base
            .home_dir()
            .to_str()?
            .trim_end_matches(['/', '\\'])
            .to_string();
        if home.is_empty() || matches!(redact_markers(&home), Cow::Owned(_)) {
            return None; // ya lo cubre el paso por marcadores
        }
        let cut = home.rfind(['/', '\\'])? + 1;
        Some((home.clone(), format!("{}{PROFILE_TOKEN}", &home[..cut])))
    })
    .as_ref()
}

// ---- background shipper -------------------------------------------------

/// Resolved server policy for the current session.
struct Policy {
    /// Full ingest URL (already joined with base_url).
    url: String,
    token: String,
    min_rank: u8,
}

fn drain_loop(rx: Receiver<LogEntry>) {
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
                    let body = LogBatch {
                        device: device.clone(),
                        entries,
                    };
                    // A 401 with a Cloud JWT means "your token is no longer
                    // valid", not "the batch was lost": swallowing it left the
                    // shipper retrying against nothing until a restart. It goes
                    // back to the outer loop, which re-resolves with whatever
                    // token the service has rotated to meanwhile.
                    if let Err(PostError::Rejected) =
                        rt.block_on(post_batch(client, &policy, &body))
                    {
                        drop_rejected_lease(&policy);
                        discard_available(&rx);
                        std::thread::sleep(REJECTED_BACKOFF);
                        break;
                    }
                }
            }

            // Re-validate roughly every loop; if the session vanished, the
            // token rotated, or the user opted out mid-run, drop back to the
            // outer loop to re-resolve (which then backs off).
            if !session_matches(&policy) || !telemetry_enabled() {
                break;
            }
        }
    }
}

enum BatchResult {
    Entries(Vec<LogEntry>),
    Empty,
    Closed,
}

/// Block up to `FLUSH_INTERVAL` for the first entry, then greedily drain up to
/// `MAX_BATCH`, filtering by level.
fn collect_batch(rx: &Receiver<LogEntry>, min_rank: u8) -> BatchResult {
    let first = match rx.recv_timeout(FLUSH_INTERVAL) {
        Ok(e) => e,
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => return BatchResult::Empty,
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return BatchResult::Closed,
    };

    let mut out = Vec::new();
    if ships_at(&first, min_rank) {
        out.push(first);
    }
    while out.len() < MAX_BATCH {
        match rx.try_recv() {
            Ok(e) => {
                if ships_at(&e, min_rank) {
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

fn discard_available(rx: &Receiver<LogEntry>) {
    while rx.try_recv().is_ok() {}
}

/// Read the session, probe `/v1/health`, and decide the ingest endpoint +
/// minimum level. Returns `None` when there's no session or the server can't
/// receive logs.
async fn resolve_policy(client: &reqwest::Client) -> Option<Policy> {
    // Respect the user's opt-out first: no session probe, no shipping.
    if !telemetry_enabled() {
        return None;
    }
    let (base, token) = current_session()?;

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
    let min_rank = level_rank(min_level);

    let path = if health.mode.as_deref() == Some("cloud") {
        "/v1/cloud/logs"
    } else {
        "/v1/logs"
    };

    Some(Policy {
        url: format!("{base}{path}"),
        token,
        min_rank,
    })
}

/// Which server and which credential we are shipping with right now:
/// `(base_url, token)`.
///
/// Cloud wins when there is a Cloud session, the same order `session::resolve_owned`
/// follows to pick the active server. The Cloud slot is filled by whoever rotates
/// the JWT; the self-hosted one comes from `current` (the loan in a client, the
/// store in the service, D.20) and its token does not expire.
fn current_session() -> Option<(String, String)> {
    if let Some(lease) = credentials::lent_cloud() {
        return Some((lease.url.trim_end_matches('/').to_string(), lease.token));
    }
    let creds = credentials::current().ok().flatten()?;
    Some((creds.url.trim_end_matches('/').to_string(), creds.token))
}

/// Whether the user is sharing diagnostic logs. Read fresh from `prefs.json`
/// each call so toggling the setting takes effect without a restart.
///
/// It is **opt-out**, not opt-in: `Prefs::default` sets
/// `anonymous_telemetry: true`, so a fresh install ships until the user says
/// stop. That is a deliberate product call and it is disclosed in the privacy
/// policy, but the comment here used to claim the opposite ("we never ship
/// without an affirmative flag"), which is the kind of thing that reads like a
/// promise when someone audits this file. The only case treated as off is a
/// missing or corrupt prefs file, where we cannot know what was chosen.
fn telemetry_enabled() -> bool {
    crate::prefs::Prefs::load_default()
        .map(|(p, _)| p.anonymous_telemetry)
        .unwrap_or(false)
}

/// A Cloud token the server has rejected is no use to anybody, so the slot is
/// emptied to stop insisting with it. Whoever holds a good one puts it back, the
/// service on its next rotation or a client the next time it borrows one, and until
/// then this thread stays quiet rather than knocking every minute with a dead
/// credential.
///
/// Only when the rejected one is the one in place: between the POST and this, the
/// rotator may already have left a new one, and throwing it away would be throwing
/// away the good one.
fn drop_rejected_lease(policy: &Policy) {
    if matches!(credentials::lent_cloud(), Some(lease) if lease.token == policy.token) {
        credentials::set_lent_cloud(None);
    }
}

/// Cheap re-check: is there still a session whose token matches the policy we
/// resolved? Avoids a full health round-trip on every batch. With Cloud it is also
/// the rotation detector: the service changes the slot's token every 45 minutes or
/// so and the next batch already goes out with the new one.
fn session_matches(policy: &Policy) -> bool {
    matches!(current_session(), Some((_, token)) if token == policy.token)
}

/// Why a batch did not get in. Only the actionable case is distinguished: the
/// server rejecting the credential. A network failure is not actionable, since the
/// next batch goes out anyway, and gets swallowed as always.
enum PostError {
    /// 401 or 403: the token is no good. It has to be re-resolved.
    Rejected,
    /// Network down, timeout, 5xx: it retries on its own with the next batch.
    Transient,
}

async fn post_batch(
    client: &reqwest::Client,
    policy: &Policy,
    body: &LogBatch,
) -> Result<(), PostError> {
    let res = client
        .post(&policy.url)
        .header("authorization", format!("Bearer {}", policy.token))
        .json(body)
        .send()
        .await
        .map_err(|_| PostError::Transient)?;

    match res.status() {
        s if s.is_success() => Ok(()),
        reqwest::StatusCode::UNAUTHORIZED | reqwest::StatusCode::FORBIDDEN => {
            Err(PostError::Rejected)
        }
        _ => Err(PostError::Transient),
    }
}

fn device_meta() -> DeviceMeta {
    let id = device_identity();
    DeviceMeta {
        name: id.name,
        os: Some(id.os),
        // The fingerprint is computed by `fingerprint()` as the `hex::encode` of a
        // SHA-256, so it always passes the gate; if it ever stopped doing so, it
        // travels as absent rather than sending something the server cannot match.
        fingerprint: MachineId::parse(&id.fingerprint).ok(),
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

/// This machine's name, cached.
///
/// Every upload stamps it so the version history can say which computer each copy
/// came from: with the same save synced in two places, "v77, two hours ago" is not
/// enough to decide which to restore. It is the same hostname that already
/// identifies the device on the account, so the history's label and the device list
/// say the same thing.
///
/// Cached because it sits on every backup's path and the hostname does not change
/// within a run (and if it did, the older copies would keep the name they were made
/// under, which is exactly what a history should preserve).
pub fn device_name() -> Option<String> {
    static NAME: OnceLock<Option<String>> = OnceLock::new();
    NAME.get_or_init(sysinfo::System::host_name).clone()
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

#[cfg(test)]
mod tests {
    use super::*;
    use hoard_core::wire::TELEMETRY_TARGET;

    fn entry(level: &str, target: &str) -> LogEntry {
        LogEntry {
            level: level.to_string(),
            target: Some(target.to_string()),
            message: String::new(),
            fields: None,
            ts: None,
        }
    }

    #[test]
    fn redacts_the_profile_segment_on_every_platform() {
        assert_eq!(
            redact("C:\\Users\\angel\\AppData\\LocalLow\\TheGameBakers\\Furi"),
            "C:\\Users\\<user>\\AppData\\LocalLow\\TheGameBakers\\Furi"
        );
        assert_eq!(
            redact("/home/angel/.steam/steam/steamapps/compatdata/105600"),
            "/home/<user>/.steam/steam/steamapps/compatdata/105600"
        );
        assert_eq!(
            redact("/Users/angel/Library/Application Support/Factorio"),
            "/Users/<user>/Library/Application Support/Factorio"
        );
        // Capitalisation as Windows writes it, and mixed slashes.
        assert_eq!(
            redact("c:/users/Angel/Saved Games"),
            "c:/users/<user>/Saved Games"
        );
        assert_eq!(redact("D:\\USERS\\angel\\x"), "D:\\USERS\\<user>\\x");
    }

    #[test]
    fn redacts_every_path_in_one_message() {
        assert_eq!(
            redact("moved /home/angel/a to /home/angel/b"),
            "moved /home/<user>/a to /home/<user>/b"
        );
    }

    /// `record_debug` renders with `{:?}` and a string's `Debug` escapes
    /// backslashes, so a Windows path arrives with its slashes doubled. Looking for
    /// a bare `\Users\` did not match that and the name left the machine through
    /// any field recorded with `?` instead of `%`.
    #[test]
    fn redacts_windows_paths_as_debug_renders_them() {
        // What `format!("{:?}", "C:\\Users\\angel\\AppData")` actually produces.
        let debug_rendered = format!("{:?}", "C:\\Users\\angel\\AppData\\LocalLow");
        let shaped = redact(&debug_rendered);
        assert!(!shaped.contains("angel"), "the name survived in {shaped}");
        assert_eq!(shaped, "\"C:\\\\Users\\\\<user>\\\\AppData\\\\LocalLow\"");
    }

    #[test]
    fn a_run_of_separators_is_still_one_separator() {
        assert_eq!(redact("/home//angel/x"), "/home//<user>/x");
        assert_eq!(redact("C:\\\\Users\\\\angel"), "C:\\\\Users\\\\<user>");
        // UNC: the leading `\\` is not a profile, the `\Users\` inside is.
        assert_eq!(
            redact("\\\\nas\\share\\Users\\angel\\Saved Games"),
            "\\\\nas\\share\\Users\\<user>\\Saved Games"
        );
    }

    /// The name does not always hang directly off `home`: some installs use
    /// `/home/users/<name>`, and there the easy mistake is to redact the containing
    /// folder and let the person through.
    #[test]
    fn the_name_can_hang_one_level_deeper() {
        assert_eq!(redact("/home/users/angel/save"), "/home/users/<user>/save");
        assert_eq!(redact("/home/home/angel"), "/home/home/<user>");
        // But if the path ends there, that folder IS the person.
        assert_eq!(redact("/home/users"), "/home/<user>");
        // Proton: `drive_c/users/steamuser` is a Wine constant, and gets redacted
        // the same. Little is lost and the shape still says what it is.
        assert_eq!(
            redact("/home/angel/.steam/steamapps/compatdata/1/pfx/drive_c/users/steamuser/AppData"),
            "/home/<user>/.steam/steamapps/compatdata/1/pfx/drive_c/users/<user>/AppData"
        );
    }

    #[test]
    fn a_name_with_accents_survives_the_byte_scan() {
        // The scan goes by bytes; a multibyte name must not split a slice down the
        // middle (that would be a panic, not a leak).
        assert_eq!(redact("/home/ángel/juegos"), "/home/<user>/juegos");
        assert_eq!(redact("/home/日本語/x"), "/home/<user>/x");
    }

    #[test]
    fn leaves_alone_what_has_no_name_in_it() {
        // With nothing to redact it comes back borrowed, which is the normal path.
        assert!(matches!(
            redact("agent: backup committed"),
            Cow::Borrowed(_)
        ));
        assert!(matches!(redact("/usr/share/hoard"), Cow::Borrowed(_)));
        // A marker with no segment behind it has no name to strip.
        assert_eq!(redact("/home/"), "/home/");
        assert_eq!(redact("guardado en /home"), "guardado en /home");
        // And what is already redacted is not redacted again (no `<<user>>`).
        assert_eq!(redact("/home/<user>/x"), "/home/<user>/x");
        // A loose word is not a profile folder.
        assert_eq!(
            redact("todos los users tienen home"),
            "todos los users tienen home"
        );
    }

    #[test]
    fn keeps_the_shape_that_makes_detection_fixable() {
        // What is kept is exactly what is useful for fixing detection: the shape of
        // the path, the game and the suffix.
        let shaped = redact("C:\\Users\\angel\\AppData\\LocalLow\\TheGameBakers\\Furi");
        assert!(shaped.contains("AppData\\LocalLow"));
        assert!(shaped.contains("TheGameBakers\\Furi"));
        assert!(!shaped.contains("angel"));
    }

    /// This runs inside `on_event`, meaning on every log line in the process: an
    /// index off a character boundary would not be a redaction failure, it would be
    /// a panic on every log line in the app. It gets hammered with random inputs
    /// from a deliberately nasty alphabet (separators run together, multibyte, loose
    /// pieces of markers) and it also checks the result keeps none of the seeded
    /// names.
    #[test]
    fn random_garbage_never_panics_and_never_keeps_a_name() {
        const ALPHABET: [&str; 14] = [
            "/", "\\", "home", "Users", "users", "HOME", "angel", "ángel", "日本", "<user>", " ",
            ":", "C", "..",
        ];
        // A deterministic LCG: a failure reproduces from the same seed.
        let mut seed = 0x2545_F491_4F6C_DD1Du64;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };

        for _ in 0..20_000 {
            let len = (next() % 12) as usize;
            let mut input = String::new();
            for _ in 0..len {
                input.push_str(ALPHABET[(next() % ALPHABET.len() as u64) as usize]);
            }
            let out = redact(&input);
            // A name can only survive if it did not hang off a profile folder;
            // what it must not do is survive *behind* one.
            for name in ["angel", "ángel"] {
                let leaked = out
                    .match_indices(name)
                    .any(|(at, _)| profile_dir_precedes(&out, at));
                assert!(!leaked, "{input:?} -> {out:?} kept {name}");
            }
        }
    }

    /// Does the segment starting at `at` hang straight off `home`/`users`?
    fn profile_dir_precedes(text: &str, at: usize) -> bool {
        let before = &text[..at];
        let Some(cut) = before.rfind(['/', '\\']) else {
            return false;
        };
        let run_start = before[..cut]
            .rfind(|c| c != '/' && c != '\\')
            .map_or(0, |i| i + before[i..].chars().next().unwrap().len_utf8());
        let Some(prev_cut) = before[..run_start].rfind(['/', '\\']) else {
            return false;
        };
        let segment = &before[prev_cut + 1..run_start];
        super::is_profile_dir(segment)
    }

    #[test]
    fn telemetry_rides_below_the_server_minimum() {
        // WARN (3) is the minimum Cloud advertises: operational INFO stays out and
        // the contradiction gets in regardless.
        assert!(!ships_at(&entry("info", "hoard_agent::agent"), 3));
        assert!(ships_at(&entry("warn", "hoard_agent::agent"), 3));
        assert!(ships_at(&entry("info", TELEMETRY_TARGET), 3));
    }
}
