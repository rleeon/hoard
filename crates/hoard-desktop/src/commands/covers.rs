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

    // Miss. Try the legacy capsule path first — present for the vast majority
    // of (older) apps and served straight from the CDN.
    let legacy = format!("https://cdn.cloudflare.steamstatic.com/steam/apps/{app_id}/header.jpg");
    let bytes = match fetch_image(&legacy).await {
        Some(b) => b,
        // Newer / unreleased games (e.g. Europa Universalis V) only publish
        // assets under a hashed `store_item_assets` path, where the legacy URL
        // 404s. Ask Steam's appdetails API for the real header image and fetch
        // that instead.
        None => {
            let url = appdetails_header_url(app_id)
                .await
                .ok_or_else(|| format!("steam cover {app_id}: no header image"))?;
            fetch_image(&url)
                .await
                .ok_or_else(|| format!("steam cover {app_id}: header fetch failed"))?
        }
    };

    // Best-effort write — a failed cache write just means we re-fetch next time.
    let _ = tokio::fs::create_dir_all(&dir).await;
    let _ = tokio::fs::write(&path, &bytes).await;
    Ok(Response::new(bytes))
}

/// GET an image URL, returning its bytes on a 2xx with a non-empty body.
async fn fetch_image(url: &str) -> Option<Vec<u8>> {
    let resp = reqwest::get(url).await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let bytes = resp.bytes().await.ok()?;
    if bytes.is_empty() {
        None
    } else {
        Some(bytes.to_vec())
    }
}

/// Resolve a game's real header-image URL via Steam's appdetails API. Covers
/// games whose store assets live only under the hashed `store_item_assets`
/// path, for which the legacy `apps/<id>/header.jpg` returns 404.
async fn appdetails_header_url(app_id: u32) -> Option<String> {
    let id = app_id.to_string();
    let resp = reqwest::Client::new()
        .get("https://store.steampowered.com/api/appdetails")
        .query(&[("appids", id.as_str()), ("filters", "basic"), ("l", "english")])
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let json: serde_json::Value = resp.json().await.ok()?;
    json.get(&id)?
        .get("data")?
        .get("header_image")?
        .as_str()
        .map(|s| s.to_string())
}

/// Resolve a game slug to its Steam app id so the UI can fetch a cover.
///
/// Covers depend on a Steam app id, but a save tracked on another device
/// arrives here with only its `game_slug` — this machine never detected it, so
/// the local detection report has no id for it. Two layered sources, cheapest
/// first:
///   1. The embedded Ludusavi catalog, keyed by the exact slug (offline,
///      instant). Resolves the long tail of catalogued games (Victoria 3,
///      Europa Universalis, …).
///   2. Steam's store search, queried with the de-slugified name. This catches
///      games Ludusavi doesn't list at all (e.g. Rust, which has no documented
///      save path) but that still exist on Steam. Best-effort and network-bound;
///      the JS side memoises the answer for the session so it runs at most once
///      per slug.
///
/// Returns `None` when neither source knows the game; the UI keeps the
/// initial-letter tile.
#[tauri::command]
pub async fn steam_app_id_for_slug(slug: String) -> Option<u32> {
    if let Some(id) = hoard_manifest::ludusavi::find_by_slug(&slug).and_then(|e| e.steam_app_id) {
        return Some(id as u32);
    }
    steam_store_search_app_id(&deslugify(&slug)).await
}

/// Turn a slug back into a search term: `europa-universalis-v` → `europa
/// universalis v`. Good enough for Steam's fuzzy, case-insensitive search.
fn deslugify(slug: &str) -> String {
    slug.split(['-', '_'])
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Ask Steam's public store search for the app id of the best match for
/// `term`. Returns the top-ranked result's id (Steam orders by relevance, so
/// the canonical game wins over demos/soundtracks). Any failure — offline, a
/// non-200, an empty result set — resolves to `None`.
async fn steam_store_search_app_id(term: &str) -> Option<u32> {
    if term.is_empty() {
        return None;
    }
    let resp = reqwest::Client::new()
        .get("https://store.steampowered.com/api/storesearch/")
        .query(&[("term", term), ("l", "english"), ("cc", "us")])
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let json: serde_json::Value = resp.json().await.ok()?;
    let first = json.get("items")?.as_array()?.first()?;
    first.get("id")?.as_u64().map(|v| v as u32)
}
