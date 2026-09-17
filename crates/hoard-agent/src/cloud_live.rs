//! Low-latency Cloud push for the engine that `hoardd` runs.
//!
//! Two halves that complement each other:
//! - The event stream (`events_loop`): `GET /v1/events` on the Hoard server, a
//!   long-lived SSE response that carries one `save` frame the moment another
//!   device of the account commits a version. Each frame turns into an
//!   immediate pull, around a second instead of waiting for the poll. The server
//!   is the messenger because it is the only one that sees the commit: Supabase
//!   Realtime watched the `saves` table, and has had nothing to watch since the
//!   tables left Supabase (17-sep-2026).
//! - The backup poll (`poll_loop`): it hits `/v1/cloud/sync` every
//!   `poll_interval` in case the stream drops or a frame is lost. The manifest is
//!   excluded from the bandwidth quota, so it is free in both money and bytes.
//!
//! Both end in the same place: the agent's version cache is fed
//! (`set_cloud_versions`) and, with global sync on, a `force_restore` is asked for
//! the saves that moved forward on the server. The version gate and the
//! mid-session vetoes live inside the agent, so asking for too much never walks
//! over data.
//!
//! All best-effort: a failing stream reconnects with backoff and a failing poll
//! retries on the next tick. The daemon never dies over this.

use std::collections::HashMap;
use std::time::Duration;

use futures::StreamExt;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{interval, sleep, Instant, MissedTickBehavior};

use crate::agent::AgentHandle;
use crate::api::ApiClient;
use crate::cloud_auth;
use crate::sse;

/// A connection's maximum life. Authentication happens once, when the stream
/// opens; recycling it now and then means a signed-out or revoked session stops
/// hearing about the account within the hour.
const CONNECTION_MAX_SECS: u64 = 45 * 60;

/// Bounds on the reconnection backoff.
const BACKOFF_MIN_SECS: u64 = 2;
const BACKOFF_MAX_SECS: u64 = 60;

/// A server without `/v1/events` (older than the move off Supabase) will not
/// grow one in a minute. The poll carries the sync meanwhile.
const UNSUPPORTED_RETRY_SECS: u64 = 30 * 60;

/// An account the server refuses (403, scheduled for deletion) comes back only
/// when its owner reactivates it from the web, so there is no point asking every
/// minute. Refreshing the token does not change the answer either, and each
/// refresh rotates the session for nothing.
const REFUSED_RETRY_SECS: u64 = 30 * 60;

/// The parked mode's cadence: with no usable session, the stream only re-reads
/// the session file waiting for a fresh login. It mirrors the daemon's periodic
/// refresher recheck (session.rs::RELOGIN_RECHECK_EVERY), since it is the same
/// event and there is no sense in learning about it at two different rates.
const RELOGIN_RECHECK_SECS: u64 = 5 * 60;

/// Settings for the Cloud push.
pub struct Config {
    /// Period of the backup poll to `/v1/cloud/sync`.
    pub poll_interval: Duration,
    /// Global sync: when a save moves forward on the server, force its restore now
    /// rather than only feeding the version cache. Mirrors `Prefs::global_sync`.
    pub global_sync: bool,
}

/// Starts the poll and event-stream loops and returns their tasks. The daemon
/// keeps them so they live as long as it does; they only stop if explicitly
/// aborted, or when the process dies.
pub fn spawn(client: ApiClient, handle: AgentHandle, cfg: Config) -> Vec<JoinHandle<()>> {
    // A "kick" channel of capacity 1: a pushed frame asks for a pull off cadence.
    // With one already pending, the `try_send` drops it, so a burst of changes
    // collapses into a single extra pull.
    let (kick_tx, kick_rx) = mpsc::channel::<()>(1);

    let events = tokio::spawn(events_loop(client.clone(), kick_tx));
    let poll = tokio::spawn(poll_loop(client, handle, cfg, kick_rx));
    vec![poll, events]
}

/// The backup poll plus the kick consumer. It runs a pull on every timer tick and
/// on every pushed frame, serialised by the `select!` so never two at once. It
/// keeps the `save_id` to `version_num` map it has seen, to spot advances.
async fn poll_loop(
    client: ApiClient,
    handle: AgentHandle,
    cfg: Config,
    mut kick_rx: mpsc::Receiver<()>,
) {
    tracing::info!(
        poll_secs = cfg.poll_interval.as_secs(),
        global_sync = cfg.global_sync,
        "cloud-live: Cloud push started"
    );

    // The map lives in memory: a new session starts from nothing and the first
    // pass only sets the baseline, with nothing counting as "advanced", the same
    // as the desktop, so no mass restore is forced right at startup.
    let mut seen: HashMap<String, i64> = HashMap::new();

    // `interval`'s first tick is immediate, so the baseline is seeded on start.
    // `Skip` avoids bursts if the system freezes for a while.
    let mut ticker = interval(cfg.poll_interval);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = ticker.tick() => {}
            k = kick_rx.recv() => {
                if k.is_none() {
                    // The sender died (which should not happen while the stream runs).
                    return;
                }
            }
        }
        // Drain the kicks piled up: several changes in a row mean a single pull.
        while kick_rx.try_recv().is_ok() {}

        run_pull(&client, &handle, cfg.global_sync, &mut seen).await;
    }
}

/// One manifest pull: it feeds the agent's version cache and, with global sync,
/// forces the restore of the saves that advanced since the last pass.
async fn run_pull(
    client: &ApiClient,
    handle: &AgentHandle,
    global_sync: bool,
    seen: &mut HashMap<String, i64>,
) {
    let manifest = match client.cloud_sync().await {
        Ok(m) => m,
        Err(e) => {
            // A transient 401 (a token on the edge) or a network drop recovers on
            // its own next tick; the daemon's periodic refresh keeps the client's
            // token current.
            tracing::debug!(error = %format!("{e:#}"), "cloud-live: the pull failed");
            return;
        }
    };

    let mut latest: HashMap<String, i64> = HashMap::with_capacity(manifest.saves.len());
    // The name→id index from the same pass: without it the agent's cache can't
    // answer for a save whose local id the cloud has never seen (see
    // `CloudHeads::aliases`).
    let mut aliases: HashMap<(String, String), String> =
        HashMap::with_capacity(manifest.saves.len());
    let mut advanced: Vec<String> = Vec::new();
    for e in &manifest.saves {
        latest.insert(e.save_id.clone(), e.latest_version_num);
        aliases.insert(
            (
                e.game_slug.clone(),
                if e.label.is_empty() {
                    "default".to_string()
                } else {
                    e.label.clone()
                },
            ),
            e.save_id.clone(),
        );
        // It only counts as an advance when we already had a previous version and
        // it went up. The ones seen for the first time (`None`) only set a baseline.
        if let Some(prev) = seen.get(&e.save_id) {
            if e.latest_version_num > *prev {
                advanced.push(e.save_id.clone());
            }
        }
    }
    *seen = latest.clone();

    // Feed the agent's version cache on every pass rather than only on deltas, so
    // the reconciliation sweep can gate by version without re-fetching the
    // manifest for each save.
    if let Err(e) = handle.set_cloud_versions(latest, aliases).await {
        tracing::warn!(error = %format!("{e:#}"), "cloud-live: could not feed the versions cache");
    }

    // Global sync: ask for the immediate pull of whatever advanced. The agent
    // gates by version and honours the mid-session vetoes, so this never walks
    // over data.
    if global_sync {
        for id in advanced {
            if let Err(e) = handle.force_restore(id).await {
                tracing::warn!(error = %format!("{e:#}"), "cloud-live: could not ask for a force-restore");
            }
        }
    }
}

/// How one connection to the stream ended.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    /// The server closed it or the life cap expired: reconnect.
    Ended,
    /// 401: the token needs refreshing first.
    Unauthorized,
    /// 403: the token is fine and the account is not (scheduled for deletion).
    Refused,
    /// The server has no `/v1/events`.
    Unsupported,
}

/// The stream's outer reconnection loop. It never ends on its own: if the Cloud
/// session disappears or GoTrue revokes the token family, instead of dying it
/// parks watching the session file, with no network, until a `hoard login` (here
/// or on the desktop, which share the file) leaves a new session, and then it
/// reconnects.
async fn events_loop(client: ApiClient, kick_tx: mpsc::Sender<()>) {
    let mut backoff = BACKOFF_MIN_SECS;
    loop {
        match connect_once(&client, &kick_tx).await {
            Ok(Outcome::Ended) => backoff = BACKOFF_MIN_SECS,
            Ok(Outcome::Unsupported) => {
                tracing::info!(
                    "cloud-live: this server has no event stream, the poll carries the sync"
                );
                sleep(Duration::from_secs(UNSUPPORTED_RETRY_SECS)).await;
                continue;
            }
            Ok(Outcome::Refused) => {
                tracing::info!(
                    "cloud-live: the server refuses this account, the stream waits for it to be reactivated"
                );
                sleep(Duration::from_secs(REFUSED_RETRY_SECS)).await;
                backoff = BACKOFF_MIN_SECS;
                continue;
            }
            Ok(Outcome::Unauthorized) => {
                // Through `refresh_freshest`, never by replaying a token captured
                // earlier: by now the periodic refresher may have rotated it, and
                // replaying a rotated one is reuse detection, which GoTrue answers
                // by revoking the whole token family. The fresh pair lands on disk
                // and in the shared client.
                match cloud_auth::refresh_freshest().await {
                    Ok(tokens) => client.set_token(tokens.access),
                    Err(e) if e.downcast_ref::<cloud_auth::RefreshTokenStale>().is_some() => {
                        park_until_new_login().await;
                        if let Ok(Some(s)) = cloud_auth::load_session() {
                            client.set_token(s.access);
                        }
                        backoff = BACKOFF_MIN_SECS;
                    }
                    Err(e) => {
                        tracing::debug!(error = %format!("{e:#}"), "cloud-live: refresh before reconnecting failed");
                    }
                }
            }
            Err(e) => {
                tracing::debug!(error = %format!("{e:#}"), "cloud-live: event stream dropped, retrying");
            }
        }
        sleep(Duration::from_secs(backoff)).await;
        backoff = (backoff * 2).min(BACKOFF_MAX_SECS);
    }
}

/// Watch the disk without touching the network: replaying a revoked token
/// against GoTrue every few minutes is exactly the noise the refresher shed.
async fn park_until_new_login() {
    let dead = cloud_auth::load_session().ok().flatten().map(|s| s.refresh);
    tracing::info!("cloud-live: event stream parked, waiting for a fresh login");
    loop {
        sleep(Duration::from_secs(RELOGIN_RECHECK_SECS)).await;
        let disk = cloud_auth::load_session().ok().flatten();
        if session_renewed(dead.as_deref(), disk.as_ref()) {
            tracing::info!("cloud-live: new session on disk, event stream reconnecting");
            return;
        }
    }
}

/// Is what is on disk no longer the session that died? Only then is reconnecting
/// worth it: a different refresh token, or a session where there was none, is a
/// fresh login; the same dead session would keep bouncing off the stream.
fn session_renewed(dead: Option<&str>, disk: Option<&cloud_auth::Session>) -> bool {
    let Some(s) = disk else { return false };
    if s.refresh.trim().is_empty() {
        return false;
    }
    dead != Some(s.refresh.as_str())
}

/// One connection: open the stream and turn its frames into kicks until it
/// closes, stalls or reaches its life cap.
async fn connect_once(client: &ApiClient, kick_tx: &mpsc::Sender<()>) -> anyhow::Result<Outcome> {
    let resp = client.event_stream().await?;
    match resp.status().as_u16() {
        401 => return Ok(Outcome::Unauthorized),
        403 => return Ok(Outcome::Refused),
        404 | 405 => return Ok(Outcome::Unsupported),
        s if !(200..300).contains(&s) => anyhow::bail!("/v1/events answered {s}"),
        _ => {}
    }
    // Whatever landed while this device was not listening produced no frame for
    // it, so a (re)connection starts with a catch-up pull.
    let _ = kick_tx.try_send(());

    let deadline = Instant::now() + Duration::from_secs(CONNECTION_MAX_SECS);
    let mut parser = sse::Parser::default();
    let mut body = resp.bytes_stream();
    loop {
        tokio::select! {
            _ = tokio::time::sleep_until(deadline) => return Ok(Outcome::Ended),
            chunk = body.next() => {
                let Some(chunk) = chunk else { return Ok(Outcome::Ended) };
                for event in parser.push(&chunk?) {
                    // `lagged` means the server dropped frames for us: the same
                    // pull catches up on whatever they said.
                    if event.kind == "save" || event.kind == "lagged" {
                        tracing::debug!(kind = %event.kind, "cloud-live: pushed change, pulling");
                        let _ = kick_tx.try_send(());
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud_auth::Session;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    fn disk(refresh: &str) -> Session {
        Session {
            server_url: "https://api.hoard.services".into(),
            access: "jwt".into(),
            refresh: refresh.into(),
        }
    }

    /// The parked loop only wakes for a session that is NOT the dead one: the same
    /// one that was refused cannot do any better the second time.
    #[test]
    fn session_renewed_ignores_the_dead_session_and_wakes_on_a_new_one() {
        // With nothing on disk, or an empty refresh: keep waiting.
        assert!(!session_renewed(Some("rt-dead"), None));
        assert!(!session_renewed(Some("rt-dead"), Some(&disk("  "))));
        // The same dead session is still on disk: keep waiting.
        assert!(!session_renewed(Some("rt-dead"), Some(&disk("rt-dead"))));
        // A fresh login (a different refresh token) wakes it up.
        assert!(session_renewed(Some("rt-dead"), Some(&disk("rt-new"))));
    }

    /// The "there was no session when we parked" case (a logout mid-flight): any
    /// session with a non-empty refresh counts as a new login.
    #[test]
    fn session_renewed_wakes_on_any_session_when_none_was_dead() {
        assert!(!session_renewed(None, None));
        assert!(!session_renewed(None, Some(&disk(""))));
        assert!(session_renewed(None, Some(&disk("rt-fresh"))));
    }

    /// A one-shot HTTP server: answers the first request with `head` and then
    /// writes `body` in the pieces given, closing after the last one.
    async fn server(head: &'static str, body: Vec<&'static str>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut req = Vec::new();
            let mut byte = [0u8; 1];
            while !req.ends_with(b"\r\n\r\n") {
                if sock.read(&mut byte).await.unwrap() == 0 {
                    return;
                }
                req.push(byte[0]);
            }
            let text = String::from_utf8_lossy(&req);
            assert!(text.starts_with("GET /v1/events "), "{text}");
            assert!(
                text.to_lowercase().contains("authorization: bearer tok"),
                "{text}"
            );
            sock.write_all(head.as_bytes()).await.unwrap();
            for piece in body {
                sock.write_all(piece.as_bytes()).await.unwrap();
                sock.flush().await.unwrap();
                sleep(Duration::from_millis(20)).await;
            }
        });
        format!("http://{addr}")
    }

    fn client(base: String) -> ApiClient {
        ApiClient::new(base, "tok").unwrap()
    }

    const SSE_HEAD: &str =
        "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\nconnection: close\r\n\r\n";

    #[tokio::test]
    async fn a_pushed_save_becomes_a_pull_and_so_does_connecting() {
        let base = server(
            SSE_HEAD,
            vec![
                ":\n\n",
                "event: save\ndata: {\"save_id\":\"s\",",
                "\"version_num\":7}\n\n",
                "event: something-else\ndata: x\n\n",
            ],
        )
        .await;
        let (tx, mut rx) = mpsc::channel(8);
        let outcome = connect_once(&client(base), &tx).await.unwrap();
        assert_eq!(outcome, Outcome::Ended);
        let mut kicks = 0;
        while rx.try_recv().is_ok() {
            kicks += 1;
        }
        assert_eq!(kicks, 2, "one catch-up on connect, one for the save frame");
    }

    #[tokio::test]
    async fn a_refused_token_and_an_old_server_are_told_apart() {
        let (tx, mut rx) = mpsc::channel(8);
        let base = server(
            "HTTP/1.1 401 Unauthorized\r\ncontent-length: 0\r\n\r\n",
            vec![],
        )
        .await;
        assert_eq!(
            connect_once(&client(base), &tx).await.unwrap(),
            Outcome::Unauthorized
        );
        let base = server(
            "HTTP/1.1 404 Not Found\r\ncontent-length: 0\r\n\r\n",
            vec![],
        )
        .await;
        assert_eq!(
            connect_once(&client(base), &tx).await.unwrap(),
            Outcome::Unsupported
        );
        // A 403 is the account, not the token: refreshing would rotate the session
        // once a minute against an answer that cannot change.
        let base = server(
            "HTTP/1.1 403 Forbidden\r\ncontent-length: 0\r\n\r\n",
            vec![],
        )
        .await;
        assert_eq!(
            connect_once(&client(base), &tx).await.unwrap(),
            Outcome::Refused
        );
        assert!(
            rx.try_recv().is_err(),
            "nothing to pull when the stream never opened"
        );

        // Maintenance answers 429: an error that backs off, not a reason to park.
        let base = server(
            "HTTP/1.1 429 Too Many Requests\r\ncontent-length: 0\r\n\r\n",
            vec![],
        )
        .await;
        assert!(connect_once(&client(base), &tx).await.is_err());
    }
}
