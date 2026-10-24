//! Miscellaneous commands used by the dev scaffolding.

/// Round-trips a name through the Rust backend so the UI can prove the
/// `invoke()` plumbing works end-to-end.
#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello from Rust, {name}! 🪙")
}

/// Open a web URL in the user's default browser with a **sanitized** child
/// environment. Replaces the frontend `@tauri-apps/plugin-shell` `open` for
/// every outward link (OAuth sign-in, upgrade/billing pages, terms).
///
/// Why not just use the plugin: inside an AppImage, `AppRun` exports
/// `LD_LIBRARY_PATH` / `LD_PRELOAD` / `GTK_PATH` / … pointing at Hoard's bundled
/// libraries. A browser spawned via the plugin inherits them and loads our
/// (version-mismatched) Wayland/EGL libs instead of the host's; on SteamOS and
/// Bazzite it then dies before drawing a window, so the "Sign in" button opened
/// *nothing* even though the loopback listener was already up. We strip those
/// vars (restoring `*_ORIG` if AppRun saved them) so the browser starts against
/// the system libraries. On macOS/Windows there's no such pollution; we just
/// hand the URL to the platform opener.
#[tauri::command]
pub async fn open_external(url: String) -> Result<(), String> {
    // Never feed an arbitrary string to a shell or opener: web schemes only.
    let allowed =
        url.starts_with("https://") || url.starts_with("http://") || url.starts_with("mailto:");
    if !allowed {
        return Err("refusing to open non-web URL".into());
    }

    #[cfg(not(target_os = "windows"))]
    let mut cmd = opener(std::ffi::OsStr::new(&url));

    #[cfg(target_os = "windows")]
    let mut cmd = {
        // Do NOT route through `cmd /C start`: cmd re-parses its command line
        // and treats every `&` in the URL as a command separator, so an OAuth
        // sign-in URL like `.../login?desktop=1&port=65491&state=<nonce>` was
        // truncated at the first `&`, so the browser only ever received
        // `?desktop=1`, dropping the loopback port and the CSRF nonce. The
        // callback then reached the app with no `state`, and every desktop
        // sign-in failed with "auth callback state mismatch". rundll32 is not a
        // shell: it hands the URL to the registered protocol handler verbatim.
        let mut c = tokio::process::Command::new("rundll32.exe");
        c.args(["url.dll,FileProtocolHandler", &url]);
        c
    };

    cmd.spawn()
        .map(|_| ())
        .map_err(|e| format!("could not open browser: {e}"))
}

/// Show a folder in the system's file manager: where a version downloaded
/// from History landed.
#[tauri::command]
pub async fn open_folder(path: String) -> Result<(), String> {
    let path = std::path::PathBuf::from(path);
    // A folder and nothing else: an opener handed a file runs it.
    if !path.is_dir() {
        return Err(format!("{} isn't a folder", path.display()));
    }

    #[cfg(not(target_os = "windows"))]
    let mut cmd = opener(path.as_os_str());

    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = tokio::process::Command::new("explorer.exe");
        c.arg(&path);
        c
    };

    cmd.spawn()
        .map(|_| ())
        .map_err(|e| format!("could not open {}: {e}", path.display()))
}

/// The platform's opener for `target`, on Linux with what the AppImage did to
/// the environment undone (see [`open_external`]).
#[cfg(not(target_os = "windows"))]
fn opener(target: &std::ffi::OsStr) -> tokio::process::Command {
    use tokio::process::Command;

    #[cfg(target_os = "linux")]
    let cmd = {
        let mut c = Command::new("xdg-open");
        c.arg(target);
        // AppImage-injected loader/toolkit vars: restore the pre-AppImage value
        // if AppRun stashed it as `<VAR>_ORIG`, otherwise drop it entirely so
        // the child falls back to the host defaults.
        const POLLUTED: &[&str] = &[
            "LD_LIBRARY_PATH",
            "LD_PRELOAD",
            "GTK_PATH",
            "GDK_PIXBUF_MODULE_FILE",
            "GIO_MODULE_DIR",
            "GST_PLUGIN_SYSTEM_PATH",
            "GSETTINGS_SCHEMA_DIR",
        ];
        for var in POLLUTED {
            match std::env::var_os(format!("{var}_ORIG")) {
                Some(orig) => {
                    c.env(var, orig);
                }
                None => {
                    c.env_remove(var);
                }
            }
        }
        // The AppImage also rewrites `XDG_DATA_DIRS`, which decides which
        // `.desktop` file wins the `https` handler. Its GTK hook puts
        // `/usr/share` ahead of the Flatpak export dirs, and SteamOS ships a stub
        // `/usr/share/applications/firefox.desktop` that opens Discover to
        // install Firefox: with the Flatpak Firefox already installed, every
        // "Sign in" still landed in Discover. Hand the opener the session's order.
        if let Some(appdir) = std::env::var_os("APPDIR") {
            let dirs = std::env::var("XDG_DATA_DIRS").unwrap_or_default();
            match host_data_dirs(&dirs, std::path::Path::new(&appdir)) {
                Some(host) => {
                    c.env("XDG_DATA_DIRS", host);
                }
                None => {
                    c.env_remove("XDG_DATA_DIRS");
                }
            }
        }
        c
    };

    #[cfg(target_os = "macos")]
    let cmd = {
        let mut c = Command::new("open");
        c.arg(target);
        c
    };

    cmd
}

/// `XDG_DATA_DIRS` as the session had it before the AppImage launched us, or
/// `None` when it had none.
///
/// Nothing saves the original, but what the AppImage does to it is
/// reversible: `AppRun` prepends `$APPDIR/usr/share/` and the linuxdeploy GTK
/// hook prepends `$APPDIR/usr/share:/usr/share:`. Dropping the `$APPDIR` entries
/// and keeping each directory's last occurrence takes the injected `/usr/share`
/// back out while the session's own copy, at the end, stays where it was.
#[cfg(any(target_os = "linux", test))]
fn host_data_dirs(dirs: &str, appdir: &std::path::Path) -> Option<String> {
    let entries: Vec<&str> = dirs
        .split(':')
        .filter(|d| !d.is_empty() && !std::path::Path::new(d).starts_with(appdir))
        .collect();
    let mut host: Vec<&str> = Vec::new();
    for (i, dir) in entries.iter().enumerate() {
        if !entries[i + 1..].contains(dir) {
            host.push(dir);
        }
    }
    (!host.is_empty()).then(|| host.join(":"))
}

#[cfg(test)]
mod tests {
    use super::host_data_dirs;
    use std::path::Path;

    const APPDIR: &str = "/tmp/.mount_HoardAbc123";

    #[test]
    fn restores_the_steamos_session_order() {
        // What SteamOS's Plasma session has, with the Flatpak exports first.
        let session = "/home/deck/.local/share/flatpak/exports/share:\
                       /var/lib/flatpak/exports/share:/usr/local/share:/usr/share";
        // `AppRun` on top of the GTK hook on top of the session.
        let inside = format!("{APPDIR}/usr/share/:{APPDIR}/usr/share:/usr/share:{session}");
        assert_eq!(
            host_data_dirs(&inside, Path::new(APPDIR)).as_deref(),
            Some(session)
        );
    }

    #[test]
    fn no_session_value_means_none() {
        // The GTK hook over an unset variable leaves a trailing `:`.
        let inside = format!("{APPDIR}/usr/share/:{APPDIR}/usr/share:");
        assert_eq!(host_data_dirs(&inside, Path::new(APPDIR)), None);
    }

    #[test]
    fn a_sibling_of_appdir_is_not_ours() {
        let sibling = format!("{APPDIR}-other/usr/share");
        let inside = format!("{APPDIR}/usr/share:{sibling}:/usr/share");
        assert_eq!(
            host_data_dirs(&inside, Path::new(APPDIR)),
            Some(format!("{sibling}:/usr/share"))
        );
    }
}

/// One line from the interface into the app's log, for what only the webview can
/// see: today, which display language it picked and why. Capped, so a runaway
/// caller can't flood the file.
#[tauri::command]
pub fn ui_log(window: tauri::Window, topic: String, message: String) {
    let topic: String = topic.chars().take(32).collect();
    let message: String = message.chars().take(400).collect();
    tracing::info!(window = window.label(), topic = %topic, "ui: {message}");
}

/// A product event from the interface. Fire-and-forget: an unknown name is
/// dropped by the agent, and an error would only ever reach a console.
#[tauri::command]
pub fn track_event(name: String, props: Option<serde_json::Value>) {
    hoard_agent::telemetry::app_event(&name, props.unwrap_or_default());
}

/// How the running app got onto this machine, shown under About. With the .deb
/// and the Flatpak installed side by side nothing in the window told them apart:
/// same version, same look, and the one difference was a path nobody sees.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallKind {
    Flatpak,
    // The spelling `install::Delivery` uses; `snake_case` would say `app_image`.
    #[serde(rename = "appimage")]
    AppImage,
    Deb,
    Rpm,
    /// Under `/usr` and not ours: a distro's own package.
    System,
    Msi,
    Nsis,
    /// The Windows app as Hoard Setup leaves it, in `%LOCALAPPDATA%\hoard\app`.
    Setup,
    MacApp,
    /// Straight out of a cargo `target/` directory.
    Dev,
    Unknown,
}

#[derive(Debug, serde::Serialize)]
pub struct InstallChannel {
    kind: InstallKind,
    /// What tells two installs apart: the Flatpak's app id, the AppImage file
    /// (its binary lives on a mount that changes every start), or the path.
    detail: String,
}

#[tauri::command]
pub fn app_install_channel() -> InstallChannel {
    let exe = std::env::current_exe().unwrap_or_default();
    let kind = install_kind(&exe);
    let detail = match kind {
        InstallKind::Flatpak => std::env::var("FLATPAK_ID").ok(),
        InstallKind::AppImage => std::env::var("APPIMAGE").ok(),
        _ => None,
    }
    .unwrap_or_else(|| exe.display().to_string());
    InstallChannel { kind, detail }
}

// `cfg!` rather than `#[cfg]` so every variant is built on every platform and
// none trips `dead_code` on the CI's Windows and macOS runs.
fn install_kind(exe: &std::path::Path) -> InstallKind {
    if exe.components().any(|c| c.as_os_str() == "target") {
        return InstallKind::Dev;
    }
    if cfg!(target_os = "windows") {
        if hoard_agent::install::layout::app_path().is_ok_and(|app| app == exe) {
            return InstallKind::Setup;
        }
        // The NSIS installer leaves its uninstaller next to the binary; an MSI
        // is removed through Windows Installer and leaves nothing there.
        return if exe.with_file_name("uninstall.exe").is_file() {
            InstallKind::Nsis
        } else {
            InstallKind::Msi
        };
    }
    if cfg!(target_os = "macos") {
        return InstallKind::MacApp;
    }
    if hoard_agent::install::running_under_flatpak() {
        return InstallKind::Flatpak;
    }
    if std::env::var_os("APPIMAGE").is_some() {
        return InstallKind::AppImage;
    }
    if !exe.starts_with("/usr") {
        return InstallKind::Unknown;
    }
    // dpkg keeps a plain list of the paths each package installed, so asking it
    // is one file read and no subprocess. rpm's database is binary, and its
    // presence is the best cheap signal there.
    let exe_str = exe.to_string_lossy();
    if std::fs::read_to_string("/var/lib/dpkg/info/hoard.list")
        .is_ok_and(|list| list.lines().any(|l| l == exe_str))
    {
        return InstallKind::Deb;
    }
    if std::path::Path::new("/var/lib/rpm").is_dir()
        || std::path::Path::new("/usr/lib/sysimage/rpm").is_dir()
    {
        return InstallKind::Rpm;
    }
    InstallKind::System
}
