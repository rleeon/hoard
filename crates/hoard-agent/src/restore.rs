//! Library half of the snapshot download/extract flow.
//!
//! Streams the tar.zst from the server, decodes it, sanitises paths, verifies
//! each file's SHA-256 against the manifest, and writes them under a target
//! directory. The CLI and GUI share this code; presentation (progress bars,
//! confirmation dialogs) lives in their respective layers.

use anyhow::{anyhow, bail, Context, Result};
use async_compression::tokio::bufread::ZstdDecoder;
use futures::{FutureExt, StreamExt, TryStreamExt};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio_util::io::StreamReader;

use crate::api::{ApiClient, SnapshotDetail, SnapshotFile};
use hoard_core::ids::Sha256 as Sha256Hex;
use hoard_core::kernel::fileclass::RestoreGate;

/// Tunables for the restore flow.
#[derive(Debug, Clone, Default)]
pub struct RestoreOptions {
    /// Skip the per-file SHA-256 + size verification step.
    pub skip_verify: bool,
    /// Allow extracting into a non-empty destination directory.
    pub force: bool,
    /// Directory to deduplicate the download against. Files already sitting
    /// there whose SHA-256 matches a manifest entry are **copied locally**
    /// instead of fetched over the network (cloud content-addressed path only).
    /// `None` disables the shortcut and downloads everything.
    ///
    /// It's a separate path rather than just `dest` because the two callers
    /// differ: a staged auto-restore extracts into an empty temp dir, so the
    /// bytes worth reusing live in the *live save folder*, not in `dest`. A
    /// direct restore writes straight into the save folder and passes `dest`.
    pub reuse_from: Option<PathBuf>,
    /// Which of the snapshot's files may touch the disk.
    ///
    /// The snapshot carries inside it the configuration of the machine that
    /// uploaded it (see [`hoard_core::kernel::fileclass`]): it is uploaded on
    /// purpose, so it is never lost, but writing it over ANOTHER machine hands
    /// the game a resolution, a GPU or a path that does not exist there. By
    /// default the gate is shut for that class of file and only the user opens it
    /// by hand (`--allow-ini`, the switch in the dialog).
    ///
    /// [`RestoreGate::permissive`] restores everything, as before this existed.
    pub gate: RestoreGate,
}

/// Result summary after a successful restore.
#[derive(Debug, Clone)]
pub struct RestoreOutcome {
    pub files_extracted: usize,
    pub bytes_extracted: u64,
    pub destination: PathBuf,
    /// Subset of `files_extracted` whose bytes came from an identical file
    /// already on disk instead of the network. Always 0 on the paths that
    /// can't dedup per file (self-hosted tar, legacy cloud archive).
    pub files_reused: usize,
    /// Subset of `bytes_extracted` that never crossed the network.
    pub bytes_reused: u64,
    /// What each half of the restore cost. See [`RestoreTimings`].
    pub timings: RestoreTimings,
}

/// How a restore's time is split between its phases.
///
/// The same save can take 25 s, 15 s or no time at all depending on which phase
/// dominates (the manifest, hashing the local disk, or the transfer) and without
/// this breakdown all three are indistinguishable from outside: a user only sees
/// "sometimes it is slow". It is filled in on the content-addressed path, Cloud's;
/// on the others only the total is known.
#[derive(Debug, Clone, Copy, Default)]
pub struct RestoreTimings {
    /// Asking for the version's manifest, with its presigned URLs.
    pub manifest_ms: u64,
    /// Indexing what is already on disk by content (the D.13 dedup). It is local
    /// CPU and IO time: it grows with the folder's size, not with the network.
    pub index_ms: u64,
    /// Moving bytes: GETs to R2 plus the local copies the index saved.
    pub transfer_ms: u64,
    /// From the first call to the last byte written.
    pub total_ms: u64,
}

/// A hard ceiling on the total bytes a single restore may write to disk: defence
/// in depth against a decompression bomb, a tiny `.tar.zst` that expands to
/// terabytes. Used as-is when the expanded size is not known ahead of time (the
/// legacy whole-archive cloud path) and as an upper clamp otherwise.
const MAX_RESTORE_BYTES: u64 = 64 * 1024 * 1024 * 1024; // 64 GiB

/// Bounded fan-out for the content-addressed restore: how many blob
/// downloads run in flight at once. Presigned-GET round-trip latency, not
/// bandwidth, dominates the many-small-files shape of most saves; a small
/// window hides it without hammering the disk with concurrent writes.
const RESTORE_CONCURRENCY: usize = 4;

/// An override for the fan-out, so it can be measured. The right value depends on
/// the save's shape (a monolith does not split, 4000 chunks do) and on the user's
/// line, and until the bench (`hoard-pruebas bench`) has swept the range there is
/// no reason to believe 4 is the right number for everybody. Outside `[1, 64]` it
/// is ignored: an absurd fan-out is not a preference, it is a slipped finger.
const CONCURRENCY_ENV: &str = "HOARD_RESTORE_CONCURRENCY";

fn restore_concurrency() -> usize {
    std::env::var(CONCURRENCY_ENV)
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .filter(|n| (1..=64).contains(n))
        .unwrap_or(RESTORE_CONCURRENCY)
}

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
            .clamp(FLOOR, MAX_RESTORE_BYTES),
        _ => MAX_RESTORE_BYTES,
    }
}

/// Where a snapshot's relative paths are planted.
///
/// Normally that is `dest` as-is. But a single-file save keeps the file, rather
/// than a folder, in `local_path`, and its snapshot carries one entry with the base
/// name, so `dest.join("save.dat")` would give `.../save.dat/save.dat`. In that
/// case the root is the parent directory and the `join` rebuilds exactly the
/// original path.
///
/// It is recognised two ways, because both happen: the file is already on disk
/// (the normal case), or the machine is new and there is nothing, and then the
/// snapshot's shape decides, a single entry named the same as the destination.
/// A single-file snapshot is never filtered, neither on upload nor on restore: the
/// user pointed at that particular file, and that outweighs any rule by name.
/// Without this exception, a save called `settings.ini` would upload (the walk
/// already excepts it) and never come back.
pub(crate) fn is_single_file_snapshot(dest: &Path, snapshot_names: &[&str]) -> bool {
    extraction_root(dest, snapshot_names) != dest
}

fn extraction_root(dest: &Path, snapshot_names: &[&str]) -> PathBuf {
    let is_single_file_save = dest.is_file()
        || (!dest.exists()
            && matches!(snapshot_names, [only]
                if Some(*only) == dest.file_name().and_then(|s| s.to_str())));
    if is_single_file_save {
        if let Some(parent) = dest.parent() {
            if !parent.as_os_str().is_empty() {
                return parent.to_path_buf();
            }
        }
    }
    dest.to_path_buf()
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
///
/// `options.reuse_from` (dedup against the local disk) only applies to the
/// cloud content-addressed path. Self-hosted ships **one monolithic
/// `tar.zst`** per snapshot: the server streams the whole archive and there's
/// no per-file GET to skip, so knowing a file is already on disk saves
/// nothing. That path is left exactly as it was.
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
    // The same shape rule as adding, and for the same reason: a restore over
    // `C:\Users\<x>` or over `~` would dump a snapshot on top of the user's
    // profile. It matters even more here, because this WRITES, and the path can
    // come from a `state.json` poisoned by an old detection or from another
    // machine. The folder may not exist yet (a new machine), so only the shape is
    // checked.
    crate::library::validate_path_shape(dest)?;
    if client.is_cloud().await {
        return download_snapshot_cloud(client, save_id, version, dest, options, progress).await;
    }
    let started = std::time::Instant::now();
    let detail: SnapshotDetail = client.snapshot_detail(save_id, version).await?;
    let expected: HashMap<String, &SnapshotFile> = detail
        .files
        .iter()
        .map(|f| (f.relative_path.clone(), f))
        .collect();

    let names: Vec<&str> = detail
        .files
        .iter()
        .map(|f| f.relative_path.as_str())
        .collect();
    let root = extraction_root(dest, &names);
    let single_file = root != dest;

    if dest.exists() {
        // "Not empty" only makes sense for a folder; a single-file save that
        // already exists is exactly what we came to replace.
        let empty = !single_file && std::fs::read_dir(dest)?.next().is_none();
        if !empty && !options.force {
            bail!(
                "destination is not empty: {} (set force = true to extract anyway)",
                dest.display()
            );
        }
    } else {
        std::fs::create_dir_all(&root).with_context(|| format!("creating {}", root.display()))?;
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

        // Sanitize: reject anything that escapes the destination. For directory
        // entries an empty result (`./`, say) just means the archive root, so there
        // is nothing to do.
        let safe_rel = match sanitize(&path_in_tar) {
            Some(p) => p,
            None if entry.header().entry_type().is_dir() => continue,
            None => bail!("unsafe path in archive: {}", path_in_tar.display()),
        };
        let dest_path = root.join(&safe_rel);

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
        // Config and litter from the machine that uploaded the snapshot are not
        // written over this one's unless the user asked for it.
        if !single_file && !options.gate.allows(&key) {
            tracing::debug!(path = %key, "restore: skipping device-local file");
            continue;
        }
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
                // `None` (an unknown digest) is compared against "" just as it was
                // before the newtype: it fails closed, never skipping verification.
                let expected = meta.sha256.as_ref().map(Sha256Hex::as_str).unwrap_or("");
                if got != expected {
                    bail!("sha256 mismatch for {key}: expected {expected}, got {got}");
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
        // A whole tar has no separable phases: download, decompression and writing
        // all happen in the same loop. There is only a total.
        timings: RestoreTimings {
            total_ms: started.elapsed().as_millis() as u64,
            ..Default::default()
        },
        // Whole-archive path: nothing to skip per file.
        files_reused: 0,
        bytes_reused: 0,
    })
}

/// Whether a blob download error is worth re-fetching the same blob for:
/// transient R2/Cloudflare connection drops (the reqwest "end of file before
/// message length reached" family) and the sha256 mismatch a truncated body
/// produces. Permanent errors (disk full, permission denied) fall through and
/// fail the restore so we don't spin on them.
fn is_retryable_blob_error(e: &anyhow::Error) -> bool {
    let s = format!("{e:#}").to_lowercase();
    s.contains("end of file")
        || s.contains("error reading a body")
        || s.contains("decoding response body")
        || s.contains("connection reset")
        || s.contains("connection closed")
        || s.contains("broken pipe")
        || s.contains("timed out")
        || s.contains("sha256 mismatch")
}

/// A Hoard Cloud download: a presigned R2 GET into a temp tar.zst, verifying the
/// whole archive's sha256, then extracting.
///
/// The cloud server stores one opaque `.tar.zst` per version and exposes no
/// per-file manifest (`snapshot_detail` does not exist there), so verification is
/// over the whole archive's sha256, recorded at commit time and returned in
/// `DownloadOut`, rather than per file. We download to a temp file first so the
/// hash check happens before we touch the destination.
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
    let started = std::time::Instant::now();
    let manifest = client
        .cloud_version_manifest(save_id, version, true)
        .await?;
    let manifest_ms = started.elapsed().as_millis() as u64;
    if manifest.content_addressed {
        let mut outcome = restore_cloud_cas(client, dest, options, manifest, progress).await?;
        outcome.timings.manifest_ms = manifest_ms;
        outcome.timings.total_ms = started.elapsed().as_millis() as u64;
        return Ok(outcome);
    }

    let meta = client.cloud_download(save_id, version).await?;

    // As on the legacy path: an existing single-file save is not a "non-empty
    // destination", it is exactly what we came to replace.
    let root = extraction_root(dest, &[]);
    if dest.exists() {
        let empty = root != dest || std::fs::read_dir(dest)?.next().is_none();
        if !empty && !options.force {
            bail!(
                "destination is not empty: {} (set force = true to extract anyway)",
                dest.display()
            );
        }
    } else {
        std::fs::create_dir_all(&root).with_context(|| format!("creating {}", root.display()))?;
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
    result.map(|mut outcome| {
        outcome.timings.manifest_ms = manifest_ms;
        outcome.timings.total_ms = started.elapsed().as_millis() as u64;
        outcome
    })
}

/// Files already on disk, keyed by the SHA-256 of their contents. Built by
/// [`build_reuse_index`] and consumed by [`plan_byte_sources`].
type ReuseIndex = HashMap<String, PathBuf>;

/// Where one manifest entry's bytes come from.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ByteSource {
    /// An identical file is already on disk at this path, so copy it.
    Reuse(PathBuf),
    /// Nothing on disk carries these bytes, so fetch the blob.
    Download,
}

/// Index the files under `dir` by the SHA-256 of their contents.
///
/// Only files whose length is one the manifest actually wants get hashed: a file of
/// a different size cannot be the content we are looking for, so the filter keeps a
/// save folder's unrelated multi-GB neighbours from being read. Worst case we read
/// as much as the snapshot itself is big, a second or two of local IO against a
/// minute of network.
///
/// Every failure degrades into a *smaller* index rather than an error: an
/// unreadable file (a lock a running game holds, a permission we do not have) just
/// means one more blob to download. It never fails the restore.
async fn build_reuse_index(
    dir: &Path,
    wanted_sizes: &HashSet<u64>,
    shields: &[String],
) -> ReuseIndex {
    if wanted_sizes.is_empty() || !dir.exists() {
        // An empty or missing destination: no index, everything downloads, which is
        // the pre-dedup behaviour, bit for bit.
        return ReuseIndex::new();
    }
    // `walk_source` is the same walk the backup side uses: sorted by relative
    // path, symlinks and transient game locks already filtered out.
    let candidates: Vec<crate::backup::UploadFile> = match crate::backup::walk_source(dir, shields)
    {
        Ok(files) => files
            .into_iter()
            .filter(|f| wanted_sizes.contains(&f.size_bytes))
            .collect(),
        Err(e) => {
            tracing::debug!(
                dir = %dir.display(),
                error = %format!("{e:#}"),
                "cloud restore: couldn't walk the local folder; downloading everything"
            );
            return ReuseIndex::new();
        }
    };

    // A few files hash in flight so per-file open latency overlaps. `buffered`
    // rather than `buffer_unordered`: results stay in walk order, so when two
    // files share content the index deterministically keeps the first by
    // relative path.
    let mut hash_futs = Vec::with_capacity(candidates.len());
    for f in candidates {
        hash_futs.push(
            async move {
                match crate::backup::hash_file(&f.absolute_path).await {
                    Ok(sha) => Some((sha, f.absolute_path)),
                    Err(e) => {
                        tracing::debug!(
                            path = %f.absolute_path.display(),
                            error = %format!("{e:#}"),
                            "cloud restore: couldn't hash local file; not a reuse candidate"
                        );
                        None
                    }
                }
            }
            .boxed(),
        );
    }
    let hashed: Vec<Option<(String, PathBuf)>> = futures::stream::iter(hash_futs)
        .buffered(restore_concurrency())
        .collect()
        .await;

    let mut index = ReuseIndex::new();
    for (sha, path) in hashed.into_iter().flatten() {
        index.entry(sha).or_insert(path);
    }
    index
}

/// Join the manifest's per-file SHAs against the on-disk index.
///
/// Pure: all the IO happened in [`build_reuse_index`]. Matching is by content
/// hash *only*: a local file that shares a manifest entry's name but not its
/// bytes hashes differently and simply isn't in the index, so it's downloaded.
fn plan_byte_sources(shas: &[String], index: &ReuseIndex) -> Vec<ByteSource> {
    shas.iter()
        .map(|sha| match index.get(sha) {
            Some(path) => ByteSource::Reuse(path.clone()),
            None => ByteSource::Download,
        })
        .collect()
}

/// Copy an already-present local file into its restore destination, verifying
/// that what landed hashes to the SHA-256 the manifest declares.
///
/// The check is the same one the download path runs, and it's what makes the
/// shortcut safe rather than merely fast: a wrong reuse (stale index, a file
/// rewritten under us) fails here and the caller falls back to the network.
async fn copy_local_blob(
    src: &Path,
    dest_path: &Path,
    file: &crate::api::CloudManifestFile,
    options: &RestoreOptions,
) -> Result<()> {
    // On a direct restore the folder we indexed *is* `dest`, so the right bytes
    // can already be at the right path. Nothing to copy, so re-verify in place and
    // "everything under dest was hash-checked this pass" still holds.
    if src == dest_path {
        if options.skip_verify {
            return Ok(());
        }
        let got = crate::backup::hash_file(dest_path).await?;
        if got != file.sha256 {
            bail!(
                "sha256 mismatch reusing {} for {}: expected {}, got {}",
                src.display(),
                file.relative_path,
                file.sha256,
                got
            );
        }
        return Ok(());
    }

    let mut input = tokio::fs::File::open(src)
        .await
        .with_context(|| format!("opening local {} for reuse", src.display()))?;
    let mut out = tokio::fs::File::create(dest_path)
        .await
        .with_context(|| format!("writing {}", dest_path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 256 * 1024];
    loop {
        let n = input
            .read(&mut buf)
            .await
            .with_context(|| format!("reading local {}", src.display()))?;
        if n == 0 {
            break;
        }
        if !options.skip_verify {
            hasher.update(&buf[..n]);
        }
        out.write_all(&buf[..n])
            .await
            .with_context(|| format!("writing {}", dest_path.display()))?;
    }
    out.flush()
        .await
        .with_context(|| format!("writing {}", dest_path.display()))?;
    drop(out);

    if !options.skip_verify {
        let got = hex::encode(hasher.finalize());
        if got != file.sha256 {
            bail!(
                "sha256 mismatch reusing {} for {}: expected {}, got {}",
                src.display(),
                file.relative_path,
                file.sha256,
                got
            );
        }
    }
    Ok(())
}

/// Whether a manifest entry's bytes were copied off the local disk or fetched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Origin {
    Reused,
    Downloaded,
}

fn as_mib(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

/// Content-addressed restore: each file in the manifest is its own R2 blob.
/// Stream each to its destination path, verifying the whole-file sha256 and
/// preserving the recorded mtime (so a cloud pull doesn't always win the
/// conflict-aware diff). No temp archive: files land directly.
///
/// Before any blob is fetched, the folder named by `options.reuse_from` is
/// indexed by content hash and every manifest entry whose SHA is already on
/// disk is served by a local copy instead of a GET. Upload already dedups
/// against the server's blobs; this is the same saving in the other direction
/// (ADR 0021 D.13). Twelve 8 MB Factorio autosaves with one changed file go
/// from ~400 MB of egress to ~8 MB.
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
    let names: Vec<&str> = manifest
        .files
        .iter()
        .map(|f| f.relative_path.as_str())
        .collect();
    let root = extraction_root(dest, &names);
    let single_file = root != dest;

    if dest.exists() {
        let empty = !single_file && std::fs::read_dir(dest)?.next().is_none();
        if !empty && !options.force {
            bail!(
                "destination is not empty: {} (set force = true to extract anyway)",
                dest.display()
            );
        }
    } else {
        std::fs::create_dir_all(&root).with_context(|| format!("creating {}", root.display()))?;
    }

    // The gate is applied once, and here, and everything below (the progress
    // total, the jobs, the byte plan) comes off THIS list. Filtering only the jobs
    // and letting the plan be computed over the whole manifest put them out of
    // step: `jobs[i]` got another file's `plan[i]`, the local copy failed its sha
    // check and everything fell back to the network. It does not corrupt (which is
    // why it is verified), but it silently kills exactly D.13's dedup against disk,
    // the 400 MB to 8 MB Factorio case.
    //
    // A single-file save is not filtered: see `is_single_file_snapshot`.
    let kept: Vec<&crate::api::CloudManifestFile> = if single_file {
        manifest.files.iter().collect()
    } else {
        manifest
            .files
            .iter()
            .filter(|f| {
                let ok = options.gate.allows(&f.relative_path);
                if !ok {
                    tracing::debug!(path = %f.relative_path, "restore: skipping device-local file");
                }
                ok
            })
            .collect()
    };

    let total: u64 = kept.iter().map(|f| f.size_bytes.max(0) as u64).sum();

    // Sanitize every path before moving any bytes: a hostile manifest aborts
    // up front, not after some files have already landed in dest.
    let mut jobs = Vec::with_capacity(kept.len());
    for file in &kept {
        let safe_rel = sanitize(Path::new(&file.relative_path))
            .ok_or_else(|| anyhow!("unsafe path in manifest: {}", file.relative_path))?;
        jobs.push((*file, root.join(safe_rel)));
    }

    // Dedup against the disk before touching the network. Hashing the folder
    // costs a couple of seconds; the blobs it lets us skip cost a minute of
    // egress each time a save rotates one file out of a dozen.
    let index_started = std::time::Instant::now();
    let plan = match options.reuse_from.as_deref() {
        Some(reuse_dir) => {
            let wanted: HashSet<u64> = kept.iter().map(|f| f.size_bytes.max(0) as u64).collect();
            let index = build_reuse_index(reuse_dir, &wanted, &options.gate.shields).await;
            let shas: Vec<String> = kept.iter().map(|f| f.sha256.clone()).collect();
            plan_byte_sources(&shas, &index)
        }
        None => vec![ByteSource::Download; jobs.len()],
    };
    let index_ms = index_started.elapsed().as_millis() as u64;

    // A few blobs download in flight at once: presigned-GET round-trip
    // latency dominates the many-small-files shape. Each blob writes to its
    // own dest path and verifies independently, so completion order doesn't
    // matter; progress counts bytes as they land. (Eager Vec of boxed
    // futures rather than `iter().map(closure)`: a closure over borrowed
    // items retained inside the stream trips rustc's "Send is not general
    // enough" false positive when the restore future crosses `tokio::spawn`.)
    //
    // `landed` counts *every* byte that reaches dest, copied or downloaded, so
    // the bar still runs 0→total when most of the restore came off the local
    // disk. Counting only network bytes would park it at 2% on the Factorio
    // case and look hung; the reused/downloaded split is reported separately,
    // in the outcome and the log line below.
    let landed = AtomicU64::new(0);
    progress(0, total);

    let transfer_started = std::time::Instant::now();
    let mut fetch_futs = Vec::with_capacity(jobs.len());
    for ((file, dest_path), planned) in jobs.iter().zip(plan.iter()) {
        let landed = &landed;
        let progress = &progress;
        let options = &options;
        fetch_futs.push(
            async move {
                if let Some(parent) = dest_path.parent() {
                    tokio::fs::create_dir_all(parent)
                        .await
                        .with_context(|| format!("creating parent {}", parent.display()))?;
                }

                // Local shortcut first. On failure, including a sha that doesn't
                // match (which is the whole point of keeping the check), we fall
                // through to the network, so a bad reuse costs a wasted read, never
                // a corrupt file or a failed restore. (That fallback is also what
                // makes a direct restore safe when a source we indexed is itself
                // some other entry's destination, as with rotating autosave names:
                // whoever loses the race fails verification and downloads.)
                if let ByteSource::Reuse(src) = planned {
                    match copy_local_blob(src, dest_path, file, options).await {
                        Ok(()) => {
                            // One bump on success rather than per chunk: a partial
                            // copy that later falls back to the network must not
                            // have its bytes counted twice.
                            let n = file.size_bytes.max(0) as u64;
                            let done = landed.fetch_add(n, Ordering::Relaxed) + n;
                            progress(done, total);
                            apply_manifest_mtime(file, dest_path);
                            return Ok::<(u64, Origin), anyhow::Error>((n, Origin::Reused));
                        }
                        Err(e) => tracing::warn!(
                            path = %file.relative_path,
                            source = %src.display(),
                            error = %format!("{e:#}"),
                            "cloud restore: local reuse failed verification, downloading the blob"
                        ),
                    }
                }

                let presigned = file.download.as_ref().ok_or_else(|| {
                    anyhow!("manifest missing download URL for {}", file.relative_path)
                })?;
                // R2/Cloudflare occasionally truncates a blob mid-stream ("end of
                // file before message length reached"). Failing the whole restore
                // over one dropped connection is a brutal retry: the next
                // reconciliation sweep re-downloads *every* blob. Blobs are
                // content-addressed and sha-verified, so just re-fetch the one blob a
                // few times with a short backoff (a truncated body fails the sha
                // check, which is retried too).
                const BLOB_FETCH_ATTEMPTS: u32 = 4;
                let mut attempt = 0u32;
                loop {
                    attempt += 1;
                    let fetch = async {
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
                            let done = landed.fetch_add(chunk.len() as u64, Ordering::Relaxed)
                                + chunk.len() as u64;
                            progress(done, total);
                        }
                        out.flush().await.context("flushing file")?;
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
                        Ok::<(), anyhow::Error>(())
                    }
                    .await;
                    match fetch {
                        Ok(()) => break,
                        Err(e) if attempt < BLOB_FETCH_ATTEMPTS && is_retryable_blob_error(&e) => {
                            tracing::warn!(
                                attempt,
                                path = %file.relative_path,
                                error = %format!("{e:#}"),
                                "cloud restore: blob download failed, retrying"
                            );
                            tokio::time::sleep(std::time::Duration::from_millis(
                                300u64 * u64::from(attempt),
                            ))
                            .await;
                        }
                        Err(e) => return Err(e),
                    }
                }

                apply_manifest_mtime(file, dest_path);

                Ok::<(u64, Origin), anyhow::Error>((
                    file.size_bytes.max(0) as u64,
                    Origin::Downloaded,
                ))
            }
            .boxed(),
        );
    }
    let restored: Vec<(u64, Origin)> = futures::stream::iter(fetch_futs)
        .buffer_unordered(restore_concurrency())
        .try_collect()
        .await?;
    let transfer_ms = transfer_started.elapsed().as_millis() as u64;

    let mut files_reused = 0usize;
    let mut bytes_reused = 0u64;
    let mut files_downloaded = 0usize;
    let mut bytes_downloaded = 0u64;
    for (bytes, origin) in &restored {
        match origin {
            Origin::Reused => {
                files_reused += 1;
                bytes_reused += bytes;
            }
            Origin::Downloaded => {
                files_downloaded += 1;
                bytes_downloaded += bytes;
            }
        }
    }
    // The dogfooding check for D.13: on a save that rotated one file out of a dozen
    // this should read about 390 MB reused and 8 MB downloaded, not 400/0. The
    // phases go on the same line because the question after "it took 25 s" is
    // always "doing what?", and answering that on two separate lines forces you to
    // match them up by timestamp when several saves are in flight.
    tracing::info!(
        files_reused,
        mib_reused = as_mib(bytes_reused),
        files_downloaded,
        mib_downloaded = as_mib(bytes_downloaded),
        index_ms,
        transfer_ms,
        concurrency = restore_concurrency(),
        dedup_source = options
            .reuse_from
            .as_deref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "disabled".to_string()),
        "cloud restore: content-addressed restore finished"
    );

    Ok(RestoreOutcome {
        files_extracted: restored.len(),
        bytes_extracted: bytes_reused + bytes_downloaded,
        destination: dest.to_path_buf(),
        files_reused,
        bytes_reused,
        timings: RestoreTimings {
            manifest_ms: 0, // filled in by `download_snapshot_cloud`, which asked for it
            index_ms,
            transfer_ms,
            total_ms: 0,
        },
    })
}

/// Stamp the manifest's recorded mtime onto a file that just landed in `dest`.
///
/// Without it every cloud pull would look strictly newer than the local copy
/// and silently win the auto-restore diff. Reused files get exactly the same
/// treatment as downloaded ones: they're indistinguishable to the
/// staging→merge step, which is what keeps `preserve_staging_mtime` honest.
/// Best-effort: a failure only degrades conflict resolution.
fn apply_manifest_mtime(file: &crate::api::CloudManifestFile, dest_path: &Path) {
    if let Some(secs) = file.modified_at {
        if secs > 0 {
            let ft = filetime::FileTime::from_unix_time(secs, 0);
            let _ = filetime::set_file_mtime(dest_path, ft);
        }
    }
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
    // The legacy path (a whole archive, with no per-file manifest): the snapshot's
    // shape is not known in advance, so a single-file save can only be recognised
    // because the file is already on disk. That is enough: only versions uploaded
    // long ago take this path.
    let root = extraction_root(dest, &[]);
    let single_file = root != dest;
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
    // is unknown, so fall back to the absolute ceiling.
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
        let dest_path = root.join(&safe_rel);

        if entry.header().entry_type().is_dir() {
            tokio::fs::create_dir_all(&dest_path).await.ok();
            continue;
        }
        if let Some(parent) = dest_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("creating parent {}", parent.display()))?;
        }

        if !single_file
            && !options
                .gate
                .allows(&safe_rel.to_string_lossy().replace('\\', "/"))
        {
            tracing::debug!(path = %safe_rel.display(), "restore: skipping device-local file");
            continue;
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
        // Legacy whole-archive cloud version: no per-file blobs to skip.
        files_reused: 0,
        bytes_reused: 0,
        // The total is stamped by `download_snapshot_cloud`, which started the
        // stopwatch (asking for the manifest included).
        timings: RestoreTimings::default(),
    })
}

/// Hoard Cloud: list the files inside a version's blob (relative path + size)
/// without extracting anything to disk. The cloud stores one opaque `.tar.zst`
/// per version and keeps no per-file index, so the History detail view streams
/// the blob through the zstd + tar decoders and reads just the entry headers.
/// File bodies are skipped (`tokio_tar` seeks past them) and nothing touches
/// the filesystem, so this stays cheap even for large saves. `sha256` is left
/// empty: the tar header doesn't carry it and the detail view doesn't show it.
pub async fn list_cloud_version_files(
    client: &ApiClient,
    save_id: &str,
    version: i64,
) -> Result<Vec<SnapshotFile>> {
    // Content-addressed versions keep a real per-file index server-side, so the
    // detail view is a single cheap call: no blob download, no bandwidth. SHAs
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
                // A sha of an invalid shape comes in as "unknown", which
                // verification treats as a failure, never as "skip it".
                sha256: Sha256Hex::parse(&f.sha256).ok(),
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
            // Legacy whole-archive version: the tar carries no per-file digest.
            // `None` is exactly the `""` that release used to emit.
            sha256: None,
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
/// every cloud pull would look strictly newer than local and silently win,
/// exactly the "everything from the cloud came down marked newer" bug.
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

    /// La cuenta que importa: `root.join(nombre)` tiene que devolver la ruta
    /// original del fichero, no `…/save.dat/save.dat`.
    #[test]
    fn a_single_file_save_extracts_into_its_parent() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("ssr_save.bin");
        std::fs::write(&file, b"x").unwrap();

        let root = extraction_root(&file, &["ssr_save.bin"]);
        assert_eq!(root, tmp.path());
        assert_eq!(root.join("ssr_save.bin"), file);
    }

    /// A fresh machine: the file does not exist yet, so the snapshot's shape
    /// decides, and a single entry named like the destination means one file.
    #[test]
    fn a_missing_single_file_save_is_recognised_from_the_snapshot_shape() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("save.dat");
        assert_eq!(extraction_root(&file, &["save.dat"]), tmp.path());
        // With more than one entry it is a folder that does not exist yet.
        assert_eq!(
            extraction_root(&file, &["a.sav", "b.sav"]),
            file,
            "several files means the destination is the folder"
        );
        // Y una entrada con OTRO nombre tampoco lo convierte en fichero suelto.
        assert_eq!(extraction_root(&file, &["otro.dat"]), file);
    }

    /// Un save de carpeta normal no se toca.
    #[test]
    fn a_folder_save_extracts_into_itself() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("Saves");
        std::fs::create_dir(&dir).unwrap();
        std::fs::write(dir.join("slot1.sav"), b"x").unwrap();
        assert_eq!(extraction_root(&dir, &["slot1.sav"]), dir);
    }

    #[test]
    fn retryable_blob_error_covers_truncation_and_sha() {
        use anyhow::anyhow;
        // The exact R2/Cloudflare truncation the user hit.
        assert!(is_retryable_blob_error(&anyhow!(
            "downloading blob: error decoding response body: request or response body error: error reading a body from connection: end of file before message length reached"
        )));
        assert!(is_retryable_blob_error(&anyhow!(
            "sha256 mismatch for save.zip: expected abc, got def"
        )));
        assert!(is_retryable_blob_error(&anyhow!(
            "connection reset by peer"
        )));
        // Permanent errors must NOT retry.
        assert!(!is_retryable_blob_error(&anyhow!(
            "writing /x: No space left on device (os error 28)"
        )));
        assert!(!is_retryable_blob_error(&anyhow!("permission denied")));
    }

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

    // ---- D.13: restore dedups against the local disk ----

    fn sha_of(bytes: &[u8]) -> String {
        let mut h = Sha256::new();
        h.update(bytes);
        hex::encode(h.finalize())
    }

    /// Write `contents` to `dir/name`, creating parents. Returns its sha256.
    fn seed(dir: &Path, name: &str, contents: &[u8]) -> String {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, contents).unwrap();
        sha_of(contents)
    }

    /// The set of sizes a manifest of these blobs would declare.
    fn sizes_of(blobs: &[Vec<u8>]) -> HashSet<u64> {
        blobs.iter().map(|b| b.len() as u64).collect()
    }

    /// N manifest entries, N-1 of them already on disk → exactly one download.
    /// The Factorio shape: a dozen autosaves, one of them rotated.
    #[tokio::test]
    async fn present_files_are_reused_and_only_the_new_one_downloads() {
        let dir = tempfile::tempdir().unwrap();
        const N: usize = 12;

        // Distinct contents of distinct lengths, one per autosave slot.
        let blobs: Vec<Vec<u8>> = (0..N).map(|i| vec![b'a' + i as u8; 4096 + i]).collect();
        let manifest_shas: Vec<String> = blobs.iter().map(|b| sha_of(b)).collect();

        // Everything but the last entry is already sitting in the destination.
        for (i, blob) in blobs.iter().take(N - 1).enumerate() {
            seed(dir.path(), &format!("_autosave{i}.zip"), blob);
        }

        let index = build_reuse_index(dir.path(), &sizes_of(&blobs), &[]).await;
        let plan = plan_byte_sources(&manifest_shas, &index);

        assert_eq!(plan.len(), N);
        let downloads = plan.iter().filter(|s| **s == ByteSource::Download).count();
        assert_eq!(downloads, 1, "only the rotated file should be fetched");
        // And each reuse points at the local file that actually holds those bytes.
        for (i, source) in plan.iter().take(N - 1).enumerate() {
            let expected = dir.path().join(format!("_autosave{i}.zip"));
            assert_eq!(*source, ByteSource::Reuse(expected));
        }
    }

    /// The regression that slipped in with the gate: the byte plan was computed
    /// over the **whole** manifest and then paired by position with the job list
    /// that had already been **filtered**. A single vetoed file shifted every
    /// other one by a position, each local copy failed its sha check and fell
    /// back to the network, and the dedup against disk was dead in silence.
    ///
    /// What is checked here is the invariant that prevents it: the plan derives
    /// from the same filtered list, so `plan[i]` belongs to `kept[i]`.
    #[tokio::test]
    async fn the_byte_plan_lines_up_with_the_filtered_job_list() {
        let dir = tempfile::tempdir().unwrap();

        // Three files in the snapshot; the middle one is config and the gate
        // vetoes it. The other two are already on disk with their good bytes.
        let save_a = b"save A".to_vec();
        let conf = b"res=1920x1080".to_vec();
        let save_b = b"save B, a different length".to_vec();
        seed(dir.path(), "a.sav", &save_a);
        seed(dir.path(), "b.sav", &save_b);
        seed(dir.path(), "graphics.ini", &conf);

        let gate = RestoreGate::default();
        let all = [
            ("a.sav", &save_a),
            ("graphics.ini", &conf),
            ("b.sav", &save_b),
        ];
        let kept: Vec<_> = all
            .iter()
            .filter(|(rel, _)| gate.allows(rel))
            .copied()
            .collect();
        assert_eq!(kept.len(), 2, "la puerta debe vetar el .ini");

        let sizes: HashSet<u64> = kept.iter().map(|(_, b)| b.len() as u64).collect();
        let index = build_reuse_index(dir.path(), &sizes, &gate.shields).await;
        let shas: Vec<String> = kept.iter().map(|(_, b)| sha_of(b)).collect();
        let plan = plan_byte_sources(&shas, &index);

        assert_eq!(plan.len(), kept.len());
        // Every plan entry points at the local file that really holds those
        // bytes. With the shift, `a.sav` got `graphics.ini`'s plan.
        assert_eq!(plan[0], ByteSource::Reuse(dir.path().join("a.sav")));
        assert_eq!(plan[1], ByteSource::Reuse(dir.path().join("b.sav")));
    }

    /// A single-file save is restored no matter what: the user pointed at that
    /// file. Without the exception, a save called `settings.ini` was uploaded
    /// (the walk already excepts it) but never came back.
    #[test]
    fn a_single_file_save_is_never_gated() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("settings.ini");
        std::fs::write(&dest, b"this really is the save").unwrap();
        assert!(is_single_file_snapshot(&dest, &["settings.ini"]));
        // The gate would veto it by name if it were asked.
        assert!(!RestoreGate::default().allows("settings.ini"));

        // And on a fresh machine too, where the file does not exist yet: the
        // snapshot's shape is what decides.
        let fresh = dir.path().join("save.cfg");
        assert!(is_single_file_snapshot(&fresh, &["save.cfg"]));
        // Una carpeta con varios ficheros no es un save de fichero suelto.
        assert!(!is_single_file_snapshot(dir.path(), &["a.sav", "b.sav"]));
    }

    /// Same relative path, different bytes: reuse is keyed on content, never on
    /// the name, so a locally-modified file must not shortcut the download.
    #[tokio::test]
    async fn same_name_different_content_is_not_reused() {
        let dir = tempfile::tempdir().unwrap();
        let remote = b"the version the server has".to_vec();
        // Same length so the size prefilter can't be what saves us; the hash
        // has to be the thing that rejects it.
        let local = b"a different local edition!".to_vec();
        assert_eq!(remote.len(), local.len());

        seed(dir.path(), "save.dat", &local);

        let index =
            build_reuse_index(dir.path(), &sizes_of(std::slice::from_ref(&remote)), &[]).await;
        let plan = plan_byte_sources(&[sha_of(&remote)], &index);

        assert_eq!(plan, vec![ByteSource::Download]);
    }

    /// Empty or missing destination: no index, everything downloads. This is the
    /// pre-D.13 behaviour and it must stay byte-for-byte the same.
    #[tokio::test]
    async fn empty_or_missing_destination_downloads_everything() {
        let blobs: Vec<Vec<u8>> = vec![b"one".to_vec(), b"two!".to_vec(), b"three".to_vec()];
        let shas: Vec<String> = blobs.iter().map(|b| sha_of(b)).collect();
        let wanted = sizes_of(&blobs);

        let empty = tempfile::tempdir().unwrap();
        let index = build_reuse_index(empty.path(), &wanted, &[]).await;
        assert!(index.is_empty());
        assert_eq!(
            plan_byte_sources(&shas, &index),
            vec![ByteSource::Download; 3]
        );

        let missing = empty.path().join("not-created-yet");
        let index = build_reuse_index(&missing, &wanted, &[]).await;
        assert!(index.is_empty());
        assert_eq!(
            plan_byte_sources(&shas, &index),
            vec![ByteSource::Download; 3]
        );
    }

    /// The index looks at content, not layout: a file that moved or was renamed
    /// still serves its bytes. This is what makes rotating autosave names dedup.
    #[tokio::test]
    async fn reuse_follows_content_across_a_rename() {
        let dir = tempfile::tempdir().unwrap();
        let blob = vec![7u8; 8192];
        seed(dir.path(), "nested/old-name.zip", &blob);

        let index =
            build_reuse_index(dir.path(), &sizes_of(std::slice::from_ref(&blob)), &[]).await;
        let plan = plan_byte_sources(&[sha_of(&blob)], &index);

        assert_eq!(
            plan,
            vec![ByteSource::Reuse(dir.path().join("nested/old-name.zip"))]
        );
    }

    /// A reused file must land verified. `copy_local_blob` is the shortcut's
    /// safety gate: right bytes copy through, wrong bytes error out (and the
    /// caller falls back to the network).
    #[tokio::test]
    async fn copy_local_blob_verifies_what_it_copied() {
        let dir = tempfile::tempdir().unwrap();
        let staging = tempfile::tempdir().unwrap();
        let blob = b"exactly these bytes".to_vec();
        seed(dir.path(), "src.dat", &blob);

        let src = dir.path().join("src.dat");
        let dest = staging.path().join("landed.dat");
        let options = RestoreOptions::default();

        let good = crate::api::CloudManifestFile {
            relative_path: "landed.dat".to_string(),
            sha256: sha_of(&blob),
            size_bytes: blob.len() as i64,
            modified_at: None,
            download: None,
        };
        copy_local_blob(&src, &dest, &good, &options).await.unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), blob);

        // Same source, a manifest claiming other content: must not pass.
        let wrong = crate::api::CloudManifestFile {
            sha256: sha_of(b"something else entirely"),
            ..good
        };
        let err = copy_local_blob(&src, &dest, &wrong, &options)
            .await
            .expect_err("a mismatched reuse has to fail verification");
        assert!(
            format!("{err:#}").contains("sha256 mismatch"),
            "unexpected error: {err:#}"
        );
    }
}
