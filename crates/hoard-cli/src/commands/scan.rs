//! `hoard scan` — benchmark the local game-detection pass.
//!
//! This runs exactly the heavy half of what Modo Automático executes on every
//! `automatic-tick`: `detection::detect_all`, the disk walk that cross-checks
//! the Ludusavi catalog + Steam libraries against the filesystem. It needs no
//! server and writes nothing, so it's a safe, repeatable way to answer "is the
//! periodic scan actually expensive on this machine?" — run it a few times and
//! watch the wall-clock figure (and a system monitor for CPU/disk).
//!
//! Note it does *not* include the backup sweep (re-hashing each tracked save),
//! which needs a server + tracked saves; the scan is the dominant, machine-
//! local cost and the part that's unique to ticking periodically.

use std::time::Instant;

use anyhow::Result;
use hoard_agent::detection::{self, Confidence};
use hoard_agent::manifest::Os;
use hoard_agent::state::CliState;

pub async fn run(verbose: bool) -> Result<()> {
    let os = Os::current();
    // Load overrides so the bench mirrors what the app actually scans.
    let (cli_state, _) = CliState::load_default()?;

    let start = Instant::now();
    let report = detection::detect_all(os, &cli_state, |_done, _total| {}).await?;
    let elapsed = start.elapsed();

    let (mut high, mut medium, mut low, mut with_paths) = (0usize, 0usize, 0usize, 0usize);
    for g in &report.games {
        match g.confidence {
            Confidence::High => high += 1,
            Confidence::Medium => medium += 1,
            Confidence::Low => low += 1,
        }
        if !g.found_paths.is_empty() {
            with_paths += 1;
        }
    }

    println!("scan completed in {:.3}s", elapsed.as_secs_f64());
    println!("  catalog entries:   {}", report.catalog_size);
    println!("  steam apps found:  {}", report.steam_apps_found);
    println!("  games detected:    {}", report.games.len());
    println!("    high confidence: {high}");
    println!("    medium:          {medium}");
    println!("    low:             {low}");
    println!("    with save paths: {with_paths}");

    if verbose {
        println!();
        println!("{:<32} {:<7} PATHS", "SLUG", "CONF");
        for g in &report.games {
            let conf = match g.confidence {
                Confidence::High => "high",
                Confidence::Medium => "medium",
                Confidence::Low => "low",
            };
            let paths = g
                .found_paths
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            println!("{:<32} {:<7} {}", g.slug, conf, paths);
        }
    }

    Ok(())
}
