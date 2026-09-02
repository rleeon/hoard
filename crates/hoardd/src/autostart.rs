//! Starting at boot as a **user service** (ADR 0021, Part A).
//!
//! The service starts two ways. One is "spawn if absent": a client that finds no
//! service brings it up ([`crate::client::Client::ensure_running`]). The other is
//! this one, where the OS's service manager starts it at login and the sync then
//! runs **without anybody opening anything**, which is the whole point. One backend
//! per platform, the same commands on all three:
//!
//! - **Linux**: a `systemd --user` unit (`hoard-sync.service`). It also tries
//!   `loginctl enable-linger`, so a headless machine (a NAS, SteamOS, a home
//!   server) keeps syncing with no graphical session open. From an
//!   AppImage the `ExecStart` is never the binary inside the mount: it is the
//!   installed `hoardd`, or a stable copy of it.
//! - **macOS**: LaunchAgent de `launchd` (`com.hoard.sync`).
//! - **Windows**: a Task Scheduler logon task (`HoardSync`) and, when the Task
//!   Scheduler refuses it without an elevated console, the user's own `Run`
//!   entry. Neither elevates; the second exists because the first says no on
//!   machines whose user token arrives filtered.
//!
//! ## When there is no way in
//!
//! "No backend here" is a legitimate answer (a machine without systemd, an
//! AppImage with a read-only `$HOME`) and it has to **reach the window**:
//! [`Unsupported`] classifies it and [`unsupported_reason`] recovers it by
//! downcast. A `tracing::warn!` does not do the job: the switch stays on, the
//! sync doesn't start at the next login, and the user has nothing to look at.
//!
//! **Per user, never system-wide**, and that is not an aesthetic preference: the
//! Cloud token lives in *your* session's secret store (Secret Service, Keychain,
//! DPAPI), which a root service cannot read. A machine service would not know whose
//! saves these are either.
//!
//! ## What the unit runs
//!
//! The `ExecStart` is **the `hoardd` binary**. It used to be `hoard sync run`, back
//! when the engine lived inside that process; that command is a client now, so
//! leaving it as the `ExecStart` meant the service manager supervised a spectator
//! rather than the service. `systemctl --user stop` now sends the signal **to the
//! daemon**, which says goodbye to its clients and stops the engine (see
//! [`hoard_core::ipc::ServerFrame::Goodbye`]).
//!
//! ## The handover: a client may have started it before the unit did
//!
//! There is only one daemon per user and the arbiter is the socket bind, so when one
//! is already running (the app brought it up when it opened) the one systemd
//! launches **loses the bind and exits with 0**, leaving the unit marked dead while
//! the sync works fine. That is why [`install`] and [`restart`] stop whatever is
//! there first and wait for it to release the socket: it is how the process's owner
//! really becomes the service manager.

#[cfg(any(target_os = "linux", target_os = "windows"))]
use std::path::Path;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};

use crate::client::Client;
use crate::endpoint::Endpoint;

/// How long the previous daemon gets to release the socket before the unit's is
/// started. Its clean shutdown includes the last presence beat (which goes over the
/// network), so it is not instant.
const HANDOVER_TIMEOUT: Duration = Duration::from_secs(20);

/// How long the freshly started service gets to listen.
const START_TIMEOUT: Duration = Duration::from_secs(20);

/// This OS's unit name, label or task. A frontend that wants to ask the service
/// manager directly needs it (`systemctl status`, `launchctl print`, `schtasks
/// /Query`); showing that output verbatim is the terminal's business, not this
/// layer's.
pub const UNIT_ID: &str = platform::UNIT;

/// Where it ended up installed, so the frontend can say so.
#[derive(Debug, Clone)]
pub struct Installed {
    /// Gestor de servicios usado (`"systemd --user"`, `"launchd"`, …).
    pub manager: &'static str,
    /// Nombre de la unidad / label / tarea.
    pub id: &'static str,
    /// Fichero escrito, si el backend usa uno.
    pub path: Option<PathBuf>,
}

/// Why this machine can't start the service at login.
///
/// Typed because this is what the window has to **say**, and saying it right
/// depends on which of the two cases it is: one is fixed by installing the core,
/// the other can't be fixed from the app at all. Both used to end in a
/// `tracing::warn!` inside the service, nowhere a user looks: the switch stayed
/// on, the sync didn't start at the next login, and there was nothing to report
/// beyond "it doesn't work".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unsupported {
    /// The app's format leaves no path that survives closing it, and a stable
    /// copy of the daemon couldn't be written either (an AppImage with a
    /// read-only `$HOME`, or none at all).
    NoStablePath,
    /// There is no user service manager to declare anything to: a machine
    /// without systemd, or an OS with no backend here.
    NoServiceManager,
}

impl Unsupported {
    /// Stable tag for the wire and the UI. Not the message: the sentence the
    /// user reads comes out of i18n keyed on this.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoStablePath => "no_stable_path",
            Self::NoServiceManager => "no_service_manager",
        }
    }
}

/// Login start can't be declared here, with the typed reason and the detail for
/// the log.
#[derive(Debug)]
pub struct LoginStartUnsupported {
    pub kind: Unsupported,
    pub detail: String,
}

impl std::fmt::Display for LoginStartUnsupported {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.detail)
    }
}

impl std::error::Error for LoginStartUnsupported {}

/// The typed reason inside `err`, if it carries one. By downcast and never by
/// the message text, the same way [`crate::engine`] classifies why there's no
/// engine: a message gets reworded without a thought and the classification
/// breaks silently.
pub fn unsupported_reason(err: &anyhow::Error) -> Option<Unsupported> {
    err.downcast_ref::<LoginStartUnsupported>().map(|u| u.kind)
}

/// The binary the unit runs: **this** installation's `hoardd`, the one travelling
/// beside whoever declares the unit (the desktop bundle packages it as
/// `externalBin`, the tarball puts it next to `hoard`), and otherwise the `PATH`'s.
///
/// Deliberately blind to whatever unit was already there: see
/// [`crate::client::own_daemon_binary`]. Whoever asks "which is this machine's
/// daemon?" uses [`crate::client::daemon_binary`], which starts from exactly what
/// gets written here.
pub fn service_binary() -> PathBuf {
    crate::client::own_daemon_binary()
}

/// The `hoardd` the installed service runs, read from the service manager's own
/// definition. `None` when no service is installed or it could not be read.
///
/// It is the arbiter of "which binary is this machine's daemon" when there is more
/// than one candidate on disk, which is normal now that the app and the terminal
/// installer each install the whole of Hoard: there is one daemon per user, and the
/// one that counts is the one the system already starts.
pub fn installed_exec_start() -> Option<PathBuf> {
    platform::exec_start()
}

/// Instala la unidad y **arranca el servicio ahora**. Idempotente.
pub async fn install() -> Result<Installed> {
    let installed = ensure_installed().await?;
    // The process's owner becomes the service manager: stop whatever daemon a client
    // brought up and wait for it to release the socket, or the one the unit launches
    // will lose the bind and exit (a dead unit with a live sync, the worst of both
    // worlds to diagnose).
    hand_over().await;
    start_now().await?;
    Ok(installed)
}

/// Writes or updates the unit and leaves it enabled for the next login,
/// **without touching** a service that is already running. Idempotent: the desktop
/// calls it on every start (just as it reaffirms its own autostart), where stopping
/// the sync to reinstall it would be absurd.
pub async fn ensure_installed() -> Result<Installed> {
    // Declaring the unit without checking the engine is there is the most expensive
    // way to fail. `own_daemon_binary` falls back to a bare name when it cannot find
    // the sibling, so the `ExecStart` comes out as plain `"hoardd"`, systemd accepts
    // it, enables it, starts it and it dies with `203/EXEC`, and all the user sees is
    // "Unable to locate executable 'hoardd'" in the journal, with not one hint that
    // what is missing is half an install. It really happened: the CLI tarballs from
    // 1.1.0 on carried no `hoardd`, so everybody who installed through `curl | sh`
    // ended up here.
    ensure_daemon_present()?;
    let (mut installed, changed) = platform::declare()?;
    // With the definition unchanged and the service already installed there is
    // nothing to do: the desktop calls this on every start, and two subprocesses per
    // start to reaffirm what is already there is a toll with nothing in return.
    if changed || !platform::installed().await {
        // The backend can end up using a different mechanism than the one it
        // declared (Windows falls from the Task Scheduler to the Run entry when
        // the task wants an elevated console), and whoever shows it has to show
        // the real one.
        if let Some(manager) = platform::enable().await? {
            installed.manager = manager;
        }
    }
    Ok(installed)
}

/// The binary going into the `ExecStart` has to exist *before* the unit is written.
/// It accepts an absolute path that is a file, or a bare name the `PATH` resolves,
/// which is the same test the service manager will apply when it starts it. Anything
/// else is half an install, and it is said in those words rather than left for
/// systemd to say in a journal nobody is watching.
fn ensure_daemon_present() -> Result<()> {
    let exe = crate::client::own_daemon_binary();
    if exe.is_file() {
        return Ok(());
    }
    // A bare name (`own_daemon_binary`'s fallback): let the `PATH` decide, just as
    // the service manager will.
    if exe.parent().is_none_or(|p| p.as_os_str().is_empty())
        && std::env::var_os("PATH")
            .is_some_and(|paths| std::env::split_paths(&paths).any(|d| d.join(&exe).is_file()))
    {
        return Ok(());
    }
    anyhow::bail!(
        "the sync engine ({}) isn't installed next to this binary or on your PATH, so the \
         service would be declared pointing at something that doesn't exist.\n\
         `hoard` is a thin client of `hoardd` and the two ship together, so reinstall the core \
         (https://hoard.services/install.sh) or drop `hoardd` beside `hoard`.",
        exe.display()
    )
}

/// Removes the login start and stops the manager's service. It does **not** send
/// `Shutdown` over IPC: that is a separate order (`hoard sync stop` does both, in
/// case a client and not the unit brought the service up). Returns `false` when
/// nothing was installed.
pub async fn uninstall() -> Result<bool> {
    if !installed().await {
        return Ok(false);
    }
    platform::disable().await?;
    Ok(true)
}

/// Reinicia el servicio bajo el gestor (tras un `hoard upgrade`, para que releve
/// el binario nuevo). Si no estaba instalado, lo instala.
pub async fn restart() -> Result<Installed> {
    if !installed().await {
        return install().await;
    }
    let (installed, _) = platform::declare()?;
    hand_over().await;
    platform::restart().await?;
    wait_until_serving().await?;
    Ok(installed)
}

/// Is there a unit installed for this user?
pub async fn installed() -> bool {
    platform::installed().await
}

/// Stops whatever daemon is running and waits for it to release the socket, so the
/// next start (the unit's) wins the bind.
///
/// Best-effort throughout: with no service there is nothing to hand over, and if it
/// does not step aside in time we carry on anyway, since the unit's start will find
/// the socket taken and exit with 0, which is ugly but breaks nothing.
async fn hand_over() {
    // Inside a Flatpak the process that would take over is the one running this
    // code: the portal starts us back through `flatpak run`, it doesn't own a
    // second copy waiting to bind. Shutting down here would hand the socket to
    // nobody, and the `wait_until_serving` that follows would then be waiting on
    // the service it had just killed.
    #[cfg(target_os = "linux")]
    if hoard_agent::install::running_under_flatpak() {
        return;
    }
    let Ok(endpoint) = Endpoint::resolve() else {
        return;
    };
    let Ok(mut client) = Client::connect(&endpoint, "hoard autostart (handover)").await else {
        return;
    };
    tracing::info!("autostart: stopping the running service so the unit can own it");
    if let Err(err) = client.request(hoard_core::ipc::Request::Shutdown).await {
        tracing::warn!(error = %format!("{err:#}"), "autostart: the service didn't acknowledge the stop");
    }
    drop(client);
    let deadline = Instant::now() + HANDOVER_TIMEOUT;
    while Instant::now() < deadline {
        // The **socket** is probed, not the handshake: what has to come free is the
        // bind. A daemon that has already said goodbye still holds it while it stops
        // the engine, and treating it as gone there is exactly what would leave the
        // next start losing the race.
        if crate::transport::connect(&endpoint).await.is_err() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    tracing::warn!("autostart: the previous service is still holding the socket");
}

/// Starts the service through the manager and confirms it **listens**. The unit
/// starting is not enough: if it lost the bind it exited with 0 and there is no new
/// service.
async fn start_now() -> Result<()> {
    platform::start().await?;
    wait_until_serving().await
}

async fn wait_until_serving() -> Result<()> {
    let endpoint = Endpoint::resolve().context("resolving the hoardd endpoint")?;
    let deadline = Instant::now() + START_TIMEOUT;
    loop {
        if let Ok(client) = Client::connect(&endpoint, "hoard autostart (probe)").await {
            tracing::info!(
                pid = client.welcome().pid,
                "autostart: the service is serving"
            );
            return Ok(());
        }
        if Instant::now() >= deadline {
            anyhow::bail!(
                "the Hoard service was installed but never started listening on {endpoint}, \
                 see `hoard sync logs`"
            );
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

// ---- helpers de proceso compartidos -----------------------------------

/// Runs a command, swallowing its output; returns whether it succeeded.
async fn run_quiet(program: &str, args: &[&str]) -> Result<bool> {
    let out = tokio::process::Command::new(program)
        .args(args)
        .output()
        .await
        .with_context(|| format!("running `{program}`"))?;
    Ok(out.status.success())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn home() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .context("no HOME/USERPROFILE in the environment")
}

// ---- Linux: systemd --user

#[cfg(target_os = "linux")]
mod platform {
    use super::*;

    pub const UNIT: &str = "hoard-sync.service";

    /// True when `name` is on the `PATH` (checked before invoking it).
    fn bin_exists(name: &str) -> bool {
        std::env::var_os("PATH")
            .map(|paths| std::env::split_paths(&paths).any(|d| d.join(name).is_file()))
            .unwrap_or(false)
    }

    pub fn unit_path() -> Result<PathBuf> {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .map(Ok)
            .unwrap_or_else(|| home().map(|h| h.join(".config")))?;
        Ok(base.join("systemd").join("user").join(UNIT))
    }

    fn ensure_systemd() -> Result<()> {
        if bin_exists("systemctl") {
            return Ok(());
        }
        Err(anyhow::Error::new(LoginStartUnsupported {
            kind: Unsupported::NoServiceManager,
            detail: "systemd not found. On a non-systemd init, run the service under your own \
                     supervisor (e.g. an OpenRC/runit service, or `nohup hoardd &`)."
                .to_string(),
        }))
    }

    /// Where we keep binaries that have to outlive the app closing.
    ///
    /// A **Hoard-owned** directory, not `~/.local/bin`, for the same reason
    /// `install::link_into` refuses to overwrite a file that isn't ours:
    /// `~/.local/bin` is exactly where the core installer puts the real
    /// `hoardd`, and dropping an AppImage's copy there would trade an
    /// installation that updates itself for one chained to the image.
    fn stable_bin_dir() -> Result<PathBuf> {
        let base = std::env::var_os("XDG_DATA_HOME")
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .map(Ok)
            .unwrap_or_else(|| home().map(|h| h.join(".local").join("share")))?;
        Ok(base.join("hoard").join("bin"))
    }

    /// A `hoardd` installed **outside** the mount, if there is one.
    ///
    /// First choice and the best one: the core installer put it there, it
    /// updates on its own and we don't have to keep it in sync. Only when it
    /// isn't there do we copy the one from the mount.
    fn daemon_outside_the_mount() -> Option<PathBuf> {
        let paths = std::env::var_os("PATH")?;
        std::env::split_paths(&paths)
            .map(|d| d.join("hoardd"))
            .find(|p| p.is_file() && !is_inside_appimage(p))
    }

    /// Leave a stable copy of the `hoardd` that travels inside the AppImage and
    /// return its path.
    ///
    /// The copy is redone when the version changes, not on every start:
    /// comparing two binaries of tens of MB every time the app opens is a toll
    /// for nothing, and a new image always carries a new version. The stamp is a
    /// separate file because a binary can't be asked its version without running
    /// it.
    ///
    /// **Written by `rename`, never copied over.** The destination may be the
    /// executable the service is running right now, and writing over a running
    /// binary is `ETXTBSY` on Linux; renaming over it leaves the live process
    /// with its inode and the next start with the new one. Same move
    /// `hoard-server upgrade` makes on its own binary.
    pub fn stage_stable_daemon(bundled: &Path) -> Result<PathBuf> {
        let dir = stable_bin_dir()?;
        std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        let dest = dir.join("hoardd");
        let stamp = dir.join("hoardd.version");
        let version = env!("CARGO_PKG_VERSION");
        let staged = std::fs::read_to_string(&stamp).ok();
        if dest.is_file() && staged.as_deref().map(str::trim) == Some(version) {
            return Ok(dest);
        }
        let tmp = dir.join("hoardd.staging");
        std::fs::copy(bundled, &tmp)
            .with_context(|| format!("copying {} to {}", bundled.display(), tmp.display()))?;
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))
                .with_context(|| format!("making {} executable", tmp.display()))?;
        }
        std::fs::rename(&tmp, &dest)
            .with_context(|| format!("moving {} into place", dest.display()))?;
        // The stamp goes in **after** the rename: crash in between and the next
        // round re-copies, which is harmless. The other order would leave a new
        // stamp over an old binary, and that never corrects itself.
        let _ = std::fs::write(&stamp, version);
        tracing::info!(
            path = %dest.display(),
            version,
            "autostart: staged a stable copy of the AppImage's sync engine"
        );
        Ok(dest)
    }

    /// The path that goes into the `ExecStart` when an AppImage is the one
    /// declaring the unit.
    ///
    /// Inside an AppImage the binary lives in an ephemeral mount point
    /// (`/tmp/.mount_XXXX/...`) that disappears when the app closes: a unit
    /// pointing there would start at the next login against a path that no
    /// longer exists. This **used to be a dead end** (it bailed out and the user
    /// was left with no login start and the switch still on) and it never had to
    /// be one: the engine is a component in its own right, so either one
    /// is installed outside the mount, or a stable copy of the one inside is
    /// left on disk.
    fn stable_exec_start(bundled: &Path) -> Result<PathBuf> {
        if let Some(installed) = daemon_outside_the_mount() {
            return Ok(installed);
        }
        stage_stable_daemon(bundled).map_err(|err| {
            anyhow::Error::new(LoginStartUnsupported {
                kind: Unsupported::NoStablePath,
                detail: format!(
                    "this AppImage has no stable path for the service ({} lives in a temporary \
                     mount) and a stable copy couldn't be written either: {err:#}. The sync \
                     still runs whenever Hoard is open. To start it at login, install the core \
                     with `curl -fsSL https://hoard.services/install.sh | sh`.",
                    bundled.display()
                ),
            })
        })
    }

    /// El texto de la unidad. Puro y testeable: es el contrato con systemd.
    pub fn unit_text(exe: &str) -> String {
        format!(
            "[Unit]\n\
             Description=Hoard game-save sync service\n\
             After=network-online.target\n\
             Wants=network-online.target\n\
             \n\
             [Service]\n\
             Type=simple\n\
             ExecStart=\"{exe}\"\n\
             Restart=on-failure\n\
             RestartSec=5\n\
             \n\
             [Install]\n\
             WantedBy=default.target\n",
        )
    }

    /// The installed unit's `ExecStart`. We wrote the unit ourselves
    /// ([`unit_text`]), so reading the line and stripping the quotes we put on it is
    /// enough.
    pub fn exec_start() -> Option<PathBuf> {
        // The portal's entry launches `flatpak run`, not a `hoardd` we could
        // point at. There is also only ever one candidate inside a sandbox, so
        // the question this arbitrates doesn't arise.
        if flatpak::active() {
            return None;
        }
        let text = std::fs::read_to_string(unit_path().ok()?).ok()?;
        let raw = text
            .lines()
            .find_map(|l| l.trim().strip_prefix("ExecStart="))?
            .trim();
        let unquoted = raw.strip_prefix('"').and_then(|r| r.strip_suffix('"'));
        Some(PathBuf::from(unquoted.unwrap_or(raw)))
    }

    /// Writes the unit when it is needed. The `bool` says whether it changed, so
    /// reaffirming it on every desktop start does not cost two subprocesses.
    pub fn declare() -> Result<(Installed, bool)> {
        if flatpak::active() {
            return flatpak::declare();
        }
        ensure_systemd()?;
        let mut exe = service_binary();
        // An AppImage runs from an ephemeral mount, so whatever `service_binary`
        // resolves is no good for a unit that starts tomorrow: it has to go
        // through the installed `hoardd`, or a stable copy of the one inside.
        if std::env::var_os("APPIMAGE").is_some() && is_inside_appimage(&exe) {
            exe = stable_exec_start(&exe)?;
        }
        let path = unit_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        // Entrecomillado para que una ruta con espacios sobreviva al tokenizador
        // de systemd (un AppImage en `~/Mis programas/`, por ejemplo).
        let unit = unit_text(&exe.to_string_lossy());
        let changed = std::fs::read_to_string(&path).ok().as_deref() != Some(unit.as_str());
        if changed {
            std::fs::write(&path, unit).with_context(|| format!("writing {}", path.display()))?;
        }
        Ok((
            Installed {
                manager: "systemd --user",
                id: UNIT,
                path: Some(path),
            },
            changed,
        ))
    }

    pub async fn enable() -> Result<Option<&'static str>> {
        if flatpak::active() {
            return flatpak::enable().await;
        }
        ensure_systemd()?;
        run_quiet("systemctl", &["--user", "daemon-reload"]).await?;
        if !run_quiet("systemctl", &["--user", "enable", UNIT]).await? {
            anyhow::bail!("`systemctl --user enable {UNIT}` failed");
        }
        // So it keeps syncing with no active session (a NAS, SteamOS, a server).
        // Best-effort: it may ask for a polkit there is nobody to show.
        let _ = run_quiet("loginctl", &["enable-linger"]).await;
        Ok(None)
    }

    pub async fn start() -> Result<()> {
        // Nothing to start: under the portal the thing that would be started is
        // this process, and it is already running. Saying so is the whole job:
        // the caller's next step is to wait for the socket, which is already
        // being served.
        if flatpak::active() {
            return Ok(());
        }
        if !run_quiet("systemctl", &["--user", "start", UNIT]).await? {
            anyhow::bail!("`systemctl --user start {UNIT}` failed, see `hoard sync`");
        }
        Ok(())
    }

    pub async fn restart() -> Result<()> {
        if flatpak::active() {
            return flatpak::cannot_restart().await;
        }
        if !run_quiet("systemctl", &["--user", "restart", UNIT]).await? {
            anyhow::bail!("`systemctl --user restart {UNIT}` failed, see `hoard sync`");
        }
        Ok(())
    }

    pub async fn disable() -> Result<()> {
        if flatpak::active() {
            return flatpak::disable().await;
        }
        ensure_systemd()?;
        run_quiet("systemctl", &["--user", "disable", "--now", UNIT]).await?;
        let path = unit_path()?;
        let _ = std::fs::remove_file(&path);
        run_quiet("systemctl", &["--user", "daemon-reload"]).await?;
        Ok(())
    }

    pub async fn installed() -> bool {
        if flatpak::active() {
            return flatpak::installed();
        }
        unit_path().map(|p| p.exists()).unwrap_or(false)
    }

    /// Is `exe` inside **this** AppImage's ephemeral mount?
    ///
    /// It is compared against `$APPDIR` (the AppImage runtime exports it) and, as a
    /// fallback, against the prefix that runtime mounts under. The distinction
    /// matters: a `hoardd` in `~/.local/bin` outlives the app being closed and is a
    /// perfectly valid path for the unit, even when an AppImage is what declares
    /// it.
    pub fn is_inside_appimage(exe: &Path) -> bool {
        if let Some(appdir) = std::env::var_os("APPDIR").map(PathBuf::from) {
            if exe.starts_with(&appdir) {
                return true;
            }
        }
        // Careful with `Path::starts_with`: it compares **components**, and the real
        // mount is called `.mount_Hoard1a2b`, so it never matches `/tmp/.mount_`. The
        // prefix has to be looked at on the component's name.
        exe.components()
            .any(|c| c.as_os_str().to_string_lossy().starts_with(".mount_"))
    }

    /// Login start inside a Flatpak, where systemd is out of reach.
    ///
    /// Nothing the path above does works in here. The runtime carries no
    /// `systemctl`, and `$XDG_CONFIG_HOME` points at the app's own private
    /// directory, so a unit written there is a unit the host's systemd never
    /// reads: the switch would go on and nothing would start, which is the
    /// exact failure [`Unsupported`] exists to stop us shipping.
    ///
    /// `org.freedesktop.portal.Background` is the sanctioned replacement: ask,
    /// and the portal writes a `.desktop` file on the **host** that starts us
    /// back through `flatpak run`. Portals need no `--talk-name`, since Flatpak's
    /// default bus policy lets every sandbox reach them.
    pub mod flatpak {
        use super::*;

        /// What [`Installed::id`] carries here. There is no unit and no task,
        /// so it names the thing that actually holds the answer.
        const PORTAL: &str = "org.freedesktop.portal.Background";

        /// Which binary the portal should start. It goes through
        /// `flatpak run --command=…`, so it is a name inside `/app/bin` and not
        /// a path on the host.
        const COMMAND: &str = "hoardd";

        pub fn active() -> bool {
            hoard_agent::install::running_under_flatpak()
        }

        /// The file the portal writes when it grants autostart.
        ///
        /// Deliberately built from `$HOME` and not from `$XDG_CONFIG_HOME`: the
        /// latter is redirected into the sandbox, and this file is the host's.
        /// Reading it needs `--filesystem=host`, which the manifest grants for
        /// unrelated reasons: if a future manifest narrows that, this answers
        /// "not installed" and the only cost is asking the portal again, which
        /// it grants without a prompt once the permission is stored.
        fn autostart_file() -> Option<PathBuf> {
            Some(autostart_file_in(
                &home().ok()?,
                &std::env::var("FLATPAK_ID").ok()?,
            ))
        }

        /// [`autostart_file`] with the two things it reads passed in, so the
        /// name can be a test. It has to match what the portal writes to the
        /// letter: get it wrong and autostart reports itself as never
        /// installed, which is silent and re-asks on every start.
        fn autostart_file_in(home: &Path, app_id: &str) -> PathBuf {
            home.join(".config")
                .join("autostart")
                .join(format!("{app_id}.desktop"))
        }

        pub fn installed() -> bool {
            autostart_file().is_some_and(|p| p.exists())
        }

        /// Nothing is written from in here (the portal owns the file), so this
        /// only reports where the answer lives and whether it is already there.
        pub fn declare() -> Result<(Installed, bool)> {
            Ok((
                Installed {
                    manager: "xdg-desktop-portal",
                    id: PORTAL,
                    path: autostart_file(),
                },
                !installed(),
            ))
        }

        pub async fn enable() -> Result<Option<&'static str>> {
            request(true).await?;
            Ok(Some("xdg-desktop-portal"))
        }

        pub async fn disable() -> Result<()> {
            request(false).await
        }

        /// A restart would mean stopping the process making the request and
        /// trusting something to bring it back, and in here there is nothing to
        /// do the bringing. It never comes up in practice, since restarting is what
        /// follows an upgrade, and a sandboxed install updates through its
        /// remote (`Delivery::Managed`), so this says so instead of failing in
        /// a way that reads like a bug.
        pub async fn cannot_restart() -> Result<()> {
            anyhow::bail!(
                "this copy of Hoard runs inside a Flatpak, where the service can't restart \
                 itself. Close the app and open it again, or run `flatpak kill {} && flatpak \
                 run {0}`.",
                std::env::var("FLATPAK_ID").unwrap_or_else(|_| "services.hoard.saves".into())
            )
        }

        /// The one call that matters. `auto_start` is the whole request: `true`
        /// asks for the entry, `false` withdraws it.
        ///
        /// The portal may put a dialog in front of the user the first time, and
        /// its answer is authoritative: a request that comes back with
        /// `auto_start` false was refused, and reporting success there would
        /// leave the switch on over nothing, which is the bug this module was
        /// written to stop.
        async fn request(auto_start: bool) -> Result<()> {
            let response = ashpd::desktop::background::Background::request()
                .auto_start(auto_start)
                .command([COMMAND])
                .reason("Keep syncing your saves after you close the window")
                .send()
                .await
                .context("asking xdg-desktop-portal for background access")?
                .response()
                .context("reading the portal's answer")?;
            if response.auto_start() != auto_start {
                anyhow::bail!(
                    "xdg-desktop-portal refused to {} starting Hoard's sync service at login",
                    if auto_start { "enable" } else { "disable" }
                );
            }
            tracing::info!(
                auto_start,
                background = response.run_in_background(),
                "autostart: the portal answered"
            );
            Ok(())
        }

        #[cfg(test)]
        mod tests {
            use super::*;

            #[test]
            fn the_entry_is_named_after_the_flatpak_id() {
                assert_eq!(
                    autostart_file_in(Path::new("/home/p"), "services.hoard.saves"),
                    Path::new("/home/p/.config/autostart/services.hoard.saves.desktop")
                );
            }
        }
    }
}

// ---- macOS: a launchd LaunchAgent

#[cfg(target_os = "macos")]
mod platform {
    use super::*;

    pub const UNIT: &str = "com.hoard.sync";

    pub fn plist_path() -> Result<PathBuf> {
        Ok(home()?
            .join("Library")
            .join("LaunchAgents")
            .join(format!("{UNIT}.plist")))
    }

    fn log_path() -> Result<PathBuf> {
        Ok(home()?.join("Library").join("Logs").join("hoard-sync.log"))
    }

    async fn current_uid() -> Result<String> {
        let out = tokio::process::Command::new("id")
            .arg("-u")
            .output()
            .await
            .context("running `id -u`")?;
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    /// El plist. Puro y testeable: es el contrato con launchd.
    pub fn plist_text(exe: &str, log: &str) -> String {
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
             <plist version=\"1.0\">\n\
             <dict>\n\
             \t<key>Label</key>\n\t<string>{label}</string>\n\
             \t<key>ProgramArguments</key>\n\t<array>\n\t\t<string>{exe}</string>\n\t</array>\n\
             \t<key>RunAtLoad</key>\n\t<true/>\n\
             \t<key>KeepAlive</key>\n\t<true/>\n\
             \t<key>StandardOutPath</key>\n\t<string>{log}</string>\n\
             \t<key>StandardErrorPath</key>\n\t<string>{log}</string>\n\
             </dict>\n\
             </plist>\n",
            label = UNIT,
        )
    }

    pub fn declare() -> Result<(Installed, bool)> {
        let exe = service_binary();
        let log = log_path()?;
        let path = plist_path()?;
        for dir in [path.parent(), log.parent()].into_iter().flatten() {
            std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        }
        let plist = plist_text(&exe.to_string_lossy(), &log.to_string_lossy());
        let changed = std::fs::read_to_string(&path).ok().as_deref() != Some(plist.as_str());
        if changed {
            std::fs::write(&path, plist).with_context(|| format!("writing {}", path.display()))?;
        }
        Ok((
            Installed {
                manager: "launchd",
                id: UNIT,
                path: Some(path),
            },
            changed,
        ))
    }

    /// launchd draws no line between "install" and "load": the plist in
    /// `~/Library/LaunchAgents` is loaded at the next login already, so writing it
    /// **is** enabling it.
    pub async fn enable() -> Result<Option<&'static str>> {
        Ok(None)
    }

    pub async fn start() -> Result<()> {
        let uid = current_uid().await?;
        let domain = format!("gui/{uid}");
        let plist = plist_path()?;
        let plist = plist.to_string_lossy().to_string();
        // Recarga limpia si ya estaba cargado.
        let _ = run_quiet("launchctl", &["bootout", &domain, &plist]).await;
        if !run_quiet("launchctl", &["bootstrap", &domain, &plist]).await? {
            anyhow::bail!("`launchctl bootstrap {domain}` failed");
        }
        Ok(())
    }

    pub async fn restart() -> Result<()> {
        let uid = current_uid().await?;
        let target = format!("gui/{uid}/{UNIT}");
        if !run_quiet("launchctl", &["kickstart", "-k", &target]).await? {
            anyhow::bail!("`launchctl kickstart {target}` failed");
        }
        Ok(())
    }

    pub async fn disable() -> Result<()> {
        let uid = current_uid().await?;
        let domain = format!("gui/{uid}");
        let plist = plist_path()?;
        let _ = run_quiet("launchctl", &["bootout", &domain, &plist.to_string_lossy()]).await;
        let _ = std::fs::remove_file(&plist);
        Ok(())
    }

    pub async fn installed() -> bool {
        plist_path().map(|p| p.exists()).unwrap_or(false)
    }

    /// The LaunchAgent's executable: the first `<string>` in `ProgramArguments`. We
    /// wrote the plist ourselves ([`plist_text`]), so no plist parser is needed to
    /// read back what we put there.
    pub fn exec_start() -> Option<PathBuf> {
        let text = std::fs::read_to_string(plist_path().ok()?).ok()?;
        let after_key = text.split("<key>ProgramArguments</key>").nth(1)?;
        let open = after_key.find("<string>")? + "<string>".len();
        let rest = &after_key[open..];
        let close = rest.find("</string>")?;
        Some(PathBuf::from(rest[..close].trim()))
    }
}

// ---- Windows: Task Scheduler (per user, at logon)

#[cfg(target_os = "windows")]
mod platform {
    use super::*;

    pub const UNIT: &str = "HoardSync";

    /// The two mechanisms, in the order they're tried.
    const TASK_SCHEDULER: &str = "Task Scheduler";
    const RUN_KEY_MANAGER: &str = "Startup entry (HKCU Run)";

    /// Windows' other way into login start: a value under
    /// `HKCU\...\CurrentVersion\Run`, which Explorer runs when the session opens.
    const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const RUN_VALUE: &str = UNIT;

    pub async fn installed() -> bool {
        task_installed().await || run_entry().is_some()
    }

    async fn task_installed() -> bool {
        run_quiet("schtasks", &["/Query", "/TN", UNIT])
            .await
            .unwrap_or(false)
    }

    fn open_run_key(write: bool) -> Result<winreg::RegKey> {
        use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};
        let hkcu = winreg::RegKey::predef(HKEY_CURRENT_USER);
        let access = if write {
            KEY_READ | KEY_WRITE
        } else {
            KEY_READ
        };
        // `create_subkey_with_flags` opens a key that already exists; `Run` is
        // in every profile, but one just created by an unattended installer may
        // not have it yet.
        hkcu.create_subkey_with_flags(RUN_KEY, access)
            .map(|(key, _)| key)
            .with_context(|| format!("opening HKCU\\{RUN_KEY}"))
    }

    /// The command recorded in the Run entry, if there is one.
    fn run_entry() -> Option<String> {
        let key = open_run_key(false).ok()?;
        let value: String = key.get_value(RUN_VALUE).ok()?;
        (!value.trim().is_empty()).then_some(value)
    }

    /// Write the Run entry pointing at `exe`.
    ///
    /// Quoted for the same reason the systemd unit is: Explorer splits the value
    /// on spaces, and `C:\Program Files\...` is the most common path there is.
    fn set_run_entry(exe: &Path) -> Result<()> {
        let key = open_run_key(true)?;
        let command = format!("\"{}\"", exe.display());
        key.set_value(RUN_VALUE, &command)
            .with_context(|| format!("writing HKCU\\{RUN_KEY}\\{RUN_VALUE}"))
    }

    fn remove_run_entry() {
        if let Ok(key) = open_run_key(true) {
            let _ = key.delete_value(RUN_VALUE);
        }
    }

    /// Where we write down which executable the task ended up with.
    ///
    /// The other two platforms keep their definition in a file we can read; the Task
    /// Scheduler keeps it in a database of its own, and getting it out is a `schtasks
    /// /Query /XML`, which is a subprocess, and [`super::installed_exec_start`] calls
    /// this from synchronous paths that resolve the daemon's path often. So it is
    /// written down at registration time and read from here, which costs one file
    /// read.
    fn recorded_exec_path() -> Option<PathBuf> {
        Some(
            hoard_agent::config::CliConfig::project_dirs()
                .ok()?
                .config_dir()
                .join("service-exec.txt"),
        )
    }

    pub fn exec_start() -> Option<PathBuf> {
        let recorded = std::fs::read_to_string(recorded_exec_path()?).ok()?;
        let trimmed = recorded.trim();
        (!trimmed.is_empty()).then(|| PathBuf::from(trimmed))
    }

    /// Writes down the task's executable. Best-effort: when it cannot be written,
    /// resolving the daemon falls back to the usual sibling or `PATH`.
    fn record_exec(exe: &Path) {
        let Some(path) = recorded_exec_path() else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, exe.to_string_lossy().as_bytes());
    }

    /// The caller's account, as `DOMAIN\user`: that is what the trigger and the
    /// principal are scoped to. With no domain the bare name will do (the Task
    /// Scheduler resolves it against the local machine).
    fn current_account() -> Result<String> {
        let user = std::env::var("USERNAME")
            .ok()
            .filter(|s| !s.is_empty())
            .context("no USERNAME in the environment")?;
        let domain = std::env::var("USERDOMAIN").ok().filter(|s| !s.is_empty());
        Ok(match domain {
            Some(d) => format!("{d}\\{user}"),
            None => user,
        })
    }

    /// The task is not a file we can compare, so it is always declared "changed"
    /// and `enable` rewrites it (`/F`). That is what makes an update moving the
    /// `.exe` re-point the task on its own, just as the desktop reaffirms its own
    /// autostart entry on every start.
    pub fn declare() -> Result<(Installed, bool)> {
        // A Run entry only exists on a machine that already fell back (`enable`
        // deletes it the moment the task takes), so it is the honest answer for
        // callers that re-declare without re-enabling: `restart`, which would
        // otherwise report a Task Scheduler that refused this machine.
        let manager = if run_entry().is_some() {
            RUN_KEY_MANAGER
        } else {
            TASK_SCHEDULER
        };
        Ok((
            Installed {
                manager,
                id: UNIT,
                path: None,
            },
            true,
        ))
    }

    /// Register login start. Tries the task; if the Task Scheduler refuses it,
    /// falls back to the user's Run entry.
    ///
    /// The task is tried first because it is the better one: it survives an
    /// Explorer that doesn't come up, it has a single-instance policy, and it
    /// can be fired by hand (`/Run`). But **it can say no without an elevated
    /// console**, and that was the end of the road: 81 events across 3 users
    /// whose only way out was re-running it as administrator, asking someone to
    /// open a PowerShell so their game saves itself.
    ///
    /// The Run entry never needs elevation (it is the user's own profile) and it
    /// does the one thing that matters here: start `hoardd` at logon. What it
    /// doesn't do is supervise, so `schtasks /Run` and `/End` have nobody left
    /// to talk to and those paths start and stop the daemon directly instead.
    pub async fn enable() -> Result<Option<&'static str>> {
        let exe = service_binary();
        let account = current_account()?;
        match create_task(&exe, &account).await {
            Ok(()) => {
                // With the task in place the Run entry is redundant and in the
                // way: two starts in the same logon means one `hoardd` losing
                // the socket bind and exiting, an error in the log for nothing.
                remove_run_entry();
                record_exec(&exe);
                Ok(Some(TASK_SCHEDULER))
            }
            Err(err) => {
                tracing::info!(
                    error = %format!("{err:#}"),
                    "autostart: the Task Scheduler refused the task; using the user Run entry"
                );
                set_run_entry(&exe).with_context(|| {
                    format!("the Task Scheduler refused the task ({err:#}) and the user Run entry")
                })?;
                record_exec(&exe);
                Ok(Some(RUN_KEY_MANAGER))
            }
        }
    }

    async fn create_task(exe: &Path, account: &str) -> Result<()> {
        let xml = super::task_xml(&exe.to_string_lossy(), account);

        // `/XML` reads the definition from a file; it is written alongside the
        // process's other temporaries, with the pid in the name, so two shells do not
        // tread on each other.
        let path = std::env::temp_dir().join(format!("hoard-sync-{}.xml", std::process::id()));
        std::fs::write(&path, super::to_utf16le_with_bom(&xml))
            .with_context(|| format!("writing {}", path.display()))?;
        let created = run_quiet(
            "schtasks",
            &[
                "/Create",
                "/TN",
                UNIT,
                "/XML",
                &path.to_string_lossy(),
                "/F",
            ],
        )
        .await;
        let _ = std::fs::remove_file(&path);

        if !created? {
            anyhow::bail!("`schtasks /Create /TN {UNIT}` was refused");
        }
        Ok(())
    }

    /// Start the service now.
    ///
    /// With a task, by firing it. Without one there is nothing to fire (the Run
    /// entry only acts at logon), so the daemon comes up the same way a client
    /// would bring it up. `install` then waits for it to listen, so a failed
    /// start doesn't pass for good down either path.
    pub async fn start() -> Result<()> {
        if task_installed().await {
            if !run_quiet("schtasks", &["/Run", "/TN", UNIT]).await? {
                anyhow::bail!("`schtasks /Run /TN {UNIT}` failed, see `hoard sync`");
            }
            return Ok(());
        }
        let endpoint = Endpoint::resolve().context("resolving the hoardd endpoint")?;
        Client::ensure_running(&endpoint, "hoard autostart (start)")
            .await
            .context("starting the Hoard service")?;
        Ok(())
    }

    pub async fn restart() -> Result<()> {
        let _ = run_quiet("schtasks", &["/End", "/TN", UNIT]).await;
        start().await
    }

    /// Remove both entries. Both, always: a machine that went through the task
    /// and later through the Run entry (or the other way round, after an update
    /// that fixed its permissions) would have the other one still set, and
    /// "turn off autostart" would leave the sync starting on its own anyway.
    pub async fn disable() -> Result<()> {
        let _ = run_quiet("schtasks", &["/End", "/TN", UNIT]).await;
        let had_task = task_installed().await;
        remove_run_entry();
        if had_task && !run_quiet("schtasks", &["/Delete", "/TN", UNIT, "/F"]).await? {
            anyhow::bail!("`schtasks /Delete /TN {UNIT}` failed");
        }
        // With no login start there is no recorded executable: leaving it would
        // lie to `daemon_binary`, which treats it as the most authoritative
        // answer there is.
        if let Some(path) = recorded_exec_path() {
            let _ = std::fs::remove_file(path);
        }
        Ok(())
    }
}

// =======================================================================
// Cualquier otro SO
// =======================================================================

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
mod platform {
    use super::*;

    pub const UNIT: &str = "hoard-sync";

    pub fn declare() -> Result<(Installed, bool)> {
        Err(anyhow::Error::new(LoginStartUnsupported {
            kind: Unsupported::NoServiceManager,
            detail: "no service backend for this OS; run `hoardd` under your own supervisor"
                .to_string(),
        }))
    }
    pub async fn enable() -> Result<Option<&'static str>> {
        anyhow::bail!("no service backend for this OS")
    }
    pub async fn start() -> Result<()> {
        anyhow::bail!("no service backend for this OS")
    }
    pub async fn restart() -> Result<()> {
        anyhow::bail!("no service backend for this OS")
    }
    pub async fn disable() -> Result<()> {
        anyhow::bail!("no service backend for this OS")
    }
    pub async fn installed() -> bool {
        false
    }
    pub fn exec_start() -> Option<PathBuf> {
        None
    }
}

// ---- Windows: XML de la tarea (puro, testeable en cualquier SO) --------

/// Escapa un valor para contenido/atributo XML.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

/// `HoardSync`'s XML: runs `hoardd` at `user`'s logon, as `user` and without
/// elevating.
///
/// `schtasks /Create /SC ONLOGON` demands an elevated console even with `/RL
/// LIMITED`; registering this XML, whose trigger and principal are scoped to the
/// caller's own account, does not. (Both checked against a real Windows machine with
/// a filtered token: ONLOGON gave "Access denied", this XML created the task.)
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn task_xml(exe: &str, user: &str) -> String {
    let exe = xml_escape(exe);
    let user = xml_escape(user);
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-16\"?>\n\
         <Task version=\"1.2\" xmlns=\"http://schemas.microsoft.com/windows/2004/02/mit/task\">\n\
         \x20 <Triggers>\n\
         \x20   <LogonTrigger>\n\
         \x20     <UserId>{user}</UserId>\n\
         \x20   </LogonTrigger>\n\
         \x20 </Triggers>\n\
         \x20 <Principals>\n\
         \x20   <Principal id=\"Author\">\n\
         \x20     <UserId>{user}</UserId>\n\
         \x20     <LogonType>InteractiveToken</LogonType>\n\
         \x20     <RunLevel>LeastPrivilege</RunLevel>\n\
         \x20   </Principal>\n\
         \x20 </Principals>\n\
         \x20 <Settings>\n\
         \x20   <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>\n\
         \x20   <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>\n\
         \x20   <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>\n\
         \x20   <StartWhenAvailable>true</StartWhenAvailable>\n\
         \x20   <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>\n\
         \x20 </Settings>\n\
         \x20 <Actions Context=\"Author\">\n\
         \x20   <Exec>\n\
         \x20     <Command>{exe}</Command>\n\
         \x20   </Exec>\n\
         \x20 </Actions>\n\
         </Task>\n",
    )
}

/// The Task Scheduler only ingests the XML reliably as UTF-16 LE with a BOM: a
/// UTF-8 file (even with the matching declaration) dies inside `schtasks /Create
/// /XML` with "unable to switch the encoding", checked against a real Windows
/// machine. [`task_xml`]'s declaration says UTF-16 to match.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn to_utf16le_with_bom(s: &str) -> Vec<u8> {
    let mut out = vec![0xFF, 0xFE];
    for unit in s.encode_utf16() {
        out.extend_from_slice(&unit.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La unidad tiene que ejecutar **el daemon**, no un cliente: desde el 4b/4c
    /// `hoard sync run` es un espectador, y supervisar a un espectador significa
    /// que `systemctl --user stop` no para el sync.
    #[cfg(target_os = "linux")]
    #[test]
    fn the_unit_execs_the_daemon_itself() {
        let unit = platform::unit_text("/usr/local/bin/hoardd");
        assert!(
            unit.contains("ExecStart=\"/usr/local/bin/hoardd\"\n"),
            "unexpected ExecStart:\n{unit}"
        );
        assert!(
            !unit.contains("sync run"),
            "the unit must not exec a client"
        );
        // With no `WantedBy` there is no start at boot, which is the module's point.
        assert!(unit.contains("WantedBy=default.target"));
    }

    /// Una ruta con espacios (un AppImage en `~/Mis programas/`) sobrevive al
    /// tokenizador de systemd gracias a las comillas.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_path_with_spaces_survives_the_unit() {
        let unit = platform::unit_text("/home/ada/Mis programas/hoardd");
        assert!(unit.contains("ExecStart=\"/home/ada/Mis programas/hoardd\""));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn the_agent_execs_the_daemon_itself() {
        let plist = platform::plist_text("/Applications/Hoard.app/Contents/MacOS/hoardd", "/tmp/l");
        assert!(plist.contains("<string>/Applications/Hoard.app/Contents/MacOS/hoardd</string>"));
        assert!(!plist.contains("<string>sync</string>"));
        assert!(plist.contains("<key>RunAtLoad</key>"));
    }

    #[test]
    fn escapes_the_five_xml_metacharacters() {
        assert_eq!(
            xml_escape(r#"a&b<c>d"e'f"#),
            "a&amp;b&lt;c&gt;d&quot;e&apos;f"
        );
        assert_eq!(
            xml_escape(r"C:\Program Files\hoardd.exe"),
            r"C:\Program Files\hoardd.exe"
        );
    }

    /// The Windows task: scoped to the account that creates it (never the machine)
    /// and running the daemon **with no arguments**, since there is no `sync run` in
    /// the way any more.
    #[test]
    fn task_xml_scopes_the_trigger_and_principal_to_the_account() {
        let xml = task_xml(r"C:\Program Files\Hoard\hoardd.exe", r"CORP\ada");
        assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"UTF-16\"?>"));
        assert!(xml.contains("<MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>"));
        assert!(xml.contains("<LogonTrigger>\n      <UserId>CORP\\ada</UserId>"));
        assert!(xml.contains("<Principal id=\"Author\">\n      <UserId>CORP\\ada</UserId>"));
        assert!(xml.contains("<RunLevel>LeastPrivilege</RunLevel>"));
        assert!(xml.contains("<Command>C:\\Program Files\\Hoard\\hoardd.exe</Command>"));
        assert!(
            !xml.contains("<Arguments>"),
            "the daemon takes no arguments"
        );
    }

    #[test]
    fn task_xml_escapes_the_exe_path() {
        let xml = task_xml(r"C:\R&D\hoardd.exe", "ada");
        assert!(xml.contains("<Command>C:\\R&amp;D\\hoardd.exe</Command>"));
    }

    #[test]
    fn utf16le_bom_encoding_round_trips() {
        let bytes = to_utf16le_with_bom("<a>ñ</a>");
        assert_eq!(&bytes[..2], &[0xFF, 0xFE], "BOM must lead the file");
        assert_eq!(bytes.len() % 2, 0, "UTF-16 LE is an even byte count");
        let units: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        assert_eq!(String::from_utf16(&units).unwrap(), "<a>ñ</a>");
    }

    /// The unit declares **this installation's** daemon, not the one another
    /// installation would have declared. If it looked at the installed unit, an
    /// update that moved the binary would rewrite it with the old path it had just
    /// read and the service would start the previous binary for ever.
    #[test]
    fn the_unit_declares_this_installations_daemon() {
        assert_eq!(service_binary(), crate::client::own_daemon_binary());
    }

    /// And a client asks for **the machine's** daemon, which starts from exactly
    /// what the unit says. They are two different questions, hence two functions;
    /// what must not happen is a client bringing up a `hoardd` different from the one
    /// the system already starts.
    #[cfg(target_os = "linux")]
    #[test]
    fn the_exec_start_we_write_is_the_one_we_read_back() {
        let unit = platform::unit_text("/opt/hoard/hoardd");
        let parsed = unit
            .lines()
            .find_map(|l| l.trim().strip_prefix("ExecStart="))
            .map(|raw| raw.trim().trim_matches('"'))
            .map(PathBuf::from);
        assert_eq!(parsed, Some(PathBuf::from("/opt/hoard/hoardd")));
    }

    /// Un `hoardd` fuera del montaje del AppImage es una ruta perfectamente
    /// estable, y es lo que permite que en SteamOS la app vaya en AppImage y el
    /// sync arranque igualmente en boot.
    #[cfg(target_os = "linux")]
    #[test]
    fn only_a_daemon_inside_the_mount_blocks_login_start() {
        assert!(platform::is_inside_appimage(Path::new(
            "/tmp/.mount_Hoard1a2b/usr/bin/hoardd"
        )));
        assert!(!platform::is_inside_appimage(Path::new(
            "/home/ada/.local/bin/hoardd"
        )));
        assert!(!platform::is_inside_appimage(Path::new("/usr/bin/hoardd")));
    }

    /// An AppImage is no longer a dead end: the daemon it carries gets copied
    /// somewhere that survives closing the app, and *that* is what the unit
    /// execs. Staging into a Hoard-owned directory is the whole point: the copy
    /// must never land on top of a `hoardd` the core installer put there.
    #[cfg(target_os = "linux")]
    #[test]
    fn the_appimages_daemon_is_staged_somewhere_stable() {
        let home = tempfile::tempdir().expect("tempdir");
        let data = home.path().join("data");
        let mount = home.path().join(".mount_Hoard9z9z/usr/bin");
        std::fs::create_dir_all(&mount).unwrap();
        let bundled = mount.join("hoardd");
        std::fs::write(&bundled, b"#!/bin/sh\nexit 0\n").unwrap();

        let staged = temp_env(&data, || platform::stage_stable_daemon(&bundled)).expect("staged");
        assert!(
            staged.is_file(),
            "the copy has to exist: {}",
            staged.display()
        );
        assert!(
            !platform::is_inside_appimage(&staged),
            "a copy still inside the mount would vanish with the app: {}",
            staged.display()
        );
        // Executable, or the unit dies with 203/EXEC at the next login and the
        // only trace is a line in a journal nobody is reading.
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&staged).unwrap().permissions().mode();
        assert_eq!(
            mode & 0o111,
            0o111,
            "staged copy isn't executable: {mode:o}"
        );
    }

    /// The re-stage path is the one that breaks silently: the destination can be
    /// the binary the service is running, so it is replaced by `rename` and the
    /// stamp decides whether there is anything to do at all.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_staged_daemon_is_replaced_not_written_over() {
        use std::os::unix::fs::MetadataExt;
        let home = tempfile::tempdir().expect("tempdir");
        let data = home.path().join("data");
        let bundled = home.path().join("hoardd-bundled");
        std::fs::write(&bundled, b"new").unwrap();

        let dir = data.join("hoard").join("bin");
        std::fs::create_dir_all(&dir).unwrap();
        let dest = dir.join("hoardd");
        std::fs::write(&dest, b"old").unwrap();
        // A stamp from another version: this is what makes it re-stage.
        std::fs::write(dir.join("hoardd.version"), "0.0.0-previous").unwrap();
        let before = std::fs::metadata(&dest).unwrap().ino();

        let staged = temp_env(&data, || platform::stage_stable_daemon(&bundled)).expect("staged");
        assert_eq!(std::fs::read(&staged).unwrap(), b"new");
        assert_ne!(
            std::fs::metadata(&staged).unwrap().ino(),
            before,
            "the copy was written in place; a running daemon would have given ETXTBSY"
        );

        // And with the stamp current there is nothing to do: re-copying tens of
        // MB on every app start would be a toll for nothing.
        let after = std::fs::metadata(&staged).unwrap().ino();
        std::fs::write(&bundled, b"newer, but same version").unwrap();
        let again = temp_env(&data, || platform::stage_stable_daemon(&bundled)).expect("staged");
        assert_eq!(std::fs::metadata(&again).unwrap().ino(), after);
    }

    /// `XDG_DATA_HOME` is process-wide, so the two staging tests have to take
    /// turns with it or they read each other's directory.
    #[cfg(target_os = "linux")]
    fn temp_env<T>(data: &Path, body: impl FnOnce() -> T) -> T {
        use std::sync::Mutex;
        static ENV: Mutex<()> = Mutex::new(());
        let _guard = ENV.lock().unwrap_or_else(|p| p.into_inner());
        let previous = std::env::var_os("XDG_DATA_HOME");
        // SAFETY: the lock above is what serialises this against the other test;
        // nothing else in this crate's suite touches `XDG_DATA_HOME`.
        unsafe { std::env::set_var("XDG_DATA_HOME", data) };
        let out = body();
        match previous {
            Some(v) => unsafe { std::env::set_var("XDG_DATA_HOME", v) },
            None => unsafe { std::env::remove_var("XDG_DATA_HOME") },
        }
        out
    }

    /// The reason travels typed. Classifying it by matching on the message is
    /// what breaks the first time somebody rewords the sentence, and rewording a
    /// user-facing sentence is the most ordinary edit there is.
    #[test]
    fn the_unsupported_reason_survives_the_error_chain() {
        let err = anyhow::Error::new(LoginStartUnsupported {
            kind: Unsupported::NoStablePath,
            detail: "no stable path".to_string(),
        })
        .context("installing the Hoard sync service");
        assert_eq!(unsupported_reason(&err), Some(Unsupported::NoStablePath));
        assert_eq!(Unsupported::NoStablePath.as_str(), "no_stable_path");

        // And an ordinary failure carries no reason: the window falls back to
        // the generic line instead of blaming the app's format.
        let plain = anyhow::anyhow!("`systemctl --user enable` failed");
        assert_eq!(unsupported_reason(&plain), None);
    }
}
