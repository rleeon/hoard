use anyhow::Result;
use clap::Subcommand;
use hoard_server::{config::Config, db};
use sqlx::Row;

#[derive(Subcommand)]
pub enum UserCommand {
    /// Create a new user (prompts for password)
    Create {
        /// Username
        username: String,
        /// Grant admin privileges
        #[arg(long)]
        admin: bool,
        /// Password (non-interactive; if omitted, prompts securely)
        #[arg(long)]
        password: Option<String>,
    },
    /// List all users
    List,
    /// Delete a user and all their data
    Delete {
        /// Username to delete
        username: String,
    },
}

pub async fn run(cmd: UserCommand, cfg: &Config) -> Result<()> {
    let pool = db::connect(&cfg.database.url, cfg.database.max_connections).await?;
    db::run_migrations(&pool).await?;

    match cmd {
        UserCommand::Create {
            username,
            admin,
            password: pw_flag,
        } => {
            use hoard_core::hashing::hash_password;

            let password = if let Some(p) = pw_flag {
                p
            } else {
                let p1 = rpassword::prompt_password("Password: ")?;
                let p2 = rpassword::prompt_password("Confirm:  ")?;
                if p1 != p2 {
                    anyhow::bail!("Passwords do not match");
                }
                p1
            };
            if password.len() < 8 {
                anyhow::bail!("Password must be at least 8 characters");
            }

            let id = uuid::Uuid::new_v4().to_string();
            let hash = hash_password(&password)?;
            sqlx::query(
                "INSERT INTO users (id, username, password_hash, is_admin) VALUES (?,?,?,?)",
            )
            .bind(&id)
            .bind(&username)
            .bind(&hash)
            .bind(admin as i64)
            .execute(&pool)
            .await?;

            println!("Created user '{}' (id: {})", username, id);
        }
        UserCommand::List => {
            let rows = sqlx::query(
                "SELECT id, username, is_admin, storage_used_bytes, created_at FROM users ORDER BY created_at",
            )
            .fetch_all(&pool)
            .await?;

            if rows.is_empty() {
                println!("No users.");
                return Ok(());
            }

            println!(
                "{:<38} {:<20} {:<6} {:<12} Created",
                "ID", "Username", "Admin", "Storage MiB"
            );
            for row in rows {
                let id: String = row.get("id");
                let username: String = row.get("username");
                let is_admin: i64 = row.get("is_admin");
                let storage: i64 = row.get("storage_used_bytes");
                let created_at: String = row.get("created_at");
                println!(
                    "{:<38} {:<20} {:<6} {:<12} {}",
                    id,
                    username,
                    if is_admin != 0 { "yes" } else { "no" },
                    storage / 1024 / 1024,
                    created_at
                );
            }
        }
        UserCommand::Delete { username } => {
            let row = sqlx::query("SELECT id FROM users WHERE username = ?")
                .bind(&username)
                .fetch_optional(&pool)
                .await?;

            let Some(row) = row else {
                anyhow::bail!("User '{}' not found", username);
            };

            let user_id: String = row.get("id");
            print!("Delete user '{}' and ALL their data? [y/N] ", username);
            use std::io::BufRead;
            let mut line = String::new();
            std::io::stdin().lock().read_line(&mut line)?;
            if line.trim().to_lowercase() != "y" {
                println!("Aborted.");
                return Ok(());
            }

            sqlx::query("DELETE FROM users WHERE id = ?")
                .bind(&user_id)
                .execute(&pool)
                .await?;

            // Clean up physical data directory
            let data_path = cfg.storage.data_dir.join(&user_id);
            if data_path.exists() {
                tokio::fs::remove_dir_all(&data_path).await?;
            }

            println!("Deleted user '{}' and their data.", username);
        }
    }
    Ok(())
}
