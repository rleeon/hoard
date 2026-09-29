//! The CLI's link to `hoardd` (ADR 0021, part A, Slice 4c).
//!
//! There is no engine here: the CLI sends commands to the service over the local
//! socket and prints what the service reports, the same way the desktop draws the
//! same thing in a window. One frontend with a window and one without, and now in
//! the process topology too.
//!
//! ## Who starts the service, and who does not
//!
//! [`ensure`] is "connect; if there is no service, start it", the ADR's idempotent
//! handshake. Only [`super::daemon::run`] (`hoard sync run`) uses it, because that
//! is the command whose job *is* having sync running.
//!
//! Everything else uses [`attached`], which connects but starts nothing. A `hoard
//! whoami` or a `hoard save pause` cannot turn the machine into a syncing machine
//! as a side effect; the explicit way to ask for that is `hoard sync start`. The
//! trade-off is that with no service there is nobody to rotate the Cloud token,
//! and that degrades rather than breaking: see [`resolve_session`].

use std::time::Duration;

use anyhow::{Context, Result};
use hoard_agent::session::{self, Active};
use hoard_core::ipc::{CloudToken, DaemonStatus, HoldState, IpcError, Payload, Request};
use hoardd::client::Client;
use hoardd::endpoint::Endpoint;

/// How we introduce ourselves in the daemon's log.
fn client_name(role: &str) -> String {
    format!("hoard {} ({role})", env!("CARGO_PKG_VERSION"))
}

/// Cap on a request over an established connection. A service that accepts and
/// then goes quiet must not hang a terminal command forever.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

/// Cap on *connecting* when the command only wants to look (status, banner). It
/// is a local socket: if it does not answer within this, nobody is there.
const PROBE_TIMEOUT: Duration = Duration::from_secs(2);

fn endpoint() -> Result<Endpoint> {
    Endpoint::resolve().context("resolving the hoardd endpoint")
}

/// Connect to the service and start it if there is none. For `hoard sync run`.
pub async fn ensure(role: &str) -> Result<Client> {
    let endpoint = endpoint()?;
    Client::ensure_running(&endpoint, &client_name(role))
        .await
        .with_context(|| format!("connecting to the Hoard service at {endpoint}"))
}

/// Connect to the service *if it is already up*. `None` means there is no service
/// (or it is not answering), and that is not an error, it is the answer.
pub async fn attached(role: &str) -> Option<Client> {
    let endpoint = endpoint().ok()?;
    let name = client_name(role);
    match tokio::time::timeout(PROBE_TIMEOUT, Client::connect(&endpoint, &name)).await {
        Ok(Ok(client)) => Some(client),
        Ok(Err(err)) => {
            tracing::debug!(error = %format!("{err:#}"), "cli: no Hoard service listening");
            None
        }
        Err(_) => {
            tracing::warn!("cli: the Hoard service accepted nothing within {PROBE_TIMEOUT:?}");
            None
        }
    }
}

/// A capped request over an established connection.
pub async fn ask(client: &mut Client, request: Request) -> Result<Payload> {
    tokio::time::timeout(REQUEST_TIMEOUT, client.request(request))
        .await
        .map_err(|_| {
            anyhow::anyhow!(
                "the Hoard service didn't answer in {}s",
                REQUEST_TIMEOUT.as_secs()
            )
        })?
}

/// What the service knows about the update, if there is a service. Never starts
/// one: asking how an update is going cannot turn the machine into one that syncs.
///
/// `None` deliberately covers two things: there is no service, and there is one
/// *older than this binary* that does not know the request. The second lasts as
/// long as the handover takes (seconds) and does not deserve an error on screen.
pub async fn update_state() -> Option<hoard_core::ipc::UpdateState> {
    let mut client = attached("updates").await?;
    match ask(&mut client, Request::UpdateStatus).await {
        Ok(Payload::Update(state)) => Some(state),
        Ok(other) => {
            tracing::debug!("cli: unexpected answer to UpdateStatus: {other:?}");
            None
        }
        Err(err) => {
            tracing::debug!(error = %format!("{err:#}"), "cli: the service didn't report update status");
            None
        }
    }
}

/// Asks the service to apply whatever it has downloaded, now. Returns the state
/// at that moment; applying carries on after the answer.
pub async fn apply_update(version: Option<String>) -> Result<hoard_core::ipc::UpdateState> {
    let mut client = attached("upgrade")
        .await
        .context("the Hoard service isn't running, so there's nobody to apply the update")?;
    match ask(&mut client, Request::ApplyUpdate { version }).await? {
        Payload::Update(state) => Ok(state),
        other => anyhow::bail!("the service answered a {other:?} to an update request"),
    }
}

/// The service's status, or `None` if there is none. For drawing (`hoard`,
/// `hoard sync`): it starts nothing.
pub async fn status() -> Option<DaemonStatus> {
    let mut client = attached("status").await?;
    match ask(&mut client, Request::Status).await {
        Ok(Payload::Status(status)) => Some(status),
        Ok(other) => {
            tracing::warn!("cli: unexpected answer to status: {other:?}");
            None
        }
        Err(err) => {
            tracing::warn!(error = %format!("{err:#}"), "cli: couldn't read the service status");
            None
        }
    }
}

/// Best-effort notice to the service, without starting it: with no service there
/// is nothing to notify (when it starts it will read the disk from scratch).
/// Returns `true` if it landed.
async fn notify(what: &str, request: Request) -> bool {
    let Some(mut client) = attached("notify").await else {
        return false;
    };
    match ask(&mut client, request).await {
        Ok(_) => true,
        Err(err) => {
            tracing::warn!(error = %format!("{err:#}"), "cli: couldn't {what}");
            false
        }
    }
}

/// The set of watched saves changed on disk, so have the service re-read it. The
/// client *tells* it, it does not send the list: the service owns the state.
///
/// Returns the sentence the CLI shows the user. These commands used to say
/// "restart `hoard sync` to apply it", which is no longer needed, and would not
/// work either: restarting a client does not restart the engine.
pub async fn notify_reload() -> &'static str {
    if notify("ask the service to reload its watch list", Request::Reload).await {
        "the sync service picked it up"
    } else {
        "it applies when the sync service starts"
    }
}

/// How long one hold on a save lasts in the service, and how often it is
/// renewed. A renewal that arrives late costs nothing; a client that dies stops
/// renewing and the service lets go on its own two minutes later.
const HOLD_SECS: u32 = 120;
const HOLD_RENEW: Duration = Duration::from_secs(30);
/// How often to ask again while the service is still finishing an upload or
/// an automatic restore of the save.
const HOLD_POLL: Duration = Duration::from_secs(1);
/// How long a restore waits for that before it goes ahead anyway. An upload
/// reading the folder while files are swapped in makes one mixed version in
/// the history, which the restored folder then goes up on top of; waiting for
/// ever would be worse.
const HOLD_WAIT_CAP: Duration = Duration::from_secs(600);

/// A restore's hold on its save (see [`Request::HoldSave`]). Taken before the
/// first byte, renewed in the background over a connection it re-opens when
/// the service restarts (an update hands over to a new one mid-restore),
/// released with [`Self::release`].
pub struct RestoreHold {
    save_id: String,
    /// The service's last answer; `None` while it has given none.
    state: tokio::sync::watch::Receiver<Option<HoldState>>,
    renew: tokio::task::JoinHandle<()>,
}

impl RestoreHold {
    /// Returns once the service has nothing of its own touching the folder: at
    /// once unless it answered [`HoldState::Busy`], and never later than
    /// [`HOLD_WAIT_CAP`].
    pub async fn until_held(&mut self) {
        if *self.state.borrow() != Some(HoldState::Busy) {
            return;
        }
        eprintln!(
            "waiting for the Hoard service to finish syncing this save before writing into it"
        );
        let settled = self.state.wait_for(|s| *s != Some(HoldState::Busy));
        if tokio::time::timeout(HOLD_WAIT_CAP, settled).await.is_err() {
            eprintln!(
                "warning: the Hoard service is still syncing this save after {} minutes; restoring anyway",
                HOLD_WAIT_CAP.as_secs() / 60
            );
        }
    }

    /// Let go: the service judges what the restore wrote right away. After a
    /// restore that went through, `adopt_head` is the newest version the server
    /// holds, and the restored folder goes up as the next one on top of it.
    pub async fn release(self, adopt_head: Option<i64>) {
        self.renew.abort();
        notify(
            "release the save after restoring",
            Request::HoldSave {
                save_id: self.save_id.clone(),
                secs: 0,
                adopt_head,
            },
        )
        .await;
    }
}

impl Drop for RestoreHold {
    fn drop(&mut self) {
        // Out through a panic or an early return: no more renewals, and the
        // service lets go by itself within `HOLD_SECS`.
        self.renew.abort();
    }
}

/// Ask the service not to back up or pull `save_id` while a restore writes into
/// its folder. `watched_here` says the restore goes into the folder the service
/// syncs for this save, where a hold that finds no save is worth a warning.
///
/// Returns the hold (`None` with no service at all, which means nobody would
/// upload anything anyway, or with one too old to hold) and what to tell the
/// user about it, if anything.
pub async fn hold_for_restore(
    save_id: &str,
    watched_here: bool,
) -> (Option<RestoreHold>, Option<String>) {
    let Some(mut client) = attached("restore").await else {
        return (None, None);
    };
    let request = Request::HoldSave {
        save_id: save_id.to_string(),
        secs: HOLD_SECS,
        adopt_head: None,
    };
    let (first, warning) = match ask(&mut client, request.clone()).await {
        Ok(Payload::Hold { state }) => {
            let warning = (state == HoldState::NotWatched && watched_here).then(|| {
                "the Hoard service isn't syncing this save yet, so it can't hold it; \
                 if it picks it up mid-restore it may back up a half-restored folder"
                    .to_string()
            });
            (Some(state), warning)
        }
        Ok(_) => (Some(HoldState::Held), None),
        Err(err) => match err.downcast_ref::<IpcError>() {
            Some(IpcError::Unsupported { .. }) => {
                return (
                    None,
                    Some(
                        "the running Hoard service is older than this command and can't pause \
                         the save, so it may back up the folder while it is being restored"
                            .to_string(),
                    ),
                )
            }
            // Renewed below all the same: an engine that comes up mid-restore
            // takes the hold from the next renewal.
            Some(IpcError::EngineDown { .. }) => (
                None,
                Some(
                    "the Hoard service's sync engine isn't running; if it starts during the \
                     restore it may back up a half-restored folder"
                        .to_string(),
                ),
            ),
            _ => (
                None,
                Some(format!(
                    "couldn't pause the save in the Hoard service ({err:#})"
                )),
            ),
        },
    };
    let (tx, state) = tokio::sync::watch::channel(first);
    let renew = tokio::spawn(async move {
        let mut client = Some(client);
        loop {
            let busy = *tx.borrow() == Some(HoldState::Busy);
            tokio::time::sleep(if busy { HOLD_POLL } else { HOLD_RENEW }).await;
            if client.is_none() {
                client = attached("restore").await;
            }
            let Some(conn) = client.as_mut() else {
                continue;
            };
            match ask(conn, request.clone()).await {
                Ok(Payload::Hold { state }) => {
                    let _ = tx.send(Some(state));
                }
                Ok(_) => {
                    let _ = tx.send(Some(HoldState::Held));
                }
                Err(err) => {
                    tracing::warn!(error = %format!("{err:#}"), "cli: couldn't renew the restore's hold");
                    // A service that restarted is a new socket: reconnect next
                    // time instead of asking a dead one for ever.
                    client = None;
                }
            }
        }
    });
    (
        Some(RestoreHold {
            save_id: save_id.to_string(),
            state,
            renew,
        }),
        warning,
    )
}

/// The on-disk session changed (login or logout), so have the service resolve
/// from scratch. An account change invalidates its `ApiClient`, its context and
/// its token rotator, and none of the three is fixed by re-reading the saves.
pub async fn notify_session_changed() {
    notify(
        "tell the service the session changed",
        Request::RestartEngine,
    )
    .await;
}

/// Hands the service the session `hoard login` just minted, so *it* stores it.
/// `false` means there was no service to hand it to.
///
/// The CLI is a third binary, and on macOS that matters: the keychain item only
/// authorises the binary that creates it, so a login from the terminal writing
/// the keychain would leave the service asking the user for a password on every
/// read (ADR 0021 D.20). Here it is minted and handed over; the owner is what
/// stores it.
///
/// It is preceded by a forget on the *same* connection: storing is
/// read-modify-write, so without deleting first, signing in with another account
/// would leave the previous one's `user` and `server_url` on disk.
pub async fn hand_over_session(session: hoard_core::ipc::AdoptedSession) -> bool {
    let Some(mut client) = attached("login").await else {
        return false;
    };
    if let Err(err) = client.forget_session().await {
        tracing::warn!(error = %format!("{err:#}"), "cli: the service couldn't drop the previous session");
        return false;
    }
    match client.adopt_session(session).await {
        Ok(()) => true,
        Err(err) => {
            tracing::warn!(error = %format!("{err:#}"), "cli: the service didn't take the new session");
            false
        }
    }
}

/// Hands the service the self-hosted session `hoard login --token` just
/// validated, so *it* stores it. That is the store the engine resolves from, so
/// without this a login from the terminal would not change which session the
/// machine syncs with if the app already had one. `false` means there was no
/// service.
pub async fn hand_over_server_session(session: hoard_core::ipc::ServerSession) -> bool {
    let Some(mut client) = attached("login").await else {
        return false;
    };
    match client.adopt_server_session(session).await {
        Ok(()) => true,
        Err(err) => {
            tracing::warn!(error = %format!("{err:#}"), "cli: the service didn't take the new self-hosted session");
            false
        }
    }
}

/// Tell the service to forget the self-hosted session. `false` means no service.
pub async fn hand_over_server_logout() -> bool {
    let Some(mut client) = attached("logout").await else {
        return false;
    };
    match client.forget_server_session().await {
        Ok(()) => true,
        Err(err) => {
            tracing::warn!(error = %format!("{err:#}"), "cli: the service didn't clear its stored self-hosted session");
            false
        }
    }
}

/// Borrow the self-hosted session. `None` means there is no service to ask, or no
/// session of this kind on the machine; either way the caller falls back to
/// `config.toml`, which is the usual headless path.
pub async fn borrow_server_session() -> Option<hoard_core::ipc::ServerSession> {
    let mut client = attached("server token").await?;
    match client.server_session().await {
        Ok(session) => Some(session),
        Err(err) => {
            tracing::debug!(error = %format!("{err:#}"), "cli: couldn't borrow the self-hosted session");
            None
        }
    }
}

/// Tell the service to forget the Cloud session. `false` means there is no
/// service, so the keyring keeps an orphaned pair (harmless: with no session file
/// there is no session, and the next login overwrites it).
pub async fn hand_over_logout() -> bool {
    let Some(mut client) = attached("logout").await else {
        return false;
    };
    match client.forget_session().await {
        Ok(()) => true,
        Err(err) => {
            tracing::warn!(error = %format!("{err:#}"), "cli: the service didn't clear its stored session");
            false
        }
    }
}

/// Borrow a Cloud token from the service. `None` when there is no service to ask.
///
/// A `CloudSessionExpired` is not swallowed: if GoTrue revoked the token family,
/// carrying on with the one on disk only produces a 401 with a worse message.
pub async fn borrow_cloud_token(rejected: Option<String>) -> Result<Option<CloudToken>> {
    let Some(mut client) = attached("cloud token").await else {
        return Ok(None);
    };
    match ask(&mut client, Request::CloudToken { rejected }).await {
        Ok(Payload::CloudToken(token)) => {
            // As on the desktop: the log shipper reads from this slot.
            hoard_agent::credentials::set_lent_cloud(Some(hoard_agent::credentials::CloudLease {
                url: token.server_url.clone(),
                token: token.access_token.clone(),
            }));
            Ok(Some(token))
        }
        Ok(other) => anyhow::bail!("unexpected answer to cloud_token: {other:?}"),
        Err(err) => {
            if matches!(
                err.downcast_ref::<IpcError>(),
                Some(IpcError::CloudSessionExpired { .. })
            ) {
                return Err(err.context("the Hoard service couldn't renew the Cloud session"));
            }
            // Transient (network, a grumpy GoTrue): the token on disk may still
            // be good, so try with it rather than abort.
            tracing::warn!(error = %format!("{err:#}"), "cli: couldn't borrow a Cloud token");
            Ok(None)
        }
    }
}

/// An active session for a one-shot that needs to talk to the server.
///
/// Asks the service for the token and resolves with it. With no service the one on
/// disk is used *without rotating it*: rotating here is the thing to avoid, since
/// two processes rotating the same refresh token is GoTrue's reuse detection and a
/// revoked session. An expired token with no service gives a 401, and there the
/// hint is shown that the service is what renews.
pub async fn resolve_session() -> Result<Active> {
    // With no Cloud session there is nothing to borrow: this is a self-hosted
    // user, and the service would answer "there is no Cloud session", an invented
    // error about something this command does not need.
    if hoard_agent::cloud_auth::load_session()?.is_none() {
        // Self-hosted: the app's session is stored by the service (D.20), so it
        // gets borrowed. With no service it falls back to `config.toml`, as ever.
        return session::resolve_borrowed(None, borrow_server_session().await).await;
    }
    let lent = borrow_cloud_token(None).await?;
    let borrowed = lent.is_some();
    match session::resolve_borrowed(lent, None).await {
        Ok(active) => Ok(active),
        Err(err) if !borrowed => Err(hint_stale_session(err)),
        Err(err) => Err(err),
    }
}

/// Adds the hint that the fix is bringing the service up rather than signing in
/// again, but only when the token on disk really has expired.
fn hint_stale_session(err: anyhow::Error) -> anyhow::Error {
    let Ok(Some(sess)) = hoard_agent::cloud_auth::load_session() else {
        return err;
    };
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    match session::stale_token_hint(&sess.access, now) {
        Some(hint) => err.context(hint),
        None => err,
    }
}
