//! Bare `hoard`: instead of dumping clap's help, it paints a fastfetch-style
//! panel, with the H logo in emerald green on the left and, on the right, the
//! name, version and on/off status of each area (cli, desktop, server, session,
//! daemon). Below that, the command cheat-sheet. It only touches the network for
//! the server: a short probe to the `/v1/health` of whatever server you connect
//! to (Cloud or self-host), so the version is meaningful even when it runs in
//! Docker or on another machine. Everything else is local.

use std::io::IsTerminal;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;

use hoard_agent::api::ApiClient;
use hoard_agent::config::CliConfig;

/// Block "H" logo (8 rows, 10 columns). Fixed width so the right-hand info
/// column lines up.
const LOGO: [&str; 8] = [
    "███      ███",
    "███      ███",
    "███      ███",
    "████████████",
    "████████████",
    "███      ███",
    "███      ███",
    "███      ███",
];
const LOGO_WIDTH: usize = 10;

/// Emerald-500 (#10b981) in truecolor. Disabled when not a tty or when the user
/// sets `NO_COLOR`.
fn emerald(s: &str, color: bool) -> String {
    if color {
        format!("\x1b[38;2;16;185;129m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

fn bold_emerald(s: &str, color: bool) -> String {
    if color {
        format!("\x1b[1;38;2;16;185;129m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

fn dim(s: &str, color: bool) -> String {
    if color {
        format!("\x1b[2m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

/// Amber-500 (#f59e0b) for "on, but needs attention": an outdated CLI.
fn amber(s: &str, color: bool) -> String {
    if color {
        format!("\x1b[38;2;245;158;11m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

/// Status dot: solid green (on) or hollow grey (off).
fn dot(on: bool, color: bool) -> String {
    if on {
        emerald("●", color)
    } else {
        dim("○", color)
    }
}

/// One "area" row: `label` + aligned `version` + on/off dot + status. Padding is
/// applied to the plain text so the dot's ANSI codes don't throw it off.
fn component(label: &str, ver: &str, on: bool, status: &str, color: bool) -> String {
    format!("{label:<8} {ver:<9} {} {status}", dot(on, color))
}

/// The `cli` row. Green dot plus "in use" normally; amber dot plus what the
/// update is doing when there's a newer release.
///
/// Since the service updates itself, "update available" was a half-truth that
/// invited people to do something already happening. What it says now is where
/// the update has got to: downloading, waiting for you to close the game, or
/// waiting for somebody to approve the privilege dialog, which is the one case
/// where there is anything to type.
fn cli_component(ver: &str, update: Option<&UpdateLine>, color: bool) -> String {
    match update {
        Some(u) => format!("{:<8} {:<9} {} {}", "cli", ver, amber("●", color), u.text),
        None => component("cli", ver, true, "in use", color),
    }
}

/// What the `cli` row has to say about the update.
struct UpdateLine {
    text: String,
}

/// Turns the service's state into one line. With no service it falls back to the
/// usual local check, which is what somebody with only the terminal installed and
/// the service stopped sees.
async fn update_line() -> Option<UpdateLine> {
    use hoard_core::ipc::{UpdateHold, UpdatePhase};

    let Some(state) = crate::commands::link::update_state().await else {
        let latest = hoard_agent::update::available_update().await?;
        return Some(UpdateLine {
            text: format!("update available → v{latest} (run `hoard upgrade`)"),
        });
    };
    let latest = state.latest.as_deref()?;
    if !hoard_agent::update::is_newer(latest, env!("CARGO_PKG_VERSION")) {
        return None;
    }
    let text = match state.phase {
        UpdatePhase::Downloading => format!("v{latest} — downloading"),
        UpdatePhase::Waiting {
            hold: UpdateHold::GameRunning,
        } => format!("v{latest} — installs when you close the game"),
        UpdatePhase::Waiting {
            hold: UpdateHold::TransferInFlight,
        } => format!("v{latest} — installs when the current backup finishes"),
        // A reason this binary does not know, sent by a newer service. During a
        // handover that is normal.
        UpdatePhase::Waiting {
            hold: UpdateHold::Unknown,
        } => format!("v{latest} — waiting for the right moment"),
        UpdatePhase::Applying => format!("v{latest} — installing"),
        UpdatePhase::Restarting => format!("v{latest} — installed, restarting"),
        UpdatePhase::Managed => format!("v{latest} out — your package manager owns this install"),
        UpdatePhase::Failed => match state.last_error.as_deref() {
            Some(err) => format!("v{latest} — last attempt failed: {err}"),
            None => format!("v{latest} — last attempt failed"),
        },
        // Downloaded and waiting. Whether anything has to be typed is decided by
        // whether this machine can relieve itself.
        UpdatePhase::Ready if state.unattended => format!("v{latest} — ready, installing shortly"),
        UpdatePhase::Ready => format!("v{latest} — ready (run `hoard upgrade` to install it)"),
        UpdatePhase::UpToDate | UpdatePhase::Unknown => format!("v{latest} available"),
    };
    Some(UpdateLine { text })
}

/// One cheat-sheet line: `cmd` (green) at fixed width + description.
fn cmd_line(cmd: &str, desc: &str, color: bool) -> String {
    let pad = 20usize.saturating_sub(cmd.chars().count()).max(1);
    format!("    {}{}{}", emerald(cmd, color), " ".repeat(pad), desc)
}

/// Look a binary up on `PATH`. Returns its path if found.
fn in_path(name: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&paths) {
        let cand = dir.join(name);
        if cand.is_file() {
            return Some(cand);
        }
        #[cfg(windows)]
        {
            let exe = dir.join(format!("{name}.exe"));
            if exe.is_file() {
                return Some(exe);
            }
        }
    }
    None
}

/// Whether the desktop app appears installed on this machine. On Unix, seeing
/// `hoard-desktop` on `PATH` is enough (it's a normal binary). On Windows the
/// Tauri app isn't on `PATH`, so also check its default NSIS install location
/// and the Start Menu shortcut. Best-effort.
fn desktop_installed() -> bool {
    if in_path("hoard-desktop").is_some() {
        return true;
    }
    #[cfg(windows)]
    {
        // Tauri's default NSIS install location.
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            let cand = PathBuf::from(&local)
                .join("Programs")
                .join("Hoard")
                .join("hoard-desktop.exe");
            if cand.is_file() {
                return true;
            }
        }
        // Start Menu shortcut: Tauri's MSI and NSIS installers create one.
        if let Some(appdata) = std::env::var_os("APPDATA") {
            let start = PathBuf::from(&appdata)
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs")
                .join("Hoard.lnk");
            if start.exists() {
                return true;
            }
        }
    }
    #[cfg(not(windows))]
    {
        // Unix is already covered by the `in_path` check above.
    }
    false
}

/// Normalize a version to `vX.Y.Z` (prepends `v` if missing).
fn with_v(ver: &str) -> String {
    if ver.starts_with('v') {
        ver.to_string()
    } else {
        format!("v{ver}")
    }
}

/// Probe the server you actually connect to via `/v1/health` (no auth needed).
/// It is the only correct way to read its version, since the server may run in
/// Docker, on local bare metal or on another machine, and a local binary tells
/// you nothing about that. Short timeout so the banner never hangs. Returns
/// `(online, version, mode)`.
async fn probe_server(url: &str) -> (bool, String, Option<String>) {
    let Ok(client) = ApiClient::new(url, "") else {
        return (false, "—".to_string(), None);
    };
    match tokio::time::timeout(Duration::from_millis(1200), client.health()).await {
        Ok(Ok(h)) => (true, with_v(&h.version), h.mode),
        _ => (false, "—".to_string(), None),
    }
}

/// The `sync` row: we ask the service, which is what does the syncing.
///
/// Until Slice 4c this read the pidfile and checked the process was still alive.
/// Not any more: the engine lives in `hoardd` and its answer is the truth, and it
/// also tells "the service is up" from "the service has an engine", which a
/// pidfile never could. Connecting starts nothing: a status panel that brought
/// the service up would be the worst possible side effect.
async fn sync_row() -> (bool, String) {
    let Some(status) = crate::commands::link::status().await else {
        return (false, "stopped".to_string());
    };
    if status.engine.running {
        let watched = status.slots.len().max(status.engine.watched);
        return (
            true,
            format!("running · {watched} save(s) · pid {}", status.pid),
        );
    }
    (
        false,
        format!(
            "service up, engine down · {}",
            status
                .engine
                .last_error
                .as_deref()
                .unwrap_or("still starting")
        ),
    )
}

/// `full` prints the whole cheat-sheet. `false` (used above `hoard sync`)
/// keeps the logo + info column and only the Sync service commands.
pub async fn show(full: bool) -> Result<()> {
    let color = std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none();
    let cli_ver = format!("v{}", env!("CARGO_PKG_VERSION"));

    // Is there anything newer, and what is happening with it? Ask the service,
    // which is the one doing it, and fall back to the cached local check with no
    // service. Best-effort: a failure leaves the dot green.
    let cli_update = update_line().await;

    // Session: Cloud wins over self-host (same as when resolving). Best-effort,
    // no network.
    let cloud = matches!(hoard_agent::cloud_auth::load_session(), Ok(Some(_)));
    let cfg = CliConfig::load_default().ok();
    let has_token = cfg
        .as_ref()
        .map(|(c, _)| c.auth.token.as_deref().is_some_and(|t| !t.is_empty()))
        .unwrap_or(false);
    let (session_on, session_label) = if cloud {
        (true, "Cloud".to_string())
    } else if has_token {
        (true, "self-host".to_string())
    } else {
        (false, "signed out".to_string())
    };

    // Server: probe the one you'll actually use (Cloud if there's a Cloud
    // session, otherwise the self-host URL from your config) via `/v1/health`.
    // That way the version and status hold whether it's in Docker, on bare metal
    // or on another machine; we don't depend on a local binary being installed.
    let server_url = if cloud {
        Some(hoard_agent::cloud_auth::cloud_base_url())
    } else {
        cfg.as_ref().map(|(c, _)| c.server.url.clone())
    };
    let (server_on, server_ver, server_mode) = match &server_url {
        Some(url) => probe_server(url).await,
        None => (false, "—".to_string(), None),
    };
    let server_status = match (server_on, server_mode.as_deref()) {
        (true, Some("cloud")) => "online · Cloud".to_string(),
        (true, Some(m)) => format!("online · {m}"),
        (true, None) => "online".to_string(),
        (false, _) if server_url.is_none() => "no server".to_string(),
        (false, _) => "unreachable".to_string(),
    };

    // Desktop: this one is local (it's a GUI). On Unix a PATH lookup suffices;
    // on Windows `desktop_installed` also probes the install location and the
    // Start Menu shortcut (the Tauri app isn't on PATH).
    let has_desktop = desktop_installed();

    let (daemon_on, daemon_label) = sync_row().await;

    // Right-hand column, in order. It may have more rows than the logo; the
    // extra ones are painted under the logo with indentation (fastfetch style).
    let info = [
        bold_emerald("hoard", color),
        dim("game save sync", color),
        String::new(),
        cli_component(&cli_ver, cli_update.as_ref(), color),
        component(
            "desktop",
            "—",
            has_desktop,
            if has_desktop {
                "installed"
            } else {
                "not installed"
            },
            color,
        ),
        component("server", &server_ver, server_on, &server_status, color),
        component("session", "", session_on, &session_label, color),
        component("sync", "", daemon_on, &daemon_label, color),
    ];

    println!();
    let rows = LOGO.len().max(info.len());
    for i in 0..rows {
        let left = match LOGO.get(i) {
            Some(row) => emerald(row, color),
            None => " ".repeat(LOGO_WIDTH),
        };
        let text = info.get(i).map(String::as_str).unwrap_or("");
        println!("  {left}   {text}");
    }

    println!();
    if full {
        println!("  Commands");
        println!("{}", cmd_line("hoard", "this status panel", color));
        println!(
            "{}",
            cmd_line("hoard desktop", "open the desktop app", color)
        );
        println!(
            "{}",
            cmd_line("hoard server", "start the self-host server", color)
        );
        println!();
        println!("  Cloud & saves");
        println!(
            "{}",
            cmd_line("hoard login", "sign in (Cloud, no browser)", color)
        );
        println!("{}", cmd_line("hoard logout", "sign out", color));
        println!(
            "{}",
            cmd_line("hoard track", "track a game (detect + remember)", color)
        );
        println!("{}", cmd_line("hoard saves", "list what you track", color));
        println!();
    }
    println!("  Sync service");
    println!(
        "{}",
        cmd_line(
            "hoard sync start",
            "install & run automatic sync (now + every login)",
            color
        )
    );
    println!(
        "{}",
        cmd_line(
            "hoard sync stop",
            "stop it and remove from autostart",
            color
        )
    );
    println!(
        "{}",
        cmd_line("hoard sync restart", "restart the service", color)
    );
    println!(
        "{}",
        cmd_line("hoard sync logs", "show recent service logs", color)
    );
    println!();
    if full {
        println!("{}", cmd_line("hoard --help", "all commands", color));
        println!();
    }
    Ok(())
}
