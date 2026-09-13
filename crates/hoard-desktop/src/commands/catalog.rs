//! Catalog (Ludusavi) commands.
//!
//! The service keeps the catalogue (ADR 0021, Slice 8): it refreshes it once a
//! day and answers what the window asks, so the window never loads it, and a
//! machine whose app stays closed for weeks still gets a fresh one. Against an
//! older service the window does it itself, through the same
//! [`hoard_agent::catalog`] functions.
//!
//! - `update_catalog()`: the "Check for updates" button. The stages arrive as
//!   `catalog://update-progress` (`downloading`, `parsing`, `saving`, `done`),
//!   from the service's notes or from here.
//! - `catalog_status()`: "20,731 games · updated 3 days ago" in Settings.
//! - `auto_update_catalog_in_background()`: the hourly check, for an older
//!   service. A loop rather than the old launch-time one-shot because the app
//!   lives in the tray for weeks: a check only at startup meant the catalog could
//!   silently go stale.

use std::time::Duration;

use hoard_agent::catalog::{self, CatalogStatus, CatalogUpdate};
use hoard_core::ipc::{GameFacts, Payload, Request};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::state::AppState;

/// How often the fallback loop re-checks staleness. Checking is a local metadata
/// read; the 17 MB download only happens once the copy is a day old.
const RECHECK_EVERY: Duration = Duration::from_secs(60 * 60);

/// A 17 MB download and its conversion, on a slow line.
const REFRESH_LIMIT: Duration = Duration::from_secs(5 * 60);

/// Downloads the upstream catalogue and stores it as the runtime override. The
/// next start of whoever reads it picks it up.
#[tauri::command]
pub async fn update_catalog(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<CatalogUpdate, String> {
    if state.daemon.owns_detection().await {
        let payload = state
            .daemon
            .request_long(Request::CatalogRefresh, REFRESH_LIMIT)
            .await
            .map_err(|e| format!("{e:#}"))?;
        return match payload {
            Payload::Catalog(info) => Ok(CatalogUpdate {
                games: info.games as usize,
                updated_at: info.updated_at.unwrap_or_default(),
                size_bytes: info.size_bytes.unwrap_or_default(),
                // Diagnostics only, and the file is the service's.
                path: String::new(),
            }),
            other => Err(format!("unexpected answer to a catalogue refresh: {other:?}")),
        };
    }
    catalog::refresh(|stage| emit_stage(&app, stage))
        .await
        .map_err(|e| e.to_string())
}

fn emit_stage(app: &AppHandle, stage: &str) {
    let _ = app.emit("catalog://update-progress", stage);
}

#[tauri::command]
pub async fn catalog_status(state: State<'_, AppState>) -> Result<CatalogStatus, String> {
    if !state.daemon.owns_detection().await {
        return Ok(catalog::status());
    }
    match state
        .daemon
        .request(Request::CatalogStatus)
        .await
        .map_err(|e| format!("{e:#}"))?
    {
        Payload::Catalog(info) => Ok(CatalogStatus {
            games: info.games as usize,
            has_runtime_override: info.has_runtime_override,
            updated_at: info.updated_at,
        }),
        other => Err(format!("unexpected answer to catalogue status: {other:?}")),
    }
}

/// What the catalogue says about one game: the Steam app id for its cover, the
/// shields for a restore. The service answers, since it has the catalogue loaded
/// for its own backups.
///
/// It is read here against an older service, and also when the service can't
/// answer. Guessing is worse than the memory: a cover resolved without the id gets
/// the wrong art cached on disk, and a restore without its shields holds back real
/// saves that look like config (`.ini` is the save pattern of 582 templates).
pub async fn game_facts(state: &AppState, slug: &str) -> GameFacts {
    if state.daemon.owns_detection().await {
        match state
            .daemon
            .request(Request::GameFacts {
                slug: slug.to_string(),
            })
            .await
        {
            Ok(Payload::GameFacts(facts)) => return facts,
            Ok(other) => tracing::warn!(?other, "unexpected answer to game facts"),
            Err(e) => tracing::warn!(
                error = %format!("{e:#}"),
                slug,
                "couldn't ask the service about a game; reading the catalogue here"
            ),
        }
    }
    let slug = slug.to_string();
    tokio::task::spawn_blocking(move || catalog::facts(&slug))
        .await
        .unwrap_or_default()
}

/// The hourly refresh, spawned from `setup()`, for when the service does not keep
/// the catalogue. Errors are logged and swallowed: the app keeps running on the
/// catalogue it has, and the next tick retries.
pub fn auto_update_catalog_in_background(app: AppHandle) {
    // `tauri::async_runtime::spawn` rather than `tokio::spawn`: this is
    // called from `setup()` which runs *before* Tauri enters its event
    // loop, so there is no ambient Tokio runtime yet. Tauri provides its
    // own multi-thread runtime that's always available.
    tauri::async_runtime::spawn(async move {
        loop {
            let here = !app.state::<AppState>().daemon.owns_detection().await;
            if here && catalog::is_stale() {
                tracing::info!("Ludusavi catalog stale; running background refresh");
                match catalog::refresh(|stage| emit_stage(&app, stage)).await {
                    Ok(result) => tracing::info!(games = result.games, "auto-refresh complete"),
                    Err(e) => {
                        tracing::warn!(error = %e, "auto-refresh failed; using embedded catalog")
                    }
                }
            }
            tokio::time::sleep(RECHECK_EVERY).await;
        }
    });
}
