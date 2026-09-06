//! Sweep of uploads that started and never finished (cloud only).
//!
//! A content-addressed upload is three steps: `cas_init` writes the version row
//! and its manifest, the client PUTs the missing blobs straight to R2, and
//! `cas_commit` checks every blob landed and stamps `sha256`. Only the third
//! step makes the version real; until then `sha256` is `''`, and both the
//! listing and "latest version" filter on `sha256 <> ''`, so an abandoned
//! attempt is correctly invisible to everyone.
//!
//! Invisible, but not free. Nothing ever cleaned up after one, and two kinds of
//! litter were building up in production (ago-2026: 23 abandoned versions
//! across 15 accounts):
//!
//! 1. **Manifest rows.** 45.479 `save_version_files` rows describing versions
//!    that will never exist. One account's emulator library accounted for
//!    21.910 of them in a single attempt.
//! 2. **Orphan blobs in R2.** Whatever the client managed to PUT before it gave
//!    up is in the bucket, but `cas_commit` never ran, so there is no
//!    `cloud_blobs` row referencing it. That makes it invisible to the refcount
//!    GC in `release_blobs` too, since it is charged by Cloudflare and reachable by
//!    nobody, forever.
//!
//! Both are the tail of a failure that is *supposed* to happen sometimes: a
//! laptop closing mid-upload, a quota refusal, an ISP that stops routing to the
//! bucket halfway through a 400 MB save. The upload path is right to give up;
//! this is what picks up behind it.
//!
//! ## Why the orphan sweep is gated on the account being quiet
//!
//! "In R2 with no `cloud_blobs` row" describes an orphan, and it describes
//! every blob of an upload that is *in flight right now*, between its PUT and
//! its commit. The two are indistinguishable from the bucket alone, and
//! deleting the second kind would make a healthy upload fail its own commit
//! with "blob was not uploaded".
//!
//! So the sweep only runs for an account with no uncommitted version younger
//! than [`ABANDONED_AFTER`]. No upload survives that long, since the presigned
//! URLs expire far sooner, so once an account is quiet by that measure, anything in
//! the bucket without a row is litter by elimination. An account that is
//! genuinely mid-upload every time the task runs simply keeps its litter until
//! a round catches it idle, which is the right way to be wrong.
//!
//! ## What is deliberately out of reach
//!
//! The candidate set is whatever `r2::blob_sizes` returns, and that lists the
//! `blobs/<user>/` prefix and keeps only keys whose last segment is exactly 64
//! hex characters. Three things fall outside it, all on purpose:
//!
//! * **Export ZIPs** (`exports/…`) and any legacy per-version archive live
//!   under other prefixes and are never listed.
//! * **Compression staging** (`compress.rs` writes `<key>.ztmp`) fails the
//!   64-character test, so a compression job in flight cannot be mistaken for
//!   litter. Keep that in mind before loosening the filter.
//! * **Archived saves.** Archiving keeps the `cloud_blobs` row (it stamps
//!   `purge_after` and drops the refcount rather than deleting) so a frozen
//!   game is referenced, and referenced is all this sweep looks at. A future
//!   change that archived by *removing* the row would turn this task into a
//!   deleter of other people's archives.

use crate::cloud::state::CloudState;
use std::time::Duration;
use uuid::Uuid;

/// How old an uncommitted version has to be before it counts as abandoned.
///
/// Presigned PUT URLs are minted with a one-hour TTL, so an upload still
/// working an hour after `cas_init` cannot finish anyway: its URLs are dead
/// and the client will start over with a fresh `upload_id`. Twelve hours is
/// that bound with a wide margin for a paused laptop that resumes and manages
/// to commit, and it is the same window that decides an account is quiet enough
/// for the R2 half.
const ABANDONED_AFTER: Duration = Duration::from_secs(12 * 60 * 60);

/// Objects deleted from one account's bucket prefix in a single round.
///
/// A cap, not a target: it bounds the blast radius of a bug in the query above
/// to something a person notices and can stop, and spreads a big backlog over
/// days instead of one enormous delete storm. An account with more orphans than
/// this keeps the remainder until tomorrow's round, which is the right way for
/// a cleanup to be slow.
const MAX_ORPHAN_DELETES_PER_ACCOUNT: usize = 2_000;

/// Spawn the daily sweep. Detached and best-effort, like the other sweepers:
/// a failure warns and the next tick retries.
/// How long after boot the first sweep runs. Long enough that a restart loop
/// cannot turn into an R2 listing loop, short enough that the work actually
/// happens on a service that rarely stays up for a day.
const STARTUP_GRACE: Duration = Duration::from_secs(5 * 60);

pub fn spawn(state: CloudState) {
    tokio::spawn(async move {
        // Once shortly after boot, then daily. The old shape skipped the first
        // tick and only fired a full day in, which on this service means never:
        // the machine idles to zero, a deploy replaces it, the watchdog restarts
        // it, and the clock goes back to zero every time. This sweep did exactly
        // that, silently, from August until 2026-09-06.
        //
        // The grace delay keeps the work off the startup path and stops a crash
        // loop from turning into a bucket-listing loop.
        tokio::time::sleep(STARTUP_GRACE).await;
        let mut tick = tokio::time::interval(Duration::from_secs(24 * 60 * 60));
        tick.tick().await; // the immediate one; the wait happens after the work
        loop {
            match sweep(&state).await {
                Ok(swept) if swept.is_empty() => {}
                Ok(swept) => tracing::info!(
                    versions = swept.versions,
                    manifest_rows = swept.manifest_rows,
                    orphan_blobs = swept.orphan_blobs,
                    orphan_bytes = swept.orphan_bytes,
                    "abandoned uploads: swept"
                ),
                Err(e) => tracing::warn!(error = %e, "abandoned uploads: sweep failed"),
            }
            tick.tick().await;
        }
    });
}

/// What one round reclaimed.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Swept {
    pub versions: u64,
    pub manifest_rows: u64,
    pub orphan_blobs: u64,
    pub orphan_bytes: i64,
}

impl Swept {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// Run one round. Public so `hoard-admin` and the tests can drive it directly
/// instead of waiting a day for the timer.
pub async fn sweep(state: &CloudState) -> Result<Swept, sqlx::Error> {
    // i32, not i64: `make_interval(hours => ...)` is declared over `int4`,
    // and a bigint argument matches no overload. Postgres answers that with
    // `42883 function does not exist`, which the daily task swallowed into a
    // log line, so the sweep never once ran and its work piled up unseen.
    let hours = (ABANDONED_AFTER.as_secs() / 3600) as i32;
    let mut swept = Swept::default();

    // Accounts carrying at least one abandoned version. Doing this per account
    // rather than in one global DELETE is what lets the R2 half ask "is *this*
    // account quiet?", and keeps one account's failure off everyone else's
    // cleanup.
    let accounts: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT DISTINCT s.user_id
           FROM save_versions v
           JOIN saves s ON s.id = v.save_id
          WHERE v.sha256 = ''
            AND v.created_at < now() - make_interval(hours => $1)",
    )
    .bind(hours)
    .fetch_all(&state.pool)
    .await?;

    for (user_id,) in accounts {
        match sweep_account(state, user_id, hours).await {
            Ok(one) => {
                swept.versions += one.versions;
                swept.manifest_rows += one.manifest_rows;
                swept.orphan_blobs += one.orphan_blobs;
                swept.orphan_bytes += one.orphan_bytes;
            }
            Err(e) => {
                tracing::warn!(error = %e, %user_id, "abandoned uploads: account failed");
            }
        }
    }
    Ok(swept)
}

async fn sweep_account(
    state: &CloudState,
    user_id: Uuid,
    hours: i32,
) -> Result<Swept, sqlx::Error> {
    let mut swept = Swept::default();

    // Quiet check first, and it gates **both** halves rather than just the
    // bucket. Doing the rows anyway and skipping only R2 looks harmless and
    // isn't: the candidate list up in [`sweep`] is "accounts with an abandoned
    // version", so an account whose rows we delete drops out of it. Skip the
    // bucket after that and its orphans are stranded, because no later round would
    // ever look at that account again. Leaving the whole account for the next
    // round costs one day and keeps the two halves together.
    let in_flight: i64 = sqlx::query_scalar(
        "SELECT count(*)
           FROM save_versions v
           JOIN saves s ON s.id = v.save_id
          WHERE s.user_id = $1
            AND v.sha256 = ''
            AND v.created_at >= now() - make_interval(hours => $2)",
    )
    .bind(user_id)
    .bind(hours)
    .fetch_one(&state.pool)
    .await?;
    if in_flight > 0 {
        tracing::debug!(
            %user_id,
            in_flight,
            "abandoned uploads: account has an upload in flight; leaving it for the next round"
        );
        return Ok(swept);
    }

    // The manifest rows go with the version row (ON DELETE CASCADE on
    // `save_version_files`), so they're counted before the delete rather than
    // after.
    let (versions, manifest_rows): (i64, i64) = sqlx::query_as(
        "SELECT count(*),
                coalesce(sum((SELECT count(*) FROM save_version_files f
                               WHERE f.save_id = v.save_id
                                 AND f.version_num = v.version_num)), 0)
           FROM save_versions v
           JOIN saves s ON s.id = v.save_id
          WHERE s.user_id = $1
            AND v.sha256 = ''
            AND v.created_at < now() - make_interval(hours => $2)",
    )
    .bind(user_id)
    .bind(hours)
    .fetch_one(&state.pool)
    .await?;

    if versions > 0 {
        sqlx::query(
            "DELETE FROM save_versions v
              USING saves s
              WHERE s.id = v.save_id
                AND s.user_id = $1
                AND v.sha256 = ''
                AND v.created_at < now() - make_interval(hours => $2)",
        )
        .bind(user_id)
        .bind(hours)
        .execute(&state.pool)
        .await?;
        swept.versions = versions as u64;
        swept.manifest_rows = manifest_rows as u64;
    }

    let in_bucket = match state.r2.blob_sizes(user_id).await {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!(error = %e, %user_id, "abandoned uploads: bucket listing failed");
            return Ok(swept);
        }
    };
    if in_bucket.is_empty() {
        return Ok(swept);
    }

    let referenced: Vec<(String,)> =
        sqlx::query_as("SELECT encode(sha256, 'hex') FROM cloud_blobs WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(&state.pool)
            .await?;
    let referenced: std::collections::HashSet<String> =
        referenced.into_iter().map(|(s,)| s).collect();

    // Sorted so a capped round is deterministic: the same objects go first
    // every time, instead of whatever order the listing happened to return.
    let mut orphans: Vec<(&String, &i64)> = in_bucket
        .iter()
        .filter(|(sha, _)| !referenced.contains(*sha))
        .collect();
    orphans.sort_unstable_by(|a, b| a.0.cmp(b.0));

    for (sha, size) in orphans.into_iter().take(MAX_ORPHAN_DELETES_PER_ACCOUNT) {
        let key = crate::cloud::r2::key_for_blob(user_id, sha);
        match state.r2.delete_object(&key).await {
            Ok(()) => {
                swept.orphan_blobs += 1;
                swept.orphan_bytes += *size;
            }
            Err(e) => {
                tracing::warn!(error = %e, r2_key = %key, "abandoned uploads: R2 delete failed");
            }
        }
    }
    if swept.orphan_blobs > 0 {
        tracing::info!(
            %user_id,
            blobs = swept.orphan_blobs,
            bytes = swept.orphan_bytes,
            "abandoned uploads: released orphan blobs nothing referenced"
        );
    }
    Ok(swept)
}
