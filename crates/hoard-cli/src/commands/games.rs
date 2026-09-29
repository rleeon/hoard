use anyhow::Result;
use clap::Subcommand;

use hoard_agent::api::ApiError;
use hoard_agent::{catalog, cloud_auth};

use crate::commands::link;

/// What the self-hosted `/v1/games` hands out when not asked for a size.
const SEARCH_LIMIT: usize = 20;

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
    // Cloud serves no catalogue: it answers from the local copy, which is the
    // same Ludusavi data a self-hosted server loads and needs no network.
    let client = if cloud_auth::load_session()?.is_some() {
        None
    } else {
        Some(link::resolve_session().await?.client)
    };

    match cmd {
        GameCommand::Search { query } => {
            let games = match &client {
                Some(client) => client.list_games(query.as_deref()).await?,
                None => catalog::search(query.as_deref(), SEARCH_LIMIT),
            };
            if games.is_empty() {
                println!("(no games)");
                return Ok(());
            }
            println!("{:<28} {:<32} ENGINE", "SLUG", "NAME");
            for g in games {
                println!(
                    "{:<28} {:<32} {}",
                    // `as_str`: the id's own Display ignores the column width.
                    g.slug.as_str(),
                    g.display_name,
                    g.engine.unwrap_or_default()
                );
            }
        }
        GameCommand::Show { slug } => {
            let g = match &client {
                Some(client) => client.get_game(&slug).await?,
                None => catalog::game(&slug).ok_or_else(|| {
                    anyhow::Error::new(ApiError::NotFound)
                        .context(format!("no game {slug} in the catalogue"))
                })?,
            };
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
