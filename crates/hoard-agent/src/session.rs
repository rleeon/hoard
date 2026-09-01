//! The active session: which server, with which token, and who renews it.
//!
//! One implementation shared by the service (`hoardd`) and the CLI (ADR 0021,
//! Slice 4c). There used to be two copies, the CLI's and the port made into
//! `hoardd/src/session.rs` so as not to touch it. With the CLI turned into a
//! client, that duplication has no excuse and it lives here, where `CLAUDE.md`'s
//! rule applies: the logic goes in `hoard-agent` and the frontends are views.
//!
//! ## Two paths, and only one rotates
//!
//! - [`resolve_owned`], the service. It resolves the credentials, refreshes the
//!   boot JWT and keeps the rotated pair. It comes with [`refresh_loop`], which
//!   renews before it expires, and [`lend_token`], which lends a valid token to
//!   whoever asks over IPC.
//! - [`resolve_borrowed`], a client (the CLI on a one-shot). It never calls
//!   GoTrue: it uses the token the service lends it and, with no service, whatever
//!   is on disk as-is.
//!
//! Two processes being able to rotate the same `cloud.toml` refresh token is the
//! root cause of a whole family of cloud bugs (401s from reuse detection, a mute
//! realtime); the pidfile avoided it by exclusion rather than by design. Here the
//! separation is in the types: the only thing that rotates is whatever calls
//! [`resolve_owned`], [`refresh_loop`] and [`lend_token`], and that is the daemon.

use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use hoard_core::ipc::CloudToken;

use crate::api::ApiClient;
use crate::cloud_auth;
use crate::config::CliConfig;
use crate::credentials::{self, Credentials};
use crate::state;
use crate::supervisor::Finished;

/// The refresher's normal cadence: renew the JWT (about an hour of life) with
/// room to spare.
const REFRESH_EVERY: Duration = Duration::from_secs(45 * 60);

/// The cadence with a dead session. It only re-reads the disk waiting for a fresh
/// login, so checking often costs nothing, and it touches no network: repeating a
/// revoked token every few minutes is what filled the system journal with the same
/// WARN for days.
const RELOGIN_RECHECK_EVERY: Duration = Duration::from_secs(5 * 60);

/// How long the boot keeps insisting on a *transient* failure of the initial
/// refresh before starting with the stored token.
const BOOT_REFRESH_GRACE: Duration = Duration::from_secs(60);

/// How long the boot waits for a self-hosted server to answer `/v1/health`.
///
/// The service starts at boot, and a self-hosted server on the same box may still
/// be coming up: without this wait the first auto-restore fails with "connection
/// refused" for every save. Bounded, so we do not hang when there simply is no
/// server (the engine retries per save anyway).
const SERVER_WAIT: Duration = Duration::from_secs(60);

/// The minimum life a token has to have left to be lent without rotating.
///
/// It is load-bearing that this be greater than the margin clients use to decide
/// "this token is about to expire" (120 s in the desktop's realtime): if it were
/// smaller, the client would ask, receive the same token, and ask again every few
/// seconds until it crossed our threshold.
const LEND_MIN_TTL: i64 = 5 * 60;

/// The resolved active session.
pub struct Active {
    pub client: ApiClient,
    pub is_cloud: bool,
    /// Human-readable description of the target (banners, logs, the IPC `Status`).
    pub server: String,
    /// Cloud credentials for the REST calls that go outside the `ApiClient`
    /// (`hoard cloud`, the refresher). `None` on self-hosted.
    pub cloud: Option<CloudEndpoint>,
}

/// Which Cloud and with which token. The refresh token only travels here when the
/// resolver is the owner ([`resolve_owned`]): a client does not rotate, so it does
/// not receive one, and without it it cannot rotate even by accident. The "single
/// rotator" rule is therefore a property of the types rather than a comment.
pub struct CloudEndpoint {
    pub server_url: String,
    pub access: String,
    pub refresh: Option<String>,
}

impl CloudEndpoint {
    /// The full session, which is what [`refresh_loop`] needs. `None` for a
    /// borrowed endpoint.
    pub fn owned(&self) -> Option<cloud_auth::Session> {
        Some(cloud_auth::Session {
            server_url: self.server_url.clone(),
            access: self.access.clone(),
            refresh: self.refresh.clone()?,
        })
    }
}

// ---- el servicio: resolver rotando ------------------------------------

/// Resolves the token owner's session: Cloud when there is one, otherwise
/// self-hosted through the config's token. It sets the sync context
/// (`state::set_active_context`) before anybody reads `state.json`; without that
/// the daemon would load another account's save map.
///
/// A transient failure of the initial refresh retries for [`BOOT_REFRESH_GRACE`]
/// and then starts with whatever is on disk. The service starts at boot, routinely
/// before DNS answers; dying there leaves the machine syncing nothing until the
/// next attempt, and the refresher repairs the token as soon as there is network.
/// A terminally expired session is fatal, though: only a fresh login fixes it and
/// waiting does not help.
///
/// Only the service calls this. It is the path that rotates.
pub async fn resolve_owned() -> Result<Active> {
    // `load_session_async` rather than `load_session`: reading the keyring is
    // synchronous and this path runs on the keeper's task, the one shutdown
    // aborts. With the read on the task's thread, a locked keyring left the engine
    // in `starting` and the daemon unable to stop (D.19).
    if let Some(sess) = cloud_auth::load_session_async().await? {
        return resolve_cloud_owned(sess).await;
    }
    let active = selfhosted_owned().await?;
    // Cloud is always up; waiting only makes sense with your own server.
    wait_for_server(&active).await;
    Ok(active)
}

async fn resolve_cloud_owned(sess: cloud_auth::Session) -> Result<Active> {
    let refreshed = initial_refresh().await?;
    let degraded = refreshed.is_none();
    let (access, refresh) = match refreshed {
        Some(t) => (t.access, t.refresh),
        None => (sess.access.clone(), sess.refresh.clone()),
    };

    let client = ApiClient::new(sess.server_url.clone(), access.clone())?;
    let cloud = Some(CloudEndpoint {
        server_url: sess.server_url.clone(),
        access: access.clone(),
        refresh: Some(refresh),
    });
    lend_to_logship(&sess.server_url, &access);

    match cloud_auth::fetch_me(&sess.server_url, &access).await {
        Ok(me) => {
            state::set_active_context(Some(state::cloud_context(&me.user_id)));
            Ok(Active {
                client,
                is_cloud: true,
                server: format!("Cloud · {} ({})", me.email, me.plan),
                cloud,
            })
        }
        // The same network cut that sank the refresh. The context is set from the
        // stored JWT's `sub`, with no network, and it is the same id `/v1/me`
        // would have given. If it cannot be read we abort: running under the wrong
        // context would sync another account's save map.
        Err(err) if degraded => {
            let user_id = cloud_auth::session_user_id()?.context(
                "Cloud is unreachable and the stored session is unreadable: run `hoard login`",
            )?;
            state::set_active_context(Some(state::cloud_context(&user_id)));
            tracing::warn!(error = %err, "session: Cloud unreachable; starting on the stored session");
            Ok(Active {
                client,
                is_cloud: true,
                server: format!("Cloud · {} (unverified)", sess.server_url),
                cloud,
            })
        }
        Err(err) => Err(err),
    }
}

/// The boot refresh, retried inside the grace window. `Ok(None)` means the grace
/// ran out on a transient failure, so it starts with the stored token.
async fn initial_refresh() -> Result<Option<cloud_auth::Tokens>> {
    let deadline = Instant::now() + BOOT_REFRESH_GRACE;
    let mut backoff = Duration::from_secs(2);
    loop {
        match cloud_auth::refresh_freshest().await {
            Ok(tokens) => return Ok(Some(tokens)),
            // Reuse detection with nothing to adopt: retrying does not fix it.
            Err(err) if cloud_auth::is_session_expired(&err) => return Err(err),
            Err(err) => {
                if Instant::now() >= deadline {
                    tracing::warn!(error = %err, "session: couldn't renew the Cloud session at boot");
                    return Ok(None);
                }
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(Duration::from_secs(10));
            }
        }
    }
}

/// Probes `/v1/health` until the server answers, with a cap. If it runs out, it
/// warns and carries on: the engine will try anyway.
async fn wait_for_server(active: &Active) {
    let deadline = Instant::now() + SERVER_WAIT;
    let mut announced = false;
    loop {
        if active.client.health().await.is_ok() {
            if announced {
                tracing::info!(server = %active.server, "session: the server is up");
            }
            return;
        }
        if Instant::now() >= deadline {
            tracing::warn!(
                server = %active.server,
                secs = SERVER_WAIT.as_secs(),
                "session: the server is still unreachable; continuing anyway"
            );
            return;
        }
        if !announced {
            tracing::info!(server = %active.server, "session: waiting for the server to come online");
            announced = true;
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

// ---- a client: resolving with a borrowed token

/// Resolves a client's session: the same as [`resolve_owned`] as far as which
/// server and which context, but without rotating anything.
///
/// `lent` is the Cloud token the service has lent (`Request::CloudToken`) and
/// `lent_server` the self-hosted session (`Request::ServerToken`); `None` in
/// either means there was no service to ask, so Cloud uses the on-disk token as-is
/// and self-hosted falls back to `config.toml`. It may be expired, since nobody
/// has renewed it precisely because the rotator is the service, and then the call
/// fails with a readable 401; [`stale_token_hint`] is the hint the CLI shows in
/// that case. Degrading this way is deliberate: the alternative was a `hoard
/// whoami` starting the sync service as a side effect.
pub async fn resolve_borrowed(
    lent: Option<CloudToken>,
    lent_server: Option<hoard_core::ipc::ServerSession>,
) -> Result<Active> {
    if let Some(sess) = cloud_auth::load_session()? {
        let (server_url, access) = match lent {
            Some(token) => (token.server_url, token.access_token),
            None => (sess.server_url.clone(), sess.access.clone()),
        };
        let client = ApiClient::new(server_url.clone(), access.clone())?;
        // `/v1/me` is still the check that the token is valid and whose it is;
        // what is gone is the refresh beforehand. If it fails, the context is set from the JWT's `sub` anyway:
        // a command that aborts must not leave another account's context behind.
        match cloud_auth::fetch_me(&server_url, &access).await {
            Ok(me) => {
                state::set_active_context(Some(state::cloud_context(&me.user_id)));
                Ok(Active {
                    client,
                    is_cloud: true,
                    server: format!("Cloud · {} ({})", me.email, me.plan),
                    cloud: Some(CloudEndpoint {
                        server_url,
                        access,
                        // Borrowed: no refresh token, because a client does not
                        // rotate.
                        refresh: None,
                    }),
                })
            }
            Err(err) => {
                let user_id = cloud_auth::session_user_id()?
                    .context("the stored Cloud session is unreadable: run `hoard login`")?;
                state::set_active_context(Some(state::cloud_context(&user_id)));
                Err(err)
            }
        }
    } else {
        selfhosted_borrowed(lent_server)
    }
}

/// The owner's self-hosted session (no Cloud and no tokens to rotate): from the
/// session store and, failing that, from `config.toml`.
///
/// The order is D.20's fix, and it was a big bug: until now this read only
/// `config.toml`, which only `hoard login --token` writes. The app stores its
/// session in `credentials` (keyring plus `session.toml`), so anybody who signed
/// into their server through the app alone had an engine that resolved no session
/// at all: "no session, sign in with `hoard login`" in `last_error`, zero syncing,
/// and a UI meanwhile saying "connected". Two disjoint stores and no bridge.
///
/// [`credentials`] wins because it is the session the user sees in the app and the
/// one every fresh login touches (the app's, and the CLI's, which hands it over
/// too). `config.toml` stays the headless path it always was, plain text with no
/// keyring, the one the self-hosting guide documents, and serves as the fallback
/// for installs that already had it.
///
/// It is `async` because the keyring is synchronous and blocks the thread while it
/// waits: on the keeper's task, the one shutdown aborts, that is half of D.19's
/// failure, so the read goes to the blocking pool just as Cloud's does.
async fn selfhosted_owned() -> Result<Active> {
    let stored = tokio::task::spawn_blocking(credentials::load_detailed)
        .await
        .map_err(|join| anyhow::Error::new(join).context("reading the self-hosted session"))??;

    // The token came from the 0600 file: either a client with no service left it
    // there, or the keyring was mute when it was stored. Lifting it NOW, from the
    // daemon, is what gives it ownership of the item, and on macOS ownership is
    // the difference between reading it quietly and a password dialog on every
    // engine start. Best-effort and on the blocking pool, like every keyring write.
    if let Some((creds, credentials::TokenStorage::File)) = &stored {
        let creds = creds.clone();
        let _ = tokio::task::spawn_blocking(move || credentials::promote_to_keyring(&creds)).await;
    }

    let creds = pick_selfhosted(stored.map(|(creds, _)| creds), config_session()?)?;
    state::set_active_context(Some(state::selfhosted_context(&creds.url)));
    let client = ApiClient::new(creds.url.clone(), creds.token)?;
    Ok(Active {
        client,
        is_cloud: false,
        server: creds.url,
        cloud: None,
    })
}

/// The precedence, and nothing else. Pure and tested because this `or` IS the bug
/// that broke self-hosted in 1.1.0: the order lived implicitly in an `if` that
/// only looked at `config.toml`, nothing pinned it, and breaking it turned nothing
/// red. A test compiling the order into the suite is what stops it coming back
/// without CI noticing.
fn pick_selfhosted(
    stored: Option<Credentials>,
    from_config: Option<Credentials>,
) -> Result<Credentials> {
    stored
        .or(from_config)
        .ok_or_else(|| anyhow::Error::new(NoSession))
}

/// The session from `config.toml`, if there is one. `None` is not an error: it is
/// the normal case for somebody who has never used the CLI.
fn config_session() -> Result<Option<Credentials>> {
    let (cfg, _) = CliConfig::load_default()?;
    Ok(cfg
        .auth
        .token
        .filter(|t| !t.is_empty())
        .map(|token| Credentials {
            url: cfg.server.url,
            token,
            user: None,
        }))
}

/// A client's self-hosted session: the one the service lends it, and with no
/// service the one in `config.toml`.
///
/// Never the keyring, and that is the point: the item belongs to the daemon, so a
/// client reading it would ask the user for their password again on macOS (D.20).
/// With no service it degrades to `config.toml`, the headless path it always was,
/// and somebody who signed in through the app alone sees "no session" until the
/// service is up, since the service is what holds their session.
fn selfhosted_borrowed(lent: Option<hoard_core::ipc::ServerSession>) -> Result<Active> {
    if let Some(lent) = lent {
        state::set_active_context(Some(state::selfhosted_context(&lent.server_url)));
        let client = ApiClient::new(lent.server_url.clone(), lent.token)?;
        return Ok(Active {
            client,
            is_cloud: false,
            server: lent.server_url,
            cloud: None,
        });
    }
    selfhosted_from_config()
}

/// `config.toml`: the headless path, plain text with no keyring. Written by
/// `hoard login --token` and documented by the self-hosting guide.
fn selfhosted_from_config() -> Result<Active> {
    let creds = config_session()?.ok_or_else(|| anyhow::Error::new(NoSession))?;
    state::set_active_context(Some(state::selfhosted_context(&creds.url)));
    let client = ApiClient::new(creds.url.clone(), creds.token)?;
    Ok(Active {
        client,
        is_cloud: false,
        server: creds.url,
        cloud: None,
    })
}

/// There is no session to use on this machine.
///
/// Its own type rather than an `anyhow!("no session")` because the daemon
/// classifies it by downcast so the window can say *this* instead of "the service
/// is offline", the generic banner that cost two support threads in July 2026,
/// with two users who had no way of knowing they were missing a session. The text
/// stays the same as before: it is what shows up in `last_error` and in the
/// service's log.
#[derive(Debug)]
pub struct NoSession;

impl std::fmt::Display for NoSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            "no session. Sign in with `hoard login` (Cloud) or \
             `hoard login --token <token>` (self-host).",
        )
    }
}

impl std::error::Error for NoSession {}

/// The self-hosted token the daemon lends a client (`Request::ServerToken`).
/// `None` means there is no self-hosted session on this machine.
///
/// It serves the two sources [`selfhosted`] resolves, in the same order, so what
/// the client uses and what the engine uses cannot diverge.
pub fn lend_server_session() -> Result<Option<hoard_core::ipc::ServerSession>> {
    if let Some(creds) = credentials::load()? {
        return Ok(Some(hoard_core::ipc::ServerSession {
            server_url: creds.url,
            token: creds.token,
            user: creds.user.map(|u| hoard_core::ipc::ServerUser {
                user_id: u.user_id,
                username: u.username,
                is_admin: u.is_admin,
            }),
        }));
    }
    let (cfg, _) = CliConfig::load_default()?;
    Ok(cfg
        .auth
        .token
        .filter(|t| !t.is_empty())
        .map(|token| hoard_core::ipc::ServerSession {
            server_url: cfg.server.url.clone(),
            token,
            // `config.toml` does not cache the whoami: a client that needs it asks
            // the server, which is where it used to come from.
            user: None,
        }))
}

/// Sets the sync context with no network: Cloud through the stored JWT's `sub`,
/// otherwise self-hosted through the config's URL. For local commands (`hoard
/// saves`) that have to work offline. Best-effort: if it cannot, it leaves the
/// default context.
pub fn set_context_offline() {
    if let Ok(Some(user_id)) = cloud_auth::session_user_id() {
        state::set_active_context(Some(state::cloud_context(&user_id)));
        return;
    }
    if let Ok((cfg, _)) = CliConfig::load_default() {
        state::set_active_context(Some(state::selfhosted_context(&cfg.server.url)));
    }
}

/// A hint for the user when a client has no service and the on-disk token is no
/// longer good: the fix is not signing in again, it is bringing the service up,
/// since that is what renews. `None` when the token is still usable.
pub fn stale_token_hint(access: &str, now_unix: i64) -> Option<&'static str> {
    let expired = match cloud_auth::jwt_expiry(access) {
        Some(exp) => exp <= now_unix,
        // Unreadable: we claim nothing. If it really is no good, the 401 will say.
        None => false,
    };
    expired.then_some(
        "the Cloud session token has expired and the Hoard service (the only thing that \
         renews it) isn't running. Start it with `hoard sync start`.",
    )
}

// ---- el servicio: prestar el token ------------------------------------

/// Why a token could not be lent.
#[derive(Debug, thiserror::Error)]
pub enum LendError {
    /// There is nothing to lend and rotating would not fix it: no session on disk,
    /// or GoTrue revoked the family. Only a fresh login.
    #[error("{0}")]
    Gone(String),
    /// A network bump or a grumpy GoTrue: the token is still alive and retrying
    /// makes sense. It must NOT make a client sign out.
    #[error(transparent)]
    Transient(anyhow::Error),
}

/// Lends a valid Cloud token, rotating only when it has to. It answers
/// `Request::CloudToken`, and only the service calls it, which is what makes it
/// the single rotator.
///
/// `rejected` is the token that came back 401 for the client. See
/// [`needs_rotation`] for the decision, which is pure and tested.
pub async fn lend_token(rejected: Option<&str>) -> Result<CloudToken, LendError> {
    let session = cloud_auth::load_session()
        .map_err(LendError::Transient)?
        .ok_or_else(|| LendError::Gone("no Cloud session on this machine".to_string()))?;

    let now = now_unix();
    let ttl = cloud_auth::jwt_expiry(&session.access).map(|exp| exp - now);
    if !needs_rotation(ttl, &session.access, rejected) {
        return Ok(CloudToken {
            expires_at: ttl.map(|t| now + t),
            access_token: session.access,
            server_url: session.server_url,
            rotated: false,
        });
    }

    match cloud_auth::refresh_freshest().await {
        Ok(tokens) => Ok(CloudToken {
            expires_at: cloud_auth::jwt_expiry(&tokens.access),
            access_token: tokens.access,
            server_url: session.server_url,
            rotated: true,
        }),
        Err(err) if cloud_auth::is_session_expired(&err) => {
            Err(LendError::Gone(format!("{err:#}")))
        }
        Err(err) => Err(LendError::Transient(err)),
    }
}

/// Does it have to rotate before lending? Pure, so the policy is a test rather
/// than a comment.
///
/// - A token the client already ate a 401 with is not handed back: that would be a
///   retry loop with the same dead token. If the one we hold is already a different
///   one, somebody rotated for us and that one serves.
/// - Below [`LEND_MIN_TTL`] it rotates: lending a token that expires in seconds is
///   lending a 401.
/// - An unreadable expiry means rotate. We do not know what it has left, and
///   `refresh_freshest`'s reuse window collapses the burst if several ask at once.
fn needs_rotation(ttl_secs: Option<i64>, stored: &str, rejected: Option<&str>) -> bool {
    if rejected.is_some_and(|r| r == stored) {
        return true;
    }
    match ttl_secs {
        Some(ttl) => ttl < LEND_MIN_TTL,
        None => true,
    }
}

fn now_unix() -> i64 {
    time::OffsetDateTime::now_utc().unix_timestamp()
}

// ---- the service: background refresher

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// A live session: renew at the normal cadence.
    Normal,
    /// GoTrue revoked the token family. There is nothing to renew until somebody
    /// signs in again, so only the disk gets watched.
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Renewed,
    /// A network or GoTrue bump: the token is still good, retry later.
    Transient,
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Announce {
    Nothing,
    Expired,
    Restored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Step {
    phase: Phase,
    sleep: Duration,
    announce: Announce,
}

/// The refresher's cadence as a pure function: "say it once then go quiet" is
/// checkable without a Cloud session and without waiting 45 minutes.
fn next_step(phase: Phase, outcome: Outcome) -> Step {
    match (phase, outcome) {
        (Phase::Normal, Outcome::Expired) => Step {
            phase: Phase::Expired,
            sleep: RELOGIN_RECHECK_EVERY,
            announce: Announce::Expired,
        },
        (Phase::Expired, Outcome::Renewed) => Step {
            phase: Phase::Normal,
            sleep: REFRESH_EVERY,
            announce: Announce::Restored,
        },
        (Phase::Expired, _) => Step {
            phase: Phase::Expired,
            sleep: RELOGIN_RECHECK_EVERY,
            announce: Announce::Nothing,
        },
        (Phase::Normal, _) => Step {
            phase: Phase::Normal,
            sleep: REFRESH_EVERY,
            announce: Announce::Nothing,
        },
    }
}

/// Mete el par renovado en el cliente vivo y en nuestra copia.
fn adopt(client: &ApiClient, sess: &mut cloud_auth::Session, tokens: cloud_auth::Tokens) {
    client.set_token(&tokens.access);
    sess.access = tokens.access;
    sess.refresh = tokens.refresh;
    // The log shipper cannot ask for anything over IPC and does not see
    // `cloud.toml`: it gets the freshly rotated token put in place, or it would
    // keep shipping with the old one until it ate a 401.
    lend_to_logship(&sess.server_url, &sess.access);
}

/// Puts the Cloud session in the slot `logship` reads. Called by the owner (the
/// service) as soon as it has a valid JWT: on start and on every rotation.
fn lend_to_logship(url: &str, token: &str) {
    credentials::set_lent_cloud(Some(credentials::CloudLease {
        url: url.to_string(),
        token: token.to_string(),
    }));
}

/// A session on disk whose refresh token is not the dead one, meaning the user
/// signed in again (here or on the desktop: they share the session file).
fn relogin_tokens(dead: Option<&str>) -> Option<cloud_auth::Tokens> {
    let s = cloud_auth::load_session().ok().flatten()?;
    if s.refresh.trim().is_empty() || Some(s.refresh.as_str()) == dead {
        return None;
    }
    Some(cloud_auth::Tokens {
        access: s.access,
        refresh: s.refresh,
    })
}

/// The loop that renews the JWT before it expires and pushes it into the live
/// `ApiClient`. Without it the engine starts returning 401s an hour after boot.
///
/// The session travels in an `Arc<Mutex<...>>` so the loop can restart under
/// `supervise` without losing the already-rotated tokens: restarting after a panic
/// and going back to the pair on disk would reintroduce the reuse detection this
/// module exists to kill.
///
/// Only the service. It is the other half of the single rotator.
pub async fn refresh_loop(
    client: ApiClient,
    session: Arc<tokio::sync::Mutex<cloud_auth::Session>>,
) -> Finished {
    let mut phase = Phase::Normal;
    let mut sleep_for = REFRESH_EVERY;
    // The refresh token GoTrue declared dead, to tell a fresh login from the same
    // dead session still sitting on disk.
    let mut dead: Option<String> = None;

    loop {
        tokio::time::sleep(sleep_for).await;

        let outcome = match phase {
            Phase::Normal => match cloud_auth::refresh_freshest().await {
                Ok(tokens) => {
                    adopt(&client, &mut *session.lock().await, tokens);
                    Outcome::Renewed
                }
                Err(err) if cloud_auth::is_session_expired(&err) => {
                    let ours = session.lock().await.refresh.clone();
                    dead = cloud_auth::load_session()
                        .ok()
                        .flatten()
                        .map(|s| s.refresh)
                        .or(Some(ours));
                    Outcome::Expired
                }
                Err(err) => {
                    tracing::warn!(error = %err, "session: periodic Cloud refresh failed");
                    Outcome::Transient
                }
            },
            // No network on purpose: repeating a revoked token every few minutes
            // is what filled the system log with the same WARN for days. Only a
            // fresh login helps, so only the disk gets watched.
            Phase::Expired => match relogin_tokens(dead.as_deref()) {
                Some(tokens) => {
                    dead = None;
                    adopt(&client, &mut *session.lock().await, tokens);
                    Outcome::Renewed
                }
                None => Outcome::Expired,
            },
        };

        let step = next_step(phase, outcome);
        match step.announce {
            Announce::Expired => {
                tracing::error!("session: the Cloud session expired, run `hoard login`")
            }
            Announce::Restored => tracing::info!("session: the Cloud session is back"),
            Announce::Nothing => {}
        }
        phase = step.phase;
        sleep_for = step.sleep;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn creds(url: &str, token: &str) -> Credentials {
        Credentials {
            url: url.to_string(),
            token: token.to_string(),
            user: None,
        }
    }

    /// The 1.1.0 bug's test. The engine resolved the self-hosted session by
    /// looking only at `config.toml`, which only `hoard login --token` writes,
    /// while the app stored its own in `credentials`. Anybody who signed into
    /// their server through the app alone had an engine with no session, zero
    /// backups, and a window that just said "the service is offline".
    ///
    /// The store winning is not a preference: it is the session the user sees in
    /// the app and the one every fresh login touches.
    #[test]
    fn the_session_store_beats_config_toml() {
        let picked = pick_selfhosted(
            Some(creds("https://saves.example", "hoard_v1_de-la-app")),
            Some(creds("http://localhost:12421", "hoard_v1_del-config")),
        )
        .expect("there is a session");
        assert_eq!(picked.token, "hoard_v1_de-la-app");
        assert_eq!(picked.url, "https://saves.example");
    }

    /// And `config.toml` is still the headless path: with no store (a machine
    /// where only the CLI has been used, which is what the self-hosting guide
    /// documents) it wins. Fixing the bug could not break this.
    #[test]
    fn config_toml_still_serves_the_headless_path() {
        let picked = pick_selfhosted(None, Some(creds("http://nas.local:12421", "hoard_v1_cli")))
            .expect("there is a session");
        assert_eq!(picked.token, "hoard_v1_cli");
    }

    /// With neither, the reason is typed: it is what the daemon classifies so the
    /// window can say "there is no session, sign in again" instead of the generic
    /// banner.
    #[test]
    fn no_session_anywhere_is_typed() {
        let err = pick_selfhosted(None, None).expect_err("there is no session");
        assert!(err.downcast_ref::<NoSession>().is_some(), "{err:#}");
        assert!(format!("{err:#}").contains("no session"), "{err:#}");
    }

    #[test]
    fn announces_the_death_once_and_then_stays_quiet() {
        let died = next_step(Phase::Normal, Outcome::Expired);
        assert_eq!(died.phase, Phase::Expired);
        assert_eq!(died.announce, Announce::Expired);
        assert_eq!(died.sleep, RELOGIN_RECHECK_EVERY);

        // Every later check with no pending login: same state, quiet.
        let again = next_step(died.phase, Outcome::Expired);
        assert_eq!(again.phase, Phase::Expired);
        assert_eq!(again.announce, Announce::Nothing);
        assert_eq!(again.sleep, RELOGIN_RECHECK_EVERY);
    }

    #[test]
    fn a_relogin_restores_the_normal_cadence() {
        let back = next_step(Phase::Expired, Outcome::Renewed);
        assert_eq!(back.phase, Phase::Normal);
        assert_eq!(back.announce, Announce::Restored);
        assert_eq!(back.sleep, REFRESH_EVERY);
    }

    #[test]
    fn a_transient_failure_neither_announces_nor_changes_phase() {
        let step = next_step(Phase::Normal, Outcome::Transient);
        assert_eq!(step.phase, Phase::Normal);
        assert_eq!(step.announce, Announce::Nothing);
        assert_eq!(step.sleep, REFRESH_EVERY);
    }

    #[test]
    fn a_transient_failure_while_expired_keeps_waiting_for_a_login() {
        let step = next_step(Phase::Expired, Outcome::Transient);
        assert_eq!(step.phase, Phase::Expired);
        assert_eq!(step.announce, Announce::Nothing);
    }

    #[test]
    fn the_happy_path_holds_the_normal_cadence() {
        let step = next_step(Phase::Normal, Outcome::Renewed);
        assert_eq!(step.phase, Phase::Normal);
        assert_eq!(step.announce, Announce::Nothing);
        assert_eq!(step.sleep, REFRESH_EVERY);
    }

    /// A token with life to spare is lent as-is: lending is not rotating, and
    /// rotating too often spends the refresh token (each rotation revokes the
    /// previous one).
    #[test]
    fn a_healthy_token_is_lent_without_rotating() {
        assert!(!needs_rotation(Some(LEND_MIN_TTL + 1), "tok", None));
        assert!(!needs_rotation(Some(3600), "tok", None));
    }

    /// The margin is what stops a 401 being lent: below it, it rotates.
    #[test]
    fn a_token_about_to_die_is_rotated_first() {
        assert!(needs_rotation(Some(LEND_MIN_TTL - 1), "tok", None));
        assert!(needs_rotation(Some(0), "tok", None));
        assert!(needs_rotation(Some(-10), "tok", None));
    }

    /// The case `rejected` exists to close: the client ate a 401 with a token that
    /// has NOT expired yet (revoked server-side, a skewed clock). Without this it
    /// would get the same token and retry in a loop.
    #[test]
    fn a_rejected_token_is_never_handed_back() {
        assert!(needs_rotation(Some(3600), "tok", Some("tok")));
    }

    /// But if the one we hold is no longer the rejected one, somebody rotated for
    /// us: serving it is free and saves a rotation.
    #[test]
    fn a_rejection_of_someone_elses_token_doesnt_force_a_rotation() {
        assert!(!needs_rotation(Some(3600), "newer", Some("older")));
    }

    /// An unreadable expiry: we do not pretend to know. Rotating is the safe
    /// direction.
    #[test]
    fn an_unreadable_expiry_rotates() {
        assert!(needs_rotation(None, "tok", None));
    }

    /// The hint only appears when the token really has expired; with a live one,
    /// or an unreadable one, we stay quiet, so the user is not sent to fix
    /// something that is not broken.
    #[test]
    fn the_stale_hint_only_fires_on_an_expired_token() {
        use base64::Engine;
        let jwt = |exp: i64| {
            let body = base64::engine::general_purpose::URL_SAFE_NO_PAD
                .encode(format!(r#"{{"exp":{exp}}}"#).as_bytes());
            format!("h.{body}.s")
        };
        assert!(stale_token_hint(&jwt(1_000), 2_000).is_some());
        assert!(stale_token_hint(&jwt(3_000), 2_000).is_none());
        assert!(stale_token_hint("opaque", 2_000).is_none());
    }
}
