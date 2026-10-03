//! Hoard-help: the form's commands. What gets sent and how is in
//! `hoard_agent::feedback`; this resolves the session, relays progress to the
//! page and turns errors into its i18n keys.

use std::path::PathBuf;
use std::sync::Arc;

use base64::Engine;
use hoard_agent::feedback::{Draft, FeedbackError};
use hoard_core::wire::FeedbackKind;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Debug, Deserialize)]
pub struct FeedbackInput {
    kind: FeedbackKind,
    message: String,
    contact: Option<String>,
    mode: Option<String>,
    files: Vec<String>,
    attach_logs: bool,
}

#[derive(Debug, Clone, Serialize)]
struct Progress {
    sent: u64,
    total: u64,
}

#[derive(Debug, Serialize)]
pub struct PickedFile {
    path: String,
    name: String,
    size: u64,
}

/// Screenshots pasted into the form, written out so they travel like any other
/// file. Emptied after a send.
fn pasted_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_cache_dir()
        .map_err(|e| e.to_string())?
        .join("hoard-help"))
}

#[tauri::command]
pub async fn feedback_send(app: AppHandle, input: FeedbackInput) -> Result<String, String> {
    // Signed in to Cloud: the report carries the account. Anything else, even a
    // session that will not refresh, goes anonymous rather than not at all.
    let creds = crate::commands::cloud::active_creds(&app)
        .await
        .ok()
        .flatten();
    let base = creds
        .as_ref()
        .map(|c| c.server_url.clone())
        .unwrap_or_else(hoard_agent::cloud_auth::cloud_base_url);

    let draft = Draft {
        kind: input.kind,
        message: input.message,
        contact: input.contact,
        app_version: Some(app.package_info().version.to_string()),
        mode: input.mode,
        files: input.files.into_iter().map(PathBuf::from).collect(),
        attach_logs: input.attach_logs,
    };
    let emitter = app.clone();
    let progress = Arc::new(move |sent, total| {
        let _ = emitter.emit("feedback://progress", Progress { sent, total });
    });
    let id = hoard_agent::feedback::send(
        &base,
        creds.as_ref().map(|c| c.access_token.as_str()),
        draft,
        progress,
    )
    .await
    .map_err(to_i18n)?;

    if let Ok(dir) = pasted_dir(&app) {
        let _ = tokio::fs::remove_dir_all(dir).await;
    }
    Ok(id)
}

fn to_i18n(e: FeedbackError) -> String {
    match e {
        FeedbackError::Empty => "i18n:help.err_empty".into(),
        FeedbackError::MessageTooLong => "i18n:help.err_message_long".into(),
        FeedbackError::TooManyFiles => "i18n:help.err_too_many".into(),
        FeedbackError::FileTooLarge(_) => "i18n:help.err_file_large".into(),
        FeedbackError::TooLarge => "i18n:help.err_total_large".into(),
        FeedbackError::Unreadable(_) => "i18n:help.err_unreadable".into(),
        FeedbackError::Throttled(_) => "i18n:help.err_throttled".into(),
        FeedbackError::Network(_) => "i18n:help.err_network".into(),
        e @ FeedbackError::Http { .. } => e.to_string(),
    }
}

/// Name and size of what the person picked or dropped, for the list under the
/// form. Folders and unreadable paths are left out.
#[tauri::command]
pub fn feedback_describe_files(paths: Vec<String>) -> Vec<PickedFile> {
    paths
        .into_iter()
        .filter_map(|p| {
            let meta = std::fs::metadata(&p).ok().filter(|m| m.is_file())?;
            let name = std::path::Path::new(&p)
                .file_name()?
                .to_string_lossy()
                .into_owned();
            Some(PickedFile {
                path: p,
                name,
                size: meta.len(),
            })
        })
        .collect()
}

/// A pasted screenshot, as the PNG the webview read off the clipboard.
#[tauri::command]
pub async fn feedback_stash_image(
    app: AppHandle,
    name: String,
    png_base64: String,
) -> Result<PickedFile, String> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(png_base64.trim())
        .map_err(|e| e.to_string())?;
    let dir = pasted_dir(&app)?;
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| e.to_string())?;
    let safe: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        .collect();
    let path = dir.join(if safe.is_empty() {
        "screenshot.png".into()
    } else {
        safe
    });
    tokio::fs::write(&path, &bytes)
        .await
        .map_err(|e| e.to_string())?;
    Ok(PickedFile {
        name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        path: path.to_string_lossy().into_owned(),
        size: bytes.len() as u64,
    })
}

/// Exactly what "include logs" will attach, for the person to read first.
#[tauri::command]
pub async fn feedback_logs_preview() -> String {
    hoard_agent::feedback::logs_excerpt().await
}
