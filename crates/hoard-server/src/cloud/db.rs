//! Postgres pool + migrations for cloud mode.
//!
//! The pool is shared via `CloudState`. Migrations live under
//! `migrations/postgres/`.

use anyhow::Result;
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    PgPool,
};
use std::str::FromStr;
use std::time::Duration;
use tracing::info;

pub async fn connect(url: &str, max_connections: u32) -> Result<PgPool> {
    let opts = PgConnectOptions::from_str(url)?;

    let pool = PgPoolOptions::new()
        .max_connections(max_connections.max(2))
        .acquire_timeout(Duration::from_secs(10))
        .connect_with(opts)
        .await?;

    Ok(pool)
}

pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    info!("running postgres migrations");
    sqlx::migrate!("./migrations/postgres").run(pool).await?;
    info!("postgres migrations complete");
    Ok(())
}
