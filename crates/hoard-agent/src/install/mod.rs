//! Which pieces of Hoard this machine needs, where they live, and how they update
//! together.
//!
//! Hoard is not a program: it is an engine (`hoardd`) and two faces that drive it,
//! the terminal (`hoard`) and the app (`hoard-desktop`). Two components:
//!
//! - [`Component::Core`] is `hoardd` plus `hoard`. Never one without the other, and
//!   the only mandatory piece: with no engine there is no product.
//! - [`Component::Desktop`] is the graphical app. Optional, and only where there is
//!   something to show; a NAS does not want WebKitGTK.
//!
//! They install and update together, at the same version, or nothing gets touched.
//! A `hoard` 1.2 talking to a `hoardd` 1.1 is worse than not having updated: the
//! handshake tolerates it (see `hoard_core::ipc`), so the mismatch says nothing and
//! merely behaves oddly.
//!
//! ## One copy of each, in one place
//!
//! [`layout`] is the only answer to "where does Hoard live": the core in the
//! user's own directory, the app beside it as a file Hoard Setup puts there.
//! Up to 1.2.x every app package carried a second core inside it, and a machine
//! that went through two front doors kept two engines and ran whichever the
//! service had been pointed at first. [`legacy`] finds what those installs left
//! and takes it off; installing and updating both go through it.
//!
//! ## Detected once; after that the manifest decides
//!
//! Which components are needed is decided on the first install ([`Probe`] plus
//! [`resolve_components`]) and recorded in the [`Manifest`]. From then on the file
//! decides, and an update updates *what is there* without weighing in again: a
//! `hoard upgrade` over SSH against your desktop machine sees no graphical
//! environment, and a detection that re-ran would take the app away from you for
//! having updated from a console.

pub mod auto;
pub mod fetch;
pub mod layout;
pub mod legacy;
pub mod remove;
pub mod stage;
#[cfg(windows)]
pub mod windows;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// An installable piece. The order matters: [`Component::Core`] is always
/// installed and updated first, because the others depend on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Component {
    /// `hoardd` + `hoard`. Obligatorio.
    Core,
    /// The graphical app (`hoard-desktop`).
    Desktop,
}

impl Component {
    /// The name for logs and for the manifest.
    pub fn as_str(self) -> &'static str {
        match self {
            Component::Core => "core",
            Component::Desktop => "desktop",
        }
    }
}

/// How the graphical app arrived on this machine. It decides who updates it and
/// how it comes off.
///
/// Three routes are current and need nobody's password: [`Delivery::AppImage`],
/// [`Delivery::Exe`] and [`Delivery::Dmg`]. The rest are what older releases
/// installed; they are still read from old manifests, and only ever removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Delivery {
    /// Before 1.2.2: `.deb` via `dpkg`/`apt`, with the core inside it.
    Deb,
    /// Before 1.2.2: `.rpm`, with the core inside it.
    Rpm,
    /// An AppImage in the home directory (Linux).
    ///
    /// The `rename` is not cosmetic: `snake_case` over `AppImage` gives
    /// `app_image`, and the wire shape and [`Delivery::as_str`] disagreeing would
    /// be a silent mismatch in every manifest already written.
    #[serde(rename = "appimage")]
    AppImage,
    /// Before 1.2.2: Tauri's NSIS installer (Windows), with the core inside it.
    Nsis,
    /// Before 1.2.2: Tauri's per-machine MSI (Windows).
    Msi,
    /// `hoard-desktop.exe` in the user's AppData, put there by Hoard Setup
    /// (Windows).
    Exe,
    /// The `.app` copied out of the `.dmg` into `/Applications` (macOS).
    Dmg,
    /// Installed and maintained by a third party: Flatpak, a distro package, a
    /// `nix`. We update nothing here; it says so and exits.
    Managed,
}

impl Delivery {
    pub fn as_str(self) -> &'static str {
        match self {
            Delivery::Deb => "deb",
            Delivery::Rpm => "rpm",
            Delivery::AppImage => "appimage",
            Delivery::Nsis => "nsis",
            Delivery::Msi => "msi",
            Delivery::Exe => "exe",
            Delivery::Dmg => "dmg",
            Delivery::Managed => "managed",
        }
    }

    /// Do we update it? `false` for what a third party maintains.
    pub fn is_ours(self) -> bool {
        !matches!(self, Delivery::Managed)
    }

    /// What an older release installed and this one only takes off.
    pub fn is_legacy(self) -> bool {
        matches!(
            self,
            Delivery::Deb | Delivery::Rpm | Delivery::Nsis | Delivery::Msi
        )
    }

    /// A `.deb` or an `.rpm`. Taking one off needs a password, so the person
    /// is asked once and may keep it: then it stays the app, and only the core
    /// is updated from here.
    pub fn is_package(self) -> bool {
        matches!(self, Delivery::Deb | Delivery::Rpm)
    }
}

// ---- what the system tells us

/// The system facts that decide the plan, gathered in one go so the policy
/// ([`resolve_components`], [`resolve_delivery`]) stays pure and testable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Probe {
    /// Does this machine boot into graphical mode? Not "do I have a screen right
    /// now?"; see [`graphical`].
    pub graphical: bool,
    /// Are we inside a Flatpak? See [`running_under_flatpak`].
    pub sandboxed: bool,
}

impl Probe {
    pub fn read() -> Self {
        Self {
            graphical: graphical(),
            sandboxed: running_under_flatpak(),
        }
    }
}

/// Are we running inside a Flatpak?
///
/// Both signals are the ones `flatpak` itself documents: it exports `FLATPAK_ID`
/// into the sandbox, and mounts `/.flatpak-info` there. The file is what makes
/// this true for our sidecars as well: `hoardd` is started by the app, and a
/// child that had its environment scrubbed would still see the mount.
pub fn running_under_flatpak() -> bool {
    std::env::var_os("FLATPAK_ID").is_some() || Path::new("/.flatpak-info").exists()
}

/// Is this a machine that shows a window?
///
/// The question is NOT whether there is a screen at this instant, which is why
/// `$DISPLAY` and `$WAYLAND_DISPLAY` are not consulted: SSHing into your laptop to
/// update it does not turn the laptop into a server, but those variables say it
/// does. What is consulted is what the system boots into (`systemctl get-default`),
/// which is a property of the machine rather than of the session you are asking
/// from.
///
/// Windows and macOS are graphical by construction.
#[cfg(target_os = "linux")]
fn graphical() -> bool {
    if let Ok(out) = std::process::Command::new("systemctl")
        .arg("get-default")
        .output()
    {
        let target = String::from_utf8_lossy(&out.stdout);
        let target = target.trim();
        if !target.is_empty() {
            return target.starts_with("graphical");
        }
    }
    // With no systemd (a container, an alternative init): are there desktop
    // sessions installed? It is weaker, but we only get here when the good signal
    // does not exist.
    ["/usr/share/xsessions", "/usr/share/wayland-sessions"]
        .iter()
        .any(|d| {
            std::fs::read_dir(d)
                .map(|mut e| e.next().is_some())
                .unwrap_or(false)
        })
}

#[cfg(not(target_os = "linux"))]
fn graphical() -> bool {
    true
}

// ---- policy (pure)

/// Which components this machine needs on a fresh install.
///
/// [`Component::Core`] always. The app only where there is something to show: it is
/// the difference between the NAS, which keeps engine and terminal without dragging
/// in WebKitGTK, and the Deck, which takes both faces in one pass.
pub fn resolve_components(probe: &Probe) -> Vec<Component> {
    let mut out = vec![Component::Core];
    if probe.graphical {
        out.push(Component::Desktop);
    }
    out
}

/// How the app reaches this machine. One route per system, and none of them
/// asks for a password: that is what lets an update apply with nobody there.
///
/// Inside a Flatpak nothing is ours to replace: `/app` is read-only and the next
/// version comes from the remote the user installed from.
pub fn resolve_delivery(probe: &Probe) -> Delivery {
    if probe.sandboxed {
        return Delivery::Managed;
    }
    if cfg!(windows) {
        Delivery::Exe
    } else if cfg!(target_os = "macos") {
        Delivery::Dmg
    } else {
        Delivery::AppImage
    }
}

// ---- the manifest

/// What is installed on this machine: which components, at which version and by
/// which route. It is what turns "install" and "update" into the same operation
/// seen from two different moments.
///
/// Only the installer writes it. Nothing deduces it from the disk any more: the
/// last thing that did saw `/usr/bin/hoard-desktop` next to `/usr/bin/hoardd`,
/// decided the package owned the core, kept pointing at the copy in
/// `~/.local/bin`, and from then on every update relieved the package and none
/// touched the engine the service actually ran.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    /// The version every component was left at by the last operation.
    pub version: String,
    /// What is installed. Sorted and deduplicated.
    pub components: Vec<Component>,
    /// The graphical app's route. `None` when there is no `Desktop`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery: Option<Delivery>,
    /// Where `hoard` and `hoardd` live.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub core_dir: Option<PathBuf>,
    /// The app's executable (or bundle, on macOS), when we know it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desktop_path: Option<PathBuf>,
}

impl Manifest {
    /// `<config>/install.json`. Alongside the rest of the user's config, and per
    /// user: two accounts on the same machine can have different installs.
    pub fn path() -> Result<PathBuf> {
        Ok(crate::config::CliConfig::project_dirs()?
            .config_dir()
            .join("install.json"))
    }

    /// Reads the manifest. `Ok(None)` when there is none yet.
    pub fn load() -> Result<Option<Self>> {
        let path = Self::path()?;
        let Ok(text) = std::fs::read_to_string(&path) else {
            return Ok(None);
        };
        let m =
            serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        Ok(Some(m))
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        let text = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, text).with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }

    /// Is this component installed?
    pub fn has(&self, c: Component) -> bool {
        self.components.contains(&c)
    }

    /// Adds a component (idempotent, keeping the order).
    pub fn add(&mut self, c: Component) {
        if !self.has(c) {
            self.components.push(c);
            self.components.sort();
        }
    }

    /// The manifest of a fresh install on this machine.
    pub fn planned(version: &str, probe: &Probe) -> Self {
        let components = resolve_components(probe);
        let delivery = components
            .contains(&Component::Desktop)
            .then(|| resolve_delivery(probe));
        Self {
            version: version.to_string(),
            components,
            delivery,
            core_dir: None,
            desktop_path: None,
        }
    }

    /// Is everything this manifest names where [`layout`] says it goes?
    pub fn is_in_layout(&self) -> bool {
        let core_ok = match (&self.core_dir, layout::core_dir()) {
            (Some(recorded), Ok(core)) => same_dir(recorded, &core),
            _ => false,
        };
        let app_ok = !self.has(Component::Desktop)
            || match self.delivery {
                // A package the person chose to keep is where they put it.
                Some(d) if d.is_package() => self.desktop_path.as_ref().is_some_and(|p| p.exists()),
                Some(d) => !d.is_legacy() && layout::app_path().is_ok_and(|p| p.exists()),
                None => false,
            };
        core_ok && app_ok
    }
}

/// The manifest for this machine, or one for the install we are running from
/// when there is none.
///
/// A missing file is not proof of no install: 1.0.x wrote none, and a manifest
/// can be deleted. But only the layout counts as evidence. The process asking has
/// to be *the* core (`hoard` or `hoardd` in [`layout::core_dir`]): a build run
/// out of `target/debug` is not an install, and recording it as one is how a
/// development tree once got an updater aimed at it.
pub fn adopt() -> Option<Manifest> {
    if let Ok(Some(m)) = Manifest::load() {
        return Some(m);
    }
    let exe = std::env::current_exe().ok()?;
    if !layout::is_core(&exe) {
        return None;
    }
    let probe = Probe::read();
    let mut m = Manifest::planned(crate::update::current(), &probe);
    m.components = vec![Component::Core];
    m.delivery = None;
    m.core_dir = layout::core_dir().ok();
    if let Ok(app) = layout::app_path() {
        if app.exists() {
            m.add(Component::Desktop);
            m.delivery = Some(resolve_delivery(&probe));
            m.desktop_path = Some(app);
        }
    }
    Some(m)
}

fn same_dir(a: &Path, b: &Path) -> bool {
    let canonical = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    canonical(a) == canonical(b)
}

// ---- the swap window: "don't start me right now"

/// How long a swap marker is believed before it's treated as debris.
///
/// It has to outlive the slowest install we drive (removing an old NSIS copy,
/// then downloading a ~100 MB AppImage on a slow line) and still be short enough
/// that a machine which crashed mid-swap isn't left unable to start its own
/// service for long. Ten minutes is both.
const SWAP_WINDOW: std::time::Duration = std::time::Duration::from_secs(10 * 60);

/// **The binaries are being replaced right now, so don't launch one.**
///
/// The client rule since Slice 4 is "spawn if absent" (`Client::ensure_running`
/// in `hoardd`): lose the socket, start a service. That rule and an installer
/// are a race: the installer stops `hoardd`, a client notices the socket is gone
/// two seconds later and starts it again from the *old* binary, and on Windows
/// the installer then hits a file that's back in use. This closes that window
/// for every client, including the ones nobody can kill by name (a `hoard`
/// invocation from a terminal, a second desktop session).
///
/// It's a file and not a flag in memory because the process that must not spawn
/// a daemon is usually not the process that started the install. Held for the
/// length of an install or an update and dropped with the guard; any `hoardd`
/// that manages to start clears it, since a live service *is* the proof the swap
/// is over.
pub struct Swap {
    path: Option<PathBuf>,
}

impl Swap {
    fn path() -> Result<PathBuf> {
        Ok(crate::config::CliConfig::state_dir()?.join("swapping-binaries"))
    }

    /// Mark the swap as started. Best-effort: a machine where we can't write
    /// this still gets updated, it just keeps the old race.
    pub fn begin() -> Self {
        let path = Self::path().ok().filter(|path| {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            std::fs::write(path, b"").is_ok()
        });
        if path.is_none() {
            tracing::debug!(
                "install: couldn't write the swap marker; clients may respawn mid-swap"
            );
        }
        Self { path }
    }

    /// Is someone replacing the binaries right now?
    pub fn in_progress() -> bool {
        let Ok(path) = Self::path() else {
            return false;
        };
        let Ok(meta) = std::fs::metadata(&path) else {
            return false;
        };
        match meta.modified().map(|at| at.elapsed()) {
            // A marker from the future (clock jump, a copied home directory) is
            // still a marker: the safe reading is "wait", because the worst case
            // is a client that waits for a service it could have started,
            // against a corrupted install.
            Ok(Err(_)) | Err(_) => true,
            Ok(Ok(age)) => age < SWAP_WINDOW,
        }
    }

    /// Drop the marker without holding a guard. This is what a starting
    /// `hoardd` calls: if it's running, the swap it would have blocked already
    /// finished (or never got past the installer that killed its predecessor).
    pub fn forget() {
        if let Ok(path) = Self::path() {
            let _ = std::fs::remove_file(path);
        }
    }
}

impl Drop for Swap {
    fn drop(&mut self) {
        if let Some(path) = &self.path {
            let _ = std::fs::remove_file(path);
        }
    }
}

// ---- making the terminal typable

/// Puts `dir` on the `PATH` of the user's **future** terminals, and says whether
/// it touched anything.
///
/// Having the binary and not being able to type its name is, in practice, not
/// having it. Best-effort and idempotent: an install doesn't fail over this, and
/// calling it twice doesn't leave the line in the file twice.
pub fn ensure_on_shell_path(dir: &Path) -> Result<PathNudge> {
    if std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|d| d == dir))
        .unwrap_or(false)
    {
        return Ok(PathNudge::AlreadyThere);
    }
    shell_path(dir)
}

/// What [`ensure_on_shell_path`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathNudge {
    /// The directory was already there. Nothing to do.
    AlreadyThere,
    /// Written to the shell's startup file (or to the registry, on Windows). A
    /// new terminal is needed to see it.
    Added(PathBuf),
    /// Couldn't, with the reason. Not fatal to the install.
    Skipped(String),
}

/// Windows: the user's `PATH` lives in the registry (`HKCU\Environment`).
#[cfg(target_os = "windows")]
fn shell_path(dir: &Path) -> Result<PathNudge> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let env = hkcu
        .open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)
        .context("opening HKCU\\Environment")?;
    let current: String = env.get_value("Path").unwrap_or_default();
    let dir_str = dir.to_string_lossy().to_string();
    if current
        .split(';')
        .any(|p| p.trim().eq_ignore_ascii_case(dir_str.trim()))
    {
        return Ok(PathNudge::AlreadyThere);
    }
    let next = if current.trim().is_empty() {
        dir_str.clone()
    } else {
        format!("{};{}", current.trim_end_matches(';'), dir_str)
    };
    env.set_value("Path", &next)
        .context("writing HKCU\\Environment\\Path")?;
    broadcast_environment_change();
    Ok(PathNudge::Added(dir.to_path_buf()))
}

/// Tells the system the environment changed.
///
/// Writing the registry is not enough, and it is the kind of failure that looks like
/// it works in a test: the value is stored correctly, but Explorer keeps its
/// environment block cached and every terminal it launches inherits the old one, so
/// broadcasting `"Environment"` is what makes a new console see it.
///
/// With a timeout and `SMTO_ABORTIFHUNG` because it goes to *every* top-level window:
/// a hung application must not keep our thread.
#[cfg(target_os = "windows")]
fn broadcast_environment_change() {
    use windows_sys::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SendMessageTimeoutW, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE,
    };

    let param: Vec<u16> = "Environment\0".encode_utf16().collect();
    // SAFETY: `HWND_BROADCAST` is valid, and `param` lives for the whole call (it is
    // synchronous with a 5 s cap). The result is unused: this is best-effort, and if
    // nobody answers the PATH is still written.
    unsafe {
        SendMessageTimeoutW(
            HWND_BROADCAST as HWND,
            WM_SETTINGCHANGE,
            0 as WPARAM,
            param.as_ptr() as LPARAM,
            SMTO_ABORTIFHUNG,
            5000,
            std::ptr::null_mut(),
        );
    }
}

/// Unix: the line into the login shell's startup file, same as `install.sh`.
///
/// Picked by `$SHELL` rather than by what happens to exist in the home
/// directory: writing to the `.bashrc` of someone who uses zsh is writing to a
/// file nobody reads. Without `$SHELL`, which an app launched from the desktop menu
/// may not have, it falls to `.profile`, which every login shell reads.
#[cfg(unix)]
fn shell_path(dir: &Path) -> Result<PathNudge> {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return Ok(PathNudge::Skipped("no HOME in the environment".into()));
    };
    let shell = std::env::var("SHELL").unwrap_or_default();
    let rc = if shell.ends_with("/zsh") {
        home.join(".zshrc")
    } else if shell.ends_with("/bash") {
        home.join(".bashrc")
    } else {
        home.join(".profile")
    };

    let line = format!("export PATH=\"{}:$PATH\"", dir.display());
    let existing = std::fs::read_to_string(&rc).unwrap_or_default();
    if existing.lines().any(|l| l.trim() == line) {
        return Ok(PathNudge::Added(rc));
    }
    let block = format!("\n# Added by the Hoard installer\n{line}\n");
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&rc)
        .and_then(|mut f| std::io::Write::write_all(&mut f, block.as_bytes()))
    {
        Ok(()) => Ok(PathNudge::Added(rc)),
        Err(e) => Ok(PathNudge::Skipped(format!("{}: {e}", rc.display()))),
    }
}

#[cfg(not(any(unix, target_os = "windows")))]
fn shell_path(_dir: &Path) -> Result<PathNudge> {
    Ok(PathNudge::Skipped("unsupported platform".into()))
}

// ---- what is here already

/// What this machine already has, asked by an installer.
///
/// The manifest when there is one, the layout when there isn't, and whatever an
/// older release left either way: a machine with only a `.deb` from 1.2.1 on it
/// is a machine with Hoard installed, and the installer's first screen has to
/// say so.
pub async fn detect() -> Option<Installed> {
    let manifest = Manifest::load().ok().flatten();
    let core_dir = layout::core_dir().ok().filter(|d| {
        d.join(format!("hoardd{}", std::env::consts::EXE_SUFFIX))
            .is_file()
    });
    let desktop = manifest
        .as_ref()
        .and_then(|m| m.desktop_path.clone())
        .filter(|p| p.exists())
        .or_else(|| layout::app_path().ok().filter(|p| p.exists()));
    let leftovers = legacy::scan().await;
    let legacy_app = leftovers.iter().any(|l| {
        matches!(
            l,
            legacy::Leftover::Package { .. }
                | legacy::Leftover::Nsis { .. }
                | legacy::Leftover::Msi { .. }
        )
    });

    if core_dir.is_none() && desktop.is_none() && leftovers.is_empty() && manifest.is_none() {
        return None;
    }
    let version = match &manifest {
        Some(m) => Some(m.version.clone()),
        None => match &core_dir {
            Some(dir) => core_version(dir).await,
            None => None,
        },
    };
    Some(Installed {
        version,
        core_dir,
        has_desktop: desktop.is_some() || legacy_app,
        manifest,
        leftovers,
    })
}

/// The version of the core in `dir`, asked of its `hoard`: `hoardd` is a
/// windowless program on Windows and prints nothing there.
pub async fn core_version(dir: &Path) -> Option<String> {
    let exe = dir.join(format!("hoard{}", std::env::consts::EXE_SUFFIX));
    let run = tokio::process::Command::new(exe)
        .arg("--version")
        .stdin(std::process::Stdio::null())
        .output();
    let out = tokio::time::timeout(std::time::Duration::from_secs(5), run)
        .await
        .ok()?
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    text.split_whitespace().nth(1).map(str::to_string)
}

/// The answer from [`detect`].
#[derive(Debug, Clone)]
pub struct Installed {
    /// What the manifest (or the core itself) says is installed.
    pub version: Option<String>,
    /// Where `hoard` and `hoardd` are, when they are in the layout.
    pub core_dir: Option<PathBuf>,
    /// The app is here, in the layout or in an older install.
    pub has_desktop: bool,
    /// The manifest itself, when there is one.
    pub manifest: Option<Manifest>,
    /// What older releases left.
    pub leftovers: Vec<legacy::Leftover>,
}

impl Installed {
    /// Is the graphical app part of this?
    pub fn has_desktop(&self) -> bool {
        self.has_desktop
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An "ordinary desktop" probe to vary a single fact per test.
    fn desktop_box() -> Probe {
        Probe {
            graphical: true,
            sandboxed: false,
        }
    }

    #[test]
    fn a_headless_box_gets_the_core_and_nothing_else() {
        let probe = Probe {
            graphical: false,
            ..desktop_box()
        };
        assert_eq!(resolve_components(&probe), vec![Component::Core]);
    }

    #[test]
    fn a_graphical_box_gets_both_faces_in_one_pass() {
        assert_eq!(
            resolve_components(&desktop_box()),
            vec![Component::Core, Component::Desktop]
        );
    }

    /// The core is optional in no combination: a face with no engine is exactly
    /// the broken artifact this module exists to eliminate.
    #[test]
    fn the_core_is_never_optional() {
        for graphical in [true, false] {
            let probe = Probe {
                graphical,
                ..desktop_box()
            };
            assert!(resolve_components(&probe).contains(&Component::Core));
        }
    }

    /// One route per system, and never one of the packages: those are what
    /// this release moves machines off.
    #[test]
    fn a_fresh_install_never_picks_an_old_route() {
        let d = resolve_delivery(&desktop_box());
        assert!(!d.is_legacy(), "{d:?}");
        assert!(d.is_ours());
        if cfg!(target_os = "linux") {
            assert_eq!(d, Delivery::AppImage);
        }
    }

    /// The sandbox outranks everything: `/app` is read-only and the next version
    /// comes from the Flatpak remote.
    #[test]
    fn a_flatpak_is_not_ours_to_update() {
        let probe = Probe {
            sandboxed: true,
            ..desktop_box()
        };
        assert_eq!(resolve_delivery(&probe), Delivery::Managed);
        assert!(!resolve_delivery(&probe).is_ours());
    }

    #[test]
    fn a_headless_plan_records_no_delivery() {
        let probe = Probe {
            graphical: false,
            ..desktop_box()
        };
        let m = Manifest::planned("1.2.0", &probe);
        assert!(!m.has(Component::Desktop));
        assert_eq!(m.delivery, None);
    }

    #[test]
    fn adding_a_component_is_idempotent() {
        let mut m = Manifest::planned("1.2.0", &desktop_box());
        let before = m.components.clone();
        m.add(Component::Core);
        m.add(Component::Desktop);
        assert_eq!(m.components, before);
    }

    #[test]
    fn the_old_routes_are_legacy_and_the_new_ones_are_not() {
        for d in [Delivery::Deb, Delivery::Rpm, Delivery::Nsis, Delivery::Msi] {
            assert!(d.is_legacy(), "{d:?}");
        }
        for d in [
            Delivery::AppImage,
            Delivery::Exe,
            Delivery::Dmg,
            Delivery::Managed,
        ] {
            assert!(!d.is_legacy(), "{d:?}");
        }
        assert!(!Delivery::Managed.is_ours());
    }

    #[test]
    fn the_manifest_round_trips_through_its_wire_shape() {
        let m = Manifest {
            version: "1.2.2".into(),
            components: vec![Component::Core, Component::Desktop],
            delivery: Some(Delivery::AppImage),
            core_dir: Some(PathBuf::from("/home/ada/.local/bin")),
            desktop_path: Some(PathBuf::from("/home/ada/.local/bin/hoard-desktop")),
        };
        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains(r#""components":["core","desktop"]"#));
        assert!(json.contains(r#""delivery":"appimage""#));
        assert_eq!(serde_json::from_str::<Manifest>(&json).unwrap(), m);
    }

    /// The manifest on the machine that started all this: written by 1.2.1, with
    /// the field that was wrong about who owned the core. It still has to parse,
    /// and it has to read as an install to be moved, not one to trust.
    #[test]
    fn a_manifest_from_1_2_1_parses_and_reads_as_legacy() {
        let m: Manifest = serde_json::from_str(
            r#"{"version":"1.2.1","components":["core","desktop"],"delivery":"deb",
                "core_dir":"/home/insider/.local/bin",
                "desktop_path":"/usr/bin/hoard-desktop","core_from_bundle":true}"#,
        )
        .unwrap();
        assert_eq!(m.delivery, Some(Delivery::Deb));
        assert!(m.delivery.unwrap().is_legacy());
        assert!(!m.is_in_layout());
    }

    #[test]
    fn every_delivery_serialises_as_the_name_it_prints() {
        for d in [
            Delivery::Deb,
            Delivery::Rpm,
            Delivery::AppImage,
            Delivery::Nsis,
            Delivery::Msi,
            Delivery::Exe,
            Delivery::Dmg,
            Delivery::Managed,
        ] {
            assert_eq!(
                serde_json::to_string(&d).unwrap(),
                format!("\"{}\"", d.as_str()),
                "{d:?} disagrees with its own as_str()"
            );
        }
    }

    #[test]
    fn every_component_serialises_as_the_name_it_prints() {
        for c in [Component::Core, Component::Desktop] {
            assert_eq!(
                serde_json::to_string(&c).unwrap(),
                format!("\"{}\"", c.as_str())
            );
        }
    }

    /// An install with no app writes no `delivery`; the absent field has to read
    /// as "there is none", not blow up the parse.
    #[test]
    fn a_manifest_without_a_desktop_parses() {
        let m: Manifest =
            serde_json::from_str(r#"{"version":"1.2.0","components":["core"]}"#).unwrap();
        assert_eq!(m.components, vec![Component::Core]);
        assert_eq!(m.delivery, None);
        assert_eq!(m.core_dir, None);
    }
}
