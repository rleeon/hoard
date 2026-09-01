use anyhow::{anyhow, bail, Context, Result};
use hoard_core::ids::GameSlug;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use thiserror::Error;
use tokio::sync::OnceCell;

/// How long a snapshot download may go without a single byte arriving before we
/// call it stalled. Not a budget for the transfer (it resets on every chunk)
/// so it bounds a dead stream without capping a big, slow, healthy one.
const STREAM_STALL_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("authentication failed: token rejected by server (401)")]
    Unauthorized,
    #[error("forbidden (403)")]
    Forbidden,
    #[error("not found (404)")]
    NotFound,
    /// HTTP 413. On Hoard Cloud the body carries the structured per-save cap
    /// (`code:"save_too_large"` with `plan` / `limit_bytes` / `actual_bytes`),
    /// so we can tell the user exactly which limit they hit and how big the
    /// save was. Self-hosted 413s (raw quota) leave `0` and fall back to the
    /// generic message.
    #[error("{}", .0.human())]
    TooLarge(SaveTooLarge),
    /// HTTP 403 with `code:"save_archived"`: the game is parked in the
    /// server-side archive ("caja negra"). Uploading it would revive its frozen
    /// blobs and re-inflate the quota, so the client must stop trying and treat
    /// the local save as frozen, not errored. Distinct from the generic
    /// `Forbidden` so the backup path can settle it silently instead of painting
    /// a red "failed".
    #[error("game is archived on the server")]
    Archived,
    /// HTTP 402 with `code:"quota_exceeded"`: the account's total stored bytes
    /// are at (or over) the plan limit. Unlike a 413 there is nothing to trim
    /// and nothing to wait for: **every** upload will fail identically until the
    /// user frees space or upgrades, so the caller must park the whole cloud leg
    /// instead of retrying per save. Carries the server's figures so the UI can
    /// say how far over the line the account is.
    #[error("{}", .0.human())]
    QuotaExceeded(QuotaExceeded),
    /// The blob endpoint never answered: the TCP connection couldn't be
    /// opened, or it timed out on the way.
    ///
    /// Typed for two reasons, both learned from one report. The first is what it
    /// looked like: on Cloud the bytes go straight to a presigned R2 URL, and
    /// `reqwest`'s own error renders the whole URL, so the feed showed the user
    /// a 400-character AWS signature and the words "error sending request"
    /// where it should have said the storage can't be reached. The second is
    /// what it cost: this is not a flake, it's a path that is down, so the
    /// exponential retry budget meant for a dropped packet just burns six
    /// attempts at a ~21 s connect timeout each, four minutes per round, and
    /// then re-arms and does it again. The caller parks instead.
    ///
    /// `host` and not the URL on purpose: it is the part a person can act on
    /// (or send to their ISP) and the part that carries no signature.
    #[error("can't reach the storage endpoint ({host}): {reason}")]
    StorageUnreachable { host: String, reason: String },
    /// HTTP 409 with `code:"non_fast_forward"`: the server's head moved past
    /// the `base_version` this upload declared, so another device advanced the
    /// save since we last synced.
    ///
    /// Typed, and not left inside [`Self::Conflict`]'s message, because the
    /// body answers the two questions the caller has to answer to recover, and
    /// as a string both were being thrown away:
    ///
    /// - **which version** is the head now (`head_version`), so the retry can
    ///   fast-forward from it instead of asking a second endpoint;
    /// - **which row** the push was rejected against (`save_id`). A client keys
    ///   saves by its own device-local id and the server resolves that id by
    ///   `(user, game_slug, label)`, so the row it rejected against can be one
    ///   whose id this device has never seen. That is what stalled a save for
    ///   two weeks in aug-2026: the reconcile looked itself up in the manifest
    ///   by the local id, found nothing, reported "nothing to pull", and parked
    ///   the conflict, with the head it needed sitting unread in this body.
    ///
    /// A server too old to send `code` still lands in [`Self::Conflict`], and
    /// the callers keep their message-text fallback for exactly that.
    #[error("{}", .0.human())]
    NonFastForward(NonFastForward),
    #[error("conflict (409): {0}")]
    Conflict(String),
    #[error("bad request (400): {0}")]
    BadRequest(String),
    #[error("server error ({status}): {body}")]
    Server { status: u16, body: String },
    /// HTTP 429. Carries the server's `retry_after_seconds` so the caller can
    /// wait the *exact* window-slide time instead of a short exponential
    /// backoff that would just burn every retry inside the still-over-quota
    /// window. `body` keeps the raw JSON for logging/diagnostics.
    ///
    /// `kind` says which of the two very different 429s this is; see
    /// [`RateLimitKind`]. Collapsing them into one is what wedged large
    /// self-hosted uploads: a "you're going too fast" answer meant for a single
    /// PUT was treated as "the whole upload doesn't fit right now".
    #[error("rate limited (429, {kind}): retry after {retry_after_seconds}s")]
    RateLimited {
        kind: RateLimitKind,
        retry_after_seconds: u32,
        body: String,
    },
}

/// Which kind of 429 came back, because Hoard's servers answer 429 for two
/// reasons that need opposite reactions.
///
/// The wire tells them apart by structure, not by a flag: every budget answer
/// Hoard writes carries a JSON `code` (`bandwidth_limit`,
/// `quota_exceeded_paced`, `restore_paced`, `too_many_attempts`), while the
/// per-IP pacer is `tower_governor` middleware that never sees our types and
/// answers a bare body. A reverse proxy's own limiter (nginx `limit_req`, which
/// the self-host guide recommends putting in front) lands in the same
/// unstructured shape, and wants the same reaction, so "no code" is the right
/// test rather than sniffing for a particular server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitKind {
    /// Per-IP request pacing: *this request* arrived too fast. The wait is
    /// milliseconds to seconds and the operation as a whole is perfectly
    /// welcome: retry the single request and carry on. Abandoning the whole
    /// operation here is how a 122-blob upload could never finish: the pacer
    /// let ~60 through per attempt, the client threw away the other 62 along
    /// with the ones that had already landed, and the next attempt started
    /// from zero.
    Paced,
    /// A budget the whole operation does not fit inside right now: the rolling
    /// bandwidth window, the storage quota, or a loop brake. Retrying the same
    /// request in a tight loop cannot help: the caller has to give up on this
    /// attempt and come back after the stated wait.
    Budget,
}

impl std::fmt::Display for RateLimitKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            RateLimitKind::Paced => "paced",
            RateLimitKind::Budget => "budget",
        })
    }
}

/// Structured body of a 413. All fields default to zero/empty so a body we
/// can't parse still yields a usable [`ApiError::TooLarge`] via
/// [`SaveTooLarge::human`].
///
/// Three different things answer 413 and the wording has to tell them apart,
/// because the fix is different in each case:
///
/// - Hoard Cloud (`code: "save_too_large"`): the account's per-save plan
///   cap. Carries `plan` and `actual_bytes`; the user upgrades or trims.
/// - A self-hosted Hoard (`code: "snapshot_too_large"`): the operator's own
///   `storage.max_snapshot_size_mb`. Carries `limit_bytes` and `received_bytes`;
///   the user edits their `config.toml`.
/// - Something in front of the server: nginx, Traefik, a Cloudflare tunnel.
///   No code, no JSON at all: the body is an HTML error page. See
///   [`Self::from_foreign_body`].
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SaveTooLarge {
    /// `save_too_large` (Cloud) or `snapshot_too_large` (self-hosted). Empty
    /// when the responder wasn't Hoard.
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub plan: String,
    #[serde(default)]
    pub limit_bytes: u64,
    #[serde(default)]
    pub actual_bytes: u64,
    /// Bytes the server had taken in when it gave up. A **floor**, not the
    /// snapshot's size: a self-hosted server aborts mid-stream, so it never
    /// learns the total. Kept separate from `actual_bytes` so the two can't be
    /// confused, since Cloud knows the real size up front and self-hosted doesn't.
    #[serde(default)]
    pub received_bytes: u64,
    #[serde(default)]
    pub upgrade_url: Option<String>,
    /// The response body was not Hoard's JSON, so the rejection came from
    /// something between the client and the server. Set by
    /// [`Self::from_foreign_body`], never deserialized.
    #[serde(skip)]
    pub from_proxy: bool,
}

/// Structured body of a `non_fast_forward` 409, from either deployment. Same
/// defaulting discipline as [`SaveTooLarge`]: a body we can't fully parse still
/// yields a usable error, it just recovers less.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct NonFastForward {
    /// The server's current head for this save. `0` when the body didn't say
    /// (a server older than this field), which the callers read as "unknown"
    /// and fall back to asking the manifest.
    #[serde(default)]
    pub head_version: i64,
    /// The base this upload declared and that the server refused.
    #[serde(default)]
    pub base_version: i64,
    /// The server's own id for the row it rejected against. Usually the id we
    /// sent; different when this device's local id resolved to another row.
    /// Empty from a server that predates the field.
    #[serde(default)]
    pub save_id: String,
}

impl NonFastForward {
    /// The head to reconcile against, when the server named one. `None` keeps
    /// "the server didn't say" distinct from "the head is version 0": the
    /// caller must go ask rather than rebase onto a number we invented.
    pub fn head(&self) -> Option<i64> {
        (self.head_version > 0).then_some(self.head_version)
    }

    /// The canonical save id, when the server named one **and** it isn't the
    /// one we already believe. `None` means "nothing to relabel".
    pub fn canonical_id_for<'a>(&'a self, local_id: &str) -> Option<&'a str> {
        (!self.save_id.is_empty() && self.save_id != local_id).then_some(self.save_id.as_str())
    }

    pub fn human(&self) -> String {
        let mut s = String::from(
            "non-fast-forward: another device advanced this save since your base version",
        );
        if self.head_version > 0 {
            s.push_str(&format!(
                " (head {}, base {})",
                self.head_version, self.base_version
            ));
        }
        s
    }
}

/// Structured body of a Hoard Cloud `quota_exceeded` 402. Same defaulting
/// discipline as [`SaveTooLarge`]: a body we can't parse still yields a usable
/// error.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct QuotaExceeded {
    #[serde(default)]
    pub plan: String,
    #[serde(default)]
    pub used_bytes: u64,
    #[serde(default)]
    pub limit_bytes: u64,
    #[serde(default)]
    pub requested_bytes: u64,
    #[serde(default)]
    pub upgrade_url: Option<String>,
}

impl QuotaExceeded {
    /// Bytes the account is over its limit by. `0` when the figures are absent
    /// or the account is exactly at the line.
    pub fn over_bytes(&self) -> u64 {
        self.used_bytes.saturating_sub(self.limit_bytes)
    }

    pub fn human(&self) -> String {
        if self.limit_bytes == 0 {
            return "storage quota exceeded (402)".into();
        }
        format!(
            "storage full: {} of {} used on the {} plan, free space or upgrade",
            fmt_bytes(self.used_bytes),
            fmt_bytes(self.limit_bytes),
            if self.plan.is_empty() {
                "current"
            } else {
                &self.plan
            },
        )
    }
}

impl SaveTooLarge {
    /// Build the "this didn't come from Hoard" case from a body we couldn't
    /// parse.
    ///
    /// A 413 whose body isn't our JSON was written by whatever sits in front of
    /// the server, and that is worth saying out loud, because the user will
    /// otherwise go hunting through Hoard's settings for a limit that isn't
    /// there. It cost a self-hoster days: nginx's default `client_max_body_size`
    /// is 1 MB, their save was bigger, and every message they saw pointed at
    /// Hoard (2026-08-07).
    pub fn from_foreign_body() -> Self {
        Self {
            from_proxy: true,
            ..Self::default()
        }
    }

    /// Who refused it, so the UI can name the right knob.
    pub fn kind(&self) -> hoard_core::ipc::events::TooLargeKind {
        use hoard_core::ipc::events::TooLargeKind;
        if self.from_proxy {
            TooLargeKind::Proxy
        } else if self.code == "snapshot_too_large" {
            TooLargeKind::ServerLimit
        } else {
            TooLargeKind::PlanCap
        }
    }

    /// A human, diagnosable one-liner: *which* limit, and whose. The desktop
    /// re-localizes from the structured fields via the `BackupTooLarge` agent
    /// event; this string is the log and CLI surface.
    pub fn human(&self) -> String {
        if self.from_proxy {
            return "payload too large (413), and the reply wasn't Hoard's: \
                    something between this machine and the server refused it \
                    (a reverse proxy or tunnel body-size limit)"
                .into();
        }
        if self.code == "snapshot_too_large" && self.limit_bytes > 0 {
            // Two different figures can come back and they do not mean the same
            // thing. `actual_bytes` is the size the manifest declared, known
            // before anything moves; `received_bytes` is how far a mid-stream
            // abort got. Wording them alike had the client announce "3.6 GB sent
            // before it stopped" about an upload that never sent a byte.
            let tail = if self.actual_bytes > 0 {
                format!(", the save is {}", fmt_bytes(self.actual_bytes))
            } else if self.received_bytes > 0 {
                format!(
                    ", {} sent before it stopped",
                    fmt_bytes(self.received_bytes)
                )
            } else {
                String::new()
            };
            return format!(
                "snapshot too large: over this server's limit of {} per snapshot \
                 (raise storage.max_snapshot_size_mb in its config.toml){tail}",
                fmt_bytes(self.limit_bytes),
            );
        }
        if self.limit_bytes == 0 {
            return "payload too large (413): exceeds the server's per-save size limit".into();
        }
        format!(
            "save too large: {} exceeds the {} plan limit of {} per save",
            fmt_bytes(self.actual_bytes),
            if self.plan.is_empty() {
                "current"
            } else {
                &self.plan
            },
            fmt_bytes(self.limit_bytes),
        )
    }
}

/// Coarse human byte size for error copy. Binary units, one decimal.
fn fmt_bytes(n: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let n = n as f64;
    if n >= GB {
        format!("{:.1} GB", n / GB)
    } else if n >= MB {
        format!("{:.0} MB", n / MB)
    } else if n >= KB {
        format!("{:.0} KB", n / KB)
    } else {
        format!("{n:.0} B")
    }
}

impl ApiError {
    pub async fn from_response(resp: reqwest::Response) -> Self {
        let status = resp.status();
        // Grab the wait hints before consuming the body: they're our fallback
        // for `retry_after_seconds` if the JSON is unparseable.
        //
        // Two header names, because the per-IP pacer speaks a different one:
        // `tower_governor` emits `x-ratelimit-after` and nothing else, so a
        // client that only looks for `Retry-After` throws away the one number
        // the pacer *did* send and invents its own. Note the pacer's value is
        // whole seconds (`Duration::as_secs`), so a sub-second wait, which is
        // every wait at the default 20 req/s, arrives as a legitimate `0`.
        // `0` means "almost immediately", not "no hint"; the retry loop applies
        // its own floor.
        let headers = resp.headers();
        let retry_after_header = ["retry-after", "x-ratelimit-after"]
            .iter()
            .find_map(|name| {
                headers
                    .get(*name)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.trim().parse::<u32>().ok())
            });
        let body = resp.text().await.unwrap_or_default();
        match status {
            StatusCode::UNAUTHORIZED => ApiError::Unauthorized,
            StatusCode::FORBIDDEN => {
                if extract_code(&body).as_deref() == Some("save_archived") {
                    ApiError::Archived
                } else {
                    ApiError::Forbidden
                }
            }
            StatusCode::NOT_FOUND => ApiError::NotFound,
            // Cloud-only. Self-hosted never issues a 402, so an unparseable body
            // here is still safest treated as "the account is full".
            StatusCode::PAYMENT_REQUIRED
                if extract_code(&body).as_deref() == Some("quota_exceeded") =>
            {
                ApiError::QuotaExceeded(
                    serde_json::from_str::<QuotaExceeded>(&body).unwrap_or_default(),
                )
            }
            // A 413 we can't parse wasn't written by Hoard: both our servers
            // answer with JSON carrying a `code`. Say so instead of shrugging:
            // the culprit is a proxy limit, and nothing in Hoard's settings will
            // fix it.
            StatusCode::PAYLOAD_TOO_LARGE => ApiError::TooLarge(
                serde_json::from_str::<SaveTooLarge>(&body)
                    .ok()
                    .filter(|d| !d.code.is_empty())
                    .unwrap_or_else(SaveTooLarge::from_foreign_body),
            ),
            // Only the tagged one is typed. Every other 409 on these clients is
            // a duplicate label on rename, which has nothing to reconcile and
            // whose callers match on `Conflict`.
            StatusCode::CONFLICT if extract_code(&body).as_deref() == Some("non_fast_forward") => {
                ApiError::NonFastForward(
                    serde_json::from_str::<NonFastForward>(&body).unwrap_or_default(),
                )
            }
            StatusCode::CONFLICT => ApiError::Conflict(extract_message(&body)),
            StatusCode::BAD_REQUEST => ApiError::BadRequest(extract_message(&body)),
            StatusCode::TOO_MANY_REQUESTS => {
                let (kind, retry_after_seconds) = classify_rate_limit(&body, retry_after_header);
                ApiError::RateLimited {
                    kind,
                    retry_after_seconds,
                    body,
                }
            }
            _ => ApiError::Server {
                status: status.as_u16(),
                body,
            },
        }
    }
}

fn extract_message(body: &str) -> String {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(body) {
        if let Some(s) = v.get("message").and_then(|x| x.as_str()) {
            return s.to_string();
        }
        if let Some(s) = v.get("error").and_then(|x| x.as_str()) {
            return s.to_string();
        }
    }
    body.to_string()
}

/// Pull the stable machine-readable `code` out of a cloud error body, if any.
/// Turn a transport failure against a presigned storage URL into something a
/// person can read and the caller can branch on.
///
/// Only *connect* failures become [`ApiError::StorageUnreachable`]: the socket
/// never opened, which is a path problem and not a flake worth six retries.
/// Everything else keeps its original shape, because everything else is worth
/// retrying.
///
/// Timeouts deliberately do **not** count, tempting as it looks: the two
/// clients that reach here are built for long transfers and a timeout means
/// something entirely different on each. `download_http` carries a per-read
/// `read_timeout`, so its timeout is a transfer that opened fine and then
/// stalled; calling that "can't reach the storage" would be exactly the kind of
/// misleading error this function exists to stop, and parking on it would give
/// up on a slow connection that just needs another go. A connect failure is
/// unambiguous on both.
///
/// The message is rebuilt from the error's cause chain rather than passed
/// through, because `reqwest`'s own `Display` starts with the full URL, and a
/// presigned URL is a 400-character AWS signature that has no business in a log
/// or a feed row. The host survives; the signature doesn't.
fn storage_transport_error(url: &str, e: reqwest::Error) -> anyhow::Error {
    if !e.is_connect() {
        return anyhow!(e);
    }
    let host = reqwest::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(str::to_owned))
        .unwrap_or_else(|| "storage".to_owned());
    // Innermost cause: the OS-level reason ("connection timed out", "network is
    // unreachable"), which is the only part that says anything new.
    let mut source: &dyn std::error::Error = &e;
    while let Some(next) = source.source() {
        source = next;
    }
    let reason = source.to_string();
    anyhow!(ApiError::StorageUnreachable { host, reason })
}

/// A paced 429 that is really "your account is full", unpacked into the same
/// figures the 402 carries.
///
/// The server answers a full account with a plain 402 until it has refused the
/// same account five times in an hour, and with a 429 (`quota_exceeded_paced`)
/// after that. Both mean the identical thing to a person (*you are out of
/// space, here is how much and here is the upgrade*) but only the 402 was
/// wired to say so, so the moment the brake engaged the account stopped seeing
/// the "free up space / go Pro" prompt and started seeing a wordless wait. The
/// figures were in the paced body the whole time; this reads them back out.
///
/// `None` for every other budget 429 (bandwidth window, login throttle), which
/// are genuinely just a wait.
pub fn paced_quota_detail(body: &str) -> Option<QuotaExceeded> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    if v.get("code")?.as_str()? != "quota_exceeded_paced" {
        return None;
    }
    serde_json::from_value(v).ok()
}

/// Read a 429's body and wait hints into a kind and a number of seconds.
///
/// Split out from `from_response` so the wire rules are testable without an
/// HTTP round trip: they're the whole point of the change and easy to break
/// by accident later.
fn classify_rate_limit(body: &str, retry_after_header: Option<u32>) -> (RateLimitKind, u32) {
    // A `code` means one of our handlers wrote this answer, and every one of
    // those is a budget: bandwidth window, storage quota, loop brake, login
    // throttle. An unknown code counts as a budget too, since backing off fully is
    // the safe way to be wrong about a server newer than us.
    let kind = match extract_code(body) {
        Some(_) => RateLimitKind::Budget,
        None => RateLimitKind::Paced,
    };
    let secs = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| {
            v.get("retry_after_seconds")
                .and_then(|x| x.as_u64())
                .map(|n| n as u32)
        })
        .or(retry_after_header)
        .unwrap_or(match kind {
            // A budget with no usable hint waits a sensible spell rather than
            // hammering. For pacing the same 60s would be a lie with teeth: the
            // real wait is milliseconds, and 122 blobs served 60s each is a
            // two-hour upload. Leave it at 0 and let the retry loop's floor
            // decide.
            RateLimitKind::Budget => 60,
            RateLimitKind::Paced => 0,
        });
    (kind, secs)
}

fn extract_code(body: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v.get("code").and_then(|x| x.as_str()).map(String::from))
}

#[derive(Debug, Clone)]
pub struct ApiClient {
    base_url: String,
    /// Bearer token, shared+swappable across every clone of this client.
    ///
    /// Self-hosted bearer tokens are stable for the process lifetime, but a
    /// Hoard Cloud session uses a short-lived Supabase JWT (~1h). The desktop's
    /// long-lived agent holds a single `ApiClient` for the whole session, so a
    /// frozen token would start answering 401 the moment the JWT expired,
    /// which is exactly what made the auto-restore sweep spam "no se pudo
    /// restaurar" once an hour in. Storing the token behind a shared `RwLock`
    /// lets the desktop's token-refresh path push a fresh JWT into the running
    /// agent's client via [`ApiClient::set_token`] without rebuilding it.
    token: Arc<RwLock<String>>,
    /// Client for the small request/response JSON endpoints. Its 60 s total
    /// timeout covers the whole request *including the body*, so nothing that
    /// streams snapshot bytes may use it; see `upload_http` and `download_http`.
    http: Client,
    /// Streaming client for snapshot **uploads** (`snapshot_upload`,
    /// `put_presigned`). Same headers as `http` but with **no per-request total
    /// timeout**: a multi-GB save (Paradox grand-strategy is the worst case)
    /// on a residential upload link blows past any fixed timeout, which
    /// previously killed the request mid-flight and silently hung the dashboard
    /// "Subiendo…" pill. A TCP keepalive surfaces a genuinely dead connection;
    /// a slow-but-progressing upload is left to finish.
    upload_http: Client,
    /// Streaming client for snapshot **downloads** (`snapshot_download`,
    /// `get_presigned`). Same no-total-timeout rationale as `upload_http`, plus
    /// a `read_timeout` that bounds a genuine stall.
    ///
    /// The read timeout is deliberately *not* on `upload_http`: reqwest arms it
    /// once when the request starts and polls it while waiting for the response
    /// head, only handing it to the body (where it becomes per-read and resets
    /// on progress) once the head arrives. A download's head lands immediately,
    /// so the timeout only ever sees body reads; an upload's head arrives after
    /// the whole body is sent, so the same setting would kill any upload slower
    /// than the timeout, exactly the bug `upload_http` exists to avoid.
    download_http: Client,
    /// Lazily-probed `/v1/health` `mode` (`Some("cloud")` on the SaaS
    /// deployment, `None`/absent self-hosted). Cached behind an `Arc` so the
    /// many `ApiClient` clones in flight share a single probe. Only cached on
    /// a successful probe: a transient health failure leaves the cell empty
    /// so the next call retries instead of wedging the client into the wrong
    /// protocol forever.
    mode: Arc<OnceCell<Option<String>>>,
    /// Does this server advertise the `/v1/saves/{id}/cas/*` routes
    /// (content-addressed upload)? It is filled in by the same probe as `mode`,
    /// not by a separate one: asking separately would mean choosing a protocol
    /// from half a picture.
    ///
    /// Empty means it has not been probed. `Some(false)` means a server older
    /// than 1.1.3, which only understands multipart.
    cas: Arc<OnceCell<bool>>,
    /// Does this server keep a device census and presence (`/v1/devices`,
    /// `/v1/presence/heartbeat`)? Filled in by the same probe, for the same
    /// reason.
    devices: Arc<OnceCell<bool>>,
    /// The plan's per-save cap, learned from the last 413.
    ///
    /// It is not a probe like the ones above, which is why it is not a
    /// `OnceCell`. The cap changes when the user changes plan, so it has to be
    /// rewritable; what it must not be is rediscovered on every copy. Today the
    /// only way to know it is for the server to refuse you: there is no `GET`
    /// that says it, so the 413 is both the error and the configuration channel.
    /// Remembering it turns the refusal into something that happens once a
    /// session rather than once an autosave; five users with big saves generated
    /// 12,996 of them in a week, all with the same answer.
    ///
    /// It lives in the client rather than in the save's state because the cap
    /// belongs to the plan, not to the save: discovering it by uploading one
    /// game counts just as well for the next.
    plan_cap: Arc<RwLock<Option<PlanCap>>>,
}

/// The plan's per-save cap and which plan it belongs to. See
/// [`ApiClient::plan_cap`].
#[derive(Debug, Clone)]
pub struct PlanCap {
    pub limit_bytes: u64,
    pub plan: String,
    /// When we learned it. See [`PLAN_CAP_TTL`].
    learned_at: std::time::Instant,
}

/// How long a learned cap is worth before it gets checked again.
///
/// It exists because of upgrades. Somebody moving to Pro multiplies their cap,
/// but the client only finds out if it asks again, and the whole point of
/// remembering the cap is to stop asking. With no expiry, somebody who has just
/// paid would carry on uploading copies trimmed against the Free cap until the
/// service restarted, which is exactly the moment you can least afford to fail
/// them.
///
/// Half an hour is the compromise: the refusal goes from one per autosave to at
/// most one every 30 minutes, two orders of magnitude fewer, and a plan change
/// takes at most that long to be noticed on its own.
const PLAN_CAP_TTL: std::time::Duration = std::time::Duration::from_secs(30 * 60);

impl ApiClient {
    pub fn new(base_url: impl Into<String>, token: impl Into<String>) -> Result<Self> {
        let http = Client::builder()
            .user_agent(concat!("hoard-agent/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(60))
            // Long-lived stream uploads/downloads handle their own timeouts via streaming
            .build()?;
        let upload_http = Client::builder()
            .user_agent(concat!("hoard-agent/", env!("CARGO_PKG_VERSION")))
            // No total timeout: snapshot bodies are arbitrary size. The TCP
            // keepalive RSTs a connection that genuinely stopped flowing, while
            // a slow-but-progressing upload is left to finish.
            .tcp_keepalive(Duration::from_secs(30))
            .pool_idle_timeout(None)
            .build()?;
        let download_http = Client::builder()
            .user_agent(concat!("hoard-agent/", env!("CARGO_PKG_VERSION")))
            .tcp_keepalive(Duration::from_secs(30))
            .pool_idle_timeout(None)
            // Per-read, not total: it resets on every chunk that arrives, so a
            // download stays alive as long as it progresses however long it
            // takes, while one that truly stalls fails instead of hanging.
            .read_timeout(STREAM_STALL_TIMEOUT)
            .build()?;
        Ok(Self {
            // Strips a `user@` the caller may still have on disk from before
            // this was normalised on the way in. Left alone it silently becomes
            // an HTTP Basic header that shadows the bearer token; see
            // `serverclass::normalize_server_url`.
            base_url: crate::serverclass::normalize_server_url(&base_url.into()),
            token: Arc::new(RwLock::new(token.into())),
            http,
            upload_http,
            download_http,
            mode: Arc::new(OnceCell::new()),
            cas: Arc::new(OnceCell::new()),
            devices: Arc::new(OnceCell::new()),
            plan_cap: Arc::new(RwLock::new(None)),
        })
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    /// Swap in a fresh bearer token. Every clone of this client shares the same
    /// token cell, so updating one updates them all, which is the mechanism the desktop
    /// uses to keep the long-lived agent client's Supabase JWT current after a
    /// refresh.
    pub fn set_token(&self, token: impl Into<String>) {
        if let Ok(mut guard) = self.token.write() {
            *guard = token.into();
        }
    }

    fn auth_header(&self) -> String {
        let token = self.token.read().map(|t| t.clone()).unwrap_or_default();
        format!("Bearer {token}")
    }

    async fn ok_or_err(resp: reqwest::Response) -> Result<reqwest::Response, ApiError> {
        if resp.status().is_success() {
            Ok(resp)
        } else {
            Err(ApiError::from_response(resp).await)
        }
    }

    /// Issue an authenticated GET to `path` (e.g. `/v1/manifest/version`) and
    /// return the response object on success. Other modules use this when
    /// their only interaction with the API is "GET this URL, decode JSON".
    pub async fn http_get(&self, path: &str) -> Result<reqwest::Response> {
        let resp = self
            .http
            .get(self.url(path))
            .header("authorization", self.auth_header())
            .send()
            .await?;
        Ok(Self::ok_or_err(resp).await?)
    }

    pub async fn whoami(&self) -> Result<Whoami> {
        let resp = self
            .http
            .get(self.url("/v1/auth/whoami"))
            .header("authorization", self.auth_header())
            .send()
            .await?;
        let resp = Self::ok_or_err(resp).await?;
        Ok(resp.json().await?)
    }

    pub async fn health(&self) -> Result<Health> {
        let resp = self.http.get(self.url("/v1/health")).send().await?;
        let resp = Self::ok_or_err(resp).await?;
        Ok(resp.json().await?)
    }

    /// Resolve (and cache) the server's deployment mode from `/v1/health`.
    /// `Some("cloud")` selects the Hoard Cloud protocol; `None` means
    /// self-hosted. A failed probe returns `None` *without* caching so the
    /// next call retries.
    pub async fn server_mode(&self) -> Option<String> {
        self.mode
            .get_or_try_init(|| async {
                let h = self.health().await?;
                // The capabilities come out of the same response. They are
                // stored here rather than in probes of their own so mode and
                // capabilities always describe the same server.
                let _ = self.cas.set(h.cas);
                let _ = self.devices.set(h.devices);
                Ok::<_, anyhow::Error>(h.mode)
            })
            .await
            .ok()
            .cloned()
            .flatten()
    }

    /// Can this server negotiate content before it is uploaded
    /// (`/v1/saves/{id}/cas/*`)? `None` until a probe has succeeded.
    ///
    /// The same honest contract as [`Self::probed_is_cloud`]: it does not
    /// collapse "it does not support it" into "I do not know yet". Choosing
    /// multipart because the probe failed would upload gigabytes for nothing.
    pub fn probed_supports_cas(&self) -> Option<bool> {
        self.cas.get().copied()
    }

    /// Does this server keep a device census and live presence?
    ///
    /// Cloud always has them and does not advertise the flag (its `/v1/health`
    /// is a different body), so whoever asks also has to consult
    /// [`Self::probed_is_cloud`], which is what [`Self::has_presence`] does.
    pub fn probed_supports_devices(&self) -> Option<bool> {
        self.devices.get().copied()
    }

    /// Is it worth sending this server presence heartbeats? Cloud always;
    /// self-hosted since 1.1.3. It probes when it has to.
    pub async fn has_presence(&self) -> bool {
        let _ = self.server_mode().await;
        self.probed_is_cloud() == Some(true) || self.probed_supports_devices() == Some(true)
    }

    /// True when the server is the SaaS (`api.hoard.services`) deployment,
    /// which speaks the `/v1/cloud/*` protocol instead of the self-hosted
    /// `/v1/saves` + multipart one.
    pub async fn is_cloud(&self) -> bool {
        self.server_mode().await.as_deref() == Some("cloud")
    }

    /// The per-save cap this client already knows, if any.
    ///
    /// `None` means nothing has been refused yet, so it is not known and not
    /// guessed: uploading against an invented cap would trim copies that fitted.
    pub fn plan_cap(&self) -> Option<PlanCap> {
        self.plan_cap
            .read()
            .ok()
            .and_then(|g| g.clone())
            .filter(|c| c.learned_at.elapsed() < PLAN_CAP_TTL)
    }

    /// Records the cap that just arrived in a 413. Idempotent; a different value
    /// (the user changed plan) overwrites the previous one.
    pub fn remember_plan_cap(&self, limit_bytes: u64, plan: &str) {
        if limit_bytes == 0 {
            return;
        }
        if let Ok(mut g) = self.plan_cap.write() {
            *g = Some(PlanCap {
                limit_bytes,
                plan: plan.to_string(),
                learned_at: std::time::Instant::now(),
            });
        }
    }

    /// The deployment mode **already probed**, without touching the network:
    /// `Some(true)` cloud, `Some(false)` self-hosted, `None` when no probe has
    /// succeeded yet.
    ///
    /// [`Self::is_cloud`] collapses "self-hosted" and "the probe failed" into
    /// the same `false`, which is fine for picking a protocol but not for
    /// deciding whether this deployment *has* a cloud head worth watching: the
    /// engine reports "cloud state stale" off that distinction (ADR 0021 D.11
    /// remate), and a network blip must not be read as "self-hosted, nothing to
    /// observe". Only a successful probe is cached, so `None` really means
    /// unresolved.
    pub fn probed_is_cloud(&self) -> Option<bool> {
        self.mode.get().map(|m| m.as_deref() == Some("cloud"))
    }

    // ---- Cloud (SaaS) protocol -----------------------------------------

    /// `POST /v1/cloud/saves`: declare upload intent. The server validates
    /// plan + quota, mints a presigned R2 PUT URL, and returns the version
    /// number the client must `commit` against.
    pub async fn cloud_init_upload(&self, init: &CloudUploadInit) -> Result<CloudUploadInitOut> {
        let resp = self
            .http
            .post(self.url("/v1/cloud/saves"))
            .header("authorization", self.auth_header())
            .json(init)
            .send()
            .await?;
        let resp = Self::ok_or_err(resp).await.map_err(|e| anyhow!(e))?;
        Ok(resp.json().await?)
    }

    /// Upload bytes directly to a presigned R2 URL. No `Authorization`
    /// header, since the presigned URL carries its own signature in the query
    /// string, and an extra auth header breaks the S3 v4 signature.
    pub async fn put_presigned(
        &self,
        presigned: &PresignedUrl,
        body: reqwest::Body,
        content_length: u64,
    ) -> Result<()> {
        let method = reqwest::Method::from_bytes(presigned.method.as_bytes())
            .unwrap_or(reqwest::Method::PUT);
        let resp = self
            .upload_http
            .request(method, &presigned.url)
            .header(reqwest::header::CONTENT_LENGTH, content_length)
            .body(body)
            .send()
            .await
            .map_err(|e| storage_transport_error(&presigned.url, e))?;
        if !resp.status().is_success() {
            let status = resp.status();
            // The bytes go straight to the bucket here, so this 429 is the
            // storage provider's own pacing, not ours. Surface it as a typed
            // `RateLimited` rather than a string so the blob retry loop can
            // honour it exactly like it honours the self-hosted one: a plain
            // `bail!` made the whole upload fail on a signal that only ever
            // meant "slow down".
            if status == StatusCode::TOO_MANY_REQUESTS {
                return Err(anyhow!(ApiError::from_response(resp).await));
            }
            let text = resp.text().await.unwrap_or_default();
            bail!("storage upload failed ({status}): {text}");
        }
        Ok(())
    }

    /// `POST /v1/cloud/saves/:id/versions/:n/commit`: finalize an upload.
    /// The server verifies the object via R2 HEAD and records the sha256.
    pub async fn cloud_commit(
        &self,
        save_id: &str,
        version: i64,
        commit: &CloudUploadCommit,
    ) -> Result<CloudUploadCommitOut> {
        let resp = self
            .http
            .post(self.url(&format!(
                "/v1/cloud/saves/{save_id}/versions/{version}/commit"
            )))
            .header("authorization", self.auth_header())
            .json(commit)
            .send()
            .await?;
        let resp = Self::ok_or_err(resp).await.map_err(|e| anyhow!(e))?;
        Ok(resp.json().await?)
    }

    /// `GET /v1/cloud/saves/:id/versions/:n/download`: mint a presigned R2
    /// GET URL plus the version's sha256/size for verification.
    pub async fn cloud_download(&self, save_id: &str, version: i64) -> Result<CloudDownloadOut> {
        let resp = self
            .http
            .get(self.url(&format!(
                "/v1/cloud/saves/{save_id}/versions/{version}/download"
            )))
            .header("authorization", self.auth_header())
            .send()
            .await?;
        let resp = Self::ok_or_err(resp).await.map_err(|e| anyhow!(e))?;
        Ok(resp.json().await?)
    }

    /// `GET /v1/cloud/sync`: the manifest of the user's saves (latest
    /// version of each). The cloud analogue of `list_saves`; excludes
    /// `backup_only` saves. Sends the device fingerprint so the server's
    /// poll guard can rate-limit per machine instead of per account.
    pub async fn cloud_sync(&self) -> Result<CloudManifest> {
        let dev = crate::logship::device_identity();
        let resp = self
            .http
            .get(self.url("/v1/cloud/sync"))
            .header("authorization", self.auth_header())
            .header("x-hoard-device-fp", &dev.fingerprint)
            .send()
            .await?;
        let resp = Self::ok_or_err(resp).await.map_err(|e| anyhow!(e))?;
        Ok(resp.json().await?)
    }

    /// The save ids parked in the server-side archive ("caja negra").
    ///
    /// Reads the same `/v1/cloud/storage/games` the desktop's storage screen
    /// uses, and keeps only the one field the engine needs. A frozen save
    /// refuses every upload with a 403 by design, so the watch set has to know
    /// which ones they are *before* deciding to back one up; otherwise the
    /// folder is re-hashed on every reconcile to be turned away at `cas_init`.
    ///
    /// Deserialised structurally rather than through `cloud_account`'s
    /// `StorageGames`: this only needs two fields, and staying off that type
    /// keeps a change to the quota figures from breaking the watch set.
    pub async fn cloud_archived_save_ids(&self) -> Result<HashSet<String>> {
        #[derive(serde::Deserialize)]
        struct Game {
            save_id: String,
            #[serde(default)]
            archived: bool,
        }
        #[derive(serde::Deserialize)]
        struct Games {
            #[serde(default)]
            games: Vec<Game>,
        }
        let resp = self
            .http
            .get(self.url("/v1/cloud/storage/games"))
            .header("authorization", self.auth_header())
            .send()
            .await?;
        let resp = Self::ok_or_err(resp).await.map_err(|e| anyhow!(e))?;
        let games: Games = resp.json().await?;
        Ok(games
            .games
            .into_iter()
            .filter(|g| g.archived)
            .map(|g| g.save_id)
            .collect())
    }

    /// `GET /v1/cloud/saves/:save_id/versions`: the full version history of a
    /// cloud save (every committed version, newest first). The cloud analogue
    /// of `list_save_snapshots`; the sync manifest only carries the latest.
    pub async fn cloud_list_versions(
        &self,
        save_id: &str,
        include_deleted: bool,
    ) -> Result<Vec<Snapshot>> {
        let resp = self
            .http
            .get(self.url(&format!("/v1/cloud/saves/{save_id}/versions")))
            .query(&[("include_deleted", include_deleted)])
            .header("authorization", self.auth_header())
            .send()
            .await?;
        let resp = Self::ok_or_err(resp).await.map_err(|e| anyhow!(e))?;
        Ok(resp.json().await?)
    }

    /// `DELETE /v1/cloud/saves/:save_id/versions/:version`: drop a single
    /// version (blob + row) and repoint `latest_version_num` to the highest
    /// remaining version. Deletes the whole save only if none remain.
    pub async fn cloud_delete_version(&self, save_id: &str, version: i64) -> Result<()> {
        let resp = self
            .http
            .delete(self.url(&format!("/v1/cloud/saves/{save_id}/versions/{version}")))
            .header("authorization", self.auth_header())
            .send()
            .await?;
        Self::ok_or_err(resp).await.map_err(|e| anyhow!(e))?;
        Ok(())
    }

    /// Current "max versions per save" cap for the logged-in user. `None` =
    /// unlimited. Self-hosted reads it off `whoami`; cloud reads it off
    /// `/v1/me` (other fields ignored).
    ///
    /// `manual` picks the cap: the one for copies the user asked for, or the one
    /// for the automatic ones. They are counted apart so that a burst of
    /// autosaves cannot push the deliberate copy out of the history.
    pub async fn get_max_versions(&self, manual: bool) -> Result<Option<i64>> {
        if self.is_cloud().await {
            #[derive(Deserialize)]
            struct MeMaxVersions {
                #[serde(default)]
                max_versions: Option<i64>,
                #[serde(default)]
                max_manual_versions: Option<i64>,
            }
            let resp = self.http_get("/v1/me").await?;
            let me: MeMaxVersions = resp.json().await?;
            return Ok(if manual {
                me.max_manual_versions
            } else {
                me.max_versions
            });
        }
        let who = self.whoami().await?;
        Ok(if manual {
            who.max_manual_versions
        } else {
            who.max_versions
        })
    }

    /// `PUT /v1/me/max-versions`: set (`Some(n)`) or clear (`None`) the
    /// per-user cap on stored versions per save. Both server modes mount the
    /// same path; both prune immediately, so the freed space is visible on
    /// the next quota poll.
    pub async fn set_max_versions(&self, max_versions: Option<i64>, manual: bool) -> Result<()> {
        let resp = self
            .http
            .put(self.url("/v1/me/max-versions"))
            .header("authorization", self.auth_header())
            .json(&MaxVersionsBody {
                max_versions,
                manual,
                dry_run: false,
            })
            .send()
            .await?;
        Self::ok_or_err(resp).await?;
        Ok(())
    }

    /// Dry-run of [`set_max_versions`]: how many stored versions a cap of
    /// `max_versions` would delete right now. Nothing is written. Frontends
    /// call this first and ask for confirmation when the count is > 0.
    pub async fn preview_max_versions(&self, max_versions: i64, manual: bool) -> Result<i64> {
        let resp = self
            .http
            .put(self.url("/v1/me/max-versions"))
            .header("authorization", self.auth_header())
            .json(&MaxVersionsBody {
                max_versions: Some(max_versions),
                manual,
                dry_run: true,
            })
            .send()
            .await?;
        let resp = Self::ok_or_err(resp).await?;
        let out: MaxVersionsResponse = resp.json().await?;
        Ok(out.pruned as i64)
    }

    /// `DELETE /v1/cloud/saves/:save_id`: remove a cloud save and all of its
    /// versions so the user reclaims storage. The cloud analogue of deleting
    /// a whole tracked save.
    pub async fn cloud_save_delete(&self, save_id: &str) -> Result<()> {
        let resp = self
            .http
            .delete(self.url(&format!("/v1/cloud/saves/{save_id}")))
            .header("authorization", self.auth_header())
            .send()
            .await?;
        Self::ok_or_err(resp).await.map_err(|e| anyhow!(e))?;
        Ok(())
    }

    /// GET the bytes behind a presigned download URL as a streaming response.
    /// No auth header, same rationale as [`put_presigned`].
    ///
    /// Streams the body, so it belongs on `download_http`: on `http` the 60 s
    /// total timeout also covered the streaming, and every Cloud restore of a
    /// save too big to land inside a minute died mid-body with "operation timed
    /// out", forever, since the next attempt was no faster.
    pub async fn get_presigned(&self, presigned: &PresignedUrl) -> Result<reqwest::Response> {
        let method = reqwest::Method::from_bytes(presigned.method.as_bytes())
            .unwrap_or(reqwest::Method::GET);
        let resp = self
            .download_http
            .request(method, &presigned.url)
            .send()
            .await
            .map_err(|e| storage_transport_error(&presigned.url, e))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            bail!("storage download failed ({status}): {text}");
        }
        Ok(resp)
    }

    /// `POST /v1/cloud/cas/init`: declare a content-addressed upload. Returns
    /// the new version number plus the subset of blobs the server is missing,
    /// each with a presigned PUT URL.
    pub async fn cloud_cas_init(&self, init: &CloudCasInit) -> Result<CloudCasInitOut> {
        let resp = self
            .http
            .post(self.url("/v1/cloud/cas/init"))
            .header("authorization", self.auth_header())
            .json(init)
            .send()
            .await?;
        let resp = Self::ok_or_err(resp).await.map_err(|e| anyhow!(e))?;
        Ok(resp.json().await?)
    }

    /// `POST /v1/cloud/saves/:id/versions/:n/cas/commit`: finalize a content-
    /// addressed upload once every missing blob has been PUT.
    pub async fn cloud_cas_commit(
        &self,
        save_id: &str,
        version: i64,
    ) -> Result<CloudUploadCommitOut> {
        let resp = self
            .http
            .post(self.url(&format!(
                "/v1/cloud/saves/{save_id}/versions/{version}/cas/commit"
            )))
            .header("authorization", self.auth_header())
            .send()
            .await?;
        let resp = Self::ok_or_err(resp).await.map_err(|e| anyhow!(e))?;
        Ok(resp.json().await?)
    }

    /// `GET /v1/cloud/saves/:id/versions/:n/manifest`: the per-file manifest
    /// of a content-addressed version. With `presign = true` each file carries
    /// a download URL (restore) and bandwidth is charged; with `false` it's a
    /// cheap listing (History detail). Returns `content_addressed = false` for
    /// legacy archive versions.
    pub async fn cloud_version_manifest(
        &self,
        save_id: &str,
        version: i64,
        presign: bool,
    ) -> Result<CloudVersionManifestOut> {
        let resp = self
            .http
            .get(self.url(&format!(
                "/v1/cloud/saves/{save_id}/versions/{version}/manifest"
            )))
            .query(&[("presign", presign)])
            .header("authorization", self.auth_header())
            .send()
            .await?;
        let resp = Self::ok_or_err(resp).await.map_err(|e| anyhow!(e))?;
        Ok(resp.json().await?)
    }

    /// `POST /v1/presence/heartbeat`: the presence beat (Cloud). It carries the
    /// same device identity headers as `/v1/me`, because the server resolves the
    /// `devices` row by `x-hoard-device-fp` and can even register it when a
    /// machine's first contact is this beat, which is the headless daemon's case,
    /// since it never goes through `/v1/me`.
    pub async fn presence_heartbeat(&self, playing: &[PlayingBeat], closing: bool) -> Result<()> {
        let body = serde_json::json!({ "closing": closing, "playing": playing });
        let dev = crate::logship::device_identity();
        let mut req = self
            .http
            .post(self.url("/v1/presence/heartbeat"))
            .header("authorization", self.auth_header())
            .header("x-hoard-device-fp", &dev.fingerprint)
            .header("x-hoard-device-os", &dev.os)
            .header("x-hoard-app-version", env!("CARGO_PKG_VERSION"));
        if let Some(name) = dev.name.as_deref() {
            req = req.header("x-hoard-device-name", name);
        }
        let resp = req.json(&body).send().await?;
        Self::ok_or_err(resp).await?;
        Ok(())
    }

    /// `GET /v1/devices`: the account's devices with their live presence (online,
    /// playing what, since when). The fingerprint header goes so the server can
    /// mark `this_device` and the UI can filter without knowing its own UUID.
    pub async fn list_devices(&self) -> Result<DeviceListOut> {
        let dev = crate::logship::device_identity();
        let resp = self
            .http
            .get(self.url("/v1/devices"))
            .header("authorization", self.auth_header())
            .header("x-hoard-device-fp", &dev.fingerprint)
            .send()
            .await?;
        let resp = Self::ok_or_err(resp).await?;
        Ok(resp.json().await?)
    }

    /// `GET /v1/notifications`: the operator's broadcasts, for the bell.
    /// `since` is the client's RFC3339 cursor: only strictly later rows come
    /// back, so nothing is re-delivered after a restart. The fingerprint goes
    /// along so the server's poll guard limits per machine and not per account.
    pub async fn list_notifications(&self, since: Option<&str>) -> Result<NotificationListOut> {
        let dev = crate::logship::device_identity();
        let mut req = self
            .http
            .get(self.url("/v1/notifications"))
            .header("authorization", self.auth_header())
            .header("x-hoard-device-fp", &dev.fingerprint);
        if let Some(s) = since {
            req = req.query(&[("since", s)]);
        }
        let resp = Self::ok_or_err(req.send().await?).await?;
        Ok(resp.json().await?)
    }

    /// `POST /v1/cloud/playtime` (cloud) or `/v1/playtime` (self-hosted):
    /// ship this machine's playtime breakdown so the recap can merge it with
    /// the account's other devices.
    ///
    /// Picks the path off the cached deployment probe rather than guessing:
    /// posting the cloud path at a self-hosted server is a 404, and the reverse
    /// is worse (a silent no-op). An unresolved probe means "don't know yet",
    /// we skip this round and try again on the next one instead of picking a
    /// protocol by coin flip.
    ///
    /// The caller is responsible for the consent gate (`prefs.wrapple_telemetry`);
    /// this is only the transport.
    pub async fn push_playtime(
        &self,
        body: &crate::cloud_account::PlaytimeUploadBody,
    ) -> Result<()> {
        let _ = self.server_mode().await;
        let path = match self.probed_is_cloud() {
            Some(true) => "/v1/cloud/playtime",
            Some(false) => "/v1/playtime",
            None => anyhow::bail!("deployment mode unresolved; not guessing a playtime endpoint"),
        };
        let resp = self
            .http
            .post(self.url(path))
            .header("authorization", self.auth_header())
            .json(body)
            .send()
            .await?;
        Self::ok_or_err(resp).await?;
        Ok(())
    }

    pub async fn list_games(&self, query: Option<&str>) -> Result<Vec<Game>> {
        let mut req = self
            .http
            .get(self.url("/v1/games"))
            .header("authorization", self.auth_header());
        if let Some(q) = query {
            req = req.query(&[("search", q)]);
        }
        let resp = Self::ok_or_err(req.send().await?).await?;
        Ok(resp.json().await?)
    }

    /// Paginated catalog fetch. Used by detection to walk the full ~11k-entry
    /// games table without blowing the per-request size budget. The server
    /// caps `limit` at 1000.
    pub async fn list_games_paged(&self, limit: u32, offset: u32) -> Result<Vec<Game>> {
        let resp = self
            .http
            .get(self.url("/v1/games"))
            .header("authorization", self.auth_header())
            .query(&[("limit", limit.to_string()), ("offset", offset.to_string())])
            .send()
            .await?;
        let resp = Self::ok_or_err(resp).await?;
        Ok(resp.json().await?)
    }

    pub async fn get_game(&self, slug: &str) -> Result<Game> {
        let resp = self
            .http
            .get(self.url(&format!("/v1/games/{}", slug)))
            .header("authorization", self.auth_header())
            .send()
            .await?;
        let resp = Self::ok_or_err(resp).await?;
        Ok(resp.json().await?)
    }

    pub async fn list_saves(&self, game: Option<&str>) -> Result<Vec<Save>> {
        let mut req = self
            .http
            .get(self.url("/v1/saves"))
            .header("authorization", self.auth_header());
        if let Some(g) = game {
            req = req.query(&[("game_slug", g)]);
        }
        let resp = Self::ok_or_err(req.send().await?).await?;
        Ok(resp.json().await?)
    }

    pub async fn create_save(&self, game_slug: &str, label: &str) -> Result<Save> {
        self.create_save_with_meta(game_slug, label, None, None)
            .await
    }

    /// Create a Save and, optionally, hint to the server what the game's
    /// display name / Steam ID are. Used by the desktop client so that
    /// servers running an older Ludusavi catalog can self-heal a missing
    /// games row from the metadata the desktop already has at hand. Older
    /// servers ignore the extra fields (Serde tolerates unknown keys), so
    /// this is forward-compatible.
    pub async fn create_save_with_meta(
        &self,
        game_slug: &str,
        label: &str,
        display_name: Option<&str>,
        steam_app_id: Option<i64>,
    ) -> Result<Save> {
        let body = CreateSaveRequest {
            // The `GameSlug` gate: a poisoned slug never gets to create a
            // server-side row (ADR 0021 C.3). Client slugs all come out of
            // `slugify`, so this only fires on corrupt data.
            game_slug: GameSlug::parse(game_slug)
                .with_context(|| format!("invalid slug creating the save: {game_slug:?}"))?,
            label: Some(label.to_string()),
            local_path_hint: None,
            client_os: None,
            display_name: display_name.map(str::to_string),
            steam_app_id,
        };
        let resp = self
            .http
            .post(self.url("/v1/saves"))
            .header("authorization", self.auth_header())
            .json(&body)
            .send()
            .await?;
        let resp = Self::ok_or_err(resp).await?;
        Ok(resp.json().await?)
    }

    pub async fn get_save(&self, save_id: &str) -> Result<Save> {
        let resp = self
            .http
            .get(self.url(&format!("/v1/saves/{}", save_id)))
            .header("authorization", self.auth_header())
            .send()
            .await?;
        let resp = Self::ok_or_err(resp).await?;
        Ok(resp.json().await?)
    }

    pub async fn delete_save(&self, save_id: &str) -> Result<()> {
        // Cloud speaks a different namespace (`/v1/cloud/saves/*`); the
        // self-hosted `DELETE /v1/saves/{id}` isn't mounted there and 404s,
        // which the UI mistranslates as "save no longer exists" ??? leaving
        // Cloud users unable to delete anything. Branch on the server mode.
        if self.is_cloud().await {
            return self.cloud_save_delete(save_id).await;
        }
        let resp = self
            .http
            .delete(self.url(&format!("/v1/saves/{}", save_id)))
            .header("authorization", self.auth_header())
            .send()
            .await?;
        Self::ok_or_err(resp).await?;
        Ok(())
    }

    /// Rename the label on an existing save. Surfaces 409 via
    /// [`ApiError::Conflict`] so the UI can show a "label already exists"
    /// message instead of a generic server error.
    pub async fn rename_save_label(&self, save_id: &str, new_label: &str) -> Result<Save> {
        // Same namespace split as `delete_save`: the self-hosted PATCH
        // isn't mounted on Cloud. Branch so both paths work.
        if self.is_cloud().await {
            return self.cloud_rename_save_label(save_id, new_label).await;
        }
        let body = PatchSaveRequest {
            label: Some(new_label.to_string()),
            ..PatchSaveRequest::default()
        };
        let resp = self
            .http
            .patch(self.url(&format!("/v1/saves/{}", save_id)))
            .header("authorization", self.auth_header())
            .json(&body)
            .send()
            .await?;
        let resp = Self::ok_or_err(resp).await?;
        Ok(resp.json().await?)
    }

    /// `PATCH /v1/cloud/saves/:save_id` ??? rename the label on a cloud save.
    /// The cloud analogue of `rename_save_label`; the server enforces
    /// `UNIQUE(user_id, game_slug, label)` and returns 409 on collision.
    pub async fn cloud_rename_save_label(&self, save_id: &str, new_label: &str) -> Result<Save> {
        let body = serde_json::json!({ "label": new_label });
        let resp = self
            .http
            .patch(self.url(&format!("/v1/cloud/saves/{save_id}")))
            .header("authorization", self.auth_header())
            .json(&body)
            .send()
            .await?;
        let resp = Self::ok_or_err(resp).await.map_err(|e| anyhow!(e))?;
        Ok(resp.json().await?)
    }

    pub async fn list_snapshots(
        &self,
        save_id: &str,
        include_deleted: bool,
    ) -> Result<Vec<Snapshot>> {
        let mut req = self
            .http
            .get(self.url(&format!("/v1/saves/{}/snapshots", save_id)))
            .header("authorization", self.auth_header());
        if include_deleted {
            req = req.query(&[("include_deleted", "true")]);
        }
        let resp = Self::ok_or_err(req.send().await?).await?;
        Ok(resp.json().await?)
    }

    pub async fn snapshot_detail(&self, save_id: &str, version: i64) -> Result<SnapshotDetail> {
        let resp = self
            .http
            .get(self.url(&format!("/v1/saves/{}/snapshots/{}", save_id, version)))
            .header("authorization", self.auth_header())
            .send()
            .await?;
        let resp = Self::ok_or_err(resp).await?;
        Ok(resp.json().await?)
    }

    pub async fn snapshot_download(
        &self,
        save_id: &str,
        version: i64,
    ) -> Result<reqwest::Response> {
        let resp = self
            .download_http
            .get(self.url(&format!(
                "/v1/saves/{}/snapshots/{}/download",
                save_id, version
            )))
            .header("authorization", self.auth_header())
            .send()
            .await?;
        Self::ok_or_err(resp)
            .await
            .map_err(|e| anyhow!(e))
            .context("download request failed")
    }

    pub async fn snapshot_delete(&self, save_id: &str, version: i64) -> Result<()> {
        let resp = self
            .http
            .delete(self.url(&format!("/v1/saves/{}/snapshots/{}", save_id, version)))
            .header("authorization", self.auth_header())
            .send()
            .await?;
        Self::ok_or_err(resp).await?;
        Ok(())
    }

    pub async fn snapshot_restore(&self, save_id: &str, version: i64) -> Result<()> {
        let resp = self
            .http
            .post(self.url(&format!(
                "/v1/saves/{}/snapshots/{}/restore",
                save_id, version
            )))
            .header("authorization", self.auth_header())
            .send()
            .await?;
        Self::ok_or_err(resp).await?;
        Ok(())
    }

    /// Upload a multipart snapshot. Each part is a file: name=`relative/path`, body bytes.
    /// Returns the created snapshot summary.
    pub async fn snapshot_upload(
        &self,
        save_id: &str,
        form: reqwest::multipart::Form,
    ) -> Result<Snapshot> {
        let resp = self
            .upload_http
            .post(self.url(&format!("/v1/saves/{}/snapshots", save_id)))
            .header("authorization", self.auth_header())
            .multipart(form)
            .send()
            .await?;
        let resp = Self::ok_or_err(resp).await.map_err(|e| anyhow!(e))?;
        Ok(resp.json().await?)
    }

    // ---- self-hosted, content-addressed
    //
    // The trio that replaces the multipart when the server announces it
    // ([`Self::probed_supports_cas`]). See `hoard_server::routes::cas` for the
    // why and for where it departs from the cloud one.

    /// `POST /v1/saves/{id}/cas/init`: declare the manifest. It returns which
    /// blobs are missing and the staging area to upload them to.
    pub async fn cas_init(&self, save_id: &str, init: &CasInit) -> Result<CasInitOut> {
        let resp = self
            .http
            .post(self.url(&format!("/v1/saves/{save_id}/cas/init")))
            .header("authorization", self.auth_header())
            .json(init)
            .send()
            .await?;
        let resp = Self::ok_or_err(resp).await.map_err(|e| anyhow!(e))?;
        Ok(resp.json().await?)
    }

    /// `PUT /v1/cas/blobs/{upload_id}/{sha}`: one missing blob.
    ///
    /// Goes through `upload_http` (no total timeout) for the same reason as the
    /// multipart: a blob can be gigabytes and a fixed ceiling would kill the
    /// upload halfway through.
    pub async fn cas_upload_blob(
        &self,
        upload_id: &str,
        sha256: &str,
        body: reqwest::Body,
        content_length: u64,
    ) -> Result<()> {
        let resp = self
            .upload_http
            .put(self.url(&format!("/v1/cas/blobs/{upload_id}/{sha256}")))
            .header("authorization", self.auth_header())
            .header(reqwest::header::CONTENT_LENGTH, content_length)
            .header(reqwest::header::CONTENT_TYPE, "application/octet-stream")
            .body(body)
            .send()
            .await?;
        Self::ok_or_err(resp).await.map_err(|e| anyhow!(e))?;
        Ok(())
    }

    /// `POST /v1/saves/{id}/cas/commit`: close the version.
    pub async fn cas_commit(&self, save_id: &str, commit: &CasCommit) -> Result<Snapshot> {
        let resp = self
            .http
            .post(self.url(&format!("/v1/saves/{save_id}/cas/commit")))
            .header("authorization", self.auth_header())
            .json(commit)
            .send()
            .await?;
        let resp = Self::ok_or_err(resp).await.map_err(|e| anyhow!(e))?;
        Ok(resp.json().await?)
    }
}

// ---- DTOs
//
// The self-hosted contract lives in `hoard_core::wire` (ADR 0021 C.6): the server
// compiles against the same shapes, so a drift between the two ends is a compile
// error instead of a 422 in production. They are re-exported here so `api::Save`
// and friends stay the public paths they have always been.

pub use hoard_core::wire::{
    CasCommit, CasFile, CasInit, CasInitOut, CasMissing, CreateSaveRequest, Game, Health,
    MaxVersionsBody, MaxVersionsResponse, PatchSaveRequest, Save, Snapshot, SnapshotDetail,
    SnapshotFile,
};
pub use hoard_core::wire::{LogBatch, LogEntry, LogIngestResponse, Whoami};

// ---- Cloud (SaaS) protocol DTOs ----------------------------------------

/// Body for `POST /v1/cloud/saves`. Mirrors `hoard-server`'s `UploadInit`.
#[derive(Debug, Clone, Serialize)]
pub struct CloudUploadInit {
    pub save_id: String,
    pub game_slug: String,
    pub label: Option<String>,
    pub size_bytes: u64,
    /// Files inside the packed tar.zst. The server stores it verbatim so the
    /// History view can show "N archivos" (the blob is opaque server-side).
    pub file_count: i64,
    pub device_name: Option<String>,
    pub notes: Option<String>,
    pub backup_only: bool,
    /// Last-synced version for this save. Drives the server's fast-forward
    /// check: a mismatch means another device pushed since, → 409.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_version: Option<i64>,
}

/// A short-lived presigned R2 URL (PUT for upload, GET for download).
#[derive(Debug, Clone, Deserialize)]
pub struct PresignedUrl {
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub expires_in_secs: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CloudQuotaInfo {
    pub plan: String,
    pub used_bytes: u64,
    pub limit_bytes: u64,
    #[serde(default)]
    pub devices_used: u32,
    #[serde(default)]
    pub devices_limit: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CloudUploadInitOut {
    pub version_num: i64,
    pub r2_key: String,
    pub upload: PresignedUrl,
    pub quota: CloudQuotaInfo,
}

#[derive(Debug, Clone, Serialize)]
pub struct CloudUploadCommit {
    pub sha256: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CloudUploadCommitOut {
    pub save_id: String,
    pub version_num: i64,
    pub committed: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CloudDownloadOut {
    pub save_id: String,
    pub version_num: i64,
    pub sha256: String,
    pub size_bytes: i64,
    pub download: PresignedUrl,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CloudManifestEntry {
    pub save_id: String,
    pub game_slug: String,
    pub label: String,
    pub latest_version_num: i64,
    #[serde(default)]
    pub latest_parent_version: Option<i64>,
    #[serde(default)]
    pub latest_size_bytes: i64,
    /// Files in the latest version (0 = unknown / pre-file-count server).
    #[serde(default)]
    pub latest_file_count: i64,
    #[serde(default)]
    pub latest_sha256: String,
    #[serde(default)]
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CloudManifest {
    #[serde(default)]
    pub generated_at: String,
    pub saves: Vec<CloudManifestEntry>,
}

/// The cloud keys a save by `(user, game_slug, label)`; this device keys it by
/// a local uuid it made up. `cas_init` bridges the two, accepting an id the
/// server has never seen and resolving it by name, so a device whose local id
/// drifted (a folder re-detected, a rebuilt state file) keeps uploading fine
/// while being unable to find itself in anything the server hands back.
///
/// That asymmetry is what has to be undone here, and in one place: every
/// lookup of "what does the cloud say about this save" goes through
/// [`CloudManifest::entry_for`], which tries the id and then the name, the
/// same two steps, in the same order, as the server's own `resolve_save_row`.
/// Matching by name can't collide: the cloud holds at most one row per
/// `(user, game_slug, label)`, which is also why multi-folder slots put their
/// number in the label.
impl CloudManifest {
    pub fn entry_for(
        &self,
        save_id: &str,
        game_slug: &str,
        label: &str,
    ) -> Option<&CloudManifestEntry> {
        if let Some(e) = self.saves.iter().find(|e| e.save_id == save_id) {
            return Some(e);
        }
        let want = canonical_label(label);
        self.saves
            .iter()
            .find(|e| e.game_slug == game_slug && canonical_label(&e.label) == want)
    }
}

/// An empty label is the server's `"default"`: `resolve_save_row` substitutes
/// it on the way in, so a client that stored the empty string would otherwise
/// fail to match its own row.
fn canonical_label(label: &str) -> &str {
    if label.is_empty() {
        "default"
    } else {
        label
    }
}

// ---- Cloud content-addressed (per-file dedup) DTOs ---------------------

/// One file in a content-addressed upload manifest. Mirrors the server's
/// `CasFileEntry`.
#[derive(Debug, Clone, Serialize)]
pub struct CloudCasFileEntry {
    pub relative_path: String,
    pub sha256: String,
    pub size_bytes: i64,
    /// Source file mtime (unix seconds), preserved on restore. `None` if the
    /// FS didn't report one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<i64>,
}

/// Body for `POST /v1/cloud/cas/init`. The client declares the whole-file
/// manifest; the server replies with the subset of blobs it doesn't have.
#[derive(Debug, Clone, Serialize)]
pub struct CloudCasInit {
    pub save_id: String,
    pub game_slug: String,
    pub label: Option<String>,
    pub device_name: Option<String>,
    pub notes: Option<String>,
    pub backup_only: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_version: Option<i64>,
    pub files: Vec<CloudCasFileEntry>,
}

/// A blob the server is missing, so the client must PUT it to `upload`.
#[derive(Debug, Clone, Deserialize)]
pub struct CloudCasMissingBlob {
    pub sha256: String,
    pub size_bytes: i64,
    #[serde(default)]
    pub r2_key: String,
    pub upload: PresignedUrl,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CloudCasInitOut {
    /// Canonical cloud save id (servers ≥ 2.3.2). Differs from the requested
    /// id when (user, game_slug, label) already maps to another cloud save,
    /// the commit must target this id or it 404s. `None` on older servers.
    #[serde(default)]
    pub save_id: Option<String>,
    pub version_num: i64,
    pub missing: Vec<CloudCasMissingBlob>,
    #[allow(dead_code)]
    pub quota: CloudQuotaInfo,
}

/// One file in a version manifest. `download` is present only when the
/// manifest was requested with `presign=true` (the restore path).
#[derive(Debug, Clone, Deserialize)]
pub struct CloudManifestFile {
    pub relative_path: String,
    pub sha256: String,
    pub size_bytes: i64,
    #[serde(default)]
    pub modified_at: Option<i64>,
    #[serde(default)]
    pub download: Option<PresignedUrl>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CloudVersionManifestOut {
    /// False for legacy archive versions, and the caller must fall back to the
    /// whole-archive `cloud_download` path.
    #[serde(default)]
    pub content_addressed: bool,
    #[serde(default)]
    pub files: Vec<CloudManifestFile>,
}

// The device census and presence are the SAME routes on both deployments
// (`/v1/devices`, `/v1/presence/heartbeat`), so their shapes live in
// `hoard_core::wire` and both ends compile against a single definition. They are
// re-exported here because `api::DeviceOut` is the public path the desktop
// already imports.
pub use hoard_core::wire::{DeviceListOut, DeviceOut, DevicePlaying, Heartbeat, PlayingBeat};

/// An operator broadcast (`GET /v1/notifications`). Same shape as the
/// `ServerNotification` the UI expects (stores/notifications.ts) plus
/// `created_at` for the client's cursor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationOut {
    pub id: String,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub action_url: Option<String>,
    #[serde(default)]
    pub action_label: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationListOut {
    pub notifications: Vec<NotificationOut>,
}

#[cfg(test)]
mod too_large_tests {
    use super::*;
    use hoard_core::ipc::events::TooLargeKind;

    fn parse(body: &str) -> SaveTooLarge {
        serde_json::from_str::<SaveTooLarge>(body)
            .ok()
            .filter(|d| !d.code.is_empty())
            .unwrap_or_else(SaveTooLarge::from_foreign_body)
    }

    /// The 413's two size fields mean opposite things and were split apart in a
    /// release the client ships *ahead* of the server, so for an hour or so a
    /// new client talks to a server that has never heard of the split. These
    /// are the three bodies it can get, and each has to name the right limit.
    #[test]
    fn every_shipped_413_shape_names_the_right_limit() {
        // Cloud, as production answers today: a plan cap, and the whole size
        // known up front. No `received_bytes` field at all.
        let cloud = parse(
            r#"{"error":"save exceeds per-save size limit","code":"save_too_large",
                "plan":"free","limit_bytes":1073741824,"actual_bytes":3865470566,
                "upgrade_url":"https://hoard.services/upgrade"}"#,
        );
        assert_eq!(cloud.kind(), TooLargeKind::PlanCap);
        let s = cloud.human();
        assert!(s.contains("free plan limit"), "{s}");
        assert!(!s.contains("sent before it stopped"), "{s}");

        // A self-hosted server on the old build: aborts mid-stream, so all it
        // can report is how far it got.
        let streamed = parse(
            r#"{"error":"snapshot exceeds size limit","code":"snapshot_too_large",
                "limit_bytes":1073741824,"received_bytes":1073745920}"#,
        );
        assert_eq!(streamed.kind(), TooLargeKind::ServerLimit);
        let s = streamed.human();
        assert!(s.contains("max_snapshot_size_mb"), "{s}");
        assert!(s.contains("sent before it stopped"), "{s}");

        // The new content-addressed rejection: the manifest declared the size
        // and not a byte has moved. Saying "3.6 GB sent" here was the bug.
        let declared = parse(
            r#"{"error":"snapshot exceeds size limit","code":"snapshot_too_large",
                "limit_bytes":1073741824,"actual_bytes":3865470566}"#,
        );
        assert_eq!(declared.kind(), TooLargeKind::ServerLimit);
        let s = declared.human();
        assert!(s.contains("max_snapshot_size_mb"), "{s}");
        assert!(s.contains("the save is 3.6 GB"), "{s}");
        assert!(!s.contains("sent before it stopped"), "{s}");
    }

    /// Anything that isn't Hoard's JSON came from whatever sits in front of the
    /// server, and pointing the user at Hoard's settings costs them days.
    #[test]
    fn a_proxys_html_page_is_not_read_as_a_plan_cap() {
        for body in [
            "<html><head><title>413 Request Entity Too Large</title></head></html>",
            "",
            "{}",
        ] {
            let d = parse(body);
            assert_eq!(d.kind(), TooLargeKind::Proxy, "{body}");
            assert!(d.human().contains("reverse proxy"), "{body}");
        }
    }
}

#[cfg(test)]
mod rate_limit_tests {
    use super::*;

    /// Every 429 one of our handlers writes carries a `code`, and every one of
    /// them means "this operation doesn't fit right now": wait it out, don't
    /// re-send.
    #[test]
    fn our_own_429s_are_budgets() {
        for (body, expect_secs) in [
            (
                r#"{"error":"bandwidth quota exceeded","code":"bandwidth_limit","retry_after_seconds":420}"#,
                420,
            ),
            (
                r#"{"error":"storage quota exceeded","code":"quota_exceeded_paced","retry_after_seconds":300}"#,
                300,
            ),
            (
                r#"{"error":"already downloaded","code":"restore_paced","retry_after_seconds":900}"#,
                900,
            ),
            (
                r#"{"error":"too many attempts","code":"too_many_attempts","retry_after_secs":30}"#,
                60,
            ),
        ] {
            let (kind, secs) = classify_rate_limit(body, None);
            assert_eq!(kind, RateLimitKind::Budget, "{body}");
            assert_eq!(secs, expect_secs, "{body}");
        }
    }

    /// A full account says the same thing whether it is answered with the 402
    /// or, once the brake engages, with the paced 429, so the figures behind
    /// "free up space / go Pro" have to survive the switch.
    #[test]
    fn a_paced_quota_429_still_carries_the_full_account() {
        let body = r#"{"error":"storage quota exceeded; retries are being spaced out",
            "code":"quota_exceeded_paced","retry_after_seconds":3600,"repeated":7,
            "plan":"free","used_bytes":2147483648,"limit_bytes":2147483648,
            "requested_bytes":1048576,"upgrade_url":"https://hoard.services/upgrade"}"#;
        let d = paced_quota_detail(body).expect("the quota figures ride in the paced body");
        assert_eq!(d.plan, "free");
        assert_eq!(d.limit_bytes, 2 * 1024 * 1024 * 1024);
        assert_eq!(
            d.upgrade_url.as_deref(),
            Some("https://hoard.services/upgrade")
        );

        // Every other budget 429 is genuinely just a wait: nothing to offer,
        // nothing to explain, and inventing a "you are full" prompt out of a
        // bandwidth window would be worse than saying nothing.
        for other in [
            r#"{"error":"bandwidth quota exceeded","code":"bandwidth_limit","retry_after_seconds":420}"#,
            r#"{"error":"already downloaded","code":"restore_paced","retry_after_seconds":900}"#,
            "Too Many Requests! Wait for 0s",
            "",
        ] {
            assert!(paced_quota_detail(other).is_none(), "{other}");
        }
    }

    /// The per-IP pacer is middleware: it never sees our types, so its answer
    /// has no `code` and its wait rides in `x-ratelimit-after`. Reading that as
    /// a budget is what wedged large self-hosted uploads.
    #[test]
    fn the_per_ip_pacer_is_not_a_budget() {
        let (kind, secs) = classify_rate_limit("Too Many Requests! Wait for 0s", Some(0));
        assert_eq!(kind, RateLimitKind::Paced);
        assert_eq!(secs, 0);
    }

    /// A reverse proxy's own limiter, which the self-host guide tells
    /// operators to put in front, answers HTML with no headers at all. Same
    /// meaning as our pacer, so the same handling, and crucially *not* the
    /// 60-second default that a budget gets: 122 blobs served a fabricated
    /// minute each is a two-hour upload.
    #[test]
    fn an_unstructured_429_paces_rather_than_inventing_a_minute() {
        let (kind, secs) = classify_rate_limit(
            "<html><head><title>429 Too Many Requests</title></head></html>",
            None,
        );
        assert_eq!(kind, RateLimitKind::Paced);
        assert_eq!(secs, 0);
    }

    /// `Retry-After` still wins when it's there: a proxy that speaks the
    /// standard header is telling the truth about its own window.
    #[test]
    fn a_standard_retry_after_header_is_honoured() {
        let (kind, secs) = classify_rate_limit("slow down", Some(5));
        assert_eq!(kind, RateLimitKind::Paced);
        assert_eq!(secs, 5);
    }
}

#[cfg(test)]
mod plan_cap_tests {
    use super::*;

    fn client() -> ApiClient {
        ApiClient::new("http://127.0.0.1:1", "t").expect("client")
    }

    /// With no prior rejection the cap is unknown, and it is not invented:
    /// trimming against a guessed number would mutilate copies that did fit.
    #[test]
    fn unknown_until_a_rejection_teaches_it() {
        assert!(client().plan_cap().is_none());
    }

    #[test]
    fn remembers_what_the_rejection_said() {
        let c = client();
        c.remember_plan_cap(50 * 1024 * 1024, "free");
        let cap = c.plan_cap().expect("learned");
        assert_eq!(cap.limit_bytes, 50 * 1024 * 1024);
        assert_eq!(cap.plan, "free");
    }

    /// Changing plan overwrites the old cap as soon as the server says so.
    #[test]
    fn a_new_limit_replaces_the_old_one() {
        let c = client();
        c.remember_plan_cap(50, "free");
        c.remember_plan_cap(500, "pro");
        let cap = c.plan_cap().expect("learned");
        assert_eq!(cap.limit_bytes, 500);
        assert_eq!(cap.plan, "pro");
    }

    /// A body with no usable `limit_bytes` (a proxy that returned its own 413)
    /// teaches nothing: better not to know than to learn a zero and trim
    /// everything.
    #[test]
    fn a_zero_limit_teaches_nothing() {
        let c = client();
        c.remember_plan_cap(0, "free");
        assert!(c.plan_cap().is_none());
    }

    /// Todas las copias de un cliente comparten el tope: aprenderlo subiendo
    /// un juego vale para el siguiente.
    #[test]
    fn clones_share_the_learned_cap() {
        let c = client();
        let clone = c.clone();
        c.remember_plan_cap(50, "free");
        assert_eq!(clone.plan_cap().expect("shared").limit_bytes, 50);
    }
}

/// The two-key bridge: the cloud names a save `(user, game, label)`, this
/// machine names it a uuid, and `cas_init` accepts either. These pin the
/// resolution order down, because getting it wrong is silent: a miss reads
/// exactly like "the server has nothing for this save".
#[cfg(test)]
mod manifest_resolution_tests {
    use super::*;

    fn entry(save_id: &str, game_slug: &str, label: &str, v: i64) -> CloudManifestEntry {
        CloudManifestEntry {
            save_id: save_id.into(),
            game_slug: game_slug.into(),
            label: label.into(),
            latest_version_num: v,
            latest_parent_version: None,
            latest_size_bytes: 0,
            latest_file_count: 0,
            latest_sha256: String::new(),
            updated_at: String::new(),
        }
    }

    fn manifest(saves: Vec<CloudManifestEntry>) -> CloudManifest {
        CloudManifest {
            generated_at: String::new(),
            saves,
        }
    }

    /// The id wins when the server knows it. Nothing clever happens in the
    /// common case.
    #[test]
    fn the_id_matches_first() {
        let m = manifest(vec![
            entry("mine", "factorio", "main", 7),
            entry("theirs", "factorio", "other", 9),
        ]);
        let e = m.entry_for("mine", "factorio", "main").expect("by id");
        assert_eq!(e.save_id, "mine");
    }

    /// The aug-2026 case. A local id the cloud has never seen still finds its
    /// row by name, the same fallback `resolve_save_row` does on the way in,
    /// so the client sees the row it is actually uploading to.
    #[test]
    fn an_unknown_id_falls_back_to_game_and_label() {
        let m = manifest(vec![entry("cloud-side", "factorio", "main", 284)]);
        let e = m
            .entry_for("local-only", "factorio", "main")
            .expect("by name");
        assert_eq!(e.save_id, "cloud-side");
        assert_eq!(e.latest_version_num, 284);
    }

    /// Two folders of one game are two rows, told apart by the label (that is
    /// what multi-folder slots put their number in). The fallback must not
    /// collapse them onto whichever came first.
    #[test]
    fn the_label_keeps_slots_of_one_game_apart() {
        let m = manifest(vec![
            entry("row-1", "factorio", "main", 284),
            entry("row-2", "factorio", "2 · shit3", 23),
        ]);
        let e = m
            .entry_for("local-only", "factorio", "2 · shit3")
            .expect("by name");
        assert_eq!(e.save_id, "row-2");
    }

    /// An empty label is the server's `"default"`, on both sides of the
    /// comparison, since otherwise a save stored with one spelling never matches
    /// its own row.
    #[test]
    fn an_empty_label_is_the_servers_default() {
        let m = manifest(vec![entry("cloud-side", "factorio", "default", 3)]);
        assert_eq!(
            m.entry_for("local-only", "factorio", "")
                .expect("empty matches default")
                .save_id,
            "cloud-side"
        );
        let m = manifest(vec![entry("cloud-side", "factorio", "", 3)]);
        assert_eq!(
            m.entry_for("local-only", "factorio", "default")
                .expect("default matches empty")
                .save_id,
            "cloud-side"
        );
    }

    /// No row for this game at all stays a miss. The fallback resolves an id,
    /// it doesn't invent a save.
    #[test]
    fn a_game_with_no_row_still_misses() {
        let m = manifest(vec![entry("row-1", "stellaris", "main", 4)]);
        assert!(m.entry_for("local-only", "factorio", "main").is_none());
    }
}

/// The 409 body carries the two things the retry needs. It used to arrive as a
/// string and both were dropped on the floor.
#[cfg(test)]
mod non_fast_forward_tests {
    use super::*;

    #[test]
    fn a_tagged_409_parses_head_and_canonical_id() {
        let body = r#"{"error":"non-fast-forward: another device advanced this save since your base version","code":"non_fast_forward","head_version":284,"base_version":283,"save_id":"cloud-side"}"#;
        let d: NonFastForward = serde_json::from_str(body).expect("parses");
        assert_eq!(d.head(), Some(284));
        assert_eq!(d.canonical_id_for("local-only"), Some("cloud-side"));
        assert!(d.human().contains("head 284, base 283"));
    }

    /// A server that answers with the id we sent has nothing to relabel: the
    /// caller must not log a divergence that didn't happen.
    #[test]
    fn the_same_id_back_is_not_a_divergence() {
        let body =
            r#"{"code":"non_fast_forward","head_version":9,"base_version":8,"save_id":"mine"}"#;
        let d: NonFastForward = serde_json::from_str(body).expect("parses");
        assert_eq!(d.canonical_id_for("mine"), None);
    }

    /// A server too old to send the fields degrades to "we diverged, and that
    /// is all I know": `head()` stays `None` rather than claiming version 0,
    /// which is what keeps the caller from rebasing onto a number we invented.
    #[test]
    fn an_older_server_says_nothing_rather_than_zero() {
        let body = r#"{"error":"non-fast-forward: another device advanced this save since your base version","code":"non_fast_forward"}"#;
        let d: NonFastForward = serde_json::from_str(body).expect("parses");
        assert_eq!(d.head(), None);
        assert_eq!(d.canonical_id_for("mine"), None);
        assert!(!d.human().contains("head"));
    }
}
