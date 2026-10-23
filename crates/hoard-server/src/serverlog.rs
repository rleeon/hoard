//! The server's own log, kept on disk so it can be read back.
//!
//! A self-hosted server logs to stdout, which is `docker logs` or the journal:
//! fine for whoever is at the console, out of reach for the app. When somebody
//! sends a Hoard-help report from a self-hosted install, the server's side of
//! the story was missing: the Bazzite report of 22-10-2026 had a
//! `cas commit: server error (500)` on 19-10 that nothing could explain without
//! that server's log of that minute.
//!
//! So a self-hosted server also writes plain text (no colours, whatever
//! `logging.format` says for stdout) into `data_dir/logs`, one file per day,
//! a week of them. `GET /v1/admin/server-log` hands the tail to an admin.

use std::path::{Path, PathBuf};

use tracing_appender::rolling::{RollingFileAppender, Rotation};

pub const LOG_DIR: &str = "logs";
const PREFIX: &str = "server";
const SUFFIX: &str = "log";
const KEEP_DAYS: usize = 7;

/// What the admin route returns at most. Two days of a family server fit with
/// room to spare; a busy one gets its most recent hours, which is the part a
/// report is about.
pub const TAIL_BYTES: u64 = 2 * 1024 * 1024;

/// The daily file writer, or `None` when `data_dir/logs` can't be created. A
/// server that can't keep its log still has to start.
pub fn appender(data_dir: &Path) -> Option<RollingFileAppender> {
    let dir = data_dir.join(LOG_DIR);
    std::fs::create_dir_all(&dir).ok()?;
    RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix(PREFIX)
        .filename_suffix(SUFFIX)
        .max_log_files(KEEP_DAYS)
        .build(&dir)
        .ok()
}

/// The last `max` bytes across the daily files, oldest first, starting on a
/// whole line. Empty when there are none yet.
pub async fn tail(data_dir: &Path, max: u64) -> std::io::Result<String> {
    let files = log_files(&data_dir.join(LOG_DIR))?;
    // Newest first, so the budget goes to the most recent lines.
    let mut parts: Vec<Vec<u8>> = Vec::new();
    let mut left = max;
    for path in files.iter().rev() {
        if left == 0 {
            break;
        }
        let bytes = tokio::fs::read(path).await?;
        let len = bytes.len() as u64;
        if len <= left {
            left -= len;
            parts.push(bytes);
        } else {
            let start = (len - left) as usize;
            let cut = &bytes[start..];
            let cut = if bytes[start - 1] == b'\n' {
                cut
            } else {
                match cut.iter().position(|b| *b == b'\n') {
                    Some(i) => &cut[i + 1..],
                    None => &[],
                }
            };
            parts.push(cut.to_vec());
            left = 0;
        }
    }
    let mut out = String::new();
    for part in parts.iter().rev() {
        out.push_str(&String::from_utf8_lossy(part));
    }
    Ok(out)
}

/// `server.2026-10-22.log` and its siblings, oldest first. The date in the name
/// sorts the same as the date it was written.
fn log_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
                    n.starts_with(&format!("{PREFIX}.")) && n.ends_with(&format!(".{SUFFIX}"))
                })
        })
        .collect();
    files.sort();
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, day: &str, text: &str) {
        let logs = dir.join(LOG_DIR);
        std::fs::create_dir_all(&logs).unwrap();
        std::fs::write(logs.join(format!("server.{day}.log")), text).unwrap();
    }

    #[tokio::test]
    async fn the_tail_spans_days_oldest_first() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "2026-10-21", "a1\na2\n");
        write(dir.path(), "2026-10-22", "b1\nb2\n");
        std::fs::write(dir.path().join(LOG_DIR).join("other.txt"), "no").unwrap();
        assert_eq!(tail(dir.path(), 1024).await.unwrap(), "a1\na2\nb1\nb2\n");
    }

    #[tokio::test]
    async fn a_short_budget_keeps_the_newest_whole_lines() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "2026-10-21", "old line\n");
        write(dir.path(), "2026-10-22", "first\nsecond\nthird\n");
        assert_eq!(tail(dir.path(), 9).await.unwrap(), "third\n");
        // A budget that ends exactly on a line start keeps that line.
        assert_eq!(tail(dir.path(), 6).await.unwrap(), "third\n");
        assert_eq!(tail(dir.path(), 0).await.unwrap(), "");
    }

    #[tokio::test]
    async fn no_log_dir_is_an_empty_log() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(tail(dir.path(), 1024).await.unwrap(), "");
    }
}
