//! Postgres pool + migrations for cloud mode.
//!
//! The pool is shared via `CloudState`. Migrations live under
//! `migrations/postgres/`.

use anyhow::{Context, Result};
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    PgPool,
};
use std::str::FromStr;
use std::time::Duration;
use tracing::{info, warn};

pub async fn connect(url: &str, max_connections: u32) -> Result<PgPool> {
    let opts = PgConnectOptions::from_str(url)?;

    let pool = PgPoolOptions::new()
        .max_connections(max_connections.max(2))
        // Boot runs migrations before we bind the port, and migrations need a
        // connection of their own. Ten seconds was enough for a warm pooler and
        // nothing else: when Supabase's pooler was slow to hand out the second
        // connection the acquire timed out, `run` returned an error, the
        // process exited 1 and Fly restarted it, forever, on a ~12s cycle,
        // because every restart hit the same cold pooler. Thirty seconds is
        // still a bounded failure, just not one that a transient stall trips.
        .acquire_timeout(Duration::from_secs(30))
        .connect_with(opts)
        .await?;

    Ok(pool)
}

pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    info!("running postgres migrations");

    let mut migrator = sqlx::migrate!("./migrations/postgres");
    // Don't refuse to boot because the database is *ahead* of this binary.
    // Rolling back to the previous image is the standard way out of a bad
    // deploy, and the previous image has never heard of the migrations the bad
    // one applied. Without this, the rollback boots into `VersionMissing` and
    // the outage you were undoing gets worse instead of better. Checksums of
    // the migrations this binary *does* know are still verified.
    migrator.set_ignore_missing(true);

    // A migration failure used to be invisible: the error went up to `main`,
    // the process exited 1 and the only trace left was a machine restarting on
    // a loop with no explanation. Log it here, where we still know it was the
    // migration step that failed.
    let mut attempt = 0;
    loop {
        attempt += 1;
        match migrator.run(pool).await {
            Ok(()) => break,
            Err(e) if attempt < 3 => {
                warn!(error = %e, attempt, "postgres migrations failed; retrying");
                tokio::time::sleep(Duration::from_secs(2 * attempt)).await;
            }
            Err(e) => {
                return Err(e).context("postgres migrations failed after 3 attempts");
            }
        }
    }

    info!("postgres migrations complete");
    Ok(())
}
