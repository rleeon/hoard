use std::time::Duration;

use anyhow::{bail, Result};
use clap::Subcommand;
use serde::Serialize;

use hoard_agent::api::ApiError;
use hoard_agent::detection::DetectionTrace;
use hoard_agent::library::PlaytimeGameInfo;
use hoard_agent::manifest::Os;
use hoard_agent::state::CliState;
use hoard_agent::{catalog, cloud_auth, detection, library, session};
use hoard_core::ipc::{CatalogInfo, Payload, Request};

use crate::commands::link;
use crate::output;

/// What the self-hosted `/v1/games` hands out when not asked for a size.
const SEARCH_LIMIT: usize = 20;

/// A diagnosis replays detection for one game, deep places included.
const DIAGNOSE_LIMIT: Duration = Duration::from_secs(30 * 60);

/// Knowing what is installed sweeps the disks, network shares included: 20 s at
/// most in the service. This only catches one that stopped answering.
const PLAYED_LIMIT: Duration = Duration::from_secs(2 * 60);

/// A 17 MB download and its conversion, on a slow line.
const CATALOG_REFRESH_LIMIT: Duration = Duration::from_secs(5 * 60);

#[derive(Subcommand)]
pub enum GameCommand {
    /// List or search games in the catalog
    Search {
        /// Optional substring (display name or slug)
        query: Option<String>,
    },
    /// Show details for a single game
    Show { slug: String },
    /// Explain how detection looks for one game: every step, the folders it
    /// tried, and why it kept or dropped each. The answer to "why doesn't this
    /// game show up?". Reads only.
    Diagnose {
        /// Game slug (`hoard games search` finds it)
        slug: String,
    },
    /// The "Played, no backup" list: online games installed here that have no
    /// save to sync, whose hours Hoard counts for the playtime recap anyway.
    /// `--exclude` stops counting a game's hours, `--include` starts again.
    Played {
        /// Stop counting this game's hours. Repeatable.
        #[arg(long, value_name = "SLUG")]
        exclude: Vec<String>,
        /// Count this game's hours again. Repeatable.
        #[arg(long, value_name = "SLUG")]
        include: Vec<String>,
    },
    /// The game catalog in use: how many games it knows and when it was last
    /// updated. The sync service refreshes it once a day on its own.
    Catalog {
        /// Download the latest catalog now
        #[arg(long)]
        refresh: bool,
    },
}

pub async fn run(cmd: GameCommand) -> Result<()> {
    match cmd {
        GameCommand::Search { query } => search(query).await,
        GameCommand::Show { slug } => show(slug).await,
        GameCommand::Diagnose { slug } => diagnose(slug).await,
        GameCommand::Played { exclude, include } => played(exclude, include).await,
        GameCommand::Catalog { refresh } => catalog_status(refresh).await,
    }
}

/// Cloud serves no catalogue: it answers from the local copy, which is the same
/// Ludusavi data a self-hosted server loads and needs no network.
async fn catalog_client() -> Result<Option<hoard_agent::api::ApiClient>> {
    if cloud_auth::load_session()?.is_some() {
        return Ok(None);
    }
    Ok(Some(link::resolve_session().await?.client))
}

async fn search(query: Option<String>) -> Result<()> {
    let games = match catalog_client().await? {
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
    Ok(())
}

async fn show(slug: String) -> Result<()> {
    let g = match catalog_client().await? {
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
    Ok(())
}

// ---- diagnose

#[derive(Serialize)]
pub struct TraceOut {
    pub slug: String,
    pub steps: Vec<TraceStepOut>,
}

/// One step of the pipeline. `step` names it (`manual_override`,
/// `steam_appid`, `filesystem`, `proton_prefix`, `refine`, `correlation`…),
/// `template` is what it worked from, `expanded` what it produced before any
/// check, `kept` what survived and `dropped` the rest, each with its reason.
#[derive(Serialize)]
pub struct TraceStepOut {
    pub step: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    pub expanded: Vec<String>,
    pub kept: Vec<String>,
    pub dropped: Vec<DroppedOut>,
}

#[derive(Serialize)]
pub struct DroppedOut {
    pub path: String,
    pub reason: String,
}

async fn diagnose(slug: String) -> Result<()> {
    let slug = slug.trim().to_string();
    if slug.is_empty() {
        bail!("the slug is empty");
    }
    let trace: DetectionTrace = match link::detecting("diagnose").await {
        Some(mut client) => {
            let request = Request::DiagnoseDetection { slug: slug.clone() };
            match link::ask_within(&mut client, request, DIAGNOSE_LIMIT).await? {
                Payload::DetectionTrace { trace } => serde_json::from_value(trace)?,
                other => bail!("the service answered a {other:?} to a diagnosis"),
            }
        }
        None => {
            session::set_context_offline();
            let (state, _) = CliState::load_default()?;
            detection::diagnose(&slug, Os::current(), &state).await
        }
    };
    let out = TraceOut {
        slug: trace.slug,
        steps: trace
            .attempts
            .into_iter()
            .map(|step| TraceStepOut {
                step: step.kind,
                template: step.template,
                expanded: step.expanded,
                kept: step.kept,
                dropped: step
                    .dropped
                    .into_iter()
                    .map(|d| DroppedOut {
                        path: d.path,
                        reason: d.reason,
                    })
                    .collect(),
            })
            .collect(),
    };
    output::emit(&out, |out| {
        println!("detection for {}", out.slug);
        for step in &out.steps {
            println!();
            match &step.template {
                Some(template) => println!("[{}] {template}", step.step),
                None => println!("[{}]", step.step),
            }
            if step.expanded.is_empty() && step.kept.is_empty() && step.dropped.is_empty() {
                println!("  (nothing)");
            }
            for path in &step.kept {
                println!("  kept     {path}");
            }
            for d in &step.dropped {
                println!("  dropped  {}  ({})", d.path, d.reason);
            }
            // What it produced and neither kept nor dropped: inputs a later step
            // works on, which is worth seeing when the last step comes up empty.
            for path in step
                .expanded
                .iter()
                .filter(|p| !step.kept.contains(*p) && !step.dropped.iter().any(|d| &d.path == *p))
            {
                println!("  tried    {path}");
            }
        }
    })
}

// ---- played

#[derive(Serialize)]
pub struct PlayedGame {
    pub slug: String,
    pub display_name: String,
    /// The user stopped counting its hours.
    pub excluded: bool,
}

#[derive(Serialize)]
pub struct PlayedOut {
    pub games: Vec<PlayedGame>,
}

async fn played(exclude: Vec<String>, include: Vec<String>) -> Result<()> {
    let mut games = played_games().await?;

    if !exclude.is_empty() || !include.is_empty() {
        let exclude: Vec<String> = exclude.iter().map(|s| s.trim().to_string()).collect();
        let include: Vec<String> = include.iter().map(|s| s.trim().to_string()).collect();
        // The list is every game this can apply to: an installed one, or an
        // excluded one no longer installed. Anything else is a typo, and storing
        // it would quietly exclude nothing.
        for slug in exclude.iter().chain(&include) {
            if !games.iter().any(|g| &g.slug == slug) {
                return Err(output::err(
                    "not_found",
                    format!("{slug} isn't in the list; `hoard games played` shows it"),
                ));
            }
        }
        session::set_context_offline();
        let (mut state, path) = CliState::load_default()?;
        for slug in &exclude {
            state.exclude_playtime(slug.clone());
        }
        for slug in &include {
            state.include_playtime(slug);
        }
        state.save(&path)?;
        for g in &mut games {
            if exclude.contains(&g.slug) {
                g.excluded = true;
            } else if include.contains(&g.slug) {
                g.excluded = false;
            }
        }
        // The service seeds the hours-only slots from `state.json`: this drops
        // the excluded ones and picks the included ones up without a restart.
        let applied = link::notify_reload().await;
        if !output::json() {
            println!("saved ({applied})");
            println!();
        }
    }

    let out = PlayedOut {
        games: games
            .into_iter()
            .map(|g| PlayedGame {
                slug: g.slug,
                display_name: g.display_name,
                excluded: g.excluded,
            })
            .collect(),
    };
    output::emit(&out, |out| {
        if out.games.is_empty() {
            println!("no online games without a save are installed here");
            return;
        }
        println!("{:<32} {:<32} HOURS", "SLUG", "NAME");
        for g in &out.games {
            println!(
                "{:<32} {:<32} {}",
                g.slug,
                g.display_name,
                if g.excluded { "not counted" } else { "counted" }
            );
        }
    })
}

/// Asked of the service when it runs detection, as the app does; worked out here
/// otherwise, or when it can't answer.
async fn played_games() -> Result<Vec<PlaytimeGameInfo>> {
    if let Some(mut client) = link::detecting("played").await {
        match link::ask_within(&mut client, Request::PlaytimeGames, PLAYED_LIMIT).await {
            Ok(Payload::PlaytimeGames { games }) => return Ok(serde_json::from_value(games)?),
            Ok(other) => tracing::warn!(?other, "unexpected answer to the playtime games"),
            Err(e) => tracing::info!(
                error = %format!("{e:#}"),
                "the service can't list the playtime games; working them out here"
            ),
        }
    }
    session::set_context_offline();
    tokio::task::spawn_blocking(library::playtime_games).await?
}

// ---- catalog

#[derive(Serialize)]
pub struct CatalogOut {
    pub games: u64,
    /// A downloaded copy is in use, not the one this build shipped with.
    pub downloaded: bool,
    /// RFC 3339; absent when it was never refreshed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    /// This invocation downloaded it.
    pub refreshed: bool,
}

async fn catalog_status(refresh: bool) -> Result<()> {
    let info = match link::detecting("catalog").await {
        Some(mut client) => {
            let request = if refresh {
                eprintln!("downloading the catalog in the Hoard service…");
                Request::CatalogRefresh
            } else {
                Request::CatalogStatus
            };
            match link::ask_within(&mut client, request, CATALOG_REFRESH_LIMIT).await? {
                Payload::Catalog(info) => info,
                other => bail!("the service answered a {other:?} about the catalog"),
            }
        }
        None if refresh => {
            let update = catalog::refresh(|stage| eprintln!("{stage}…")).await?;
            CatalogInfo {
                games: update.games as u64,
                has_runtime_override: true,
                updated_at: Some(update.updated_at),
                size_bytes: Some(update.size_bytes),
            }
        }
        None => {
            let status = tokio::task::spawn_blocking(catalog::status).await?;
            CatalogInfo {
                games: status.games as u64,
                has_runtime_override: status.has_runtime_override,
                updated_at: status.updated_at,
                size_bytes: None,
            }
        }
    };
    let out = CatalogOut {
        games: info.games,
        downloaded: info.has_runtime_override,
        updated_at: info.updated_at.and_then(rfc3339),
        refreshed: refresh,
    };
    output::emit(&out, |out| {
        let when = out.updated_at.as_deref().unwrap_or("never refreshed");
        let source = if out.downloaded {
            "downloaded"
        } else {
            "shipped with this build"
        };
        println!("{} games · {source} · updated {when}", out.games);
    })
}

fn rfc3339(unix: u64) -> Option<String> {
    let at = time::OffsetDateTime::from_unix_timestamp(unix as i64).ok()?;
    at.format(&time::format_description::well_known::Rfc3339)
        .ok()
}
