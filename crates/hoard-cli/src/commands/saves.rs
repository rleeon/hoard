use anyhow::Result;
use clap::Subcommand;
use serde::Serialize;

use hoard_agent::api::ApiClient;
use hoard_agent::config::CliConfig;
use hoard_agent::library;
use hoard_agent::state::CliState;

use crate::commands::link;
use crate::output;

/// The result of a local mutation, so an agent can confirm what it changed
/// instead of parsing a sentence.
#[derive(Serialize)]
pub struct MutationOut {
    pub save_id: String,
    /// Which mutation ran.
    pub action: &'static str,
    /// What the resident service said when told to reload.
    pub applied: String,
}

/// A save as the server knows it. Shared by `save list` and `save show`: the
/// same row, so an agent parses one shape.
#[derive(Serialize)]
pub struct SaveInfo {
    pub save_id: String,
    pub game_slug: String,
    pub label: String,
    pub latest_version_num: Option<i64>,
    pub snapshot_count: Option<i64>,
    pub total_size_bytes: Option<i64>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

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
    /// Pause automatic tracking for a save (the sync service stops watching it
    /// right away). Local-only; no server needed.
    Pause {
        /// Save id (UUID), see `hoard saves`
        id: String,
    },
    /// Resume automatic tracking for a paused save. Local-only.
    Resume {
        /// Save id (UUID), see `hoard saves`
        id: String,
    },
    /// Pin (or clear) the sync preset for a save. Omit `--preset` to clear back
    /// to the global defaults. Local-only.
    Preset {
        /// Save id (UUID), see `hoard saves`
        id: String,
        /// Preset id (see `hoard save preset --help`); omit to clear the override.
        #[arg(long)]
        preset: Option<String>,
    },
    /// Stop watching a save on THIS machine. The cloud copy and its whole
    /// version history stay untouched; this is not `delete`. Re-add it later from
    /// the Library or with `hoard track`. Local-only.
    Untrack {
        /// Save id (UUID), see `hoard saves`
        id: String,
    },
    /// Change the local save folder for a save (moved install, new drive). The
    /// folder is created if missing. Local-only.
    Path {
        /// Save id (UUID), see `hoard saves`
        id: String,
        /// New save folder on this machine
        path: String,
    },
}

pub async fn run(cmd: SaveCommand) -> Result<()> {
    // Local-only commands mutate `state.json` and need no server session (a Cloud
    // user has no self-host token). Handle them before building a client.
    //
    // All four change *what the engine watches*, so since Slice 4c they tell the
    // service (`Request::Reload`) instead of asking the user to restart `hoard
    // sync`, which would no longer restart any engine anyway, because the engine
    // stopped living in this process.
    match &cmd {
        SaveCommand::Pause { id } => {
            library::set_paused(id, true)?;
            let applied = link::notify_reload().await;
            println!("paused {id} ({applied})");
            return Ok(());
        }
        SaveCommand::Resume { id } => {
            library::set_paused(id, false)?;
            let applied = link::notify_reload().await;
            println!("resumed {id} ({applied})");
            return Ok(());
        }
        SaveCommand::Preset { id, preset } => {
            library::set_preset(id, preset.clone())?;
            let applied = link::notify_reload().await;
            match preset {
                Some(p) => println!("preset for {id} set to '{p}' ({applied})"),
                None => println!("preset for {id} cleared (standard) ({applied})"),
            }
            return Ok(());
        }
        SaveCommand::Path { id, path } => {
            library::set_local_path(id, path)?;
            let applied = link::notify_reload().await;
            println!("path for {id} set to {path} ({applied})");
            return Ok(());
        }
        SaveCommand::Untrack { id } => {
            // `library::untrack` drops the row whether or not it was there. For
            // a person that is harmless; for a caller that got the id wrong, a
            // silent Ok reads as "stopped tracking" while the real save goes on
            // being watched.
            let (state, _) = CliState::load_default()?;
            if !state.saves.contains_key(id) {
                return Err(output::err(
                    "not_tracked",
                    format!(
                        "no save with id {id} is tracked on this machine — \
                         see `hoard saves --json`"
                    ),
                ));
            }
            library::untrack(id)?;
            let applied = link::notify_reload().await;
            let out = MutationOut {
                save_id: id.clone(),
                action: "untrack",
                applied: applied.to_string(),
            };
            return output::emit(&out, |o| {
                println!(
                    "stopped tracking {} here ({}). The cloud copy is untouched.",
                    o.save_id, o.applied
                );
            });
        }
        _ => {}
    }

    let (cfg, _) = CliConfig::load_default()?;
    let token = output::require_token(&cfg)?;
    let client = ApiClient::new(cfg.server.url.clone(), token)?;

    match cmd {
        SaveCommand::Create { game, label } => {
            let s = client.create_save(&game, &label).await?;
            println!("created save {} ({}/{})", s.id, s.game_slug, s.label);
        }
        SaveCommand::List { game } => {
            let saves = client.list_saves(game.as_deref()).await?;
            let rows: Vec<SaveInfo> = saves
                .into_iter()
                .map(|s| SaveInfo {
                    save_id: s.id.to_string(),
                    game_slug: s.game_slug.to_string(),
                    label: s.label,
                    latest_version_num: s.latest_version_num,
                    snapshot_count: None,
                    total_size_bytes: s.total_size_bytes,
                    created_at: None,
                    updated_at: None,
                })
                .collect();
            output::emit(&rows, |rows| {
                if rows.is_empty() {
                    println!("(no saves)");
                    return;
                }
                println!(
                    "{:<38} {:<24} {:<16} {:>5} {:>10}",
                    "ID", "GAME", "LABEL", "VERS", "SIZE"
                );
                for s in rows {
                    println!(
                        "{:<38} {:<24} {:<16} {:>5} {:>10}",
                        s.save_id,
                        s.game_slug,
                        s.label,
                        s.latest_version_num.unwrap_or(0),
                        fmt_size(s.total_size_bytes.unwrap_or(0))
                    );
                }
            })?;
        }
        SaveCommand::Show { id } => {
            let s = client.get_save(&id).await?;
            let info = SaveInfo {
                save_id: s.id.to_string(),
                game_slug: s.game_slug.to_string(),
                label: s.label,
                latest_version_num: s.latest_version_num,
                snapshot_count: s.snapshot_count,
                total_size_bytes: s.total_size_bytes,
                created_at: Some(s.created_at.to_string()),
                updated_at: Some(s.updated_at.to_string()),
            };
            output::emit(&info, |s| {
                println!("id:        {}", s.save_id);
                println!("game:      {}", s.game_slug);
                println!("label:     {}", s.label);
                println!("snapshots: {}", s.snapshot_count.unwrap_or(0));
                println!("latest:    v{}", s.latest_version_num.unwrap_or(0));
                println!("size:      {}", fmt_size(s.total_size_bytes.unwrap_or(0)));
                println!("created:   {}", s.created_at.as_deref().unwrap_or("—"));
                println!("updated:   {}", s.updated_at.as_deref().unwrap_or("—"));
            })?;
        }
        SaveCommand::Delete { id, yes } => {
            if !yes && !output::interactive() {
                return Err(output::err(
                    "needs_confirmation",
                    format!(
                        "deleting save {id} destroys it and ALL its stored versions, \
                         and there is no terminal to confirm it. Pass --yes if you \
                         mean it. To stop watching the folder while keeping every \
                         version, use `hoard save untrack {id}` instead."
                    ),
                ));
            }
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
        // Local-only variants are handled above with an early return.
        SaveCommand::Pause { .. }
        | SaveCommand::Resume { .. }
        | SaveCommand::Preset { .. }
        | SaveCommand::Untrack { .. }
        | SaveCommand::Path { .. } => unreachable!("handled before the client is built"),
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
