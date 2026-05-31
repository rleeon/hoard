//! Library half of the snapshot download/extract flow.
//!
//! Streams the tar.zst from the server, decodes it, sanitises paths, verifies
//! each file's SHA-256 against the manifest, and writes them under a target
//! directory. The CLI and GUI share this code; presentation (progress bars,
//! confirmation dialogs) lives in their respective layers.

use anyhow::{anyhow, bail, Context, Result};
use async_compression::tokio::bufread::ZstdDecoder;
use futures::StreamExt;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio_util::io::StreamReader;

use crate::api::{ApiClient, SnapshotDetail, SnapshotFile};

/// Tunables for the restore flow.
#[derive(Debug, Clone, Default)]
pub struct RestoreOptions {
    /// Skip the per-file SHA-256 + size verification step.
    pub skip_verify: bool,
    /// Allow extracting into a non-empty destination directory.
    pub force: bool,
}

/// Result summary after a successful restore.
#[derive(Debug, Clone)]
pub struct RestoreOutcome {
    pub files_extracted: usize,
    pub bytes_extracted: u64,
    pub destination: PathBuf,
}

/// Resolve the snapshot version to use: the explicit one if supplied, else the
/// save's `latest_version_num`. Errors if the save has no snapshots yet.
pub async fn resolve_version(
    client: &ApiClient,
    save_id: &str,
    version: Option<i64>,
) -> Result<i64> {
    if let Some(v) = version {
        return Ok(v);
    }
    let save = client.get_save(save_id).await?;
    save.latest_version_num
        .ok_or_else(|| anyhow!("save has no snapshots yet"))
}

/// Stream-download snapshot `version` of `save_id` into `dest`.
///
/// `progress(downloaded, total_or_zero)` is called as bytes flow from the
/// server. `total_or_zero` is 0 if the server didn't send Content-Length.
pub async fn download_snapshot<F>(
    client: &ApiClient,
    save_id: &str,
    version: i64,
    dest: &Path,
    options: RestoreOptions,
    progress: F,
) -> Result<RestoreOutcome>
where
    F: Fn(u64, u64) + Send + Sync + 'static,
{
    let detail: SnapshotDetail = client.snapshot_detail(save_id, version).await?;
    let expected: HashMap<String, &SnapshotFile> = detail
        .files
        .iter()
        .map(|f| (f.relative_path.clone(), f))
        .collect();

    if dest.exists() {
        let empty = std::fs::read_dir(dest)?.next().is_none();
        if !empty && !options.force {
            bail!(
                "destination is not empty: {} (set force = true to extract anyway)",
                dest.display()
            );
        }
    } else {
        std::fs::create_dir_all(dest).with_context(|| format!("creating {}", dest.display()))?;
    }

    let resp = client.snapshot_download(save_id, version).await?;
    let total = resp.content_length().unwrap_or(0);

    let progress = std::sync::Arc::new(progress);
    let progress_for_stream = progress.clone();
    let downloaded = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let downloaded_for_stream = downloaded.clone();

    let byte_stream = resp.bytes_stream().map(move |chunk| {
        let chunk = chunk.map_err(std::io::Error::other)?;
        let new_total = downloaded_for_stream
            .fetch_add(chunk.len() as u64, std::sync::atomic::Ordering::Relaxed)
            + chunk.len() as u64;
        progress_for_stream(new_total, total);
        Ok::<_, std::io::Error>(chunk)
    });

    let reader = StreamReader::new(byte_stream);
    let buf = BufReader::new(reader);
    let zstd = ZstdDecoder::new(buf);
    let zstd = BufReader::new(zstd);
    let mut archive = tokio_tar::Archive::new(zstd);

    let mut entries = archive.entries().context("opening tar archive")?;
    let mut files_extracted = 0usize;
    let mut bytes_extracted = 0u64;

    while let Some(entry) = entries.next().await {
        let mut entry = entry.context("reading tar entry")?;
        let path_in_tar = entry.path()?.into_owned();

        // Sanitize: reject anything that escapes the destination. For
        // directory entries an empty result (e.g. "./") just means the
        // archive root — nothing to do.
        let safe_rel = match sanitize(&path_in_tar) {
            Some(p) => p,
            None if entry.header().entry_type().is_dir() => continue,
            None => bail!("unsafe path in archive: {}", path_in_tar.display()),
        };
        let dest_path = dest.join(&safe_rel);

        if entry.header().entry_type().is_dir() {
            tokio::fs::create_dir_all(&dest_path).await.ok();
            continue;
        }
        if let Some(parent) = dest_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("creating parent {}", parent.display()))?;
        }

        let key = safe_rel.to_string_lossy().replace('\\', "/");
        let expected_file = expected.get(&key);

        // Stream the entry to disk in fixed-size chunks while hashing, instead
        // of buffering the whole file in a Vec. A 2 GB file no longer means
        // 2 GB of RAM. The SHA-256 is computed incrementally over the same
        // bytes we write.
        let mut out = tokio::fs::File::create(&dest_path)
            .await
            .with_context(|| format!("writing {}", dest_path.display()))?;
        let mut hasher = Sha256::new();
        let mut written = 0u64;
        let mut buf = vec![0u8; 256 * 1024];
        loop {
            let n = entry
                .read(&mut buf)
                .await
                .with_context(|| format!("reading entry {key}"))?;
            if n == 0 {
                break;
            }
            if !options.skip_verify && expected_file.is_some() {
                hasher.update(&buf[..n]);
            }
            out.write_all(&buf[..n])
                .await
                .with_context(|| format!("writing {}", dest_path.display()))?;
            written += n as u64;
        }
        out.flush()
            .await
            .with_context(|| format!("writing {}", dest_path.display()))?;

        if !options.skip_verify {
            if let Some(meta) = expected_file {
                let got = hex::encode(hasher.finalize());
                if got != meta.sha256 {
                    bail!(
                        "sha256 mismatch for {key}: expected {}, got {}",
                        meta.sha256,
                        got
                    );
                }
                if (written as i64) != meta.size_bytes {
                    bail!(
                        "size mismatch for {key}: expected {}, got {}",
                        meta.size_bytes,
                        written
                    );
                }
            }
            // Files outside the manifest still get extracted but not verified.
        }

        bytes_extracted += written;
        files_extracted += 1;
    }

    Ok(RestoreOutcome {
        files_extracted,
        bytes_extracted,
        destination: dest.to_path_buf(),
    })
}

/// Reject absolute paths, `..`, drive prefixes. Returns a relative `PathBuf`
/// composed of only `Normal` components. Returns `None` if the path is empty
/// or the input was unsafe.
pub fn sanitize(p: &Path) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::Normal(s) => out.push(s),
            Component::CurDir => {}
            _ => return None,
        }
    }
    if out.as_os_str().is_empty() {
        None
    } else {
        Some(out)
    }
}
