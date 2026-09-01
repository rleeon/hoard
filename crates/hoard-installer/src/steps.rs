//! **The install itself.** Everything the window doesn't do.
//!
//! Not one line here decides anything: which components this machine wants,
//! which release asset matches it, how a signature is checked and how each
//! package is applied all live in `hoard_agent::install`, because `install.sh`,
//! `hoard upgrade` and the in-app updater already go through there. A second
//! copy of that reasoning behind a pretty window is how you end up with an
//! installer that puts a `.deb` where the terminal puts an AppImage.
//!
//! What this module *is* responsible for is the order, and the order is the
//! part that bites: the core has to be on disk before the service can be
//! declared against it, and the service has to be told which `hoardd` it owns
//! before it writes a unit pointing at whatever it found on the `PATH`.

use std::path::PathBuf;

use anyhow::{Context, Result};
use hoard_agent::install::{self, fetch, remove, Component, Manifest, Probe, Swap};

/// What the last screen needs to know.
pub struct Outcome {
    /// The app, when it was installed and we know where it landed. `None` for a
    /// core-only install, and then the finished screen has nothing to launch.
    pub launch: Option<PathBuf>,
}

/// Where `hoard` and `hoardd` go.
///
/// The same directories `install.sh` and `install.ps1` use, and that is not a
/// coincidence to be tidied away: a machine that gets the core from the script
/// and later runs this installer has to overwrite the binaries it already has,
/// not gain a second pair somewhere else for the service to disagree about.
pub fn core_dir() -> Result<PathBuf> {
    #[cfg(windows)]
    {
        let base = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .context("no LOCALAPPDATA in the environment")?;
        Ok(base.join("hoard").join("bin"))
    }
    #[cfg(not(windows))]
    {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .context("no HOME in the environment")?;
        Ok(home.join(".local").join("bin"))
    }
}

/// The `hoardd` the service manager should be told about.
///
/// `autostart` looks for its daemon next to the running executable, and the
/// running executable here is the installer, which sits wherever it happened
/// to be downloaded to. Left alone, the unit gets written pointing at a
/// `hoardd` resolved off the `PATH`, or off nothing, and systemd reports it
/// hours later in a journal nobody is reading.
///
/// Called from `main` before any thread exists, because `set_var` is only sound
/// while the program is single-threaded.
pub fn daemon_binary() -> Result<PathBuf> {
    Ok(core_dir()?.join(format!("hoardd{}", std::env::consts::EXE_SUFFIX)))
}

/// Somewhere to drop the downloads. They are deleted on the way out; a failure
/// leaves them, which is what you want when the next step is asking why.
fn staging() -> PathBuf {
    std::env::temp_dir().join("hoard-setup")
}

/// Which release this installer is about to fetch.
///
/// The welcome screen starts out labelled with the installer's *own* version,
/// which is right the day it ships and wrong for every copy that sits in a
/// downloads folder for a month. This asks, so the label says what will
/// actually land. A failure is silent on purpose: the network is needed for
/// the install itself, and there is a screen that reports that properly.
pub async fn latest_version() -> Result<String> {
    let (version, _) = fetch::release_assets(None).await?;
    Ok(version)
}

/// Install everything this machine wants, at one version.
///
/// `want_desktop` is the only thing the window gets to decide. Everything else
/// is the agent's answer about this machine.
pub async fn run(want_desktop: bool) -> Result<Outcome> {
    // Held for the whole install. Any Hoard client that loses the socket starts
    // a service ("spawn if absent"), and one that starts from the binaries we
    // are in the middle of replacing is a locked file on Windows and a version
    // mismatch everywhere else.
    let _swap = Swap::begin();

    let probe = Probe::read();
    let stage = staging();

    let (version, assets) = fetch::release_assets(None)
        .await
        .context("asking GitHub which release is current")?;

    // The toggle is a person answering, and it outranks the probe in both
    // directions. `Manifest::planned` only adds the app where the machine
    // *looks* graphical (`systemctl get-default`), which is a good default and a
    // bad veto: on a box that boots to a console but has a desktop session (a Deck
    // in some modes, a container, a minimal WM without systemd) the user ticks
    // "install the desktop app" and silently doesn't get it. Same shape as
    // `hoard install --with-desktop` and `--headless`.
    let mut manifest = Manifest::planned(&version, &probe);
    manifest.version = version.clone();
    if want_desktop {
        manifest.add(Component::Desktop);
        if manifest.delivery.is_none() {
            manifest.delivery = Some(install::resolve_delivery(&probe));
        }
    } else {
        manifest.components.retain(|c| *c != Component::Desktop);
        manifest.delivery = None;
    }

    // ---- the core ---------------------------------------------------------
    let dir = core_dir()?;
    let core = fetch::core_asset_for(&version, &assets).with_context(|| {
        format!("release {version} has no core build for this OS and architecture")
    })?;
    let tarball = fetch::download_verified(core, &assets, &stage)
        .await
        .context("downloading the sync engine")?;
    fetch::apply_core(&tarball, &dir)
        .await
        .context("installing the sync engine")?;
    manifest.core_dir = Some(dir.clone());

    // Having the binary and not being able to type its name is, in practice,
    // not having it. Best-effort: an install doesn't fail over a shell profile.
    let _ = install::ensure_on_shell_path(&dir);

    // ---- the service
    // Which `hoardd` the unit points at was settled in `main`, before any thread
    // existed; see [`daemon_binary`].
    //
    // Not fatal: a machine whose service manager refuses still has a working
    // Hoard. What it loses is starting at login, and `hoard install` fixes that
    // later without downloading anything again.
    let _ = hoardd::autostart::install().await;

    // ---- the app ----------------------------------------------------------
    let mut launch = None;
    if manifest.has(Component::Desktop) {
        let delivery = install::resolve_delivery(&probe);
        manifest.delivery = Some(delivery);

        let asset = fetch::asset_for(delivery, &assets).with_context(|| {
            format!(
                "release {version} has no {} build for this machine",
                delivery.as_str()
            )
        })?;
        let package = fetch::download_verified(asset, &assets, &stage)
            .await
            .context("downloading the desktop app")?;
        // `false`: there is a person in front of this window, so a package that
        // needs privileges may raise the system's own password dialog.
        let path = fetch::apply_desktop(delivery, &package, false)
            .await
            .context("installing the desktop app")?;
        manifest.desktop_path = Some(path.clone());
        launch = Some(path);
    }

    manifest.save().context("recording what was installed")?;
    let _ = std::fs::remove_dir_all(&stage);

    Ok(Outcome { launch })
}

/// What this machine already has, if anything.
///
/// Cheap and synchronous (it only looks at the filesystem) so `main` can ask
/// before the window is even shown and the first screen already knows whether
/// this is an install or a second visit.
pub fn detect() -> Option<install::Installed> {
    let hint = core_dir().ok()?;
    install::detect(&hint)
}

/// Take Hoard off this machine.
///
/// Returns where the user's own things still are: **nothing under those paths
/// is touched**. Saves are the reason someone used Hoard in the first place and
/// there can be tens of gigabytes of them; an uninstaller that removes the
/// program is doing its job, one that removes the backups is destroying the
/// thing it was protecting.
///
/// The order is the whole trick. The service goes first, because a manager that
/// still has a unit will start the daemon again the moment the socket goes quiet,
/// and then the binary we are about to delete is in use.
pub async fn uninstall(found: &install::Installed) -> Result<Vec<PathBuf>> {
    // Same marker as an install: it stops any client that notices the missing
    // socket from starting a daemon out of the binaries we are deleting.
    let _swap = Swap::begin();

    // Not fatal on its own: a machine with no unit installed answers `false`
    // and there is still an app and two binaries to take away.
    let _ = hoardd::autostart::uninstall().await;
    remove::stop_running().await;

    if let Some(path) = &found.desktop {
        let delivery = found
            .delivery
            .unwrap_or_else(|| install::resolve_delivery(&Probe::read()));
        remove::desktop(delivery, Some(path.as_path()), false)
            .await
            .context("removing the desktop app")?;
    }

    if let Some(dir) = &found.core_dir {
        remove::core(dir).context("removing the sync engine")?;
        remove::shell_path_line(dir);
    }
    remove::manifest().context("clearing the install record")?;

    Ok(remove::kept_data())
}
