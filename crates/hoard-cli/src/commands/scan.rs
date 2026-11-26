//! `hoard scan`: benchmark the local game-detection pass.
//!
//! This runs exactly the heavy half of what Automatic Mode executes on every
//! `automatic-tick`: `detection::detect_all`, the disk walk that cross-checks the
//! Ludusavi catalog and Steam libraries against the filesystem. It needs no server
//! and writes nothing, so it is a safe, repeatable way to answer "is the periodic
//! scan actually expensive on this machine?". Run it a few times and watch the
//! wall-clock figure, and a system monitor for CPU and disk.
//!
//! Note it does *not* include the backup sweep (re-hashing each tracked save),
//! which needs a server and tracked saves. The scan is the dominant machine-local
//! cost and the part that is unique to ticking periodically.
//!
//! Two flags leave the benchmark behind. `--refresh` is the Library's scan
//! button: the result is kept as the list the app shows, and the service runs it
//! when it is the one that does detection. `--folder` is "add from folder", the
//! games inside one folder the user points at.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{bail, Result};
use hoard_agent::detection::{self, Confidence, DetectedGame, DetectionReport};
use hoard_agent::manifest::Os;
use hoard_agent::state::CliState;
use hoard_core::ipc::{Payload, Request};
use serde::Serialize;

use crate::commands::link;
use crate::output;

/// A deep sweep walks every Wine prefix and mounted drive: minutes on a big
/// disk are normal, an hour is not.
const SCAN_LIMIT: Duration = Duration::from_secs(30 * 60);

/// One folder's walk has its own timeout in the service; this is the backstop.
const FOLDER_LIMIT: Duration = Duration::from_secs(5 * 60);

/// One detected game.
#[derive(Serialize)]
pub struct ScanGame {
    pub slug: String,
    pub display_name: String,
    pub confidence: &'static str,
    pub paths: Vec<String>,
    /// The game is installed and its save folder was not located: `paths` is
    /// empty and will stay empty until someone points at a folder. Named
    /// instead of left to be inferred, so a caller can tell "installed, needs a
    /// folder" from "detected with saves" without reading an empty array and
    /// guessing what it means.
    pub needs_folder: bool,
    /// Whether this machine already tracks it, which is the difference between
    /// "we found 200 games" and the only question worth asking, namely which of
    /// them are not being backed up yet.
    pub tracked: bool,
}

#[derive(Serialize)]
pub struct ScanOut {
    pub elapsed_secs: f64,
    /// The exhaustive pass (`--deep`), which walks arbitrary Wine prefixes and
    /// Flatpak/Snap roots.
    pub deep: bool,
    /// `--refresh`: the result is now the Library's list. Without it nothing was
    /// written.
    pub saved: bool,
    pub catalog_entries: usize,
    pub steam_apps_found: usize,
    pub games_detected: usize,
    pub high_confidence: usize,
    pub medium_confidence: usize,
    pub low_confidence: usize,
    pub with_save_paths: usize,
    /// Installed games whose save folder was not located: the ones a caller can
    /// do something about, by picking a folder. The complement of
    /// `with_save_paths`, spelled out so it does not have to be derived.
    pub needing_folder: usize,
    /// Folders added to / removed from the exclusion list by this same
    /// invocation, echoed back so the caller sees what its flags did.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub excluded_added: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub excluded_removed: Vec<String>,
    /// The detected games. Always present under `--json`, and only with
    /// `--verbose` in the human output.
    ///
    /// `--verbose` is a knob for how much a person wants printed; a caller parsing
    /// JSON always wants the list, and getting the counts alone reads as "nothing
    /// found" rather than as "you didn't ask". Absent, not empty, when it
    /// genuinely was not produced, so an empty list means no games.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub games: Option<Vec<ScanGame>>,
}

/// Where the PATHS column starts, so the paths after the first line up under
/// it. Sum of the three left-hand columns and their separating spaces.
const PATH_COLUMN: usize = 32 + 1 + 7 + 1 + 8 + 1;

/// Does this invocation carry the per-game list?
///
/// `--verbose` says how much a person wants printed. A caller parsing JSON
/// always wants the list: handing it the counts alone reads as "nothing found"
/// rather than as "you didn't ask for it", and there is no way to tell those
/// apart from the envelope. So under `--json` the list is not optional.
fn should_list_games(verbose: bool, json: bool) -> bool {
    verbose || json
}

/// The human table: one row per game, and one path per line.
///
/// The paths used to be joined with `", "`, in a column whose values contain that
/// separator for real (`.../unity3d/Cipher Prime Studios, Inc./...` is one save
/// folder, not two), so the row could not be split back apart by anything but
/// guesswork. The machine-readable answer to that is `--json`, which is the
/// contract; this is so a person reading the table can still see where one path
/// ends and the next begins.
fn render_table(games: &[ScanGame]) -> String {
    use std::fmt::Write;

    let mut out = String::new();
    let _ = writeln!(out, "{:<32} {:<7} {:<8} PATHS", "SLUG", "CONF", "TRACKED");
    for g in games {
        let first = if g.needs_folder {
            "-- no save folder located --"
        } else {
            g.paths.first().map(String::as_str).unwrap_or("")
        };
        let _ = writeln!(
            out,
            "{:<32} {:<7} {:<8} {first}",
            g.slug,
            g.confidence,
            if g.tracked { "yes" } else { "no" },
        );
        for path in g.paths.iter().skip(1) {
            let _ = writeln!(out, "{:PATH_COLUMN$}{path}", "");
        }
    }
    out
}

/// The exclusion list, for `--list-excluded`.
#[derive(Serialize)]
pub struct ExcludedOut {
    pub excluded: Vec<String>,
}

pub async fn run(
    verbose: bool,
    deep: bool,
    refresh: bool,
    folder: Option<PathBuf>,
    exclude: Vec<String>,
    unexclude: Vec<String>,
    list_excluded: bool,
) -> Result<()> {
    // Managing discarded folders. It goes before the scan so that an `--exclude`
    // and the scan in the same invocation already reflect it.
    //
    // These print nothing of their own under `--json`: stdout has to hold one
    // envelope and nothing else, so what they did is echoed inside it instead.
    for p in &exclude {
        hoard_agent::library::exclude_path(std::path::Path::new(p.trim()))?;
        if !output::json() {
            println!("excluded {p}");
        }
    }
    for p in &unexclude {
        hoard_agent::library::unexclude_path(std::path::Path::new(p.trim()))?;
        if !output::json() {
            println!("no longer excluded: {p}");
        }
    }
    if list_excluded {
        let paths = hoard_agent::library::list_excluded_paths()?;
        let out = ExcludedOut {
            excluded: paths.iter().map(|p| p.display().to_string()).collect(),
        };
        return output::emit(&out, |out| {
            if out.excluded.is_empty() {
                println!("no folders are excluded from scanning");
            } else {
                for p in &out.excluded {
                    println!("{p}");
                }
            }
        });
    }

    // Load overrides so the bench mirrors what the app actually scans. The
    // tracked-save warnings read the account's saves, hence the context first.
    hoard_agent::session::set_context_offline();
    let (cli_state, _) = CliState::load_default()?;

    if let Some(folder) = folder {
        return scan_folder(folder, &cli_state).await;
    }

    let start = Instant::now();
    let report = if refresh {
        refreshed(deep).await?
    } else {
        let os = Os::current();
        let mut report = if deep {
            detection::detect_all_deep(os, &cli_state, |_done, _total| {}).await?
        } else {
            detection::detect_all(os, &cli_state, |_done, _total| {}).await?
        };
        // The same edge filters the desktop applies, so the bench reflects what the
        // user sees rather than a different list.
        report.games.retain(|g| !cli_state.is_ignored(&g.slug));
        hoard_agent::library::apply_excluded_paths(&mut report, &cli_state);
        report
    };
    let elapsed = start.elapsed();

    let (mut high, mut medium, mut low, mut with_paths) = (0usize, 0usize, 0usize, 0usize);
    let mut needing_folder = 0usize;
    for g in &report.games {
        match g.confidence {
            Confidence::High => high += 1,
            Confidence::Medium => medium += 1,
            Confidence::Low => low += 1,
        }
        if !g.found_paths.is_empty() {
            with_paths += 1;
        }
        if g.needs_folder {
            needing_folder += 1;
        }
    }

    let out = ScanOut {
        elapsed_secs: elapsed.as_secs_f64(),
        deep,
        saved: refresh,
        catalog_entries: report.catalog_size,
        steam_apps_found: report.steam_apps_found,
        games_detected: report.games.len(),
        high_confidence: high,
        medium_confidence: medium,
        low_confidence: low,
        with_save_paths: with_paths,
        needing_folder,
        excluded_added: exclude.clone(),
        excluded_removed: unexclude.clone(),
        games: should_list_games(verbose, output::json()).then(|| {
            report
                .games
                .iter()
                .map(|g| scan_game(g, &cli_state))
                .collect()
        }),
    };

    output::emit(&out, |out| {
        println!("scan completed in {:.3}s", out.elapsed_secs);
        println!("  catalog entries:   {}", out.catalog_entries);
        println!("  steam apps found:  {}", out.steam_apps_found);
        println!("  games detected:    {}", out.games_detected);
        println!("    high confidence: {}", out.high_confidence);
        println!("    medium:          {}", out.medium_confidence);
        println!("    low:             {}", out.low_confidence);
        println!("    with save paths: {}", out.with_save_paths);
        println!("    needs a folder:  {}", out.needing_folder);
        if out.saved {
            println!("  kept as the Library's list");
        }

        if let Some(games) = &out.games {
            println!();
            print!("{}", render_table(games));
        }
    })
}

/// One detected game as the output carries it.
fn scan_game(g: &DetectedGame, state: &CliState) -> ScanGame {
    ScanGame {
        slug: g.slug.clone(),
        display_name: g.display_name.clone(),
        confidence: match g.confidence {
            Confidence::High => "high",
            Confidence::Medium => "medium",
            Confidence::Low => "low",
        },
        paths: g
            .found_paths
            .iter()
            .map(|p| p.display().to_string())
            .collect(),
        needs_folder: g.needs_folder,
        tracked: is_tracked(g, state),
    }
}

/// A game counts as tracked if a save points at one of its folders, or if one
/// carries its slug. Both, because the two can disagree: the slug is not stable
/// across catalog revisions, and a folder can be tracked under a hand-typed name
/// that no longer matches the detected slug.
fn is_tracked(g: &DetectedGame, state: &CliState) -> bool {
    state
        .saves
        .values()
        .any(|s| s.game_slug == g.slug || g.found_paths.contains(&s.local_path))
}

/// `--refresh`: the Library's scan, kept as its list. In the service when it
/// runs detection, so an app open meanwhile reads the same cache; here when it
/// doesn't, the way the desktop falls back.
async fn refreshed(deep: bool) -> Result<DetectionReport> {
    if let Some(mut client) = link::detecting("scan").await {
        eprintln!("scanning in the Hoard service…");
        return match link::ask_within(&mut client, Request::Scan { deep }, SCAN_LIMIT).await? {
            Payload::Detection { report } => Ok(serde_json::from_value(report)?),
            other => bail!("the service answered a {other:?} to a scan"),
        };
    }
    let report = hoard_agent::library::detect_filtered(deep, |_, _| {}).await?;
    Ok(hoard_agent::library::remember_detection(report).report)
}

#[derive(Serialize)]
pub struct FolderScanOut {
    pub folder: String,
    pub games: Vec<ScanGame>,
}

/// `--folder`: the games inside one folder. Reads only; tracking one is
/// `hoard track --path`.
async fn scan_folder(folder: PathBuf, state: &CliState) -> Result<()> {
    if !folder.is_dir() {
        bail!("{} isn't a folder", folder.display());
    }
    // The service has a working directory of its own: a relative path would be
    // read against that one.
    let folder = std::path::absolute(&folder)?;
    let games: Vec<DetectedGame> = match (link::detecting("scan").await, folder.to_str()) {
        (Some(mut client), Some(path)) => {
            let request = Request::ScanFolder {
                path: path.to_string(),
            };
            match link::ask_within(&mut client, request, FOLDER_LIMIT).await? {
                Payload::Detected { games } => serde_json::from_value(games)?,
                other => bail!("the service answered a {other:?} to a folder scan"),
            }
        }
        // A path that isn't UTF-8 doesn't fit the JSON wire, but this process can
        // still walk it.
        _ => {
            let root = folder.clone();
            tokio::task::spawn_blocking(move || hoard_agent::library::scan_folder(&root)).await??
        }
    };
    let out = FolderScanOut {
        folder: folder.display().to_string(),
        games: games.iter().map(|g| scan_game(g, state)).collect(),
    };
    output::emit(&out, |out| {
        if out.games.is_empty() {
            println!("no games found in {}", out.folder);
            return;
        }
        println!("{} game(s) in {}:", out.games.len(), out.folder);
        println!();
        print!("{}", render_table(&out.games));
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(slug: &str, paths: &[&str]) -> ScanGame {
        ScanGame {
            slug: slug.into(),
            display_name: slug.into(),
            confidence: "high",
            paths: paths.iter().map(|p| (*p).to_string()).collect(),
            needs_folder: paths.is_empty(),
            tracked: false,
        }
    }

    /// A save path with a comma in it, a real one from a studio that put a comma
    /// in its name, has to come back out of the table whole.
    #[test]
    fn every_path_gets_its_own_line() {
        let comma = "/home/u/.config/unity3d/Cipher Prime Studios, Inc./Splice";
        let plain = "/home/u/.local/share/Splice/saves";
        let table = render_table(&[game("splice", &[comma, plain])]);

        let lines: Vec<&str> = table.lines().collect();
        assert_eq!(lines.len(), 3, "header, the row, the second path: {table}");
        assert!(lines[1].ends_with(comma), "{}", lines[1]);
        assert_eq!(lines[2].trim(), plain);
        // And nothing joined them, which is what made the old format
        // unsplittable: the comma in the line is the one inside the path.
        assert_eq!(lines[1].matches(", ").count(), 1);
    }

    /// A game with no folder says so where the paths would be, rather than
    /// leaving the column blank.
    #[test]
    fn a_game_without_a_folder_says_so_in_the_table() {
        let table = render_table(&[game("mojo-hanako", &[])]);
        assert!(table.contains("no save folder located"), "{table}");
    }

    /// The per-game list is a `--verbose` choice for a person and not negotiable
    /// for a machine: `--json` alone has to carry it.
    #[test]
    fn json_always_carries_the_games() {
        assert!(should_list_games(false, true), "--json alone");
        assert!(should_list_games(true, true));
        assert!(should_list_games(true, false), "--verbose alone");
        assert!(
            !should_list_games(false, false),
            "plain scan stays a summary"
        );
    }

    /// Later paths start in the column the PATHS header and the first path
    /// start in; `PATH_COLUMN` is computed by hand from the widths above it, so
    /// widening one of those columns without it would shift them off.
    #[test]
    fn later_paths_line_up_under_the_paths_header() {
        let table = render_table(&[game("splice", &["/a/first", "/b/second"])]);
        let lines: Vec<&str> = table.lines().collect();
        assert_eq!(lines[0].find("PATHS"), Some(PATH_COLUMN), "{table}");
        assert_eq!(lines[1].find("/a/first"), Some(PATH_COLUMN), "{table}");
        assert_eq!(lines[2].find("/b/second"), Some(PATH_COLUMN), "{table}");
        assert!(lines[2][..PATH_COLUMN].chars().all(|c| c == ' '));
    }
}
