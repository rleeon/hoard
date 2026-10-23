//! **Taking Hoard off a machine**, the same way it was put on.
//!
//! The mirror of [`super::fetch`], and it has to be a mirror: a `.deb` comes
//! off with `dpkg -r`, an AppImage with `unlink`, an NSIS install by running
//! the uninstaller it left behind. Deleting the files under a package manager's
//! feet would leave `dpkg` believing Hoard is installed for ever.
//!
//! # What this never touches
//!
//! **Your saves and your settings stay.** Uninstalling removes the program, not
//! the backups it made: those are the point of having used it, and they can be
//! tens of gigabytes, and a user who is only switching machines or reinstalling
//! would have no way to get them back. The paths are returned in [`Removed`] so
//! whoever asked can say where they are; deleting them is a separate decision
//! that has to be made by a person.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use super::{fetch, Delivery};

/// Stops anything of ours that is running, so its files can be replaced or
/// deleted.
///
/// On Unix this is a courtesy: a running binary can be unlinked and the kernel
/// keeps the inode alive until the process exits. On Windows it is the whole
/// operation: an open executable cannot be deleted at all, and this is exactly
/// the failure the NSIS hook exists to prevent ("Error opening file for
/// writing"). Best-effort throughout: something we can't stop shows up as a
/// deletion that fails, with its own message.
pub async fn stop_running() {
    // `hoard-screen` no longer ships, but an older install can still have one
    // running out of the install dir.
    let names = ["hoard-desktop", "hoard-screen", "hoardd"];

    #[cfg(windows)]
    for name in names {
        let _ = tokio::process::Command::new("taskkill")
            .args(["/F", "/IM", &format!("{name}.exe")])
            .output()
            .await;
    }

    #[cfg(unix)]
    for name in names {
        let _ = tokio::process::Command::new("pkill")
            .args(["-x", name])
            .output()
            .await;
    }

    // `taskkill` and `pkill` both return before the handles are actually gone.
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
}

/// Takes the graphical app off, by the route it arrived on.
///
/// Returns what was removed, or `Ok(None)` when there was nothing there.
pub async fn desktop(
    delivery: Delivery,
    path: Option<&Path>,
    noninteractive: bool,
) -> Result<Option<PathBuf>> {
    let Some(path) = path else {
        return Ok(None);
    };
    if !path.exists() {
        return Ok(None);
    }

    match delivery {
        Delivery::Deb => {
            let package = owning_package("dpkg", &["-S"], path)
                .await
                .unwrap_or_else(|| "hoard".to_string());
            fetch::elevated_argv(&["dpkg".into(), "-r".into(), package], noninteractive).await?;
            Ok(Some(path.to_path_buf()))
        }
        Delivery::Rpm => {
            let package = owning_package("rpm", &["-qf", "--queryformat", "%{NAME}"], path)
                .await
                .unwrap_or_else(|| "hoard".to_string());
            // The Bazzite report of 22-10-2026: the package was layered with
            // `rpm-ostree`, and `rpm -e` against a read-only `/usr` fails after
            // asking for the password. Only the person can take a layer off, and
            // it needs a reboot, so they get the exact command.
            if super::image_based() {
                bail!(
                    "this system is image-based: remove the old package with \
                     `rpm-ostree uninstall {package}` and restart"
                );
            }
            fetch::elevated_argv(&["rpm".into(), "-e".into(), package], noninteractive).await?;
            Ok(Some(path.to_path_buf()))
        }
        Delivery::AppImage => {
            std::fs::remove_file(path).with_context(|| format!("deleting {}", path.display()))?;
            remove_desktop_entry();
            Ok(Some(path.to_path_buf()))
        }
        Delivery::Nsis => {
            let dir = path.parent().context("the app has no parent directory")?;
            nsis(dir).await?;
            Ok(Some(path.to_path_buf()))
        }
        Delivery::Msi => {
            bail!("the MSI install is removed by its product code, from the list of leftovers")
        }
        Delivery::Exe => {
            #[cfg(windows)]
            super::windows::unregister().await;
            std::fs::remove_file(path).with_context(|| format!("deleting {}", path.display()))?;
            let _ = std::fs::remove_file(path.with_extension("old"));
            if let Some(dir) = path.parent() {
                let _ = std::fs::remove_dir(dir);
            }
            Ok(Some(path.to_path_buf()))
        }
        Delivery::Dmg => {
            // A `.app` is a directory, and it is ours: it was copied in whole
            // by `install_dmg`, so it comes out whole.
            std::fs::remove_dir_all(path)
                .with_context(|| format!("deleting {}", path.display()))?;
            Ok(Some(path.to_path_buf()))
        }
        Delivery::Managed => {
            bail!(
                "this copy is managed by your package manager; remove it the same way you \
                 installed it"
            )
        }
    }
}

/// Runs the NSIS installer's own uninstaller, silently and in place.
///
/// It is the only thing that knows what it wrote: shortcuts, the registry
/// entry, the PATH. Deleting the folder ourselves would leave every one of
/// those behind.
pub async fn nsis(dir: &Path) -> Result<()> {
    let uninstaller = dir.join("uninstall.exe");
    if !uninstaller.is_file() {
        bail!(
            "no uninstaller at {}; remove Hoard from Settings › Apps instead",
            uninstaller.display()
        );
    }
    // `_?=` is what makes this synchronous. Without it an NSIS uninstaller
    // copies itself to the temp directory, launches that copy detached and
    // returns success immediately, so the caller reports "removed" while the
    // files are still there, and whatever it does next races the deletion.
    // With `_?=` it runs in place and blocks; the price is that it can no
    // longer delete its own executable, which is why we do it here.
    //
    // It has to arrive bare, spaces and all: NSIS only sees `_?=` after a space
    // and takes the rest of the line as the path. `arg` quotes anything with a
    // space in it, and under `C:\Users\Juan Pérez` that quote alone turned this
    // back into the detached run described above.
    let mut command = tokio::process::Command::new(&uninstaller);
    command.arg("/S");
    let in_place = format!("_?={}", dir.display());
    #[cfg(windows)]
    command.raw_arg(in_place);
    #[cfg(not(windows))]
    command.arg(in_place);
    let status = command
        .status()
        .await
        .with_context(|| format!("running {}", uninstaller.display()))?;
    if !status.success() {
        bail!("the uninstaller exited with {status}");
    }
    let _ = std::fs::remove_file(&uninstaller);
    // Only if it emptied: anything the user dropped in there is theirs.
    let _ = std::fs::remove_dir(dir);
    Ok(())
}

/// Removes a per-machine MSI install. It needs an administrator, so only with
/// somebody there to answer the UAC prompt.
pub async fn msi(product_code: &str, interactive: bool) -> Result<()> {
    if !interactive {
        bail!("removing the MSI install needs an administrator; run Hoard Setup to finish");
    }
    let status = tokio::process::Command::new("msiexec")
        .args(["/x", product_code, "/passive", "/norestart"])
        .status()
        .await
        .context("running msiexec")?;
    // 1605: "this product is not installed", already gone.
    if !status.success() && status.code() != Some(1605) {
        bail!("msiexec exited with {status}");
    }
    Ok(())
}

/// Which native package manager owns `path`: a `.deb` or an `.rpm` we shipped.
/// `None` when neither does, which means somebody else's package.
pub async fn package_manager_of(path: &Path) -> Option<Delivery> {
    if owning_package("dpkg", &["-S"], path).await.is_some() {
        return Some(Delivery::Deb);
    }
    if owning_package("rpm", &["-qf", "--queryformat", "%{NAME}"], path)
        .await
        .is_some()
    {
        return Some(Delivery::Rpm);
    }
    None
}

/// Which package owns `path`, according to the package manager.
///
/// Asked rather than assumed because the package name is CI's to choose and
/// ours to guess wrong; `dpkg -r` against a name that doesn't exist fails with
/// a message about the wrong thing entirely. `None` falls back to the
/// conventional name, which is better than not trying.
async fn owning_package(program: &str, args: &[&str], path: &Path) -> Option<String> {
    let out = tokio::process::Command::new(program)
        .args(args)
        .arg(path)
        .output()
        .await
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let first = text.lines().next()?.trim();
    // `dpkg -S` answers "package: /path"; `rpm -qf --queryformat %{NAME}` answers
    // the bare name.
    let name = first.split(':').next()?.trim();
    (!name.is_empty()).then(|| name.to_string())
}

/// The menu entry the AppImage route writes, and its icon. Its counterpart is
/// `fetch::write_desktop_entry`.
///
/// Only the two files: the `hicolor` directories around the icon are the
/// user's, and other apps keep theirs in them.
fn remove_desktop_entry() {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return;
    };
    let entry = home
        .join(".local")
        .join("share")
        .join("applications")
        .join("dev.hoard.desktop.desktop");
    let _ = std::fs::remove_file(entry);
    let _ = std::fs::remove_file(fetch::menu_icon_path(&home));
}

/// The app's own start at login, which `tauri-plugin-autostart` names after the
/// product: `Hoard` under `HKCU\...\Run`, `~/.config/autostart/Hoard.desktop`,
/// `~/Library/LaunchAgents/Hoard.plist`. Not the service's, which is
/// `hoardd::autostart`'s. Left behind, it named a deleted app at every login.
///
/// Only when it starts `hoard-desktop`: the name is generic enough to be
/// somebody else's, and a Flatpak's entry belongs to the Flatpak.
pub fn app_autostart() {
    const NAME: &str = "Hoard";
    let ours = |text: &str| text.contains("hoard-desktop") && !text.contains("flatpak");
    #[cfg(windows)]
    {
        use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};
        use winreg::RegKey;
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let run = r"Software\Microsoft\Windows\CurrentVersion\Run";
        let Ok(key) = hkcu.open_subkey_with_flags(run, KEY_READ | KEY_WRITE) else {
            return;
        };
        if key
            .get_value::<String, _>(NAME)
            .is_ok_and(|value| ours(&value))
        {
            let _ = key.delete_value(NAME);
            let approved =
                r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
            if let Ok(key) = hkcu.open_subkey_with_flags(approved, KEY_WRITE) {
                let _ = key.delete_value(NAME);
            }
        }
    }
    #[cfg(unix)]
    {
        let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
            return;
        };
        let entry = if cfg!(target_os = "macos") {
            home.join("Library")
                .join("LaunchAgents")
                .join(format!("{NAME}.plist"))
        } else {
            home.join(".config")
                .join("autostart")
                .join(format!("{NAME}.desktop"))
        };
        if std::fs::read_to_string(&entry).is_ok_and(|text| ours(&text)) {
            let _ = std::fs::remove_file(&entry);
        }
    }
}

/// Deletes `hoard` and `hoardd` from `dir`, and returns what actually went.
///
/// Only those two names and their own debris (the copy an update parked as
/// `.old` on Windows, a staging `.new` an interrupted one left), never the
/// directory itself on Unix: `~/.local/bin` belongs to the user and is full of
/// things that are not ours. On Windows the folder is Hoard's, and it goes when
/// it is empty.
pub fn core(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut gone = Vec::new();
    let mut failed: Vec<String> = Vec::new();
    for name in ["hoard", "hoardd"] {
        let path = dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
        if !path.exists() {
            continue;
        }
        match std::fs::remove_file(&path) {
            Ok(()) => gone.push(path),
            Err(e) => failed.push(format!("{}: {e}", path.display())),
        }
    }
    for name in ["hoard", "hoardd"] {
        let _ = std::fs::remove_file(dir.join(format!("{name}.old")));
        let _ = std::fs::remove_file(dir.join(format!(".{name}.new")));
    }
    if !failed.is_empty() {
        bail!("could not delete {}", failed.join(", "));
    }
    if cfg!(windows) {
        let _ = std::fs::remove_dir(dir);
    }
    Ok(gone)
}

/// Takes the line `super::ensure_on_shell_path` added back out.
///
/// Only the exact pair it wrote: its comment and the export directly under it.
/// Anything else in that file is the user's, including a `PATH` line they wrote
/// themselves that happens to name the same directory.
pub fn shell_path_line(dir: &Path) {
    #[cfg(target_os = "windows")]
    {
        // The counterpart of `super::shell_path`, which put this directory
        // into the user's `Path`. Leaving it is litter that points at nothing,
        // and the next install would find it already present and skip writing
        // it, so the asymmetry is not even harmless.
        use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let Ok(env) = hkcu.open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE) else {
            return;
        };
        let current: String = env.get_value("Path").unwrap_or_default();
        let want = dir.to_string_lossy();
        let kept: Vec<&str> = current
            .split(';')
            .filter(|p| !p.trim().is_empty())
            .filter(|p| !p.trim().eq_ignore_ascii_case(want.trim()))
            .collect();
        if kept.len() != current.split(';').filter(|p| !p.trim().is_empty()).count()
            && env.set_value("Path", &kept.join(";")).is_ok()
        {
            super::broadcast_environment_change();
        }
    }
    #[cfg(unix)]
    {
        let _ = dir;
        let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
            return;
        };
        const MARKER: &str = "# Added by the Hoard installer";
        for rc in [".zshrc", ".bashrc", ".profile"] {
            let path = home.join(rc);
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            if !text.contains(MARKER) {
                continue;
            }
            let mut out = String::with_capacity(text.len());
            let mut skip_next = false;
            for line in text.lines() {
                if skip_next {
                    skip_next = false;
                    // Only skip it if it is the export we wrote.
                    if line.trim_start().starts_with("export PATH=") {
                        continue;
                    }
                }
                if line.trim() == MARKER {
                    skip_next = true;
                    continue;
                }
                out.push_str(line);
                out.push('\n');
            }
            let _ = std::fs::write(&path, out);
        }
    }
}

/// Forgets what was installed. Called last: while it exists, an update would
/// still believe in the install we just took apart.
pub fn manifest() -> Result<()> {
    let path = super::Manifest::path()?;
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e).with_context(|| format!("deleting {}", path.display())),
    }
}

/// Where the user's own things live: saves, settings, the local database.
///
/// Returned so an uninstaller can say "your backups are still here", never so
/// it can delete them. See this module's header.
pub fn kept_data() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(dirs) = crate::config::CliConfig::project_dirs() {
        // State first, config second. Whoever reports this shows the first one,
        // and the state directory is the one worth pointing at: it holds what
        // Hoard learned about this machine: which folders are tracked, the
        // sync history, the local database. The config directory is a
        // `config.toml` that can be rewritten in a minute.
        for dir in [dirs.data_local_dir(), dirs.data_dir(), dirs.config_dir()] {
            if dir.exists() && !out.contains(&dir.to_path_buf()) {
                out.push(dir.to_path_buf());
            }
        }
    }
    out
}
