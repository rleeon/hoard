use anyhow::Result;
use clap::Subcommand;
use hoard_server::{config::Config, db};
#[allow(unused_imports)]
use sqlx::Row;

#[derive(Subcommand)]
pub enum GameCommand {
    /// Add a game to the catalog
    Add {
        /// Kebab-case slug (e.g. stardew-valley)
        slug: String,
        /// Display name
        #[arg(long)]
        name: String,
        /// Game engine (optional)
        #[arg(long)]
        engine: Option<String>,
    },
    /// List games in the catalog
    List {
        #[arg(long)]
        search: Option<String>,
    },
    /// Remove a game from the catalog
    Remove {
        slug: String,
        /// Force removal even if saves exist
        #[arg(long)]
        force: bool,
    },
}

static SLUG_RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();

fn slug_re() -> &'static regex::Regex {
    SLUG_RE.get_or_init(|| regex::Regex::new(r"^[a-z0-9][a-z0-9-]*[a-z0-9]$").unwrap())
}

pub async fn run(cmd: GameCommand, cfg: &Config) -> Result<()> {
    let pool = db::connect(&cfg.database.url, cfg.database.max_connections).await?;
    db::run_migrations(&pool).await?;

    match cmd {
        GameCommand::Add { slug, name, engine } => {
            if !slug_re().is_match(&slug) {
                anyhow::bail!(
                    "Invalid slug '{}'. Must match ^[a-z0-9][a-z0-9-]*[a-z0-9]$",
                    slug
                );
            }
            sqlx::query("INSERT INTO games (slug, display_name, engine) VALUES (?,?,?)")
                .bind(&slug)
                .bind(&name)
                .bind(&engine)
                .execute(&pool)
                .await?;
            println!("Added game '{}' ({}).", name, slug);
        }
        GameCommand::List { search } => {
            let rows = if let Some(q) = search {
                let pattern = format!("%{}%", q);
                sqlx::query(
                    "SELECT slug, display_name, engine FROM games
                     WHERE slug LIKE ? OR display_name LIKE ?
                     ORDER BY slug",
                )
                .bind(&pattern)
                .bind(&pattern)
                .fetch_all(&pool)
                .await?
            } else {
                sqlx::query("SELECT slug, display_name, engine FROM games ORDER BY slug")
                    .fetch_all(&pool)
                    .await?
            };

            if rows.is_empty() {
                println!("No games.");
                return Ok(());
            }
            println!("{:<30} {:<30} Engine", "Slug", "Display Name");
            for row in rows {
                let slug: String = row.get("slug");
                let name: String = row.get("display_name");
                let engine: Option<String> = row.get("engine");
                println!(
                    "{:<30} {:<30} {}",
                    slug,
                    name,
                    engine.unwrap_or_else(|| "-".into())
                );
            }
        }
        GameCommand::Remove { slug, force } => {
            if !force {
                let saves: i64 =
                    sqlx::query_scalar("SELECT COUNT(*) FROM saves WHERE game_slug = ?")
                        .bind(&slug)
                        .fetch_one(&pool)
                        .await?;
                if saves > 0 {
                    anyhow::bail!(
                        "Game '{}' has {} save(s). Use --force to remove anyway.",
                        slug,
                        saves
                    );
                }
            }
            sqlx::query("DELETE FROM games WHERE slug = ?")
                .bind(&slug)
                .execute(&pool)
                .await?;
            println!("Removed game '{}'.", slug);
        }
    }
    Ok(())
}
