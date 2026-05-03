mod commands;

use anyhow::Result;
use clap::{Parser, Subcommand};
use hoard_server::config::Config;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "hoard-admin",
    version,
    about = "Hoard server administration CLI"
)]
struct Cli {
    /// Path to server configuration file
    #[arg(long, default_value = "/etc/hoard/config.toml")]
    config: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Database management
    Db {
        #[command(subcommand)]
        action: commands::db::DbCommand,
    },
    /// User management
    User {
        #[command(subcommand)]
        action: commands::user::UserCommand,
    },
    /// Token management
    Token {
        #[command(subcommand)]
        action: commands::token::TokenCommand,
    },
    /// Game catalog management
    Game {
        #[command(subcommand)]
        action: commands::game::GameCommand,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    let cli = Cli::parse();
    let cfg = Config::load(&cli.config)?;

    match cli.command {
        Commands::Db { action } => commands::db::run(action, &cfg).await,
        Commands::User { action } => commands::user::run(action, &cfg).await,
        Commands::Token { action } => commands::token::run(action, &cfg).await,
        Commands::Game { action } => commands::game::run(action, &cfg).await,
    }
}
