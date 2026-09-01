//! The local service's IPC protocol (ADR 0021, part A).
//!
//! The engine stopped being embedded in every frontend and moved into a process
//! of its own (`hoardd`, one engine per user). Desktop and CLI connect to it
//! over a local socket, a UDS on Linux and macOS, a named pipe on Windows, and
//! this module is what travels over it: the envelopes, the requests, the
//! replies, the event journal and the framing.
//!
//! It lives in `hoard-core` for the same reason [`crate::wire`] does (C.6): a
//! contract cannot belong to one of its two ends. `hoard-core` is the leaf
//! kernel, `serde` and nothing else, no `tokio`, so the types and the framing
//! are here while the transport (bind, accept, permissions, reading and writing
//! the socket) lives in the daemon crate, which has a runtime.
//!
//! ## Framing
//!
//! Every message goes out as a big-endian `u32` length followed by that many
//! bytes of JSON. Framed rather than line-delimited because an event carries
//! file paths (`SaveConflictsBackedUp::conflict_dir`) and a `\n` inside a field
//! must not be syntax. The cap ([`MAX_FRAME_BYTES`]) is checked *before* the
//! buffer is reserved: a local socket is still untrusted input, and a 4 GiB
//! prefix must not become a 4 GiB allocation.
//!
//! ## Versioned handshake
//!
//! There are now two or more updatable artefacts (service, app, CLI) that have
//! to speak the same protocol, and they do not update together: someone updates
//! the app and the user service stays on the old binary until they log back in.
//! So the client sends [`Hello`] with its [`PROTOCOL_VERSION`] and the daemon
//! answers [`Welcome`] or [`Rejected`] with its own, which lets the client say
//! "update or restart the service" instead of dying on a parse error.
//!
//! Within one protocol version, [`crate::wire`]'s discipline applies: append
//! only, `#[serde(default)]` on every new field, never repurpose a field. The
//! version goes up only when a change is not compatible.
//!
//! ## Event delivery: journal plus push
//!
//! Both, not one or the other (D.14.2). The client connects, asks for everything
//! after its cursor ([`Request::Subscribe`]), gets a [`Payload::Backlog`], and
//! listens for live [`ServerFrame::Event`]s from there. See [`journal`].

pub mod events;
pub mod journal;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

pub use events::{AgentEvent, AgentSlotStatus, BackupReason};
pub use journal::{Backlog, JournalEntry};

/// Protocol version. Goes up only on an incompatible change; adding a field with
/// `#[serde(default)]`, or a variant the other side can ignore, is not one.
pub const PROTOCOL_VERSION: u32 = 1;

/// Header bytes on every frame (the length `u32`).
pub const HEADER_BYTES: usize = 4;

/// Frame cap. Real messages are hundreds of bytes; the cap exists so an absurd
/// length prefix cannot become an absurd allocation. The largest imaginable
/// backlog, 1024 journal rows, fits with room to spare.
pub const MAX_FRAME_BYTES: usize = 8 * 1024 * 1024;

/// A framing failure. Unlike an application error ([`IpcError`]), this means the
/// connection is no longer trustworthy and gets closed.
#[derive(Debug, thiserror::Error)]
pub enum FrameError {
    #[error("frame of {size} bytes exceeds the {MAX_FRAME_BYTES}-byte limit")]
    TooLarge { size: usize },
    #[error("malformed frame: {0}")]
    Malformed(#[from] serde_json::Error),
}

/// Serialises `msg` as a complete frame: header plus JSON.
pub fn encode_frame<T: Serialize>(msg: &T) -> Result<Vec<u8>, FrameError> {
    let body = serde_json::to_vec(msg)?;
    if body.len() > MAX_FRAME_BYTES {
        return Err(FrameError::TooLarge { size: body.len() });
    }
    let mut out = Vec::with_capacity(HEADER_BYTES + body.len());
    out.extend_from_slice(&(body.len() as u32).to_be_bytes());
    out.extend_from_slice(&body);
    Ok(out)
}

/// The length a header declares, validated against [`MAX_FRAME_BYTES`]. The
/// reader calls this before reserving the body buffer.
pub fn frame_len(header: [u8; HEADER_BYTES]) -> Result<usize, FrameError> {
    let size = u32::from_be_bytes(header) as usize;
    if size > MAX_FRAME_BYTES {
        return Err(FrameError::TooLarge { size });
    }
    Ok(size)
}

/// Deserialises a frame body.
pub fn decode_frame<T: DeserializeOwned>(body: &[u8]) -> Result<T, FrameError> {
    Ok(serde_json::from_slice(body)?)
}

// ---- handshake

/// The client's first frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hello {
    pub protocol: u32,
    /// Who is calling, for the daemon's logs: `"hoard-desktop 7.7.16"`.
    pub client: String,
}

/// Handshake accepted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Welcome {
    pub protocol: u32,
    pub daemon_version: String,
    pub pid: u32,
    /// Identity of *this run* of the daemon. Journal `seq`s start over on every
    /// boot, so a stored cursor is only worth anything if the epoch matches; when
    /// it changed, the client starts from 0. Without this, a client holding
    /// cursor 500 against a freshly restarted daemon would sit waiting for events
    /// that already happened.
    pub epoch: String,
    /// The journal cursor right now, so the client knows how much is there
    /// without asking.
    pub cursor: u64,
}

/// Handshake rejected. Carries the daemon's version so the client can say what
/// needs updating.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rejected {
    pub reason: String,
    pub daemon_protocol: u32,
    pub daemon_version: String,
}

// ---- envelopes

/// A client-to-daemon frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "frame", rename_all = "snake_case")]
pub enum ClientFrame {
    Hello(Hello),
    /// The client picks `id` and it comes back in [`ServerFrame::Reply`], so
    /// several requests can be in flight on one connection.
    Request {
        id: u64,
        request: Request,
    },
}

/// A daemon-to-client frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "frame", rename_all = "snake_case")]
pub enum ServerFrame {
    Welcome(Welcome),
    Rejected(Rejected),
    Reply {
        id: u64,
        reply: Reply,
    },
    /// Live push: one new journal row. Collapses, meaning runs of the same rest,
    /// are not pushed. See [`journal::Appended`].
    Event(JournalEntry),
    /// The client could not keep up with the push channel and missed rows. It
    /// must re-issue [`Request::Subscribe`] from its cursor. The honest
    /// alternative to swallowing the gap in silence.
    Resync {
        cursor: u64,
        dropped: u64,
    },
    /// The service is stopping on purpose and says goodbye before closing the
    /// socket (ADR 0021 D.17).
    ///
    /// Without this, a connected client cannot tell "it was stopped" from "it
    /// crashed", and since its reconnect is spawn-if-absent, a `hoard sync stop`
    /// resurrected the service about three seconds later: a deliberate shutdown
    /// would not stay down. With the goodbye, the client keeps reconnecting, so
    /// it latches on if somebody starts it again, but does not start it itself. A
    /// daemon that really dies (panic, OOM, kill -9) sends nothing, so there the
    /// client still brings it back up, which is right.
    Goodbye {
        reason: String,
    },
    /// A frame this client does not know, sent by a newer daemon.
    ///
    /// Two or more artefacts update separately, so a daemon can learn a frame
    /// before the client does. Without this variant the first unknown frame would
    /// be a framing error, and broken framing drops the connection, which is a
    /// wildly disproportionate response to "I don't know what this is". Ignoring
    /// it is what lets frames be added inside one protocol version, which is
    /// exactly what [`ServerFrame::Goodbye`] just did.
    #[serde(other)]
    Unknown,
}

/// Requests. A mirror of `AgentHandle`'s public surface: the IPC *is* the remote
/// `AgentHandle`, so a new command in the engine shows up here rather than in
/// every frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Request {
    /// Heartbeat: confirms the other end is a live daemon.
    Ping,
    /// Full status of the daemon and of every watched slot.
    Status,
    /// Backlog since `since` (or from the beginning) plus a subscription to the
    /// live push. `None` means "I have no cursor, give me what there is".
    Subscribe {
        since: Option<u64>,
    },
    BackupNow {
        save_id: String,
    },
    SweepAll {
        window_secs: u64,
    },
    ForceRestore {
        save_id: String,
        /// Head the caller already knows (SSE `save` frame, cloud poller).
        /// Kernel `cloud_ahead` needs this in cache; a bare `ForceRestore` is
        /// only a tick nudge and no-ops on self-hosted when heads were never
        /// observed. `None` from older clients; the engine still reconciles.
        #[serde(default)]
        version_num: Option<i64>,
    },
    SetAutoRestore {
        enabled: bool,
    },
    SetGlobalSync {
        enabled: bool,
    },
    /// The set of tracked saves changed on disk (`state.json`): re-hydrate it.
    /// The daemon owns the state, so the client *tells* it rather than sending
    /// the list. A `WatchedSave` on the wire would be the client deciding what
    /// the engine watches.
    Reload,
    /// Candidate folders, detected but not tracked, that the engine should probe
    /// to correlate process against writes. It is the one thing a client does
    /// send as a list: detection lives in the frontend (Slice 8 moves it) and the
    /// engine cannot guess it.
    ///
    /// They travel as `String` because the wire is JSON: a non-UTF-8 path does
    /// not fit and gets dropped on the client, which is where that can be said.
    SetProbeCandidates {
        dirs: Vec<String>,
    },
    /// Lend me a valid Cloud token. The daemon is the only thing that rotates
    /// `cloud.toml` (part A, "a single rotator"), so anyone who needs to talk to
    /// the cloud, the desktop for its REST calls or the CLI for a one-shot, asks
    /// here instead of refreshing on its own. Two processes rotating the same
    /// refresh token is GoTrue's reuse detection, which revokes the whole family:
    /// a permanent 401 and a dead session with no way back.
    ///
    /// `rejected` is the token the client just had refused with a 401. Without
    /// it, a client that eats a 401 on a token that still looks fresh (revoked
    /// server-side, skewed clock) would get the same token back over and over and
    /// retry in a loop. With it, the daemon rotates only if the token it would
    /// serve is that one; if somebody else already rotated, it answers with the
    /// new one without spending a rotation.
    CloudToken {
        #[serde(default)]
        rejected: Option<String>,
    },
    /// Take this freshly minted Cloud session and store it yourself. The client
    /// has just finished an OAuth (or an email login) and hands over the pair
    /// rather than writing it: the daemon is the only thing that touches the
    /// secret store.
    ///
    /// This is not symmetry for its own sake with [`Request::CloudToken`], it is
    /// what fixes the password dialogs on macOS. There every keychain item
    /// carries an ACL of authorised binaries, and whoever *creates* the item is
    /// the only one on that list. With login writing from the app and the engine
    /// reading from `hoardd`, every read by the service was a foreign binary
    /// asking permission, so one dialog per read, with the keeper retrying every
    /// few seconds. With the daemon writing it, creator and reader are the same
    /// binary and there is nothing to authorise. On Linux (Secret Service) and
    /// Windows (Credential Manager) the secret is not tied to a binary, so there
    /// this is just consistency: the engine owns the secret, clients are views.
    ///
    /// Implies the effect of [`Request::RestartEngine`]: we have just learned a
    /// new session, and whatever engine is running is talking to the old one.
    AdoptSession {
        session: AdoptedSession,
    },
    /// Forget the Cloud session (logout, or a deleted account). The client asks
    /// for the same reason as [`Request::AdoptSession`]: deleting a keychain item
    /// also has to be authorised, and the one that can do it without asking the
    /// user anything is its owner. Implies restarting the engine.
    ForgetSession,
    /// Take this self-hosted session and store it yourself. The twin of
    /// [`Request::AdoptSession`] for someone's own server: the client validates
    /// the token against `/v1/auth/whoami` and hands over `(url, token, user)`.
    ///
    /// It fixes two things at once, and the second is the big one. The first is
    /// the same macOS keychain ACL as [`Request::AdoptSession`]. The second is
    /// that the engine could not see the desktop's session at all: the app stored
    /// into `credentials` (keychain plus `session.toml`) while the engine
    /// resolved self-hosted by reading `config.toml`, which only
    /// `hoard login --token` writes. Two disjoint stores, so anyone who logged in
    /// through the app alone had an engine syncing nothing. With one owner there
    /// is one store.
    ///
    /// Implies the effect of [`Request::RestartEngine`], like its twin.
    AdoptServerSession {
        session: ServerSession,
    },
    /// Forget the self-hosted session (logout). Like [`Request::ForgetSession`],
    /// but for someone's own server.
    ForgetServerSession,
    /// Lend me the token for my own server. The twin of [`Request::CloudToken`]
    /// for self-hosted, and far simpler: a `hoard_v1_...` token is static, it
    /// neither expires nor rotates, so there is no rotation to decide here, only
    /// the store, which belongs to the daemon.
    ///
    /// It also returns `user`, so a client that lost its `session.toml` (the ACL
    /// an old Windows build used to leave stuck) recovers who it is without
    /// waiting for the daemon to repair the file.
    ServerToken,
    /// The on-disk session changed: drop the engine and let the keeper bring it
    /// back up resolving credentials afresh.
    ///
    /// No login needs it any more, since all four session requests
    /// (`AdoptSession`/`ForgetSession` and their self-hosted twins) carry it. It
    /// stays because it is still the way to say "I touched the disk underneath
    /// you": a `hoard login --token` with no service in reach, a hand-edited
    /// `config.toml`.
    ///
    /// Different from [`Request::Reload`], which only re-hydrates the set of
    /// saves: an account change invalidates the `ApiClient`, the `state.json`
    /// context and the token rotator, and none of the three is fixed by adding
    /// and removing slots. And different from [`Request::Shutdown`]: the service
    /// stays alive, it just changes session.
    RestartEngine,
    /// Stop the engine and the daemon. An explicit user order (`hoard sync
    /// stop`), not a side effect of closing a client: closing the app can never
    /// kill sync, which is the point of the whole slice.
    Shutdown,
    /// How is the update going? The service owns the updater, being the one
    /// thing always running, so clients do not look at GitHub, they ask it.
    /// Answers [`Payload::Update`].
    UpdateStatus,
    /// Apply whatever has been downloaded, now. A client asks when there is
    /// somebody in front of it: the Settings button, `hoard upgrade`, or the
    /// window on open.
    ///
    /// The difference from waiting for the background cycle is not urgency, it is
    /// permission: with a human present the service can fire a `pkexec` and the
    /// polkit dialog has somebody to ask. In the background cycle it cannot,
    /// which is why a `.deb` does not update itself.
    ///
    /// `version` is the one the client believed it was applying. If another one
    /// shipped meanwhile, the service answers with its new state rather than
    /// knowingly installing something that is no longer the latest.
    ApplyUpdate {
        #[serde(default)]
        version: Option<String>,
    },
    /// Not now. Silences whatever can be postponed for `hours`. It does not move
    /// the deadline: postponing delays the question, not the due date.
    SnoozeUpdate {
        hours: u32,
    },
    /// A request this daemon does not know, sent by a newer client.
    ///
    /// Without this variant the first unknown request would be a *framing* error,
    /// and broken framing drops the connection, so a client updated thirty
    /// seconds ago talking to the old service would lose the service instead of
    /// getting an "I can't do that". With it the answer is
    /// [`IpcError::Unsupported`] and the connection stays alive, which is what
    /// lets requests be added without bumping the protocol version (C.6).
    #[serde(other)]
    Unknown,
}

/// The answer to a request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum Reply {
    Ok(Payload),
    Error(IpcError),
}

/// The payload of a successful reply.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "payload", rename_all = "snake_case")]
pub enum Payload {
    /// Accepted. Engine commands are fire and forget; what happened afterwards
    /// arrives through the journal.
    Ack,
    Pong {
        daemon_version: String,
        pid: u32,
    },
    Status(DaemonStatus),
    Backlog(Backlog),
    CloudToken(CloudToken),
    /// The lent self-hosted session (answer to [`Request::ServerToken`]).
    ServerSession(ServerSession),
    /// How the update is going (answer to [`Request::UpdateStatus`] and to
    /// [`Request::ApplyUpdate`]).
    Update(UpdateState),
}

/// Everything the service knows about the update, which is all of it: what is
/// running, what has shipped, what has been downloaded and when it stops being
/// optional.
///
/// It is all a client needs to draw the update, and deliberately includes
/// nothing a client could *decide*. The policy lives in
/// `hoard_agent::install::auto` and the service runs it; this is the view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateState {
    /// The version running in the service.
    pub current: String,
    /// The latest published one, if it could be asked for.
    #[serde(default)]
    pub latest: Option<String>,
    /// The one downloaded and verified, ready to apply in the time a `rename`
    /// takes.
    #[serde(default)]
    pub staged: Option<String>,
    pub phase: UpdatePhase,
    /// When it stops being optional. `None` means nothing is pending.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub deadline: Option<OffsetDateTime>,
    /// The deadline passed: the window must not let anyone carry on unupdated.
    #[serde(default)]
    pub mandatory: bool,
    /// This machine relieves itself (AppImage, per-user NSIS, a core in the home
    /// directory). `false` means a human is needed, since a `.deb` wants polkit
    /// and a `.dmg` wants a hand, and it is what decides whether the client has
    /// to show something or can stay quiet.
    #[serde(default)]
    pub unattended: bool,
    /// What went wrong on the last attempt, if anything did.
    #[serde(default)]
    pub last_error: Option<String>,
}

/// Where it has got to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum UpdatePhase {
    /// There is nothing newer.
    UpToDate,
    /// Downloading and verifying.
    Downloading,
    /// Downloaded, waiting for the moment or for somebody to say yes.
    Ready,
    /// Downloaded and held back, with a reason.
    Waiting { hold: UpdateHold },
    /// Being applied right now.
    Applying,
    /// Applied. The service is relieving itself with the new binary.
    Restarting,
    /// The last attempt failed; the reason is in `last_error`.
    Failed,
    /// We update nothing here: a third party maintains it (the distro's package
    /// manager, Flatpak, a `nix`).
    Managed,
    /// A phase this client does not know, from a newer daemon.
    #[serde(other)]
    Unknown,
}

/// Why an already-downloaded update is being held back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateHold {
    /// A backup or restore is half done. Always holds, deadline or not.
    TransferInFlight,
    /// A game is open. Holds the silent path, not the mandatory one.
    GameRunning,
    /// A reason this client does not know.
    #[serde(other)]
    Unknown,
}

/// A Cloud session a client hands to the daemon ([`Request::AdoptSession`]).
///
/// It is the only place in the protocol a refresh token travels, and it goes one
/// way: client to daemon, once, when the session is minted. Coming back, only
/// the access token is lent ([`CloudToken`]), never the refresh. A client that
/// does not hold it cannot rotate it, and that is the rule holding up "a single
/// rotator".
#[derive(Clone, Serialize, Deserialize)]
pub struct AdoptedSession {
    /// Which Cloud the session belongs to.
    pub server_url: String,
    /// A freshly issued JWT.
    pub access_token: String,
    /// The refresh token the daemon will renew with from now on.
    pub refresh_token: String,
}

/// By hand, and redacted: the derived `Debug` would print both tokens, and one
/// `?request` in a daemon log is enough to put the whole session in the system
/// journal, which is plain text and outlives the logout.
impl std::fmt::Debug for AdoptedSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdoptedSession")
            .field("server_url", &self.server_url)
            .field("access_token", &"<redacted>")
            .field("refresh_token", &"<redacted>")
            .finish()
    }
}

/// A self-hosted session: which server, which token, whose.
///
/// It travels both ways, and that is right here: a token for someone's own
/// server is static (`hoard_v1_` plus 64 hex) and never rotates, so handing it
/// over and lending it are the same shape. Not so on Cloud, where the refresh
/// token only goes in ([`AdoptedSession`]) and only the access comes out
/// ([`CloudToken`]).
#[derive(Clone, Serialize, Deserialize)]
pub struct ServerSession {
    /// URL of the user's own server.
    pub server_url: String,
    /// The bearer token.
    pub token: String,
    /// A snapshot of `/v1/auth/whoami` so the client knows who it is without a
    /// network call. `None` when it could not be asked.
    #[serde(default)]
    pub user: Option<ServerUser>,
}

/// Like [`AdoptedSession`]: `Debug` by hand so the token never turns up in a log
/// by accident.
impl std::fmt::Debug for ServerSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServerSession")
            .field("server_url", &self.server_url)
            .field("token", &"<redacted>")
            .field("user", &self.user)
            .finish()
    }
}

/// Who the user is on their own server. Mirrors
/// `hoard_agent::credentials::UserSection`, which is what the app caches on disk;
/// it lives here because the wire cannot depend on the agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerUser {
    pub user_id: String,
    pub username: String,
    pub is_admin: bool,
}

/// A Cloud token lent by the daemon (answer to [`Request::CloudToken`]).
///
/// A loan, not a transfer: the client uses it for its requests and does not
/// persist it. The full pair, access plus refresh, lives where it always did, in
/// the keyring and `cloud.toml`, and only the daemon writes it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudToken {
    /// A Supabase JWT with enough life left to use right now.
    pub access_token: String,
    /// Which Cloud server it belongs to. The client should not assume the
    /// default, since a dev build points somewhere else by environment.
    pub server_url: String,
    /// The JWT's `exp` in epoch seconds, when it could be read. Lets the client
    /// get ahead of expiry without decoding anything.
    #[serde(default)]
    pub expires_at: Option<i64>,
    /// The daemon rotated to serve this answer. Informational, for logs: the
    /// client does not care, and that is precisely why it is no longer its
    /// business.
    #[serde(default)]
    pub rotated: bool,
}

/// An application error. The connection stays alive, unlike with [`FrameError`].
///
/// It implements `Error`, and therefore `Display`, on purpose: the client
/// propagates it as-is and the message ends up in a desktop toast or on the
/// CLI's stdout. Dumping it with `{:?}` would show the user
/// `EngineDown { reason: ... }`.
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
#[serde(tag = "error", rename_all = "snake_case")]
pub enum IpcError {
    /// The daemon is up and the engine is not. `reason` explains why (no
    /// session, another agent holds the engine, a failing start). A client that
    /// only saw "error" would retry forever with nothing to tell the user.
    #[error("the Hoard service has no engine: {reason}")]
    EngineDown { reason: String },
    /// There is no Cloud session to lend and rotating will not fix it: either
    /// there is no session on disk, or GoTrue revoked the whole token family
    /// (reuse detection). Only a fresh login gets it back.
    ///
    /// Its own variant because the client behaves differently: a transient
    /// failure it retries, this one it logs out locally and asks the user to sign
    /// in again. Before Slice 4c each frontend made that distinction by
    /// downcasting its own `RefreshTokenStale`; now it travels on the wire,
    /// because the daemon is what discovers it.
    #[error("the Hoard Cloud session is gone: {reason}")]
    CloudSessionExpired { reason: String },
    /// There is no self-hosted session on this machine. The twin of
    /// [`IpcError::CloudSessionExpired`] for someone's own server, and a separate
    /// variant on purpose: a `hoard_v1_` token never expires, so this only means
    /// "there is no session here", never "it expired". Merging them would make a
    /// self-hosted client fire the *Cloud* session cleanup, which is what
    /// `CloudSessionExpired` triggers on the desktop.
    #[error("there is no self-hosted session on this machine: {reason}")]
    NoServerSession { reason: String },
    /// That request does not exist in this version of the protocol.
    #[error("this Hoard service doesn't support `{op}`")]
    Unsupported { op: String },
    #[error("the Hoard service couldn't do it: {message}")]
    Internal { message: String },
}

/// The daemon's status. What a client needs to draw something without having
/// seen a single event: the snapshot the silent bells were missing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonStatus {
    pub daemon_version: String,
    pub protocol: u32,
    pub pid: u32,
    pub epoch: String,
    pub uptime_secs: u64,
    /// The journal cursor, for starting a subscription with no backlog.
    pub cursor: u64,
    /// The service sends the OS's native notifications itself (ADR 0021 D.14.1).
    /// A client reading `true` must not send its own, or the user sees every
    /// notice twice with the app open.
    ///
    /// `false` means "this daemon build cannot notify on this platform yet"
    /// (today: Windows and macOS), and then the notice is still the frontend's,
    /// which is exactly how it used to work. A new field with a `default`, so an
    /// older client reads `false` and keeps notifying as before (C.6, append
    /// only).
    #[serde(default)]
    pub notifications: bool,
    pub engine: EngineStatus,
    pub slots: Vec<AgentSlotStatus>,
}

/// The engine's status inside the daemon.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EngineStatus {
    pub running: bool,
    /// The server the engine talks to (`"Cloud · ..."`, or the self-hosted URL).
    #[serde(default)]
    pub server: Option<String>,
    #[serde(default)]
    pub is_cloud: bool,
    #[serde(default)]
    pub watched: usize,
    /// How long this engine has been alive.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub since: Option<OffsetDateTime>,
    /// Why it is not up, or the last failed attempt. An engine that is down
    /// *with a reason* is diagnosable; without one it is the invisible failure
    /// that D.11 and D.12 cost two sessions.
    #[serde(default)]
    pub last_error: Option<String>,
    /// The same reason in a type the UI can translate and offer the fixing
    /// button for. A new field with a `default`, so an older client reads
    /// [`EngineDownReason::Unknown`] and draws what it drew before (C.6, append
    /// only, the protocol does not go up).
    #[serde(default)]
    pub reason: EngineDownReason,
    /// Which way the keyring failed, when [`EngineDownReason::KeyringUnreadable`]
    /// is the reason. `None` for any other reason, and on a service too old to
    /// classify it, and the window then shows the general keyring sentence,
    /// which is what it showed before this field existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keyring: Option<KeyringFault>,
}

/// Why there is no engine, classified at source.
///
/// `last_error` is for the log and for us; this is for the screen. It came out of
/// two support threads (jul-2026, self-hosted 1.1.0) where the user could only
/// say "the sync service is offline": the reason existed in here and was lost
/// before it reached the window, so neither of them could report the one thing
/// needed to diagnose it.
///
/// Classified by typed downcast, not by looking at the error's text: a message
/// gets rewritten without a second thought and nobody notices the classification
/// broke, which is exactly the silent failure this exists to kill.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineDownReason {
    /// Nobody said anything: the engine is up, starting, or being reported by a
    /// daemon older than this field.
    #[default]
    Unknown,
    /// There is no session to use. Signing in fixes it; nothing else does.
    NoSession,
    /// There is a stored session and the keyring will not hand it over: locked,
    /// no D-Bus in a session with no desktop, or a macOS ACL that does not
    /// authorise this binary. Distinct from [`Self::NoSession`] because the
    /// advice is the opposite: here the user *did* sign in, and signing in again
    /// rewrites the item in the name of whoever reads it.
    KeyringUnreadable,
    /// Terminally expired session (Cloud): only a fresh login fixes it.
    SessionExpired,
    /// We couldn't ask. The service didn't answer a status query, so nothing is
    /// known about the engine, including whether it is running.
    ///
    /// Never sent by the daemon: it is what a *client* fills in when its own
    /// read failed, and it exists so that "I couldn't ask" stops borrowing the
    /// sentence for "it is stopped". They are not the same fact, and on
    /// 2026-08-28 the difference was the whole complaint: the service had been
    /// up for thirteen hours and the window said it was stopped.
    Unreachable,
    /// Anything else. `last_error` carries the detail.
    Other,
}

/// How the keyring failed, when [`EngineDownReason::KeyringUnreadable`] is the
/// reason.
///
/// The reason is one; the advice is not. A machine with no secret-service daemon
/// at all is never going to answer, and telling that user to unlock their login
/// keyring sends them looking for something that isn't installed. A locked one
/// unlocks. A damaged entry is rewritten by signing in again. Four errors from
/// production, four different next steps:
/// `The name is not activatable` (nothing to talk to), `Did not receive a reply`
/// (there, mute), `Crypto error: Unpad Error` (there, answering, and what it
/// holds can't be decrypted) and our own five-second cap.
///
/// Travels as a **new optional field** on [`EngineStatus`] rather than as new
/// [`EngineDownReason`] variants, and that is deliberate: a field an older client
/// doesn't know is a field it ignores, while a variant it doesn't know fails the
/// parse of the whole status and leaves it with no data at all about the daemon.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyringFault {
    /// There is no secret-service daemon on this machine to talk to.
    Missing,
    /// It's there and it doesn't answer: locked and waiting on an unlock prompt
    /// nobody can see, or a session bus that swallowed the call.
    Locked,
    /// It answered and said no: a macOS ACL that authorises a different binary,
    /// or a denied access rule.
    Refused,
    /// It answered, and what it holds can't be read back: a corrupt entry, a
    /// crypto session that won't negotiate.
    Damaged,
    /// A fault a newer service classified and this build doesn't know. Keeps an
    /// older client parsing a newer status instead of dropping it whole.
    #[serde(other)]
    #[default]
    Unknown,
}

impl KeyringFault {
    /// Stable tag for the wire and for the UI to key its sentence on.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Locked => "locked",
            Self::Refused => "refused",
            Self::Damaged => "damaged",
            Self::Unknown => "unknown",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_round_trip() {
        let msg = ClientFrame::Request {
            id: 7,
            request: Request::BackupNow {
                save_id: "s1".into(),
            },
        };
        let bytes = encode_frame(&msg).unwrap();
        let header: [u8; HEADER_BYTES] = bytes[..HEADER_BYTES].try_into().unwrap();
        let len = frame_len(header).unwrap();
        assert_eq!(len, bytes.len() - HEADER_BYTES);
        let back: ClientFrame = decode_frame(&bytes[HEADER_BYTES..]).unwrap();
        assert!(matches!(
            back,
            ClientFrame::Request {
                id: 7,
                request: Request::BackupNow { .. }
            }
        ));
    }

    /// An absurd header is rejected *before* anything is reserved. A local
    /// socket with 0600 permissions is still input that has to be validated.
    #[test]
    fn an_absurd_header_is_rejected_before_allocating() {
        let err = frame_len(u32::MAX.to_be_bytes()).unwrap_err();
        assert!(matches!(err, FrameError::TooLarge { .. }));
    }

    #[test]
    fn a_truncated_body_is_a_frame_error_not_a_panic() {
        let bytes = encode_frame(&Hello {
            protocol: PROTOCOL_VERSION,
            client: "test".into(),
        })
        .unwrap();
        let body = &bytes[HEADER_BYTES..bytes.len() - 3];
        assert!(decode_frame::<Hello>(body).is_err());
    }

    /// The JSON shape of the events is the contract Slice 4a moved out of
    /// `hoard_agent::agent` into [`events`]. If anyone renames a variant or a
    /// field, this fails: the desktop keys the UI off that `type` name and the
    /// daemon stores it in the journal.
    #[test]
    fn events_wire_shape_is_frozen() {
        let ev = AgentEvent::BackupSuccess {
            save_id: "s1".into(),
            version_num: 42,
            total_bytes: 1024,
            set_hash: Some("cheap:content".into()),
            already_landed: false,
            deliberate: true,
        };
        let json = serde_json::to_value(&ev).unwrap();
        assert_eq!(json["type"], "backup_success");
        assert_eq!(json["version_num"], 42);
        assert_eq!(json["set_hash"], "cheap:content");

        // A new field with a `default` (D.8.3): a payload from an older daemon,
        // with no `already_landed`, still deserialises. The append-only
        // discipline is what lets it be added without bumping the protocol.
        let legacy: AgentEvent = serde_json::from_str(
            r#"{"type":"backup_success","save_id":"s1","version_num":7,"total_bytes":10,"set_hash":null}"#,
        )
        .unwrap();
        assert_eq!(json["deliberate"], true);
        assert!(matches!(
            legacy,
            AgentEvent::BackupSuccess {
                already_landed: false,
                // Like `already_landed`: an older daemon does not send it and it
                // reads as "automatic", which is how it behaved.
                deliberate: false,
                ..
            }
        ));

        let scheduled = AgentEvent::BackupScheduled {
            save_id: "s1".into(),
            delay_ms: 5000,
            reason: BackupReason::FilesystemSettled,
        };
        let json = serde_json::to_value(&scheduled).unwrap();
        assert_eq!(json["type"], "backup_scheduled");
        assert_eq!(json["reason"], "filesystem_settled");

        let deferred: AgentEvent = serde_json::from_str(
            r#"{"type":"restore_deferred","save_id":"s1","game_slug":"factorio","reason":"game is running"}"#,
        )
        .unwrap();
        assert!(matches!(deferred, AgentEvent::RestoreDeferred { .. }));
    }

    /// The goodbye carries its reason, so the client can show it ("`hoard sync
    /// stop` stopped it") instead of reporting a lost connection.
    #[test]
    fn the_farewell_carries_its_reason() {
        let frame = ServerFrame::Goodbye {
            reason: "stopped on request".into(),
        };
        let json = serde_json::to_value(&frame).unwrap();
        assert_eq!(json["frame"], "goodbye");
        assert_eq!(json["reason"], "stopped on request");
    }

    /// A frame from a newer daemon is ignored rather than breaking the framing,
    /// and with it the connection. That is what makes adding frames, like the
    /// goodbye, a compatible change.
    #[test]
    fn an_unknown_frame_degrades_instead_of_breaking_the_connection() {
        let frame: ServerFrame =
            serde_json::from_str(r#"{"frame":"invented_in_2027","payload":{"a":1}}"#).unwrap();
        assert!(matches!(frame, ServerFrame::Unknown));
    }

    /// The handshake states its own version when rejecting, so the client can
    /// tell the user what to update.
    #[test]
    fn a_rejection_carries_the_daemon_version() {
        let frame = ServerFrame::Rejected(Rejected {
            reason: "protocol 2 not supported".into(),
            daemon_protocol: PROTOCOL_VERSION,
            daemon_version: "7.7.16".into(),
        });
        let json = serde_json::to_value(&frame).unwrap();
        assert_eq!(json["frame"], "rejected");
        assert_eq!(json["daemon_protocol"], PROTOCOL_VERSION);
    }

    /// Every request's wire name is contract: the daemon dispatches on `op`, so
    /// renaming a variant breaks an already-installed client without the
    /// handshake noticing (the version only goes up on an incompatible change,
    /// and adding variants is not one).
    #[test]
    fn request_op_names_are_frozen() {
        let cases: Vec<(Request, &str)> = vec![
            (Request::Ping, "ping"),
            (Request::Status, "status"),
            (Request::Subscribe { since: Some(7) }, "subscribe"),
            (Request::Reload, "reload"),
            (
                Request::SetProbeCandidates {
                    dirs: vec!["/tmp/x".into()],
                },
                "set_probe_candidates",
            ),
            (Request::RestartEngine, "restart_engine"),
            (Request::CloudToken { rejected: None }, "cloud_token"),
            (
                Request::AdoptSession {
                    session: AdoptedSession {
                        server_url: "https://api.hoard.services".into(),
                        access_token: "jwt".into(),
                        refresh_token: "r0".into(),
                    },
                },
                "adopt_session",
            ),
            (Request::ForgetSession, "forget_session"),
            (
                Request::AdoptServerSession {
                    session: ServerSession {
                        server_url: "https://hoard.example".into(),
                        token: "hoard_v1_dead".into(),
                        user: None,
                    },
                },
                "adopt_server_session",
            ),
            (Request::ForgetServerSession, "forget_server_session"),
            (Request::ServerToken, "server_token"),
            (Request::Shutdown, "shutdown"),
        ];
        for (request, op) in cases {
            let json = serde_json::to_value(&request).unwrap();
            assert_eq!(json["op"], op, "wire name changed for {request:?}");
        }
    }

    /// Older desktops send `force_restore` without `version_num`. New daemon
    /// must still accept that: a missing field is "tick only", not a handshake
    /// break.
    #[test]
    fn force_restore_version_num_defaults_when_absent() {
        let v: Request = serde_json::from_str(r#"{"op":"force_restore","save_id":"abc"}"#).unwrap();
        match v {
            Request::ForceRestore {
                save_id,
                version_num,
            } => {
                assert_eq!(save_id, "abc");
                assert_eq!(version_num, None);
            }
            other => panic!("{other:?}"),
        }
    }

    /// The handed-over session travels whole on the wire, since otherwise the
    /// daemon could not store it, but never through the logs: the hand-written
    /// `Debug` exists for exactly that. A `?request` in one of the daemon's
    /// `tracing::` calls cannot end up publishing the refresh token to the system
    /// journal.
    #[test]
    fn an_adopted_session_travels_whole_but_never_prints() {
        let session = AdoptedSession {
            server_url: "https://api.hoard.services".into(),
            access_token: "the-jwt".into(),
            refresh_token: "the-refresh".into(),
        };
        let wire = serde_json::to_string(&Request::AdoptSession {
            session: session.clone(),
        })
        .unwrap();
        assert!(wire.contains("the-jwt") && wire.contains("the-refresh"));

        let printed = format!("{session:?}");
        assert!(!printed.contains("the-jwt"), "{printed}");
        assert!(!printed.contains("the-refresh"), "{printed}");
        // The server is worth seeing: it is what tells a dev login from a
        // production one when something does not add up.
        assert!(printed.contains("api.hoard.services"), "{printed}");
    }

    /// And the same for the self-hosted session, which travels both ways: whole
    /// on the wire, never in the log. The risk is higher here than on Cloud,
    /// because a `hoard_v1_` token does not expire, so one leaked into the
    /// journal is good forever until somebody revokes it.
    #[test]
    fn a_server_session_travels_whole_but_never_prints() {
        let session = ServerSession {
            server_url: "https://hoard.example".into(),
            token: "hoard_v1_secret".into(),
            user: Some(ServerUser {
                user_id: "u1".into(),
                username: "rai".into(),
                is_admin: true,
            }),
        };
        let wire = serde_json::to_string(&Payload::ServerSession(session.clone())).unwrap();
        assert!(wire.contains("hoard_v1_secret"));

        let printed = format!("{session:?}");
        assert!(!printed.contains("hoard_v1_secret"), "{printed}");
        assert!(printed.contains("hoard.example"), "{printed}");
        // The user is not a secret, and is exactly what makes the log useful.
        assert!(printed.contains("rai"), "{printed}");
    }

    /// New fields with a `default`: an older daemon that emits neither `server`
    /// nor `since` still deserialises in a new client. And the other way round,
    /// the field 4d deleted (`blocked_by_pid`, the pidfile) arrives as a leftover
    /// from an older daemon and is ignored instead of breaking the connection.
    #[test]
    fn older_payloads_still_deserialize() {
        let engine: EngineStatus = serde_json::from_str(r#"{"running":true}"#).unwrap();
        assert!(engine.running);
        assert!(engine.server.is_none());
        assert!(engine.since.is_none());

        let legacy: EngineStatus =
            serde_json::from_str(r#"{"running":false,"blocked_by_pid":4242}"#).unwrap();
        assert!(!legacy.running);
    }

    /// A daemon older than native notifications does not send the field, and the
    /// `false` that gets assumed is the safe side: the frontend keeps notifying.
    /// The other way round (new flag, old client) the field is surplus and gets
    /// ignored. An inverted default would leave the user with no notice at all
    /// while versions coexist.
    #[test]
    fn a_daemon_that_doesnt_notify_reads_as_not_notifying() {
        let old: DaemonStatus = serde_json::from_str(
            r#"{"daemon_version":"7.7.17","protocol":1,"pid":1,"epoch":"e",
                "uptime_secs":0,"cursor":0,"engine":{"running":false},"slots":[]}"#,
        )
        .unwrap();
        assert!(!old.notifications);
    }

    /// The engine's typed down-reason is append-only: an older daemon does not
    /// send it and the client reads `Unknown` (the generic banner, as before)
    /// instead of failing the parse of the whole status, which would leave the
    /// client with no data at all about the daemon over an informational field.
    #[test]
    fn an_engine_without_a_reason_reads_as_unknown() {
        let old: DaemonStatus = serde_json::from_str(
            r#"{"daemon_version":"1.1.0","protocol":1,"pid":1,"epoch":"e",
                "uptime_secs":0,"cursor":0,
                "engine":{"running":false,"last_error":"no session"},"slots":[]}"#,
        )
        .unwrap();
        assert_eq!(old.engine.reason, EngineDownReason::Unknown);
        assert_eq!(old.engine.last_error.as_deref(), Some("no session"));
    }

    /// The keyring fault is append-only in the way that matters: a service too
    /// old to classify it doesn't send the field, and the client reads `None` and
    /// shows the general keyring sentence, exactly what it showed before the
    /// field existed. This is the reason it is a field and not two more
    /// `EngineDownReason` variants: an unknown *variant* fails the parse of the
    /// whole status and leaves the client with no data about the daemon at all.
    #[test]
    fn an_engine_without_a_keyring_fault_reads_as_none() {
        let old: DaemonStatus = serde_json::from_str(
            r#"{"daemon_version":"1.1.4","protocol":1,"pid":1,"epoch":"e",
                "uptime_secs":0,"cursor":0,
                "engine":{"running":false,"reason":"keyring_unreadable"},"slots":[]}"#,
        )
        .unwrap();
        assert_eq!(old.engine.reason, EngineDownReason::KeyringUnreadable);
        assert_eq!(old.engine.keyring, None);
    }

    /// And the same protection the other way round: a fault a newer service
    /// classifies and this build has no name for reads as `Unknown` instead of
    /// dropping the status. Without `#[serde(other)]` every future variant would
    /// be a client that goes blind against a newer daemon.
    #[test]
    fn an_unknown_keyring_fault_doesnt_sink_the_status() {
        let newer: DaemonStatus = serde_json::from_str(
            r#"{"daemon_version":"9.9.9","protocol":1,"pid":1,"epoch":"e",
                "uptime_secs":0,"cursor":0,
                "engine":{"running":false,"reason":"keyring_unreadable",
                          "keyring":"eaten_by_a_grue"},"slots":[]}"#,
        )
        .unwrap();
        assert_eq!(newer.engine.keyring, Some(KeyringFault::Unknown));

        let json = serde_json::to_value(&EngineStatus {
            running: false,
            reason: EngineDownReason::KeyringUnreadable,
            keyring: Some(KeyringFault::Missing),
            ..EngineStatus::default()
        })
        .unwrap();
        assert_eq!(json["keyring"], "missing");
    }

    /// And on the wire it goes as `snake_case`, which is what the UI compares.
    #[test]
    fn the_engine_reason_travels_in_snake_case() {
        let json = serde_json::to_string(&EngineStatus {
            running: false,
            reason: EngineDownReason::KeyringUnreadable,
            ..EngineStatus::default()
        })
        .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["reason"], "keyring_unreadable");
    }

    /// The token loan: `rejected` is optional on the wire, since a client that
    /// only wants a valid one sends nothing, and the answer carries the expiry so
    /// the client does not have to decode the JWT.
    #[test]
    fn the_cloud_token_loan_round_trips() {
        let asked: Request = serde_json::from_str(r#"{"op":"cloud_token"}"#).unwrap();
        assert!(matches!(asked, Request::CloudToken { rejected: None }));

        let lent = Payload::CloudToken(CloudToken {
            access_token: "jwt".into(),
            server_url: "https://api.hoard.services".into(),
            expires_at: Some(1_800_000_000),
            rotated: true,
        });
        let json = serde_json::to_value(&lent).unwrap();
        assert_eq!(json["payload"], "cloud_token");
        assert_eq!(json["access_token"], "jwt");
        assert_eq!(json["expires_at"], 1_800_000_000i64);

        // A daemon that does not know the expiry is still a valid answer.
        let minimal: CloudToken =
            serde_json::from_str(r#"{"access_token":"jwt","server_url":"u"}"#).unwrap();
        assert!(minimal.expires_at.is_none());
        assert!(!minimal.rotated);
    }

    /// "The Cloud session is over" gets its own variant because the client acts
    /// differently than on a transient failure: it logs out instead of retrying.
    /// Its wire name is contract.
    #[test]
    fn a_dead_cloud_session_is_its_own_error() {
        let err = IpcError::CloudSessionExpired {
            reason: "the refresh token family was revoked".into(),
        };
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["error"], "cloud_session_expired");
        let back: IpcError = serde_json::from_value(json).unwrap();
        assert!(matches!(back, IpcError::CloudSessionExpired { .. }));
        // It reaches the user readable (toast, stdout), not as `{:?}`.
        assert!(back.to_string().contains("revoked"));
    }
}
