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
    /// Set a user's password.
    ///
    /// Until the web panel existed this had no reason to be here: the password
    /// was written once at `create` and never read again, because the API
    /// authenticates with tokens. Now it is what someone types into `/panel`,
    /// so it needs a way to change and, more to the point, a way to be reset by the
    /// operator when it is forgotten.
    Passwd {
        /// Username
        username: String,
        /// Password (non-interactive; if omitted, prompts securely)
        #[arg(long)]
        password: Option<String>,
    },
    /// Grant admin rights.
    Promote {
        /// Username
        username: String,
    },
    /// Take admin rights away.
    ///
    /// Refuses on the last remaining admin: `is_admin` guards the routes that
    /// hand it back out, so a server with zero admins can only be repaired
    /// from a shell on the box.
    Demote {
        /// Username
        username: String,
    },
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
        UserCommand::Passwd {
            username,
            password: pw_flag,
        } => {
            use hoard_core::hashing::hash_password;

            let row = sqlx::query("SELECT id FROM users WHERE username = ?")
                .bind(&username)
                .fetch_optional(&pool)
                .await?;
            let Some(row) = row else {
                anyhow::bail!("User '{}' not found", username);
            };
            let user_id: String = row.get("id");

            let password = match pw_flag {
                Some(p) => p,
                None => {
                    let p1 = rpassword::prompt_password("New password: ")?;
                    let p2 = rpassword::prompt_password("Confirm:      ")?;
                    if p1 != p2 {
                        anyhow::bail!("Passwords do not match");
                    }
                    p1
                }
            };
            if password.len() < 8 {
                anyhow::bail!("Password must be at least 8 characters");
            }

            let hash = hash_password(&password)?;
            sqlx::query(
                "UPDATE users SET password_hash = ?, \
                 updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE id = ?",
            )
            .bind(&hash)
            .bind(&user_id)
            .execute(&pool)
            .await?;

            // Browser sessions are ordinary token rows tagged with this device
            // name (see `hoard_server::routes::session`). A password reset that
            // left them alive would be a reset that changes nothing for whoever
            // is already logged in.
            let closed = sqlx::query(
                "UPDATE api_tokens SET revoked_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') \
                 WHERE user_id = ? AND device_name = 'web panel' AND revoked_at IS NULL",
            )
            .bind(&user_id)
            .execute(&pool)
            .await?
            .rows_affected();

            println!("Password updated for '{}'", username);
            if closed > 0 {
                println!("Closed {closed} browser session(s). Device tokens were left alone.");
            }
        }
        UserCommand::Promote { username } => {
            let affected = sqlx::query("UPDATE users SET is_admin = 1 WHERE username = ?")
                .bind(&username)
                .execute(&pool)
                .await?
                .rows_affected();
            if affected == 0 {
                anyhow::bail!("User '{}' not found", username);
            }
            println!("'{}' is now an admin", username);
        }
        UserCommand::Demote { username } => {
            let row = sqlx::query("SELECT id, is_admin FROM users WHERE username = ?")
                .bind(&username)
                .fetch_optional(&pool)
                .await?;
            let Some(row) = row else {
                anyhow::bail!("User '{}' not found", username);
            };
            let is_admin: i64 = row.get("is_admin");
            if is_admin == 0 {
                println!("'{}' is not an admin", username);
                return Ok(());
            }

            let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE is_admin <> 0")
                .fetch_one(&pool)
                .await?;
            if count <= 1 {
                anyhow::bail!(
                    "'{}' is the last admin. Promote someone else first, or the server \
                     will have nobody who can grant admin back.",
                    username
                );
            }

            sqlx::query("UPDATE users SET is_admin = 0 WHERE username = ?")
                .bind(&username)
                .execute(&pool)
                .await?;
            println!("'{}' is no longer an admin", username);
        }
        UserCommand::Delete { username } => {
            let row = sqlx::query("SELECT id, is_admin FROM users WHERE username = ?")
                .bind(&username)
                .fetch_optional(&pool)
                .await?;

            let Some(row) = row else {
                anyhow::bail!("User '{}' not found", username);
            };

            let user_id: String = row.get("id");
            let is_admin: i64 = row.get("is_admin");

            // The admin flag guards its own route, so a server with zero
            // admins cannot promote anyone back from the panel: it needs this
            // command and a shell. Deleting the last one is how you lock
            // yourself out of your own server, and nothing used to stop it.
            if is_admin != 0 {
                let (admins,): (i64,) =
                    sqlx::query_as("SELECT COUNT(*) FROM users WHERE is_admin <> 0")
                        .fetch_one(&pool)
                        .await?;
                if admins <= 1 {
                    anyhow::bail!(
                        "'{}' is the only admin. Promote someone else first \
                         (`hoard-admin user promote <name>`).",
                        username
                    );
                }
            }

            print!("Delete user '{}' and ALL their data? [y/N] ", username);
            use std::io::BufRead;
            let mut line = String::new();
            std::io::stdin().lock().read_line(&mut line)?;
            if line.trim().to_lowercase() != "y" {
                println!("Aborted.");
                return Ok(());
            }

            // Stored objects first: the ON DELETE CASCADE below takes the
            // blobs/chunks rows, which are the only record of which keys were
            // theirs. This used to be a `remove_dir_all` of
            // `data_dir/<user_id>`, a path nothing has written to since the
            // content-addressed store landed, so the command reported success
            // while leaving every byte on disk.
            let store = hoard_server::store::build_store(cfg).await?;
            let (objects, bytes) =
                hoard_server::store::purge_user_objects(&pool, &store, &user_id).await?;

            sqlx::query("DELETE FROM users WHERE id = ?")
                .bind(&user_id)
                .execute(&pool)
                .await?;

            println!(
                "Deleted user '{}' and their data ({} objects, {:.1} MiB).",
                username,
                objects,
                bytes as f64 / (1024.0 * 1024.0)
            );
        }
    }
    Ok(())
}
