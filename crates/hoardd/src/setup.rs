//! Installing, repairing and taking Hoard off a machine: the one sequence every
//! front door runs.
//!
//! Hoard Setup, `hoard install`, `hoard upgrade --version` and the repair the
//! service starts on its own all go through [`run`]. It lives in the service's
//! crate rather than in the agent because it needs both halves: the agent knows
//! the files (where they go, how they are checked, what an older release left),
//! and this crate knows the service (its manager, its socket, which version is
//! answering).
//!
//! ## The order
//!
//! 1. Download and verify everything. Nothing has been touched yet, so a failure
//!    here leaves the machine exactly as it was.
//! 2. Stop every Hoard that is running: the service through its manager, any
//!    other `hoardd`, the app.
//! 3. Take off what would undo the new install if it ran afterwards (the old
//!    NSIS folder, whose uninstaller deletes `%LOCALAPPDATA%\hoard\bin`).
//! 4. Put the core and the app in place.
//! 5. Start the service from the new core and check that the engine answering
//!    is the version just installed. A service that comes back on an older
//!    binary is the bug this module exists for, so it is checked, not assumed.
//! 6. Take off the rest of what older releases left. A package needs a
//!    password; with nobody there to type it, it stays and the outcome says so.
//! 7. Write the manifest, last: while it is missing, nothing believes in an
//!    install that is half done.
//!
//! If anything between 2 and 5 fails, the service is brought back on whatever
//! binary is there, so a failed install never costs the sync that was running.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use hoard_agent::install::legacy::{self, Leftover};
use hoard_agent::install::{
    self, fetch, layout, remove, Component, Delivery, Manifest, Probe, Swap,
};

use crate::autostart;
use crate::client::{self, Client};
use crate::endpoint::Endpoint;

/// Where the core comes from.
#[derive(Debug, Clone)]
pub enum Source {
    /// The release's tarball, downloaded and checked against its signature.
    Download,
    /// A folder that already has `hoard` and `hoardd` in it: the one the
    /// terminal installer just unpacked, or the one `hoard install` runs from.
    Local(PathBuf),
}

/// Which components the person asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Want {
    /// What is there already, or what the machine needs on a first install.
    Keep,
    /// Core only, even with a screen.
    Headless,
    /// The app too, even if nothing looks graphical.
    Desktop,
}

#[derive(Debug, Clone)]
pub struct Plan {
    /// `None`: the latest release, or this binary's own version when the core
    /// is [`Source::Local`].
    pub version: Option<String>,
    pub want: Want,
    pub source: Source,
    /// Somebody can answer a password prompt.
    pub interactive: bool,
    /// Hoard Setup itself, kept as the uninstaller "Installed apps" runs
    /// (Windows). Without it the release's copy is downloaded.
    pub setup_binary: Option<PathBuf>,
    /// The app from a file on disk instead of the release. For installing
    /// without a network, and for trying a build before it is published.
    pub app_file: Option<PathBuf>,
}

/// The pieces a folder of Hoard binaries carries, for `--from <dir>`: the core
/// always, the app and Hoard Setup when they are there.
pub fn from_dir(dir: &Path) -> (Option<PathBuf>, Option<PathBuf>) {
    let exe = std::env::consts::EXE_SUFFIX;
    let app = Some(dir.join(format!("hoard-desktop{exe}"))).filter(|p| p.is_file());
    let setup = [format!("HoardSetup{exe}"), format!("hoard-installer{exe}")]
        .into_iter()
        .map(|name| dir.join(name))
        .find(|p| p.is_file());
    (app, setup)
}

#[derive(Debug, Clone)]
pub struct Outcome {
    pub version: String,
    /// The app, when there is one.
    pub app: Option<PathBuf>,
    /// The app was open and got closed to be replaced.
    pub app_was_running: bool,
    /// Older pieces that are still here, and why.
    pub left: Vec<String>,
    /// Things that went wrong without stopping the install.
    pub warnings: Vec<String>,
}

/// Puts this machine at one version of Hoard, with one copy of each piece.
pub async fn run(plan: Plan) -> Result<Outcome> {
    if install::running_under_flatpak() {
        bail!("this Hoard is a Flatpak; Flatpak updates it");
    }
    let _swap = Swap::begin();
    let probe = Probe::read();
    let version = resolve_version(&plan).await?;
    let existing = Manifest::load().ok().flatten();
    let leftovers = legacy::scan().await;
    let on_disk = OnDisk::read();
    let mut manifest = plan_manifest(
        plan.want,
        &version,
        &probe,
        existing.as_ref(),
        &leftovers,
        on_disk,
    );
    let first_install = existing.is_none() && leftovers.is_empty() && !on_disk.core;
    // The app a package put here is the person's to give up, and giving it up
    // takes a password. With nobody to ask it stays the app and only the core
    // and the service move; with somebody there it is offered in
    // `place_and_start`, and a "no" keeps it the same way.
    if !plan.interactive && manifest.has(Component::Desktop) {
        if let Some((delivery, app)) = package_app(&leftovers) {
            keep_package(&mut manifest, delivery, &app);
        }
    }
    let mut warnings = Vec::new();
    tracing::info!(
        version = %version,
        components = ?manifest.components,
        leftovers = leftovers.len(),
        "setup: starting"
    );

    // ---- 1. download
    let stage = hoard_agent::config::CliConfig::cache_dir()?
        .join("setup")
        .join(&version);
    let core = match &plan.source {
        Source::Download => {
            let name = fetch::core_asset_name(&version).with_context(|| {
                format!(
                    "no build of Hoard for {}-{}",
                    std::env::consts::OS,
                    std::env::consts::ARCH
                )
            })?;
            CoreFrom::Tarball(
                fetch::download_named(&version, &name, &stage)
                    .await
                    .context("downloading the sync engine")?,
            )
        }
        Source::Local(dir) => {
            refuse_downgrade(&version).await?;
            CoreFrom::Dir(dir.clone())
        }
    };
    // The app this version already put here needs no new download: a repair
    // after an old installer was run over the top only has shortcuts and
    // entries to put back, and re-fetching 100 MB for that is a waste.
    let app_in_place = existing.as_ref().is_some_and(|m| {
        m.version == version
            && m.delivery == manifest.delivery
            && m.desktop_path.as_ref().is_some_and(|p| p.exists())
    }) && on_disk.app;
    // An app opened from where it belongs with nothing recording it hands over
    // its own file, which is the destination: copied onto itself, a bundle is
    // deleted before it is read.
    let given_in_place = plan
        .app_file
        .as_deref()
        .is_some_and(|file| layout::app_path().is_ok_and(|app| same_dir(file, &app)));
    let app_file = match manifest
        .delivery
        .filter(|d| manifest.has(Component::Desktop) && !d.is_package())
    {
        Some(_) if given_in_place => None,
        Some(_) if plan.app_file.is_some() => plan.app_file.clone(),
        Some(_) if app_in_place => None,
        Some(delivery) => Some(
            fetch::download_app(delivery, &version, &stage)
                .await
                .context("downloading the desktop app")?,
        ),
        None => None,
    };
    #[cfg(windows)]
    if app_file.is_some() {
        if let Err(err) = install::windows::ensure_webview2(&stage).await {
            warnings.push(format!("{err:#}"));
        }
    }
    #[cfg(windows)]
    let desktop_shortcut = first_install || install::windows::has_desktop_shortcut().await;
    #[cfg(not(windows))]
    let desktop_shortcut = false;

    #[cfg(windows)]
    let app_autostart = install::windows::app_autostart_line();

    // ---- 2. stop
    let had_unit = autostart::installed().await;
    let app_was_running = stop_everything().await;

    // ---- 3, 4, 5
    let placed = place_and_start(
        &plan,
        &version,
        &core,
        app_file.as_deref(),
        &leftovers,
        &mut manifest,
        had_unit || first_install,
        desktop_shortcut,
        &mut warnings,
    )
    .await;
    if let Err(err) = placed {
        bring_back(had_unit).await;
        return Err(err);
    }
    #[cfg(windows)]
    if let (Some(before), Some(app)) = (&app_autostart, &manifest.desktop_path) {
        install::windows::repoint_app_autostart(before, app);
    }
    #[cfg(target_os = "linux")]
    if let Some(app) = &manifest.desktop_path {
        fetch::repoint_app_autostart(app);
    }

    // ---- 6. what is left of older installs
    let mut left = Vec::new();
    for item in leftovers.iter().filter(|l| !l.in_the_way()) {
        // The app's own package was offered, or kept, when the app was placed.
        if matches!(item, Leftover::Package { .. }) && manifest.has(Component::Desktop) {
            if manifest.delivery.is_some_and(|d| d.is_package()) {
                left.push(item.describe());
            }
            continue;
        }
        if item.needs_admin() && !plan.interactive {
            left.push(item.describe());
            continue;
        }
        match legacy::remove(item, plan.interactive).await {
            Ok(()) => tracing::info!(item = %item.describe(), "setup: removed"),
            Err(err) => left.push(format!("{}: {err:#}", item.describe())),
        }
    }
    forget_markers();
    forget_parked();

    // ---- 7. the record
    manifest.version.clone_from(&version);
    manifest.save().context("writing the install manifest")?;
    let _ = std::fs::remove_dir_all(&stage);
    tracing::info!(version = %version, left = left.len(), "setup: done");

    Ok(Outcome {
        version,
        app: manifest.desktop_path.clone(),
        app_was_running,
        left,
        warnings,
    })
}

enum CoreFrom {
    Tarball(PathBuf),
    Dir(PathBuf),
}

async fn resolve_version(plan: &Plan) -> Result<String> {
    if let Some(v) = &plan.version {
        return Ok(v.trim_start_matches('v').to_string());
    }
    match &plan.source {
        // The binaries there say which version they are; this process may be
        // an older `hoard` pointed at a newer folder.
        Source::Local(dir) => Ok(install::core_version(dir)
            .await
            .unwrap_or_else(|| hoard_agent::update::current().to_string())),
        Source::Download => fetch::latest_version().await,
    }
}

/// What to install. The person's answer outranks the probe in both
/// directions, and an update never takes away an app that is there.
fn plan_manifest(
    want: Want,
    version: &str,
    probe: &Probe,
    existing: Option<&Manifest>,
    leftovers: &[Leftover],
    on_disk: OnDisk,
) -> Manifest {
    // An app the record names but nobody can find was taken off by hand
    // (Settings, `apt remove`): an update that put it back is how "I
    // uninstalled Hoard and it came back" happened.
    let recorded_app = existing.is_some_and(|m| {
        m.has(Component::Desktop) && m.desktop_path.as_ref().is_none_or(|p| p.exists())
    });
    let app_here = recorded_app || leftovers.iter().any(is_old_app) || on_disk.app;
    let known = existing.is_some() || !leftovers.is_empty() || on_disk.core;

    let mut m = Manifest::planned(version, probe);
    match want {
        Want::Keep if known => {
            m.components = vec![Component::Core];
            if app_here {
                m.add(Component::Desktop);
            }
        }
        Want::Keep => {}
        Want::Headless => m.components.retain(|c| *c == Component::Core),
        Want::Desktop => m.add(Component::Desktop),
    }
    m.delivery = m
        .has(Component::Desktop)
        .then(|| install::resolve_delivery(probe));
    m
}

fn is_old_app(l: &Leftover) -> bool {
    matches!(
        l,
        Leftover::Package { .. } | Leftover::Nsis { .. } | Leftover::Msi { .. }
    )
}

/// What the layout already has on this machine, read once so that
/// [`plan_manifest`] answers from its arguments and not from the disk of
/// whoever runs the tests.
#[derive(Debug, Clone, Copy, Default)]
struct OnDisk {
    core: bool,
    app: bool,
}

impl OnDisk {
    fn read() -> Self {
        Self {
            core: layout::core_binary("hoardd").is_ok_and(|p| p.is_file()),
            app: layout::app_path().is_ok_and(|p| p.exists()),
        }
    }
}

/// The app a `.deb` or `.rpm` put in `/usr/bin`, if there is one.
fn package_app(leftovers: &[Leftover]) -> Option<(Delivery, PathBuf)> {
    leftovers.iter().find_map(|l| match l {
        Leftover::Package { delivery, app } => Some((*delivery, app.clone())),
        _ => None,
    })
}

/// The package stays the app: the core and the service still move, and the
/// app is the person's to update.
fn keep_package(manifest: &mut Manifest, delivery: Delivery, app: &Path) {
    manifest.delivery = Some(delivery);
    manifest.desktop_path = Some(app.to_path_buf());
}

/// `hoard install` run from an older copy must not put that copy over a newer
/// core. It happens: `/usr/bin/hoard` from a forgotten package, typed by hand.
async fn refuse_downgrade(version: &str) -> Result<()> {
    let Ok(dir) = layout::core_dir() else {
        return Ok(());
    };
    if let Some(installed) = install::core_version(&dir).await {
        if hoard_agent::update::is_newer(&installed, version) {
            bail!(
                "Hoard {installed} is already installed in {}, newer than this {version}; \
                 run that one instead (`{}`)",
                dir.display(),
                dir.join(format!("hoard{}", std::env::consts::EXE_SUFFIX))
                    .display()
            );
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn place_and_start(
    plan: &Plan,
    version: &str,
    core: &CoreFrom,
    app_file: Option<&Path>,
    leftovers: &[Leftover],
    manifest: &mut Manifest,
    declare_service: bool,
    desktop_shortcut: bool,
    warnings: &mut Vec<String>,
) -> Result<()> {
    for item in leftovers.iter().filter(|l| l.in_the_way()) {
        legacy::remove(item, plan.interactive)
            .await
            .with_context(|| format!("removing {}", item.describe()))?;
        tracing::info!(item = %item.describe(), "setup: removed");
    }

    let core_dir = layout::core_dir()?;
    match core {
        CoreFrom::Tarball(tarball) => {
            fetch::apply_core(tarball, &core_dir)
                .await
                .context("installing the sync engine")?;
        }
        CoreFrom::Dir(dir) if !same_dir(dir, &core_dir) => {
            fetch::place_core_from(dir, &core_dir).context("installing the sync engine")?;
        }
        CoreFrom::Dir(_) => {}
    }
    manifest.core_dir = Some(core_dir.clone());
    if let Err(err) = install::ensure_on_shell_path(&core_dir) {
        warnings.push(format!("{err:#}"));
    }

    // A package is asked about before anything replaces it. Placed beside a
    // package the person then keeps, the new app would be a second Hoard in
    // the menu for good.
    let mut app_file = app_file;
    if plan.interactive && manifest.has(Component::Desktop) {
        if let Some((delivery, app)) = package_app(leftovers) {
            for item in leftovers
                .iter()
                .filter(|l| matches!(l, Leftover::Package { .. }))
            {
                match legacy::remove(item, true).await {
                    Ok(()) => tracing::info!(item = %item.describe(), "setup: removed"),
                    Err(err) => {
                        tracing::info!(error = %format!("{err:#}"), "setup: the package stays the app");
                        warnings.push(format!("{}: {err:#}", item.describe()));
                        keep_package(manifest, delivery, &app);
                        app_file = None;
                    }
                }
            }
        }
    }

    if manifest.delivery.is_some_and(|d| d.is_package()) {
        // The person's package, left where it is.
    } else if let (Some(file), Some(delivery)) = (app_file, manifest.delivery) {
        let path = fetch::apply_desktop(delivery, file)
            .await
            .context("installing the desktop app")?;
        #[cfg(windows)]
        if let Err(err) = install::windows::register(&path, version, desktop_shortcut).await {
            warnings.push(format!("{err:#}"));
        }
        let _ = desktop_shortcut;
        manifest.desktop_path = Some(path);
    } else if manifest.has(Component::Desktop) {
        // Already in place at this version; what an old installer may have
        // overwritten is the shortcuts and the entries around it.
        let path = layout::app_path()?;
        #[cfg(windows)]
        if let Err(err) = install::windows::register(&path, version, desktop_shortcut).await {
            warnings.push(format!("{err:#}"));
        }
        manifest.desktop_path = Some(path);
    } else if plan.want == Want::Headless {
        // Asked for the core alone on a machine that had the app: it goes.
        if let Ok(app) = layout::app_path() {
            if app.exists() {
                let delivery = install::resolve_delivery(&Probe::read());
                if let Err(err) = remove::desktop(delivery, Some(&app), !plan.interactive).await {
                    warnings.push(format!("{err:#}"));
                }
            }
        }
        remove::app_autostart();
        manifest.desktop_path = None;
    }

    #[cfg(windows)]
    keep_uninstaller(plan, version, warnings).await;

    // The binaries are in place: from here on, a client starting the service
    // starts the new one. On Windows the service manager's own start goes
    // through a client, which would refuse while the marker says otherwise.
    Swap::forget();
    start_service(declare_service, version).await
}

/// The copy of Hoard Setup that "Installed apps" runs.
#[cfg(windows)]
async fn keep_uninstaller(plan: &Plan, version: &str, warnings: &mut Vec<String>) {
    let from = match &plan.setup_binary {
        Some(path) => Ok(path.clone()),
        None => fetch_setup(version).await,
    };
    if let Err(err) = from.and_then(|p| install::windows::keep_setup_copy(&p)) {
        warnings.push(format!("keeping the uninstaller: {err:#}"));
    }
}

/// Starts the service from the core just placed and checks who answers.
async fn start_service(declare: bool, version: &str) -> Result<()> {
    client::use_daemon_binary(layout::core_binary("hoardd")?);
    for attempt in 0..2 {
        if declare {
            if let Err(err) = autostart::install().await {
                // A machine with no service manager (a container, an init
                // without systemd) still gets a running engine; what it loses is
                // starting at login.
                tracing::warn!(error = %format!("{err:#}"), "setup: login start couldn't be declared");
                start_detached().await?;
            }
        } else {
            start_detached().await?;
        }
        match serving_version().await {
            Some(v) if v == version => return Ok(()),
            Some(other) if attempt == 0 => {
                // Something brought an old engine back between the stop and the
                // start. It is told to leave, and the new one gets its turn.
                tracing::warn!(serving = %other, expected = version, "setup: an older engine answered; replacing it");
                shutdown_serving().await;
                if kills_by_name() {
                    kill_everything().await;
                }
            }
            Some(other) => {
                bail!("the sync service answering is {other}, not {version}; see `hoard sync logs`")
            }
            None => bail!("the sync service didn't start; see `hoard sync logs`"),
        }
    }
    unreachable!("the loop returns or bails on its second pass")
}

async fn start_detached() -> Result<()> {
    if serving_version().await.is_none() {
        client::respawn_service().context("starting the sync service")?;
    }
    autostart::wait_until_serving().await
}

/// After a failure halfway through: whatever engine is on disk is started
/// again, so a failed install does not also cost the sync.
async fn bring_back(had_unit: bool) {
    // The marker `run` holds would turn this start down: on Windows without a
    // login task the service manager's start goes through a client, and a
    // client refuses while an install says it is mid-swap.
    Swap::forget();
    if serving_version().await.is_some() {
        return;
    }
    let result = if had_unit {
        autostart::restart().await.map(|_| ())
    } else {
        client::respawn_service().map(|_| ())
    };
    if let Err(err) = result {
        tracing::error!(error = %format!("{err:#}"), "setup: the install failed and the service couldn't be brought back");
    }
}

/// The version of the engine answering on the socket, if one is.
pub async fn serving_version() -> Option<String> {
    let endpoint = Endpoint::resolve().ok()?;
    let client = Client::connect(&endpoint, "hoard setup (check)")
        .await
        .ok()?;
    Some(client.welcome().daemon_version.clone())
}

async fn shutdown_serving() {
    let Ok(endpoint) = Endpoint::resolve() else {
        return;
    };
    let Ok(mut client) = Client::connect(&endpoint, "hoard setup").await else {
        return;
    };
    let _ = client.request(hoard_core::ipc::Request::Shutdown).await;
    drop(client);
    let _ = autostart::wait_until_free().await;
}

/// Stops every Hoard on the machine and says whether the app was open.
///
/// The service manager first (launchd would restart the old binary), then a
/// clean `Shutdown` for an engine a client started, then whatever is left by
/// force. The [`Swap`] marker the caller holds keeps clients from starting one
/// back meanwhile.
async fn stop_everything() -> bool {
    autostart::stop().await;
    shutdown_serving().await;
    if !kills_by_name() {
        return false;
    }
    let app = app_running().await;
    kill_everything().await;
    app
}

/// A `HOARD_PROFILE` install is a second Hoard beside the real one (staging, a
/// test): it stops through its own socket, and killing by name would take the
/// real one down with it.
fn kills_by_name() -> bool {
    hoard_agent::config::profile_suffix().is_none()
}

#[cfg(target_os = "linux")]
const OURS: [&str; 3] = ["hoard-desktop", "hoard-screen", "hoardd"];

/// Every process of ours on Linux, read from `/proc`, except the ones inside a
/// Flatpak (that install is separate, and its engine is not ours to stop) and
/// the ones of other accounts: their Hoard is theirs, the signal would only
/// bounce, and an app open in another session read as ours being open.
#[cfg(target_os = "linux")]
fn our_processes() -> Vec<i32> {
    use std::os::unix::fs::MetadataExt;
    let me = std::process::id() as i32;
    let Ok(uid) = std::fs::metadata("/proc/self").map(|m| m.uid()) else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|e| e.file_name().to_str()?.parse::<i32>().ok())
        .filter(|pid| *pid != me)
        .filter(|pid| std::fs::metadata(format!("/proc/{pid}")).is_ok_and(|m| m.uid() == uid))
        .filter(|pid| {
            let comm = std::fs::read_to_string(format!("/proc/{pid}/comm")).unwrap_or_default();
            OURS.contains(&comm.trim())
        })
        .filter(|pid| {
            std::fs::read_link(format!("/proc/{pid}/exe"))
                .map(|exe| !exe.starts_with("/app/"))
                .unwrap_or(true)
        })
        .collect()
}

#[cfg(target_os = "linux")]
async fn app_running() -> bool {
    our_processes().iter().any(|pid| {
        std::fs::read_to_string(format!("/proc/{pid}/comm"))
            .is_ok_and(|c| c.trim() == "hoard-desktop")
    })
}

#[cfg(target_os = "linux")]
async fn kill_everything() {
    let send = |signal| {
        for pid in our_processes() {
            // SAFETY: `kill` takes plain integers and touches no memory of ours.
            unsafe {
                libc::kill(pid, signal);
            }
        }
    };
    send(libc::SIGTERM);
    for _ in 0..30 {
        if our_processes().is_empty() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    send(libc::SIGKILL);
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
}

#[cfg(not(target_os = "linux"))]
async fn app_running() -> bool {
    #[cfg(windows)]
    let out = tokio::process::Command::new("tasklist")
        .args(["/FI", "IMAGENAME eq hoard-desktop.exe", "/NH"])
        .output()
        .await;
    #[cfg(not(windows))]
    let out = tokio::process::Command::new("pgrep")
        .args(["-x", "hoard-desktop"])
        .output()
        .await;
    match out {
        Ok(out) if cfg!(windows) => {
            String::from_utf8_lossy(&out.stdout).contains("hoard-desktop.exe")
        }
        Ok(out) => out.status.success(),
        Err(_) => false,
    }
}

#[cfg(not(target_os = "linux"))]
async fn kill_everything() {
    remove::stop_running().await;
}

/// Markers older installers left for the service to act on, which no longer
/// mean anything.
fn forget_markers() {
    if let Ok(dirs) = hoard_agent::config::CliConfig::project_dirs() {
        let _ = std::fs::remove_file(dirs.config_dir().join("login-start-removed"));
    }
}

/// The copies Windows had to move aside to replace a running binary
/// (`fetch::replace_binary`). Nothing runs them once the new service has
/// answered; one still in use stays for the next pass.
fn forget_parked() {
    if !cfg!(windows) {
        return;
    }
    let mut parked = Vec::new();
    if let Ok(dir) = layout::core_dir() {
        parked.extend(["hoard.old", "hoardd.old"].map(|n| dir.join(n)));
    }
    if let Ok(app) = layout::app_path() {
        parked.push(app.with_extension("old"));
    }
    for path in parked {
        let _ = std::fs::remove_file(path);
    }
}

fn same_dir(a: &Path, b: &Path) -> bool {
    let canonical = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    canonical(a) == canonical(b)
}

// ---- taking it off

/// Takes Hoard off this machine: every piece, by the route it arrived on, older
/// installs included. Returns where the user's own things still are: **nothing
/// under those paths is touched**. Saves are the reason someone used Hoard and
/// there can be tens of gigabytes of them.
pub async fn uninstall(interactive: bool) -> Result<Vec<PathBuf>> {
    let _swap = Swap::begin();
    let manifest = Manifest::load().ok().flatten();
    let leftovers = legacy::scan().await;

    stop_everything().await;
    // Login start goes after the stop: removing it no longer stops the
    // service, and brings it back where the manager can't separate the two.
    let _ = autostart::uninstall().await;

    let mut failed = Vec::new();
    let app = manifest
        .as_ref()
        .and_then(|m| m.desktop_path.clone())
        .or_else(|| layout::app_path().ok())
        .filter(|p| p.exists());
    if let Some(app) = app {
        let delivery = manifest
            .as_ref()
            .and_then(|m| m.delivery)
            .unwrap_or_else(|| install::resolve_delivery(&Probe::read()));
        // An old route comes off through `leftovers`, which knows its uninstaller.
        if !delivery.is_legacy() {
            if let Err(err) = remove::desktop(delivery, Some(&app), !interactive).await {
                failed.push(format!("the app: {err:#}"));
            }
        }
    }
    remove::app_autostart();
    for item in &leftovers {
        if item.needs_admin() && !interactive {
            failed.push(format!("{} (needs an administrator)", item.describe()));
            continue;
        }
        if let Err(err) = legacy::remove(item, interactive).await {
            failed.push(format!("{}: {err:#}", item.describe()));
        }
    }

    let core_dir = layout::core_dir()?;
    if let Err(err) = remove::core(&core_dir) {
        failed.push(format!("{err:#}"));
    }
    remove::shell_path_line(&core_dir);
    #[cfg(windows)]
    {
        install::windows::unregister().await;
        install::windows::forget_setup_copy();
    }
    remove::manifest().context("clearing the install record")?;
    forget_markers();

    if !failed.is_empty() {
        bail!("Hoard is off, except for: {}", failed.join("; "));
    }
    Ok(remove::kept_data())
}

// ---- is this machine in order?

/// What an install looks like from the inside: whether it is in the layout, and
/// what older releases left beside it.
#[derive(Debug, Clone)]
pub struct Health {
    pub manifest: Option<Manifest>,
    pub leftovers: Vec<Leftover>,
    /// The service runs a `hoardd` that is not the core. The exact state the
    /// 1.2.1 `.deb` machines were left in.
    pub unit_elsewhere: bool,
}

impl Health {
    pub fn needs_repair(&self) -> bool {
        !self.leftovers.is_empty()
            || self.unit_elsewhere
            || self.manifest.as_ref().is_some_and(|m| !m.is_in_layout())
    }

    /// Something only an administrator can take off (a package, the MSI).
    pub fn needs_admin(&self) -> bool {
        self.leftovers.iter().any(Leftover::needs_admin)
    }

    /// A repair with nobody around would change something. Without this the
    /// service would start one on every boot for a `.deb` it can never remove.
    pub fn fixable_unattended(&self) -> bool {
        self.unit_elsewhere
            || self.manifest.as_ref().is_some_and(|m| !m.is_in_layout())
            || self.leftovers.iter().any(|l| !l.needs_admin())
    }

    /// Hoard runs here and nothing records it: the app was downloaded from the
    /// release page and opened without Hoard Setup. With no manifest the
    /// service takes the install for a package manager's and never updates it,
    /// and on Windows the bare `.exe` doesn't even carry the service.
    ///
    /// A core in the layout is enough to rule it out, since the service adopts
    /// that one ([`install::adopt`]). An older install that needs a password or
    /// has to go first is the repair's business, not this.
    pub fn unrecorded(&self) -> bool {
        self.manifest.is_none()
            && !install::running_under_flatpak()
            && !layout::core_binary("hoardd").is_ok_and(|p| p.is_file())
            && !self
                .leftovers
                .iter()
                .any(|l| l.needs_admin() || l.in_the_way())
    }

    pub fn describe(&self) -> String {
        let mut parts: Vec<String> = self.leftovers.iter().map(Leftover::describe).collect();
        if self.unit_elsewhere {
            parts.push("the service runs a hoardd outside the core".into());
        }
        if self.manifest.as_ref().is_some_and(|m| !m.is_in_layout()) {
            parts.push("the install record points outside the layout".into());
        }
        parts.join("; ")
    }
}

pub async fn health() -> Health {
    if install::running_under_flatpak() {
        return Health {
            manifest: None,
            leftovers: Vec::new(),
            unit_elsewhere: false,
        };
    }
    let manifest = install::adopt();
    let leftovers = legacy::scan().await;
    let unit_elsewhere = match (
        autostart::installed_exec_start(),
        layout::core_binary("hoardd"),
    ) {
        (Some(exec), Ok(core)) => {
            let canonical = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
            canonical(&exec) != canonical(&core)
        }
        _ => false,
    };
    Health {
        manifest,
        leftovers,
        unit_elsewhere,
    }
}

/// Starts `hoard install` from beside this process, detached and with nobody
/// to ask, to bring the machine into the layout. The service calls it when it
/// finds itself running somewhere it shouldn't; `hoard install` stops this very
/// process on its way, which is why it is a separate one.
pub async fn spawn_unattended_repair() -> Result<()> {
    let exe = std::env::current_exe().context("resolving our own path")?;
    let hoard_in = |dir: &Path| dir.join(format!("hoard{}", std::env::consts::EXE_SUFFIX));
    let core = layout::core_dir().ok().filter(|d| hoard_in(d).is_file());
    let own = exe
        .parent()
        .map(Path::to_path_buf)
        .filter(|d| hoard_in(d).is_file());
    // The core's own `hoard`, unless it is older than this service. This
    // process may be a copy an old installer left behind, and repairing from it
    // would install that; but the core can be the stale one too (a `.deb`'s
    // service beside an old pair in `~/.local/bin`), and an old `hoard install`
    // is the old installer: it points the service back at its own engine.
    let dir = match (core, own) {
        (Some(core), Some(own)) => {
            let current = hoard_agent::update::current();
            let stale = install::core_version(&core)
                .await
                .is_none_or(|v| hoard_agent::update::is_newer(current, &v));
            if stale {
                own
            } else {
                core
            }
        }
        (Some(dir), None) | (None, Some(dir)) => dir,
        (None, None) => bail!("no `hoard` to run the repair with near {}", exe.display()),
    };
    let hoard = hoard_in(&dir);
    let mut command = outside_our_unit(&hoard, &["install"]);
    command.env("HOARD_NONINTERACTIVE", "1");
    match command.spawn() {
        Ok(_) => Ok(()),
        #[cfg(windows)]
        Err(_) => spawn_through_wmi(&hoard, &["install"]),
        #[cfg(not(windows))]
        Err(err) => Err(err).with_context(|| format!("starting {}", hoard.display())),
    }
}

/// Starts a program through WMI, whose processes are children of the WMI
/// service and in no job of ours: the way out when the login task's job does
/// not allow breaking away, and ending the task would end the repair with it.
/// Nobody can answer a prompt from there, which `hoard install` sees for
/// itself (its stdin is not a terminal).
#[cfg(windows)]
fn spawn_through_wmi(exe: &Path, args: &[&str]) -> Result<()> {
    let quote = |s: &str| format!("\"{}\"", s.replace('"', ""));
    let mut line = quote(&exe.to_string_lossy());
    for arg in args {
        line.push(' ');
        line.push_str(&quote(arg));
    }
    let script = format!(
        "$r = Invoke-CimMethod -ClassName Win32_Process -MethodName Create \
           -Arguments @{{ CommandLine = '{}' }}; exit $r.ReturnValue",
        line.replace('\'', "''")
    );
    let status = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .context("running powershell")?;
    if !status.success() {
        bail!("WMI refused to start {}", exe.display());
    }
    Ok(())
}

/// A command that won't die with the process starting it.
///
/// Detaching is not enough. Under systemd every process a service starts stays
/// in that service's cgroup, and stopping the service (which is the first
/// thing an install does) kills the whole cgroup: the repair `hoardd` starts,
/// or the Hoard Setup an autostarted app opens, would be killed by the very
/// stop it asked for. `systemd-run --scope` puts it in a cgroup of its own. On
/// Windows the same trap is the login task's job object.
fn outside_our_unit(exe: &Path, args: &[&str]) -> std::process::Command {
    #[cfg(target_os = "linux")]
    let mut command = if which("systemd-run") && std::env::var_os("XDG_RUNTIME_DIR").is_some() {
        let mut c = std::process::Command::new("systemd-run");
        c.args(["--user", "--scope", "--quiet", "--"])
            .arg(exe)
            .args(args);
        c
    } else {
        let mut c = std::process::Command::new(exe);
        c.args(args);
        c
    };
    #[cfg(not(target_os = "linux"))]
    let mut command = {
        let mut c = std::process::Command::new(exe);
        c.args(args);
        c
    };
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    client::detach(&mut command);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;
        command.creation_flags(
            DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_BREAKAWAY_FROM_JOB,
        );
    }
    command
}

#[cfg(target_os = "linux")]
fn which(name: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|d| d.join(name).is_file()))
        .unwrap_or(false)
}

// ---- Hoard Setup, for whoever needs a window

/// How the app that is running reaches Hoard Setup to be recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunningApp {
    /// Its own file, installed instead of downloading the same release again.
    File(PathBuf),
    /// A bundle outside `/Applications`: Setup installs the app from a disk
    /// image, so it fetches one.
    Fetch,
}

/// The app this process is, as an install. `None` for what isn't one: a build
/// tree, and on Linux anything but an AppImage (a package is asked about on its
/// own).
pub fn running_app() -> Option<RunningApp> {
    let exe = std::env::current_exe().ok()?;
    if exe.components().any(|c| c.as_os_str() == "target") {
        return None;
    }
    #[cfg(target_os = "linux")]
    {
        // The binary itself lives on a mount that goes away with the app.
        std::env::var_os("APPIMAGE")
            .map(PathBuf::from)
            .filter(|p| p.is_file())
            .map(RunningApp::File)
    }
    #[cfg(windows)]
    {
        Some(RunningApp::File(exe))
    }
    #[cfg(target_os = "macos")]
    {
        let bundle = exe
            .ancestors()
            .find(|p| p.extension().is_some_and(|e| e == "app"))?;
        Some(match layout::app_path() {
            Ok(app) if same_dir(bundle, &app) => RunningApp::File(app),
            _ => RunningApp::Fetch,
        })
    }
    #[cfg(not(any(target_os = "linux", windows, target_os = "macos")))]
    {
        None
    }
}

/// Downloads Hoard Setup for `version` and returns the program to run.
pub async fn fetch_setup(version: &str) -> Result<PathBuf> {
    let name = fetch::setup_asset_name().context("no Hoard Setup for this platform")?;
    let dir = hoard_agent::config::CliConfig::cache_dir()?
        .join("setup-bin")
        .join(version.trim_start_matches('v'));
    let file = fetch::download_named(version, &name, &dir).await?;
    if cfg!(target_os = "macos") {
        let status = tokio::process::Command::new("ditto")
            .args(["-x", "-k"])
            .arg(&file)
            .arg(&dir)
            .status()
            .await
            .context("unzipping Hoard Setup")?;
        if !status.success() {
            bail!("couldn't unzip {}", file.display());
        }
        return Ok(dir
            .join("Hoard Setup.app")
            .join("Contents")
            .join("MacOS")
            .join("HoardSetup"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755))?;
    }
    Ok(file)
}

/// What starts the app once it is in place. On macOS that is the bundle's
/// executable, not the bundle: `Command` runs a file, it doesn't "open" one.
pub fn launchable(app: &Path) -> PathBuf {
    if app.extension().is_some_and(|e| e == "app") {
        return app.join("Contents").join("MacOS").join("hoard-desktop");
    }
    app.to_path_buf()
}

/// Opens the app again after an install closed it ([`Outcome::app_was_running`]),
/// detached so it outlives whoever is asking.
pub fn reopen_app(app: &Path) {
    if !can_show_windows() {
        tracing::info!(app = %app.display(), "setup: nowhere to show the app, leaving it closed");
        return;
    }
    let mut command = std::process::Command::new(launchable(app));
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    client::detach(&mut command);
    if let Err(err) = command.spawn() {
        tracing::warn!(error = %err, app = %app.display(), "setup: couldn't open the app again");
    }
}

/// Would a window opened from here reach the person?
///
/// Not from session 0, where Windows puts what WMI starts: the window is
/// invisible there, and the single-instance lock it holds turns every later
/// click on Hoard into nothing. Not without a display on Linux, where the app
/// dies on its first line.
fn can_show_windows() -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::RemoteDesktop::ProcessIdToSessionId;
        let mut session = 0u32;
        // SAFETY: our own pid and a pointer to a local the call fills in.
        let ok = unsafe { ProcessIdToSessionId(std::process::id(), &mut session) } != 0;
        ok && session != 0
    }
    #[cfg(target_os = "linux")]
    {
        std::env::var_os("WAYLAND_DISPLAY").is_some() || std::env::var_os("DISPLAY").is_some()
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        true
    }
}

/// Runs Hoard Setup detached, so it outlives whoever started it: it is about to
/// close that process.
pub fn launch_setup(exe: &Path, args: &[&str]) -> Result<()> {
    let mut command = outside_our_unit(exe, args);
    #[cfg(target_os = "linux")]
    leave_the_appimage(&mut command);
    let spawned = command.spawn();
    #[cfg(windows)]
    let spawned = match spawned {
        // The job doesn't allow breaking away; the app is not in the login
        // task's job anyway, so a plain detached start is enough.
        Err(_) => {
            let mut plain = std::process::Command::new(exe);
            plain.args(args);
            client::detach(&mut plain);
            plain.spawn()
        }
        ok => ok,
    };
    spawned.with_context(|| format!("starting {}", exe.display()))?;
    Ok(())
}

/// Hoard Setup opened from an AppImage would inherit what `AppRun` exports to
/// point the app at the libraries inside the image: it would load that
/// Wayland and X11 (which the browser already died on, see the desktop's
/// `open_external`), from a mount that goes away the moment Setup closes the
/// app. The values from before the AppImage come back where it kept them.
#[cfg(target_os = "linux")]
fn leave_the_appimage(command: &mut std::process::Command) {
    if std::env::var_os("APPIMAGE").is_none() {
        return;
    }
    for var in [
        "LD_LIBRARY_PATH",
        "LD_PRELOAD",
        "GTK_PATH",
        "GDK_PIXBUF_MODULE_FILE",
        "GIO_MODULE_DIR",
        "GST_PLUGIN_SYSTEM_PATH",
        "GSETTINGS_SCHEMA_DIR",
    ] {
        match std::env::var_os(format!("{var}_ORIG")) {
            Some(orig) => command.env(var, orig),
            None => command.env_remove(var),
        };
    }
    for var in ["APPIMAGE", "APPDIR", "ARGV0", "OWD"] {
        command.env_remove(var);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn probe(graphical: bool) -> Probe {
        Probe {
            graphical,
            sandboxed: false,
        }
    }

    fn manifest(components: &[Component], delivery: Option<Delivery>) -> Manifest {
        Manifest {
            version: "1.2.1".into(),
            components: components.to_vec(),
            delivery,
            core_dir: None,
            desktop_path: None,
        }
    }

    /// An update over SSH sees no screen; the app that is there stays.
    #[test]
    fn an_update_keeps_the_app_whatever_the_probe_says() {
        let existing = manifest(
            &[Component::Core, Component::Desktop],
            Some(Delivery::AppImage),
        );
        let m = plan_manifest(
            Want::Keep,
            "1.2.2",
            &probe(false),
            Some(&existing),
            &[],
            OnDisk::default(),
        );
        assert!(m.has(Component::Desktop));
        assert!(!m.delivery.unwrap().is_legacy());
    }

    /// The `.deb` machine: the app moves to the current route, it doesn't vanish.
    #[test]
    fn an_old_package_becomes_the_current_route() {
        let existing = manifest(&[Component::Core, Component::Desktop], Some(Delivery::Deb));
        let deb = Leftover::Package {
            delivery: Delivery::Deb,
            app: "/usr/bin/hoard-desktop".into(),
        };
        let m = plan_manifest(
            Want::Keep,
            "1.2.2",
            &probe(true),
            Some(&existing),
            &[deb],
            OnDisk::default(),
        );
        assert!(m.has(Component::Desktop));
        assert_eq!(m.delivery, Some(install::resolve_delivery(&probe(true))));
    }

    /// A machine that only ever had the core doesn't get an app on update.
    #[test]
    fn an_update_of_a_headless_box_stays_headless() {
        let existing = manifest(&[Component::Core], None);
        let m = plan_manifest(
            Want::Keep,
            "1.2.2",
            &probe(true),
            Some(&existing),
            &[],
            OnDisk::default(),
        );
        assert_eq!(m.components, vec![Component::Core]);
        assert_eq!(m.delivery, None);
    }

    /// The app was removed from Settings and the update came anyway: it stays
    /// gone, and the core keeps syncing.
    #[test]
    fn an_app_removed_by_hand_is_not_put_back() {
        let mut existing = manifest(&[Component::Core, Component::Desktop], Some(Delivery::Nsis));
        existing.desktop_path = Some("/nonexistent/hoard-desktop.exe".into());
        let m = plan_manifest(
            Want::Keep,
            "1.2.2",
            &probe(true),
            Some(&existing),
            &[],
            OnDisk::default(),
        );
        assert_eq!(m.components, vec![Component::Core]);
    }

    #[test]
    fn the_person_outranks_the_probe() {
        let m = plan_manifest(
            Want::Desktop,
            "1.2.2",
            &probe(false),
            None,
            &[],
            OnDisk::default(),
        );
        assert!(m.has(Component::Desktop));
        let m = plan_manifest(
            Want::Headless,
            "1.2.2",
            &probe(true),
            None,
            &[],
            OnDisk::default(),
        );
        assert!(!m.has(Component::Desktop));
        assert_eq!(m.delivery, None);
    }

    #[test]
    fn a_package_is_left_for_somebody_who_can_answer() {
        let deb = Leftover::Package {
            delivery: Delivery::Deb,
            app: "/usr/bin/hoard-desktop".into(),
        };
        let only_admin = Health {
            manifest: None,
            leftovers: vec![deb.clone()],
            unit_elsewhere: false,
        };
        assert!(only_admin.needs_repair());
        assert!(only_admin.needs_admin());
        // Nothing a repair without a person could do: the service must not keep
        // starting one.
        assert!(!only_admin.fixable_unattended());

        let with_unit = Health {
            unit_elsewhere: true,
            ..only_admin
        };
        assert!(with_unit.fixable_unattended());
    }
}
