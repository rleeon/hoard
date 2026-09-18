//! Forget machines nobody has used in three months.
//!
//! `0003_devices.sql` said a daily cron did this. No such cron ever existed:
//! not in the code, and no `pg_cron` in the database it was written for. So on
//! Free a dead laptop held one of three slots for good, and signing out did not
//! give it back either. Harmless while nothing enforced the allowance, and the
//! reason this had to land before anything did.
//!
//! Self-hosted has had the same rule for ages (`cleanup.rs`), which is where
//! the 90 days comes from.
//!
//! The three foreign keys pointing at `devices` (`save_versions`, `sync_log`,
//! `client_logs`) are all `ON DELETE SET NULL`, and version history keeps the
//! device *name* in a column of its own, so pruning a row costs no provenance:
//! an old version still says which machine made it.

use super::incidents::{self, Kind};
use crate::cloud::notices;
use crate::cloud::state::CloudState;
use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;

/// How long a device may go unseen before its slot is reclaimed. The agent
/// beats every 30 s while it runs, so this is three months of the machine
/// never once being turned on with Hoard installed.
const DEVICE_RETENTION_DAYS: i64 = 90;

pub fn spawn(state: CloudState) {
    tokio::spawn(async move {
        // Daily, and not on the first tick: a deploy should not open with a
        // sweep, and a machine that idles to zero restarts often enough that
        // "daily" is really "a few minutes after most boots".
        tokio::time::sleep(Duration::from_secs(8 * 60)).await;
        let mut tick = tokio::time::interval(Duration::from_secs(24 * 60 * 60));
        tick.tick().await;
        loop {
            match prune(&state.pool).await {
                Ok(0) => {}
                Ok(n) => tracing::info!(devices = n, "device prune: dropped stale devices"),
                Err(e) => {
                    tracing::warn!(error = %e, "device prune: sweep failed");
                    incidents::record(Kind::Delete, "device prune: sweep failed");
                }
            }
            tick.tick().await;
        }
    });
}

/// Delete every device unseen for [`DEVICE_RETENTION_DAYS`] and repair the
/// cached count of each account that lost one. Returns how many rows went.
pub async fn prune(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let owners: Vec<(Uuid,)> = sqlx::query_as(
        "DELETE FROM devices
          WHERE last_seen_at < now() - make_interval(days => $1)
      RETURNING user_id",
    )
    .bind(DEVICE_RETENTION_DAYS as i32)
    .fetch_all(pool)
    .await?;

    let deleted = owners.len() as u64;
    let mut affected: Vec<Uuid> = owners.into_iter().map(|(u,)| u).collect();
    affected.sort_unstable();
    affected.dedup();

    for user_id in affected {
        sqlx::query(
            "UPDATE profiles
                SET devices_count = (SELECT count(*) FROM devices WHERE user_id = $1)
              WHERE user_id = $1",
        )
        .bind(user_id)
        .execute(pool)
        .await?;
        // A slot just opened without the user doing anything, so the warning
        // about being full should be able to fire again.
        notices::clear(
            pool,
            user_id,
            notices::Kind::DevicesFull,
            notices::ACCOUNT,
        )
        .await?;
    }
    Ok(deleted)
}
