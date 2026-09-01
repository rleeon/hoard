//! `cargo run --example smoke_detect -p hoard-agent`
//!
//! Tiny harness that runs `detect_all` against the host and prints the
//! report. Use it to sanity-check detection on your dev machine without
//! firing up the desktop app: if this returns 0 games on a box that has
//! Steam + saves on disk, detection is broken.

use hoard_agent::detection::detect_all;
use hoard_agent::manifest::Os;
use hoard_agent::state::CliState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let os = Os::current();
    println!("Scanning for installed games on {os:?}...");

    // The smoke binary doesn't load the on-disk state, it's a quick
    // detection sanity check, not a session. An empty CliState means no
    // manual_paths overrides apply, which is fine for "did the heuristics
    // find anything?".
    let state = CliState::default();
    let report = detect_all(os, &state, |done, total| {
        if total > 0 && done % 4096 == 0 {
            println!("  progress: {done}/{total}");
        }
    })
    .await?;

    println!();
    println!("=== Detection report ===");
    println!("catalog_size:   {}", report.catalog_size);
    println!("steam_apps:     {}", report.steam_apps_found);
    println!("detected games: {}", report.games.len());
    println!();
    for g in report.games.iter().take(50) {
        println!(
            "  {:<40} ({:?}, {:?})",
            g.display_name, g.confidence, g.source
        );
        for p in &g.found_paths {
            println!("      → {}", p.display());
        }
    }
    if report.games.len() > 50 {
        println!("  ... and {} more", report.games.len() - 50);
    }
    Ok(())
}
