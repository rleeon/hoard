//! Library half of the snapshot download/extract flow.
//!
//! Streams the tar.zst from the server, decodes it, sanitises paths, verifies
//! each file's SHA-256 against the manifest, and writes them into a private
//! staging folder. Only once the whole version is there and verified are the
//! files moved into the destination (see [`download_snapshot`]). The CLI and GUI
//! share this code; presentation (progress bars, confirmation dialogs) lives in
//! their respective layers.

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

/// Stream one blob body into `dest_path`, decoding zstd on the way when the
/// object holds it.
///
/// The sha of a blob is always taken over its *raw* content, so hashing happens
/// after decoding, never before: what is verified is what lands on disk. `cap`
/// bounds how many decoded bytes are allowed through, and only the compressed
/// path passes one, because only there can a small body become a large file.
/// The sha check would catch a bomb too, but only after writing all of it.
///
/// `on_bytes` is called with each batch that reaches the file, so the progress
/// bar counts what the user actually gets rather than what crossed the wire.
///
/// Returns the hex sha of what was written when `verify`, and `None` when the
/// caller asked to skip verification.
async fn write_blob_body(
    body: impl tokio::io::AsyncRead + Unpin + Send,
    zstd: bool,
    cap: Option<u64>,
    dest_path: &Path,
    verify: bool,
    on_bytes: &(dyn Fn(u64) + Sync),
) -> Result<Option<String>> {
    let mut out = tokio::fs::File::create(dest_path)
        .await
        .with_context(|| format!("writing {}", dest_path.display()))?;
    let mut hasher = Sha256::new();
    let mut written: u64 = 0;

    let mut src: Box<dyn tokio::io::AsyncRead + Unpin + Send> = if zstd {
        Box::new(ZstdDecoder::new(BufReader::new(body)))
    } else {
        Box::new(body)
    };

    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = src.read(&mut buf).await.context("reading the blob body")?;
        if n == 0 {
            break;
        }
        written += n as u64;
        if let Some(cap) = cap {
            if written > cap {
                bail!(
                    "blob at {} expanded past its declared {} bytes",
                    dest_path.display(),
                    cap
                );
            }
        }
        if verify {
            hasher.update(&buf[..n]);
        }
        out.write_all(&buf[..n])
            .await
            .with_context(|| format!("writing {}", dest_path.display()))?;
        on_bytes(n as u64);
    }
    out.flush().await.context("flushing file")?;
    Ok(verify.then(|| hex::encode(hasher.finalize())))
}

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
    /// Every caller points it at the live save folder: the bytes are written to
    /// a staging folder first, so the files worth reusing are never where the
    /// download lands.
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
    /// Where a file is copied before the restore replaces it, as
    /// `<backup_root>/<save_id>/<timestamp>/<path>`. `None` is the conflicts
    /// folder the automatic restore already parks its losers in, so the same
    /// retention sweep clears both.
    pub backup_root: Option<PathBuf>,
    /// Where the version is staged before it is placed. `None` is
    /// `restore-staging` under Hoard's state folder; tests point it at their own
    /// temp folder so they never touch the real one.
    pub staging_root: Option<PathBuf>,
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
    /// Where the files this restore replaced were copied first: the way back.
    /// `None` when it replaced nothing.
    pub kept_in: Option<PathBuf>,
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
        // version.
        let manifest = client.cloud_sync().await?;
        if let Some(e) = manifest.saves.into_iter().find(|e| e.save_id == save_id) {
            return Ok(e.latest_version_num);
        }
        // Backup-only and archived saves are left out of the manifest on purpose
        // but keep their versions.
        return client
            .cloud_list_versions(save_id, false)
            .await?
            .iter()
            .map(|v| v.version_num)
            .max()
            .ok_or_else(|| anyhow!("save has no snapshots yet"));
    }
    let save = client.get_save(save_id).await?;
    save.latest_version_num
        .ok_or_else(|| anyhow!("save has no snapshots yet"))
}

/// Restore snapshot `version` of `save_id` into `dest`.
///
/// Nothing under `dest` is touched until the whole version is down and verified.
/// The files land in a private staging folder first; only when every one of them
/// is there, with the SHA-256 and size the manifest records, are they moved into
/// place, each replaced file copied aside first so a failure halfway puts the
/// folder back as it was (see [`place_staged`]). Writing straight into the save
/// and checking the hash afterwards is how a download R2 cut short left a
/// truncated file where a good save had been, and every retry emptied it again.
///
/// `progress(downloaded, total_or_zero)` is called as bytes flow from the
/// server. `total_or_zero` is 0 if the server didn't send Content-Length.
///
/// `options.reuse_from` (dedup against the local disk) only applies to the
/// cloud content-addressed path. Self-hosted ships **one monolithic
/// `tar.zst`** per snapshot: the server streams the whole archive and there's
/// no per-file GET to skip, so knowing a file is already on disk saves
/// nothing.
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
    download_snapshot_gated(
        client,
        save_id,
        version,
        dest,
        options,
        progress,
        std::future::ready(()),
    )
    .await
}

/// [`download_snapshot`], awaiting `before_place` between the verified download
/// and the first write into `dest`. The CLI and the History dialog wait there
/// for the service to finish whatever it was doing with the folder (an upload
/// reading it, an automatic restore merging into it) before they move files in.
pub async fn download_snapshot_gated<F, G>(
    client: &ApiClient,
    save_id: &str,
    version: i64,
    dest: &Path,
    options: RestoreOptions,
    progress: F,
    before_place: G,
) -> Result<RestoreOutcome>
where
    F: Fn(u64, u64) + Send + Sync + 'static,
    G: std::future::Future<Output = ()> + Send,
{
    // The same shape rule as adding, and for the same reason: a restore over
    // `C:\Users\<x>` or over `~` would dump a snapshot on top of the user's
    // profile. It matters even more here, because this WRITES, and the path can
    // come from a `state.json` poisoned by an old detection or from another
    // machine. The folder may not exist yet (a new machine), so only the shape is
    // checked.
    crate::library::validate_path_shape(dest)?;
    let staging = new_staging_dir_in(options.staging_root.as_deref())?;
    ensure_outside(staging.path(), dest)?;
    // Resolved before a byte is fetched: finding out there is nowhere to keep
    // the originals after a 2 GB download would waste the download.
    let kept_in = match &options.backup_root {
        Some(root) => root.clone(),
        None => default_backup_root()?,
    }
    .join(path_safe(save_id))
    .join(backup_stamp());

    let fetched = fetch_into(
        client,
        save_id,
        version,
        dest,
        staging.path(),
        options,
        progress,
    )
    .await?;

    before_place.await;
    let dest_owned = dest.to_path_buf();
    let staging_path = staging.path().to_path_buf();
    let single_file = fetched.single_file;
    let kept = kept_in.clone();
    let placed = tokio::task::spawn_blocking(move || {
        place_staged(&staging_path, &dest_owned, single_file, &kept)
    })
    .await
    .context("placing the restored files")??;
    tracing::info!(
        save_id,
        version,
        replaced = placed.replaced,
        created = placed.created,
        unchanged = placed.unchanged,
        kept_in = %kept_in.display(),
        "restore: version placed"
    );

    let mut outcome = fetched.outcome;
    outcome.destination = dest.to_path_buf();
    outcome.kept_in = (placed.replaced > 0).then_some(kept_in);
    Ok(outcome)
}

/// Download and verify `version` into `staging`, an empty folder the caller
/// owns, and stop there. The automatic restore merges from it on its own terms
/// (`agent::restore_files_into`, mtime and conflict copies), so it takes the
/// bytes without the placement.
pub(crate) async fn download_into_staging<F>(
    client: &ApiClient,
    save_id: &str,
    version: i64,
    staging: &Path,
    options: RestoreOptions,
    progress: F,
) -> Result<RestoreOutcome>
where
    F: Fn(u64, u64) + Send + Sync + 'static,
{
    Ok(fetch_into(
        client, save_id, version, staging, staging, options, progress,
    )
    .await?
    .outcome)
}

/// A version downloaded and verified into a staging folder.
struct Fetched {
    outcome: RestoreOutcome,
    /// Decided against the real destination, not the staging folder: the
    /// placement has to agree with the gate the download applied.
    single_file: bool,
}

/// Fetch `version` into `out`, deciding shape, gate and the "not empty" refusal
/// against `dest`. The two are the same folder only for the automatic restore,
/// whose `dest` is already its own staging folder.
async fn fetch_into<F>(
    client: &ApiClient,
    save_id: &str,
    version: i64,
    dest: &Path,
    out: &Path,
    options: RestoreOptions,
    progress: F,
) -> Result<Fetched>
where
    F: Fn(u64, u64) + Send + Sync + 'static,
{
    if client.is_cloud().await {
        return download_snapshot_cloud(client, save_id, version, dest, out, options, progress)
            .await;
    }
    fetch_self_hosted(client, save_id, version, dest, out, options, progress).await
}

async fn fetch_self_hosted<F>(
    client: &ApiClient,
    save_id: &str,
    version: i64,
    dest: &Path,
    out: &Path,
    options: RestoreOptions,
    progress: F,
) -> Result<Fetched>
where
    F: Fn(u64, u64) + Send + Sync + 'static,
{
    let started = std::time::Instant::now();
    let detail: SnapshotDetail = client.snapshot_detail(save_id, version).await?;
    let mut expected: HashMap<String, &SnapshotFile> = HashMap::with_capacity(detail.files.len());
    for f in &detail.files {
        let key = manifest_key(&f.relative_path)
            .ok_or_else(|| anyhow!("unsafe path in manifest: {}", f.relative_path))?;
        expected.insert(key, f);
    }

    let names: Vec<&str> = detail
        .files
        .iter()
        .map(|f| f.relative_path.as_str())
        .collect();
    ensure_single_file_shape(dest, &names)?;
    let single_file = extraction_root(dest, &names) != dest;
    ensure_may_write(dest, single_file, options.force)?;

    // What has to arrive: every file of the version, minus the ones the gate
    // keeps off this machine. The server builds the tar from the same rows it
    // listed, so an archive that ends early, repeats a path or brings one of its
    // own is not this version.
    let mut awaited: HashSet<String> = expected
        .keys()
        .filter(|k| single_file || options.gate.allows(k))
        .cloned()
        .collect();
    let mut clash = CaseClash::new(dest);
    let mut listed: Vec<&String> = awaited.iter().collect();
    listed.sort();
    for key in listed {
        clash.check(key)?;
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
        let kind = entry.header().entry_type();

        // A directory entry needs nothing: files create their own parents.
        if kind.is_dir() {
            continue;
        }
        // Sanitize: reject anything that escapes the destination.
        let safe_rel = sanitize(&path_in_tar)
            .ok_or_else(|| anyhow!("unsafe path in archive: {}", path_in_tar.display()))?;
        let key = safe_rel.to_string_lossy().replace('\\', "/");
        // The server only ever writes regular files. A link or a device here
        // used to land as an empty file in place of whatever had that name.
        if !kind.is_file() {
            bail!("archive entry {key} is not a regular file ({kind:?})");
        }
        let Some(meta) = expected.get(&key).copied() else {
            bail!("archive entry {key} is not in v{version}'s file list");
        };
        // Config and litter from the machine that uploaded the snapshot are not
        // written over this one's unless the user asked for it.
        if !single_file && !options.gate.allows(&key) {
            tracing::debug!(path = %key, "restore: skipping device-local file");
            continue;
        }
        if !awaited.remove(&key) {
            bail!("archive carries {key} twice");
        }

        let dest_path = out.join(&safe_rel);
        if let Some(parent) = dest_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("creating parent {}", parent.display()))?;
        }

        // Stream the entry to disk in fixed-size chunks while hashing, instead
        // of buffering the whole file in a Vec. A 2 GB file no longer means
        // 2 GB of RAM. The SHA-256 is computed incrementally over the same
        // bytes we write.
        let mut outf = tokio::fs::File::create(&dest_path)
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
            if !options.skip_verify {
                hasher.update(&buf[..n]);
            }
            outf.write_all(&buf[..n])
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
        outf.flush()
            .await
            .with_context(|| format!("writing {}", dest_path.display()))?;
        drop(outf);
        apply_entry_mtime(&entry, &dest_path);

        if !options.skip_verify {
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

        bytes_extracted += written;
        files_extracted += 1;
    }

    if !awaited.is_empty() {
        let mut missing: Vec<&String> = awaited.iter().collect();
        missing.sort();
        bail!(
            "the archive for v{version} ended without {} of its files (first: {})",
            missing.len(),
            missing[0]
        );
    }

    Ok(Fetched {
        outcome: RestoreOutcome {
            files_extracted,
            bytes_extracted,
            destination: dest.to_path_buf(),
            // A whole tar has no separable phases: download, decompression and
            // writing all happen in the same loop. There is only a total.
            timings: RestoreTimings {
                total_ms: started.elapsed().as_millis() as u64,
                ..Default::default()
            },
            // Whole-archive path: nothing to skip per file.
            files_reused: 0,
            bytes_reused: 0,
            kept_in: None,
        },
        single_file,
    })
}

/// A manifest path as the extractors key it: sanitised, `/`-separated. `None`
/// for a path that would escape the destination.
fn manifest_key(relative_path: &str) -> Option<String> {
    sanitize(Path::new(relative_path)).map(|p| p.to_string_lossy().replace('\\', "/"))
}

/// A single-file save only takes a version whose one entry carries its name.
/// Anything else used to spill into the folder the file sits in, over whatever
/// lived next to it under those names, and without the config gate either,
/// because a single-file save is never gated.
fn ensure_single_file_shape<S: AsRef<str>>(dest: &Path, names: &[S]) -> Result<()> {
    if !dest.is_file() {
        return Ok(());
    }
    let own = dest.file_name().and_then(|s| s.to_str());
    match names {
        [only] if Some(only.as_ref()) == own => Ok(()),
        _ => bail!(
            "{} is a single file, but this version holds {} file(s){}; restore it into a folder instead",
            dest.display(),
            names.len(),
            names
                .first()
                .map(|n| format!(" ({}{})", n.as_ref(), if names.len() > 1 { ", ..." } else { "" }))
                .unwrap_or_default()
        ),
    }
}

/// The refusal to restore over a folder that already has something in it,
/// unless the caller said so. A single-file save that exists needs the same
/// permission: it is exactly what gets replaced.
fn ensure_may_write(dest: &Path, single_file: bool, force: bool) -> Result<()> {
    if force || !dest.exists() {
        return Ok(());
    }
    if !single_file && std::fs::read_dir(dest)?.next().is_none() {
        return Ok(());
    }
    bail!(
        "destination is not empty: {} (set force = true to extract anyway)",
        dest.display()
    );
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
/// hash check happens before anything is extracted.
async fn download_snapshot_cloud<F>(
    client: &ApiClient,
    save_id: &str,
    version: i64,
    dest: &Path,
    out: &Path,
    options: RestoreOptions,
    progress: F,
) -> Result<Fetched>
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
        let mut fetched = restore_cloud_cas(client, dest, out, options, manifest, progress).await?;
        fetched.outcome.timings.manifest_ms = manifest_ms;
        fetched.outcome.timings.total_ms = started.elapsed().as_millis() as u64;
        return Ok(fetched);
    }

    let meta = client.cloud_download(save_id, version).await?;

    // The legacy path (a whole archive, with no per-file manifest): the
    // snapshot's shape is not known in advance, so a single-file save can only be
    // recognised because the file is already on disk. That is enough: only
    // versions uploaded long ago take this path.
    let single_file = extraction_root(dest, &[]) != dest;
    ensure_may_write(dest, single_file, options.force)?;

    // 1. Stream the archive to a temp file, hashing as we go. A name of its own
    // making, created exclusively: a name somebody else can guess is a name
    // somebody else can plant a link on.
    let tmp = tempfile::Builder::new()
        .prefix("hoard-download-")
        .suffix(".tar.zst")
        .tempfile_in(prepared_staging_root(options.staging_root.as_deref())?)
        .context("creating the temp file for the download")?
        .into_temp_path();

    let outcome = download_and_extract_cloud(
        client,
        &meta,
        dest,
        out,
        &tmp,
        single_file,
        &options,
        progress,
    )
    .await;
    drop(tmp);
    outcome.map(|mut outcome| {
        outcome.timings.manifest_ms = manifest_ms;
        outcome.timings.total_ms = started.elapsed().as_millis() as u64;
        Fetched {
            outcome,
            single_file,
        }
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
    let candidates: Vec<crate::backup::UploadFile> = match crate::backup::walk_source(
        dir,
        &crate::backup::SourceFilter::shields_only(shields),
    ) {
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
/// Stream each into `out`, verifying the whole-file sha256 and preserving the
/// recorded mtime (so a cloud pull doesn't always win the conflict-aware diff).
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
    out: &Path,
    options: RestoreOptions,
    manifest: crate::api::CloudVersionManifestOut,
    progress: F,
) -> Result<Fetched>
where
    F: Fn(u64, u64) + Send + Sync + 'static,
{
    let names: Vec<&str> = manifest
        .files
        .iter()
        .map(|f| f.relative_path.as_str())
        .collect();
    ensure_single_file_shape(dest, &names)?;
    let single_file = extraction_root(dest, &names) != dest;
    ensure_may_write(dest, single_file, options.force)?;

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
    // up front. Two entries that land on the same path would race each other
    // into one file, so that aborts too.
    let mut jobs = Vec::with_capacity(kept.len());
    let mut seen: HashSet<PathBuf> = HashSet::with_capacity(kept.len());
    let mut clash = CaseClash::new(dest);
    for file in &kept {
        let safe_rel = sanitize(Path::new(&file.relative_path))
            .ok_or_else(|| anyhow!("unsafe path in manifest: {}", file.relative_path))?;
        if !seen.insert(safe_rel.clone()) {
            bail!("the manifest lists {} twice", file.relative_path);
        }
        clash.check(&file.relative_path)?;
        jobs.push((*file, out.join(&safe_rel)));
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
                // a corrupt file or a failed restore.
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
                        let zstd = file.encoding.as_deref() == Some("zstd");
                        let body =
                            StreamReader::new(resp.bytes_stream().map_err(std::io::Error::other));
                        // The cap is only for the compressed path: there a small
                        // body can become a large file, and `size_bytes` says
                        // exactly how large it is allowed to get. Raw bodies are
                        // already their own size and have never been capped, so
                        // leave that alone.
                        let cap = zstd.then(|| file.size_bytes.max(0) as u64);
                        let got = write_blob_body(
                            body,
                            zstd,
                            cap,
                            dest_path,
                            !options.skip_verify,
                            &|n| {
                                let done = landed.fetch_add(n, Ordering::Relaxed) + n;
                                progress(done, total);
                            },
                        )
                        .await?;
                        if let Some(got) = got {
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

    Ok(Fetched {
        outcome: RestoreOutcome {
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
            kept_in: None,
        },
        single_file,
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

#[allow(clippy::too_many_arguments)]
async fn download_and_extract_cloud<F>(
    client: &ApiClient,
    meta: &crate::api::CloudDownloadOut,
    dest: &Path,
    out: &Path,
    tmp: &Path,
    single_file: bool,
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

    let mut archive_file = tokio::fs::File::create(tmp)
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
        archive_file
            .write_all(&chunk)
            .await
            .with_context(|| format!("writing {}", tmp.display()))?;
        downloaded += chunk.len() as u64;
        progress(downloaded, total);
    }
    archive_file
        .flush()
        .await
        .context("flushing temp archive")?;
    drop(archive_file);

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
    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut clash = CaseClash::new(dest);

    while let Some(entry) = entries.next().await {
        let mut entry = entry.context("reading tar entry")?;
        let path_in_tar = entry.path()?.into_owned();
        let kind = entry.header().entry_type();

        if kind.is_dir() {
            continue;
        }
        let safe_rel = sanitize(&path_in_tar)
            .ok_or_else(|| anyhow!("unsafe path in archive: {}", path_in_tar.display()))?;
        // The client that built these archives only ever added regular files.
        if !kind.is_file() {
            bail!(
                "archive entry {} is not a regular file ({kind:?})",
                safe_rel.display()
            );
        }
        if !seen.insert(safe_rel.clone()) {
            bail!("archive carries {} twice", safe_rel.display());
        }
        let dest_path = out.join(&safe_rel);

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
        clash.check(&safe_rel.to_string_lossy())?;

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
        kept_in: None,
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

// ---- staging and placement

/// Staging left behind longer than this is from a restore that died without
/// cleaning up (a crash, a power cut). No restore runs for a week.
const STALE_STAGING: std::time::Duration = std::time::Duration::from_secs(7 * 24 * 3600);

/// Where restores stage their bytes: under Hoard's own state folder, not the
/// system temp. `/tmp` is RAM on Fedora and Arch, among others, so a save bigger
/// than what it can hold could not be restored at all; the state folder is on
/// disk, and usually on the same one as the saves, so placing a file is a rename.
fn staging_root() -> PathBuf {
    crate::config::CliConfig::state_dir()
        .map(|d| d.join("restore-staging"))
        .unwrap_or_else(|_| std::env::temp_dir())
}

/// Best-effort sweep of staging a dead restore left behind. The system temp
/// was cleaned by the OS; this folder is ours to clean.
fn sweep_stale_staging(root: &Path) {
    let Ok(read) = std::fs::read_dir(root) else {
        return;
    };
    for entry in read.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with("hoard-restore-") && !name.starts_with("hoard-download-") {
            continue;
        }
        let stale = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age > STALE_STAGING);
        if stale {
            let path = entry.path();
            let gone = if path.is_dir() {
                std::fs::remove_dir_all(&path)
            } else {
                std::fs::remove_file(&path)
            };
            match gone {
                Ok(()) => tracing::info!(path = %path.display(), "restore: removed stale staging"),
                Err(e) => {
                    tracing::warn!(path = %path.display(), error = %e, "restore: could not remove stale staging")
                }
            }
        }
    }
}

/// The staging root (`custom`, or [`staging_root`]), created and swept, ready
/// for a new entry.
fn prepared_staging_root(custom: Option<&Path>) -> Result<PathBuf> {
    let root = custom.map(Path::to_path_buf).unwrap_or_else(staging_root);
    std::fs::create_dir_all(&root)
        .with_context(|| format!("creating the restore staging folder {}", root.display()))?;
    sweep_stale_staging(&root);
    Ok(root)
}

/// A private folder for one restore's bytes: a name nobody can guess, created
/// exclusively and, on Unix, readable by this user only, so nobody can plant a
/// file or a link in it first.
pub(crate) fn new_staging_dir() -> Result<tempfile::TempDir> {
    new_staging_dir_in(None)
}

/// [`new_staging_dir`] under `root` instead of the default staging root.
fn new_staging_dir_in(root: Option<&Path>) -> Result<tempfile::TempDir> {
    let root = prepared_staging_root(root)?;
    let mut builder = tempfile::Builder::new();
    builder.prefix("hoard-restore-");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(std::fs::Permissions::from_mode(0o700));
    }
    builder
        .tempdir_in(&root)
        .context("creating a private staging folder for the restore")
}

/// The staging folder must not sit inside the folder being restored: the
/// watcher would see the download as the user's own writes.
fn ensure_outside(staging: &Path, dest: &Path) -> Result<()> {
    let dest = std::fs::canonicalize(dest).unwrap_or_else(|_| dest.to_path_buf());
    let staging = std::fs::canonicalize(staging).unwrap_or_else(|_| staging.to_path_buf());
    if staging.starts_with(&dest) {
        bail!(
            "Hoard's staging folder {} is inside {}; restoring over Hoard's own data is not supported",
            staging.display(),
            dest.display()
        );
    }
    Ok(())
}

fn default_backup_root() -> Result<PathBuf> {
    crate::config::CliConfig::state_dir()
        .map(|d| d.join("conflicts"))
        .context("finding where to keep the files the restore replaces")
}

/// `save_id` as one path component. It reaches here from the command line
/// too, and `..` in it must not walk out of the backup root.
fn path_safe(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// The timestamp folder under `<save_id>/`, in the shape the automatic restore
/// uses, so `agent::cleanup_old_conflicts` expires both alike.
fn backup_stamp() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "unknown-ts".to_string())
        .replace(':', "-")
}

/// What placing a staged version did to the destination.
#[derive(Debug, Default)]
struct Placed {
    replaced: usize,
    created: usize,
    unchanged: usize,
}

/// One file moved into the destination, remembered so it can be taken back.
enum Undo {
    Created(PathBuf),
    Replaced { target: PathBuf, kept: PathBuf },
}

/// Move a verified version from `staging` into `dest`, all or nothing.
///
/// Every file the version would change is first put aside in `kept_in` (see
/// [`keep_original`]), and only once all of them are, and on disk, is each one
/// replaced in a single rename, so at any instant each path holds either the
/// old file or the new one, whole. If any file cannot be placed (a game
/// holding it open on Windows, a full disk, a folder where the version has a
/// file) the ones already placed are put back and the error says so. Files the
/// folder has and the version does not are left alone, as they always were.
///
/// Blocking IO throughout: it runs on `spawn_blocking`.
fn place_staged(staging: &Path, dest: &Path, single_file: bool, kept_in: &Path) -> Result<Placed> {
    let staged = list_staged(staging)?;
    let names: Vec<String> = staged
        .iter()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .collect();
    let root = if single_file {
        ensure_single_file_shape(dest, &names)?;
        match dest.parent() {
            Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
            _ => PathBuf::from("."),
        }
    } else {
        dest.to_path_buf()
    };

    // Planned in full before anything moves, so the cheap refusals cost nothing.
    let in_prefix = in_wine_prefix(&root);
    let mut plan = Vec::with_capacity(staged.len());
    for rel in &staged {
        let from = staging.join(rel);
        let target = through_link(dest_in(&root, rel, in_prefix))?;
        if target.is_dir() {
            bail!(
                "{} is a folder here, but a file in this version",
                target.display()
            );
        }
        let same = target.is_file()
            && same_bytes(&from, &target)
                .with_context(|| format!("comparing {} with the version", target.display()))?;
        plan.push((rel, from, target, same));
    }

    let mut made_dirs: Vec<PathBuf> = Vec::new();
    // An empty version still leaves its folder, as extracting into it did.
    if !single_file {
        create_dirs_tracked(&root, &mut made_dirs)?;
    }
    let mut placed = Placed::default();
    let mut restamp: Vec<(PathBuf, PathBuf)> = Vec::new();

    // 1. The way back, before anything changes: every file about to be replaced
    // is put aside, and it is on disk before the first rename.
    let mut steps: Vec<Step> = Vec::new();
    let mut kept_files: Vec<PathBuf> = Vec::new();
    for (rel, from, target, same) in plan {
        if same {
            placed.unchanged += 1;
            restamp.push((from, target));
            continue;
        }
        let original = match std::fs::metadata(&target) {
            Ok(meta) => Some(meta),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => {
                discard_kept(&kept_files);
                roll_back(Vec::new(), made_dirs);
                return Err(e).with_context(|| format!("reading {}", target.display()));
            }
        };
        let kept = match &original {
            Some(meta) => {
                let kept = kept_in.join(rel);
                if let Err(e) = keep_original(&target, &kept, meta) {
                    discard_kept(&kept_files);
                    roll_back(Vec::new(), made_dirs);
                    return Err(e.context("restore stopped before changing anything"));
                }
                kept_files.push(kept.clone());
                Some(kept)
            }
            None => None,
        };
        steps.push(Step {
            from,
            target,
            kept,
            perms: original.map(|m| m.permissions()),
        });
    }
    // The staged bytes reach the disk before a rename can point the save at
    // them: a power cut right after a rename must not leave an empty file where
    // the save was.
    let staged: Vec<&Path> = steps.iter().map(|s| s.from.as_path()).collect();
    if let Err(e) = sync_files(&staged) {
        discard_kept(&kept_files);
        roll_back(Vec::new(), made_dirs);
        return Err(anyhow::Error::new(e).context("writing the restored files to disk"));
    }
    sync_dirs(kept_files.iter().filter_map(|k| k.parent()));

    // 2. The swap.
    let mut done: Vec<Undo> = Vec::new();
    for step in steps {
        match place_one(&step, &mut made_dirs) {
            Ok(undo) => {
                match undo {
                    Undo::Created(_) => placed.created += 1,
                    Undo::Replaced { .. } => placed.replaced += 1,
                }
                done.push(undo);
            }
            Err(e) => {
                let left = roll_back(done, made_dirs);
                if left.is_empty() {
                    return Err(e.context("restore undone, the folder is as it was"));
                }
                tracing::error!(
                    kept_in = %kept_in.display(),
                    failures = ?left,
                    "restore: could not put every file back"
                );
                return Err(e.context(format!(
                    "restore failed and {} file(s) could not be put back; the originals are in {}",
                    left.len(),
                    kept_in.display()
                )));
            }
        }
    }

    // Unchanged files only take the version's mtime once nothing can be undone,
    // so a rollback never leaves one of them touched.
    for (from, target) in restamp {
        copy_mtime(&from, &target);
    }
    // The renames themselves survive a power cut, not just the bytes they
    // point at.
    sync_dirs(done.iter().filter_map(|undo| {
        let (Undo::Created(t) | Undo::Replaced { target: t, .. }) = undo;
        t.parent()
    }));
    Ok(placed)
}

/// One file of the version on its way into the destination.
struct Step {
    from: PathBuf,
    target: PathBuf,
    /// Where the file it replaces was put aside; `None` when there was none.
    kept: Option<PathBuf>,
    perms: Option<std::fs::Permissions>,
}

/// Every regular file under `staging`, relative to it, sorted.
fn list_staged(staging: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut stack = vec![staging.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in
            std::fs::read_dir(&dir).with_context(|| format!("reading {}", dir.display()))?
        {
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_dir() {
                stack.push(entry.path());
            } else if kind.is_file() {
                let path = entry.path();
                files.push(path.strip_prefix(staging)?.to_path_buf());
            } else {
                bail!(
                    "unexpected entry in the staging folder: {}",
                    entry.path().display()
                );
            }
        }
    }
    files.sort();
    Ok(files)
}

/// A save that is a link (to an SD card, to a synced folder) keeps being a
/// link: the new bytes go where it points. Only the last component is followed;
/// linked folders on the way are followed by the OS anyway.
fn through_link(path: PathBuf) -> Result<PathBuf> {
    match std::fs::symlink_metadata(&path) {
        Ok(meta) if meta.file_type().is_symlink() => std::fs::canonicalize(&path)
            .with_context(|| format!("{} is a link to something that isn't there", path.display())),
        _ => Ok(path),
    }
}

/// Swap the staged file in over `target`; the original is already in `kept`.
fn place_one(step: &Step, made_dirs: &mut Vec<PathBuf>) -> Result<Undo> {
    if let Some(parent) = step.target.parent() {
        create_dirs_tracked(parent, made_dirs)?;
    }
    replace_with(&step.from, &step.target, step.perms.clone()).with_context(|| {
        format!(
            "replacing {} (if the game is running, close it and try again)",
            step.target.display()
        )
    })?;
    Ok(match &step.kept {
        Some(kept) => Undo::Replaced {
            target: step.target.clone(),
            kept: kept.clone(),
        },
        None => Undo::Created(step.target.clone()),
    })
}

/// Put the file at `target` aside as `kept` before the restore replaces it. A
/// hard link when the two share a disk: instant, and no second copy of a
/// multi-GB file where space may already be short. A copy otherwise, written
/// to disk before this returns. The original keeps its mtime either way, so
/// putting it back puts back what the next merge compares against.
fn keep_original(target: &Path, kept: &Path, meta: &std::fs::Metadata) -> Result<()> {
    if let Some(parent) = kept.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    if std::fs::hard_link(target, kept).is_ok() {
        return Ok(());
    }
    let copied = (|| -> std::io::Result<()> {
        // By hand rather than `fs::copy`, which would carry a read-only mode
        // over before the file could be flushed through a handle that writes.
        let mut src = std::fs::File::open(target)?;
        let mut dst = std::fs::File::create(kept)?;
        std::io::copy(&mut src, &mut dst)?;
        sync_handle(&dst)?;
        drop(dst);
        if let Ok(mtime) = meta.modified() {
            filetime::set_file_mtime(kept, filetime::FileTime::from_system_time(mtime))?;
        }
        std::fs::set_permissions(kept, meta.permissions())
    })();
    if let Err(e) = copied {
        let _ = std::fs::remove_file(kept);
        return Err(e).with_context(|| {
            format!("keeping a copy of {} before replacing it", target.display())
        });
    }
    Ok(())
}

/// Take back the copies a placement that never started had put aside.
fn discard_kept(kept: &[PathBuf]) {
    for k in kept {
        let _ = std::fs::remove_file(k);
    }
}

/// `fsync` through `file`. A filesystem that cannot (some FUSE and network
/// mounts answer "not supported") is let through: it never could, and
/// refusing would make restoring there impossible.
fn sync_handle(file: &std::fs::File) -> std::io::Result<()> {
    match file.sync_all() {
        Err(e)
            if matches!(
                e.kind(),
                std::io::ErrorKind::Unsupported | std::io::ErrorKind::InvalidInput
            ) =>
        {
            Ok(())
        }
        other => other,
    }
}

/// `fsync` the files we wrote, a few at a time: one after another, a save of
/// thousands of small files spent seconds on it on an SD card. Opened for
/// writing because Windows only flushes through a handle that may write.
fn sync_files(paths: &[&Path]) -> std::io::Result<()> {
    const WORKERS: usize = 8;
    if paths.is_empty() {
        return Ok(());
    }
    let per = paths.len().div_ceil(WORKERS);
    std::thread::scope(|scope| {
        let workers: Vec<_> = paths
            .chunks(per)
            .map(|chunk| {
                scope.spawn(move || -> std::io::Result<()> {
                    for path in chunk {
                        let file = std::fs::OpenOptions::new().write(true).open(path)?;
                        sync_handle(&file)?;
                    }
                    Ok(())
                })
            })
            .collect();
        workers.into_iter().try_for_each(|w| {
            w.join()
                .unwrap_or_else(|_| Err(std::io::Error::other("sync worker panicked")))
        })
    })
}

/// Best-effort `fsync` of each distinct folder, so the names created or
/// renamed in it survive a power cut too. Unix only: Windows has no handle on
/// a folder to flush, and NTFS journals its renames.
fn sync_dirs<'a>(dirs: impl Iterator<Item = &'a Path>) {
    #[cfg(unix)]
    {
        let unique: HashSet<&Path> = dirs.collect();
        for dir in unique {
            let _ = std::fs::File::open(dir).and_then(|d| d.sync_all());
        }
    }
    #[cfg(not(unix))]
    {
        let _ = dirs;
    }
}

/// Put `from` at `target` in one step. A plain rename when both share a
/// filesystem; otherwise a copy into a temp file beside the target, flushed,
/// then renamed over it. Either way `target` is never seen half written, and on
/// failure it is left as it was. `from`'s mtime travels with it.
fn replace_with(from: &Path, target: &Path, perms: Option<std::fs::Permissions>) -> Result<()> {
    if std::fs::rename(from, target).is_ok() {
        if let Some(p) = perms {
            let _ = std::fs::set_permissions(target, p);
        }
        return Ok(());
    }
    let tmp = temp_beside(target);
    let copied = (|| -> std::io::Result<()> {
        // By hand rather than `fs::copy`, which would carry a read-only mode
        // over before the flush below could open the file for writing.
        let mut src = std::fs::File::open(from)?;
        let mut dst = std::fs::File::create(&tmp)?;
        std::io::copy(&mut src, &mut dst)?;
        sync_handle(&dst)?;
        drop(dst);
        let meta = std::fs::metadata(from)?;
        filetime::set_file_mtime(&tmp, filetime::FileTime::from_system_time(meta.modified()?))?;
        std::fs::set_permissions(&tmp, perms.unwrap_or_else(|| meta.permissions()))?;
        std::fs::rename(&tmp, target)
    })();
    if let Err(e) = copied {
        let _ = std::fs::remove_file(&tmp);
        return Err(e.into());
    }
    Ok(())
}

/// `.<name>.<pid>-<seq>.hoard.tmp` next to `target`. The `.tmp` extension is
/// junk to the backup walk, so one left behind by a crash is never uploaded.
fn temp_beside(target: &Path) -> PathBuf {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let name = target
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".to_string());
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    target.with_file_name(format!(".{name}.{}-{seq}.hoard.tmp", std::process::id()))
}

/// `create_dir_all`, remembering which folders it made so a rollback can take
/// them away again.
fn create_dirs_tracked(dir: &Path, made: &mut Vec<PathBuf>) -> Result<()> {
    if dir.is_dir() {
        return Ok(());
    }
    let mut missing = Vec::new();
    let mut cur = Some(dir);
    while let Some(d) = cur {
        if d.as_os_str().is_empty() || d.exists() {
            break;
        }
        missing.push(d.to_path_buf());
        cur = d.parent();
    }
    for d in missing.into_iter().rev() {
        std::fs::create_dir(&d).with_context(|| format!("creating {}", d.display()))?;
        made.push(d);
    }
    Ok(())
}

/// Undo a partial placement, newest first. Returns what could not be undone.
fn roll_back(done: Vec<Undo>, made_dirs: Vec<PathBuf>) -> Vec<String> {
    let mut left = Vec::new();
    for undo in done.into_iter().rev() {
        match undo {
            Undo::Created(target) => {
                if let Err(e) = std::fs::remove_file(&target) {
                    left.push(format!("{}: {e}", target.display()));
                }
            }
            Undo::Replaced { target, kept } => {
                if let Err(e) = replace_with(&kept, &target, None) {
                    left.push(format!("{}: {e:#}", target.display()));
                }
            }
        }
    }
    // Only empty ones go: `remove_dir` refuses anything else.
    for dir in made_dirs.into_iter().rev() {
        let _ = std::fs::remove_dir(&dir);
    }
    left
}

fn same_bytes(a: &Path, b: &Path) -> std::io::Result<bool> {
    use std::io::Read;
    if std::fs::metadata(a)?.len() != std::fs::metadata(b)?.len() {
        return Ok(false);
    }
    let mut fa = std::io::BufReader::new(std::fs::File::open(a)?);
    let mut fb = std::io::BufReader::new(std::fs::File::open(b)?);
    let mut ba = vec![0u8; 64 * 1024];
    let mut bb = vec![0u8; 64 * 1024];
    loop {
        let n = fa.read(&mut ba)?;
        if n == 0 {
            return Ok(true);
        }
        fb.read_exact(&mut bb[..n])?;
        if ba[..n] != bb[..n] {
            return Ok(false);
        }
    }
}

fn copy_mtime(from: &Path, to: &Path) {
    if let Ok(mtime) = std::fs::metadata(from).and_then(|m| m.modified()) {
        let _ = filetime::set_file_mtime(to, filetime::FileTime::from_system_time(mtime));
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

/// Whether two of a version's paths that differ only in case name one file
/// where this restore writes. Windows and macOS fold case by default (and the
/// staging folder is on the same kind of disk), and so does a Wine prefix
/// through [`dest_in`].
fn folds_case(dest: &Path) -> bool {
    cfg!(any(windows, target_os = "macos")) || in_wine_prefix(dest)
}

/// Refuses a path that names the same file as an earlier one once case is
/// folded. A version uploaded from Linux can hold `Save.sav` and `save.sav`;
/// restored where case folds, both would be written into one staged file, the
/// bytes of one over the other's, while each download checked its own stream
/// and not what ended up on disk. Exact repeats are refused by the callers,
/// with their own words.
struct CaseClash {
    fold: bool,
    seen: HashMap<String, String>,
}

impl CaseClash {
    fn new(dest: &Path) -> Self {
        Self {
            fold: folds_case(dest),
            seen: HashMap::new(),
        }
    }

    fn check(&mut self, rel: &str) -> Result<()> {
        if !self.fold {
            return Ok(());
        }
        let rel = rel.replace('\\', "/");
        match self.seen.get(&rel.to_lowercase()) {
            Some(first) if *first != rel => bail!(
                "this version holds both {first} and {rel}, which are the same file here; \
                 restore it somewhere that tells capitals apart"
            ),
            Some(_) => Ok(()),
            None => {
                self.seen.insert(rel.to_lowercase(), rel);
                Ok(())
            }
        }
    }
}

/// Whether `root` lives inside a Wine/Proton prefix, or is one.
pub(crate) fn in_wine_prefix(root: &Path) -> bool {
    root.components().any(|c| {
        c.as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case("drive_c")
    }) || crate::junkdirs::folder_kind(root) == Some(crate::junkdirs::FolderKind::WinePrefix)
}

/// Where `rel` lands under `root`. Inside a Wine/Proton prefix a component that
/// already exists with other capitals is reused: the game running under Wine
/// sees `Savegame.sav` and `SaveGame.sav` as one file, Linux sees two, and
/// restoring next to the one the game uses left it loading its own save while
/// the synced one sat beside it (reported with Silent Hill: Townfall on two
/// Bazzite machines, each with its own spelling). Outside a prefix, a plain join.
/// `in_prefix` is [`in_wine_prefix`] of `root`, worked out once per restore by
/// the caller rather than once per file.
pub(crate) fn dest_in(root: &Path, rel: &Path, in_prefix: bool) -> PathBuf {
    if !in_prefix {
        return root.join(rel);
    }
    let mut cur = root.to_path_buf();
    for comp in rel.components() {
        let exact = cur.join(comp.as_os_str());
        if exact.exists() {
            cur = exact;
            continue;
        }
        let want = comp.as_os_str().to_string_lossy().to_lowercase();
        let found = std::fs::read_dir(&cur).ok().and_then(|read| {
            read.flatten()
                .find(|e| e.file_name().to_string_lossy().to_lowercase() == want)
                .map(|e| e.path())
        });
        cur = found.unwrap_or(exact);
    }
    cur
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sweep_takes_only_week_old_staging_of_ours() {
        let root = tempfile::tempdir().unwrap();
        let week_ago = filetime::FileTime::from_system_time(
            std::time::SystemTime::now() - STALE_STAGING - std::time::Duration::from_secs(60),
        );
        let old_dir = root.path().join("hoard-restore-old");
        let old_file = root.path().join("hoard-download-old.tar.zst");
        let fresh = root.path().join("hoard-restore-fresh");
        let foreign = root.path().join("something-else");
        std::fs::create_dir_all(old_dir.join("sub")).unwrap();
        std::fs::write(old_dir.join("sub/f"), b"x").unwrap();
        std::fs::write(&old_file, b"x").unwrap();
        std::fs::create_dir_all(&fresh).unwrap();
        std::fs::create_dir_all(&foreign).unwrap();
        for p in [&old_dir, &old_file, &foreign] {
            filetime::set_file_mtime(p, week_ago).unwrap();
        }

        sweep_stale_staging(root.path());

        assert!(!old_dir.exists() && !old_file.exists());
        assert!(fresh.exists(), "a restore running now keeps its folder");
        assert!(foreign.exists(), "only our own names are touched");
    }

    #[test]
    fn dest_in_reuses_the_spelling_on_disk_inside_a_prefix() {
        let tmp = tempfile::tempdir().unwrap();
        let saves = tmp
            .path()
            .join("pfx/drive_c/users/steamuser/AppData/Local/Townfall/Saved/SaveGames");
        std::fs::create_dir_all(&saves).unwrap();
        std::fs::write(saves.join("SaveGame.sav"), b"machine 2").unwrap();
        assert_eq!(
            dest_in(&saves, Path::new("Savegame.sav"), in_wine_prefix(&saves)),
            saves.join("SaveGame.sav")
        );
        // A file that does not exist in any spelling keeps its own.
        assert_eq!(
            dest_in(&saves, Path::new("Other.sav"), in_wine_prefix(&saves)),
            saves.join("Other.sav")
        );
        // Outside a prefix Linux's rules stand.
        let native = tmp.path().join("native");
        std::fs::create_dir_all(&native).unwrap();
        std::fs::write(native.join("SaveGame.sav"), b"x").unwrap();
        assert_eq!(
            dest_in(&native, Path::new("Savegame.sav"), in_wine_prefix(&native)),
            native.join("Savegame.sav")
        );
    }

    // ---- placement

    fn read(path: &Path) -> Vec<u8> {
        std::fs::read(path).unwrap()
    }

    #[test]
    fn placement_keeps_what_it_replaces_and_leaves_the_rest_alone() {
        let tmp = tempfile::tempdir().unwrap();
        let staging = tmp.path().join("staging");
        let dest = tmp.path().join("Saves");
        let kept = tmp.path().join("kept");
        seed(&staging, "slot1.sav", b"slot 1, from the version");
        seed(&staging, "sub/slot2.sav", b"slot 2, new here");
        seed(&staging, "same.sav", b"identical");
        seed(&dest, "slot1.sav", b"slot 1, local");
        seed(&dest, "same.sav", b"identical");
        seed(&dest, "local-only.sav", b"not in the version");

        let placed = place_staged(&staging, &dest, false, &kept).unwrap();

        assert_eq!(
            (placed.replaced, placed.created, placed.unchanged),
            (1, 1, 1)
        );
        assert_eq!(read(&dest.join("slot1.sav")), b"slot 1, from the version");
        assert_eq!(read(&dest.join("sub/slot2.sav")), b"slot 2, new here");
        assert_eq!(read(&dest.join("local-only.sav")), b"not in the version");
        // The way back: only what was replaced, byte for byte.
        assert_eq!(read(&kept.join("slot1.sav")), b"slot 1, local");
        assert!(!kept.join("same.sav").exists());
    }

    /// A file that cannot be placed halfway through puts back the ones already
    /// placed, their mtime included: the folder ends as it began.
    #[cfg(unix)]
    #[test]
    fn a_failed_placement_puts_the_folder_back() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let staging = tmp.path().join("staging");
        let dest = tmp.path().join("Saves");
        let kept = tmp.path().join("kept");
        seed(&staging, "a.sav", b"a from the version");
        seed(&staging, "b.sav", b"b, created by the version");
        seed(&staging, "z/locked.sav", b"never lands");
        seed(&dest, "a.sav", b"a, the good local save");
        seed(&dest, "z/locked.sav", b"the locked one");
        let old = filetime::FileTime::from_unix_time(1_700_000_000, 0);
        filetime::set_file_mtime(dest.join("a.sav"), old).unwrap();
        // A folder nobody may write in stands for a file the game holds open.
        let locked = dest.join("z");
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o555)).unwrap();
        if std::fs::write(locked.join("probe"), b"").is_ok() {
            // Root ignores the mode; there is nothing to prove here.
            std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
            return;
        }

        let err = place_staged(&staging, &dest, false, &kept).unwrap_err();
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();

        assert!(
            format!("{err:#}").contains("the folder is as it was"),
            "{err:#}"
        );
        assert_eq!(read(&dest.join("a.sav")), b"a, the good local save");
        let mtime = filetime::FileTime::from_last_modification_time(
            &std::fs::metadata(dest.join("a.sav")).unwrap(),
        );
        assert_eq!(mtime, old);
        assert!(!dest.join("b.sav").exists());
        assert_eq!(read(&dest.join("z/locked.sav")), b"the locked one");
    }

    /// A save that is a link keeps being one; the bytes go where it points.
    #[cfg(unix)]
    #[test]
    fn placement_writes_through_a_linked_file() {
        let tmp = tempfile::tempdir().unwrap();
        let staging = tmp.path().join("staging");
        let dest = tmp.path().join("Saves");
        let card = tmp.path().join("sdcard");
        seed(&staging, "slot.sav", b"new");
        seed(&card, "slot.sav", b"old");
        std::fs::create_dir_all(&dest).unwrap();
        std::os::unix::fs::symlink(card.join("slot.sav"), dest.join("slot.sav")).unwrap();

        place_staged(&staging, &dest, false, &tmp.path().join("kept")).unwrap();

        assert!(std::fs::symlink_metadata(dest.join("slot.sav"))
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(read(&card.join("slot.sav")), b"new");
    }

    /// Point a single-file save at a version of a folder and nothing lands next
    /// to it: it used to spill every entry into the file's parent.
    #[test]
    fn a_single_file_save_refuses_a_version_of_several_files() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("save.dat");
        std::fs::write(&file, b"x").unwrap();
        assert!(ensure_single_file_shape(&file, &["save.dat"]).is_ok());
        assert!(ensure_single_file_shape(&file, &["save.dat", "other.dat"]).is_err());
        assert!(ensure_single_file_shape(&file, &["other.dat"]).is_err());
        // Not a file: nothing to check.
        assert!(ensure_single_file_shape(tmp.path(), &["a", "b"]).is_ok());
    }

    #[test]
    fn a_staging_folder_inside_the_destination_is_refused() {
        let tmp = tempfile::tempdir().unwrap();
        let inside = tmp.path().join("staging");
        std::fs::create_dir(&inside).unwrap();
        assert!(ensure_outside(&inside, tmp.path()).is_err());
        assert!(ensure_outside(tmp.path(), &inside).is_ok());
    }

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
            encoding: None,
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

#[cfg(test)]
mod blob_body_tests {
    use super::*;

    /// zstd-compress in memory, the way the server's sweep would have.
    async fn squash(raw: &[u8]) -> Vec<u8> {
        let mut enc = async_compression::tokio::write::ZstdEncoder::new(Vec::new());
        enc.write_all(raw).await.expect("compress");
        enc.shutdown().await.expect("finish");
        enc.into_inner()
    }

    fn sha_of(raw: &[u8]) -> String {
        hex::encode(Sha256::digest(raw))
    }

    /// The whole point of letting the client fetch compressed blobs straight
    /// from storage: what lands on disk is the raw file, and the sha that gets
    /// verified is the raw file's. Hash the compressed bytes by mistake and
    /// every restore fails; write them by mistake and the save is corrupt.
    #[tokio::test]
    async fn a_compressed_body_lands_raw_and_hashes_raw() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("save.dat");
        let raw = b"a save file that repeats itself a save file that repeats itself".repeat(50);
        let body = squash(&raw).await;
        assert!(
            body.len() < raw.len(),
            "the fixture has to actually compress"
        );

        let seen = std::sync::atomic::AtomicU64::new(0);
        let got = write_blob_body(
            body.as_slice(),
            true,
            Some(raw.len() as u64),
            &dest,
            true,
            &|n| {
                seen.fetch_add(n, Ordering::Relaxed);
            },
        )
        .await
        .expect("decodes");

        assert_eq!(tokio::fs::read(&dest).await.unwrap(), raw, "raw on disk");
        assert_eq!(got.as_deref(), Some(sha_of(&raw).as_str()), "raw sha");
        assert_eq!(
            seen.load(Ordering::Relaxed),
            raw.len() as u64,
            "progress counts what the user gets, not what crossed the wire"
        );
    }

    /// A raw body still goes through untouched, which is every blob today that
    /// was never worth compressing and every response from an older server.
    #[tokio::test]
    async fn a_raw_body_is_passed_straight_through() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("save.dat");
        let raw = b"not compressed at all".to_vec();

        let got = write_blob_body(raw.as_slice(), false, None, &dest, true, &|_| {})
            .await
            .expect("copies");

        assert_eq!(tokio::fs::read(&dest).await.unwrap(), raw);
        assert_eq!(got.as_deref(), Some(sha_of(&raw).as_str()));
    }

    /// The bomb guard. A blob that keeps expanding past its declared size is
    /// corrupt or hostile, and it has to stop before it fills the disk rather
    /// than after, which is all the sha check on its own would give us.
    #[tokio::test]
    async fn a_body_that_expands_past_its_declared_size_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("save.dat");
        // A megabyte of zeroes compresses to almost nothing: the shape of a
        // decompression bomb, in miniature.
        let raw = vec![0u8; 1024 * 1024];
        let body = squash(&raw).await;

        let err = write_blob_body(body.as_slice(), true, Some(1024), &dest, true, &|_| {})
            .await
            .expect_err("must refuse");
        assert!(
            format!("{err:#}").contains("expanded past its declared"),
            "unexpected error: {err:#}"
        );
    }

    /// Skipping verification skips the hash, not the writing.
    #[tokio::test]
    async fn skipping_verification_still_writes_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("save.dat");
        let raw = b"bytes".repeat(100);

        let got = write_blob_body(
            squash(&raw).await.as_slice(),
            true,
            Some(raw.len() as u64),
            &dest,
            false,
            &|_| {},
        )
        .await
        .expect("decodes");

        assert!(got.is_none(), "no sha was asked for");
        assert_eq!(tokio::fs::read(&dest).await.unwrap(), raw);
    }
}
