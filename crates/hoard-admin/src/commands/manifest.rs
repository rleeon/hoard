//! `hoard-admin manifest import`: pulls the Ludusavi public manifest into the
//! games catalog.
//!
//! The Ludusavi manifest is a community-maintained YAML file mapping game
//! display names to per-OS save paths. We download (or read from disk),
//! parse, and upsert each entry into the `games` table without clobbering
//! manually-added rows.
//!
//! See <https://github.com/mtkennerly/ludusavi-manifest>.

use anyhow::{Context, Result};
use clap::Subcommand;
use hoard_server::{config::Config, db};
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::BTreeMap;
use std::path::PathBuf;

const DEFAULT_URL: &str =
    "https://raw.githubusercontent.com/mtkennerly/ludusavi-manifest/master/data/manifest.yaml";

#[derive(Subcommand)]
pub enum ManifestCommand {
    /// Import or refresh the games catalog from the Ludusavi manifest.
    Import {
        /// URL to download the manifest YAML from (default: official upstream).
        #[arg(long, conflicts_with = "from_file")]
        from_url: Option<String>,
        /// Read the YAML from a local file instead of downloading.
        #[arg(long)]
        from_file: Option<PathBuf>,
        /// Delete games previously imported from the manifest that no longer
        /// appear in the new file. Hand-added games are never pruned.
        #[arg(long)]
        prune: bool,
        /// Print what would change without writing to the database.
        #[arg(long)]
        dry_run: bool,
        /// Manifest version string to record (e.g. a git SHA or release tag).
        /// Defaults to the current UTC date.
        #[arg(long)]
        version: Option<String>,
    },
}

pub async fn run(cmd: ManifestCommand, cfg: &Config) -> Result<()> {
    match cmd {
        ManifestCommand::Import {
            from_url,
            from_file,
            prune,
            dry_run,
            version,
        } => {
            let pool = db::connect(&cfg.database.url, cfg.database.max_connections).await?;
            db::run_migrations(&pool).await?;

            let raw = load_yaml(from_url, from_file).await?;
            let manifest_version = version.unwrap_or_else(|| {
                time::OffsetDateTime::now_utc()
                    .format(&time::format_description::well_known::Iso8601::DEFAULT)
                    .unwrap_or_else(|_| "unknown".into())
            });

            do_import(&pool, &raw, &manifest_version, prune, dry_run).await
        }
    }
}

// ---- IO ----------------------------------------------------------------

async fn load_yaml(url: Option<String>, file: Option<PathBuf>) -> Result<String> {
    if let Some(path) = file {
        return tokio::fs::read_to_string(&path)
            .await
            .with_context(|| format!("reading {}", path.display()));
    }
    let url = url.unwrap_or_else(|| DEFAULT_URL.to_string());
    println!("Downloading manifest from {url}");
    let resp = reqwest::Client::builder()
        .user_agent(concat!("hoard-admin/", env!("CARGO_PKG_VERSION")))
        .build()?
        .get(&url)
        .send()
        .await
        .with_context(|| format!("requesting {url}"))?
        .error_for_status()?;
    Ok(resp.text().await?)
}

// ---- Ludusavi schema (subset) -----------------------------------------

#[derive(Debug, Deserialize)]
struct LudusaviManifest(BTreeMap<String, LudusaviEntry>);

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct LudusaviEntry {
    /// If set, this row is a pointer to another entry, so skip it on import.
    alias: Option<String>,
    files: BTreeMap<String, LudusaviPath>,
    steam: Option<LudusaviSteamRef>,
    gog: Option<LudusaviStoreRef>,
    cloud: Option<LudusaviCloud>,
    notes: Option<String>,
}

/// `cloud:`, which storefronts sync this game themselves. Distinct from
/// `steam:`/`gog:`, which only say the game is *sold* there. Deriving the
/// former from the latter is what made the `games` table claim Steam Cloud
/// for every game with an appid.
#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct LudusaviCloud {
    steam: bool,
    gog: bool,
}

impl LudusaviEntry {
    /// `(steam, gog)`: whether each storefront syncs this game itself. A
    /// missing `cloud:` block means neither does.
    fn cloud_flags(&self) -> (bool, bool) {
        self.cloud
            .as_ref()
            .map_or((false, false), |c| (c.steam, c.gog))
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct LudusaviPath {
    tags: Vec<String>,
    when: Vec<LudusaviWhen>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct LudusaviWhen {
    os: Option<String>,
    store: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LudusaviSteamRef {
    id: u64,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // we only key on presence today
struct LudusaviStoreRef {
    id: serde_yaml::Value,
}

// ---- Our normalised shape (also lives in hoard-agent) -----------------

/// A save path entry annotated with constraints, suitable for storing as
/// JSON in the `games.save_paths_json` column.
#[derive(Debug, Clone, Serialize)]
struct SavePathOut {
    path: String,
    constraints: Vec<ConstraintOut>,
    tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ConstraintOut {
    #[serde(skip_serializing_if = "Option::is_none")]
    store: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
struct PathsByOs {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    windows: Vec<SavePathOut>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    linux: Vec<SavePathOut>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    mac: Vec<SavePathOut>,
}

impl PathsByOs {
    fn is_empty(&self) -> bool {
        self.windows.is_empty() && self.linux.is_empty() && self.mac.is_empty()
    }
}

// ---- Import logic ------------------------------------------------------

async fn do_import(
    pool: &SqlitePool,
    raw: &str,
    manifest_version: &str,
    prune: bool,
    dry_run: bool,
) -> Result<()> {
    let manifest: LudusaviManifest = serde_yaml::from_str(raw).context("parsing manifest YAML")?;
    println!("Parsed {} entries from manifest", manifest.0.len());

    let mut inserted = 0i64;
    let mut updated = 0i64;
    let mut skipped_alias = 0i64;
    let mut skipped_no_paths = 0i64;
    let mut seen_slugs: Vec<String> = Vec::with_capacity(manifest.0.len());

    let pb = ProgressBar::new(manifest.0.len() as u64);
    pb.set_style(
        ProgressStyle::with_template("{spinner} [{bar:30}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("=> "),
    );

    let mut tx = pool.begin().await?;

    for (display_name, entry) in &manifest.0 {
        pb.inc(1);
        if entry.alias.is_some() {
            skipped_alias += 1;
            continue;
        }
        let paths = transform_paths(&entry.files);
        if paths.is_empty() {
            skipped_no_paths += 1;
            continue;
        }
        let slug = slugify(display_name);
        seen_slugs.push(slug.clone());

        let paths_json = serde_json::to_string(&paths)?;
        let (has_steam_cloud, has_gog_cloud) = entry.cloud_flags();
        let cloud_steam = i64::from(has_steam_cloud);
        let cloud_gog = i64::from(has_gog_cloud);
        let steam_app_id = entry.steam.as_ref().map(|s| s.id as i64);

        if dry_run {
            // Don't write; just count by checking if the row exists.
            let exists: Option<String> =
                sqlx::query_scalar("SELECT slug FROM games WHERE slug = ?")
                    .bind(&slug)
                    .fetch_optional(&mut *tx)
                    .await?;
            if exists.is_some() {
                updated += 1;
            } else {
                inserted += 1;
            }
            continue;
        }

        // Preserve hand-added rows: only mass-update fields if the row is
        // either new or already imported_from='ludusavi-manifest'. If it's
        // 'manual', leave it alone (admins may have customised it).
        let existing: Option<(String,)> =
            sqlx::query_as("SELECT imported_from FROM games WHERE slug = ?")
                .bind(&slug)
                .fetch_optional(&mut *tx)
                .await?;

        match existing {
            None => {
                sqlx::query(
                    "INSERT INTO games (
                        slug, display_name, save_paths_json,
                        notes, cloud_steam, cloud_gog, steam_app_id,
                        imported_from, manifest_version, imported_at
                     ) VALUES (?, ?, ?, ?, ?, ?, ?, 'ludusavi-manifest', ?, strftime('%Y-%m-%dT%H:%M:%SZ','now'))",
                )
                .bind(&slug)
                .bind(display_name)
                .bind(&paths_json)
                .bind(&entry.notes)
                .bind(cloud_steam)
                .bind(cloud_gog)
                .bind(steam_app_id)
                .bind(manifest_version)
                .execute(&mut *tx)
                .await?;
                inserted += 1;
            }
            Some((source,)) if source == "ludusavi-manifest" => {
                sqlx::query(
                    "UPDATE games SET
                        display_name = ?, save_paths_json = ?,
                        notes = ?, cloud_steam = ?, cloud_gog = ?, steam_app_id = ?,
                        manifest_version = ?,
                        imported_at = strftime('%Y-%m-%dT%H:%M:%SZ','now')
                     WHERE slug = ?",
                )
                .bind(display_name)
                .bind(&paths_json)
                .bind(&entry.notes)
                .bind(cloud_steam)
                .bind(cloud_gog)
                .bind(steam_app_id)
                .bind(manifest_version)
                .bind(&slug)
                .execute(&mut *tx)
                .await?;
                updated += 1;
            }
            Some((_source_manual,)) => {
                // Admin-customised row; leave alone.
            }
        }
    }
    pb.finish_and_clear();

    let mut pruned = 0i64;
    if prune && !dry_run {
        // Build a NOT IN list. SQLite has a limit (~999 placeholders), so we
        // chunk. Hand-added rows are excluded by `imported_from`.
        let mut to_keep: std::collections::HashSet<String> = seen_slugs.iter().cloned().collect();
        let all_imported: Vec<(String,)> =
            sqlx::query_as("SELECT slug FROM games WHERE imported_from = 'ludusavi-manifest'")
                .fetch_all(&mut *tx)
                .await?;
        let stale: Vec<String> = all_imported
            .into_iter()
            .filter_map(|(s,)| if !to_keep.remove(&s) { Some(s) } else { None })
            .collect();
        for slug in &stale {
            // Skip if there are saves attached: never lose user data silently.
            let saves: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM saves WHERE game_slug = ?")
                .bind(slug)
                .fetch_one(&mut *tx)
                .await?;
            if saves > 0 {
                continue;
            }
            sqlx::query("DELETE FROM games WHERE slug = ? AND imported_from = 'ludusavi-manifest'")
                .bind(slug)
                .execute(&mut *tx)
                .await?;
            pruned += 1;
        }
    }

    if !dry_run {
        sqlx::query(
            "INSERT INTO manifest_imports (source, manifest_version, games_inserted, games_updated, games_pruned)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind("ludusavi-manifest")
        .bind(manifest_version)
        .bind(inserted)
        .bind(updated)
        .bind(pruned)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
    } else {
        tx.rollback().await?;
    }

    println!();
    println!(
        "Manifest import {}:",
        if dry_run { "(dry-run)" } else { "complete" }
    );
    println!("  inserted:        {inserted}");
    println!("  updated:         {updated}");
    println!("  pruned:          {pruned}");
    println!("  skipped (alias): {skipped_alias}");
    println!("  skipped (no paths): {skipped_no_paths}");
    if dry_run {
        println!("\nNo changes were written. Drop --dry-run to apply.");
    }
    Ok(())
}

// ---- Helpers -----------------------------------------------------------

/// Convert a Ludusavi `files` map into our per-OS structure.
fn transform_paths(files: &BTreeMap<String, LudusaviPath>) -> PathsByOs {
    let mut out = PathsByOs::default();

    for (path_template, p) in files {
        // Skip registry-only and save-state-less things: only keep entries
        // tagged "save". (`tags` empty also gets through; many Ludusavi
        // entries lack tags but are clearly saves.)
        if !p.tags.is_empty() && !p.tags.iter().any(|t| t == "save") {
            continue;
        }

        // No `when` clause → applies to every OS.
        let oses = if p.when.is_empty() {
            vec!["windows", "linux", "mac"]
        } else {
            let mut v: Vec<&str> = p
                .when
                .iter()
                .filter_map(|w| w.os.as_deref())
                .filter_map(normalise_os)
                .collect();
            // If `when` is set but no os entry was usable, treat as "all".
            if v.is_empty() {
                v = vec!["windows", "linux", "mac"];
            }
            v.sort();
            v.dedup();
            v
        };

        let constraints: Vec<ConstraintOut> = if p.when.is_empty() {
            vec![ConstraintOut { store: None }]
        } else {
            p.when
                .iter()
                .map(|w| ConstraintOut {
                    store: w.store.clone(),
                })
                .collect()
        };

        let tags = if p.tags.is_empty() {
            vec!["save".into()]
        } else {
            p.tags.clone()
        };

        let entry = SavePathOut {
            path: path_template.clone(),
            constraints,
            tags,
        };

        for os in oses {
            match os {
                "windows" => out.windows.push(entry.clone()),
                "linux" => out.linux.push(entry.clone()),
                "mac" => out.mac.push(entry.clone()),
                _ => {}
            }
        }
    }

    out
}

fn normalise_os(s: &str) -> Option<&'static str> {
    match s.to_ascii_lowercase().as_str() {
        "windows" | "win" => Some("windows"),
        "linux" => Some("linux"),
        "mac" | "macos" | "osx" | "darwin" => Some("mac"),
        _ => None,
    }
}

/// Convert a Ludusavi display name into our kebab-case slug. Lowercases,
/// replaces every non-alphanumeric run with `-`, trims leading/trailing
/// dashes, and clamps the length to keep DB rows reasonable.
fn slugify(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut last_was_dash = true;
    for ch in name.chars() {
        let c = ch.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
            last_was_dash = false;
        } else if !last_was_dash {
            out.push('-');
            last_was_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        out.push_str("game");
    }
    if out.len() > 96 {
        out.truncate(96);
        while out.ends_with('-') {
            out.pop();
        }
    }
    if !out.starts_with(|c: char| c.is_ascii_alphanumeric()) {
        out.insert(0, 'g');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("Stardew Valley"), "stardew-valley");
        assert_eq!(
            slugify("The Witcher 3: Wild Hunt"),
            "the-witcher-3-wild-hunt"
        );
        assert_eq!(slugify("  spaced  "), "spaced");
        assert_eq!(slugify(""), "game");
        assert_eq!(slugify("123 Numbers"), "123-numbers");
    }

    #[test]
    fn slugify_handles_unicode_and_punct() {
        assert_eq!(slugify("Pokémon: Yellow!"), "pok-mon-yellow");
        assert_eq!(slugify("---"), "game");
    }

    #[test]
    fn transform_simple_path_no_when() {
        let mut files = BTreeMap::new();
        files.insert(
            "<winAppData>/StardewValley/Saves".into(),
            LudusaviPath {
                tags: vec!["save".into()],
                when: vec![],
            },
        );
        let out = transform_paths(&files);
        assert_eq!(out.windows.len(), 1);
        assert_eq!(out.linux.len(), 1);
        assert_eq!(out.mac.len(), 1);
        assert_eq!(out.windows[0].tags, vec!["save"]);
    }

    #[test]
    fn transform_filters_to_specific_os() {
        let mut files = BTreeMap::new();
        files.insert(
            "<winAppData>/X".into(),
            LudusaviPath {
                tags: vec!["save".into()],
                when: vec![LudusaviWhen {
                    os: Some("windows".into()),
                    store: Some("any".into()),
                }],
            },
        );
        let out = transform_paths(&files);
        assert_eq!(out.windows.len(), 1);
        assert!(out.linux.is_empty());
        assert!(out.mac.is_empty());
    }

    #[test]
    fn cloud_flags_come_from_the_cloud_block_not_the_store_id() {
        // Sold on Steam, synced by nobody: the row the old derivation got
        // wrong, and the one the restore path would have to trust.
        let yaml = "
Some Game:
  steam:
    id: 1245620
  gog:
    id: 1207666283
";
        let m: LudusaviManifest = serde_yaml::from_str(yaml).unwrap();
        let entry = &m.0["Some Game"];
        assert!(entry.steam.is_some());
        assert!(entry.gog.is_some());
        assert_eq!(entry.cloud_flags(), (false, false));

        let yaml = "
Cloudy Game:
  steam:
    id: 1245620
  cloud:
    steam: true
";
        let m: LudusaviManifest = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(m.0["Cloudy Game"].cloud_flags(), (true, false));
    }

    #[test]
    fn transform_drops_non_save_tags() {
        let mut files = BTreeMap::new();
        files.insert(
            "<winAppData>/X".into(),
            LudusaviPath {
                tags: vec!["config".into()],
                when: vec![],
            },
        );
        let out = transform_paths(&files);
        assert!(out.is_empty());
    }
}
