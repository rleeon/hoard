//! Daily comparison of the bucket against `cloud_blobs` (cloud only).
//!
//! Two things can disagree, and neither shows up anywhere else:
//!
//! * **An object with no row.** Nobody is charged for it and nothing will ever
//!   delete it. The sweep in `abandoned.rs` only visits accounts that still
//!   have an uncommitted version, so it misses an upload whose save was deleted
//!   before the commit (the versions cascade away with it) and whatever is left
//!   over its 2,000-a-round cap once the versions it keyed on are gone. On
//!   2026-09-27 that was 23,938 objects, 14.7 GB, in 68 accounts, growing every
//!   month.
//! * **A row with no object.** A live file that can't be restored. There were
//!   none on 2026-09-27; this is the alarm for the day there are.
//!
//! By default it only counts. `cloud.reconcile_delete` lets it delete the first
//! kind, under the same rules the upload path relies on: older than
//! [`MIN_AGE`], no row, and not named by any version, pending or committed,
//! checked again right before each delete. The account must also be quiet (no
//! upload started in the last [`QUIET`]), as in `abandoned.rs`.

use super::incidents::{self, Kind};
use crate::cloud::state::CloudState;
use futures::StreamExt;
use std::collections::HashSet;
use std::time::Duration;
use uuid::Uuid;

const INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// Later than the integrity audit's grace, so the two don't overlap at boot.
const STARTUP_GRACE: Duration = Duration::from_secs(30 * 60);

const PAUSE: Duration = Duration::from_millis(200);

/// Presigned PUT URLs live an hour; two days is that with room for a laptop
/// that slept mid-upload and a client that retries the commit the next day.
pub const MIN_AGE: Duration = Duration::from_secs(48 * 60 * 60);

/// Same window `abandoned.rs` uses to call an account idle.
const QUIET: Duration = Duration::from_secs(12 * 60 * 60);

/// Deletes per account per round. Unlike the old sweep, the account list is the
/// bucket itself, so whatever is over the cap is found again tomorrow.
const MAX_DELETES_PER_ACCOUNT: usize = 5_000;

/// Re-checked together and then deleted, a batch at a time.
const BATCH: usize = 500;

const CONCURRENCY: usize = 16;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Findings {
    pub accounts: u64,
    pub orphan_objects: u64,
    pub orphan_bytes: i64,
    pub missing_objects: u64,
    pub missing_accounts: Vec<Uuid>,
    pub deleted_objects: u64,
    pub deleted_bytes: i64,
}

pub fn spawn(state: CloudState) {
    tokio::spawn(async move {
        tokio::time::sleep(STARTUP_GRACE).await;
        let mut tick = tokio::time::interval(INTERVAL);
        loop {
            tick.tick().await;
            let delete = state
                .config
                .cloud
                .as_ref()
                .is_some_and(|c| c.reconcile_delete);
            let now = time::OffsetDateTime::now_utc().unix_timestamp();
            match reconcile(&state, None, now, delete).await {
                Ok(f) => report(&f, delete),
                Err(e) => tracing::warn!(error = %e, "reconcile: round failed"),
            }
        }
    });
}

/// One round. `only` narrows it to some accounts; `None` walks every prefix in
/// the bucket, including accounts the database no longer has.
pub async fn reconcile(
    state: &CloudState,
    only: Option<&[Uuid]>,
    now_unix: i64,
    delete: bool,
) -> anyhow::Result<Findings> {
    let owners = match only {
        Some(u) => u.to_vec(),
        None => state.r2.blob_owners().await?,
    };
    let mut f = Findings::default();
    for (i, user) in owners.iter().enumerate() {
        if only.is_none() && i > 0 {
            tokio::time::sleep(PAUSE).await;
        }
        if let Err(e) = account(state, *user, now_unix, delete, &mut f).await {
            // One account's failure is logged and skipped, like every sweeper.
            tracing::warn!(error = %e, user_id = %user, "reconcile: account failed");
        }
        f.accounts += 1;
    }
    Ok(f)
}

async fn account(
    state: &CloudState,
    user: Uuid,
    now_unix: i64,
    delete: bool,
    f: &mut Findings,
) -> anyhow::Result<()> {
    // Live rows are read *before* the listing. A committed row's object was
    // PUT before its commit, so it is already in the bucket when this runs; a
    // row read after the listing could belong to an upload that landed
    // mid-listing and read as missing when it isn't.
    let live: HashSet<String> = sqlx::query_scalar(
        "SELECT encode(sha256, 'hex') FROM cloud_blobs WHERE user_id = $1 AND refcount > 0",
    )
    .bind(user)
    .fetch_all(&state.pool)
    .await?
    .into_iter()
    .collect();

    let listed = state.r2.blobs_with_age(user).await?;

    // Every row, any refcount, read *after* the listing: a row at 0 still owns
    // its object until the GC takes both, and an upload that committed during
    // the listing must not read as an orphan.
    let rows: HashSet<String> =
        sqlx::query_scalar("SELECT encode(sha256, 'hex') FROM cloud_blobs WHERE user_id = $1")
            .bind(user)
            .fetch_all(&state.pool)
            .await?
            .into_iter()
            .collect();
    let named = named_by_any_version(&state.pool, user).await?;

    let in_bucket: HashSet<&str> = listed.iter().map(|(s, _, _)| s.as_str()).collect();
    let missing = live
        .iter()
        .filter(|s| !in_bucket.contains(s.as_str()))
        .count() as u64;
    if missing > 0 {
        f.missing_objects += missing;
        if f.missing_accounts.len() < 20 {
            f.missing_accounts.push(user);
        }
    }

    let min_age = MIN_AGE.as_secs() as i64;
    let orphans: Vec<(&String, i64)> = listed
        .iter()
        .filter(|(sha, _, at)| {
            now_unix - at >= min_age && !rows.contains(sha) && !named.contains(sha)
        })
        .map(|(sha, size, _)| (sha, *size))
        .collect();
    f.orphan_objects += orphans.len() as u64;
    f.orphan_bytes += orphans.iter().map(|(_, s)| s).sum::<i64>();

    if !delete || orphans.is_empty() || !quiet(&state.pool, user).await? {
        return Ok(());
    }
    let orphans: Vec<(String, i64)> = orphans
        .into_iter()
        .take(MAX_DELETES_PER_ACCOUNT)
        .map(|(sha, size)| (sha.clone(), size))
        .collect();
    for batch in orphans.chunks(BATCH) {
        // Checked again right before deleting: an upload of one of these very
        // files may have started since the listing. Fails closed, a query
        // error leaves the whole batch alone.
        let shas: Vec<String> = batch.iter().map(|(s, _)| s.clone()).collect();
        let still: HashSet<String> = still_orphans(&state.pool, user, &shas).await?;
        let doomed: Vec<(String, i64)> = batch
            .iter()
            .filter(|(s, _)| still.contains(s))
            .cloned()
            .collect();
        let results: Vec<(i64, bool)> = futures::stream::iter(doomed.into_iter().map(|(sha, size)| {
            let r2 = state.r2.clone();
            async move {
                let key = crate::cloud::r2::key_for_blob(user, &sha);
                match r2.delete_object(&key).await {
                    Ok(()) => (size, true),
                    Err(e) => {
                        tracing::warn!(error = %e, r2_key = %key, "reconcile: R2 delete failed");
                        (size, false)
                    }
                }
            }
        }))
        .buffer_unordered(CONCURRENCY)
        .collect()
        .await;
        for (size, ok) in results {
            if ok {
                f.deleted_objects += 1;
                f.deleted_bytes += size;
            }
        }
    }
    Ok(())
}

/// Shas any version of this account names, pending or committed. A pending
/// version's blobs are an upload in flight; a committed one's are live files,
/// row or no row.
async fn named_by_any_version(
    pool: &sqlx::PgPool,
    user: Uuid,
) -> Result<HashSet<String>, sqlx::Error> {
    Ok(sqlx::query_scalar(
        "SELECT DISTINCT encode(fe.sha256, 'hex')
           FROM saves s
           JOIN save_versions v ON v.save_id = s.id
           JOIN version_files vf ON vf.version_id = v.id
           JOIN file_entries fe ON fe.id = vf.entry_id
          WHERE s.user_id = $1",
    )
    .bind(user)
    .fetch_all(pool)
    .await?
    .into_iter()
    .collect())
}

async fn quiet(pool: &sqlx::PgPool, user: Uuid) -> Result<bool, sqlx::Error> {
    let in_flight: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM save_versions v JOIN saves s ON s.id = v.save_id
          WHERE s.user_id = $1 AND v.sha256 = ''
            AND v.created_at >= now() - make_interval(secs => $2)",
    )
    .bind(user)
    .bind(QUIET.as_secs() as f64)
    .fetch_one(pool)
    .await?;
    Ok(in_flight == 0)
}

/// Which of `shas` still have no row and no version naming them.
async fn still_orphans(
    pool: &sqlx::PgPool,
    user: Uuid,
    shas: &[String],
) -> Result<HashSet<String>, sqlx::Error> {
    Ok(sqlx::query_scalar(
        "SELECT u FROM unnest($2::text[]) AS u
          WHERE NOT EXISTS (SELECT 1 FROM cloud_blobs b
                             WHERE b.user_id = $1 AND b.sha256 = decode(u, 'hex'))
            AND NOT EXISTS (SELECT 1 FROM file_entries fe
                              JOIN saves s ON s.id = fe.save_id
                              JOIN version_files vf ON vf.entry_id = fe.id
                             WHERE s.user_id = $1 AND fe.sha256 = decode(u, 'hex'))",
    )
    .bind(user)
    .bind(shas)
    .fetch_all(pool)
    .await?
    .into_iter()
    .collect())
}

fn report(f: &Findings, delete: bool) {
    tracing::info!(
        accounts = f.accounts,
        orphan_objects = f.orphan_objects,
        orphan_bytes = f.orphan_bytes,
        deleted_objects = f.deleted_objects,
        deleted_bytes = f.deleted_bytes,
        delete,
        "reconcile: bucket compared with the database"
    );
    if f.missing_objects > 0 {
        tracing::error!(
            objects = f.missing_objects,
            accounts = ?f.missing_accounts,
            "reconcile: live blobs missing from the bucket"
        );
        incidents::record(Kind::Other, "reconcile: live blobs missing from the bucket");
    }
}
