//! The log shipper's whole journey, against a real server.
//!
//! It exists because of the bug that started all of this: `logship` resolved the
//! session by looking at the self-hosted store **only**, so a machine signed into
//! Cloud shipped nothing, and nobody noticed for three months because not one test
//! checked that a batch arrives. The unit tests for redaction and filtering can all
//! be green with the pipe unplugged; this one cannot.
//!
//! It stands up a minimal HTTP server that talks like Cloud (`/v1/health` with
//! `mode: "cloud"`), starts the real layer over the process's `tracing`, emits
//! events and checks **the body that reaches the POST**: that it arrives, that it
//! goes down the Cloud path, with the right token, with the paths redacted, with
//! the verdict inside despite being below the minimum, and with no operational
//! INFO.
//!
//! It gets its own test binary because it touches two process-global things,
//! `XDG_DATA_HOME` (so it does not read the user's real prefs) and the `tracing`
//! subscriber, and neither can be shared with other tests.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

use hoard_agent::credentials::{self, CloudLease};
use hoard_agent::prefs::Prefs;
use hoard_core::wire::{LogBatch, LogEntry, TELEMETRY_TARGET};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// What the stub server saw in one request.
struct Seen {
    method: String,
    path: String,
    authorization: Option<String>,
    body: String,
}

/// A one-piece HTTP server: it answers `/v1/health` like Cloud and collects
/// whatever reaches the ingest. It closes the connection on every response
/// (`Connection: close`) so keep-alive never has to be implemented.
fn spawn_stub() -> (String, Receiver<Seen>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    let (tx, rx): (Sender<Seen>, Receiver<Seen>) = channel();

    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { break };
            if serve_one(stream, &tx).is_err() {
                break;
            }
        }
    });

    (format!("http://{addr}"), rx)
}

fn serve_one(mut stream: TcpStream, tx: &Sender<Seen>) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);

    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let path = parts.next().unwrap_or_default().to_string();

    let mut content_length = 0usize;
    let mut authorization = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        let (name, value) = line.split_once(':').unwrap_or((line, ""));
        match name.to_ascii_lowercase().as_str() {
            "content-length" => content_length = value.trim().parse().unwrap_or(0),
            "authorization" => authorization = Some(value.trim().to_string()),
            _ => {}
        }
    }

    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body)?;
    }
    let body = String::from_utf8_lossy(&body).into_owned();

    // Cloud announces WARN: the operational INFO is filtered at the source and the
    // verdict still gets in through its `target`. That is the combination this test
    // checks.
    let payload = if path.starts_with("/v1/health") {
        r#"{"status":"ok","version":"9.9.9","mode":"cloud","log_min_level":"warn"}"#.to_string()
    } else {
        r#"{"accepted":1}"#.to_string()
    };

    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        payload.len(),
        payload
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()?;

    let _ = tx.send(Seen {
        method,
        path,
        authorization,
        body,
    });
    Ok(())
}

/// Gathers the POSTs that arrive until there are `wanted` entries or time runs out.
///
/// There are several because the shipper batches by time: if the thread wakes up
/// halfway through the burst, some events go in one batch and the rest in another.
/// Waiting for "the" batch would be a test that passes or fails on clock luck.
fn collect_entries(rx: &Receiver<Seen>, wanted: usize) -> (Vec<Seen>, Vec<LogEntry>) {
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    let mut posts = Vec::new();
    let mut entries = Vec::new();
    while entries.len() < wanted {
        let left = deadline.saturating_duration_since(std::time::Instant::now());
        assert!(
            !left.is_zero(),
            "only {} of {wanted} entries arrived in 30s: {entries:#?}",
            entries.len()
        );
        match rx.recv_timeout(left) {
            Ok(seen) if seen.method == "POST" => {
                let batch: LogBatch =
                    serde_json::from_str(&seen.body).expect("the body is a LogBatch");
                entries.extend(batch.entries);
                posts.push(seen);
            }
            Ok(_) => continue, // the health probe
            Err(e) => panic!("the stub server went quiet: {e}"),
        }
    }
    (posts, entries)
}

/// A redacted Linux path, home segment replaced.
///
/// The fixtures spell these out whole rather than building them with
/// `Path::join`, so the separators are `/` on every platform and the
/// expectation can be too, the redaction only rewrites the profile segment
/// and leaves the rest of the string alone. Deriving the separator from the
/// host here would put a `\` in the middle of a path the fixture never wrote
/// that way, and fail on Windows.
fn under_home(tail: &str) -> String {
    format!("/home/<user>/{tail}")
}

/// Los campos de una desmentida, por veredicto.
fn verdict<'a>(entries: &'a [LogEntry], name: &str) -> &'a serde_json::Value {
    entries
        .iter()
        .find(|e| {
            e.target.as_deref() == Some(TELEMETRY_TARGET)
                && e.fields.as_ref().and_then(|f| f.get("verdict"))
                    == Some(&serde_json::Value::String(name.to_string()))
        })
        .unwrap_or_else(|| panic!("the `{name}` verdict never arrived: {entries:#?}"))
        .fields
        .as_ref()
        .expect("fields")
}

#[test]
fn a_batch_actually_reaches_the_server_redacted() {
    let home = tempfile::tempdir().expect("tempdir");
    // Prefs and state come out of `XDG_DATA_HOME`: without this the test would read
    // (and the shipper would obey) the real prefs of whoever runs it.
    std::env::set_var("XDG_DATA_HOME", home.path());
    std::env::set_var("XDG_CONFIG_HOME", home.path());

    let prefs = Prefs::default();
    assert!(
        prefs.anonymous_telemetry,
        "shipping is on out of the box; if that changes, this test has to turn it \
         on by hand instead of assuming the default"
    );
    let prefs_path = Prefs::default_path().expect("prefs path");
    prefs.save(&prefs_path).expect("write prefs");

    let (base_url, rx) = spawn_stub();

    // The slot the bug never looked at. It is set BEFORE the layer starts so the
    // thread's first pass already finds a session and does not wait out the backoff.
    credentials::set_lent_cloud(Some(CloudLease {
        url: base_url,
        token: "test-jwt".to_string(),
    }));

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new("info"))
        .with(hoard_agent::logship::start())
        .init();

    // 1. A WARN with a Windows path exactly as `{:?}` renders it (with the slashes
    //    escaped), which is the shape that slips through most easily.
    tracing::warn!(
        path = ?"C:\\Users\\angel\\AppData\\LocalLow\\TheGameBakers\\Furi",
        "agent: refusing to back up this save"
    );
    // 2. An operational INFO: below Cloud's minimum, so it must not travel.
    tracing::info!(target: "hoard_agent::agent", "agent: backup committed");
    // 3. The five verdicts: they are INFO, but they travel through their `target`.
    // Linux-layout paths written out in full, with no `join`: on Windows the native
    // separator is `\\`, so joining a Unix literal produces
    // `/home/angel\\.local/share/Furi` and the test would end up checking which path
    // the runner passed rather than what redaction did with it. The Windows shape has
    // its own case, a few lines above.
    let home = std::path::Path::new("/home/angel");
    let p = |tail: &str| std::path::PathBuf::from(format!("/home/angel/{tail}"));
    hoard_agent::telemetry::repointed(
        "furi",
        &p(".local/share/Furi"),
        &p(".steam/steam/steamapps/compatdata/1052500"),
    );
    hoard_agent::telemetry::manual_path("planet-s", &p("Saved Games/Planet S"));
    hoard_agent::telemetry::untracked("dispatch", &p(".local/share/Dispatch"));
    hoard_agent::telemetry::no_snapshots("v-rising", &p(".local/share/VRising"));
    hoard_agent::telemetry::rejected_root("stellaris", home, "the user profile root");

    // 1 WARN + 5 desmentidas; el INFO operativo no debe aparecer.
    let (posts, entries) = collect_entries(&rx, 6);

    for post in &posts {
        assert_eq!(
            post.path, "/v1/cloud/logs",
            "con `mode: cloud` el lote va al namespace de Cloud"
        );
        assert_eq!(
            post.authorization.as_deref(),
            Some("Bearer test-jwt"),
            "the batch travels with the token from the Cloud slot"
        );
        assert!(
            !post.body.contains("angel"),
            "the person's name came out in the batch: {}",
            post.body
        );
    }

    assert!(
        entries
            .iter()
            .any(|e| e.level == "warn" && e.message.contains("refusing to back up")),
        "el WARN operativo tiene que entrar"
    );
    assert!(
        !entries
            .iter()
            .any(|e| e.message.contains("backup committed")),
        "el INFO operativo tiene que quedarse fuera: {entries:#?}"
    );

    // The field contract, verdict by verdict. The panel paints columns by field
    // name: `path` is "from where" and `to` is "to where" in EVERY one of them, or a
    // column ends up showing the good value under a heading that says "bad path".
    let repointed = verdict(&entries, "repointed");
    assert_eq!(repointed["slug"], "furi");
    assert_eq!(repointed["path"], under_home(".local/share/Furi"));
    assert_eq!(
        repointed["to"],
        under_home(".steam/steam/steamapps/compatdata/1052500")
    );

    let manual = verdict(&entries, "manual_path");
    assert_eq!(manual["slug"], "planet-s");
    assert_eq!(manual["to"], under_home("Saved Games/Planet S"));
    assert!(
        manual.get("path").is_none(),
        "the folder the user chose is the destination, not the bad path: {manual}"
    );

    let untracked = verdict(&entries, "untracked");
    assert_eq!(untracked["slug"], "dispatch");
    assert_eq!(untracked["path"], under_home(".local/share/Dispatch"));

    let never = verdict(&entries, "no_snapshots");
    assert_eq!(never["slug"], "v-rising");
    assert_eq!(never["path"], under_home(".local/share/VRising"));

    let rejected = verdict(&entries, "rejected_root");
    assert_eq!(rejected["slug"], "stellaris");
    assert_eq!(rejected["path"], "/home/<user>");
    assert_eq!(rejected["reason"], "the user profile root");

    // Signing out empties the slot: the JWT is an in-memory copy and deleting the
    // session file does not touch it, so without this the process would keep shipping
    // under the account that was just closed.
    assert!(credentials::lent_cloud().is_some());
    hoard_agent::cloud_auth::forget_tokens_unlocked().expect("service-less logout");
    assert!(
        credentials::lent_cloud().is_none(),
        "the service-less logout left the token in place"
    );

    // The other logout, `clear_session`, does the same thing with an identical line,
    // but it is **deliberately not called here**: on top of the file it deletes the
    // keyring item, and the keyring belongs to the system, not to the `XDG_DATA_HOME`
    // this test redirects. A test that called it would wipe the developer's real Cloud
    // session every time the suite ran.
}
