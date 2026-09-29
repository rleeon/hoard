//! Read-only audit of blob accounting (cloud only).
//!
//! A blob's `refcount` is supposed to equal the number of committed versions of
//! live (not archived) saves that reference it, and `profiles.storage_bytes` is
//! supposed to equal the size of every blob with `refcount > 0`. Nothing checks
//! either, and both have drifted before without anyone noticing: in
//! September 2026 a save delete cut short by the client's 60 s timeout left ten
//! Free accounts charged for up to 976 MB of files nothing referenced, and the
//! first sign was a user complaining on Discord twelve days later.
//!
//! This task recomputes both from the manifest every few hours and reports what
//! disagrees. It never writes: a repair is a decision a person makes after
//! looking at the numbers, and a background job that "fixed" refcounts from a
//! query with a bug in it would delete people's saves.
//!
//! What it reports, each as an incident plus a log line with the accounts:
//!
//! * **Charged for nothing**: `refcount > 0` but no committed version of a live
//!   save references the blob. The user pays for it and can't delete it.
//! * **Under-counted**: fewer references than committed versions. The blob can
//!   reach 0 while something still points at it, and the GC then deletes a
//!   file a live version needs. This is the one that loses data.
//! * **Over-counted**: more references than versions, while still referenced.
//!   Harmless today, a "charged for nothing" once the versions go.
//! * **Stuck**: `refcount = 0` with no `purge_after`, so no sweep will ever
//!   collect it. What an interrupted release leaves behind.
//! * **Quota drift**: `storage_bytes` differs from what the blobs add up to.
//!   The trigger keeps them in step, so any drift means someone wrote around it.

use super::incidents::{self, Kind};
use crate::cloud::state::CloudState;
use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;

/// Daily. The audit reads every manifest row, and on this machine (32 MB of
/// shared buffers) the heaviest account alone costs ~10 s of disk reads; the
/// point is to hear about drift within a day instead of twelve.
const INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// Off the startup path, like the other sweepers.
const STARTUP_GRACE: Duration = Duration::from_secs(10 * 60);

/// Breathing room between accounts on the full sweep, so the audit never holds
/// the disk for more than one account at a time.
const PAUSE: Duration = Duration::from_millis(250);

/// Accounts named per finding in the log. The counts carry the size; this is
/// only so whoever reads the line can start looking.
const SAMPLE: usize = 20;

/// What one audit found. All zero is a clean bill.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Report {
    pub charged_for_nothing_blobs: i64,
    pub charged_for_nothing_bytes: i64,
    pub charged_for_nothing_accounts: Vec<Uuid>,
    pub under_counted_blobs: i64,
    pub under_counted_accounts: Vec<Uuid>,
    pub over_counted_blobs: i64,
    pub stuck_blobs: i64,
    pub drift_accounts: Vec<Uuid>,
    pub drift_bytes: i64,
}

impl Report {
    pub fn is_clean(&self) -> bool {
        self.charged_for_nothing_blobs == 0
            && self.under_counted_blobs == 0
            && self.over_counted_blobs == 0
            && self.stuck_blobs == 0
            && self.drift_accounts.is_empty()
    }
}

pub fn spawn(state: CloudState) {
    tokio::spawn(async move {
        tokio::time::sleep(STARTUP_GRACE).await;
        let mut tick = tokio::time::interval(INTERVAL);
        loop {
            tick.tick().await;
            match audit(&state.pool, None).await {
                Ok(r) => report(&r),
                Err(e) => tracing::warn!(error = %e, "integrity: audit failed"),
            }
        }
    });
}

/// Run the audit, one account at a time. `only` narrows it to some accounts
/// (a support question, or a test sharing its database with others); `None`
/// is everyone, with a pause between accounts.
///
/// One account per query rather than one query over everything: the global
/// version took 56 s in production, most of it spilling a 1.2 M-row aggregate
/// to disk, while per account every join rides an index and the worst case is
/// the ~10 s account above.
pub async fn audit(pool: &PgPool, only: Option<&[Uuid]>) -> Result<Report, sqlx::Error> {
    let users: Vec<Uuid> = match only {
        Some(u) => u.to_vec(),
        None => {
            sqlx::query_scalar(
                "SELECT user_id FROM cloud_blobs
             UNION SELECT user_id FROM saves
             UNION SELECT user_id FROM profiles WHERE storage_bytes <> 0",
            )
            .fetch_all(pool)
            .await?
        }
    };
    let mut report = Report::default();
    for (i, user) in users.iter().enumerate() {
        if only.is_none() && i > 0 {
            tokio::time::sleep(PAUSE).await;
        }
        let one = audit_account(pool, *user).await?;
        report.absorb(*user, one);
    }
    Ok(report)
}

/// One account's findings, before they are folded into a [`Report`].
#[derive(Debug, Default)]
struct Account {
    charged_for_nothing_blobs: i64,
    charged_for_nothing_bytes: i64,
    under_counted_blobs: i64,
    over_counted_blobs: i64,
    stuck_blobs: i64,
    drift_bytes: i64,
}

impl Report {
    fn absorb(&mut self, user: Uuid, a: Account) {
        let name = |list: &mut Vec<Uuid>| {
            if list.len() < SAMPLE {
                list.push(user);
            }
        };
        if a.charged_for_nothing_blobs > 0 {
            self.charged_for_nothing_blobs += a.charged_for_nothing_blobs;
            self.charged_for_nothing_bytes += a.charged_for_nothing_bytes;
            name(&mut self.charged_for_nothing_accounts);
        }
        if a.under_counted_blobs > 0 {
            self.under_counted_blobs += a.under_counted_blobs;
            name(&mut self.under_counted_accounts);
        }
        if a.drift_bytes != 0 {
            self.drift_bytes += a.drift_bytes.abs();
            name(&mut self.drift_accounts);
        }
        self.over_counted_blobs += a.over_counted_blobs;
        self.stuck_blobs += a.stuck_blobs;
    }
}

/// One read-only, repeatable-read transaction, so the blob table and the
/// manifest are compared as of the same instant: an upload committing halfway
/// through can't show up as a mismatch.
async fn audit_account(pool: &PgPool, user: Uuid) -> Result<Account, sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SET LOCAL statement_timeout = '60s'")
        .execute(&mut *tx)
        .await?;

    // `expected` is the rule `cas_commit` follows: +1 per committed version.
    // Archived saves are left out because archiving hands their references
    // back. The FULL JOIN catches a referenced blob that has no row at all,
    // which counts as under-counted (0 < n).
    let counts: (i64, i64, i64, i64, i64) = sqlx::query_as(
        r#"
        WITH expected AS (
            SELECT fe.sha256, count(DISTINCT v.id) AS n
              FROM saves s
              JOIN save_versions v ON v.save_id = s.id
              JOIN version_files vf ON vf.version_id = v.id
              JOIN file_entries fe ON fe.id = vf.entry_id
             WHERE s.user_id = $1 AND s.archived_at IS NULL
               AND v.sha256 <> '' AND v.content_addressed
             GROUP BY 1
        ),
        cmp AS (
            SELECT coalesce(b.size_bytes, 0) AS size_bytes,
                   coalesce(b.refcount, 0) AS refcount,
                   b.purge_after,
                   b.sha256 IS NULL AS no_row,
                   coalesce(e.n, 0) AS n
              FROM (SELECT * FROM cloud_blobs WHERE user_id = $1) b
              FULL JOIN expected e ON e.sha256 = b.sha256
        )
        SELECT count(*) FILTER (WHERE refcount > 0 AND n = 0),
               coalesce(sum(size_bytes) FILTER (WHERE refcount > 0 AND n = 0), 0)::bigint,
               count(*) FILTER (WHERE refcount < n),
               count(*) FILTER (WHERE n > 0 AND refcount > n),
               count(*) FILTER (WHERE NOT no_row AND refcount = 0 AND purge_after IS NULL)
          FROM cmp
        "#,
    )
    .bind(user)
    .fetch_one(&mut *tx)
    .await?;

    // Legacy (non content-addressed) versions charge through their own row, so
    // they belong in the sum even though production has none left.
    let drift: Option<i64> = sqlx::query_scalar(
        r#"
        SELECT (p.storage_bytes
               - coalesce((SELECT sum(size_bytes) FROM cloud_blobs
                            WHERE user_id = $1 AND refcount > 0), 0)
               - coalesce((SELECT sum(v.size_bytes) FROM save_versions v
                             JOIN saves s ON s.id = v.save_id
                            WHERE s.user_id = $1 AND NOT v.content_addressed
                              AND v.deleted_at IS NULL), 0))::bigint
          FROM profiles p WHERE p.user_id = $1
        "#,
    )
    .bind(user)
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Account {
        charged_for_nothing_blobs: counts.0,
        charged_for_nothing_bytes: counts.1,
        under_counted_blobs: counts.2,
        over_counted_blobs: counts.3,
        stuck_blobs: counts.4,
        drift_bytes: drift.unwrap_or(0),
    })
}

/// Log what the audit found and raise one incident per kind of problem, so the
/// status embed's "Other" count and its "Last:" line point here.
fn report(r: &Report) {
    if r.is_clean() {
        tracing::info!("integrity: blob accounting is consistent");
        return;
    }
    if r.under_counted_blobs > 0 {
        tracing::error!(
            blobs = r.under_counted_blobs,
            accounts = ?r.under_counted_accounts,
            "integrity: blobs with fewer references than versions (the GC could delete live files)"
        );
        incidents::record(Kind::Other, "integrity: under-counted blobs");
    }
    if r.charged_for_nothing_blobs > 0 {
        tracing::warn!(
            blobs = r.charged_for_nothing_blobs,
            bytes = r.charged_for_nothing_bytes,
            accounts = ?r.charged_for_nothing_accounts,
            "integrity: accounts charged for blobs nothing references"
        );
        incidents::record(
            Kind::Other,
            "integrity: quota charged for unreferenced blobs",
        );
    }
    if !r.drift_accounts.is_empty() {
        tracing::warn!(
            bytes = r.drift_bytes,
            accounts = ?r.drift_accounts,
            "integrity: storage_bytes disagrees with the blobs"
        );
        incidents::record(Kind::Other, "integrity: quota drift");
    }
    if r.stuck_blobs > 0 {
        tracing::warn!(
            blobs = r.stuck_blobs,
            "integrity: unreferenced rows no sweep will collect"
        );
        incidents::record(Kind::Other, "integrity: stuck blob rows");
    }
    if r.over_counted_blobs > 0 {
        tracing::warn!(
            blobs = r.over_counted_blobs,
            "integrity: blobs with more references than versions"
        );
        incidents::record(Kind::Other, "integrity: over-counted blobs");
    }
}
