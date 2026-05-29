//! Background cleanup task: purges old tmp uploads and trashed snapshots.

use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tracing::{info, warn};

pub async fn run_periodic(
    pool: SqlitePool,
    data_dir: PathBuf,
    tmp_cleanup_hours: u64,
    trash_retention_days: u64,
) {
    // Run every hour
    let mut interval = tokio::time::interval(Duration::from_secs(3600));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        interval.tick().await;
        if let Err(e) = run_once(&pool, &data_dir, tmp_cleanup_hours, trash_retention_days).await {
            warn!(error = %e, "cleanup task error");
        }
    }
}

pub async fn run_once(
    pool: &SqlitePool,
    data_dir: &Path,
    tmp_cleanup_hours: u64,
    trash_retention_days: u64,
) -> anyhow::Result<()> {
    purge_tmp(data_dir, tmp_cleanup_hours).await?;
    purge_trash(pool, data_dir, trash_retention_days).await?;
    purge_client_logs(pool, CLIENT_LOG_RETENTION_DAYS).await?;
    Ok(())
}

/// Client diagnostic logs are kept for 14 days on both branches.
const CLIENT_LOG_RETENTION_DAYS: i64 = 14;

async fn purge_client_logs(pool: &SqlitePool, retention_days: i64) -> anyhow::Result<()> {
    let cutoff = time::OffsetDateTime::now_utc() - time::Duration::days(retention_days);
    let cutoff_str = cutoff.format(&time::format_description::well_known::Rfc3339)?;
    // Runtime query (not the `query!` macro) so this doesn't depend on the
    // .sqlx offline cache being regenerated.
    let res = sqlx::query("DELETE FROM client_logs WHERE received_at < ?")
        .bind(&cutoff_str)
        .execute(pool)
        .await?;
    let removed = res.rows_affected();
    if removed > 0 {
        info!(removed, "purged expired client logs");
    }
    Ok(())
}

async fn purge_tmp(data_dir: &Path, max_age_hours: u64) -> anyhow::Result<()> {
    let tmp_dir = data_dir.join("tmp");
    if !tmp_dir.exists() {
        return Ok(());
    }
    let max_age = Duration::from_secs(max_age_hours * 3600);
    let now = SystemTime::now();

    let mut entries = tokio::fs::read_dir(&tmp_dir).await?;
    let mut removed = 0;
    while let Some(entry) = entries.next_entry().await? {
        let meta = entry.metadata().await?;
        let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        if let Ok(age) = now.duration_since(mtime) {
            if age > max_age {
                let _ = tokio::fs::remove_dir_all(entry.path()).await;
                removed += 1;
            }
        }
    }
    if removed > 0 {
        info!(removed, "purged stale tmp uploads");
    }
    Ok(())
}

async fn purge_trash(
    pool: &SqlitePool,
    data_dir: &Path,
    retention_days: u64,
) -> anyhow::Result<()> {
    let cutoff = time::OffsetDateTime::now_utc() - time::Duration::days(retention_days as i64);
    let cutoff_str = cutoff.format(&time::format_description::well_known::Rfc3339)?;

    let rows = sqlx::query!(
        "SELECT id FROM snapshots WHERE deleted_at IS NOT NULL AND deleted_at < ?",
        cutoff_str
    )
    .fetch_all(pool)
    .await?;

    let mut removed = 0;
    for row in rows {
        let trash_path = data_dir.join("trash").join(&row.id);
        let _ = tokio::fs::remove_dir_all(&trash_path).await;
        sqlx::query!("DELETE FROM snapshots WHERE id=?", row.id)
            .execute(pool)
            .await?;
        removed += 1;
    }
    if removed > 0 {
        info!(removed, "purged trashed snapshots");
    }
    Ok(())
}
