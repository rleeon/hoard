//! Native overlay process (`hoard-screen`) control.
//!
//! Generic process glue, nothing Pro-specific: spawn the overlay binary as a
//! Tauri **sidecar**, feed it newline-JSON layout on stdin, and run it once with
//! `--list-windows` to enumerate capturable windows. The scene JSON is built by
//! the Pro UI and forwarded here verbatim (an opaque `String`), so the overlay /
//! scene schema never lives in this public repo — only the compiled, Pro binary
//! supplied at bundle time. Without that binary (community build) every command
//! just errors and the Hoard Screen section stays gated.
//!
//! Sidecar config: `bundle.externalBin = ["hoard-screen"]` in
//! `tauri.pro.conf.json` + a `shell:allow-execute` scope for `hoard-screen`
//! in `capabilities/screen.json`. The platform-suffixed binary
//! (`hoard-screen-<target-triple>`) is dropped next to `tauri.conf.json` (the
//! src-tauri root) by the Pro build (or `scripts/local-link.sh` for local runs).
//! A bare name (no `binaries/` subdir) is required so the bundled sidecar — which
//! the bundler flattens next to the app exe — is found by `exe_dir.join(name)`.

use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;

/// The sidecar name, matching the `externalBin` entry and the capability scope.
///
/// Must be a bare basename (no `binaries/` prefix): the bundler flattens the
/// external binary next to the app exe (`/usr/bin/hoard-screen`, `Contents/MacOS`,
/// next to the `.exe`), while the shell plugin resolves a sidecar as
/// `exe_dir.join(name)`. A subdir prefix here makes runtime look for
/// `<exe_dir>/binaries/hoard-screen`, which doesn't exist → ENOENT on spawn.
const SIDECAR: &str = "hoard-screen";

/// Holds the running overlay child so later `screen_send` / `screen_close` reach
/// it. Managed once in `lib.rs`; cleared when the child exits or is closed.
#[derive(Default)]
pub struct ScreenProc(pub Mutex<Option<CommandChild>>);

/// Launch the overlay if it isn't already running. Idempotent.
#[tauri::command]
pub async fn screen_open(app: AppHandle, proc: State<'_, ScreenProc>) -> Result<(), String> {
    if proc.0.lock().unwrap().is_some() {
        return Ok(());
    }
    let (mut rx, child) = app
        .shell()
        .sidecar(SIDECAR)
        .map_err(|e| format!("sidecar {SIDECAR}: {e}"))?
        .spawn()
        .map_err(|e| format!("spawn {SIDECAR}: {e}"))?;

    // Pump the overlay's stdout/stderr to the log, and clear the stored handle
    // when it exits — so a crashed or self-closed overlay doesn't leave
    // `screen_open` thinking it's still up.
    let app2 = app.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(ev) = rx.recv().await {
            match ev {
                CommandEvent::Stderr(line) => {
                    tracing::info!(target: "hoard_screen", "{}", String::from_utf8_lossy(&line).trim_end());
                }
                CommandEvent::Stdout(line) => {
                    // The overlay emits newline-JSON back: `{"type":"mode",…}`
                    // when Ctrl+O/Esc flips the mode, `{"type":"scene",…}` after
                    // an in-overlay drag/resize. Forward verbatim to the UI so
                    // the editor stays in sync; the backend stays schema-blind.
                    let s = String::from_utf8_lossy(&line);
                    let s = s.trim();
                    if s.starts_with('{') {
                        let _ = app2.emit("screen://event", s.to_string());
                    } else if !s.is_empty() {
                        tracing::debug!(target: "hoard_screen", "{s}");
                    }
                }
                CommandEvent::Terminated(payload) => {
                    tracing::info!(target: "hoard_screen", code = ?payload.code, "overlay exited");
                    if let Some(state) = app2.try_state::<ScreenProc>() {
                        *state.0.lock().unwrap() = None;
                    }
                    break;
                }
                _ => {}
            }
        }
    });

    *proc.0.lock().unwrap() = Some(child);
    Ok(())
}

/// Forward one opaque JSON line to the overlay's stdin (e.g. `set_scene`,
/// `set_editor`, `quit`). The UI owns the message schema.
#[tauri::command]
pub async fn screen_send(proc: State<'_, ScreenProc>, line: String) -> Result<(), String> {
    let mut guard = proc.0.lock().unwrap();
    let Some(child) = guard.as_mut() else {
        return Err("overlay not running".into());
    };
    let mut bytes = line.into_bytes();
    bytes.push(b'\n');
    child.write(&bytes).map_err(|e| format!("write overlay stdin: {e}"))
}

/// Ask the overlay to quit and drop the handle.
#[tauri::command]
pub async fn screen_close(proc: State<'_, ScreenProc>) -> Result<(), String> {
    let child = proc.0.lock().unwrap().take();
    if let Some(mut child) = child {
        let _ = child.write(b"{\"type\":\"quit\"}\n");
        let _ = child.kill();
    }
    Ok(())
}

/// True while the overlay process is running.
#[tauri::command]
pub fn screen_is_open(proc: State<'_, ScreenProc>) -> bool {
    proc.0.lock().unwrap().is_some()
}

/// Enumerate capturable windows by running the sidecar with `--list-windows`.
/// Returns the raw JSON array string (`[{id,title,app,protected}, …]`) for the
/// Pro UI to parse; the backend stays agnostic to the shape.
#[tauri::command]
pub async fn screen_list_windows(app: AppHandle) -> Result<String, String> {
    let out = app
        .shell()
        .sidecar(SIDECAR)
        .map_err(|e| format!("sidecar {SIDECAR}: {e}"))?
        .args(["--list-windows"])
        .output()
        .await
        .map_err(|e| format!("run {SIDECAR} --list-windows: {e}"))?;
    if !out.status.success() {
        tracing::warn!(target: "hoard_screen", "{}", String::from_utf8_lossy(&out.stderr).trim_end());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Enumerate the physical monitors by running the sidecar with
/// `--list-monitors`. Returns the raw JSON array string
/// (`[{id,name,x,y,w,h,primary}, …]`) for the Pro UI's per-panel screen picker;
/// the backend stays agnostic to the shape. Empty array on platforms without
/// native enumeration (the UI then falls back to its own monitor list).
#[tauri::command]
pub async fn screen_list_monitors(app: AppHandle) -> Result<String, String> {
    let out = app
        .shell()
        .sidecar(SIDECAR)
        .map_err(|e| format!("sidecar {SIDECAR}: {e}"))?
        .args(["--list-monitors"])
        .output()
        .await
        .map_err(|e| format!("run {SIDECAR} --list-monitors: {e}"))?;
    if !out.status.success() {
        tracing::warn!(target: "hoard_screen", "{}", String::from_utf8_lossy(&out.stderr).trim_end());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}
