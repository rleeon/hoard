use anyhow::Result;
use clap::Subcommand;
use hoard_server::{config::Config, db};
#[allow(unused_imports)]
use sqlx::Row;

#[derive(Subcommand)]
pub enum TokenCommand {
    /// Create a new API token for a user
    Create {
        /// Username
        username: String,
        /// Device label (e.g. "PC-Sobremesa")
        #[arg(long)]
        device: Option<String>,
    },
    /// List tokens for a user
    List {
        /// Username
        username: String,
    },
    /// Revoke a token by the prefix `token list` prints for it.
    ///
    /// The prefix is the start of the token's hash, not of the token itself:
    /// the plaintext is never stored, so there is nothing else to match on.
    Revoke {
        /// Prefix from the `Prefix` column of `token list`.
        token_prefix: String,
    },
}

pub async fn run(cmd: TokenCommand, cfg: &Config) -> Result<()> {
    let pool = db::connect(&cfg.database.url, cfg.database.max_connections).await?;
    db::run_migrations(&pool).await?;

    match cmd {
        TokenCommand::Create { username, device } => {
            use hoard_core::hashing::{generate_token, hash_token};
            use time::OffsetDateTime;

            let row = sqlx::query("SELECT id FROM users WHERE username = ?")
                .bind(&username)
                .fetch_optional(&pool)
                .await?;
            let Some(row) = row else {
                anyhow::bail!("User '{}' not found", username);
            };
            let user_id: String = row.get("id");

            let token = generate_token();
            let token_hash = hash_token(&token);
            let id = uuid::Uuid::new_v4().to_string();

            let expires_at = if cfg.auth.token_lifetime_days > 0 {
                let exp = OffsetDateTime::now_utc()
                    + time::Duration::days(cfg.auth.token_lifetime_days as i64);
                Some(exp.format(&time::format_description::well_known::Rfc3339)?)
            } else {
                None
            };

            sqlx::query(
                "INSERT INTO api_tokens (id, user_id, token_hash, device_name, expires_at)
                 VALUES (?,?,?,?,?)",
            )
            .bind(&id)
            .bind(&user_id)
            .bind(&token_hash)
            .bind(&device)
            .bind(&expires_at)
            .execute(&pool)
            .await?;

            println!("\n⚠  Save this token NOW; it cannot be recovered later!\n");
            println!("Token: {}", token);
            // Two lines, and the real key name. This hint used to print
            // `hoard config set server <url> && hoard login --token …`: the key
            // is `server.url`, so the first half errored out, and `&&` is not a
            // separator in Windows PowerShell, so the second half never ran at
            // all. It is the first thing a new self-hoster copies.
            println!("\nSet it in your CLI:");
            println!("  hoard config set server.url <url>");
            println!("  hoard login --token {}", token);
        }
        TokenCommand::List { username } => {
            let rows = sqlx::query(
                "SELECT t.id, t.token_hash, t.device_name, t.last_used_at, t.revoked_at, t.created_at
                 FROM api_tokens t
                 JOIN users u ON u.id = t.user_id
                 WHERE u.username = ?
                 ORDER BY t.created_at DESC",
            )
            .bind(&username)
            .fetch_all(&pool)
            .await?;

            if rows.is_empty() {
                println!("No tokens for '{}'.", username);
                return Ok(());
            }

            println!(
                "{:<12} {:<20} {:<26} {:<26} Status",
                "Prefix", "Device", "Last used", "Created"
            );
            for row in rows {
                let hash: String = row.get("token_hash");
                let prefix = &hash[..16.min(hash.len())];
                let device: Option<String> = row.get("device_name");
                let last_used: Option<String> = row.get("last_used_at");
                let revoked: Option<String> = row.get("revoked_at");
                let created: String = row.get("created_at");
                let status = if revoked.is_some() {
                    "REVOKED"
                } else {
                    "active"
                };
                println!(
                    "{:<12} {:<20} {:<26} {:<26} {}",
                    prefix,
                    device.unwrap_or_else(|| "-".into()),
                    last_used.unwrap_or_else(|| "never".into()),
                    created,
                    status
                );
            }
        }
        TokenCommand::Revoke { token_prefix } => {
            let rows = sqlx::query(
                "UPDATE api_tokens SET revoked_at = strftime('%Y-%m-%dT%H:%M:%SZ','now')
                 WHERE token_hash LIKE ? AND revoked_at IS NULL",
            )
            .bind(format!("{}%", token_prefix))
            .execute(&pool)
            .await?;

            if rows.rows_affected() == 0 {
                // The likeliest mistake is pasting the token instead of the
                // prefix `token list` prints, so say which one is wanted.
                anyhow::bail!(
                    "No active token found with prefix '{}'. Use the Prefix column \
                     from `hoard-admin token list <user>`; it is the start of the \
                     token's hash, not of the token itself.",
                    token_prefix
                );
            }
            println!("Token(s) with prefix '{}' revoked.", token_prefix);
        }
    }
    Ok(())
}
