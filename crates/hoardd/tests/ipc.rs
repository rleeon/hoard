//! The protocol end to end over a real socket: handshake, commands, backlog by
//! cursor and live push.
//!
//! The engine is **not** started in any test: the IPC server is stood up with an
//! empty [`Engine`] and the journal is fed by hand. A test cannot bring the real
//! engine up, it would start syncing the saves of whoever runs the tests.
//!
//! For the same reason there is no `Request::CloudToken` case here: lending it
//! reads the **real** Cloud session of whoever runs the tests (keyring plus
//! `cloud.toml`) and, when it is close to expiring, **rotates** it. A `cargo test`
//! must not touch anybody's session. What decides whether to rotate is pure and
//! tested in `hoard_agent::session` (`needs_rotation`), and the shapes of the
//! request and the response are in `hoard_core::ipc`'s golden test.
//!
//! `AdoptSession` and `ForgetSession` (D.20) are missing for the same reason, and
//! with more force: they **write** the keyring and the `cloud.toml` of whoever runs
//! the tests, so a case here would change their real session. What can be checked
//! without touching secrets is checked: the wire shape and that a handed-over
//! session is never printed (`hoard_core::ipc`), and the service-less path
//! (`hoard_agent::cloud_auth`).

use std::sync::Arc;

use hoard_core::ipc::{ClientFrame, Hello, Payload, Request, ServerFrame, PROTOCOL_VERSION};
use hoardd::client::{Client, Push};
use hoardd::codec::{read_frame, write_frame};
use hoardd::endpoint::Endpoint;
use hoardd::engine::Engine;
use hoardd::journal::EventLog;
use hoardd::server::{accept_loop, Daemon};
use hoardd::transport::{self, Listener};
use time::OffsetDateTime;

/// Sufijo irrepetible para el endpoint de un test.
fn unique(tag: &str) -> String {
    use std::sync::atomic::{AtomicU32, Ordering};
    static SEQ: AtomicU32 = AtomicU32::new(0);
    format!(
        "{tag}-{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    )
}

/// Un daemon sirviendo en un socket temporal, con su journal a mano.
struct Fixture {
    endpoint: Endpoint,
    log: Arc<EventLog>,
    _dir: tempfile::TempDir,
    accept: tokio::task::JoinHandle<hoard_agent::supervisor::Finished>,
}

impl Fixture {
    fn start() -> Self {
        let dir = tempfile::tempdir().unwrap();
        // A unique name: on Windows pipes share a global namespace, so two tests in
        // parallel (or two `cargo test` runs at once) would collide.
        let endpoint = Endpoint::scoped(dir.path(), &unique("ipc"));
        let listener = Listener::bind(&endpoint).expect("bind");
        let log = Arc::new(EventLog::new());
        let daemon = Arc::new(Daemon::new(log.clone(), Engine::new()));
        let accept = tokio::spawn(accept_loop(
            Arc::new(tokio::sync::Mutex::new(listener)),
            daemon,
        ));
        Self {
            endpoint,
            log,
            _dir: dir,
            accept,
        }
    }

    async fn client(&self) -> Client {
        Client::connect(&self.endpoint, "hoardd tests")
            .await
            .expect("connect")
    }

    fn record(&self, save: &str) {
        self.log.record(
            OffsetDateTime::now_utc(),
            hoard_agent::agent::AgentEvent::GameStarted {
                save_id: save.to_string(),
                game_slug: "factorio".to_string(),
            },
        );
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.accept.abort();
    }
}

#[tokio::test]
async fn the_handshake_identifies_the_daemon() {
    let fx = Fixture::start();
    let mut client = fx.client().await;
    let welcome = client.welcome().clone();
    assert_eq!(welcome.protocol, PROTOCOL_VERSION);
    assert_eq!(welcome.pid, std::process::id());
    assert!(!welcome.epoch.is_empty(), "each run identifies itself");
    assert_eq!(welcome.cursor, 0);

    let (version, pid) = client.ping().await.unwrap();
    assert_eq!(pid, std::process::id());
    assert_eq!(version, env!("CARGO_PKG_VERSION"));
}

/// A client arriving late recovers the history by cursor and **then** listens live.
/// Both halves of D.14.2 in one test: push-only would have lost the first two events
/// (the mute-bell bug).
#[tokio::test]
async fn a_late_client_gets_the_backlog_and_then_live_pushes() {
    let fx = Fixture::start();
    fx.record("a");
    fx.record("b");

    let mut client = fx.client().await;
    let backlog = client.subscribe(None).await.unwrap();
    assert_eq!(backlog.entries.len(), 2);
    assert_eq!(backlog.cursor, 2);
    assert!(!backlog.gap);

    fx.record("c");
    let push = tokio::time::timeout(std::time::Duration::from_secs(2), client.next_push())
        .await
        .expect("a live push must arrive")
        .unwrap()
        .expect("stream open");
    match push {
        Push::Event(entry) => {
            assert_eq!(entry.seq, 3);
            assert!(matches!(
                entry.event,
                hoard_agent::agent::AgentEvent::GameStarted { .. }
            ));
        }
        other => panic!("unexpected push: {other:?}"),
    }
}

/// Reconnecting with the stored cursor neither re-delivers what was already seen
/// nor skips what happened while we were disconnected.
#[tokio::test]
async fn reconnecting_resumes_exactly_at_the_cursor() {
    let fx = Fixture::start();
    fx.record("a");
    let mut first = fx.client().await;
    let cursor = first.subscribe(None).await.unwrap().cursor;
    drop(first);

    // Mientras nadie escucha.
    fx.record("b");
    fx.record("c");

    let mut second = fx.client().await;
    let backlog = second.subscribe(Some(cursor)).await.unwrap();
    assert_eq!(backlog.entries.len(), 2, "only what comes after the cursor");
    assert_eq!(backlog.entries[0].seq, 2);
    assert!(!backlog.gap);
}

/// A client on another protocol version is rejected **while stating the daemon's
/// version**, which is what lets the app say "restart the service" instead of dying
/// with a parse error.
#[tokio::test]
async fn a_foreign_protocol_is_rejected_with_the_daemon_version() {
    let fx = Fixture::start();
    let stream = transport::connect(&fx.endpoint).await.unwrap();
    let (mut reader, mut writer) = tokio::io::split(stream);
    write_frame(
        &mut writer,
        &ClientFrame::Hello(Hello {
            protocol: PROTOCOL_VERSION + 41,
            client: "from the future".into(),
        }),
    )
    .await
    .unwrap();
    match read_frame::<_, ServerFrame>(&mut reader).await.unwrap() {
        Some(ServerFrame::Rejected(rejected)) => {
            assert_eq!(rejected.daemon_protocol, PROTOCOL_VERSION);
            assert_eq!(rejected.daemon_version, env!("CARGO_PKG_VERSION"));
        }
        other => panic!("expected a rejection, got {other:?}"),
    }
}

/// Sin handshake no se atiende nada.
#[tokio::test]
async fn a_request_before_the_handshake_is_rejected() {
    let fx = Fixture::start();
    let stream = transport::connect(&fx.endpoint).await.unwrap();
    let (mut reader, mut writer) = tokio::io::split(stream);
    write_frame(
        &mut writer,
        &ClientFrame::Request {
            id: 1,
            request: Request::Ping,
        },
    )
    .await
    .unwrap();
    match read_frame::<_, ServerFrame>(&mut reader).await.unwrap() {
        Some(ServerFrame::Rejected(rejected)) => {
            assert!(rejected.reason.contains("hello"), "{}", rejected.reason)
        }
        other => panic!("expected a rejection, got {other:?}"),
    }
}

/// The daemon serves the IPC with no engine, and a command says **why** there is
/// none. A client that only saw "error" would retry for ever with nothing to tell
/// the user.
#[tokio::test]
async fn commands_without_an_engine_say_why() {
    let fx = Fixture::start();
    let mut client = fx.client().await;

    let status = client.status().await.unwrap();
    assert!(!status.engine.running);
    assert_eq!(status.protocol, PROTOCOL_VERSION);
    assert!(status.slots.is_empty());

    let err = client
        .request(Request::BackupNow {
            save_id: "s1".into(),
        })
        .await
        .expect_err("no engine, no backup");
    // The reason travels **inside the message**, not only in the variant: this text
    // ends up in a desktop toast or in the CLI's stdout, so an `EngineDown { reason:
    // ... }` dumped with `{:?}` is what the user would read.
    let text = err.to_string();
    assert!(text.contains("no engine"), "{text}");
    assert!(
        !text.contains("EngineDown"),
        "leaked the Debug shape: {text}"
    );
}

/// Asking for an engine restart is **not** answered with `EngineDown` when there is
/// no engine: it is precisely the request that can bring it back (the keeper
/// resolves the session again). Answering "I cannot because it is broken" would
/// leave the desktop no way to tell the service the user has just signed in.
#[tokio::test]
async fn restarting_the_engine_is_accepted_even_without_one() {
    let fx = Fixture::start();
    let mut client = fx.client().await;
    assert!(matches!(
        client.request(Request::RestartEngine).await.unwrap(),
        Payload::Ack
    ));
}

/// The probe candidates do need an engine, and without one the reason is stated.
/// This test exists mostly as a wire check: it is the only request that sends a list
/// from the client to the engine, and if somebody drops it from the dispatch, it
/// shows up here.
#[tokio::test]
async fn probe_candidates_need_an_engine_and_say_so() {
    let fx = Fixture::start();
    let mut client = fx.client().await;
    let err = client
        .request(Request::SetProbeCandidates {
            dirs: vec!["/tmp/candidate".into()],
        })
        .await
        .expect_err("no engine, no probing");
    assert!(err.to_string().contains("no engine"), "{err}");
}

/// The status is served with an empty journal: it is the snapshot a client paints
/// from without having seen a single event.
#[tokio::test]
async fn status_answers_on_an_empty_journal() {
    let fx = Fixture::start();
    let mut client = fx.client().await;
    let status = client.status().await.unwrap();
    assert_eq!(status.cursor, 0);
    assert_eq!(status.pid, std::process::id());

    let backlog = client.subscribe(None).await.unwrap();
    assert!(backlog.entries.is_empty());
    assert!(!backlog.gap);
    assert!(matches!(
        client.request(Request::Ping).await.unwrap(),
        Payload::Pong { .. }
    ));
}
