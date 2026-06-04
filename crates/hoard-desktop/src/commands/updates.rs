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

use crate::commands::error::AppError;
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
pub async fn apply_desktop_update(app: AppHandle) -> Result<ApplyOutcome, AppError> {
    // Network/HTTP failures fetching the release feed: surface as "unknown",
    // since this branch fires before we even know what version we're trying
    // to install. The raw `reqwest` message goes into `detail`.
    let release = fetch_gh_release().await.map_err(|e| {
        AppError::new("updates.error.title", "updates.error.unknown").with_detail(e)
    })?;
    let version = release.tag_name.trim_start_matches('v').to_string();

    let asset = pick_asset(&release.assets)
        .ok_or_else(|| {
            let assets = release
                .assets
                .iter()
                .map(|a| a.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            AppError::new("updates.error.title", "updates.error.no_installer")
                .with_detail(format!("v{version} · Assets: {assets}"))
        })?
        .clone();

    // Download into the OS download dir so the file is findable later if the
    // installer asks the user to "Save As".
    let download_dir = app
        .path()
        .download_dir()
        .or_else(|_| app.path().temp_dir())
        .map_err(|e| {
            AppError::new("updates.error.title", "updates.error.download_failed")
                .with_detail(format!("locating a writable directory: {e}"))
        })?;
    tokio::fs::create_dir_all(&download_dir)
        .await
        .map_err(|e| {
            AppError::new("updates.error.title", "updates.error.download_failed")
                .with_detail(e.to_string())
        })?;
    let dest = download_dir.join(&asset.name);

    download_to(&asset.browser_download_url, &dest)
        .await
        .map_err(|e| {
            AppError::new("updates.error.title", "updates.error.download_failed").with_detail(e)
        })?;

    // Capture our own binary path *before* the installer runs. On Linux the
    // .deb install makes dpkg unlink+recreate /usr/bin/hoard-desktop, after
    // which `std::env::current_exe()` resolves to ".../hoard-desktop (deleted)"
    // — spawning that path fails and the relaunch silently no-ops, which is the
    // "tengo que cerrar y abrir para que cambie de versión" bug. Snapshotting
    // the path here (and sanitizing a stale " (deleted)" suffix just in case)
    // gives us a stable path to the freshly-installed binary.
    let exe_before = std::env::current_exe().ok().map(sanitize_exe_path);

    // Try to launch the platform installer. If that fails, we still succeeded
    // at *downloading* the update, so report Downloaded with the path so the
    // user can do it themselves.
    let path_str = dest.to_string_lossy().to_string();
    match launch_installer(&dest).await {
        Ok(()) => {
            // Quit the old process shortly after we return. Without this the
            // user is stuck running 1.3.5 even though dpkg/msiexec already
            // dropped 1.4.0 on disk — that's exactly what the 1.3.5 → 1.4.0
            // upgrade looked like in the wild on Linux ("se queda tan
            // pancho") and is what makes Windows refuse to overlay the .exe
            // until the running copy goes away. The small delay lets the
            // frontend paint the "installed, reopen" toast before we cut the
            // process. On Linux we additionally relaunch the freshly-
            // installed binary so the user doesn't have to dig around the
            // app menu — the .deb sits at the same /usr/bin path so
            // `current_exe()` already points at the new binary, and
            // `setsid` detaches it from our dying process group.
            let app2 = app.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(Duration::from_millis(1500)).await;
                relaunch_then_exit(&app2, exe_before);
            });
            Ok(ApplyOutcome::InstallerLaunched {
                path: path_str,
                version,
            })
        }
        Err(e) => {
            tracing::warn!(error = %e, "installer launch failed, falling back to manual");
            Ok(ApplyOutcome::Downloaded {
                path: path_str,
                version,
            })
        }
    }
}

/// Try to spawn the new binary detached from the current process, then exit.
/// Failures are logged and swallowed — the worst case is the user has to
/// reopen the app from their app menu, which is exactly the 1.4.0 status
/// quo we're trying to improve.
fn relaunch_then_exit(app: &AppHandle, exe_before: Option<std::path::PathBuf>) {
    #[cfg(target_os = "linux")]
    {
        // Prefer the path we snapshotted before the install. Fall back to a
        // freshly-sanitized `current_exe()` if we somehow didn't capture one.
        let exe = exe_before.or_else(|| std::env::current_exe().ok().map(sanitize_exe_path));
        if let Some(exe) = exe {
            use std::process::{Command, Stdio};
            // `setsid` puts the child in its own session so it survives our
            // imminent `app.exit(0)`. If `setsid` isn't on PATH (unusual on
            // any modern Linux) we just skip the relaunch — exit is still
            // useful on its own because the old process was blocking the
            // user from running the new binary anyway.
            let spawn = Command::new("setsid")
                .arg(&exe)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
            if let Err(e) = spawn {
                tracing::warn!(error = %e, exe = %exe.display(), "couldn't relaunch with setsid; user will have to reopen manually");
            }
        }
    }
    // On Windows we *don't* relaunch: msiexec is still running asynchronously
    // and the .exe is mid-replace. Starting a process from the path that's
    // being overwritten races msiexec and usually crashes. The user reopens
    // from the Start menu after the installer finishes.
    //
    // On macOS we leave it to the user too — `open` on the .dmg pops Finder
    // and the user drags into Applications.
    app.exit(0);
}

/// Strip a trailing " (deleted)" marker the Linux kernel appends to
/// `/proc/self/exe` once the on-disk binary has been unlinked (e.g. by dpkg
/// mid-upgrade). `current_exe()` reads that link, so without this we'd try to
/// spawn ".../hoard-desktop (deleted)" and fail.
fn sanitize_exe_path(exe: std::path::PathBuf) -> std::path::PathBuf {
    if let Some(s) = exe.to_str() {
        if let Some(stripped) = s.strip_suffix(" (deleted)") {
            return std::path::PathBuf::from(stripped);
        }
    }
    exe
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

// ---------------------------------------------------------------------------
// Server upgrade — in-app, for local servers only
// ---------------------------------------------------------------------------
//
// Why this exists: pre-1.4.7 the Server card in Settings only knew how to
// *copy* `sudo hoard-server upgrade` to the clipboard. That's a sensible
// fallback when the server is on another box (you have to SSH anyway), but
// for the common self-hosted case where the server runs on the same machine
// as the desktop app it's busywork — the user already has both binaries on
// disk and just wants a button.
//
// We piggy-back on the same `pkexec` flow the desktop installer uses: one
// graphical sudo prompt, no password handling in our own code, no need to
// stash a sudo password in prefs. The wrapper invocation runs the upgrade
// *and* restarts the service in a single auth prompt, so the user doesn't
// have to authenticate twice. If the upgrade succeeds but the restart
// fails (no systemd unit, manual launch, etc.) we still surface success
// with a "restart it yourself" hint.
//
// Gated to Linux because:
//   - `hoard-server upgrade` itself is Linux-x86_64 only (see
//     `crates/hoard-server/src/upgrade.rs::preferred_asset_name`).
//   - `pkexec` is the Linux polkit entry point; Windows/macOS would
//     need entirely different privilege-escalation plumbing.
// On any other platform `apply_server_update` returns the same error
// regardless of `is_local_server`, so the UI falls back to copy-command.

/// Outcome of `apply_server_update`. The UI uses `kind` to decide what to
/// show next: on `upgraded_and_restarted` the panel can flip straight to
/// "up to date"; on `upgraded` it tells the user to restart manually.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ServerUpgradeOutcome {
    /// The new binary is on disk *and* `systemctl restart hoard-server`
    /// returned 0. The user can keep using the app immediately.
    UpgradedAndRestarted { output: String },
    /// The new binary is on disk but the service restart didn't succeed —
    /// usually because the server isn't running under systemd. The user
    /// needs to restart it themselves with whatever supervisor they use.
    Upgraded {
        output: String,
        restart_error: String,
    },
}

/// Tauri command. Runs `hoard-server upgrade` (and `systemctl restart`)
/// through a single `pkexec` invocation so the user only authenticates
/// once. Only available on Linux; only meaningful when the server is on
/// the same machine — the UI is responsible for gating the button on
/// `is_local_server`, but we don't *require* it because the call simply
/// fails on a non-local box (the local `hoard-server` binary either
/// doesn't exist, or it does and the user is fine to upgrade their copy).
#[tauri::command]
pub async fn apply_server_update() -> Result<ServerUpgradeOutcome, AppError> {
    apply_server_update_impl().await
}

#[cfg(target_os = "linux")]
async fn apply_server_update_impl() -> Result<ServerUpgradeOutcome, AppError> {
    // We launch a shell so we can chain "upgrade && restart" into one
    // pkexec → one polkit prompt. `set -e` makes the script fail fast if
    // the upgrade itself returns non-zero (download failed, binary path
    // not writable, etc.) so we don't restart a half-upgraded server.
    //
    // We capture stdout+stderr so the UI can surface the friendly progress
    // output from `hoard-server upgrade` ("downloading", "Installed
    // hoard-server v…"). On Linux pkexec returns exit code 126 if the user
    // cancels the auth prompt and 127 if the underlying binary isn't found;
    // we surface both as a single human-readable error string.
    let upgrade = tokio::process::Command::new("pkexec")
        .args([
            "sh",
            "-c",
            // Resolve `hoard-server` through PATH inside the elevated
            // shell — polkit doesn't preserve the caller's PATH for the
            // binary it spawns, but `sh` does for *its* exec, and `sh`'s
            // default PATH includes the standard system bin dirs we ship to.
            "set -e; hoard-server upgrade",
        ])
        .output()
        .await
        .map_err(|e| {
            AppError::new("updates.error.title", "updates.error.server_pkexec_failed")
                .with_detail(format!("spawning pkexec: {e}"))
        })?;

    if !upgrade.status.success() {
        let stderr = String::from_utf8_lossy(&upgrade.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&upgrade.stdout).trim().to_string();
        // pkexec emits its own message for "Request dismissed" / "Not
        // authorized"; we pass that through verbatim so the UI can show
        // the user-facing reason rather than a generic "upgrade failed".
        let detail = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            format!("pkexec exited with status {}", upgrade.status)
        };
        return Err(
            AppError::new("updates.error.title", "updates.error.server_pkexec_failed")
                .with_detail(detail),
        );
    }

    let upgrade_output = String::from_utf8_lossy(&upgrade.stdout).trim().to_string();
    tracing::info!(?upgrade_output, "server upgrade pkexec call returned 0");

    // Try the restart in a second pkexec call. Polkit caches the auth
    // grant for ~5 minutes by default, so this typically won't re-prompt
    // — but on a stricter polkit policy the user *might* see a second
    // prompt. We accept that as a price for keeping the chain trivial to
    // reason about (one pkexec per privileged op, no shell escapes).
    let restart = tokio::process::Command::new("pkexec")
        .args(["systemctl", "restart", "hoard-server"])
        .output()
        .await;

    match restart {
        Ok(out) if out.status.success() => Ok(ServerUpgradeOutcome::UpgradedAndRestarted {
            output: upgrade_output,
        }),
        Ok(out) => {
            let detail = String::from_utf8_lossy(&out.stderr).trim().to_string();
            let detail = if detail.is_empty() {
                format!("systemctl exited with status {}", out.status)
            } else {
                detail
            };
            tracing::warn!(error = %detail, "server upgraded but restart failed");
            Ok(ServerUpgradeOutcome::Upgraded {
                output: upgrade_output,
                restart_error: detail,
            })
        }
        Err(e) => Ok(ServerUpgradeOutcome::Upgraded {
            output: upgrade_output,
            restart_error: format!("spawning systemctl restart: {e}"),
        }),
    }
}

#[cfg(not(target_os = "linux"))]
async fn apply_server_update_impl() -> Result<ServerUpgradeOutcome, AppError> {
    Err(
        AppError::new("updates.error.title", "updates.error.server_unknown").with_detail(
            "In-app server upgrade is only supported on Linux today. \
             Run `sudo hoard-server upgrade` on the server host."
                .to_string(),
        ),
    )
}

// ---------------------------------------------------------------------------
// Remote-triggered server upgrade — works from any OS, any machine (ADR 0017)
// ---------------------------------------------------------------------------
//
// Unlike `apply_server_update` (which runs `pkexec hoard-server upgrade` on
// *this* machine and therefore only helps when the server shares the box),
// this asks the server to upgrade *itself* over HTTP. The desktop can run on
// Windows and still upgrade a headless Linux server across the network.
//
// The server-side handler (`POST /v1/admin/upgrade`) only drops a marker file;
// a root systemd oneshot does the signed binary swap + restart. So all we do
// here is fire the authenticated request and then poll `/v1/health` until the
// reported version changes (the server is briefly unreachable mid-restart,
// which we tolerate) or a timeout elapses.

/// Outcome of `trigger_server_upgrade`.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RemoteUpgradeOutcome {
    /// The server came back on a new version within the poll window.
    Confirmed { version: String },
    /// The request was accepted (202) but we couldn't confirm the new
    /// version before the timeout — the server may still be restarting, or
    /// it was already on the latest signed release. The UI tells the user to
    /// re-check in a moment rather than treating this as a failure.
    Scheduled,
}

/// Tauri command. Sends `POST {server_url}/v1/admin/upgrade` with the saved
/// bearer token, then polls `/v1/health` for a version change. Requires an
/// admin token (the server returns 403 otherwise); the UI gates the button
/// on `is_admin` so this is a defence-in-depth check, not the primary one.
#[tauri::command]
pub async fn trigger_server_upgrade(
    state: State<'_, AppState>,
) -> Result<RemoteUpgradeOutcome, AppError> {
    let server_url = state
        .user
        .lock()
        .unwrap()
        .as_ref()
        .map(|u| u.server_url.clone())
        .ok_or_else(|| {
            AppError::new("updates.error.title", "updates.error.server_not_logged_in")
        })?;
    let token = hoard_agent::credentials::load()
        .ok()
        .flatten()
        .map(|c| c.token)
        .ok_or_else(|| {
            AppError::new("updates.error.title", "updates.error.server_not_logged_in")
        })?;

    let base = server_url.trim_end_matches('/').to_string();
    let client = reqwest::Client::builder()
        .user_agent(concat!("hoard-desktop/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| {
            AppError::new("updates.error.title", "updates.error.unknown").with_detail(e.to_string())
        })?;

    // Snapshot the current version so we can detect the flip after restart.
    let before = fetch_server_health(&base).await.ok();

    let resp = client
        .post(format!("{base}/v1/admin/upgrade"))
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|e| {
            AppError::new("updates.error.title", "updates.error.server_unreachable")
                .with_detail(e.to_string())
        })?;

    let status = resp.status();
    if status == reqwest::StatusCode::FORBIDDEN {
        let detail = resp.text().await.unwrap_or_default();
        return Err(
            AppError::new("updates.error.title", "updates.error.server_forbidden")
                .with_detail(detail),
        );
    }
    if !status.is_success() {
        let detail = resp.text().await.unwrap_or_default();
        return Err(
            AppError::new("updates.error.title", "updates.error.server_unknown")
                .with_detail(format!("HTTP {status}: {detail}")),
        );
    }

    // Poll /v1/health for ~90s (30 × 3s). The server is unreachable for a few
    // seconds while systemd restarts it; we ignore probe errors and keep going.
    for _ in 0..30 {
        tokio::time::sleep(Duration::from_secs(3)).await;
        if let Ok(version) = fetch_server_health(&base).await {
            if before.as_deref() != Some(version.as_str()) {
                return Ok(RemoteUpgradeOutcome::Confirmed { version });
            }
        }
    }

    Ok(RemoteUpgradeOutcome::Scheduled)
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
