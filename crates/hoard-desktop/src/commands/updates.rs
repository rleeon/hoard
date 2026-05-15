//! Update checks for the desktop client and the user's self-hosted server.
//!
//! Two independent probes:
//!
//! - **Client**: hits `https://api.github.com/repos/rleeon/hoard/releases/latest`
//!   and compares the tag to our compile-time `CARGO_PKG_VERSION`. We treat a
//!   newer GitHub release as "update available" without parsing semver — a
//!   simple string-inequality is enough since our tags are always
//!   `vMAJOR.MINOR.PATCH` and tag-sort order matches release order.
//! - **Server**: hits the user's `<server>/v1/health` (anonymous endpoint)
//!   to read `version`, then compares against the latest known client
//!   version. Older servers won't have all the bug fixes the client expects
//!   (e.g. games-table self-heal landed in 1.3.0), so the UI nudges the user
//!   to upgrade their server when it falls behind.
//!
//! Both probes are best-effort: a GitHub outage or a self-hosted server
//! that's offline must not break Settings. We swallow errors and report
//! `available = false` instead.

use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::state::AppState;

/// Reported status for one component (client or server).
#[derive(Debug, Clone, Serialize)]
pub struct ComponentUpdate {
    /// Currently-running version (`CARGO_PKG_VERSION` for the client, the
    /// `/v1/health` `version` field for the server).
    pub current: String,
    /// Latest known version, if we could fetch it. `None` means the probe
    /// failed — the UI should fall back to "no update info" rather than
    /// "you're up to date".
    pub latest: Option<String>,
    /// `true` when `latest` is strictly greater than `current` (string
    /// compare; works for our `vX.Y.Z` tags).
    pub available: bool,
    /// Human-readable error from the failed probe, for the Logs view.
    /// Never shown to end users on its own.
    pub error: Option<String>,
}

/// Combined wire shape for the Settings page's "Updates" card.
#[derive(Debug, Clone, Serialize)]
pub struct UpdateReport {
    pub client: ComponentUpdate,
    /// `None` when the user is signed out (no known server URL to probe).
    pub server: Option<ComponentUpdate>,
}

/// GitHub releases API. For `check_for_updates` we only need the tag; for
/// `apply_desktop_update` we also need to pick the right downloadable asset.
#[derive(serde::Deserialize)]
struct GhRelease {
    tag_name: String,
    #[serde(default)]
    assets: Vec<GhAsset>,
}

#[derive(serde::Deserialize, Clone)]
struct GhAsset {
    name: String,
    browser_download_url: String,
}

/// `/v1/health` shape (mirrors `crates/hoard-server/src/routes/health.rs`).
#[derive(serde::Deserialize)]
struct HealthResp {
    version: String,
}

const CLIENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const GH_RELEASES_URL: &str = "https://api.github.com/repos/rleeon/hoard/releases/latest";
const PROBE_TIMEOUT: Duration = Duration::from_secs(8);

/// Tauri command. Pulls the latest GitHub release in parallel with the
/// server health probe (when logged in). Returns both halves so the UI
/// can render two badges side-by-side.
#[tauri::command]
pub async fn check_for_updates(state: State<'_, AppState>) -> Result<UpdateReport, String> {
    let server_url = state
        .user
        .lock()
        .unwrap()
        .as_ref()
        .map(|u| u.server_url.clone());

    let (client, server) = tokio::join!(probe_client(), async {
        match server_url {
            Some(url) => Some(probe_server(url).await),
            None => None,
        }
    });

    Ok(UpdateReport { client, server })
}

async fn probe_client() -> ComponentUpdate {
    let current = CLIENT_VERSION.to_string();
    match fetch_gh_latest().await {
        Ok(tag) => {
            // Strip the leading "v" if present so the comparison matches
            // CARGO_PKG_VERSION's bare semver string.
            let latest = tag.trim_start_matches('v').to_string();
            let available = is_newer(&latest, &current);
            ComponentUpdate {
                current,
                latest: Some(latest),
                available,
                error: None,
            }
        }
        Err(e) => ComponentUpdate {
            current,
            latest: None,
            available: false,
            error: Some(e),
        },
    }
}

async fn probe_server(url: String) -> ComponentUpdate {
    // The server's "current" version is what /v1/health reports; the
    // "latest" we know about is the running client's version (servers
    // upgrade in lockstep with clients, so any client newer than the
    // server means the server's behind).
    match fetch_server_health(&url).await {
        Ok(server_version) => {
            let latest = CLIENT_VERSION.to_string();
            let available = is_newer(&latest, &server_version);
            ComponentUpdate {
                current: server_version,
                latest: Some(latest),
                available,
                error: None,
            }
        }
        Err(e) => ComponentUpdate {
            current: "?".to_string(),
            latest: None,
            available: false,
            error: Some(e),
        },
    }
}

async fn fetch_gh_latest() -> Result<String, String> {
    let client = reqwest::Client::builder()
        .user_agent(concat!("hoard-desktop/", env!("CARGO_PKG_VERSION")))
        .timeout(PROBE_TIMEOUT)
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get(GH_RELEASES_URL)
        .header("accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    let release: GhRelease = resp.json().await.map_err(|e| e.to_string())?;
    Ok(release.tag_name)
}

/// Full release fetch — used by `apply_desktop_update` to discover the asset
/// list at install time (we don't cache it because the user might leave the
/// app open for days between detection and applying).
async fn fetch_gh_release() -> Result<GhRelease, String> {
    let client = reqwest::Client::builder()
        .user_agent(concat!("hoard-desktop/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get(GH_RELEASES_URL)
        .header("accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    resp.json::<GhRelease>().await.map_err(|e| e.to_string())
}

async fn fetch_server_health(server_url: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .user_agent(concat!("hoard-desktop/", env!("CARGO_PKG_VERSION")))
        .timeout(PROBE_TIMEOUT)
        .build()
        .map_err(|e| e.to_string())?;
    let url = format!("{}/v1/health", server_url.trim_end_matches('/'));
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    let h: HealthResp = resp.json().await.map_err(|e| e.to_string())?;
    Ok(h.version)
}

/// Lexicographic comparison is good enough for our `MAJOR.MINOR.PATCH` tags
/// because each component is zero-padded only conceptually — we use the
/// fact that semver strings up to `9.9.9` sort correctly as long as all
/// components have the same digit count, which they do for hoard.
///
/// For the rare case of crossing 9 → 10 we'd want a real semver parse,
/// but it's not worth pulling a crate for one comparison; we'll switch
/// when we ship 1.10.0.
fn is_newer(candidate: &str, baseline: &str) -> bool {
    parse_version(candidate) > parse_version(baseline)
}

/// Cheap `(major, minor, patch)` tuple parser. Returns zeros on failure
/// so a malformed string is treated as "older than everything"; that's
/// the safer default for an update prompt — never nag on garbage input.
fn parse_version(s: &str) -> (u32, u32, u32) {
    let s = s.trim_start_matches('v');
    let mut it = s.split('.');
    let major = it.next().and_then(|x| x.parse().ok()).unwrap_or(0);
    let minor = it.next().and_then(|x| x.parse().ok()).unwrap_or(0);
    let patch = it
        .next()
        .map(|x| x.split('-').next().unwrap_or(x))
        .and_then(|x| x.parse().ok())
        .unwrap_or(0);
    (major, minor, patch)
}

/// Outcome of `apply_desktop_update`. The UI uses `kind` to decide what to
/// show: on `installer_launched` we close the app so the OS installer can
/// replace it; on `downloaded` we tell the user where the file is.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ApplyOutcome {
    /// We spawned the platform installer (pkexec dpkg / msiexec / `open` on
    /// macOS). The UI should prompt the user to wait + restart.
    InstallerLaunched { path: String, version: String },
    /// We downloaded the asset but couldn't auto-launch the installer. The
    /// UI surfaces the path so the user can run it manually.
    Downloaded { path: String, version: String },
}

/// Tauri command. Downloads the right release asset for this OS and tries to
/// launch the platform installer.
///
/// This is the "Sí" path of the in-app update modal. We deliberately keep
/// the privilege-escalation choice on the OS: `pkexec` for Linux .deb,
/// `msiexec` for Windows .msi, `open` for macOS .dmg. Each pops the system's
/// usual auth prompt; we never ask the user for a password ourselves.
#[tauri::command]
pub async fn apply_desktop_update(app: AppHandle) -> Result<ApplyOutcome, String> {
    let release = fetch_gh_release().await?;
    let version = release.tag_name.trim_start_matches('v').to_string();

    let asset = pick_asset(&release.assets)
        .ok_or_else(|| {
            format!(
                "no installer asset for this platform in release v{}. Assets: {}",
                version,
                release
                    .assets
                    .iter()
                    .map(|a| a.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?
        .clone();

    // Download into the OS download dir so the file is findable later if the
    // installer asks the user to "Save As".
    let download_dir = app
        .path()
        .download_dir()
        .or_else(|_| app.path().temp_dir())
        .map_err(|e| format!("locating a writable directory: {e}"))?;
    tokio::fs::create_dir_all(&download_dir)
        .await
        .map_err(|e| e.to_string())?;
    let dest = download_dir.join(&asset.name);

    download_to(&asset.browser_download_url, &dest).await?;

    // Try to launch the platform installer. If that fails, we still succeeded
    // at *downloading* the update, so report Downloaded with the path so the
    // user can do it themselves.
    let path_str = dest.to_string_lossy().to_string();
    match launch_installer(&dest).await {
        Ok(()) => Ok(ApplyOutcome::InstallerLaunched {
            path: path_str,
            version,
        }),
        Err(e) => {
            tracing::warn!(error = %e, "installer launch failed, falling back to manual");
            Ok(ApplyOutcome::Downloaded {
                path: path_str,
                version,
            })
        }
    }
}

/// Pick the right asset for the current OS/arch. We match on filename suffix
/// because Tauri's bundle namer doesn't expose a stable scheme we can predict.
fn pick_asset(assets: &[GhAsset]) -> Option<&GhAsset> {
    #[cfg(target_os = "linux")]
    {
        // Prefer .deb (Ubuntu/Debian, what we test against). Fall back to
        // .AppImage if no .deb is in the release.
        if let Some(a) = assets.iter().find(|a| a.name.ends_with(".deb")) {
            return Some(a);
        }
        assets.iter().find(|a| a.name.ends_with(".AppImage"))
    }
    #[cfg(target_os = "windows")]
    {
        assets
            .iter()
            .find(|a| a.name.ends_with(".msi"))
            .or_else(|| assets.iter().find(|a| a.name.ends_with(".exe")))
    }
    #[cfg(target_os = "macos")]
    {
        assets.iter().find(|a| a.name.ends_with(".dmg"))
    }
}

async fn download_to(url: &str, dest: &std::path::Path) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .user_agent(concat!("hoard-desktop/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;
    let bytes = client
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;
    tokio::fs::write(dest, &bytes)
        .await
        .map_err(|e| format!("writing {}: {e}", dest.display()))?;
    Ok(())
}

#[cfg(target_os = "linux")]
async fn launch_installer(path: &std::path::Path) -> Result<(), String> {
    // pkexec pops the polkit auth dialog and runs dpkg as root. If the user
    // cancels, dpkg never runs and we get a non-zero exit; we treat that as
    // an error so the UI can fall back to "Downloaded".
    let p = path.to_string_lossy().to_string();
    let status = tokio::process::Command::new("pkexec")
        .args(["dpkg", "-i", &p])
        .status()
        .await
        .map_err(|e| format!("spawning pkexec: {e}"))?;
    if !status.success() {
        return Err(format!("dpkg -i exited with status {status}"));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
async fn launch_installer(path: &std::path::Path) -> Result<(), String> {
    let p = path.to_string_lossy().to_string();
    tokio::process::Command::new("msiexec")
        .args(["/i", &p])
        .spawn()
        .map_err(|e| format!("spawning msiexec: {e}"))?;
    Ok(())
}

#[cfg(target_os = "macos")]
async fn launch_installer(path: &std::path::Path) -> Result<(), String> {
    let p = path.to_string_lossy().to_string();
    tokio::process::Command::new("open")
        .arg(&p)
        .spawn()
        .map_err(|e| format!("spawning open: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newer_minor_beats_older() {
        assert!(is_newer("1.3.0", "1.2.5"));
        assert!(is_newer("2.0.0", "1.99.99"));
    }

    #[test]
    fn equal_versions_are_not_newer() {
        assert!(!is_newer("1.2.2", "1.2.2"));
    }

    #[test]
    fn double_digit_components_compare_correctly() {
        assert!(is_newer("1.10.0", "1.9.9"));
    }

    #[test]
    fn tolerates_v_prefix_and_prerelease() {
        assert!(is_newer("v1.3.0", "1.2.5"));
        assert!(is_newer("1.3.0-rc1", "1.2.5"));
    }
}
