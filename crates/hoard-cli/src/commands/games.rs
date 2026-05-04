use anyhow::Result;
use clap::Subcommand;

use hoard_agent::api::ApiClient;
use hoard_agent::config::CliConfig;

#[derive(Subcommand)]
pub enum GameCommand {
    /// List or search games in the catalog
    Search {
        /// Optional substring (display name or slug)
        query: Option<String>,
    },
    /// Show details for a single game
    Show { slug: String },
}

pub async fn run(cmd: GameCommand) -> Result<()> {
    let (cfg, _) = CliConfig::load_default()?;
    let token = cfg.require_token()?;
    let client = ApiClient::new(cfg.server.url.clone(), token)?;

    match cmd {
        GameCommand::Search { query } => {
            let games = client.list_games(query.as_deref()).await?;
            if games.is_empty() {
                println!("(no games)");
                return Ok(());
            }
            println!("{:<28} {:<32} ENGINE", "SLUG", "NAME");
            for g in games {
                println!(
                    "{:<28} {:<32} {}",
                    g.slug,
                    g.display_name,
                    g.engine.unwrap_or_default()
                );
            }
        }
        GameCommand::Show { slug } => {
            let g = client.get_game(&slug).await?;
            println!("slug:    {}", g.slug);
            println!("name:    {}", g.display_name);
            if let Some(e) = g.engine {
                println!("engine:  {}", e);
            }
            if let Some(p) = g.save_paths_json {
                println!("paths:   {}", p);
            }
        }
    }
    Ok(())
}
