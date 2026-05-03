mod api;
mod commands;
mod config;
mod state;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "hoard", version, about = "Hoard save-sync client")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show server status (uses /v1/health)
    Status,
    /// Configuration file management
    Config {
        #[command(subcommand)]
        action: commands::config::ConfigCommand,
    },
    /// Save the bearer token from the server into the local config
    Login {
        /// Bearer token (`hoard_v1_<hex>`). Get it from the admin via `hoard-admin token create`.
        #[arg(long)]
        token: String,
    },
    /// Clear the saved bearer token
    Logout,
    /// Show the currently logged-in user
    Whoami,
    /// Browse the game catalog
    Games {
        #[command(subcommand)]
        action: commands::games::GameCommand,
    },
    /// Manage save namespaces
    Save {
        #[command(subcommand)]
        action: commands::saves::SaveCommand,
    },
    /// Manage snapshots (list / delete / undelete)
    Snapshots {
        #[command(subcommand)]
        action: SnapshotCommand,
    },
    /// Upload a directory as a new snapshot
    Backup {
        /// Save id (UUID) — see `hoard save list`
        save_id: String,
        /// Source directory to back up. Required unless previously remembered.
        #[arg(long)]
        from: Option<PathBuf>,
        /// Save the (save_id → local_path) mapping in local state for future runs
        #[arg(long)]
        remember: bool,
    },
    /// Restore a snapshot to disk
    Restore {
        /// Save id (UUID)
        save_id: String,
        /// Snapshot version number; defaults to latest
        #[arg(long)]
        version: Option<i64>,
        /// Destination directory (or use the remembered local_path if omitted)
        #[arg(long)]
        to: Option<PathBuf>,
        /// Skip SHA256 verification
        #[arg(long)]
        no_verify: bool,
        /// Allow extracting into a non-empty directory
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum SnapshotCommand {
    /// List snapshots for a save
    List {
        save_id: String,
        /// Include soft-deleted snapshots (in trash)
        #[arg(long)]
        all: bool,
    },
    /// Soft-delete a snapshot (moves it to trash; recover with `undelete`)
    Delete {
        save_id: String,
        version: i64,
        #[arg(long)]
        yes: bool,
    },
    /// Restore a soft-deleted snapshot back to active state
    Undelete {
        save_id: String,
        version: i64,
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
    if let Err(e) = dispatch(cli).await {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
    Ok(())
}

async fn dispatch(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Status => commands::status::run().await,
        Commands::Config { action } => commands::config::run(action),
        Commands::Login { token } => commands::auth::login(token).await,
        Commands::Logout => commands::auth::logout().await,
        Commands::Whoami => commands::auth::whoami().await,
        Commands::Games { action } => commands::games::run(action).await,
        Commands::Save { action } => commands::saves::run(action).await,
        Commands::Snapshots { action } => snapshots_dispatch(action).await,
        Commands::Backup { save_id, from, remember } => {
            commands::backup::run(save_id, from, remember).await
        }
        Commands::Restore { save_id, version, to, no_verify, force } => {
            commands::restore::apply(save_id, version, to, no_verify, force).await
        }
    }
}

async fn snapshots_dispatch(cmd: SnapshotCommand) -> Result<()> {
    let (cfg, _) = config::CliConfig::load_default()?;
    let token = cfg.require_token()?;
    let client = api::ApiClient::new(cfg.server.url.clone(), token)?;
    match cmd {
        SnapshotCommand::List { save_id, all } => list_snapshots(&client, save_id, all).await,
        SnapshotCommand::Delete { save_id, version, yes } => {
            if !yes {
                use std::io::Write;
                print!("soft-delete v{} of save {}? [y/N] ", version, save_id);
                std::io::stdout().flush()?;
                let mut buf = String::new();
                std::io::stdin().read_line(&mut buf)?;
                if !matches!(buf.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
                    println!("aborted");
                    return Ok(());
                }
            }
            client.snapshot_delete(&save_id, version).await?;
            println!("soft-deleted v{} of save {}", version, save_id);
            Ok(())
        }
        SnapshotCommand::Undelete { save_id, version } => {
            client.snapshot_restore(&save_id, version).await?;
            println!("undeleted v{} of save {}", version, save_id);
            Ok(())
        }
    }
}

async fn list_snapshots(client: &api::ApiClient, save_id: String, include_deleted: bool) -> Result<()> {
    let snaps = client.list_snapshots(&save_id, include_deleted).await?;
    if snaps.is_empty() {
        println!("(no snapshots)");
        return Ok(());
    }
    println!(
        "{:>5}  {:>5}  {:>10}  {:<25}  STATE",
        "VER", "FILES", "SIZE", "CREATED"
    );
    for s in snaps {
        let state_label = if s.deleted_at.is_some() {
            "TRASH"
        } else if s.is_pinned {
            "PINNED"
        } else {
            "active"
        };
        println!(
            "{:>5}  {:>5}  {:>10}  {:<25}  {}",
            s.version_num,
            s.file_count,
            fmt_bytes(s.total_size_bytes as u64),
            s.created_at,
            state_label
        );
    }
    Ok(())
}

fn fmt_bytes(b: u64) -> String {
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
        format!("{}B", b as u64)
    }
}
