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
use crate::cloud::r2::PatchKey;
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
    let live = rows_of(&state.pool, user, true).await?;

    let listed = state.r2.objects_with_age(user).await?;

    // Every row, any refcount, read *after* the listing: a row at 0 still owns
    // its object until the GC takes both, and an upload that committed during
    // the listing must not read as an orphan.
    let all_rows = rows_of(&state.pool, user, false).await?;
    let rows: HashSet<&str> = all_rows.iter().map(|(s, _)| s.as_str()).collect();
    let row_patches: HashSet<&PatchKey> = all_rows.iter().filter_map(|(_, p)| p.as_ref()).collect();
    let named = named_by_any_version(&state.pool, user).await?;

    let in_bucket: HashSet<&str> = listed.blobs.iter().map(|(s, _, _)| s.as_str()).collect();
    let patches_in_bucket: HashSet<&PatchKey> = listed.patches.iter().map(|(p, _, _)| p).collect();
    let missing = live
        .iter()
        .filter(|(sha, patch)| match patch {
            Some(p) => !patches_in_bucket.contains(p),
            None => !in_bucket.contains(sha.as_str()),
        })
        .count() as u64;
    if missing > 0 {
        f.missing_objects += missing;
        if f.missing_accounts.len() < 20 {
            f.missing_accounts.push(user);
        }
    }

    let min_age = MIN_AGE.as_secs() as i64;
    let orphans: Vec<(&String, i64)> = listed
        .blobs
        .iter()
        .filter(|(sha, _, at)| {
            now_unix - at >= min_age && !rows.contains(sha.as_str()) && !named.contains(sha)
        })
        .map(|(sha, size, _)| (sha, *size))
        .collect();
    // A patch object belongs to its row only if base and encoding match too:
    // one that lost a race to another upload of the same file has none. Only
    // an upload still pending can claim it.
    let pending = named_by_pending_version(&state.pool, user).await?;
    let patch_orphans: Vec<(&PatchKey, i64)> = listed
        .patches
        .iter()
        .filter(|(p, _, at)| {
            now_unix - at >= min_age && !row_patches.contains(p) && !pending.contains(&p.sha)
        })
        .map(|(p, size, _)| (p, *size))
        .collect();
    f.orphan_objects += (orphans.len() + patch_orphans.len()) as u64;
    f.orphan_bytes += orphans.iter().map(|(_, s)| s).sum::<i64>()
        + patch_orphans.iter().map(|(_, s)| s).sum::<i64>();

    if !delete
        || (orphans.is_empty() && patch_orphans.is_empty())
        || !quiet(&state.pool, user).await?
    {
        return Ok(());
    }
    let mut budget = MAX_DELETES_PER_ACCOUNT;
    let patch_orphans: Vec<(PatchKey, i64)> = patch_orphans
        .into_iter()
        .take(budget)
        .map(|(p, size)| (p.clone(), size))
        .collect();
    budget -= patch_orphans.len();
    for batch in patch_orphans.chunks(BATCH) {
        let keys: Vec<PatchKey> = batch.iter().map(|(p, _)| p.clone()).collect();
        let still = still_patch_orphans(&state.pool, user, &keys).await?;
        let doomed: Vec<(PatchKey, i64)> = batch
            .iter()
            .filter(|(p, _)| still.contains(p))
            .cloned()
            .collect();
        let results: Vec<(i64, bool)> = futures::stream::iter(doomed.into_iter().map(|(p, size)| {
            let r2 = state.r2.clone();
            async move {
                let key = p.key(user);
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
    let orphans: Vec<(String, i64)> = orphans
        .into_iter()
        .take(budget)
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

/// Every row's sha, and for a patch the object it lives in. `live` keeps only
/// the referenced ones.
async fn rows_of(
    pool: &sqlx::PgPool,
    user: Uuid,
    live: bool,
) -> Result<Vec<(String, Option<PatchKey>)>, sqlx::Error> {
    let rows: Vec<(String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT encode(sha256, 'hex'), encode(base_sha256, 'hex'), encoding
           FROM cloud_blobs WHERE user_id = $1 AND (refcount > 0 OR NOT $2)",
    )
    .bind(user)
    .bind(live)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(sha, base, encoding)| {
            let patch = base.map(|base| PatchKey {
                sha: sha.clone(),
                base,
                inflated: encoding.as_deref() == Some(crate::patch::INFLATED),
            });
            (sha, patch)
        })
        .collect())
}

/// Shas an upload still pending names.
async fn named_by_pending_version(
    pool: &sqlx::PgPool,
    user: Uuid,
) -> Result<HashSet<String>, sqlx::Error> {
    Ok(sqlx::query_scalar(
        "SELECT DISTINCT encode(fe.sha256, 'hex')
           FROM saves s
           JOIN save_versions v ON v.save_id = s.id AND v.sha256 = ''
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

/// Which of `keys` still have no row of their own and no pending upload
/// naming their sha.
async fn still_patch_orphans(
    pool: &sqlx::PgPool,
    user: Uuid,
    keys: &[PatchKey],
) -> Result<HashSet<PatchKey>, sqlx::Error> {
    let shas: Vec<&str> = keys.iter().map(|k| k.sha.as_str()).collect();
    let bases: Vec<&str> = keys.iter().map(|k| k.base.as_str()).collect();
    let inflated: Vec<bool> = keys.iter().map(|k| k.inflated).collect();
    let rows: Vec<(String, String, bool)> = sqlx::query_as(
        "SELECT t.s, t.b, t.i FROM unnest($2::text[], $3::text[], $4::bool[]) AS t(s, b, i)
          WHERE NOT EXISTS (SELECT 1 FROM cloud_blobs c
                             WHERE c.user_id = $1 AND c.sha256 = decode(t.s, 'hex')
                               AND c.base_sha256 = decode(t.b, 'hex')
                               AND (c.encoding = $5) = t.i)
            AND NOT EXISTS (SELECT 1 FROM file_entries fe
                              JOIN saves s ON s.id = fe.save_id
                              JOIN version_files vf ON vf.entry_id = fe.id
                              JOIN save_versions v ON v.id = vf.version_id AND v.sha256 = ''
                             WHERE s.user_id = $1 AND fe.sha256 = decode(t.s, 'hex'))",
    )
    .bind(user)
    .bind(&shas)
    .bind(&bases)
    .bind(&inflated)
    .bind(crate::patch::INFLATED)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(sha, base, inflated)| PatchKey {
            sha,
            base,
            inflated,
        })
        .collect())
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
