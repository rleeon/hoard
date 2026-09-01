//! `hoard track <game>`: detects a game, creates its save on the server and
//! remembers the local path in `state.json`. Populates what `hoard daemon` and
//! `hoard sync` then watch. It is the headless equivalent of the desktop's
//! `add_game_to_tracking`: same flow (detection, `create_save` with re-link on
//! 409, `CliState.saves.insert`), no GUI.

use anyhow::{bail, Context, Result};
use std::path::PathBuf;

use hoard_agent::detection::{self, DetectedGame};
use hoard_agent::library::{self, AddGameArgs};
use hoard_agent::manifest::Os;
use hoard_agent::state::CliState;

use crate::commands::link;

pub struct Args {
    /// Game name or slug to search for. Optional if `--slug` is given.
    pub query: Option<String>,
    /// Exact slug: skips the fuzzy search.
    pub slug: Option<String>,
    /// Explicit save folder. Wins over whatever the scan detects.
    pub path: Option<PathBuf>,
    /// Save label (default "main").
    pub label: Option<String>,
    /// Deep scan (arbitrary Wine prefixes, Flatpak/Snap, deep walks).
    pub deep: bool,
}

pub async fn run(args: Args) -> Result<()> {
    // Resolve the session (Cloud or self-host) and pin the sync context before
    // loading state, so the save is remembered in that account's map. The Cloud
    // token comes on loan from the service; the CLI does not rotate (ADR 0021).
    let active = link::resolve_session().await?;
    let client = &active.client;

    let label = args.label.clone().unwrap_or_else(|| "main".to_string());
    let (state, _) = CliState::load_default()?;

    // No arguments means interactive mode: scan, pick from the list or type a
    // name and path by hand (any folder, any disk). With arguments it resolves
    // directly, which is scriptable. Both branches produce the target and the
    // save folder.
    let (target, local_path) = if args.query.is_none() && args.slug.is_none() && args.path.is_none()
    {
        interactive_select(&state).await?
    } else {
        let target = resolve_target(&args, &state).await?;
        // `--path` rules; otherwise the best path the scan detected.
        let local_path = match args.path.clone().or_else(|| target.best_path.clone()) {
            Some(p) => p,
            None => bail!(
                "detected \"{}\" ({}) but no save folder on disk. \
                 Pass it with --path <folder>.",
                target.display_name,
                target.slug
            ),
        };
        (target, local_path)
    };

    // Folder creation, cloud/self-hosted branching, dedup and 409 re-link all live
    // in `hoard_agent::library::add_to_tracking`, the same code the desktop's
    // `add_game_to_tracking` runs, so the CLI stays in lockstep.
    let outcome = library::add_to_tracking(
        client,
        AddGameArgs {
            name: None,
            slot: None,
            repoint: false,
            game_slug: target.slug.clone(),
            label: Some(label),
            local_path: local_path.to_string_lossy().into_owned(),
            display_name: Some(target.display_name.clone()),
            steam_app_id: target.steam_app_id.map(|id| id as i64),
            preset: None,
            processes: None,
            // One game per entry: nobody else tracks its process.
            shared_processes: false,
        },
    )
    .await?;

    // The watched set changed, so have the service re-read it. The service owns
    // that set, so it gets told rather than restarted.
    let applied = link::notify_reload().await;
    println!(
        "tracking {} ({})\n  path:    {}\n  save_id: {}\n  {applied}",
        target.display_name,
        target.slug,
        local_path.display(),
        outcome.tracked.save_id
    );
    Ok(())
}

struct Target {
    slug: String,
    display_name: String,
    best_path: Option<PathBuf>,
    steam_app_id: Option<u64>,
}

/// Decides which game to track. With `--slug` it goes direct (and looks up its
/// path in the scan); otherwise it matches `query` against the detected games.
async fn resolve_target(args: &Args, state: &CliState) -> Result<Target> {
    let os = Os::current();
    let report = if args.deep {
        detection::detect_all_deep(os, state, |_, _| {}).await?
    } else {
        detection::detect_all(os, state, |_, _| {}).await?
    };

    if let Some(slug) = &args.slug {
        let found = report.games.iter().find(|g| &g.slug == slug);
        return Ok(Target {
            slug: slug.clone(),
            display_name: found
                .map(|g| g.display_name.clone())
                .unwrap_or_else(|| slug.clone()),
            best_path: found.and_then(|g| g.found_paths.first().cloned()),
            steam_app_id: found.and_then(|g| g.steam_app_id),
        });
    }

    let query = args
        .query
        .as_deref()
        .context("name the game: `hoard track \"<name>\"` or --slug <slug>")?;
    let q = query.to_lowercase();

    // Exact match (slug or name) first; otherwise substring.
    let exact: Vec<&DetectedGame> = report
        .games
        .iter()
        .filter(|g| g.slug.to_lowercase() == q || g.display_name.to_lowercase() == q)
        .collect();
    let candidates: Vec<&DetectedGame> = if !exact.is_empty() {
        exact
    } else {
        report
            .games
            .iter()
            .filter(|g| {
                g.slug.to_lowercase().contains(&q) || g.display_name.to_lowercase().contains(&q)
            })
            .collect()
    };

    match candidates.as_slice() {
        [] => bail!(
            "didn't detect any game matching \"{query}\". \
             Try --slug <slug> or --path <folder>."
        ),
        [g] => Ok(Target {
            slug: g.slug.clone(),
            display_name: g.display_name.clone(),
            best_path: g.found_paths.first().cloned(),
            steam_app_id: g.steam_app_id,
        }),
        many => {
            let list = many
                .iter()
                .take(10)
                .map(|g| format!("  {} ({})", g.display_name, g.slug))
                .collect::<Vec<_>>()
                .join("\n");
            bail!("\"{query}\" matches several games; narrow it down with --slug:\n{list}")
        }
    }
}

/// `hoard track` with no arguments. Scans, lists the detected games (with the path
/// found) and lets you pick one by number, or "Other" to type a name and path by
/// hand, taking any folder on any disk or partition. Returns the target and the
/// chosen path, which feed the same pipeline as the flag-driven route.
async fn interactive_select(state: &CliState) -> Result<(Target, PathBuf)> {
    use std::io::{self, Write};

    // Checked before the scan, not after: without a terminal this can only end
    // in a prompt nobody answers, and making the caller wait through a full
    // detection pass first to reach that dead end is the worst of both.
    if !crate::output::interactive() {
        return Err(crate::output::err(
            "needs_choice",
            "`hoard track` with no arguments asks which game to track, and there \
             is no terminal to ask. Name the game (`hoard track \"<name>\"`), or \
             pick one with --slug and --path. `hoard scan --verbose` lists what \
             this machine detects.",
        ));
    }

    println!("Scanning games…");
    let report = detection::detect_all(Os::current(), state, |_, _| {}).await?;

    // Only the ones with a path on disk: tracking one without a folder adds
    // nothing (that's what "Other" is for). Sorted by name for a stable list.
    let mut games: Vec<&DetectedGame> = report
        .games
        .iter()
        .filter(|g| !g.found_paths.is_empty())
        .collect();
    games.sort_by_key(|g| g.display_name.to_lowercase());

    println!();
    for (i, g) in games.iter().enumerate() {
        let path = g
            .found_paths
            .first()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        println!("  {:>2}) {:<30}  {}", i + 1, g.display_name, path);
    }
    let other = games.len() + 1;
    println!("  {other:>2}) Other (name + path by hand)");
    println!();

    let choice = loop {
        print!("Pick a number [1-{other}]: ");
        io::stdout().flush().ok();
        let mut buf = String::new();
        if io::stdin().read_line(&mut buf)? == 0 {
            bail!("no input; use `hoard track \"<name>\"` or --slug/--path.");
        }
        match buf.trim().parse::<usize>() {
            Ok(n) if (1..=other).contains(&n) => break n,
            _ => println!("Number out of range, try again."),
        }
    };

    if choice == other {
        let name = prompt_nonempty("Name: ")?;
        let raw = prompt_nonempty("Path: ")?;
        let path = expand_tilde(&raw);
        Ok((
            Target {
                slug: slugify(&name),
                display_name: name,
                best_path: Some(path.clone()),
                steam_app_id: None,
            },
            path,
        ))
    } else {
        let g = games[choice - 1];
        let path = g
            .found_paths
            .first()
            .cloned()
            .context("the chosen game had no detected path")?;
        Ok((
            Target {
                slug: g.slug.clone(),
                display_name: g.display_name.clone(),
                best_path: Some(path.clone()),
                steam_app_id: g.steam_app_id,
            },
            path,
        ))
    }
}

/// Reads a line from stdin and repeats until it's non-empty.
fn prompt_nonempty(label: &str) -> Result<String> {
    if !crate::output::interactive() {
        return Err(crate::output::err(
            "needs_input",
            format!("this step asks for input ({label:?}) and there is no terminal to ask"),
        ));
    }
    use std::io::{self, Write};
    loop {
        print!("{label}");
        io::stdout().flush().ok();
        let mut buf = String::new();
        if io::stdin().read_line(&mut buf)? == 0 {
            bail!("input closed");
        }
        let s = buf.trim().to_string();
        if !s.is_empty() {
            return Ok(s);
        }
    }
}

/// Expands `~/` (and `~\` on Windows) using HOME/USERPROFILE. Leaves the rest.
fn expand_tilde(raw: &str) -> PathBuf {
    if let Some(rest) = raw.strip_prefix("~/").or_else(|| raw.strip_prefix("~\\")) {
        if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(raw)
}

/// Local slug for the "Other" case (game outside the catalog). Same style as
/// `hoard_manifest::ludusavi::slugify` but without pulling that dep into the CLI.
fn slugify(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut last_dash = true; // avoids a leading dash
    for ch in name.chars() {
        let c = ch.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        out.push_str("game");
    }
    if !out.starts_with(|c: char| c.is_ascii_alphanumeric()) {
        out.insert(0, 'g');
    }
    out
}
