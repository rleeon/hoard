//! Library half of the snapshot upload flow.
//!
//! The CLI and the desktop app both want to walk a directory, build a
//! multipart body, and POST it to the server. The two diverge on how they
//! present progress (indicatif progress bar vs a Tauri event stream), so we
//! expose the work as an async function with a `progress` callback.
//!
//! State-file bookkeeping (`saves` map in `state.json`) lives here too so the
//! GUI gets it for free.

use anyhow::{anyhow, bail, Context, Result};
use futures::stream::{self, StreamExt, TryStreamExt};
use futures::FutureExt;
use reqwest::multipart;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use time::OffsetDateTime;
use tokio::io::AsyncReadExt;

use crate::api::{
    ApiClient, ApiError, CasCommit, CasFile, CasInit, CloudCasFileEntry, CloudCasInit,
    CloudCasMissingBlob, RateLimitKind, Snapshot,
};
use crate::state::{CliState, SaveState};
use hoard_core::ids::SaveId;
use hoard_core::kernel::fileclass;
use hoard_core::wire::VersionOrigin;

/// Bounded fan-out for per-file work in the cloud path (hashing local files,
/// PUTting missing blobs). Saves are mostly many small files, so per-file
/// open/read and R2 round-trip latency dominates over raw throughput; a small
/// window hides that latency without saturating the disk or the uplink.
const TRANSFER_CONCURRENCY: usize = 4;

/// How many times one blob may be turned away by a request pacer before we stop
/// and let the whole attempt fail.
///
/// This exists so a genuinely misconfigured server (an operator who sets
/// `per_second = 1`, or a proxy that refuses everything) fails fast with a clear
/// error instead of crawling for an hour pretending to work.
const MAX_PACED_RETRIES_PER_BLOB: u32 = 6;

/// Shortest we'll wait after being paced, and the base of the per-blob backoff.
///
/// The pacer's own hint is in whole seconds, so at any sane rate limit it says
/// `0`, true but not something to act on literally. Four workers each waiting
/// ~200 ms converges on roughly 20 requests a second, which is the default limit
/// the server is actually enforcing.
const PACED_RETRY_FLOOR: Duration = Duration::from_millis(200);

/// Ceiling for a single paced wait. Past this, a "slow down" is better handled
/// by failing the attempt and re-arming on the agent's long backoff.
const PACED_RETRY_CEILING: Duration = Duration::from_secs(10);

/// Total time one upload may spend sitting in pacer waits, summed across all
/// its blobs and all workers.
///
/// Summed rather than wall-clock on purpose: wall-clock would also count the
/// transfer itself, so a legitimately slow 4 GB upload would abort the moment
/// anything paced it. This counts only time actually spent blocked.
/// Generous, because the per-blob cap above is what really guards against a
/// hostile server; this one only has to stop a huge folder from crawling
/// indefinitely against a very tight limit. A folder of a few thousand small
/// files paced at 20 requests a second legitimately spends minutes here, and
/// aborting that would be the same bug in a new hat.
const PACED_WAIT_BUDGET: Duration = Duration::from_secs(900);

/// The wait a pacer asked for, if this error is one.
///
/// Only [`RateLimitKind::Paced`] retries here. A budget 429 (bandwidth window,
/// storage quota, loop brake) means the operation does not fit right now, and
/// re-sending the same PUT can only make it worse; those keep travelling up to
/// the agent, which parks the save and comes back later.
fn paced_wait_hint(e: &anyhow::Error) -> Option<u32> {
    if let Some(hint) = e
        .chain()
        .find_map(|c| c.downcast_ref::<ApiError>())
        .and_then(|api| match api {
            ApiError::RateLimited {
                kind: RateLimitKind::Paced,
                retry_after_seconds,
                ..
            } => Some(*retry_after_seconds),
            _ => None,
        })
    {
        return Some(hint);
    }
    // A pacer that answers 429 without draining the body does not read as a 429:
    // the socket dies while we are still writing the PUT and the response goes
    // with it. On Windows always, since the stack discards whatever was already
    // buffered when the RST lands, so the 429 does not exist for us. That is
    // issue #17: a 173-file folder that never finished, with `error writing a
    // body to connection` for its only clue.
    //
    // Treated as pacing with no hint. A genuine network drop lands here too and
    // gets that blob retried a few times, which beats throwing away the whole
    // batch over one stumble; and if it is persistent,
    // `MAX_PACED_RETRIES_PER_BLOB` turns it back into the same failure as before,
    // just a few seconds later.
    is_body_write_reset(e).then_some(0)
}

/// Did the connection die while we were writing the request body?
///
/// Matched on the `io::Error` at the bottom of the chain, never on the text: the
/// message comes in the language of the Windows install (the issue's arrived in
/// German) and comparing localised strings is an expensive way to detect
/// nothing.
fn is_body_write_reset(e: &anyhow::Error) -> bool {
    e.chain().any(|c| {
        c.downcast_ref::<std::io::Error>().is_some_and(|io| {
            matches!(
                io.kind(),
                std::io::ErrorKind::ConnectionAborted   // WSAECONNABORTED (10053)
                    | std::io::ErrorKind::ConnectionReset // WSAECONNRESET (10054)
                    | std::io::ErrorKind::BrokenPipe // EPIPE, el mismo caso en unix
            )
        })
    })
}

/// Run one blob's upload, retrying it, and only it, while a pacer says "too
/// fast".
///
/// The upload of a save is N independent PUTs, one per missing blob, and N is the
/// user's file count: 122 for a Cyberpunk folder with 46 save slots. The per-IP
/// pacer allows a burst and then a steady rate, so on a fast link the tail of a
/// large upload is *expected* to be turned away a few times. Letting that abort
/// the set (`try_collect` cancels every sibling on the first error) meant a large
/// save could never finish: each attempt got roughly a burst's worth of blobs
/// through, kept none of them (a fresh `upload_id` stages from zero) and
/// re-uploaded everything on the next pass, forever.
///
/// `attempt` is a closure rather than a future, because a retry needs a new body:
/// the file gets re-opened and re-hashed on the way out, so a save the game
/// rewrote mid-upload is still caught by the sha check rather than silently
/// retried with stale bytes.
async fn put_blob_paced<F, Fut>(
    relative_path: &str,
    paced_wait_ms: &AtomicU64,
    mut attempt: F,
) -> Result<()>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<()>>,
{
    let mut retries = 0u32;
    loop {
        let err = match attempt().await {
            Ok(()) => return Ok(()),
            Err(e) => e,
        };
        let Some(hint_secs) = paced_wait_hint(&err) else {
            return Err(err);
        };
        if retries >= MAX_PACED_RETRIES_PER_BLOB {
            return Err(err.context(format!(
                "{relative_path}: still being rate limited after {retries} retries: \
                 the server's request limit is too tight for this save's file count"
            )));
        }
        // Honour the server's number when it gave a real one; otherwise back
        // off from the floor. Either way it's capped, so a bogus hint can't
        // park the upload.
        let wait = Duration::from_secs(u64::from(hint_secs))
            .max(PACED_RETRY_FLOOR * 2u32.pow(retries.min(5)))
            .min(PACED_RETRY_CEILING);
        let spent = paced_wait_ms.fetch_add(wait.as_millis() as u64, Ordering::Relaxed);
        if Duration::from_millis(spent) > PACED_WAIT_BUDGET {
            return Err(err.context(format!(
                "{relative_path}: gave up after {}s of rate-limit waiting",
                PACED_WAIT_BUDGET.as_secs()
            )));
        }
        retries += 1;
        tracing::debug!(
            file = relative_path,
            retries,
            wait_ms = wait.as_millis() as u64,
            hint_secs,
            "upload: paced by the server, retrying this blob"
        );
        tokio::time::sleep(wait).await;
    }
}

/// The source directory exists but holds no regular files to upload (only empty
/// subdirs, or nothing). Typed so the agent can treat it as "nothing to back up"
/// (a `BackupSkippedEmpty`) rather than a red failure: pushing an empty snapshot
/// would clobber the last good server copy. See `agent::run_backup_with_retry`.
#[derive(Debug, thiserror::Error)]
#[error("no files found in {path}")]
pub struct EmptySource {
    pub path: PathBuf,
}

/// The tracked folder cannot be a game's: a whole profile, a system root, an
/// entire Wine or Proton prefix.
///
/// It exists because the structural guard only ran on adding (a manual add, an
/// adoption, a repoint). A row poisoned before the guard existed, or by an add
/// path that forgot to validate, never went through it again and carried on
/// uploading. Reported in aug-2026: a Steam Deck uploading
/// `steamapps/compatdata/423230/pfx`, the whole prefix, 308 MB, for a save of a
/// few KB.
///
/// It is checked on the backup path, before the disk is touched, so it covers
/// both the old rows and any future add that does not validate.
#[derive(Debug, thiserror::Error)]
#[error("refusing to back up {path}: {reason}. Pick the game's own save folder inside it.")]
pub struct UnsafeSource {
    pub path: PathBuf,
    pub reason: String,
}

/// Not a single file in the folder could be read.
///
/// The backup skips unreadable files one by one ([`split_unreadable`]), but when
/// none is left there is no snapshot to upload: publishing an empty version would
/// delete the last good copy in the cloud, just as in [`EmptySource`]. The
/// difference from that one is the reason, and the reason is the only actionable
/// part: "it is empty" sends you to check the path, "it will not be read" sends
/// you to check the file provider. The known trigger is OneDrive Files On-Demand
/// with the provider stopped, which leaves the files there, with their size, and
/// denies the bytes.
#[derive(Debug, thiserror::Error)]
#[error("none of the {count} files in {path} could be read: {first}")]
pub struct UnreadableSource {
    pub path: PathBuf,
    /// Cuántos ficheros enumeró el recorrido (todos ilegibles).
    pub count: usize,
    /// The first one's error, which is the one that explains the rest.
    pub first: String,
}

/// One file enumerated from the source directory.
#[derive(Debug, Clone)]
pub struct UploadFile {
    /// Forward-slash relative path used as the multipart filename header.
    pub relative_path: String,
    /// Absolute path on disk.
    pub absolute_path: PathBuf,
    /// File size in bytes (read once during the walk so progress totals are correct).
    pub size_bytes: u64,
    /// Last-modified time, captured during the walk. Used only to build the
    /// cheap skip-by-set-hash signature; `None` if the platform/FS didn't
    /// report one.
    pub modified: Option<SystemTime>,
}

/// What a per-save-cap trim left out of an upload. See
/// [`upload_directory_cloud`]'s trim-and-retry: when a save's logical size
/// exceeds the plan's per-save cap, the client uploads the newest files that fit
/// and reports the omitted tail here so the UI can tell the user their plan isn't
/// big enough (Free). The backup succeeded, but it is *partial*.
#[derive(Debug, Clone)]
pub struct TrimInfo {
    pub kept_files: usize,
    pub kept_bytes: u64,
    pub omitted_files: usize,
    pub omitted_bytes: u64,
    /// Plan slug the cap belongs to (e.g. `"free"`), for the upgrade nudge.
    pub plan: String,
    /// The per-save cap in bytes that forced the trim.
    pub limit_bytes: u64,
}

/// Trims `working` down to what fits under `limit`, keeping the newest files, and
/// describes what was left out.
///
/// `working` arrives sorted by descending mtime, so "what fits" is also "what is
/// most recent": an enormous save folder uploads partially rather than failing
/// whole, and what is lost is the oldest. A deliberately generic rule, recency
/// and size, with zero per-game knowledge.
///
/// `None` when not even the newest file fits: there no trim is possible and the
/// caller has to treat it as a terminal "too large".
///
/// Extracted so the pre-emptive trim (against the already-known cap) and the
/// reactive one (against the 413) are literally the same code: two criteria that
/// drifted apart would give two different versions of the same save depending on
/// which had done the trimming.
fn trim_to_cap(working: &mut Vec<&UploadFile>, limit: u64, plan: &str) -> Option<TrimInfo> {
    let mut kept: Vec<&UploadFile> = Vec::new();
    let mut kept_bytes = 0u64;
    for f in working.iter() {
        if kept_bytes + f.size_bytes <= limit {
            kept.push(*f);
            kept_bytes += f.size_bytes;
        }
    }
    if kept.is_empty() {
        return None;
    }
    let full_bytes: u64 = working.iter().map(|f| f.size_bytes).sum();
    let info = TrimInfo {
        kept_files: kept.len(),
        kept_bytes,
        omitted_files: working.len() - kept.len(),
        omitted_bytes: full_bytes - kept_bytes,
        plan: plan.to_string(),
        limit_bytes: limit,
    };
    *working = kept;
    Some(info)
}

/// A file the walk enumerated but whose bytes cannot be read.
///
/// Not a backup failure: it is content *this* copy cannot take. It travels up to
/// the caller because a version missing a file without the user knowing is worse
/// than an error in their face: upload what can be uploaded, and say out loud
/// what stayed behind.
#[derive(Debug, Clone)]
pub struct UnreadableFile {
    /// The path relative to the save, in the same shape as in [`UploadFile`].
    pub relative_path: String,
    /// The system error verbatim. It is the only thing that tells an unhydrated
    /// OneDrive placeholder from a denied permission or a dying disk, so it is
    /// carried whole up to the UI.
    pub error: String,
}

/// Result of a successful upload.
#[derive(Debug, Clone)]
pub struct UploadOutcome {
    pub snapshot: Snapshot,
    pub file_count: usize,
    pub total_bytes: u64,
    /// Files the walk saw and the upload could not take because their bytes would
    /// not be read. Empty in the normal case. Non-empty means this version is
    /// partial, and the caller has to say so: see
    /// `AgentEvent::BackupFilesUnreadable`.
    pub unreadable: Vec<UnreadableFile>,
    /// `Some` when the save was too big for the plan's per-save cap and only
    /// its newest files were uploaded; `None` when the whole save went up.
    pub trimmed: Option<TrimInfo>,
    /// Nothing was uploaded: this content was already on the server, and
    /// `snapshot` describes the version that already had it (ADR 0021 D.8.3). See
    /// [`ServerHead`].
    pub landed: bool,
}

/// The head the server publishes for a save: which version it is and what content
/// it has, as a digest of its manifest.
///
/// It is what makes ADR 0021 C.1's crash-robust anti-relaunch possible: a local
/// "upload in progress" flag does not survive a daemon restart, and with the
/// service, restarting is routine, so the question "does this need uploading?" is
/// asked of the server's truth, which is content-addressed. If the digest of what
/// we were about to upload is the head's, the previous upload *landed* and
/// uploading again would only create a duplicate version: same content, new
/// number, quota spent and a pointless pull on every other machine.
///
/// The digest arrives in the cloud manifest (`latest_sha256`), which the engine
/// already fetches on its own (D.12), so the check costs not one request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerHead {
    pub version_num: i64,
    /// The digest of that version's manifest, exactly as the server computes it.
    /// Empty means an old version (a whole archive, with no per-file manifest):
    /// it cannot be compared, and it is not.
    pub digest: String,
}

/// Outcome of a skip-aware backup ([`upload_directory_checked`]).
// One value per backup run, moved straight to the caller and never stored in
// bulk, so the size gap between variants costs nothing worth a Box.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum BackupResult {
    /// The cheap set signature matched the cached one, so nothing was read or
    /// uploaded. The fast path.
    Skipped,
    /// The cheap signature drifted (the game rewrote its save files, bumping
    /// mtimes) but the actual bytes are identical to the last upload, so no
    /// new snapshot was created. `signature` is the refreshed composite the
    /// caller should persist so the *next* check hits the fast path again
    /// instead of re-hashing the whole save every cycle.
    Unchanged { signature: String },
    /// A new snapshot was created. `signature` is the freshly-computed
    /// composite signature the caller should persist for the next skip check.
    Uploaded {
        outcome: UploadOutcome,
        signature: String,
    },
    /// It was already on the server: the local content is, byte for byte, that of
    /// the version the cloud publishes as its head (ADR 0021 D.8.3). Nothing was
    /// uploaded; the caller adopts `version_num` as the version it is synced to
    /// and persists `signature`.
    ///
    /// What produces it is a daemon restart with an upload in flight that did
    /// commit: the in-memory `in_flight` was lost, but the content is up there.
    AlreadyLanded { version_num: i64, signature: String },
}

/// A cheap signature over the sorted `(relative_path, size, mtime)` set.
///
/// Deliberately *not* a content hash: it never reads file bytes, so it adds no IO
/// on top of the directory walk. Two walks with identical paths, sizes and mtimes
/// produce the same signature, which is exactly the "watcher settled but nothing
/// was actually written" case we want to skip. It will not catch a rewrite that
/// preserves size *and* mtime while changing bytes (rare for game saves), trading
/// that corner for zero read overhead.
pub fn compute_set_signature(files: &[UploadFile]) -> String {
    let mut h = Sha256::new();
    for f in files {
        h.update(f.relative_path.as_bytes());
        h.update([0u8]);
        h.update(f.size_bytes.to_le_bytes());
        let mtime_nanos = f
            .modified
            .and_then(|m| m.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        h.update(mtime_nanos.to_le_bytes());
        h.update([0u8]);
    }
    hex::encode(h.finalize())
}

/// The digest of a version's manifest: the content identity the server publishes
/// (`save_versions.sha256` for a content-addressed version, and therefore the
/// cloud manifest's `latest_sha256`).
///
/// It has to match byte for byte what the server does on commit (`cas_commit`):
/// sha256 over the manifest's rows sorted by path, each one
/// `path \0 sha \0 size(le) \0`. If this drifted, D.8.3's check would never find
/// a match and we would go back to uploading too much, a silent and expensive
/// failure, so there is a test with a fixed vector.
///
/// `files` must arrive sorted by path (which is what [`walk_source`] returns).
/// Sorting in Rust is byte order and the server sorts in the database, so a
/// different collation can give them different digests for the same content: that
/// produces a false negative (we upload anyway), never a false positive (two
/// equal digests only come out of the same byte stream).
pub fn manifest_digest<'a>(files: impl Iterator<Item = (&'a str, &'a str, i64)>) -> String {
    let mut h = Sha256::new();
    for (path, sha, size) in files {
        h.update(path.as_bytes());
        h.update([0u8]);
        h.update(sha.as_bytes());
        h.update([0u8]);
        h.update(size.to_le_bytes());
        h.update([0u8]);
    }
    hex::encode(h.finalize())
}

/// What goes into the content digest in place of the bytes of a file that will
/// not be read. An arbitrary value that cannot be a prefix of real content, so
/// "unreadable" and "empty" never give the same digest.
const UNREADABLE_MARKER: &[u8] = b"\x01hoard:unreadable\x01";

/// A content signature over the sorted `(relative_path, bytes)` set.
///
/// Unlike [`compute_set_signature`] this *reads every file*, so it is only used as
/// a fallback when the cheap signature drifted: many games, and some background
/// launchers and cloud-sync daemons, rewrite save files on a timer, bumping the
/// mtime without changing a single byte. The cheap check would treat that as a
/// change and cut a redundant snapshot every few hours; this confirms whether the
/// bytes actually moved before we upload.
///
/// An unreadable file does not bring the pass down: it is skipped with a warning
/// and enters the digest through [`UNREADABLE_MARKER`] rather than through its
/// bytes. The earlier asymmetry was the bug: [`walk_source`] already skips what it
/// cannot interrogate, on purpose ("one unreadable transient file shouldn't lose
/// the backup of everything else"), and this pass propagated any read error with
/// `?`, so a single file lost the whole snapshot. A real case: a OneDrive Files
/// On-Demand placeholder ("the cloud file provider is not running") inside one
/// game's save; 3,934 attempts in 13 days and not one version uploaded.
///
/// The marker, rather than simply omitting the path, keeps the digest stable while
/// the file stays unreadable, which is what stops it retrying in a loop, and
/// changes it the moment it can be read again, which is exactly when it has to be
/// uploaded again.
async fn compute_content_signature(files: &[UploadFile]) -> String {
    use tokio::io::AsyncReadExt;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 128 * 1024];
    for f in files {
        h.update(f.relative_path.as_bytes());
        h.update([0u8]);
        // The bytes are poured into the hash as they arrive, as always: a 2 GB
        // save is not materialised in RAM to sign it. A failure halfway leaves in
        // `h` the part that was read plus the marker, and that stalls nothing:
        // what decides whether to re-read is the *cheap* signature, which only
        // looks at paths, sizes and mtimes and does not depend on this.
        let read = async {
            let mut file = tokio::fs::File::open(&f.absolute_path)
                .await
                .with_context(|| format!("opening {}", f.absolute_path.display()))?;
            loop {
                let n = file
                    .read(&mut buf)
                    .await
                    .with_context(|| format!("reading {}", f.absolute_path.display()))?;
                if n == 0 {
                    break;
                }
                h.update(&buf[..n]);
            }
            Ok::<(), anyhow::Error>(())
        }
        .await;
        if let Err(e) = read {
            tracing::warn!(
                path = %f.relative_path,
                error = %format!("{e:#}"),
                "hashing: skipping unreadable file"
            );
            h.update(UNREADABLE_MARKER);
        }
        h.update([0u8]);
    }
    hex::encode(h.finalize())
}

/// Sets aside the files whose bytes cannot be read right now, so the upload can
/// take everything else.
///
/// It is the other half of [`compute_content_signature`]'s tolerance: that one
/// stops an unreadable file bringing the signature down, and this one stops it
/// bringing the transfer down. Without it the file would stay in the list and blow
/// up further along, in the cloud path's tar or in the CAS hashing, which is where
/// half the fault lived.
///
/// It is checked by opening *and reading* the first block, not just opening: some
/// on-demand file providers let the handle open and fail on the first read. It
/// costs one `open` per file on top of the one the upload will do, and only on the
/// upload path, never in the engine's L1 sampling, the restore or the preview,
/// because opening a placeholder is what triggers its hydration: forcing that on
/// every tick would pull the whole folder down from the user's cloud to compute a
/// fingerprint.
///
/// It preserves the input order (`buffered`, not `buffer_unordered`): the list
/// arrives sorted by path from [`walk_source`] and the manifest's digest depends
/// on that order.
async fn split_unreadable(files: Vec<UploadFile>) -> (Vec<UploadFile>, Vec<UnreadableFile>) {
    let probes = files.into_iter().map(|f| {
        async move {
            match probe_readable(&f.absolute_path).await {
                Ok(()) => Ok(f),
                Err(e) => Err(UnreadableFile {
                    relative_path: f.relative_path.clone(),
                    error: format!("{e:#}"),
                }),
            }
        }
        .boxed()
    });
    let probed: Vec<_> = stream::iter(probes)
        .buffered(TRANSFER_CONCURRENCY)
        .collect()
        .await;
    let mut readable = Vec::with_capacity(probed.len());
    let mut unreadable = Vec::new();
    for outcome in probed {
        match outcome {
            Ok(f) => readable.push(f),
            Err(u) => {
                tracing::warn!(
                    path = %u.relative_path,
                    error = %u.error,
                    "upload: leaving out a file whose bytes can't be read"
                );
                unreadable.push(u);
            }
        }
    }
    (readable, unreadable)
}

/// Can this file's bytes be read? It opens and reads one byte.
async fn probe_readable(path: &Path) -> Result<()> {
    let mut file = tokio::fs::File::open(path)
        .await
        .with_context(|| format!("opening {}", path.display()))?;
    let mut byte = [0u8; 1];
    file.read(&mut byte)
        .await
        .with_context(|| format!("reading {}", path.display()))?;
    Ok(())
}

/// Persisted skip signature is a composite `"<cheap>:<content>"`. We split it
/// back into its two halves; a legacy value with no `:` (pre-fallback state
/// files held only the cheap hash) is treated as cheap-only with no known
/// content hash, so the first drift after upgrading reads bytes once and then
/// stores the composite.
fn split_signature(sig: Option<&str>) -> (Option<&str>, Option<&str>) {
    match sig {
        None => (None, None),
        Some(s) => match s.split_once(':') {
            Some((cheap, content)) => (Some(cheap), Some(content)),
            None => (Some(s), None),
        },
    }
}

fn join_signature(cheap: &str, content: &str) -> String {
    format!("{cheap}:{content}")
}

/// Walks `root` and returns the files that are save data.
///
/// `shields` are the file patterns the manifest declares for this game
/// ([`crate::savefilter::shields_for_slug`]); passing `&[]` leaves the kernel
/// deciding by name alone. Whatever
/// [`fileclass::classify`](hoard_core::kernel::fileclass::classify) marks as
/// [`Junk`](hoard_core::kernel::fileclass::FileClass::Junk) (OS litter,
/// temporaries, crash dumps, engine telemetry, locks the game holds open) does not
/// go into the snapshot. Config does: it is on the restore that whether to write
/// it gets decided (see `RestoreOptions::gate`).
///
/// Everybody has to come through here with the same `shields`.
/// [`compute_set_signature`]'s cheap signature is computed over this list, and the
/// engine's L1 sampling (`observe_local_fingerprint`) compares it against the one
/// the backup stored: two different filters give two different signatures for the
/// same quiet folder, the reducer sees a pending change that never resolves, and
/// there is a hot loop.
///
/// Symlinks are skipped on purpose: we don't want to follow links out of the save
/// directory, and tar archives with symlinks make restore ambiguous.
pub fn walk_source(root: &Path, shields: &[String]) -> Result<Vec<UploadFile>> {
    // A single-file save: the `local_path` IS the file. One `UploadFile` comes out
    // with its base name as the relative path, so the snapshot has exactly the
    // same shape as one from a folder with one file in it, and everything
    // downstream (signature, dedup, restore) carries on unaware. Over 8,000
    // manifest entries look like this: `<winAppData>/Game/save.dat`,
    // `<base>/140.sav`.
    if root.is_file() {
        let meta =
            std::fs::metadata(root).with_context(|| format!("reading {}", root.display()))?;
        let name = root
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow!("save file has no usable name: {}", root.display()))?;
        // A single-file save IS the file: the path was chosen by pointing at it,
        // so nothing is classified here. Filtering it would leave the save empty
        // and the whole backup in `EmptySource`. The user pointed at that file,
        // and that outweighs any rule by name.
        return Ok(vec![UploadFile {
            relative_path: name.to_string(),
            absolute_path: root.to_path_buf(),
            size_bytes: meta.len(),
            modified: meta.modified().ok(),
        }]);
    }

    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        // An unreadable subdirectory is skipped; only the root is a hard error.
        //
        // This used to be a `?` at any level, so ONE folder without permission
        // aborted the game's whole backup. On Windows that is the norm rather than
        // the exception: the profile's legacy junctions
        // (`AppData\Local\Application Data`, which points at its own parent)
        // return access denied and are a cycle into the bargain. Losing an
        // unreadable subfolder is infinitely better than losing the backup.
        let read = match std::fs::read_dir(&dir) {
            Ok(r) => r,
            Err(e) if dir != root => {
                tracing::warn!(path = %dir.display(), error = %e, "skipping unreadable directory");
                continue;
            }
            Err(e) => {
                return Err(e).with_context(|| format!("reading dir {}", dir.display()));
            }
        };
        for entry in read {
            // As above: an entry that vanishes or cannot be interrogated
            // mid-walk does not invalidate the rest.
            let Ok(entry) = entry else { continue };
            let path = entry.path();
            let Ok(ft) = entry.file_type() else {
                tracing::warn!(path = %path.display(), "skipping entry with unreadable type");
                continue;
            };
            if ft.is_dir() {
                stack.push(path);
            } else if ft.is_file() {
                let rel = path
                    .strip_prefix(root)
                    .map_err(|e| anyhow!("strip_prefix: {e}"))?
                    .to_string_lossy()
                    .replace('\\', "/");
                // What is not save data does not go into the snapshot.
                if !fileclass::classify(&rel, shields).is_backed_up() {
                    tracing::debug!(path = %rel, "skipping non-save file");
                    continue;
                }
                // A file we can't stat (locked, vanished mid-walk, permission)
                // is skipped with a warning rather than failing the whole
                // upload: one unreadable transient file shouldn't lose the
                // backup of everything else.
                let meta = match entry.metadata() {
                    Ok(m) => m,
                    Err(e) => {
                        tracing::warn!(path = %path.display(), error = %e, "skipping unreadable file");
                        continue;
                    }
                };
                out.push(UploadFile {
                    relative_path: rel,
                    absolute_path: path,
                    size_bytes: meta.len(),
                    modified: meta.modified().ok(),
                });
            }
            // symlinks: ignored on purpose.
        }
    }
    out.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    Ok(out)
}

/// Upload a directory as a new snapshot for `save_id`.
///
/// `progress(uploaded, total)` is called once per file as it's added to the
/// multipart form. Both values are byte counts. The callback is `Fn` so the caller
/// can wire any UI on top.
///
/// `game_slug` and `label` are only consulted on the Hoard Cloud path, where the
/// server keys the save row on `(user_id, game_slug, label)` and the snapshot list
/// endpoints don't exist. They're ignored self-hosted.
///
/// A file whose bytes will not be read is left out and reported, rather than
/// losing the whole snapshot. Of the two possible outcomes, skipping the file or
/// parking the save, skipping wins, because parking is exactly the state we came
/// from: the case that exposed it (a OneDrive Files On-Demand placeholder with the
/// provider stopped, inside one game's save) had gone 13 days and 3,934 attempts
/// uploading nothing, and the cause can last weeks. A whole save minus one file is
/// worth more than no save.
///
/// The price of that choice is paid in full in [`UploadOutcome::unreadable`]: the
/// version is partial and whoever publishes it has to say so (the engine emits
/// `AgentEvent::BackupFilesUnreadable` and the UI leaves a sticky warning on the
/// game's card). A silently incomplete version is not an option: it would only be
/// discovered on a restore.
///
/// If not a single readable file is left, nothing is uploaded
/// ([`UnreadableSource`]): publishing an empty version would delete the last good
/// copy in the cloud.
#[allow(clippy::too_many_arguments)]
pub async fn upload_directory<F>(
    client: &ApiClient,
    save_id: &str,
    game_slug: &str,
    label: &str,
    source: &Path,
    base_version: Option<i64>,
    head: Option<&ServerHead>,
    origin: VersionOrigin,
    progress: F,
) -> Result<UploadOutcome>
where
    // `Sync` because the cloud path shares the callback by reference across
    // its in-flight uploads.
    F: Fn(u64, u64) + Send + Sync,
{
    let source = source
        .canonicalize()
        .with_context(|| format!("source path does not exist: {}", source.display()))?;
    // A folder or a single file; anything else (a socket, a device) is not a
    // save.
    if !source.is_dir() && !source.is_file() {
        bail!("source must be a folder or a file: {}", source.display());
    }

    let files = walk_source(&source, &crate::savefilter::shields_for_slug(game_slug))?;
    if files.is_empty() {
        return Err(EmptySource { path: source }.into());
    }
    // A file that will not be read leaves the list here, at the ONE point all four
    // upload paths (cloud, CAS, pack, multipart) pass through, so none of them
    // meets it mid-transfer. Whatever is left out travels in the `UploadOutcome`
    // so the caller can report it: a silently incomplete version is the outcome
    // that does not count.
    let (files, unreadable) = split_unreadable(files).await;
    if files.is_empty() {
        // Nothing readable is left: uploading here would publish an empty version
        // and delete the last good copy in the cloud.
        let first = unreadable
            .first()
            .map(|u| u.error.clone())
            .unwrap_or_default();
        return Err(UnreadableSource {
            path: source,
            count: unreadable.len(),
            first,
        }
        .into());
    }
    let total_bytes: u64 = files.iter().map(|f| f.size_bytes).sum();
    let file_count = files.len();

    // Choosing a protocol requires KNOWING which one the server speaks, not
    // assuming it. `is_cloud()` collapses "self-hosted" and "the `/v1/health`
    // probe failed" into the same `false`, which is convenient for a UI ornament
    // and poison here: with the probe down it takes the self-hosted branch and
    // uploads against `/v1/saves/:id/snapshots`, which does not exist on cloud.
    // The user sees "uploading snapshot: not found (404)" and goes looking for a
    // deleted save that is perfectly fine. Reported aug-2026, and the trigger is
    // as ordinary as it gets: the Fly machine sleeps on inactivity and the probe
    // catches the cold start.
    //
    // With no resolved probe nothing is chosen: it fails as what it is, the server
    // not being reachable, and the usual backoff retries it.
    //
    // Both calls are needed and not redundant: `server_mode()` is what probes and
    // caches (its own `None` is just as ambiguous, because it swallows the error),
    // and `probed_is_cloud()` is what then gives the honest answer, returning
    // `Some` only when a probe succeeded.
    let _ = client.server_mode().await;
    let Some(is_cloud) = client.probed_is_cloud() else {
        bail!(
            "can't tell which protocol this server speaks yet (the /v1/health probe hasn't \
             succeeded). Not guessing: uploading with the wrong one fails as a misleading 404."
        );
    };
    // Hoard Cloud (api.hoard.services) speaks a different protocol: the
    // self-hosted `/v1/saves/:id/snapshots` multipart endpoint doesn't exist
    // there. Pack the save into a single tar.zst, declare the upload, PUT the
    // bytes straight to R2 via a presigned URL, then commit.
    if is_cloud {
        let mut outcome = upload_directory_cloud(
            client,
            save_id,
            game_slug,
            label,
            &files,
            total_bytes,
            base_version,
            head,
            origin,
            progress,
        )
        .await?;
        outcome.unreadable = unreadable;
        return Ok(outcome);
    }

    // A self-hosted server that can negotiate content: the manifest is declared to
    // it and only the blobs it lacks travel. The multipart below stays for servers
    // older than 1.1.3, which do not advertise the capability.
    //
    // The condition is `Some(true)`, not `unwrap_or(false)`: a `None` means the
    // probe has not resolved, and that case was already cut off by the `bail!`
    // above.
    if client.probed_supports_cas() == Some(true) {
        let mut outcome = upload_directory_cas(
            client,
            save_id,
            &files,
            total_bytes,
            base_version,
            origin,
            progress,
        )
        .await?;
        outcome.unreadable = unreadable;
        return Ok(outcome);
    }

    // Adaptive ingest by save shape (ADR 0019): many small files travel better as
    // a single tar (one round trip, one handle) than as N multipart parts. The
    // threshold is on file count; the server unpacks the `pack` field and dedups
    // per file exactly as in the normal mode, so the storage model does not
    // change.
    const PACK_THRESHOLD: usize = 500;

    let mut form = multipart::Form::new();
    // Declare the base version so the server can reject a non-fast-forward
    // (another device advanced this save since we last synced).
    if let Some(b) = base_version {
        form = form.text("base_version", b.to_string());
    }
    // Who uploads. The column has existed since day one and the server stores and
    // returns it; what was missing was somebody filling it in, so the history could
    // not tell two machines syncing the same save apart.
    if let Some(device) = crate::logship::device_name() {
        form = form.text("device_name", device);
    }
    // The version's origin: the server has always accepted this field and nobody
    // filled it in. Without it retention cannot tell the copy the user made before
    // the boss from the forty the timer made.
    if let Some(note) = origin.as_note() {
        form = form.text("notes", note);
    }
    progress(0, total_bytes);

    if file_count > PACK_THRESHOLD {
        // Build the tar on the fly through an in-memory pipe and stream it as the
        // request body, never materialising the whole archive in RAM.
        let (writer, reader) = tokio::io::duplex(256 * 1024);
        let files_for_tar = files.clone();
        tokio::spawn(async move {
            let mut tar = tokio_tar::Builder::new(writer);
            for f in &files_for_tar {
                if let Err(e) = tar
                    .append_path_with_name(&f.absolute_path, &f.relative_path)
                    .await
                {
                    // Dropping the writer truncates the tar; the server then
                    // rejects it as a malformed pack, surfacing as an upload
                    // error rather than a silent partial snapshot.
                    tracing::warn!(error = %e, path = %f.relative_path, "pack tar build error");
                    return;
                }
            }
            if let Ok(mut inner) = tar.into_inner().await {
                let _ = tokio::io::AsyncWriteExt::shutdown(&mut inner).await;
            }
        });
        let stream = tokio_util::io::ReaderStream::new(reader);
        let body = reqwest::Body::wrap_stream(stream);
        let part = multipart::Part::stream(body)
            .file_name("pack.tar")
            .mime_str("application/x-tar")?;
        form = form.part("pack", part);
        progress(total_bytes, total_bytes);
    } else {
        let mut uploaded = 0u64;
        for f in &files {
            // Stream each file from disk instead of reading it whole into RAM:
            // open the handle, wrap it as a byte stream and hand it to reqwest
            // as a streaming multipart part. A 2 GB save no longer means 2 GB
            // of process memory.
            let file = tokio::fs::File::open(&f.absolute_path)
                .await
                .with_context(|| format!("reading {}", f.absolute_path.display()))?;
            let stream = tokio_util::io::ReaderStream::new(file);
            let body = reqwest::Body::wrap_stream(stream);
            let part = multipart::Part::stream_with_length(body, f.size_bytes)
                .file_name(f.relative_path.clone())
                .mime_str("application/octet-stream")?;
            // Server keys files by the field NAME = "files" and reads the
            // relative path from the multipart filename header.
            form = form.part("files", part);
            uploaded += f.size_bytes;
            progress(uploaded, total_bytes);
        }
    }

    let snap = client
        .snapshot_upload(save_id, form)
        .await
        .context("uploading snapshot")?;

    Ok(UploadOutcome {
        snapshot: snap,
        file_count,
        total_bytes,
        unreadable,
        // The self-hosted multipart path has no per-save cap trim.
        trimmed: None,
        landed: false,
    })
}

/// Whole-file SHA-256 of every file in the manifest, a few in flight at once so
/// per-file open/read latency overlaps instead of adding up.
///
/// (The futures are built eagerly into a Vec of `BoxFuture`s rather than through
/// `iter().map(closure)`: a closure over borrowed items retained inside the
/// stream trips rustc's "Send/FnOnce is not general enough" false positive when
/// the whole upload future crosses a `tokio::spawn`. One small allocation per
/// file, all of them IO-bound.)
async fn hash_manifest(files: &[UploadFile]) -> Result<HashMap<&str, String>> {
    let mut hash_futs = Vec::with_capacity(files.len());
    for f in files {
        hash_futs.push(
            async move {
                let sha = hash_file(&f.absolute_path).await?;
                Ok::<_, anyhow::Error>((f.relative_path.as_str(), sha))
            }
            .boxed(),
        );
    }
    stream::iter(hash_futs)
        .buffer_unordered(TRANSFER_CONCURRENCY)
        .try_collect()
        .await
}

/// Self-hosted content-addressed upload: hash, declare the manifest, upload only
/// the blobs the server is missing, commit.
///
/// It is the same deal as [`upload_directory_cloud`] with one difference that
/// governs everything: the bytes go to the server, not to a bucket. Self-hosted
/// signs no URLs (ADR 0020) because behind it there may be a disk, MinIO or an
/// `rclone serve s3` over OneDrive; the server is always in the middle. In
/// exchange, self-hosted has no plan and no per-save cap, so there is no
/// trim-and-retry here.
///
/// What this takes away from a self-hoster is the repeated upload: until 1.1.2 a
/// copy sent the whole folder even when the server already had the content, since
/// it deduplicated on store rather than in transit, so a 3 GB save with 10 MB of
/// changes cost 3 GB of upload and ran into `max_snapshot_size_mb` and any
/// proxy's body limit along the way.
async fn upload_directory_cas<F>(
    client: &ApiClient,
    save_id: &str,
    files: &[UploadFile],
    total_bytes: u64,
    base_version: Option<i64>,
    origin: VersionOrigin,
    progress: F,
) -> Result<UploadOutcome>
where
    F: Fn(u64, u64) + Send + Sync,
{
    use hoard_core::ids::Sha256 as Sha256Hex;

    progress(0, total_bytes);
    let sha_by_path = hash_manifest(files).await?;

    let mut manifest: Vec<CasFile> = Vec::with_capacity(files.len());
    for f in files {
        let sha = &sha_by_path[f.relative_path.as_str()];
        manifest.push(CasFile {
            relative_path: f.relative_path.clone(),
            sha256: Sha256Hex::parse(sha)
                .with_context(|| format!("hashing {}", f.relative_path))?,
            size_bytes: f.size_bytes as i64,
            modified_at: f
                .modified
                .and_then(|m| m.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64),
        });
    }

    let init = client
        .cas_init(
            save_id,
            &CasInit {
                base_version,
                files: manifest.clone(),
            },
        )
        .await
        .context("cas init")?;

    // Several files with the same content share a blob: it is uploaded once.
    let mut by_sha: HashMap<&str, &UploadFile> = HashMap::new();
    for f in files {
        by_sha
            .entry(sha_by_path[f.relative_path.as_str()].as_str())
            .or_insert(f);
    }

    // Resolve each missing blob to its source file before a byte moves, so a
    // manifest that does not add up aborts at the start rather than mid-upload.
    let mut pending: Vec<(&UploadFile, String)> = Vec::with_capacity(init.missing.len());
    for blob in &init.missing {
        let Some(f) = by_sha.get(blob.sha256.as_str()) else {
            bail!(
                "server requested a blob not in the manifest: {}",
                blob.sha256.as_str()
            );
        };
        pending.push((*f, blob.sha256.as_str().to_string()));
    }

    let upload_total: u64 = init
        .missing
        .iter()
        .map(|b| b.size_bytes.max(0) as u64)
        .sum();
    tracing::info!(
        save_id,
        files = files.len(),
        upload_blobs = pending.len(),
        upload_bytes = upload_total,
        logical_bytes = total_bytes,
        "self-hosted upload negotiated: only the missing blobs travel"
    );

    // The bar measures what really travels, not the save's size: that is the
    // figure the user is waiting on.
    let denom = upload_total.max(1);
    let uploaded = AtomicU64::new(0);
    // Shared across every worker: the pacer waits are the same queue, so the
    // give-up budget has to be counted once for the whole upload.
    let paced_wait_ms = AtomicU64::new(0);
    progress(0, denom);
    let mut put_futs = Vec::with_capacity(pending.len());
    for (f, sha) in pending {
        let uploaded = &uploaded;
        let paced_wait_ms = &paced_wait_ms;
        let progress = &progress;
        let upload_id = init.upload_id.as_str();
        put_futs.push(
            async move {
                put_blob_paced(&f.relative_path, paced_wait_ms, || async {
                    let file = tokio::fs::File::open(&f.absolute_path)
                        .await
                        .with_context(|| format!("opening {}", f.absolute_path.display()))?;
                    let (stream, sent) = hashing_stream(file);
                    client
                        .cas_upload_blob(
                            upload_id,
                            &sha,
                            reqwest::Body::wrap_stream(stream),
                            f.size_bytes,
                        )
                        .await
                        .with_context(|| format!("uploading {}", f.relative_path))?;
                    // The server rejects a blob whose content does not match its
                    // sha, so cross-contaminated content can no longer slip
                    // through here. It is checked anyway to give the good message
                    // ("the game rotated the save mid-upload") rather than the
                    // server's raw 400.
                    {
                        let sent = sent.lock().map_err(|_| anyhow!("upload hasher poisoned"))?;
                        verify_sent(&f.relative_path, &sha, f.size_bytes, &sent)?;
                    }
                    Ok(())
                })
                .await?;
                let done = uploaded.fetch_add(f.size_bytes, Ordering::Relaxed) + f.size_bytes;
                progress(done, denom);
                Ok::<_, anyhow::Error>(())
            }
            .boxed(),
        );
    }
    stream::iter(put_futs)
        .buffer_unordered(TRANSFER_CONCURRENCY)
        .try_collect::<Vec<()>>()
        .await?;
    progress(denom, denom);

    let snapshot = client
        .cas_commit(
            save_id,
            &CasCommit {
                upload_id: init.upload_id,
                base_version,
                device_name: crate::logship::device_name(),
                notes: origin.as_note().map(str::to_string),
                files: manifest,
            },
        )
        .await
        .context("cas commit")?;

    Ok(UploadOutcome {
        snapshot,
        file_count: files.len(),
        total_bytes,
        // Filled in by `upload_directory`, which is what sets the unreadable ones
        // aside.
        unreadable: Vec::new(),
        // With no plan there is no per-save cap to trim against.
        trimmed: None,
        landed: false,
    })
}

/// Hoard Cloud upload (content-addressed): hash each file, declare the manifest,
/// upload only the blobs the server is missing, commit.
///
/// Unlike the old archive path this never packs the whole save: each file is its
/// own R2 object keyed by its whole-file SHA-256, so a 600 MB save the game
/// rewrote in place with 10 MB of real change costs a 10 MB upload. Files are
/// never decompressed; the game's `.v3` and zip blobs are deduped whole.
#[allow(clippy::too_many_arguments)]
async fn upload_directory_cloud<F>(
    client: &ApiClient,
    save_id: &str,
    game_slug: &str,
    label: &str,
    files: &[UploadFile],
    total_bytes: u64,
    base_version: Option<i64>,
    head: Option<&ServerHead>,
    origin: VersionOrigin,
    progress: F,
) -> Result<UploadOutcome>
where
    F: Fn(u64, u64) + Send + Sync,
{
    progress(0, total_bytes);

    // 1. Whole-file SHA-256 of every file, the dedup key. Hashed once up front and
    //    cached by path so a per-save-cap trim-and-retry (below) does not re-read
    //    the files.
    let sha_by_path = hash_manifest(files).await?;

    // 1b. Is it already up there? (ADR 0021 D.8.3.) With the hashes already
    // computed, asking the server's truth whether this exact content is its head
    // costs neither a request nor another read, and if it is, the upload a daemon
    // restart left half done did commit, so uploading again would only create a
    // duplicate version (quota, R2 ops and a pointless pull on every other
    // machine). Anti-relaunch against the server, not against a local flag that
    // does not survive a restart.
    if let Some(head) = head.filter(|h| !h.digest.is_empty()) {
        let digest = manifest_digest(files.iter().map(|f| {
            (
                f.relative_path.as_str(),
                sha_by_path[f.relative_path.as_str()].as_str(),
                f.size_bytes as i64,
            )
        }));
        if digest == head.digest {
            tracing::info!(
                save_id,
                version_num = head.version_num,
                "cloud upload skipped: this exact content is already the server's head"
            );
            return Ok(UploadOutcome {
                snapshot: landed_snapshot(save_id, head, files.len(), total_bytes),
                file_count: files.len(),
                total_bytes,
                unreadable: Vec::new(),
                trimmed: None,
                landed: true,
            });
        }
    }

    // Working set, newest first, so if the save is too big for the plan's per-save
    // cap we keep the most recent saves and drop the oldest: a generic rule
    // (recency and size only, no per-game knowledge) that lets a huge Paradox
    // `save games` folder back up *partially* instead of failing whole. `trimmed`
    // records what was left out for the UI's "your plan isn't big enough" nudge.
    let mut working: Vec<&UploadFile> = files.iter().collect();
    working.sort_by_key(|f| std::cmp::Reverse(f.modified));
    let mut trimmed: Option<TrimInfo> = None;

    // The pre-emptive trim: if we already know this plan's cap, the server does not
    // need to remind us again.
    //
    // The cap is only learned by being refused, since no endpoint states it, so the
    // session's first big copy still costs a 413. The rest do not: they are trimmed
    // here and go up first time. It is the difference between asking once and
    // asking on every autosave, which is what turned five users into 12,996
    // refusals a week.
    if let Some(cap) = client.plan_cap() {
        if total_bytes > cap.limit_bytes {
            if let Some(info) = trim_to_cap(&mut working, cap.limit_bytes, &cap.plan) {
                tracing::debug!(
                    save_id,
                    game_slug,
                    limit_bytes = cap.limit_bytes,
                    kept_files = info.kept_files,
                    omitted_files = info.omitted_files,
                    "cloud: trimmed to the known per-save cap without asking"
                );
                trimmed = Some(info);
            }
        }
    }

    // 2/3/4. Declare manifest → upload missing blobs → commit. Wrapped in a
    // loop so a per-save-cap 413 can trim the working set and retry exactly
    // once (the trim can only shrink, so it converges).
    let (init, by_sha, file_count, total_bytes) = loop {
        let file_count = working.len();
        let logical: u64 = working.iter().map(|f| f.size_bytes).sum();

        let manifest: Vec<CloudCasFileEntry> = working
            .iter()
            .map(|f| CloudCasFileEntry {
                relative_path: f.relative_path.clone(),
                sha256: sha_by_path[f.relative_path.as_str()].clone(),
                size_bytes: f.size_bytes as i64,
                modified_at: f
                    .modified
                    .and_then(|m| m.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64),
            })
            .collect();

        match client
            .cloud_cas_init(&CloudCasInit {
                save_id: save_id.to_string(),
                game_slug: game_slug.to_string(),
                label: Some(label.to_string()),
                device_name: crate::logship::device_name(),
                notes: origin.as_note().map(str::to_string),
                backup_only: false,
                base_version,
                files: manifest,
            })
            .await
        {
            Ok(init) => {
                // Files sharing a SHA upload once.
                let mut by_sha: HashMap<&str, &UploadFile> = HashMap::new();
                for f in &working {
                    by_sha
                        .entry(sha_by_path[f.relative_path.as_str()].as_str())
                        .or_insert(*f);
                }
                break (init, by_sha, file_count, logical);
            }
            Err(e) => {
                // Per-save size cap (413). Trim to the newest files that fit
                // under the cap and retry once. Only trim on the first hit
                // (`trimmed.is_none()`) so we can't loop.
                let cap = if trimmed.is_none() {
                    e.downcast_ref::<crate::api::ApiError>()
                        .and_then(|api_err| match api_err {
                            crate::api::ApiError::TooLarge(d) if d.limit_bytes > 0 => {
                                Some(d.clone())
                            }
                            _ => None,
                        })
                } else {
                    None
                };
                let Some(detail) = cap else {
                    return Err(e).context("cloud cas init");
                };
                // Record it before anything else: even if this trim fails, the
                // next copy will not have to ask.
                client.remember_plan_cap(detail.limit_bytes, &detail.plan);
                let Some(info) = trim_to_cap(&mut working, detail.limit_bytes, &detail.plan) else {
                    // Even the single newest file is over the cap, so there is
                    // nothing to trim to; let the caller surface it as terminal
                    // too-large.
                    return Err(e).context("cloud cas init");
                };
                tracing::warn!(
                    save_id,
                    game_slug,
                    plan = %detail.plan,
                    limit_bytes = detail.limit_bytes,
                    kept_files = info.kept_files,
                    omitted_files = info.omitted_files,
                    "cloud: save exceeds plan per-save cap, uploading only the newest files that fit"
                );
                trimmed = Some(info);
                continue;
            }
        }
    };
    let upload_total: u64 = init
        .missing
        .iter()
        .map(|b| b.size_bytes.max(0) as u64)
        .sum();
    // Progress is reported against the bytes actually transferred, so the bar
    // reflects the real (deduped) upload rather than the whole save size.
    let denom = upload_total.max(1);
    // Resolve every missing blob to its source file before moving any bytes,
    // so a manifest mismatch aborts up front rather than mid-upload.
    let mut pending: Vec<(&CloudCasMissingBlob, &UploadFile)> =
        Vec::with_capacity(init.missing.len());
    for blob in &init.missing {
        let Some(f) = by_sha.get(blob.sha256.as_str()) else {
            bail!(
                "server requested a blob not in the manifest: {}",
                blob.sha256
            );
        };
        pending.push((blob, *f));
    }
    // A few PUTs in flight at once: presigned-URL round-trip latency, not
    // bandwidth, dominates the many-small-blobs shape. Completion order is
    // irrelevant, since each blob is its own R2 object, so progress just counts
    // bytes as they land. (An eager Vec of boxed futures for the same
    // trait-inference reason as the hashing pass above.)
    let uploaded = AtomicU64::new(0);
    // Shared across every worker: the pacer waits are the same queue, so the
    // give-up budget has to be counted once for the whole upload.
    let paced_wait_ms = AtomicU64::new(0);
    progress(0, denom);
    let mut put_futs = Vec::with_capacity(pending.len());
    for (blob, f) in pending {
        let uploaded = &uploaded;
        let paced_wait_ms = &paced_wait_ms;
        let progress = &progress;
        put_futs.push(
            async move {
                put_blob_paced(&f.relative_path, paced_wait_ms, || async {
                    let file = tokio::fs::File::open(&f.absolute_path)
                        .await
                        .with_context(|| format!("opening {}", f.absolute_path.display()))?;
                    let (stream, sent) = hashing_stream(file);
                    client
                        .put_presigned(
                            &blob.upload,
                            reqwest::Body::wrap_stream(stream),
                            f.size_bytes,
                        )
                        .await
                        .with_context(|| format!("uploading {}", f.relative_path))?;
                    // The object is already in the bucket, but without a commit it
                    // exists for nobody: the `cloud_blobs` row is created when the
                    // version is confirmed, and the server's dedup looks at that
                    // table, not at the bucket. So aborting here leaves the object
                    // orphaned (the GC sweeps it) and the next attempt asks for it
                    // again and overwrites it with the good content. What must not
                    // happen, and what did, is a version being confirmed that
                    // points at bytes that are not its own.
                    {
                        let sent = sent.lock().map_err(|_| anyhow!("upload hasher poisoned"))?;
                        verify_sent(&f.relative_path, &blob.sha256, f.size_bytes, &sent)?;
                    }
                    Ok(())
                })
                .await?;
                let done = uploaded.fetch_add(f.size_bytes, Ordering::Relaxed) + f.size_bytes;
                progress(done, denom);
                Ok::<_, anyhow::Error>(())
            }
            .boxed(),
        );
    }
    stream::iter(put_futs)
        .buffer_unordered(TRANSFER_CONCURRENCY)
        .try_collect::<Vec<()>>()
        .await?;

    // 4. Commit: the server verifies the new blobs landed and finalizes. The commit
    // must target the *canonical* cloud save id: when another device already
    // created this (game, label) under a different id, the server resolved ours to
    // that one at init, and committing against our local id would 404 forever.
    let canonical_id = init.save_id.as_deref().unwrap_or(save_id);
    if canonical_id != save_id {
        tracing::info!(
            local_save_id = save_id,
            canonical_save_id = canonical_id,
            game_slug,
            label,
            "cloud save id diverged, committing against the canonical cloud id"
        );
    }
    let commit = client
        .cloud_cas_commit(canonical_id, init.version_num)
        .await
        .context("cloud cas commit")?;

    // Synthesize a Snapshot for the shared `UploadOutcome` shape.
    // `total_size_bytes` is the logical save size (sum of file sizes), matching
    // self-hosted snapshot semantics.
    let snapshot = Snapshot {
        id: String::new(),
        // The canonical id comes back from the cloud commit; if it arrived in a
        // shape the gate does not recognise, the synthetic `Snapshot` goes
        // without it rather than bringing down a backup that ALREADY uploaded
        // the bytes.
        save_id: SaveId::parse(&commit.save_id).ok(),
        version_num: commit.version_num,
        parent_version: base_version,
        device_name: crate::logship::device_name(),
        notes: origin.as_note().map(str::to_string),
        file_count: file_count as i64,
        total_size_bytes: total_bytes as i64,
        is_pinned: false,
        created_at: OffsetDateTime::now_utc(),
        deleted_at: None,
        // Derived server-side from the manifest, and the cloud commit response
        // doesn't carry it back. Nothing is lost: this synthetic snapshot only
        // reports what just landed, and the History view reads the real row.
        insight: None,
    };
    Ok(UploadOutcome {
        snapshot,
        file_count,
        total_bytes,
        unreadable: Vec::new(),
        trimmed,
        landed: false,
    })
}

/// The `Snapshot` describing an upload that had already landed: the version is the
/// server's rather than an invented one, and the count is the local content's,
/// which by definition is the same. The server is not asked: the whole point of
/// D.8.3 is saving the round trip, and all the caller needs is which version we
/// are now synced to.
fn landed_snapshot(
    save_id: &str,
    head: &ServerHead,
    file_count: usize,
    total_bytes: u64,
) -> Snapshot {
    Snapshot {
        id: String::new(),
        save_id: SaveId::parse(save_id).ok(),
        version_num: head.version_num,
        parent_version: None,
        device_name: crate::logship::device_name(),
        notes: None,
        file_count: file_count as i64,
        total_size_bytes: total_bytes as i64,
        is_pinned: false,
        created_at: OffsetDateTime::now_utc(),
        deleted_at: None,
        insight: None,
    }
}

/// SHA-256 of a file's bytes, read in fixed-size chunks.
///
/// Shared with the restore side: the same whole-file digest that keys the upload's
/// dedup against the server's blobs keys the download's dedup against the local
/// disk (ADR 0021 D.13). There is no per-file hash *cache* to reuse, since
/// `state.json`'s `set_hash` is a signature over the whole set (paths, sizes and
/// mtimes, plus a content hash of the concatenation) rather than per-file digests,
/// so both sides hash on demand.
pub(crate) async fn hash_file(path: &Path) -> Result<String> {
    let mut file = tokio::fs::File::open(path)
        .await
        .with_context(|| format!("hashing {}", path.display()))?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 256 * 1024];
    loop {
        let n = file
            .read(&mut buf)
            .await
            .with_context(|| format!("reading {}", path.display()))?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(hex::encode(h.finalize()))
}

/// Wrap a file as a streaming reqwest body for the presigned PUT.
/// What really went out over the socket on a PUT.
#[derive(Default)]
pub(crate) struct Sent {
    digest: Sha256,
    len: u64,
}

impl Sent {
    fn sha256(&self) -> String {
        hex::encode(self.digest.clone().finalize())
    }
}

/// The file as a stream, hashing what is *sent* rather than trusting what was read
/// earlier.
///
/// The hash and the PUT are two separate reads of the same file, and between them
/// the game may have rotated the save (`save` to `save.bak`, with a new `save` in
/// its place: the ordinary autosave pattern). When that happens, the NEW bytes go
/// into the bucket under the OLD sha: a blob whose content is not what its name
/// promises. Restoring it hands back a different save, or junk, and nothing along
/// the way complains. It is the only silent corruption we have found (aug-2026,
/// about 1.7% of the population at risk).
///
/// By hashing the stream itself, it can be checked after the PUT. See
/// [`verify_sent`] for what is done with the verdict.
fn hashing_stream(
    file: tokio::fs::File,
) -> (
    impl futures::Stream<Item = std::io::Result<bytes::Bytes>>,
    std::sync::Arc<std::sync::Mutex<Sent>>,
) {
    let sent = std::sync::Arc::new(std::sync::Mutex::new(Sent::default()));
    let tap = sent.clone();
    let stream = tokio_util::io::ReaderStream::new(file).inspect_ok(move |chunk| {
        if let Ok(mut s) = tap.lock() {
            s.digest.update(chunk);
            s.len += chunk.len() as u64;
        }
    });
    (stream, sent)
}

/// Is what went out what was declared? Pure, so it can be tested without a network.
///
/// Size alone is not enough: a rotation can leave a file of the same length. The
/// sha decides; the size is checked as well because a mismatch there also means the
/// PUT's `content-length` lied.
fn verify_sent(
    relative_path: &str,
    declared_sha: &str,
    declared_len: u64,
    sent: &Sent,
) -> Result<()> {
    let actual = sent.sha256();
    if actual == declared_sha && sent.len == declared_len {
        return Ok(());
    }
    bail!(
        "{relative_path} changed while it was being uploaded \
         (declared {declared_sha} / {declared_len} B, sent {actual} / {} B). \
         Nothing is committed; the next backup will pick up the new contents.",
        sent.len
    )
}

/// Skip-aware wrapper around [`upload_directory`] (ADR 0019).
///
/// Two-tier check against the persisted composite `prev_signature`:
/// 1. Cheap `(path, size, mtime)` signature matches → [`BackupResult::Skipped`],
///    no file read, no network.
/// 2. Cheap drifted (usually just an mtime bump from a game/daemon rewriting
///    saves on a timer) → read bytes once; if the content hash matches the
///    stored one → [`BackupResult::Unchanged`] carrying the refreshed composite
///    so the next cycle hits the fast path again instead of re-hashing.
/// 3. Bytes actually moved → upload and return [`BackupResult::Uploaded`].
///
/// **Both gates are skipped for a deliberate copy** ([`VersionOrigin::is_deliberate`]:
/// the user's own "back up now" and the safety net taken before a restore
/// overwrites the folder). A copy the user
/// asked for is a marker they placed ("right here, before the boss") and
/// whether the bytes happen to match the last autosave is beside the point. It
/// used to fall through the same gate as a watcher no-op, so pressing the button
/// with nothing changed did nothing at all: no version, no message, just an INFO
/// line in a log file the user never opens (ago-2026). The rest of the design
/// already assumes a deliberate copy is worth keeping on its own terms: manual
/// versions have their own budget precisely so an autosave burst can't evict
/// them.
///
/// It cannot loop: the gates exist to stop the watcher re-cutting identical
/// snapshots on a timer, and this path only runs when a person presses a button.
/// It costs no transfer either, since the content is addressed, so the blobs are
/// already there and the commit only adds a version row.
///
/// The signature persisted by the caller is `"<cheap>:<content>"`.
#[allow(clippy::too_many_arguments)]
pub async fn upload_directory_checked<F, G>(
    client: &ApiClient,
    save_id: &str,
    game_slug: &str,
    label: &str,
    source: &Path,
    prev_signature: Option<&str>,
    base_version: Option<i64>,
    head: Option<&ServerHead>,
    origin: VersionOrigin,
    progress: F,
    on_upload_start: G,
) -> Result<BackupResult>
where
    F: Fn(u64, u64) + Send + Sync,
    G: FnOnce(),
{
    // Before the disk is touched: an impossible root is not walked. Walking a whole
    // Proton prefix to discover it should not be uploaded costs exactly what we are
    // trying to avoid.
    if let Some(reason) = crate::junkdirs::dangerous_sync_root(source) {
        return Err(UnsafeSource {
            path: source.to_path_buf(),
            reason,
        }
        .into());
    }
    let canonical = source
        .canonicalize()
        .with_context(|| format!("source path does not exist: {}", source.display()))?;
    if !canonical.is_dir() && !canonical.is_file() {
        bail!("source must be a folder or a file: {}", canonical.display());
    }
    // And again over the resolved path: an innocent symlink can point at the whole
    // profile, and what gets walked is the destination.
    if let Some(reason) = crate::junkdirs::dangerous_sync_root(&canonical) {
        return Err(UnsafeSource {
            path: canonical,
            reason,
        }
        .into());
    }
    let files = walk_source(&canonical, &crate::savefilter::shields_for_slug(game_slug))?;
    if files.is_empty() {
        return Err(EmptySource { path: canonical }.into());
    }
    let (prev_cheap, prev_content) = split_signature(prev_signature);
    let cheap = compute_set_signature(&files);
    // `is_deliberate` rather than `== Manual`: the safety net taken before a
    // restore counts too, and skipping it there is worse, since it is the copy that
    // lets a wrong restore be undone.
    let deliberate = origin.is_deliberate();
    if !deliberate && prev_cheap == Some(cheap.as_str()) {
        // Fast path: the cheap (path, size, mtime) signature is unchanged, so
        // the bytes can't have moved either, so skip without reading any file.
        return Ok(BackupResult::Skipped);
    }
    // The cheap signature drifted. That's often just an mtime bump (a game or
    // background daemon rewriting save files on a timer), so confirm whether
    // the actual bytes changed before cutting a snapshot.
    let content = compute_content_signature(&files).await;
    if !deliberate && prev_content == Some(content.as_str()) {
        return Ok(BackupResult::Unchanged {
            signature: join_signature(&cheap, &content),
        });
    }
    // The bytes genuinely moved: we're about to push a real snapshot. Signal
    // it now (after every skip/unchanged check) so callers only surface a
    // "uploading…" notice when something actually uploads.
    on_upload_start();
    let outcome = upload_directory(
        client,
        save_id,
        game_slug,
        label,
        &canonical,
        base_version,
        head,
        origin,
        progress,
    )
    .await?;
    // The content was already up there (D.8.3): there was no upload, but there is a
    // version we are now synced to. It is kept apart from `Uploaded` because the
    // caller must NOT count it as a committing backup: moving the min-interval
    // anchor with something that was not uploaded is the R.E.P.O. regression.
    if outcome.landed {
        return Ok(BackupResult::AlreadyLanded {
            version_num: outcome.snapshot.version_num,
            signature: join_signature(&cheap, &content),
        });
    }
    Ok(BackupResult::Uploaded {
        outcome,
        signature: join_signature(&cheap, &content),
    })
}

/// Persist (or refresh) the `(save_id → local_path)` mapping in `state.json`.
///
/// If `remember` is true, fetch the save's metadata from the server and write
/// a fresh entry. If false but an entry already exists, just bump the
/// `last_backup_at` and `last_version_num` fields.
pub async fn remember_save(
    client: &ApiClient,
    state: &mut CliState,
    save_id: &str,
    local_path: &Path,
    last_version_num: i64,
    remember: bool,
) -> Result<()> {
    if remember {
        let save = client.get_save(save_id).await?;
        // Preserve any user-set pause flag if the entry already existed:
        // re-fetching from the server shouldn't silently un-pause it.
        let was_paused = state.saves.get(save_id).map(|s| s.paused).unwrap_or(false);
        // Preserve the skip-by-hash signature across a metadata refresh too,
        // so re-remembering a save doesn't force a redundant next upload.
        let prev_hash = state.saves.get(save_id).and_then(|s| s.set_hash.clone());
        let prev_preset = state.saves.get(save_id).and_then(|s| s.preset.clone());
        let prev_processes = state
            .saves
            .get(save_id)
            .map(|s| s.processes.clone())
            .unwrap_or_default();
        let prev_shared = state.saves.get(save_id).is_some_and(|s| s.shared_processes);
        // Same as the pause flag and the preset: a metadata refresh cannot undo
        // a user setting. This one decides whether their config gets written on
        // restore, so losing it here would be losing it silently.
        let prev_allow_device_local = state.saves.get(save_id).and_then(|s| s.allow_device_local);
        state.saves.insert(
            save_id.to_string(),
            SaveState {
                local_path: local_path.to_path_buf(),
                game_slug: save.game_slug.into_inner(),
                label: save.label,
                last_backup_at: Some(OffsetDateTime::now_utc()),
                last_version_num: Some(last_version_num),
                paused: was_paused,
                preset: prev_preset,
                set_hash: prev_hash,
                processes: prev_processes,
                shared_processes: prev_shared,
                allow_device_local: prev_allow_device_local,
            },
        );
    } else if let Some(existing) = state.saves.get(save_id).cloned() {
        state.saves.insert(
            save_id.to_string(),
            SaveState {
                last_backup_at: Some(OffsetDateTime::now_utc()),
                last_version_num: Some(last_version_num),
                ..existing
            },
        );
    }
    Ok(())
}

#[cfg(test)]
mod trim_tests {
    use super::*;

    fn f(path: &str, size: u64, secs: u64) -> UploadFile {
        UploadFile {
            relative_path: path.to_string(),
            absolute_path: PathBuf::from(path),
            size_bytes: size,
            modified: Some(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(secs)),
        }
    }

    /// The trim keeps the newest, not the first ones it sees.
    #[test]
    fn keeps_the_newest_files_that_fit() {
        let files = vec![f("old", 60, 100), f("new", 30, 300), f("mid", 30, 200)];
        let mut working: Vec<&UploadFile> = files.iter().collect();
        working.sort_by_key(|f| std::cmp::Reverse(f.modified));

        let info = trim_to_cap(&mut working, 60, "free").expect("something fits");

        let kept: Vec<&str> = working.iter().map(|f| f.relative_path.as_str()).collect();
        assert_eq!(kept, ["new", "mid"]);
        assert_eq!(info.kept_bytes, 60);
        assert_eq!(info.omitted_files, 1);
        assert_eq!(info.omitted_bytes, 60);
        assert_eq!(info.limit_bytes, 60);
    }

    /// Not even the newest fits: there is no trim, and the caller has to treat it
    /// as a terminal "too large" rather than uploading an empty copy.
    #[test]
    fn refuses_when_even_the_newest_file_is_over_the_cap() {
        let files = vec![f("huge", 500, 300)];
        let mut working: Vec<&UploadFile> = files.iter().collect();
        assert!(trim_to_cap(&mut working, 100, "free").is_none());
        // And it does not touch the set: nothing has been decided yet.
        assert_eq!(working.len(), 1);
    }

    /// Everything fits: it is kept whole and the report says nothing was omitted.
    #[test]
    fn keeps_everything_when_it_all_fits() {
        let files = vec![f("a", 10, 100), f("b", 10, 200)];
        let mut working: Vec<&UploadFile> = files.iter().collect();
        let info = trim_to_cap(&mut working, 1000, "pro").expect("all fits");
        assert_eq!(working.len(), 2);
        assert_eq!(info.omitted_files, 0);
        assert_eq!(info.omitted_bytes, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uf(rel: &str, size: u64, mtime_secs: u64) -> UploadFile {
        UploadFile {
            relative_path: rel.to_string(),
            absolute_path: PathBuf::from("/x").join(rel),
            size_bytes: size,
            modified: Some(UNIX_EPOCH + std::time::Duration::from_secs(mtime_secs)),
        }
    }

    /// The heart of the issue #17 fix: a server that answers and closes without
    /// draining the body must be recognised as pacing.
    ///
    /// Assuming the `io::Error` survives the `reqwest` → `hyper` → `anyhow`
    /// trip is not good enough, so the real failure is manufactured here: a
    /// listener that accepts, reads a little and aborts with RST while the
    /// client is still writing a large body. Exactly what the per-IP limiter
    /// does when it turns a PUT away.
    #[tokio::test]
    async fn a_reset_while_writing_the_body_reads_as_pacing() {
        use tokio::io::AsyncReadExt;

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            // Just enough for the client to have started writing the body.
            let mut buf = [0u8; 1024];
            let _ = sock.read(&mut buf).await;
            // Closed with the body half-arrived. The client keeps writing,
            // those bytes land on a closed socket and the server's stack
            // answers RST: exactly what a client sees when its PUT is turned
            // away undrained.
            drop(sock);
        });

        // Large, lazy body: it has to still be writing when the RST lands, or
        // the failure would be reading the response rather than writing.
        let body = reqwest::Body::wrap_stream(stream::iter(
            (0..4096).map(|_| Ok::<_, std::io::Error>(vec![0u8; 16 * 1024])),
        ));
        let sent = reqwest::Client::new()
            .put(format!("http://{addr}/v1/cas/blobs/x/y"))
            .body(body)
            .send()
            .await;

        let err = anyhow::Error::from(sent.expect_err("the server aborts the connection"))
            .context("uploading promo/sandbox/junkyard.jpg");

        assert!(
            is_body_write_reset(&err),
            "an aborted body write must be recognised; got: {err:#}"
        );
        assert_eq!(
            paced_wait_hint(&err),
            Some(0),
            "and must be treated as pacing without a hint"
        );
    }

    /// The other half: an error that is not a socket teardown still aborts the
    /// batch. Without this the pacer would swallow any failure and retry a blob
    /// that is never going to upload six times over.
    #[test]
    fn an_unrelated_error_is_not_pacing() {
        let err = anyhow::anyhow!("opening /x/save.dat: no such file or directory");
        assert!(!is_body_write_reset(&err));
        assert_eq!(paced_wait_hint(&err), None);
    }

    /// The stream feeding the PUT has to hash what it sends, not what was read
    /// earlier. It is checked against `hash_file`, which is the hash declared in
    /// the manifest: if the two agree over the same file, the later check cannot
    /// give false positives.
    #[tokio::test]
    async fn the_upload_stream_hashes_exactly_what_it_sends() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("save.dat");
        // More than one `ReaderStream` chunk (8 KiB) so the digest really has to
        // accumulate.
        let bytes: Vec<u8> = (0..40_000u32).map(|i| (i % 251) as u8).collect();
        std::fs::write(&path, &bytes).unwrap();

        let file = tokio::fs::File::open(&path).await.unwrap();
        let (stream, sent) = hashing_stream(file);
        let drained: u64 = stream
            .fold(0u64, |acc, chunk| async move {
                acc + chunk.unwrap().len() as u64
            })
            .await;

        // The file's hash is asked for BEFORE the lock is taken: holding one across
        // an `await` is what `clippy::await_holding_lock` reports, and CI runs
        // clippy with `-D warnings` over `--all-targets`.
        let want = hash_file(&path).await.unwrap();

        let sent = sent.lock().unwrap();
        assert_eq!(drained, bytes.len() as u64);
        assert_eq!(sent.len, bytes.len() as u64);
        assert_eq!(sent.sha256(), want);
        assert!(verify_sent("save.dat", &sent.sha256(), sent.len, &sent).is_ok());
    }

    /// The rotation, which is the real case: the old `save`'s sha is declared and
    /// the new one's bytes go out over the socket. Before this the version was
    /// confirmed anyway and the blob was left lying about its content.
    #[tokio::test]
    async fn a_file_rotated_mid_upload_is_caught() {
        let tmp = tempfile::tempdir().unwrap();
        let viejo = tmp.path().join("viejo.dat");
        let nuevo = tmp.path().join("nuevo.dat");
        std::fs::write(&viejo, b"partida de ayer").unwrap();
        // Mismo tamaño a propósito: el largo no basta como árbitro.
        std::fs::write(&nuevo, b"partida de HOY!").unwrap();
        let sha_viejo = hash_file(&viejo).await.unwrap();
        let len = std::fs::metadata(&viejo).unwrap().len();
        assert_ne!(sha_viejo, hash_file(&nuevo).await.unwrap());

        // What is there now (the rotated file) gets uploaded under the declared sha.
        let file = tokio::fs::File::open(&nuevo).await.unwrap();
        let (stream, sent) = hashing_stream(file);
        stream.for_each(|_| async {}).await;

        let sent = sent.lock().unwrap();
        assert_eq!(sent.len, len, "misma longitud: sólo el sha lo delata");
        let err = verify_sent("save.dat", &sha_viejo, len, &sent).unwrap_err();
        assert!(
            err.to_string()
                .contains("changed while it was being uploaded"),
            "{err}"
        );

        // And a different size is caught too (a file truncated mid-upload).
        let corto = Sent {
            digest: Sha256::new(),
            len: 3,
        };
        assert!(verify_sent("save.dat", &corto.sha256(), len, &corto).is_err());
    }

    #[test]
    fn signature_stable_for_identical_set() {
        let a = vec![uf("a.sav", 10, 100), uf("b.sav", 20, 200)];
        let b = vec![uf("a.sav", 10, 100), uf("b.sav", 20, 200)];
        assert_eq!(compute_set_signature(&a), compute_set_signature(&b));
    }

    #[test]
    fn signature_changes_on_size_mtime_or_path() {
        let base = [uf("a.sav", 10, 100)];
        let base_sig = compute_set_signature(&base);
        assert_ne!(base_sig, compute_set_signature(&[uf("a.sav", 11, 100)]));
        assert_ne!(base_sig, compute_set_signature(&[uf("a.sav", 10, 101)]));
        assert_ne!(base_sig, compute_set_signature(&[uf("b.sav", 10, 100)]));
    }

    #[test]
    fn signature_distinguishes_extra_file() {
        let one = vec![uf("a.sav", 10, 100)];
        let two = vec![uf("a.sav", 10, 100), uf("b.sav", 5, 50)];
        assert_ne!(compute_set_signature(&one), compute_set_signature(&two));
    }

    #[test]
    fn split_join_round_trip() {
        assert_eq!(split_signature(None), (None, None));
        // Legacy cheap-only state (pre-fallback): no content half.
        assert_eq!(split_signature(Some("abc")), (Some("abc"), None));
        let composite = join_signature("cheap", "content");
        assert_eq!(composite, "cheap:content");
        assert_eq!(
            split_signature(Some(&composite)),
            (Some("cheap"), Some("content"))
        );
    }

    /// The manifest's digest has to be the same number the server computes when it
    /// commits a content-addressed version
    /// (`hoard-server/src/cloud/routes/saves.rs`, `cas_commit`): sha256 of
    /// `path \0 sha \0 size(i64 le) \0` per row, sorted by path. If our half
    /// drifted, D.8.3's check would never find a match and we would silently go
    /// back to uploading too much: no error to look at, only a bill. That is why
    /// the vector is fixed and computed separately rather than derived from this
    /// same function.
    #[test]
    fn manifest_digest_matches_the_servers_algorithm() {
        let rows = [
            ("saves/autosave.sav", "9f".repeat(32), 4096i64),
            ("saves/slot1.sav", "ab".repeat(32), 12i64),
        ];
        let digest = manifest_digest(rows.iter().map(|(p, sha, size)| (*p, sha.as_str(), *size)));
        assert_eq!(
            digest, "729ed0eaf73d058e463dea699aa20a6d131b9a347d5ace1c4f93fdda86cac9fe",
            "the manifest digest drifted from the server's"
        );
    }

    /// And all three things that make it up count: the order, the size and the
    /// path. A digest ignoring any of them could call content "already uploaded"
    /// when it is not up there, which is the only way this check can lose data.
    #[test]
    fn manifest_digest_is_sensitive_to_order_size_and_path() {
        let sha_a = "11".repeat(32);
        let sha_b = "22".repeat(32);
        let base =
            manifest_digest([("a", sha_a.as_str(), 1i64), ("b", sha_b.as_str(), 2i64)].into_iter());
        let swapped =
            manifest_digest([("b", sha_b.as_str(), 2i64), ("a", sha_a.as_str(), 1i64)].into_iter());
        let resized =
            manifest_digest([("a", sha_a.as_str(), 9i64), ("b", sha_b.as_str(), 2i64)].into_iter());
        let renamed = manifest_digest(
            [("a2", sha_a.as_str(), 1i64), ("b", sha_b.as_str(), 2i64)].into_iter(),
        );
        assert_ne!(base, swapped);
        assert_ne!(base, resized);
        assert_ne!(base, renamed);
    }

    #[tokio::test]
    async fn content_signature_ignores_mtime_but_tracks_bytes() {
        let dir = std::env::temp_dir().join(format!("hoard-sig-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("save.dat");
        std::fs::write(&path, b"hello world").unwrap();
        let mk = |mtime: u64| UploadFile {
            relative_path: "save.dat".to_string(),
            absolute_path: path.clone(),
            size_bytes: 11,
            modified: Some(UNIX_EPOCH + std::time::Duration::from_secs(mtime)),
        };
        // Cheap signature drifts with mtime, content signature does not.
        let a = vec![mk(100)];
        let b = vec![mk(999)];
        assert_ne!(compute_set_signature(&a), compute_set_signature(&b));
        assert_eq!(
            compute_content_signature(&a).await,
            compute_content_signature(&b).await
        );
        // Changing the bytes does move the content signature.
        let before = compute_content_signature(&a).await;
        std::fs::write(&path, b"hello WORLD").unwrap();
        let after = compute_content_signature(&a).await;
        assert_ne!(before, after);
        std::fs::remove_dir_all(&dir).ok();
    }

    /// The bug: a single unreadable file brought the content signature down, and
    /// the whole snapshot with it. The walk already skipped what it could not
    /// interrogate; the read propagated the error with `?`. One OneDrive
    /// placeholder inside one game's save was enough for 3,934 attempts in 13 days
    /// without a single version uploaded.
    #[cfg(unix)]
    #[tokio::test]
    async fn one_unreadable_file_does_not_kill_the_signature() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::write(root.join("good.sav"), b"real save").unwrap();
        let bad = root.join("bad.sav");
        std::fs::write(&bad, b"placeholder").unwrap();
        std::fs::set_permissions(&bad, std::fs::Permissions::from_mode(0o000)).unwrap();

        let files = walk_source(root, &[]).unwrap();
        assert_eq!(
            files.len(),
            2,
            "el walk sí ve el fichero: puede hacerle stat"
        );
        let sig = compute_content_signature(&files).await;
        // And it is stable while it stays unreadable: if it were not, every pass
        // would see a change and we would be back to the upload loop.
        assert_eq!(sig, compute_content_signature(&files).await);

        // The readable one's bytes do count.
        std::fs::write(root.join("good.sav"), b"moved on").unwrap();
        let moved = walk_source(root, &[]).unwrap();
        assert_ne!(sig, compute_content_signature(&moved).await);

        std::fs::set_permissions(&bad, std::fs::Permissions::from_mode(0o644)).unwrap();
    }

    /// An unreadable file must not be confused with an empty one: if the marker
    /// did not enter the digest, "it will not be read" and "it is empty" would
    /// sign the same, and a folder that regains access to an empty file would not
    /// be re-uploaded.
    #[cfg(unix)]
    #[tokio::test]
    async fn unreadable_and_empty_do_not_sign_the_same() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let f = root.join("a.sav");
        std::fs::write(&f, b"").unwrap();
        let as_empty = compute_content_signature(&walk_source(root, &[]).unwrap()).await;
        std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o000)).unwrap();
        let as_unreadable = compute_content_signature(&walk_source(root, &[]).unwrap()).await;
        std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert_ne!(as_empty, as_unreadable);
    }

    /// The upload's filter: the unreadable ones leave the list and get reported,
    /// and the rest travel. It is what stops the file reappearing inside the cloud
    /// path's tar or the CAS hashing.
    #[cfg(unix)]
    #[tokio::test]
    async fn unreadable_files_are_split_off_and_reported() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::write(root.join("a.sav"), b"one").unwrap();
        std::fs::write(root.join("c.sav"), b"three").unwrap();
        let bad = root.join("b.sav");
        std::fs::write(&bad, b"two").unwrap();
        std::fs::set_permissions(&bad, std::fs::Permissions::from_mode(0o000)).unwrap();

        let (ok, skipped) = split_unreadable(walk_source(root, &[]).unwrap()).await;
        std::fs::set_permissions(&bad, std::fs::Permissions::from_mode(0o644)).unwrap();

        assert_eq!(
            ok.iter()
                .map(|f| f.relative_path.as_str())
                .collect::<Vec<_>>(),
            ["a.sav", "c.sav"],
            "el orden por ruta se conserva: el digest del manifiesto depende de él"
        );
        assert_eq!(skipped.len(), 1);
        assert_eq!(skipped[0].relative_path, "b.sav");
        assert!(
            !skipped[0].error.is_empty(),
            "el error del sistema es lo único accionable que ve el usuario"
        );
    }

    /// A subfolder without permission cannot bring down the whole game's backup:
    /// on Windows the profile's legacy junctions return access denied as a matter
    /// of course.
    /// forma rutinaria.
    #[cfg(unix)]
    #[test]
    fn an_unreadable_subdir_is_skipped_not_fatal() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::write(root.join("save.dat"), b"real save").unwrap();
        let locked = root.join("locked");
        std::fs::create_dir(&locked).unwrap();
        std::fs::write(locked.join("inner.dat"), b"x").unwrap();
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();

        let files = walk_source(root, &[]).expect("un subdir ilegible no debe abortar el walk");
        // Restore the permissions so the tempdir can be cleaned up.
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();

        assert_eq!(
            files
                .iter()
                .map(|f| f.relative_path.as_str())
                .collect::<Vec<_>>(),
            ["save.dat"],
            "el save legible debe seguir estando"
        );
    }

    /// The root IS a hard error: if it cannot be read there is no backup to make.
    /// y decirlo en voz alta es lo correcto.
    #[test]
    fn an_unreadable_root_is_still_an_error() {
        let missing = std::path::Path::new("/definitely/not/here/hoard-test");
        assert!(walk_source(missing, &[]).is_err());
    }

    /// A single-file save: 4,900 games in the catalogue have only templates
    /// pointing at a file. The snapshot comes out with the same shape as one from
    /// a folder with a file in it, so signature,
    /// dedup y restore siguen funcionando sin cambios.
    /// The backup path's guard: an impossible root is rejected before anything is
    /// walked. It is the aug-2026 Steam Deck case, where the save had been tracked
    /// long before, and the structural guard only ran on adding, so that row never
    /// went through it again.
    ///
    /// It is checked with a REAL, populated folder: if the rejection depended on
    /// the path not existing or being empty, this test would pass for the wrong
    /// reason.
    /// equivocado.
    #[tokio::test]
    async fn a_whole_proton_prefix_is_refused_before_walking_it() {
        let tmp = tempfile::tempdir().unwrap();
        // .../compatdata/423230/pfx with content in it, like the real one.
        let save_dir = tmp.path().join(
            "steamapps/compatdata/423230/pfx/drive_c/users/steamuser/AppData/LocalLow/TheGameBakers/Furi",
        );
        std::fs::create_dir_all(&save_dir).unwrap();
        std::fs::write(save_dir.join("algo.dat"), b"x").unwrap();
        let prefix_root = tmp.path().join("steamapps/compatdata/423230/pfx");

        let client = crate::api::ApiClient::new("http://127.0.0.1:1", "t").unwrap();
        let err = upload_directory_checked(
            &client,
            "save-1",
            "furi",
            "main",
            &prefix_root,
            None,
            None,
            None,
            VersionOrigin::Automatic,
            |_, _| {},
            || {},
        )
        .await
        .expect_err("un prefijo entero no puede subirse");

        let unsafe_src = err
            .chain()
            .find_map(|c| c.downcast_ref::<UnsafeSource>())
            .expect("debe ser UnsafeSource, no un error de red ni de walk");
        assert!(
            unsafe_src.reason.to_lowercase().contains("prefix"),
            "el motivo debe nombrar el prefijo: {}",
            unsafe_src.reason
        );

        // And the good folder INSIDE the prefix is not rejected here: it will fail
        // later for not being able to reach the server, which is a different thing.
        // Mind the path: `drive_c/users/steamuser` on its own is a whole profile
        // and the guard rejects it quite rightly (the Windows rules are reused
        // inside the prefix). It has to be the game's own folder.
        // juego de verdad.
        let err = upload_directory_checked(
            &client,
            "save-1",
            "furi",
            "main",
            &save_dir,
            None,
            None,
            None,
            VersionOrigin::Automatic,
            |_, _| {},
            || {},
        )
        .await
        .expect_err("sin servidor, falla igual");
        assert!(
            !err.chain().any(|c| c.is::<UnsafeSource>()),
            "la carpeta de dentro no puede rechazarse por forma: {err:#}"
        );
    }

    /// La carpeta real de Cell to Singularity: partidas y telemetría de Unity
    /// mixed together. Before this the snapshot took both, and a log rewritten on
    /// every launch cut a new cloud version every time the user opened the game.
    #[test]
    fn the_walk_leaves_engine_telemetry_out_of_the_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        for (rel, body) in [
            ("savedGames.gd", "partida"),
            ("savedGames2.gd", "partida"),
            ("Player.log", "log"),
            ("Player-prev.log", "log"),
            ("steam_autocloud.vdf", "vdf"),
        ] {
            std::fs::write(root.join(rel), body).unwrap();
        }
        let analytics = root.join("Unity/0a8833bc-a8ad/Analytics");
        std::fs::create_dir_all(&analytics).unwrap();
        std::fs::write(analytics.join("values"), "telemetría").unwrap();

        let files = walk_source(root, &[]).unwrap();
        let names: Vec<&str> = files.iter().map(|f| f.relative_path.as_str()).collect();
        assert_eq!(names, vec!["savedGames.gd", "savedGames2.gd"], "{names:?}");
    }

    /// Config does upload: losing it is not an option, and the protection against
    /// the crash lives in the restore, not here.
    #[test]
    fn config_still_goes_up() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("slot1.sav"), "partida").unwrap();
        std::fs::write(root.join("graphics.ini"), "res=1920x1080").unwrap();

        let files = walk_source(root, &[]).unwrap();
        let names: Vec<&str> = files.iter().map(|f| f.relative_path.as_str()).collect();
        assert_eq!(names, vec!["graphics.ini", "slot1.sav"], "{names:?}");
    }

    /// The log the game rewrites on every launch no longer moves the signature, so
    /// it stops cutting a version per session.
    #[test]
    fn a_rewritten_engine_log_no_longer_drifts_the_signature() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("slot1.sav"), "partida").unwrap();
        std::fs::write(root.join("Player.log"), "arranque 1").unwrap();
        let before = compute_set_signature(&walk_source(root, &[]).unwrap());

        std::fs::write(root.join("Player.log"), "arranque 2, más largo").unwrap();
        let after = compute_set_signature(&walk_source(root, &[]).unwrap());
        assert_eq!(before, after, "el log no debe mover la firma");

        // And the save does move it, which is what has to keep happening.
        std::fs::write(root.join("slot1.sav"), "partida avanzada").unwrap();
        assert_ne!(
            before,
            compute_set_signature(&walk_source(root, &[]).unwrap())
        );
    }

    /// The manifest's shields rescue what the name rules would take: `.log` is the
    /// save pattern of 64 catalogue templates.
    #[test]
    fn a_manifest_pattern_rescues_a_file_the_rules_would_drop() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("player.log"), "esto sí es la partida").unwrap();

        assert!(walk_source(root, &[]).unwrap().is_empty());
        let shielded = walk_source(root, &["*.log".to_string()]).unwrap();
        assert_eq!(shielded.len(), 1);
    }

    /// Tracking the folder that holds one subfolder per save takes every save
    /// under it, new ones included, which is the whole point of pointing Hoard at the
    /// parent instead of filing each save by hand. Detection is what used to
    /// stand in the way (see `detection::is_nest_of_save_dirs`); the walk never
    /// did, and this pins that down so no future depth cap quietly breaks it.
    #[test]
    fn tracking_the_parent_takes_every_save_folder_under_it() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("Cyberpunk 2077");
        for slot in ["AutoSave-0", "ManualSave-0", "QuickSave-0"] {
            let dir = game.join(slot);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("sav.dat"), b"save").unwrap();
            std::fs::write(dir.join("metadata.9.json"), b"{}").unwrap();
        }

        let files = walk_source(&game, &[]).unwrap();
        let paths: Vec<&str> = files.iter().map(|f| f.relative_path.as_str()).collect();
        assert_eq!(
            paths,
            vec![
                "AutoSave-0/metadata.9.json",
                "AutoSave-0/sav.dat",
                "ManualSave-0/metadata.9.json",
                "ManualSave-0/sav.dat",
                "QuickSave-0/metadata.9.json",
                "QuickSave-0/sav.dat",
            ]
        );
    }

    /// A single-file save uploads even when its name looks like config: the user
    /// pointed at that file, and that outweighs any rule.
    #[test]
    fn a_single_file_save_is_never_filtered_out() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("settings.ini");
        std::fs::write(&file, "en realidad es la partida").unwrap();
        let files = walk_source(&file, &[]).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].relative_path, "settings.ini");
    }

    #[test]
    fn a_single_file_save_walks_to_one_entry_named_after_it() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("ssr_save.bin");
        std::fs::write(&file, b"0123456789").unwrap();

        let files = walk_source(&file, &[]).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].relative_path, "ssr_save.bin");
        assert_eq!(files[0].absolute_path, file);
        assert_eq!(files[0].size_bytes, 10);
        assert!(files[0].modified.is_some());
    }

    /// And its signature behaves like any other: it changes with the content,
    /// which is what keeps skip-by-set-hash correct.
    #[test]
    fn a_single_file_saves_signature_tracks_its_content() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("save.dat");
        std::fs::write(&file, b"a").unwrap();
        let before = compute_set_signature(&walk_source(&file, &[]).unwrap());
        // Un tamaño distinto mueve la firma aunque el mtime tenga poca
        // resolución en este sistema de ficheros.
        std::fs::write(&file, b"bbbb").unwrap();
        let after = compute_set_signature(&walk_source(&file, &[]).unwrap());
        assert_ne!(before, after);
    }
}

#[cfg(test)]
mod paced_upload_tests {
    use super::*;
    use std::sync::atomic::AtomicU32;

    fn paced_error() -> anyhow::Error {
        anyhow!(ApiError::RateLimited {
            kind: RateLimitKind::Paced,
            retry_after_seconds: 0,
            body: "Too Many Requests! Wait for 0s".into(),
        })
        .context("uploading AutoSave-6/sav.dat")
    }

    fn budget_error() -> anyhow::Error {
        anyhow!(ApiError::RateLimited {
            kind: RateLimitKind::Budget,
            retry_after_seconds: 420,
            body: r#"{"code":"bandwidth_limit","retry_after_seconds":420}"#.into(),
        })
        .context("uploading AutoSave-6/sav.dat")
    }

    /// The bug this whole path exists for: a save with more blobs than the
    /// server's burst allows used to lose every blob it had already uploaded
    /// the moment the pacer turned one away.
    #[tokio::test(start_paused = true)]
    async fn a_paced_blob_is_retried_rather_than_dropped() {
        let calls = AtomicU32::new(0);
        let waited = AtomicU64::new(0);
        let r = put_blob_paced("AutoSave-6/sav.dat", &waited, || async {
            match calls.fetch_add(1, Ordering::Relaxed) {
                0 | 1 => Err(paced_error()),
                _ => Ok(()),
            }
        })
        .await;
        assert!(r.is_ok(), "{:?}", r.err());
        assert_eq!(calls.load(Ordering::Relaxed), 3);
    }

    /// A budget is the opposite instruction: the operation doesn't fit right
    /// now, and re-sending the same PUT can only make it worse. It has to
    /// travel up untouched so the agent parks the save and comes back later.
    #[tokio::test(start_paused = true)]
    async fn a_budget_429_is_not_retried() {
        let calls = AtomicU32::new(0);
        let waited = AtomicU64::new(0);
        let r = put_blob_paced("AutoSave-6/sav.dat", &waited, || async {
            calls.fetch_add(1, Ordering::Relaxed);
            Err::<(), _>(budget_error())
        })
        .await;
        assert!(r.is_err());
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        let api = r.unwrap_err();
        let api = api.chain().find_map(|c| c.downcast_ref::<ApiError>());
        assert!(matches!(
            api,
            Some(ApiError::RateLimited {
                kind: RateLimitKind::Budget,
                ..
            })
        ));
    }

    /// A server whose limit is simply too tight for this save has to fail
    /// loudly and quickly. Crawling for an hour while pretending to work is
    /// worse than a clear error the operator can act on.
    #[tokio::test(start_paused = true)]
    async fn a_pacer_that_never_relents_gives_up() {
        let calls = AtomicU32::new(0);
        let waited = AtomicU64::new(0);
        let r = put_blob_paced("AutoSave-6/sav.dat", &waited, || async {
            calls.fetch_add(1, Ordering::Relaxed);
            Err::<(), _>(paced_error())
        })
        .await;
        assert!(r.is_err());
        assert_eq!(
            calls.load(Ordering::Relaxed),
            MAX_PACED_RETRIES_PER_BLOB + 1
        );
        assert!(format!("{:#}", r.unwrap_err()).contains("too tight"));
    }

    /// Anything that isn't a pacer keeps its old behaviour: straight up, no
    /// retry. A blob whose bytes stopped matching its sha must not be re-sent.
    #[tokio::test(start_paused = true)]
    async fn a_real_failure_is_not_retried() {
        let calls = AtomicU32::new(0);
        let waited = AtomicU64::new(0);
        let r = put_blob_paced("AutoSave-6/sav.dat", &waited, || async {
            calls.fetch_add(1, Ordering::Relaxed);
            Err::<(), _>(anyhow!("the game rotated the save while it was uploading"))
        })
        .await;
        assert!(r.is_err());
        assert_eq!(calls.load(Ordering::Relaxed), 1);
    }
}
