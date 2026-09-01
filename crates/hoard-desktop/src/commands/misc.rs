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

    use tokio::process::Command;

    #[cfg(target_os = "linux")]
    let mut cmd = {
        let mut c = Command::new("xdg-open");
        c.arg(&url);
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
        c
    };

    #[cfg(target_os = "macos")]
    let mut cmd = {
        let mut c = Command::new("open");
        c.arg(&url);
        c
    };

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
        let mut c = Command::new("rundll32.exe");
        c.args(["url.dll,FileProtocolHandler", &url]);
        c
    };

    cmd.spawn()
        .map(|_| ())
        .map_err(|e| format!("could not open browser: {e}"))
}
