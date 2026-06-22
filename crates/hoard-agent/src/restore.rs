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

/// Hard ceiling on the total bytes a single restore may write to disk —
/// defense-in-depth against a decompression bomb: a tiny `.tar.zst` that
/// expands to terabytes. Used as-is when the expanded size isn't known ahead of
/// time (the legacy whole-archive cloud path) and as an upper clamp otherwise.
const MAX_RESTORE_BYTES: u64 = 64 * 1024 * 1024 * 1024; // 64 GiB

/// Slack factor over the manifest-declared expanded size: enough room for
/// unlisted sidecar files while still bounding a bomb to ~2× the real payload.
const RESTORE_SIZE_SLACK: u64 = 2;

/// Per-restore decompression cap derived from the declared expanded size (sum
/// of manifest file sizes) when known, clamped to `[FLOOR, MAX_RESTORE_BYTES]`.
/// `None` (size unknown) falls back to the absolute ceiling.
fn restore_byte_cap(declared_expanded: Option<u64>) -> u64 {
    // Floor so tiny saves tolerate minor overhead without nuisance failures.
    const FLOOR: u64 = 256 * 1024 * 1024; // 256 MiB
    match declared_expanded {
        Some(n) if n > 0 => n
            .saturating_mul(RESTORE_SIZE_SLACK)
            .max(FLOOR)
            .min(MAX_RESTORE_BYTES),
        _ => MAX_RESTORE_BYTES,
    }
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
    if client.is_cloud().await {
        // Cloud has no `get_save`; the manifest carries each save's latest
        // version. A missing entry means nothing has been uploaded yet.
        let manifest = client.cloud_sync().await?;
        return manifest
            .saves
            .into_iter()
            .find(|e| e.save_id == save_id)
            .map(|e| e.latest_version_num)
            .ok_or_else(|| anyhow!("save has no snapshots yet"));
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
    if client.is_cloud().await {
        return download_snapshot_cloud(client, save_id, version, dest, options, progress).await;
    }
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

    // Decompression-bomb guard: cap total bytes written against the manifest's
    // declared expanded size (×slack), falling back to the absolute ceiling.
    let declared: u64 = detail
        .files
        .iter()
        .map(|f| f.size_bytes.max(0) as u64)
        .sum();
    let cap = restore_byte_cap(Some(declared));

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
            if bytes_extracted + written > cap {
                bail!(
                    "restore aborted: decompressed output exceeds the {cap}-byte limit \
                     (possible archive bomb)"
                );
            }
        }
        out.flush()
            .await
            .with_context(|| format!("writing {}", dest_path.display()))?;
        drop(out);
        apply_entry_mtime(&entry, &dest_path);

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

/// Hoard Cloud download: presigned R2 GET → temp tar.zst → verify whole-
/// archive sha256 → extract.
///
/// The cloud server stores one opaque `.tar.zst` per version and exposes no
/// per-file manifest (`snapshot_detail` doesn't exist there), so verification
/// is over the whole archive's sha256 — recorded at commit time and returned
/// in `DownloadOut` — rather than per-file. We download to a temp file first
/// so the hash check happens before we touch the destination.
async fn download_snapshot_cloud<F>(
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
    // New uploads are content-addressed: pull the per-file manifest (with
    // presigned GETs) and download each blob. Legacy archive versions report
    // `content_addressed = false` and fall through to the whole-archive path.
    let manifest = client
        .cloud_version_manifest(save_id, version, true)
        .await?;
    if manifest.content_addressed {
        return restore_cloud_cas(client, dest, options, manifest, progress).await;
    }

    let meta = client.cloud_download(save_id, version).await?;

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

    // 1. Stream the archive to a temp file, hashing as we go.
    let suffix = format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    let tmp = std::env::temp_dir().join(format!("hoard-download-{suffix}.tar.zst"));

    let result = download_and_extract_cloud(client, &meta, dest, &tmp, &options, progress).await;
    let _ = tokio::fs::remove_file(&tmp).await;
    result
}

/// Content-addressed restore: each file in the manifest is its own R2 blob.
/// Stream each to its destination path, verifying the whole-file sha256 and
/// preserving the recorded mtime (so a cloud pull doesn't always win the
/// conflict-aware diff). No temp archive — files land directly.
async fn restore_cloud_cas<F>(
    client: &ApiClient,
    dest: &Path,
    options: RestoreOptions,
    manifest: crate::api::CloudVersionManifestOut,
    progress: F,
) -> Result<RestoreOutcome>
where
    F: Fn(u64, u64) + Send + Sync + 'static,
{
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

    let total: u64 = manifest
        .files
        .iter()
        .map(|f| f.size_bytes.max(0) as u64)
        .sum();
    let mut downloaded = 0u64;
    let mut files_extracted = 0usize;
    let mut bytes_extracted = 0u64;
    progress(0, total);

    for file in &manifest.files {
        let safe_rel = sanitize(Path::new(&file.relative_path))
            .ok_or_else(|| anyhow!("unsafe path in manifest: {}", file.relative_path))?;
        let dest_path = dest.join(&safe_rel);
        if let Some(parent) = dest_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("creating parent {}", parent.display()))?;
        }

        let presigned = file
            .download
            .as_ref()
            .ok_or_else(|| anyhow!("manifest missing download URL for {}", file.relative_path))?;
        let resp = client.get_presigned(presigned).await?;

        let mut out = tokio::fs::File::create(&dest_path)
            .await
            .with_context(|| format!("writing {}", dest_path.display()))?;
        let mut hasher = Sha256::new();
        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("downloading blob")?;
            if !options.skip_verify {
                hasher.update(&chunk);
            }
            out.write_all(&chunk)
                .await
                .with_context(|| format!("writing {}", dest_path.display()))?;
            downloaded += chunk.len() as u64;
            progress(downloaded, total);
        }
        out.flush().await.context("flushing file")?;
        drop(out);

        if !options.skip_verify {
            let got = hex::encode(hasher.finalize());
            if got != file.sha256 {
                bail!(
                    "sha256 mismatch for {}: expected {}, got {}",
                    file.relative_path,
                    file.sha256,
                    got
                );
            }
        }

        // Preserve the recorded mtime — without it every cloud pull would look
        // strictly newer than the local copy and silently win the auto-restore
        // diff. Best-effort: a failure only degrades conflict resolution.
        if let Some(secs) = file.modified_at {
            if secs > 0 {
                let ft = filetime::FileTime::from_unix_time(secs, 0);
                let _ = filetime::set_file_mtime(&dest_path, ft);
            }
        }

        bytes_extracted += file.size_bytes.max(0) as u64;
        files_extracted += 1;
    }

    Ok(RestoreOutcome {
        files_extracted,
        bytes_extracted,
        destination: dest.to_path_buf(),
    })
}

async fn download_and_extract_cloud<F>(
    client: &ApiClient,
    meta: &crate::api::CloudDownloadOut,
    dest: &Path,
    tmp: &Path,
    options: &RestoreOptions,
    progress: F,
) -> Result<RestoreOutcome>
where
    F: Fn(u64, u64) + Send + Sync + 'static,
{
    let resp = client.get_presigned(&meta.download).await?;
    let total = resp
        .content_length()
        .unwrap_or_else(|| meta.size_bytes.max(0) as u64);

    let mut out = tokio::fs::File::create(tmp)
        .await
        .with_context(|| format!("creating temp archive {}", tmp.display()))?;
    let mut hasher = Sha256::new();
    let mut downloaded = 0u64;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("downloading archive")?;
        if !options.skip_verify {
            hasher.update(&chunk);
        }
        out.write_all(&chunk)
            .await
            .with_context(|| format!("writing {}", tmp.display()))?;
        downloaded += chunk.len() as u64;
        progress(downloaded, total);
    }
    out.flush().await.context("flushing temp archive")?;
    drop(out);

    if !options.skip_verify {
        let got = hex::encode(hasher.finalize());
        if got != meta.sha256 {
            bail!(
                "sha256 mismatch for v{}: expected {}, got {}",
                meta.version_num,
                meta.sha256,
                got
            );
        }
    }

    // 2. Decode + extract from the verified temp file.
    let file = tokio::fs::File::open(tmp)
        .await
        .with_context(|| format!("opening {}", tmp.display()))?;
    let zstd = ZstdDecoder::new(BufReader::new(file));
    let zstd = BufReader::new(zstd);
    let mut archive = tokio_tar::Archive::new(zstd);

    // Decompression-bomb guard. This legacy path has no per-file manifest and
    // `meta.size_bytes` is the *compressed* archive size, so the expanded size
    // is unknown — fall back to the absolute ceiling.
    let cap = restore_byte_cap(None);

    let mut entries = archive.entries().context("opening tar archive")?;
    let mut files_extracted = 0usize;
    let mut bytes_extracted = 0u64;

    while let Some(entry) = entries.next().await {
        let mut entry = entry.context("reading tar entry")?;
        let path_in_tar = entry.path()?.into_owned();

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

        let mut writer = tokio::fs::File::create(&dest_path)
            .await
            .with_context(|| format!("writing {}", dest_path.display()))?;
        // Cap the copy at the remaining budget +1 so an over-long entry is
        // caught instead of streamed to disk in full.
        let remaining = cap.saturating_sub(bytes_extracted);
        let written = tokio::io::copy(&mut (&mut entry).take(remaining + 1), &mut writer)
            .await
            .with_context(|| format!("extracting {}", dest_path.display()))?;
        if bytes_extracted + written > cap {
            bail!(
                "restore aborted: decompressed output exceeds the {cap}-byte limit \
                 (possible archive bomb)"
            );
        }
        writer
            .flush()
            .await
            .with_context(|| format!("writing {}", dest_path.display()))?;
        drop(writer);
        apply_entry_mtime(&entry, &dest_path);

        bytes_extracted += written;
        files_extracted += 1;
    }

    Ok(RestoreOutcome {
        files_extracted,
        bytes_extracted,
        destination: dest.to_path_buf(),
    })
}

/// Hoard Cloud: list the files inside a version's blob (relative path + size)
/// without extracting anything to disk. The cloud stores one opaque `.tar.zst`
/// per version and keeps no per-file index, so the History detail view streams
/// the blob through the zstd + tar decoders and reads just the entry headers.
/// File bodies are skipped (`tokio_tar` seeks past them) and nothing touches
/// the filesystem, so this stays cheap even for large saves. `sha256` is left
/// empty — the tar header doesn't carry it and the detail view doesn't show it.
pub async fn list_cloud_version_files(
    client: &ApiClient,
    save_id: &str,
    version: i64,
) -> Result<Vec<SnapshotFile>> {
    // Content-addressed versions keep a real per-file index server-side, so the
    // detail view is a single cheap call — no blob download, no bandwidth. SHAs
    // come back too. Legacy archive versions fall through to streaming the tar.
    let manifest = client
        .cloud_version_manifest(save_id, version, false)
        .await?;
    if manifest.content_addressed {
        let mut files: Vec<SnapshotFile> = manifest
            .files
            .into_iter()
            .map(|f| SnapshotFile {
                relative_path: f.relative_path,
                size_bytes: f.size_bytes,
                sha256: f.sha256,
            })
            .collect();
        files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
        return Ok(files);
    }

    let meta = client.cloud_download(save_id, version).await?;
    let resp = client.get_presigned(&meta.download).await?;

    // Adapt the byte stream into an AsyncRead so the archive flows straight
    // through the decoders without buffering the whole thing in memory.
    let reader = StreamReader::new(
        resp.bytes_stream()
            .map(|r| r.map_err(std::io::Error::other)),
    );
    let zstd = ZstdDecoder::new(BufReader::new(reader));
    let zstd = BufReader::new(zstd);
    let mut archive = tokio_tar::Archive::new(zstd);

    let mut entries = archive.entries().context("opening tar archive")?;
    let mut files = Vec::new();
    while let Some(entry) = entries.next().await {
        let entry = entry.context("reading tar entry")?;
        if entry.header().entry_type().is_dir() {
            continue;
        }
        let path_in_tar = entry.path()?.into_owned();
        let rel = match sanitize(&path_in_tar) {
            Some(p) => p.to_string_lossy().into_owned(),
            None => continue,
        };
        let size = entry.header().size().unwrap_or(0) as i64;
        files.push(SnapshotFile {
            relative_path: rel,
            size_bytes: size,
            sha256: String::new(),
        });
    }
    files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    Ok(files)
}

/// Re-apply the tar entry's recorded mtime onto the extracted file.
///
/// We write each file with `File::create`, which stamps it with mtime=now.
/// The conflict-aware auto-restore diff (`agent::local_mtime_wins`) compares
/// the freshly-pulled file's mtime against the local copy's, so without this
/// every cloud pull would look strictly newer than local and silently win —
/// exactly the "todos mis saves de la nube los puso más recientes" bug.
/// Best-effort: a failure here only degrades conflict resolution, never the
/// extraction itself, so errors are swallowed.
fn apply_entry_mtime<R>(entry: &tokio_tar::Entry<R>, path: &Path)
where
    R: tokio::io::AsyncRead + Unpin,
{
    if let Ok(secs) = entry.header().mtime() {
        if secs > 0 {
            let ft = filetime::FileTime::from_unix_time(secs as i64, 0);
            let _ = filetime::set_file_mtime(path, ft);
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cap_unknown_size_uses_absolute_ceiling() {
        assert_eq!(restore_byte_cap(None), MAX_RESTORE_BYTES);
        assert_eq!(restore_byte_cap(Some(0)), MAX_RESTORE_BYTES);
    }

    #[test]
    fn cap_tiny_save_gets_the_floor() {
        // A few KB declared → the floor still applies, not 2×KB.
        assert_eq!(restore_byte_cap(Some(4096)), 256 * 1024 * 1024);
    }

    #[test]
    fn cap_scales_with_declared_size() {
        let declared = 4 * 1024 * 1024 * 1024; // 4 GiB
        assert_eq!(
            restore_byte_cap(Some(declared)),
            declared * RESTORE_SIZE_SLACK
        );
    }

    #[test]
    fn cap_is_clamped_to_the_ceiling() {
        // 2 × 40 GiB = 80 GiB would exceed the 64 GiB hard cap.
        let declared = 40 * 1024 * 1024 * 1024;
        assert_eq!(restore_byte_cap(Some(declared)), MAX_RESTORE_BYTES);
    }
}
