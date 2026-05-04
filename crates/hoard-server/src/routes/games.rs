use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::routes::health::ServerState;

#[derive(Deserialize)]
pub struct SearchQuery {
    search: Option<String>,
    #[serde(default = "default_limit")]
    limit: i64,
    #[serde(default)]
    offset: i64,
}

fn default_limit() -> i64 {
    20
}

#[derive(Serialize)]
pub struct GameResponse {
    slug: String,
    display_name: String,
    engine: Option<String>,
    save_paths_json: Option<String>,
}

pub async fn list(
    State(state): State<Arc<ServerState>>,
    Query(q): Query<SearchQuery>,
) -> Result<Json<Vec<GameResponse>>, StatusCode> {
    // We allow up to 1000 per page so the desktop client can sweep the full
    // catalog (~11k entries) in a handful of round-trips during auto-detection.
    // Smaller callers (the search bar) keep the default 20.
    let limit = q.limit.clamp(1, 1000);
    let offset = q.offset.max(0);

    let rows = if let Some(search) = q.search {
        let pattern = format!("%{}%", search);
        sqlx::query_as!(
            GameRow,
            "SELECT slug, display_name, engine, save_paths_json FROM games
             WHERE slug LIKE ? OR display_name LIKE ?
             ORDER BY slug LIMIT ? OFFSET ?",
            pattern,
            pattern,
            limit,
            offset
        )
        .fetch_all(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    } else {
        sqlx::query_as!(
            GameRow,
            "SELECT slug, display_name, engine, save_paths_json FROM games
             ORDER BY slug LIMIT ? OFFSET ?",
            limit,
            offset
        )
        .fetch_all(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    };

    Ok(Json(
        rows.into_iter()
            .map(|r| GameResponse {
                slug: r.slug,
                display_name: r.display_name,
                engine: r.engine,
                save_paths_json: r.save_paths_json,
            })
            .collect(),
    ))
}

pub async fn get_one(
    State(state): State<Arc<ServerState>>,
    Path(slug): Path<String>,
) -> Result<Json<GameResponse>, StatusCode> {
    let row = sqlx::query_as!(
        GameRow,
        "SELECT slug, display_name, engine, save_paths_json FROM games WHERE slug = ?",
        slug
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(GameResponse {
        slug: row.slug,
        display_name: row.display_name,
        engine: row.engine,
        save_paths_json: row.save_paths_json,
    }))
}

#[derive(Serialize)]
pub struct KnownPathsResponse {
    pub slug: String,
    pub display_name: String,
    pub steam_app_id: Option<i64>,
    pub cloud_steam: bool,
    pub cloud_gog: bool,
    pub manifest_version: Option<String>,
    /// Parsed JSON from `save_paths_json`. We re-emit it as a value so
    /// clients receive structured data, not an opaque escaped string.
    pub paths: serde_json::Value,
}

/// Return the structured save-path information for one game, ready for the
/// client to expand against the local filesystem.
pub async fn known_paths(
    State(state): State<Arc<ServerState>>,
    Path(slug): Path<String>,
) -> Result<Json<KnownPathsResponse>, StatusCode> {
    let row = sqlx::query_as!(
        KnownPathsRow,
        "SELECT slug, display_name, save_paths_json,
                steam_app_id, cloud_steam, cloud_gog, manifest_version
         FROM games WHERE slug = ?",
        slug
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    let paths: serde_json::Value = row
        .save_paths_json
        .as_deref()
        .map(|s| serde_json::from_str(s).unwrap_or(serde_json::Value::Null))
        .unwrap_or(serde_json::Value::Null);

    Ok(Json(KnownPathsResponse {
        slug: row.slug,
        display_name: row.display_name,
        steam_app_id: row.steam_app_id,
        cloud_steam: row.cloud_steam != 0,
        cloud_gog: row.cloud_gog != 0,
        manifest_version: row.manifest_version,
        paths,
    }))
}

#[derive(Serialize)]
pub struct ManifestVersionResponse {
    pub source: Option<String>,
    pub manifest_version: Option<String>,
    pub imported_at: Option<String>,
    pub games_inserted: i64,
    pub games_updated: i64,
    pub games_pruned: i64,
}

/// Most recent manifest import so the client can show "catalog as of <date>".
/// Returns 404 if no import has ever happened (fresh server).
pub async fn manifest_version(
    State(state): State<Arc<ServerState>>,
) -> Result<Json<ManifestVersionResponse>, StatusCode> {
    let row = sqlx::query_as!(
        ManifestVersionRow,
        "SELECT source, manifest_version, imported_at,
                games_inserted, games_updated, games_pruned
         FROM manifest_imports ORDER BY id DESC LIMIT 1",
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(ManifestVersionResponse {
        source: row.source,
        manifest_version: row.manifest_version,
        imported_at: row.imported_at,
        games_inserted: row.games_inserted,
        games_updated: row.games_updated,
        games_pruned: row.games_pruned,
    }))
}

struct GameRow {
    slug: String,
    display_name: String,
    engine: Option<String>,
    save_paths_json: Option<String>,
}

struct KnownPathsRow {
    slug: String,
    display_name: String,
    save_paths_json: Option<String>,
    steam_app_id: Option<i64>,
    cloud_steam: i64,
    cloud_gog: i64,
    manifest_version: Option<String>,
}

struct ManifestVersionRow {
    source: Option<String>,
    manifest_version: Option<String>,
    imported_at: Option<String>,
    games_inserted: i64,
    games_updated: i64,
    games_pruned: i64,
}
