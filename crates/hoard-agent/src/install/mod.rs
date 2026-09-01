//! Which pieces of Hoard this machine needs, and how they update together.
//!
//! Hoard is not a program: it is an engine (`hoardd`) and two faces that drive it,
//! the terminal (`hoard`) and the app (`hoard-desktop` plus `hoard-screen`). Until
//! now they were published cut along the wrong axis, "CLI" against "desktop", and
//! each cut left out something essential: the tarball carried a terminal with no
//! engine (which cannot start) and the bundle carried an engine with no terminal.
//! This module cuts along the right axis, components:
//!
//! - [`Component::Core`] is `hoardd` plus `hoard`. Never one without the other, and
//!   the only mandatory piece: with no engine there is no product, and a face with
//!   no engine is a binary that can do nothing.
//! - [`Component::Desktop`] is the graphical app. Optional, and only where there is
//!   something to show; a NAS does not want WebKitGTK.
//!
//! The rule that governs everything here: they install and update together, at the
//! same version, or nothing gets touched. A `hoard` 1.2 talking to a `hoardd` 1.1
//! is worse than not having updated: the handshake tolerates it (see
//! `hoard_core::ipc`), so the mismatch says nothing and merely behaves oddly.
//!
//! ## Detected once; after that the manifest decides
//!
//! Which components are needed is decided on the first install ([`Probe`] plus
//! [`resolve_components`]) and recorded in the [`Manifest`]. From then on the file
//! decides, and an update updates *what is there* without weighing in again. That
//! is not a detail: a `hoard upgrade` over SSH against your desktop machine sees no
//! graphical environment, and a detection that re-ran would conclude "the app does
//! not belong here", taking it away from you for having updated from a console.
//!
//! ## The engine is a component, not a passenger
//!
//! `hoardd` used to travel inside the desktop's bundle as a sidecar. That is what
//! stops an AppImage starting sync at login: its binary lives on an ephemeral mount
//! (`/tmp/.mount_XXXX/...`) that does not exist on the next boot, which is why
//! [`crate::install`] exists. Installed as a component in its own right, on a
//! stable path, the AppImage stays the graphical face and the engine starts at boot
//! just as it would from a native package.

pub mod auto;
pub mod fetch;
pub mod remove;
pub mod stage;

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
    /// The graphical app (`hoard-desktop` and its `hoard-screen` overlay).
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

/// How the graphical app arrived, or will arrive, on this machine. It determines
/// who updates it: a native package is relieved by its installer, an AppImage is
/// replaced by us, and [`Delivery::Managed`] is not touched at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Delivery {
    /// `.deb` via `dpkg`/`apt`.
    Deb,
    /// `.rpm` via `rpm`/`dnf`.
    Rpm,
    /// An AppImage in the user's directory, with no privileges.
    ///
    /// The `rename` is not cosmetic: `snake_case` over `AppImage` gives
    /// `app_image`, and this field is read and written by the shell installers,
    /// which look for the literal string. The wire shape and [`Delivery::as_str`]
    /// disagreeing would be a silent mismatch.
    #[serde(rename = "appimage")]
    AppImage,
    /// Instalador NSIS (Windows).
    Nsis,
    /// `.dmg` arrastrado a `/Applications` (macOS).
    Dmg,
    /// Installed and maintained by a third party: the distro's package manager,
    /// Flatpak, a `nix`. We update nothing here; it says so and exits.
    Managed,
}

impl Delivery {
    pub fn as_str(self) -> &'static str {
        match self {
            Delivery::Deb => "deb",
            Delivery::Rpm => "rpm",
            Delivery::AppImage => "appimage",
            Delivery::Nsis => "nsis",
            Delivery::Dmg => "dmg",
            Delivery::Managed => "managed",
        }
    }

    /// Do we update it? `false` for what a third party maintains.
    pub fn is_ours(self) -> bool {
        !matches!(self, Delivery::Managed)
    }

    /// Does it need privileges to be applied?
    pub fn needs_elevation(self) -> bool {
        matches!(self, Delivery::Deb | Delivery::Rpm)
    }
}

// ---- what the system tells us

/// The system facts that decide the plan, gathered in one go so the policy
/// ([`resolve_components`], [`resolve_delivery`]) stays pure and testable without a
/// NAS, a Deck and three distros in front of you.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Probe {
    /// Does this machine boot into graphical mode? Not "do I have a screen right
    /// now?"; see [`graphical`].
    pub graphical: bool,
    /// A read-only root (SteamOS, Bazzite and the other atomic images): the native
    /// package manager cannot write even where it exists.
    pub immutable_root: bool,
    /// `dpkg` disponible.
    pub has_dpkg: bool,
    /// `rpm` disponible.
    pub has_rpm: bool,
    /// Can we elevate privileges without hanging on a human?
    /// Ver [`can_elevate`].
    pub can_elevate: bool,
    /// Are we inside a Flatpak? See [`running_under_flatpak`].
    pub sandboxed: bool,
}

impl Probe {
    /// Interrogates the system. All best-effort: any signal that cannot be read
    /// counts as "no", and the worst case of being wrong is falling back to the
    /// AppImage, which works everywhere.
    pub fn read() -> Self {
        Self {
            graphical: graphical(),
            immutable_root: immutable_root(),
            has_dpkg: bin_exists("dpkg"),
            has_rpm: bin_exists("rpm"),
            can_elevate: can_elevate(),
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

/// Is `name` on the `PATH`?
fn bin_exists(name: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| {
            std::env::split_paths(&paths).any(|d| {
                let p = d.join(name);
                p.is_file() || p.with_extension("exe").is_file()
            })
        })
        .unwrap_or(false)
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

/// An immutable root: SteamOS, Bazzite and the rest of the atomic images. They
/// have `rpm` on the `PATH` and `dnf install` still writes nothing, so without this
/// check the plan would pick a native package that cannot be applied.
#[cfg(target_os = "linux")]
fn immutable_root() -> bool {
    // The tools give the image away before any mount does.
    if bin_exists("rpm-ostree") || bin_exists("steamos-readonly") {
        return true;
    }
    let Ok(mounts) = std::fs::read_to_string("/proc/mounts") else {
        return false;
    };
    mounts.lines().any(|line| {
        let mut f = line.split_whitespace();
        let (Some(_dev), Some(point), Some(_fs), Some(opts)) =
            (f.next(), f.next(), f.next(), f.next())
        else {
            return false;
        };
        (point == "/" || point == "/usr") && opts.split(',').any(|o| o == "ro")
    })
}

#[cfg(not(target_os = "linux"))]
fn immutable_root() -> bool {
    false
}

/// Can we elevate without blocking to wait for a human?
///
/// That nuance is what makes this work inside a `curl ... | sh`: there the script's
/// stdin is the script itself, so a `sudo` that asks for a password has nobody to
/// ask and either hangs or fails ugly. Only the routes that resolve themselves
/// count: already being root, a `sudo` with a cached credential (`-n`), or `pkexec`
/// with a graphical session, which opens its own dialog and does not depend on this
/// terminal.
#[cfg(unix)]
fn can_elevate() -> bool {
    // SAFETY: `geteuid` takes no arguments, cannot fail and touches no memory of
    // ours.
    if unsafe { libc::geteuid() } == 0 {
        return true;
    }
    if bin_exists("sudo") {
        let cached = std::process::Command::new("sudo")
            .args(["-n", "true"])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if cached {
            return true;
        }
    }
    // `pkexec` does have somebody to ask: it draws its own dialog in the graphical
    // session rather than in this pipe. With no session to draw it in it is no use,
    // and here `$DISPLAY` and `$WAYLAND_DISPLAY` do count: the question is exactly
    // the one those variables answer well, "is there a screen right now?".
    let has_session =
        std::env::var_os("DISPLAY").is_some() || std::env::var_os("WAYLAND_DISPLAY").is_some();
    bin_exists("pkexec") && has_session
}

#[cfg(not(unix))]
fn can_elevate() -> bool {
    // On Windows the installer elevates itself (UAC), not us.
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

/// How to deliver the graphical app: native when it really can, AppImage when it
/// cannot.
///
/// "Really can" means all three at once: the manager exists, the root is writable,
/// and we can elevate without hanging. Any one failing falls back to the AppImage,
/// which needs none of the three. That is why SteamOS and Bazzite, with `rpm`
/// present but a read-only root, land where they have to without a special case
/// written for them.
#[cfg(target_os = "linux")]
pub fn resolve_delivery(probe: &Probe) -> Delivery {
    // Inside a Flatpak nothing here is ours to replace: `/app` is read-only and the
    // version that lands next comes from the remote the user installed from. This
    // has to be the first question, before the package managers, because the
    // runtime carries neither `dpkg` nor `rpm`, so falling through would pick the
    // AppImage and aim it at `/app/bin`.
    if probe.sandboxed {
        return Delivery::Managed;
    }
    if probe.immutable_root || !probe.can_elevate {
        return Delivery::AppImage;
    }
    if probe.has_dpkg {
        return Delivery::Deb;
    }
    if probe.has_rpm {
        return Delivery::Rpm;
    }
    Delivery::AppImage
}

#[cfg(target_os = "windows")]
pub fn resolve_delivery(_probe: &Probe) -> Delivery {
    Delivery::Nsis
}

#[cfg(target_os = "macos")]
pub fn resolve_delivery(_probe: &Probe) -> Delivery {
    Delivery::Dmg
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
pub fn resolve_delivery(_probe: &Probe) -> Delivery {
    Delivery::AppImage
}

// ---- the manifest

/// What is installed on this machine: which components, at which version and by
/// which route. It is what turns "install" and "update" into the same operation
/// seen from two different moments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    /// The version every component was left at by the last operation. If a binary
    /// on disk does not match this, the install was left half done and has to be
    /// redone.
    pub version: String,
    /// What is installed. Sorted and deduplicated.
    pub components: Vec<Component>,
    /// The graphical app's route. `None` when there is no `Desktop`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery: Option<Delivery>,
    /// Where `hoard` and `hoardd` live.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub core_dir: Option<PathBuf>,
    /// The app's executable, when we know it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desktop_path: Option<PathBuf>,
    /// Does the core travel inside the app's bundle? Then the app's installer
    /// relieves it and ours can only duplicate it.
    ///
    /// It is recorded rather than inferred from the paths matching, and the
    /// difference is the AppImage route: there the app lands in
    /// `~/.local/bin/hoard-desktop`, the same directory where the installer left
    /// the core, so "they are in the same folder" would give `true` and an update
    /// would stop touching the core, on the SteamOS path, which is exactly the one
    /// that cannot fail. Whoever put it there knows; let them say so.
    #[serde(default)]
    pub core_from_bundle: bool,
}

impl Manifest {
    /// `<config>/install.json`. Alongside the rest of the user's config, and per
    /// user: two accounts on the same machine can have different installs.
    pub fn path() -> Result<PathBuf> {
        Ok(crate::config::CliConfig::project_dirs()?
            .config_dir()
            .join("install.json"))
    }

    /// Reads the manifest. `Ok(None)` when there is none yet (an install predating
    /// this module, or a first time).
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
            // A plan is made by our installer, so the core is ours.
            core_from_bundle: false,
        }
    }

    /// This machine's manifest, created by observation when it does not exist.
    ///
    /// The case that forces this: somebody who installed the app before manifests
    /// existed (a `.deb` downloaded from the web) has no file, and assuming "there
    /// is no app here" would leave it out of the first unified update. So the first
    /// thing done is to look at the disk.
    pub fn load_or_observe() -> Result<Self> {
        if let Some(m) = Self::load()? {
            return Ok(m);
        }
        let m = observe();
        // Best-effort: with no write permission we carry on with what we observed.
        let _ = m.save();
        Ok(m)
    }

    /// Reconciles the manifest with what is on disk and saves it when it changed.
    /// The frontends call it on start: it is how an app installed on its own ends
    /// up recorded without the user doing anything.
    pub fn reconcile() -> Result<Self> {
        let observed = observe();
        let mut m = match Self::load()? {
            Some(m) => m,
            None => {
                observed.save()?;
                return Ok(observed);
            }
        };
        let mut changed = false;
        for c in &observed.components {
            if !m.has(*c) {
                m.add(*c);
                changed = true;
            }
        }
        if m.delivery.is_none() && observed.delivery.is_some() {
            m.delivery = observed.delivery;
            changed = true;
        }
        if m.desktop_path.is_none() && observed.desktop_path.is_some() {
            m.desktop_path.clone_from(&observed.desktop_path);
            changed = true;
        }
        if m.core_dir.is_none() && observed.core_dir.is_some() {
            m.core_dir.clone_from(&observed.core_dir);
            changed = true;
        }
        // It only goes up to `true`: if our installer already said the core is
        // its own, a later observation cannot take that back, since the app and the
        // core can end up in the same folder without one containing the other.
        if observed.core_from_bundle && !m.core_from_bundle {
            m.core_from_bundle = true;
            changed = true;
        }
        if changed {
            m.save()?;
        }
        Ok(m)
    }
}

// ---- the swap window: "don't start me right now"

/// How long a swap marker is believed before it's treated as debris.
///
/// It has to outlive the slowest installer we drive (an NSIS `-setup.exe /S`
/// unpacking ~90 MB onto a cold disk) and still be short enough that a machine
/// which crashed mid-swap isn't left unable to start its own service. Three
/// minutes is both.
const SWAP_WINDOW: std::time::Duration = std::time::Duration::from_secs(3 * 60);

/// **The binaries are being replaced right now, so don't launch one.**
///
/// The client rule since Slice 4 is "spawn if absent" (`Client::ensure_running`
/// in `hoardd`): lose the socket, start a service. That rule and an installer
/// are a deadlock on Windows. The NSIS hook stops `hoardd.exe` before it can
/// overwrite it, the desktop notices the socket is gone two seconds later and
/// starts it again from the *old* binary, and NSIS then hits a file that's back
/// in use ("Error opening file for writing"), the update aborted, and the same
/// thing an hour later. The kill order in `installer-hooks.nsh` narrows that
/// window; this closes it, and covers the clients the hook can't kill by name
/// (a `hoard` invocation from a terminal, a second desktop session).
///
/// It's a file and not a flag in memory because the process that must not spawn
/// a daemon is usually not the process that started the install.
///
/// Held for the length of [`stage::apply`] and dropped with the guard. Being
/// killed mid-swap is the normal Windows path, so a marker that outlives its
/// process is expected: it expires on its own after [`SWAP_WINDOW`], and any
/// `hoardd` that manages to start clears it: a live service *is* the proof the
/// swap is over.
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
            // is a client that waits three minutes for a service it could have
            // started, against a corrupted install.
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

/// Leaves `hoard` reachable from a terminal, and says what it did.
///
/// The app's bundle carries the binary, but carrying it is not enough: in a `.deb`
/// it lands in `/usr/bin` and is already on the `PATH`, while on Windows it ends up
/// under `%LOCALAPPDATA%` and on macOS inside `Hoard.app`, where nobody is ever
/// going to type it. Having the binary and not being able to invoke it is, in
/// practice, not having it, so the app fixes this on start.
///
/// Idempotent and best-effort: it is called on every start, asks for no privileges,
/// and fails nobody's startup if it cannot.
pub fn ensure_cli_reachable() -> Result<CliReach> {
    let exe = std::env::current_exe().context("resolving our own path")?;
    let dir = exe.parent().context("our own path has no parent")?;
    ensure_dir_reachable(dir)
}

/// The same, for a directory that isn't ours.
///
/// An installer needs it: it has just dropped `hoard` into a folder, and that
/// folder is not where the installer itself lives, so [`ensure_cli_reachable`],
/// which starts from `current_exe()`, would look in the wrong place and conclude
/// [`CliReach::NotBundled`].
pub fn ensure_dir_reachable(dir: &Path) -> Result<CliReach> {
    let cli = dir.join(format!("hoard{}", std::env::consts::EXE_SUFFIX));
    if !cli.is_file() {
        return Ok(CliReach::NotBundled);
    }
    if on_path("hoard") {
        return Ok(CliReach::AlreadyReachable);
    }
    platform_reach(dir, &cli)
}

/// Puts `dir` on the `PATH` of the user's **future** terminals, and says whether
/// it touched anything.
///
/// [`ensure_dir_reachable`] solves "can I invoke `hoard`" with a symlink from a
/// directory that is already on the `PATH`. That doesn't work when the real
/// binary already lives in `~/.local/bin` and it is that directory missing from
/// the `PATH`: the link would point at itself. This fixes the `PATH` instead,
/// which is what `install.sh` does at the end of a terminal install and what an
/// installer with a window has to do the same way, or whoever installs
/// through the window ends up with a CLI they can't type.
///
/// Best-effort and idempotent: an install doesn't fail over this, and calling it
/// twice doesn't leave the line in the file twice.
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

/// Windows: the user's `PATH` lives in the registry, and we already know how to
/// write it.
#[cfg(target_os = "windows")]
fn shell_path(dir: &Path) -> Result<PathNudge> {
    match platform_reach(dir, &dir.join("hoard.exe"))? {
        CliReach::AddedToPath(d) => Ok(PathNudge::Added(d)),
        CliReach::AlreadyReachable => Ok(PathNudge::AlreadyThere),
        other => Ok(PathNudge::Skipped(format!("{other:?}"))),
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

/// What happened when trying to leave the terminal within reach.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliReach {
    /// `hoard` was already typable. Nothing to do.
    AlreadyReachable,
    /// `dir` was added to the user's `PATH`. It needs a fresh terminal.
    AddedToPath(PathBuf),
    /// A link was created at `path`.
    Linked(PathBuf),
    /// This bundle does not carry the terminal (an old build, or an AppImage whose
    /// core the installer puts elsewhere).
    NotBundled,
    /// There is no way to fix it from here, with the reason.
    Unreachable(String),
}

fn on_path(name: &str) -> bool {
    let exe = format!("{name}{}", std::env::consts::EXE_SUFFIX);
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|d| d.join(&exe).is_file()))
        .unwrap_or(false)
}

/// Windows: the app's folder onto the USER's `PATH` (`HKCU\Environment`).
///
/// It is done from here rather than from the NSIS installer's hook on purpose: that
/// way it works for a fresh install, for an update that moves the folder, and for a
/// bundle that was already in place, and it is the same line of code fixing the case
/// in all three. `winreg` is already a dependency.
#[cfg(target_os = "windows")]
fn platform_reach(dir: &Path, _cli: &Path) -> Result<CliReach> {
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
        return Ok(CliReach::AlreadyReachable);
    }
    let next = if current.trim().is_empty() {
        dir_str.clone()
    } else {
        format!("{};{}", current.trim_end_matches(';'), dir_str)
    };
    env.set_value("Path", &next)
        .context("writing HKCU\\Environment\\Path")?;
    broadcast_environment_change();
    Ok(CliReach::AddedToPath(dir.to_path_buf()))
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

/// macOS: a symlink where the default `PATH` already looks.
///
/// `/usr/local/bin` is the convention and is on the `PATH` out of the box, but it
/// belongs to root; it is tried without elevating and, failing that, falls back to
/// `~/.local/bin`, which is always writable. Asking for privileges on opening the app
/// over this would be
/// desproporcionado.
#[cfg(target_os = "macos")]
fn platform_reach(_dir: &Path, cli: &Path) -> Result<CliReach> {
    let mut candidates = vec![PathBuf::from("/usr/local/bin")];
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        candidates.push(home.join(".local").join("bin"));
    }
    link_into(&candidates, cli)
}

/// Linux: on a `.deb` or `.rpm` the binary is already in `/usr/bin` and this never
/// gets called. Only the AppImage is left, where there is nothing to link: its
/// content lives on a mount that disappears when the app closes, and the link would
/// be broken the moment it does. There the terminal is put in place by the installer,
/// which is what can leave it on a stable path.
#[cfg(target_os = "linux")]
fn platform_reach(_dir: &Path, cli: &Path) -> Result<CliReach> {
    if cli
        .components()
        .any(|c| c.as_os_str().to_string_lossy().starts_with(".mount_"))
        || std::env::var_os("APPIMAGE").is_some()
    {
        return Ok(CliReach::Unreachable(
            "this AppImage's copy would vanish when the app closes; install the core with \
             `curl -fsSL https://hoard.services/install.sh | sh`"
                .into(),
        ));
    }
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return Ok(CliReach::Unreachable("no HOME in the environment".into()));
    };
    link_into(&[home.join(".local").join("bin")], cli)
}

/// Links `cli` from the first directory in `candidates` that will take it.
///
/// It NEVER replaces a file that is not a link of ours. That is the whole rule of
/// this function and the reason it exists: `~/.local/bin` is exactly where the
/// terminal installer leaves the real `hoard`, and an app starting without that
/// directory on its `PATH`, which is normal when launched from the desktop menu since
/// that does not read your shell profile, would conclude "it is not reachable" and
/// delete the installed binary to put a link to its bundle copy in its place. It
/// would trade a standalone, self-updating install for one tied to the app,
/// destroying the good one on the way.
///
/// When there is already a flesh-and-blood `hoard` there, the right answer is to
/// leave it alone: the binary is there and reachable from that path. Whether that
/// directory is on your shell's `PATH` is a matter for your profile, not something to
/// arregle borrando ejecutables ajenos.
#[cfg(unix)]
fn link_into(candidates: &[PathBuf], cli: &Path) -> Result<CliReach> {
    let mut last = String::from("no candidate directory");
    for dir in candidates {
        if std::fs::create_dir_all(dir).is_err() && !dir.is_dir() {
            last = format!("{} is not writable", dir.display());
            continue;
        }
        let link = dir.join("hoard");
        match std::fs::symlink_metadata(&link) {
            // A link already pointing correctly is left alone. One pointing
            // somewhere else is repointed: that is the case after an update
            // that moves the bundle, and repointing a link of ours destroys
            // nothing.
            Ok(meta) if meta.file_type().is_symlink() => {
                if std::fs::read_link(&link).is_ok_and(|t| t == cli) {
                    return Ok(CliReach::AlreadyReachable);
                }
                let _ = std::fs::remove_file(&link);
            }
            // Something that is NOT a link: a real installed `hoard`. It is
            // respected and taken as reachable.
            Ok(_) => return Ok(CliReach::AlreadyReachable),
            // Nothing there: clear road.
            Err(_) => {}
        }
        match std::os::unix::fs::symlink(cli, &link) {
            Ok(()) => return Ok(CliReach::Linked(link)),
            Err(e) => last = format!("{}: {e}", link.display()),
        }
    }
    Ok(CliReach::Unreachable(last))
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn platform_reach(_dir: &Path, _cli: &Path) -> Result<CliReach> {
    Ok(CliReach::Unreachable("unsupported platform".into()))
}

/// What this machine already has, asked by something that is **not** part of
/// the install.
///
/// [`observe`] can't answer this. It starts from `current_exe().parent()`,
/// which is right when the asker is `hoard` or `hoardd` (they live in the
/// directory it is looking for) and wrong for an installer, which sits in
/// whatever downloads folder it was saved to and would report that folder as
/// the place Hoard lives.
///
/// `core_hint` is where the caller *would* install the core. It is tried first
/// and then the `PATH`, so a machine that got its core from `install.sh` and a
/// machine that got it from a package both come back found.
pub fn detect(core_hint: &Path) -> Option<Installed> {
    let daemon = format!("hoardd{}", std::env::consts::EXE_SUFFIX);

    let core_dir = Some(core_hint.to_path_buf())
        .filter(|d| d.join(&daemon).is_file())
        .or_else(|| {
            std::env::var_os("PATH")
                .and_then(|paths| std::env::split_paths(&paths).find(|d| d.join(&daemon).is_file()))
        });
    // The manifest is the only thing that knows which version this is; without
    // one (an install older than the manifest, or a hand-placed binary) the
    // honest answer is "we don't know", not a guess.
    let manifest = Manifest::load().ok().flatten();

    // Path and delivery have to come from the same source or they contradict
    // each other, and removing the app is what pays for that: a path found by
    // scanning paired with a delivery read from the manifest can mean running
    // `dpkg -r` against an AppImage, which removes a package that isn't the
    // copy we found and leaves the copy we found in place. The manifest's own
    // pair wins whenever the path it names is really there.
    let recorded = manifest
        .as_ref()
        .and_then(|m| m.desktop_path.clone())
        .filter(|p| p.exists());
    let (desktop, delivery) = match recorded {
        Some(path) => {
            let delivery = manifest
                .as_ref()
                .and_then(|m| m.delivery)
                .unwrap_or_else(|| observed_delivery(&path));
            (Some(path), Some(delivery))
        }
        None => {
            let found = installed_desktop();
            let delivery = found.as_deref().map(observed_delivery);
            (found, delivery)
        }
    };

    // Neither half present means nothing to update and nothing to remove.
    if core_dir.is_none() && desktop.is_none() {
        return None;
    }

    Some(Installed {
        version: manifest.as_ref().map(|m| m.version.clone()),
        delivery,
        core_dir,
        desktop,
        manifest,
    })
}

/// The answer from [`detect`].
#[derive(Debug, Clone)]
pub struct Installed {
    /// What the manifest says is installed. `None` when there is no manifest.
    pub version: Option<String>,
    /// Where `hoard` and `hoardd` are, if they are anywhere.
    pub core_dir: Option<PathBuf>,
    /// The app's executable, if it is installed.
    pub desktop: Option<PathBuf>,
    /// How the app got here, needed to take it away again the same way.
    pub delivery: Option<Delivery>,
    /// The manifest itself, when there is one.
    pub manifest: Option<Manifest>,
}

impl Installed {
    /// Is the graphical app part of this?
    pub fn has_desktop(&self) -> bool {
        self.desktop.is_some()
    }
}

/// What is installed **according to the disk**, with no manifest in the middle.
fn observe() -> Manifest {
    let core_dir = observed_core_dir();
    let desktop_path = observed_desktop();
    let mut components = vec![Component::Core];
    if desktop_path.is_some() {
        components.push(Component::Desktop);
    }
    let delivery = desktop_path.as_deref().map(observed_delivery);
    // With no manifest saying so it has to be deduced, and the only case where the
    // core travels inside is a real bundle: the AppImage shares a folder with the
    // core without containing it, so it is excluded explicitly.
    let core_from_bundle = match (&core_dir, &desktop_path, delivery) {
        (Some(core), Some(desktop), Some(d)) => {
            d != Delivery::AppImage && desktop.parent() == Some(core.as_path())
        }
        _ => false,
    };
    Manifest {
        version: crate::update::current().to_string(),
        components,
        delivery,
        core_dir,
        desktop_path,
        core_from_bundle,
    }
}

/// Where the core lives: the directory of this very executable, which is `hoard`
/// or `hoardd` depending on who asks and is the right answer either way.
fn observed_core_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(Path::to_path_buf))
}

/// The app's executable, if it is there. Looked for where the delivery routes we
/// know leave it, and the `PATH` too for a hand-rolled install.
fn observed_desktop() -> Option<PathBuf> {
    let name = format!("hoard-desktop{}", std::env::consts::EXE_SUFFIX);

    // Next to us: the desktop bundle, or a `cargo build` of the workspace.
    if let Some(dir) = observed_core_dir() {
        let sibling = dir.join(&name);
        if sibling.is_file() {
            return Some(sibling);
        }
    }
    installed_desktop()
}

/// The app **where the install routes leave it**, without looking next to whoever
/// is asking.
///
/// The exclusion matters: [`observed_desktop`] starts from its own directory
/// because the asker is usually `hoard` or `hoardd`, which travel with the app. An
/// installer does not: it lives in the downloads folder, or in `target/debug`
/// during development, and there "a hoard-desktop sits next to me" means "somebody
/// just built the workspace", not "this machine has Hoard installed".
pub(crate) fn installed_desktop() -> Option<PathBuf> {
    let name = format!("hoard-desktop{}", std::env::consts::EXE_SUFFIX);
    for dir in known_desktop_dirs() {
        let candidate = dir.join(&name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|d| d.join(&name))
            .find(|p| p.is_file())
    })
}

/// Directories where the app lands per delivery route, by platform.
fn known_desktop_dirs() -> Vec<PathBuf> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from);

    #[cfg(target_os = "linux")]
    {
        let mut dirs = vec![PathBuf::from("/usr/bin"), PathBuf::from("/usr/local/bin")];
        if let Some(h) = home {
            dirs.push(h.join(".local").join("bin"));
        }
        dirs
    }
    #[cfg(target_os = "macos")]
    {
        let _ = &home;
        vec![PathBuf::from("/Applications/Hoard.app/Contents/MacOS")]
    }
    #[cfg(target_os = "windows")]
    {
        let mut dirs = Vec::new();
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            dirs.push(PathBuf::from(local).join("Hoard"));
        }
        if let Some(pf) = std::env::var_os("ProgramFiles") {
            dirs.push(PathBuf::from(pf).join("Hoard"));
        }
        let _ = &home;
        dirs
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        let _ = &home;
        Vec::new()
    }
}

/// Where the app at `path` came from. Deduced from where it lives: that is the
/// only clue that survives the installer having put it there months ago.
fn observed_delivery(path: &Path) -> Delivery {
    observed_delivery_in(path, running_under_flatpak())
}

/// [`observed_delivery`] with the sandbox answered for it, so the one case that
/// can't be expressed as a path is still a test and not a comment.
fn observed_delivery_in(path: &Path, sandboxed: bool) -> Delivery {
    // A Flatpak install writes no manifest (it never runs our installer), so
    // this is the function that names it, and naming it wrong is expensive:
    // `/app/bin/hoard-desktop` is under neither `$HOME` nor `/usr`, so it used
    // to fall through to the `AppImage` at the bottom and hand the updater a
    // read-only directory to overwrite, once an hour, for ever.
    if sandboxed {
        return Delivery::Managed;
    }
    let s = path.to_string_lossy();
    if cfg!(target_os = "windows") {
        return Delivery::Nsis;
    }
    if cfg!(target_os = "macos") {
        return Delivery::Dmg;
    }
    // Under the home there is no package manager in the middle.
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        if path.starts_with(&home) {
            return Delivery::AppImage;
        }
    }
    // In `/usr` a package put it. Which one, the machine says.
    if s.starts_with("/usr/") {
        if bin_exists("dpkg") {
            return Delivery::Deb;
        }
        if bin_exists("rpm") {
            return Delivery::Rpm;
        }
        // Neither dpkg nor rpm and still under /usr: something else put it
        // there (Arch, Nix, a hand-rolled tarball). Not ours, not touched.
        return Delivery::Managed;
    }
    Delivery::AppImage
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An "ordinary desktop" probe to vary a single fact per test, so each
    /// assertion says which fact is the one in charge.
    fn desktop_box() -> Probe {
        Probe {
            graphical: true,
            sandboxed: false,
            immutable_root: false,
            has_dpkg: true,
            has_rpm: false,
            can_elevate: true,
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
            for immutable_root in [true, false] {
                let probe = Probe {
                    graphical,
                    immutable_root,
                    ..desktop_box()
                };
                assert!(resolve_components(&probe).contains(&Component::Core));
            }
        }
    }

    #[cfg(target_os = "linux")]
    mod linux_delivery {
        use super::*;

        #[test]
        fn a_writable_debian_box_gets_the_native_package() {
            assert_eq!(resolve_delivery(&desktop_box()), Delivery::Deb);
        }

        #[test]
        fn a_writable_fedora_box_gets_the_rpm() {
            let probe = Probe {
                has_dpkg: false,
                has_rpm: true,
                ..desktop_box()
            };
            assert_eq!(resolve_delivery(&probe), Delivery::Rpm);
        }

        /// SteamOS and Bazzite have `rpm` on the PATH and the native package
        /// still cannot be applied. Without the immutable-root check the plan
        /// would pick an `.rpm` that writes nothing, which is exactly the case
        /// that opened this whole redesign.
        #[test]
        fn an_immutable_image_falls_back_to_the_appimage() {
            let probe = Probe {
                immutable_root: true,
                has_rpm: true,
                has_dpkg: false,
                ..desktop_box()
            };
            assert_eq!(resolve_delivery(&probe), Delivery::AppImage);
        }

        /// Inside `curl … | sh` there is nobody to ask for the password, so
        /// "we cannot elevate" has to lead to the AppImage and not to a `.deb`
        /// that would hang waiting.
        #[test]
        fn no_way_to_elevate_falls_back_to_the_appimage() {
            let probe = Probe {
                can_elevate: false,
                ..desktop_box()
            };
            assert_eq!(resolve_delivery(&probe), Delivery::AppImage);
        }

        #[test]
        fn a_box_with_no_package_manager_falls_back_to_the_appimage() {
            let probe = Probe {
                has_dpkg: false,
                has_rpm: false,
                ..desktop_box()
            };
            assert_eq!(resolve_delivery(&probe), Delivery::AppImage);
        }

        /// The sandbox outranks the package manager, and the probe it runs on
        /// is deliberately the friendliest one there is: a writable Debian box
        /// that can elevate. Order matters here and nothing else would catch
        /// it: read the checks in the other order and this same probe comes
        /// back `Deb`.
        #[test]
        fn a_flatpak_is_not_ours_to_update() {
            let probe = Probe {
                sandboxed: true,
                ..desktop_box()
            };
            assert_eq!(resolve_delivery(&probe), Delivery::Managed);
            assert!(!resolve_delivery(&probe).is_ours());
        }

        /// `/app/bin` is under neither `$HOME` nor `/usr`, so the path alone
        /// says AppImage, which is how the updater ended up aiming at a
        /// read-only directory. The second half of this test is the bug, kept
        /// so that removing the flag can't look harmless.
        #[test]
        fn the_sandbox_names_the_delivery_the_path_cannot() {
            let app = Path::new("/app/bin/hoard-desktop");
            assert_eq!(observed_delivery_in(app, true), Delivery::Managed);
            assert_eq!(observed_delivery_in(app, false), Delivery::AppImage);
        }
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

    /// What a third party maintains is not updated: replacing a distro package
    /// manager's binary underneath it leaves the system lying about what it has
    /// installed.
    #[test]
    fn a_managed_install_is_not_ours_to_update() {
        assert!(!Delivery::Managed.is_ours());
        assert!(Delivery::Deb.is_ours());
        assert!(Delivery::AppImage.is_ours());
    }

    #[test]
    fn only_native_packages_need_elevation() {
        assert!(Delivery::Deb.needs_elevation());
        assert!(Delivery::Rpm.needs_elevation());
        assert!(!Delivery::AppImage.needs_elevation());
        assert!(!Delivery::Nsis.needs_elevation());
    }

    /// The manifest is a contract with the shell installer, which writes and
    /// reads it without serde. If the shape changes, that side finds out here.
    #[test]
    fn the_manifest_round_trips_through_its_wire_shape() {
        let m = Manifest {
            version: "1.2.0".into(),
            components: vec![Component::Core, Component::Desktop],
            delivery: Some(Delivery::AppImage),
            core_dir: Some(PathBuf::from("/home/ada/.local/bin")),
            desktop_path: Some(PathBuf::from("/home/ada/.local/bin/hoard-desktop")),
            core_from_bundle: false,
        };
        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains(r#""components":["core","desktop"]"#));
        assert!(json.contains(r#""delivery":"appimage""#));
        assert_eq!(serde_json::from_str::<Manifest>(&json).unwrap(), m);
    }

    /// The shell installers compare literal strings against this field, so the
    /// wire shape and the one we print have to be the same. Without this test a
    /// `rename_all` turning `AppImage` into `app_image` goes unnoticed until an
    /// `upgrade` fails to recognise its own route.
    #[test]
    fn every_delivery_serialises_as_the_name_it_prints() {
        for d in [
            Delivery::Deb,
            Delivery::Rpm,
            Delivery::AppImage,
            Delivery::Nsis,
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

    /// **The rule that protects the user's install.** `~/.local/bin` is exactly
    /// where the terminal installer leaves the real `hoard`, and an app launched
    /// from the desktop menu does not read your shell profile, so it can start
    /// without that directory on its `PATH` and conclude "not reachable". If it
    /// deleted what is there at that point, it would trade a standalone install
    /// for a link tied to the bundle, destroying the good one on the way.
    #[cfg(unix)]
    #[test]
    fn a_real_binary_in_the_target_dir_is_never_replaced() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("bin");
        std::fs::create_dir_all(&dir).unwrap();
        let installed = dir.join("hoard");
        std::fs::write(&installed, b"the real installed hoard").unwrap();

        let bundled = tmp.path().join("bundle").join("hoard");
        std::fs::create_dir_all(bundled.parent().unwrap()).unwrap();
        std::fs::write(&bundled, b"the bundle copy").unwrap();

        assert_eq!(
            link_into(std::slice::from_ref(&dir), &bundled).unwrap(),
            CliReach::AlreadyReachable
        );
        assert_eq!(
            std::fs::read(&installed).unwrap(),
            b"the real installed hoard",
            "the installed binary got clobbered"
        );
        assert!(
            !std::fs::symlink_metadata(&installed)
                .unwrap()
                .file_type()
                .is_symlink(),
            "the real file has been turned into a link"
        );
    }

    /// A link of ours does get repointed: that is what happens after an update
    /// that moves the bundle, and repointing a link destroys nothing.
    #[cfg(unix)]
    #[test]
    fn a_stale_symlink_of_ours_gets_repointed() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("bin");
        std::fs::create_dir_all(&dir).unwrap();
        let old = tmp.path().join("v1").join("hoard");
        let new = tmp.path().join("v2").join("hoard");
        for p in [&old, &new] {
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, b"x").unwrap();
        }
        let link = dir.join("hoard");
        std::os::unix::fs::symlink(&old, &link).unwrap();

        assert_eq!(
            link_into(std::slice::from_ref(&dir), &new).unwrap(),
            CliReach::Linked(link.clone())
        );
        assert_eq!(std::fs::read_link(&link).unwrap(), new);
    }

    /// A link already pointing where it should is left alone: this runs on every
    /// app start, and rewriting it for the sake of it is a disk write for
    /// nothing.
    #[cfg(unix)]
    #[test]
    fn a_correct_symlink_is_left_alone() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("bin");
        std::fs::create_dir_all(&dir).unwrap();
        let target = tmp.path().join("bundle").join("hoard");
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(&target, b"x").unwrap();
        let link = dir.join("hoard");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        assert_eq!(
            link_into(&[dir], &target).unwrap(),
            CliReach::AlreadyReachable
        );
    }

    /// Empty directory: clear road, it gets linked.
    #[cfg(unix)]
    #[test]
    fn an_empty_dir_gets_the_link() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("bin");
        let target = tmp.path().join("bundle").join("hoard");
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(&target, b"x").unwrap();

        assert_eq!(
            link_into(std::slice::from_ref(&dir), &target).unwrap(),
            CliReach::Linked(dir.join("hoard"))
        );
    }

    /// **The AppImage shares a folder with the core without containing it.** That
    /// is the trap that makes deducing core ownership by comparing paths useless:
    /// `place_appimage` leaves the app at `~/.local/bin/hoard-desktop`, which is
    /// where the installer left `hoard` and `hoardd`, so "same folder" would give
    /// "the bundle brings it" and `hoard upgrade` would stop updating the core,
    /// on the SteamOS route, which is the one that motivated all of this.
    #[test]
    fn an_appimage_sharing_the_core_dir_does_not_own_the_core() {
        let m = Manifest {
            version: "1.2.0".into(),
            components: vec![Component::Core, Component::Desktop],
            delivery: Some(Delivery::AppImage),
            core_dir: Some(PathBuf::from("/home/ada/.local/bin")),
            desktop_path: Some(PathBuf::from("/home/ada/.local/bin/hoard-desktop")),
            core_from_bundle: false,
        };
        assert!(
            !m.core_from_bundle,
            "an AppImage cannot claim the core just by living next to it"
        );
    }

    /// An old manifest (from before the field) parses without blowing up and
    /// gives the prudent value: the core is ours and therefore updatable.
    #[test]
    fn a_manifest_from_before_the_field_defaults_to_ours() {
        let m: Manifest = serde_json::from_str(
            r#"{"version":"1.1.1","components":["core","desktop"],"delivery":"deb"}"#,
        )
        .unwrap();
        assert!(!m.core_from_bundle);
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
