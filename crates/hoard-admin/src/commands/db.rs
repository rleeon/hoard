use anyhow::Result;
use clap::Subcommand;
use hoard_server::{config::Config, db};
#[allow(unused_imports)]
use sqlx::Row;

#[derive(Subcommand)]
pub enum DbCommand {
    /// Show migration version, record counts, and disk usage
    Status,
    /// Apply pending migrations
    Migrate,
    /// Run VACUUM to reclaim disk space
    Vacuum,
}

pub async fn run(cmd: DbCommand, cfg: &Config) -> Result<()> {
    let pool = db::connect(&cfg.database.url, cfg.database.max_connections).await?;

    match cmd {
        DbCommand::Status => {
            // Applied migration version
            let version: Option<i64> = sqlx::query_scalar(
                "SELECT MAX(version) FROM _sqlx_migrations WHERE success = TRUE",
            )
            .fetch_optional(&pool)
            .await
            .unwrap_or(None);

            let users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
                .fetch_one(&pool)
                .await?;
            let snapshots: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM snapshots")
                .fetch_one(&pool)
                .await?;
            let storage: i64 =
                sqlx::query_scalar("SELECT COALESCE(SUM(storage_used_bytes),0) FROM users")
                    .fetch_one(&pool)
                    .await?;

            // SQLite page size * page count ≈ file size
            let page_size: i64 = sqlx::query_scalar("PRAGMA page_size")
                .fetch_one(&pool)
                .await?;
            let page_count: i64 = sqlx::query_scalar("PRAGMA page_count")
                .fetch_one(&pool)
                .await?;
            let db_bytes = page_size * page_count;

            println!("DB migration version : {}", version.unwrap_or(0));
            println!("DB file size         : {} KiB", db_bytes / 1024);
            println!("Users                : {}", users);
            println!("Snapshots            : {}", snapshots);
            println!("Total storage used   : {} MiB", storage / 1024 / 1024);
        }
        DbCommand::Migrate => {
            db::run_migrations(&pool).await?;
            println!("Migrations applied.");
        }
        DbCommand::Vacuum => {
            sqlx::query("VACUUM").execute(&pool).await?;
            println!("VACUUM complete.");
        }
    }

    Ok(())
}
