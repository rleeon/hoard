use anyhow::Result;
use clap::Subcommand;

use hoard_agent::api::ApiClient;
use hoard_agent::config::CliConfig;

#[derive(Subcommand)]
pub enum SaveCommand {
    /// Create a new save namespace for a game
    Create {
        /// Game slug (use `hoard games search` to find one)
        #[arg(long)]
        game: String,
        /// Label for this save (e.g. "main", "speedrun"). Defaults to "default".
        #[arg(long, default_value = "default")]
        label: String,
    },
    /// List your saves
    List {
        /// Filter by game slug
        #[arg(long)]
        game: Option<String>,
    },
    /// Show details for a save
    Show { id: String },
    /// Delete a save (and all its snapshots)
    Delete {
        id: String,
        #[arg(long)]
        yes: bool,
    },
}

pub async fn run(cmd: SaveCommand) -> Result<()> {
    let (cfg, _) = CliConfig::load_default()?;
    let token = cfg.require_token()?;
    let client = ApiClient::new(cfg.server.url.clone(), token)?;

    match cmd {
        SaveCommand::Create { game, label } => {
            let s = client.create_save(&game, &label).await?;
            println!("created save {} ({}/{})", s.id, s.game_slug, s.label);
        }
        SaveCommand::List { game } => {
            let saves = client.list_saves(game.as_deref()).await?;
            if saves.is_empty() {
                println!("(no saves)");
                return Ok(());
            }
            println!(
                "{:<38} {:<24} {:<16} {:>5} {:>10}",
                "ID", "GAME", "LABEL", "VERS", "SIZE"
            );
            for s in saves {
                println!(
                    "{:<38} {:<24} {:<16} {:>5} {:>10}",
                    s.id,
                    s.game_slug,
                    s.label,
                    s.latest_version_num.unwrap_or(0),
                    fmt_size(s.total_size_bytes.unwrap_or(0))
                );
            }
        }
        SaveCommand::Show { id } => {
            let s = client.get_save(&id).await?;
            println!("id:        {}", s.id);
            println!("game:      {}", s.game_slug);
            println!("label:     {}", s.label);
            println!("snapshots: {}", s.snapshot_count.unwrap_or(0));
            println!("latest:    v{}", s.latest_version_num.unwrap_or(0));
            println!("size:      {}", fmt_size(s.total_size_bytes.unwrap_or(0)));
            println!("created:   {}", s.created_at);
            println!("updated:   {}", s.updated_at);
        }
        SaveCommand::Delete { id, yes } => {
            if !yes {
                use std::io::Write;
                print!("delete save {} and ALL its snapshots? [y/N] ", id);
                std::io::stdout().flush()?;
                let mut buf = String::new();
                std::io::stdin().read_line(&mut buf)?;
                if !matches!(buf.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
                    println!("aborted");
                    return Ok(());
                }
            }
            client.delete_save(&id).await?;
            println!("deleted save {}", id);
        }
    }
    Ok(())
}

fn fmt_size(b: i64) -> String {
    let b = b as f64;
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    if b >= GB {
        format!("{:.2}G", b / GB)
    } else if b >= MB {
        format!("{:.2}M", b / MB)
    } else if b >= KB {
        format!("{:.2}K", b / KB)
    } else {
        format!("{}B", b as i64)
    }
}
