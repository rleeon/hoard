//! Hoard-help: a report or an idea, with whatever files the person adds, sent to
//! Hoard Cloud (`/v1/feedback` on the server).
//!
//! It always goes to Hoard Cloud, whatever server this install syncs with. That
//! is the point of it: a self-hoster's own server is not who needs to read
//! "the restore button does nothing". It is also the only thing a self-hosted
//! install ever sends to us, and only when the person presses send.
//!
//! The logs, when asked for, are the tail of `agent.log` and `hoardd.log` with
//! the profile folder in every path replaced the same way log shipping does it
//! (`/home/<user>/...`). The app shows that before anything is sent.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures::TryStreamExt;
use hoard_core::wire::{
    FeedbackCreate, FeedbackCreated, FeedbackFileDecl, FeedbackKind, FEEDBACK_MAX_FILES,
    FEEDBACK_MAX_FILE_BYTES, FEEDBACK_MAX_MESSAGE_CHARS, FEEDBACK_MAX_REPORT_BYTES,
    FEEDBACK_TOKEN_HEADER,
};
use tokio::io::{AsyncReadExt, AsyncSeekExt};

use crate::config::CliConfig;

/// Per log file. A day of a busy `hoardd.log` is a few MB; the last one is the
/// part anybody reads.
const LOG_TAIL_BYTES: u64 = 1024 * 1024;

pub const LOGS_FILE_NAME: &str = "hoard-logs.txt";

pub struct Draft {
    pub kind: FeedbackKind,
    pub message: String,
    pub contact: Option<String>,
    pub app_version: Option<String>,
    /// `cloud`, `selfhosted` or `none`.
    pub mode: Option<String>,
    pub files: Vec<PathBuf>,
    pub attach_logs: bool,
}

#[derive(Debug)]
pub enum FeedbackError {
    Empty,
    MessageTooLong,
    TooManyFiles,
    FileTooLarge(String),
    TooLarge,
    Unreadable(String),
    /// The server's throttle, with how long it asked us to wait.
    Throttled(u64),
    Http {
        status: u16,
        body: String,
    },
    Network(String),
}

impl std::fmt::Display for FeedbackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FeedbackError::Empty => f.write_str("write something first"),
            FeedbackError::MessageTooLong => write!(
                f,
                "the message is over {FEEDBACK_MAX_MESSAGE_CHARS} characters"
            ),
            FeedbackError::TooManyFiles => write!(f, "at most {FEEDBACK_MAX_FILES} files"),
            FeedbackError::FileTooLarge(n) => write!(
                f,
                "{n} is over {} MB",
                FEEDBACK_MAX_FILE_BYTES / 1024 / 1024
            ),
            FeedbackError::TooLarge => write!(
                f,
                "the files add up to more than {} MB",
                FEEDBACK_MAX_REPORT_BYTES / 1024 / 1024
            ),
            FeedbackError::Unreadable(n) => write!(f, "{n} could not be read"),
            FeedbackError::Throttled(s) => write!(f, "too many reports, try again in {s} s"),
            FeedbackError::Http { status, body } => {
                write!(f, "Hoard Cloud answered {status}: {body}")
            }
            FeedbackError::Network(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for FeedbackError {}

/// A file as it will be sent: where it is read from and the name it goes under.
#[derive(Debug, Clone)]
struct Outgoing {
    path: PathBuf,
    name: String,
    size: u64,
}

/// Sends `draft` and returns the report id. `progress(sent, total)` is called
/// as file bytes go out.
///
/// `token` is the Cloud session's, when there is one, so the report arrives
/// with the account attached. The server ignores one that does not verify, so
/// an expired session is not a reason for this to fail.
pub async fn send(
    base: &str,
    token: Option<&str>,
    draft: Draft,
    progress: Arc<dyn Fn(u64, u64) + Send + Sync>,
) -> Result<String, FeedbackError> {
    let message = draft.message.trim().to_string();
    if message.is_empty() {
        return Err(FeedbackError::Empty);
    }
    if message.chars().count() > FEEDBACK_MAX_MESSAGE_CHARS {
        return Err(FeedbackError::MessageTooLong);
    }

    let mut files = Vec::new();
    for path in &draft.files {
        files.push(describe(path)?);
    }
    // The logs file lives in a temp dir that goes away with this function.
    let logs_dir = if draft.attach_logs {
        let dir = tempfile::tempdir().map_err(|e| FeedbackError::Network(e.to_string()))?;
        let path = dir.path().join(LOGS_FILE_NAME);
        tokio::fs::write(&path, logs_excerpt().await)
            .await
            .map_err(|_| FeedbackError::Unreadable(LOGS_FILE_NAME.into()))?;
        files.push(describe(&path)?);
        Some(dir)
    } else {
        None
    };
    check_limits(&files)?;

    let http = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(20))
        .read_timeout(Duration::from_secs(90))
        .user_agent(concat!("hoard-agent/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| FeedbackError::Network(e.to_string()))?;
    let base = base.trim_end_matches('/');

    let body = FeedbackCreate {
        kind: draft.kind,
        message,
        contact: draft.contact,
        app_version: draft.app_version,
        os: Some(os_label()),
        arch: Some(std::env::consts::ARCH.to_string()),
        mode: draft.mode,
        files: files
            .iter()
            .map(|f| FeedbackFileDecl {
                name: f.name.clone(),
                size: f.size,
            })
            .collect(),
    };
    let mut req = http.post(format!("{base}/v1/feedback")).json(&body);
    if let Some(t) = token {
        req = req.bearer_auth(t);
    }
    let resp = req.send().await.map_err(network)?;
    let created: FeedbackCreated = ok(resp)
        .await?
        .json()
        .await
        .map_err(|e| FeedbackError::Network(e.to_string()))?;

    let total: u64 = files.iter().map(|f| f.size).sum();
    let sent = Arc::new(AtomicU64::new(0));
    progress(0, total);
    for (idx, file) in files.iter().enumerate() {
        let before = sent.load(Ordering::Relaxed);
        let mut attempt = 0;
        loop {
            attempt += 1;
            sent.store(before, Ordering::Relaxed);
            let url = format!("{base}/v1/feedback/{}/files/{idx}", created.id);
            match put_file(
                &http,
                &url,
                &created.upload_token,
                file,
                &sent,
                total,
                &progress,
            )
            .await
            {
                Ok(()) => break,
                Err(Attempt::Retry(secs)) if attempt < 4 => {
                    tokio::time::sleep(Duration::from_secs(secs.min(60))).await;
                }
                Err(Attempt::Retry(_)) => {
                    return Err(FeedbackError::Network("the upload kept failing".into()));
                }
                Err(Attempt::Fail(e)) => return Err(e),
            }
        }
    }

    let resp = http
        .post(format!("{base}/v1/feedback/{}/complete", created.id))
        .header(FEEDBACK_TOKEN_HEADER, &created.upload_token)
        .send()
        .await
        .map_err(network)?;
    ok(resp).await?;
    drop(logs_dir);
    Ok(created.id)
}

enum Attempt {
    /// Worth another go after this many seconds: the connection dropped, the
    /// server was busy or failed.
    Retry(u64),
    Fail(FeedbackError),
}

async fn put_file(
    http: &reqwest::Client,
    url: &str,
    token: &str,
    file: &Outgoing,
    sent: &Arc<AtomicU64>,
    total: u64,
    progress: &Arc<dyn Fn(u64, u64) + Send + Sync>,
) -> Result<(), Attempt> {
    let f = tokio::fs::File::open(&file.path)
        .await
        .map_err(|_| Attempt::Fail(FeedbackError::Unreadable(file.name.clone())))?;
    let counter = sent.clone();
    let report = progress.clone();
    let stream = tokio_util::io::ReaderStream::new(f).inspect_ok(move |chunk| {
        let now = counter.fetch_add(chunk.len() as u64, Ordering::Relaxed) + chunk.len() as u64;
        report(now, total);
    });
    // Slow uplinks exist: a minute of grace plus 32 KB/s for the body.
    let budget = Duration::from_secs(60 + file.size / (32 * 1024));
    let resp = http
        .put(url)
        .header(FEEDBACK_TOKEN_HEADER, token)
        .header(reqwest::header::CONTENT_LENGTH, file.size)
        .timeout(budget)
        .body(reqwest::Body::wrap_stream(stream))
        .send()
        .await;
    let resp = match resp {
        Ok(r) => r,
        Err(_) => return Err(Attempt::Retry(5)),
    };
    let status = resp.status();
    if status.is_success() {
        return Ok(());
    }
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err(Attempt::Retry(retry_after(&resp).unwrap_or(30)));
    }
    if status.is_server_error() {
        return Err(Attempt::Retry(10));
    }
    let body = resp.text().await.unwrap_or_default();
    Err(Attempt::Fail(FeedbackError::Http {
        status: status.as_u16(),
        body,
    }))
}

fn retry_after(resp: &reqwest::Response) -> Option<u64> {
    resp.headers()
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse()
        .ok()
}

fn network(e: reqwest::Error) -> FeedbackError {
    FeedbackError::Network(e.to_string())
}

async fn ok(resp: reqwest::Response) -> Result<reqwest::Response, FeedbackError> {
    let status = resp.status();
    if status.is_success() {
        return Ok(resp);
    }
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err(FeedbackError::Throttled(retry_after(&resp).unwrap_or(3600)));
    }
    Err(FeedbackError::Http {
        status: status.as_u16(),
        body: resp.text().await.unwrap_or_default(),
    })
}

fn describe(path: &Path) -> Result<Outgoing, FeedbackError> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".into());
    let meta = std::fs::metadata(path).map_err(|_| FeedbackError::Unreadable(name.clone()))?;
    if !meta.is_file() {
        return Err(FeedbackError::Unreadable(name));
    }
    Ok(Outgoing {
        path: path.to_path_buf(),
        name,
        size: meta.len(),
    })
}

fn check_limits(files: &[Outgoing]) -> Result<(), FeedbackError> {
    if files.len() > FEEDBACK_MAX_FILES {
        return Err(FeedbackError::TooManyFiles);
    }
    if let Some(f) = files.iter().find(|f| f.size > FEEDBACK_MAX_FILE_BYTES) {
        return Err(FeedbackError::FileTooLarge(f.name.clone()));
    }
    if files.iter().map(|f| f.size).sum::<u64>() > FEEDBACK_MAX_REPORT_BYTES {
        return Err(FeedbackError::TooLarge);
    }
    Ok(())
}

/// "Windows 11 (26100)", "Linux 24.04 Ubuntu": more use than `windows`/`linux`
/// when the bug is a Steam Deck or one distro's packaging.
fn os_label() -> String {
    sysinfo::System::long_os_version().unwrap_or_else(|| std::env::consts::OS.to_string())
}

/// What the "include logs" box attaches, also shown to the person before
/// sending so they can see what it is.
pub async fn logs_excerpt() -> String {
    let Ok(dir) = CliConfig::logs_dir() else {
        return String::new();
    };
    let mut out = String::new();
    for prefix in ["hoardd.log", "agent.log"] {
        let Some(path) = newest(&dir, prefix) else {
            continue;
        };
        let Ok(tail) = tail(&path, LOG_TAIL_BYTES).await else {
            continue;
        };
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        out.push_str(&format!("# {name}\n"));
        for line in tail.lines() {
            out.push_str(&crate::logship::redact(&strip_ansi(line)));
            out.push('\n');
        }
        out.push('\n');
    }
    out
}

/// `hoardd.log` is written by the same formatter as stderr, colours included,
/// and `\x1b[2m` in a text file is noise to whoever opens it.
fn strip_ansi(line: &str) -> std::borrow::Cow<'_, str> {
    if !line.contains('\x1b') {
        return line.into();
    }
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // CSI: `ESC [`, parameters, one final letter.
            if chars.next() == Some('[') {
                for c in chars.by_ref() {
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(c);
    }
    out.into()
}

/// The most recently written file of a set. By modification time, not by name:
/// `hoardd.log` never rolls, and an old dated sibling from before it stopped
/// rolling sorts after it.
fn newest(dir: &Path, prefix: &str) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with(prefix))
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .max_by_key(|(t, _)| *t)
        .map(|(_, p)| p)
}

/// The last `max` bytes, starting at a line boundary.
async fn tail(path: &Path, max: u64) -> std::io::Result<String> {
    let mut f = tokio::fs::File::open(path).await?;
    let len = f.metadata().await?.len();
    let start = len.saturating_sub(max);
    f.seek(std::io::SeekFrom::Start(start)).await?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).await?;
    let text = String::from_utf8_lossy(&buf);
    Ok(match (start > 0, text.find('\n')) {
        (true, Some(i)) => text[i + 1..].to_string(),
        _ => text.into_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(name: &str, size: u64) -> Outgoing {
        Outgoing {
            path: PathBuf::from(name),
            name: name.into(),
            size,
        }
    }

    #[test]
    fn limits_name_the_file_that_breaks_them() {
        assert!(check_limits(&[file("a", 10)]).is_ok());
        match check_limits(&[file("a", 10), file("clip.mp4", FEEDBACK_MAX_FILE_BYTES + 1)]) {
            Err(FeedbackError::FileTooLarge(n)) => assert_eq!(n, "clip.mp4"),
            other => panic!("{other:?}"),
        }
        let three = vec![file("x", FEEDBACK_MAX_FILE_BYTES); 3];
        assert!(matches!(check_limits(&three), Err(FeedbackError::TooLarge)));
        let many = vec![file("x", 1); FEEDBACK_MAX_FILES + 1];
        assert!(matches!(
            check_limits(&many),
            Err(FeedbackError::TooManyFiles)
        ));
    }

    #[test]
    fn colours_are_stripped() {
        let raw = "\x1b[2m2026-09-29T10:16:03Z\x1b[0m \x1b[32m INFO\x1b[0m hoardd: ok";
        assert_eq!(strip_ansi(raw), "2026-09-29T10:16:03Z  INFO hoardd: ok");
        assert_eq!(strip_ansi("plain"), "plain");
    }

    #[tokio::test]
    async fn the_tail_starts_on_a_whole_line() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("hoardd.log");
        std::fs::write(&p, "first line\nsecond line\nthird\n").unwrap();
        assert_eq!(
            tail(&p, 1024).await.unwrap(),
            "first line\nsecond line\nthird\n"
        );
        assert_eq!(tail(&p, 15).await.unwrap(), "third\n");
    }

    #[test]
    fn newest_goes_by_time_not_by_name() {
        let dir = tempfile::tempdir().unwrap();
        let old = dir.path().join("hoardd.log.2026-01-01");
        std::fs::write(&old, "old").unwrap();
        let f = std::fs::File::options().write(true).open(&old).unwrap();
        f.set_modified(std::time::SystemTime::now() - Duration::from_secs(86_400))
            .unwrap();
        std::fs::write(dir.path().join("hoardd.log"), "new").unwrap();
        assert_eq!(
            newest(dir.path(), "hoardd.log").unwrap(),
            dir.path().join("hoardd.log")
        );
    }
}
