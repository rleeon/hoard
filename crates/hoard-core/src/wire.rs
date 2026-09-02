//! Wire types shared by client and server (ADR 0021, C.6).
//!
//! These shapes used to be maintained by hand on both sides. One definition now
//! means drift is a compile error instead of a 422 in production, and there was
//! real drift when this was written: the client sent `LogEntry.target` and `ts`
//! as required fields while the server declared them `Option`, and the two ends'
//! save types had been carrying different subsets of the same columns since
//! 2025.
//!
//! ## Scope
//!
//! The self-hosted contract: `/v1/health`, `/v1/auth/whoami`,
//! `/v1/me/max-versions`, `/v1/games`, `/v1/saves`, `/v1/saves/*/snapshots` and
//! `/v1/logs`, which the cloud namespace reuses as-is. The cloud-only DTOs are
//! still duplicated; they are the obvious next increment, not part of this one.
//!
//! ## Compatibility discipline, read before touching anything
//!
//! Compiling is not enough, because client and server deploy separately. A
//! self-hoster runs a server three releases old against a desktop updated this
//! morning, and Hoard Cloud updates the server without anybody touching the
//! installed clients. So:
//!
//! - Append only. Fields get added, never removed.
//! - Every new field carries `#[serde(default)]`, and `Option` when there is no
//!   sensible default, so an older version's JSON still deserialises.
//! - Never repurpose a field. Changing the meaning or the type of an existing
//!   one is indistinguishable from corruption at the other end. If the meaning
//!   changes, it is a new field.
//! - `#[serde(skip_serializing_if)]` only where the field is not emitted today.
//!   The golden test (`tests/golden_wire.rs`) pins the last release's bytes; if
//!   a change moves them, it fails and someone has to justify it.
//!
//! ## Persisted state is not new data
//!
//! The newtypes in [`crate::ids`] carry the strict gate in `serde`, so a
//! poisoned value never arrives over the wire. But the server builds these
//! responses by reading its own DB, which is persisted state and can hold years
//! of poison, so there it uses the lenient gate
//! ([`crate::ids::GameSlug::repair`]) and never `parse`. One bad row would
//! otherwise take down a user's whole listing. Same rule as `state.json`
//! (ADR C.3).

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::ids::{GameSlug, MachineId, SaveId, Sha256, Username};
use crate::kernel::insight::VersionInsight;

/// RFC3339, the representation that already crossed the wire.
///
/// The self-hosted server writes timestamps into SQLite with
/// `strftime('%Y-%m-%dT%H:%M:%SZ','now')` and used to hand them back as a
/// `String`. Unifying the type moves no bytes: `time` emits `Z` for a zero
/// offset, which is exactly what SQLite writes, and parses both that and the
/// `+00:00` the cloud namespace emits.
use time::serde::rfc3339 as ts;

// ---- GET /v1/health

/// The client branches its whole protocol on [`Health::mode`], which makes this
/// the most load-bearing type in the contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Health {
    /// `"ok"` or `"db_error"`.
    pub status: String,
    /// The server binary's version.
    pub version: String,
    #[serde(default)]
    pub uptime_secs: u64,
    /// Lowest log level this server accepts on the client log ingest. Absent on
    /// pre-ingest servers, which the client reads as "this server takes no logs"
    /// and stops sending.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log_min_level: Option<String>,
    /// `"cloud"` on the SaaS deployment, absent self-hosted. Selects the
    /// namespace: `/v1/cloud/*` or `/v1/saves`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    /// This server speaks the content-addressed protocol
    /// (`/v1/saves/{id}/cas/*`): the client declares the manifest and uploads
    /// only the blobs the server is missing, instead of shipping the whole
    /// folder in a multipart.
    ///
    /// A capability, not a preference. Absent means a server older than 1.1.3
    /// that only understands multipart, and the client cannot infer it from the
    /// version because server and client versions move independently. Omitted
    /// when `false` so the release golden still matches byte for byte.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub cas: bool,
    /// This server keeps a device registry and live presence (`/v1/devices`,
    /// `/v1/presence/heartbeat`). Same discipline as [`Health::cas`]: a property
    /// of the binary, not a setting. Absent means a server older than 1.1.3,
    /// which should not be sent heartbeats.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub devices: bool,
}

// ---- GET /v1/auth/whoami and PUT /v1/me/max-versions

/// Identity plus quota for the authenticated user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Whoami {
    pub user_id: String,
    pub username: Username,
    pub is_admin: bool,
    /// Bytes stored right now. Servers before v0.3 omit it, so it reads as 0.
    #[serde(default)]
    pub storage_used_bytes: i64,
    /// Total quota. Servers before v0.3 omit it, and the UI reads the resulting
    /// 0 as "quota unknown".
    #[serde(default)]
    pub storage_quota_bytes: i64,
    /// Cap on stored versions per save. `None` is unlimited, and also what a
    /// server without the feature reports. Counts automatic versions only.
    #[serde(default)]
    pub max_versions: Option<i64>,
    /// Cap on deliberate copies: the ones the user asked for, plus the safety
    /// net taken before a restore. `None` is unlimited, which is the default,
    /// because there are few of them and they are precisely the ones worth
    /// keeping. An older server does not send it and it reads as unlimited,
    /// which is how it behaved.
    ///
    /// Omitted when there is no cap so the release golden still matches byte for
    /// byte; absent and `null` read the same thanks to the `default`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_manual_versions: Option<i64>,
    /// This server's per-snapshot ceiling (`storage.max_snapshot_size_mb`, in
    /// bytes). `None` on a server old enough not to report it, and always on
    /// Cloud, where the equivalent is the plan's per-save cap that
    /// `/v1/cloud/me` already carries.
    ///
    /// It lives here and not in `/v1/health` on purpose: health is anonymous,
    /// and an operator's ceiling is nobody's business until they authenticate.
    /// The client shows it so the number is on screen *before* a backup bounces
    /// off it. A self-hoster whose config still had the old 1 GB example spent a
    /// support round finding out it existed (aug-2026).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_snapshot_size_bytes: Option<i64>,
}

/// Body of `PUT /v1/me/max-versions`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaxVersionsBody {
    /// `null` removes the cap.
    pub max_versions: Option<i64>,
    /// Which budget this is about: `true` for deliberate copies, `false` (the
    /// default) for automatic ones. An older client omits the field and touches
    /// the automatic budget, which is what it touched before this existed.
    ///
    /// Omitted when `false`, on the same reasoning as [`Health::cas`]: the
    /// release golden keeps matching, and the server reads absence as the
    /// automatic budget it always meant.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub manual: bool,
    /// Dry run: writes and deletes nothing, only counts how many snapshots the
    /// cap *would* drop. The client shows that in the confirmation.
    #[serde(default)]
    pub dry_run: bool,
}

/// Response of `PUT /v1/me/max-versions`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaxVersionsResponse {
    pub max_versions: Option<i64>,
    /// Echo of which budget was touched. Omitted for the automatic one, for the
    /// same reason as in [`MaxVersionsBody::manual`].
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub manual: bool,
    /// Snapshots binned for exceeding the new cap, or under `dry_run`, the ones
    /// that would be.
    #[serde(default)]
    pub pruned: u64,
}

// ---- GET /v1/games

/// One entry of the game catalogue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub slug: GameSlug,
    pub display_name: String,
    pub engine: Option<String>,
    #[serde(default)]
    pub save_paths_json: Option<String>,
}

// ---- /v1/saves

/// Body of `POST /v1/saves`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSaveRequest {
    pub game_slug: GameSlug,
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_path_hint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_os: Option<String>,
    /// Optional metadata from newer clients. When the server's `games` table
    /// does not know the slug (a server seeded from an older catalogue), this
    /// upserts a stub row instead of returning 422.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub steam_app_id: Option<i64>,
}

/// Body of `PATCH /v1/saves/{id}`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PatchSaveRequest {
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub local_path_hint: Option<String>,
    #[serde(default)]
    pub client_os: Option<String>,
}

/// A save: the union of what the server used to emit and what the client used
/// to expect. Fields only one end had are `Option` plus `default` so no
/// deployed version breaks.
///
/// The aggregates (`snapshot_count`, `total_size_bytes`) are `Option` because
/// cloud does not compute them, and `None` ("I don't know") is not `Some(0)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Save {
    pub id: SaveId,
    /// Cloud only. Skipped when missing so self-hosted keeps emitting exactly
    /// the release's JSON.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    pub game_slug: GameSlug,
    pub label: String,
    #[serde(default)]
    pub local_path_hint: Option<String>,
    #[serde(default)]
    pub client_os: Option<String>,
    #[serde(default)]
    pub latest_version_num: Option<i64>,
    #[serde(default)]
    pub snapshot_count: Option<i64>,
    #[serde(default)]
    pub total_size_bytes: Option<i64>,
    #[serde(with = "ts")]
    pub created_at: OffsetDateTime,
    #[serde(with = "ts")]
    pub updated_at: OffsetDateTime,
}

// ---- /v1/saves/{id}/snapshots

/// Who asked for this version.
///
/// It matters for retention. A game that autosaves every minute fills the whole
/// budget in one session, and if every version competes for the same slot, that
/// burst of automatic copies evicts the one the user deliberately made before a
/// boss. Which is the one they wanted to keep. With the origin recorded, each
/// class gets its own budget and an automatic burst can only push out other
/// automatic ones.
///
/// It travels in the snapshot's `notes` field, which had existed unused from the
/// start. `Automatic` writes nothing, so every row from before this (with a null
/// `notes`) reads as automatic, which is what they are, and nothing needs
/// migrating.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionOrigin {
    /// The engine did it on its own: the timer, the game closing, the sweep.
    Automatic,
    /// The user asked for it.
    Manual,
    /// The safety net taken before overwriting the folder on a restore. Counts
    /// as manual: it is the copy that lets a wrong restore be undone, and losing
    /// it means losing exactly what made it valuable.
    PreRestore,
}

impl VersionOrigin {
    /// What gets stored in `notes`. `None` for automatic ones.
    pub fn as_note(self) -> Option<&'static str> {
        match self {
            Self::Automatic => None,
            Self::Manual => Some("manual"),
            Self::PreRestore => Some("pre-restore"),
        }
    }

    /// Reads the origin off a `notes`. Anything unrecognised is automatic:
    /// that is what old rows were, and what a note written by some future
    /// version this client does not understand should be.
    pub fn from_note(note: Option<&str>) -> Self {
        match note.map(str::trim) {
            Some("manual") => Self::Manual,
            Some("pre-restore") => Self::PreRestore,
            _ => Self::Automatic,
        }
    }

    /// Does it count against the deliberate budget?
    pub fn is_deliberate(self) -> bool {
        matches!(self, Self::Manual | Self::PreRestore)
    }
}

/// Summary of one snapshot, meaning one version of a save.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub save_id: Option<SaveId>,
    pub version_num: i64,
    /// Parent in the DAG, `None` for a root. The edge that makes divergence
    /// detectable. Older servers omit it and it reads as `None`.
    #[serde(default)]
    pub parent_version: Option<i64>,
    #[serde(default)]
    pub device_name: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    pub total_size_bytes: i64,
    pub file_count: i64,
    pub is_pinned: bool,
    #[serde(default, with = "ts::option")]
    pub deleted_at: Option<OffsetDateTime>,
    #[serde(with = "ts")]
    pub created_at: OffsetDateTime,
    /// What this version is *about*: the save's name, what changed since the
    /// previous one, how many saves the folder holds. Derived by the server
    /// from the manifest, so an old client that ignores it loses nothing and a
    /// server that doesn't know about it omits the field entirely.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub insight: Option<VersionInsight>,
}

/// A snapshot plus its file listing (`GET /v1/saves/{id}/snapshots/{n}`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotDetail {
    #[serde(flatten)]
    pub snapshot: Snapshot,
    pub files: Vec<SnapshotFile>,
}

/// One file inside a snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotFile {
    pub relative_path: String,
    pub size_bytes: i64,
    /// `None` means *unknown*, not "bad hash". It happens with the legacy
    /// whole-archive versions, where the listing is synthesised by reading the
    /// tar and there is no per-file digest. In JSON that is the empty string,
    /// which is what the release emitted: see [`sha_opt`].
    #[serde(default, with = "sha_opt")]
    pub sha256: Option<Sha256>,
}

/// `Option<Sha256>` with `""` as the JSON shape of `None`.
///
/// The release already used the empty string for "no digest", so relaxing
/// [`Sha256`]'s gate to let it through would have put an impossible value inside
/// the type. The right home for "not applicable" is the `Option`, and this
/// module translates between the two representations without moving a byte on
/// the wire.
mod sha_opt {
    use super::Sha256;
    use serde::{de::Error as _, Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(value: &Option<Sha256>, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(value.as_ref().map(Sha256::as_str).unwrap_or(""))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Sha256>, D::Error> {
        let raw = Option::<String>::deserialize(d)?;
        match raw.as_deref() {
            None | Some("") => Ok(None),
            Some(s) => Sha256::parse(s).map(Some).map_err(D::Error::custom),
        }
    }
}

// ---- /v1/saves/{id}/cas/*, content-addressed upload (self-hosted)
//
// Until 1.1.2 self-hosted could only upload multipart: the whole folder on every
// copy, even when the server already held 99% of the bytes. It deduplicated on
// store (blobs by sha, ADR 0018), not in transit. A 3 GB save with 10 MB of
// changes cost 3 GB of upload, and on top of that ran into
// `storage.max_snapshot_size_mb` and any proxy with a body limit in front.
//
// These three calls are the negotiation Hoard Cloud already had: declare the
// manifest, upload what is missing, commit. The difference from cloud is where
// the bytes land. Cloud signs R2 URLs and the client writes into the bucket;
// here the client never talks to storage at all (ADR 0020), so every missing
// blob goes to the server and the server places it.

/// One manifest file: which path, which content, how big.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CasFile {
    pub relative_path: String,
    pub sha256: Sha256,
    pub size_bytes: i64,
    /// Source mtime in unix seconds. This is what lets the history say *which*
    /// save was touched; without it every file in the folder looks equally
    /// recent. Cloud already stored it and this end did not.
    ///
    /// Absent when the filesystem does not report one, and on every client older
    /// than this. The server treats it as unknown, never as zero.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<i64>,
}

/// Body of `POST /v1/saves/{id}/cas/init`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CasInit {
    /// The version the client believes it is building on. As in the multipart
    /// path: if it is no longer the head, another machine got there first and
    /// this is rejected, here *before* a byte moves, which is the whole point.
    #[serde(default)]
    pub base_version: Option<i64>,
    pub files: Vec<CasFile>,
}

/// A blob the server does not have and the client must upload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CasMissing {
    pub sha256: Sha256,
    pub size_bytes: i64,
}

/// Response of `cas/init`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CasInitOut {
    /// Identifies this upload's staging area
    /// (`PUT /v1/cas/blobs/{upload_id}/{sha}`). The server mints it, and it
    /// expires with the `tmp/` sweep (`retention.tmp_cleanup_hours`), so an
    /// abandoned upload cleans itself up.
    pub upload_id: String,
    /// The version the commit will produce if nobody gets there first.
    /// Indicative: the real number is assigned by the commit, inside the same
    /// transaction that checks the head.
    pub version_num: i64,
    /// What has to be uploaded. Everything else in the manifest is already
    /// stored.
    pub missing: Vec<CasMissing>,
    /// Bytes that will really travel (the sum of `missing`), against the save's
    /// logical size. The difference is what dedup saves, and the client uses it
    /// so the progress bar measures the actual upload.
    pub missing_bytes: i64,
}

/// Body of `POST /v1/saves/{id}/cas/commit`. It repeats the manifest, because
/// the server keeps nothing between init and commit except the staged bytes. So
/// a commit is self-sufficient and a lost init leaves no half-built row in the
/// database, which is what forces cloud to carry "pending" versions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CasCommit {
    pub upload_id: String,
    #[serde(default)]
    pub base_version: Option<i64>,
    #[serde(default)]
    pub device_name: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    pub files: Vec<CasFile>,
}

// ---- GET /v1/devices and POST /v1/presence/heartbeat
//
// Both deployments mount these same routes (they do not live under `/v1/cloud/`),
// so here both ends really do compile against one definition. The client already
// used them against cloud; self-hosted serves them from 1.1.3 on.
//
// What they answer, and why all three at once matter: which machine each version
// came from (`Snapshot::device_name` already carries that), which machines on
// the account exist at all, and which are switched on right now and playing
// what.

/// One game in a heartbeat: slug plus how many seconds it has been running.
///
/// A duration rather than a timestamp, deliberately. The server anchors it to
/// *its* clock (`now - secs`), so a client with a skewed clock cannot claim to
/// have been playing since three minutes into the future.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayingBeat {
    pub slug: String,
    pub for_secs: u64,
}

/// Body of `POST /v1/presence/heartbeat`. Everything optional: an idle machine's
/// keepalive is `{}`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Heartbeat {
    /// Games running now, most recent first. Empty means idle.
    #[serde(default)]
    pub playing: Vec<PlayingBeat>,
    /// The final heartbeat of an orderly shutdown: turns the dot off at once
    /// instead of waiting for the machine to age out of the window.
    #[serde(default)]
    pub closing: bool,
}

/// A game running on a device (`GET /v1/devices`): slug plus the RFC3339 start
/// of the session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevicePlaying {
    pub slug: String,
    #[serde(default)]
    pub since: Option<String>,
}

/// One device on the account, with its live presence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceOut {
    pub id: String,
    pub device_name: String,
    #[serde(default)]
    pub device_kind: Option<String>,
    #[serde(default)]
    pub os: Option<String>,
    #[serde(default)]
    pub last_seen_at: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    /// A fresh heartbeat and no closing one. Computed on read rather than
    /// stored, so a machine that died without saying goodbye switches itself off
    /// instead of staying lit forever.
    #[serde(default)]
    pub online: bool,
    /// Games running right now, most recent first; only present when `online`.
    /// Empty means idle.
    #[serde(default)]
    pub playing: Vec<DevicePlaying>,
    /// True on the row matching the caller's own fingerprint, so the UI can
    /// recognise itself without knowing its own id.
    #[serde(default)]
    pub this_device: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceListOut {
    pub devices: Vec<DeviceOut>,
}

// ---- POST /v1/logs (the cloud namespace mounts this same body)

/// The `target` of the contradictions: events saying where detection got it
/// wrong and what the human did to fix it (`hoard_agent::telemetry`).
///
/// It lives here because it is a contract on both sides: the client exempts it
/// from its level filter and the server accepts it below the minimum it
/// advertises. A `where target = 'hoard::telemetry'` is the whole query.
pub const TELEMETRY_TARGET: &str = "hoard::telemetry";

/// The `target` of Hoard Screen's telemetry: when the overlay opens, how long it
/// stays up and what gets put inside it (`hoard_desktop::screen_telemetry`).
///
/// Separate from [`TELEMETRY_TARGET`] on purpose, rather than being one more
/// `verdict`. They answer two different questions ("where does detection fail"
/// and "does anyone use the overlay"), and mixing them forces every query on
/// either to filter by `fields`. With its own target, each panel is one
/// `where target = ...`.
pub const SCREEN_TARGET: &str = "hoard::screen";

/// Targets that travel whatever their level.
///
/// Both are INFO and both have to reach Cloud, whose minimum is WARN. They are
/// listed here, in the contract, so adding a third does not mean editing client
/// and server separately, which is exactly how this broke the first time.
pub const EXEMPT_TARGETS: [&str; 2] = [TELEMETRY_TARGET, SCREEN_TARGET];

/// The Terms version the client asks people to accept, exactly as the website
/// shows it: a date, not a semver.
///
/// A contract on both sides. The client sends it to `POST /v1/me/terms` on
/// login and the server stores it verbatim; `GET /v1/me` returns the last one
/// accepted, and if it does not match this the client asks for the checkbox
/// again. That is why it is a date: what has to be matchable on the day of a
/// dispute is "which text was published when this person said yes", and the date
/// is what the page shows.
///
/// Bump it only when the substance of the document changes. A comma that moves
/// this literal interrupts everybody for nothing. It pairs with `TERMS_VERSION`
/// in `web/src/lib/legal.ts`.
pub const TERMS_VERSION: &str = "2026-08-11";

/// Device metadata, sent once per batch.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeviceMeta {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub os: Option<String>,
    /// The machine's fingerprint, the same one device registration uses, so a
    /// log row and the device count join on this field.
    #[serde(default)]
    pub fingerprint: Option<MachineId>,
    #[serde(default)]
    pub app_version: Option<String>,
}

/// One log line sent by a client.
///
/// `target` and `ts` are `Option` because that is how the server declared them.
/// The client always sent both, but the contract tolerates their absence and
/// breaking that would reject older clients' batches.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub level: String,
    #[serde(default)]
    pub target: Option<String>,
    pub message: String,
    /// Structured fields as a JSON object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fields: Option<serde_json::Value>,
    /// The client's timestamp (RFC3339). Kept as a `String`: it is diagnostic
    /// data from an end whose clock may be wrong, and the server stores it
    /// verbatim without interpreting it.
    #[serde(default)]
    pub ts: Option<String>,
}

/// A batch of logs (`POST /v1/logs`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogBatch {
    pub device: DeviceMeta,
    pub entries: Vec<LogEntry>,
}

/// Response of the log ingest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogIngestResponse {
    pub accepted: usize,
}

/// Level ordering, so this can compare without depending on `tracing`, which the
/// server does not have. An unknown level counts as INFO: nothing is dropped for
/// not being recognised.
pub fn level_rank(level: &str) -> u8 {
    match level.trim().to_ascii_lowercase().as_str() {
        "trace" => 0,
        "debug" => 1,
        "warn" => 3,
        "error" => 4,
        _ => 2, // info y cualquier cosa que no conozcamos
    }
}

/// What Cloud keeps: WARN. With the whole population sending, operational INFO
/// fills the table with noise and the 14-day prune takes the rare cases with it,
/// and the rare cases are the useful ones.
pub const CLOUD_MIN_RANK: u8 = 3;

/// The rule about which line travels and which gets stored, written once for
/// both sides: the client filters at source with it, and the server uses it
/// because it does not trust the client.
///
/// It used to be written three times, in the agent, the server and cloud, and
/// that is a leak waiting to happen. If the client filters at one level and the
/// server at another, either bandwidth is spent on lines the server throws away
/// or lines the client sends are dropped, and both silently. Same reason
/// `LogEntry` lives here rather than duplicated (ADR 0021 C.6).
///
/// The per-target exception is the heart of it: detection contradictions and
/// Screen telemetry are INFO and have to reach Cloud, whose minimum is WARN. See
/// [`EXEMPT_TARGETS`].
pub fn ships_at(entry: &LogEntry, min_rank: u8) -> bool {
    entry
        .target
        .as_deref()
        .is_some_and(|t| EXEMPT_TARGETS.contains(&t))
        || level_rank(&entry.level) >= min_rank
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(level: &str, target: &str) -> LogEntry {
        LogEntry {
            level: level.to_string(),
            target: Some(target.to_string()),
            message: String::new(),
            fields: None,
            ts: None,
        }
    }

    /// The whole matrix of the shared rule. Client and server both call in
    /// here, so this test is what stops them drifting apart again.
    #[test]
    fn one_rule_decides_what_travels_and_what_is_stored() {
        // Cloud (WARN): operational noise stays out.
        for level in ["trace", "debug", "info", "notice", "TRACE"] {
            assert!(
                !ships_at(&entry(level, "hoard_agent::agent"), CLOUD_MIN_RANK),
                "{level} should not get into Cloud"
            );
        }
        for level in ["warn", "WARN", " error ", "error"] {
            assert!(
                ships_at(&entry(level, "hoard_agent::agent"), CLOUD_MIN_RANK),
                "{level} should get into Cloud"
            );
        }
        // Contradictions reach Cloud even at INFO, which is their level.
        assert!(ships_at(&entry("info", TELEMETRY_TARGET), CLOUD_MIN_RANK));
        // ...and even a DEBUG on that target, in case it ever drops a level.
        assert!(ships_at(&entry("debug", TELEMETRY_TARGET), CLOUD_MIN_RANK));
        // Same for Screen, for the same reason: if its INFO does not reach
        // Cloud, the overlay panel reads zero and nothing looks broken.
        for level in ["info", "debug", "trace"] {
            assert!(ships_at(&entry(level, SCREEN_TARGET), CLOUD_MIN_RANK));
        }
        // A target that merely looks like ours does not slip through.
        assert!(!ships_at(
            &entry("info", "hoard::screenshot"),
            CLOUD_MIN_RANK
        ));
        // Self-hosted (DEBUG) keeps almost everything, but not TRACE.
        assert!(ships_at(&entry("debug", "hoard_agent::agent"), 1));
        assert!(!ships_at(&entry("trace", "hoard_agent::agent"), 1));
        // With no target (an older client, or a half-built entry) the level
        // decides.
        let mut naked = entry("info", "x");
        naked.target = None;
        assert!(!ships_at(&naked, CLOUD_MIN_RANK));
    }

    /// A level we do not know is not dropped for being unknown; it counts as
    /// INFO.
    #[test]
    fn an_unknown_level_ranks_as_info() {
        assert_eq!(level_rank("notice"), level_rank("info"));
        assert_eq!(level_rank(""), level_rank("info"));
        assert_eq!(level_rank(" WARN "), level_rank("warn"));
    }

    /// Timestamps go out with the release's `Z` suffix and come back in both
    /// that shape and the `+00:00` the cloud namespace emits.
    #[test]
    fn timestamps_keep_the_release_shape() {
        let json = r#"{
            "id": "3f2504e0-4f89-41d3-9a0c-0305e82c3301",
            "game_slug": "stardew-valley",
            "label": "default",
            "local_path_hint": null,
            "client_os": null,
            "latest_version_num": 7,
            "snapshot_count": 3,
            "total_size_bytes": 1024,
            "created_at": "2026-07-24T12:34:56Z",
            "updated_at": "2026-07-24T12:34:56+00:00"
        }"#;
        let save: Save = serde_json::from_str(json).unwrap();
        let out = serde_json::to_value(&save).unwrap();
        assert_eq!(out["created_at"], "2026-07-24T12:34:56Z");
        assert_eq!(out["updated_at"], "2026-07-24T12:34:56Z");
        // An absent `user_id` is not emitted: the self-hosted bytes do not move.
        assert!(out.get("user_id").is_none());
    }

    /// A save with a poisoned slug does not get in over the wire. This is the
    /// strict half of ADR C.3: new data goes through the gate.
    #[test]
    fn poisoned_slug_never_crosses_the_wire() {
        let json = r#"{
            "id": "3f2504e0-4f89-41d3-9a0c-0305e82c3301",
            "game_slug": "GSE Saves",
            "label": "default",
            "created_at": "2026-07-24T12:34:56Z",
            "updated_at": "2026-07-24T12:34:56Z"
        }"#;
        assert!(serde_json::from_str::<Save>(json).is_err());
    }

    /// JSON from an older version, without the fields added since, still
    /// deserialises. The append-only discipline as a test.
    #[test]
    fn older_payloads_still_deserialize() {
        let old_health = r#"{"status":"ok","version":"0.2.0"}"#;
        let h: Health = serde_json::from_str(old_health).unwrap();
        assert_eq!(h.uptime_secs, 0);
        assert!(h.mode.is_none());

        let old_whoami = r#"{"user_id":"u1","username":"jacka","is_admin":false}"#;
        let w: Whoami = serde_json::from_str(old_whoami).unwrap();
        assert_eq!(w.storage_quota_bytes, 0);
        assert!(w.max_versions.is_none());

        let old_snapshot = r#"{
            "id":"s1","version_num":1,"total_size_bytes":10,"file_count":2,
            "is_pinned":false,"created_at":"2026-07-24T12:34:56Z"
        }"#;
        let s: Snapshot = serde_json::from_str(old_snapshot).unwrap();
        assert!(s.parent_version.is_none());
        assert!(s.deleted_at.is_none());
        assert!(s.save_id.is_none());
    }

    /// `SnapshotDetail` flattens the summary: the snapshot's fields sit at the
    /// same level as `files`, as in the release.
    #[test]
    fn snapshot_detail_stays_flat() {
        let json = r#"{
            "id":"s1","version_num":1,"total_size_bytes":10,"file_count":1,
            "is_pinned":false,"created_at":"2026-07-24T12:34:56Z",
            "files":[{"relative_path":"a.sav","size_bytes":10,
                      "sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}]
        }"#;
        let d: SnapshotDetail = serde_json::from_str(json).unwrap();
        assert_eq!(d.files.len(), 1);
        let out = serde_json::to_value(&d).unwrap();
        assert_eq!(out["version_num"], 1);
        assert!(out["files"].is_array());
    }
}
