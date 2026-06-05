//! Steam cover-art cache.
//!
//! The UI shows each game's Steam capsule (`header.jpg`) in the Library and on
//! the Map. Fetching it from Steam's CDN on every paint adds network latency
//! and breaks offline, so we cache the bytes on disk under the app cache dir
//! and serve them from there. First sight of a given app id downloads once;
//! every subsequent call (this session or a later launch) reads the local
//! file. The frontend receives the raw bytes as an `ArrayBuffer` (via
//! `tauri::ipc::Response`) and wraps them in an object URL — no base64 bloat,
//! no canvas-tainting cross-origin draws.
//!
//! A missing app id, a 404, or being offline surfaces as an `Err`, which the
//! JS side catches and falls back to the initial-letter placeholder.

use tauri::ipc::Response;
use tauri::Manager;

/// Returns the JPEG bytes of a game's Steam header capsule, reading from the
/// on-disk cache when present and downloading + persisting on first miss.
#[tauri::command]
pub async fn cover_bytes(app: tauri::AppHandle, app_id: u32) -> Result<Response, String> {
    let dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| e.to_string())?
        .join("covers");
    let path = dir.join(format!("{app_id}.jpg"));

    // Fast path: already on disk.
    if let Ok(bytes) = tokio::fs::read(&path).await {
        if !bytes.is_empty() {
            return Ok(Response::new(bytes));
        }
    }

    // Miss: pull from Steam's public CDN, persist, and return.
    let url = format!("https://cdn.cloudflare.steamstatic.com/steam/apps/{app_id}/header.jpg");
    let resp = reqwest::get(&url).await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("steam cover {app_id}: {}", resp.status()));
    }
    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    // Best-effort write — a failed cache write just means we re-fetch next time.
    let _ = tokio::fs::create_dir_all(&dir).await;
    let _ = tokio::fs::write(&path, &bytes).await;
    Ok(Response::new(bytes.to_vec()))
}
