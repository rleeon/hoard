//! CLI self-update check: what's the newest published release, and is this
//! binary behind it? Used by `hoard upgrade` (decide whether to run the
//! installer at all) and by the `hoard` status panel (paint the `cli` dot amber
//! when an update is available).
//!
//! The version comes from the GitHub "latest release" API, the same source the
//! `install.sh` / `install.ps1` one-liners resolve. Every network path here is
//! best-effort with a short timeout: a check that fails or times out must never
//! block the CLI, so callers treat `None` as "assume up to date".

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// GitHub repo the releases live under (matches `install.sh`'s `REPO`).
const REPO: &str = "rleeon/hoard";

/// How long a cached "latest version" answer is trusted before we re-check. Keeps
/// the status panel instant on repeated `hoard` runs and stays well under
/// GitHub's unauthenticated rate limit.
const CACHE_TTL: Duration = Duration::from_secs(6 * 60 * 60);

/// This binary's version (compile-time).
pub fn current() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// `(major, minor, patch)` from a version string, tolerant of a leading `v` and
/// a pre-release/build suffix on the patch (`1.0.4-rc1` → `(1,0,4)`).
fn parse(v: &str) -> Option<(u64, u64, u64)> {
    let v = v.trim().trim_start_matches('v');
    let mut it = v.split('.');
    let major = it.next()?.trim().parse().ok()?;
    let minor = it.next()?.trim().parse().ok()?;
    let patch_raw = it.next().unwrap_or("0");
    let digits: String = patch_raw
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    let patch = digits.parse().ok()?;
    Some((major, minor, patch))
}

/// True when `candidate` is a strictly newer semver than `base`. Unparseable
/// input → `false` (never nag on a version we can't compare).
pub fn is_newer(candidate: &str, base: &str) -> bool {
    match (parse(candidate), parse(base)) {
        (Some(a), Some(b)) => a > b,
        _ => false,
    }
}

/// Hit the GitHub API for the latest release tag (no cache). Best-effort; `None`
/// on any network/parse error or non-2xx. GitHub requires a User-Agent.
pub async fn fetch_latest() -> Option<String> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(2500))
        .user_agent(concat!("hoard-cli/", env!("CARGO_PKG_VERSION")))
        .build()
        .ok()?;
    let resp = client
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let json: serde_json::Value = resp.json().await.ok()?;
    let tag = json.get("tag_name")?.as_str()?;
    Some(tag.trim_start_matches('v').to_string())
}

#[derive(Serialize, Deserialize)]
struct Cache {
    latest: String,
    checked_at: u64,
}

fn cache_path() -> Option<std::path::PathBuf> {
    Some(
        crate::config::CliConfig::cache_dir()
            .ok()?
            .join("update-check.json"),
    )
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn read_cache() -> Option<Cache> {
    let txt = std::fs::read_to_string(cache_path()?).ok()?;
    serde_json::from_str(&txt).ok()
}

fn write_cache(latest: &str) {
    let Some(path) = cache_path() else { return };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let c = Cache {
        latest: latest.to_string(),
        checked_at: now_secs(),
    };
    if let Ok(txt) = serde_json::to_string(&c) {
        let _ = std::fs::write(path, txt);
    }
}

/// Latest version, served from the on-disk cache when it's fresh (< 6h) and
/// otherwise re-fetched and re-cached. Best-effort: it falls back to a stale cache
/// value if the refresh fails, or `None` if there's nothing to go on.
pub async fn cached_latest() -> Option<String> {
    if let Some(c) = read_cache() {
        if now_secs().saturating_sub(c.checked_at) < CACHE_TTL.as_secs() {
            return Some(c.latest);
        }
        // Stale: try to refresh, but keep the old answer if the network is down.
        return match fetch_latest().await {
            Some(v) => {
                write_cache(&v);
                Some(v)
            }
            None => Some(c.latest),
        };
    }
    let v = fetch_latest().await?;
    write_cache(&v);
    Some(v)
}

/// The newest version if it's ahead of this binary, else `None`. What the status
/// panel uses to decide the amber dot. Uses the cache, so it's instant on
/// repeated runs.
pub async fn available_update() -> Option<String> {
    let latest = cached_latest().await?;
    is_newer(&latest, current()).then_some(latest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newer_compares_semver_not_lexically() {
        assert!(is_newer("1.0.10", "1.0.9"));
        assert!(is_newer("1.1.0", "1.0.99"));
        assert!(is_newer("2.0.0", "1.9.9"));
        assert!(!is_newer("1.0.3", "1.0.3"));
        assert!(!is_newer("1.0.2", "1.0.3"));
    }

    #[test]
    fn tolerates_v_prefix_and_prerelease_suffix() {
        assert_eq!(parse("v1.0.4"), Some((1, 0, 4)));
        assert_eq!(parse("1.0.4-rc1"), Some((1, 0, 4)));
        assert!(is_newer("v1.0.4", "1.0.3"));
    }

    #[test]
    fn unparseable_never_nags() {
        assert!(!is_newer("garbage", "1.0.3"));
        assert!(!is_newer("1.0.4", "nightly"));
    }
}
