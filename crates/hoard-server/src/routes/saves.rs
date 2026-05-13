use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::routes::health::ServerState;

// ─── Request/Response types ─────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateSaveRequest {
    pub game_slug: String,
    pub label: Option<String>,
    pub local_path_hint: Option<String>,
    pub client_os: Option<String>,
    /// Optional metadata supplied by newer clients. When the server's `games`
    /// table doesn't know the slug yet (e.g. an older server seeded before
    /// the game existed in the Ludusavi manifest), we use these fields to
    /// upsert a stub games row instead of returning 422. Older clients omit
    /// the fields and still get the 422 — but new clients self-heal the
    /// catalog as they go.
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub steam_app_id: Option<i64>,
}

#[derive(Deserialize)]
pub struct PatchSaveRequest {
    pub label: Option<String>,
    pub local_path_hint: Option<String>,
    pub client_os: Option<String>,
}

#[derive(Serialize)]
pub struct SaveResponse {
    pub id: String,
    pub game_slug: String,
    pub label: String,
    pub local_path_hint: Option<String>,
    pub client_os: Option<String>,
    pub latest_version_num: i64,
    pub snapshot_count: i64,
    pub total_size_bytes: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Deserialize)]
pub struct ListQuery {
    pub game_slug: Option<String>,
}

// ─── Handlers ───────────────────────────────────────────────────────────────

pub async fn create(
    State(state): State<Arc<ServerState>>,
    Extension(user): Extension<AuthUser>,
    Json(body): Json<CreateSaveRequest>,
) -> Result<(StatusCode, Json<SaveResponse>), (StatusCode, Json<serde_json::Value>)> {
    // Validate game exists
    let game_count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) as cnt FROM games WHERE slug = ?",
        body.game_slug
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|_| internal_err())?;

    if game_count == 0 {
        // Self-heal path: if the client supplied a display_name (newer
        // desktops do; older CLI clients don't), insert a minimal games
        // stub. This unblocks tracking when the desktop's Ludusavi catalog
        // is fresher than the server's seed — the alternative is making
        // the user manually re-import the manifest on every server upgrade.
        if let Some(display) = body.display_name.as_deref().filter(|s| !s.is_empty()) {
            let display = display.to_string();
            let slug_for_insert = body.game_slug.clone();
            let res = sqlx::query!(
                "INSERT INTO games (slug, display_name, steam_app_id, imported_from)
                 VALUES (?, ?, ?, 'client-supplied')
                 ON CONFLICT(slug) DO NOTHING",
                slug_for_insert,
                display,
                body.steam_app_id,
            )
            .execute(&state.pool)
            .await;
            if let Err(e) = res {
                tracing::warn!(error = %e, slug = %body.game_slug,
                    "couldn't self-heal games row from client metadata");
                return Err((
                    StatusCode::UNPROCESSABLE_ENTITY,
                    Json(serde_json::json!({"error": "game not found"})),
                ));
            }
            tracing::info!(slug = %body.game_slug,
                "inserted client-supplied games row to unblock tracking");
        } else {
            return Err((
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(serde_json::json!({"error": "game not found"})),
            ));
        }
    }

    let label = body.label.unwrap_or_else(|| "default".to_string());
    let id = Uuid::new_v4().to_string();
    let user_id = user.user_id.to_string();

    sqlx::query!(
        "INSERT INTO saves (id, user_id, game_slug, label, local_path_hint, client_os)
         VALUES (?,?,?,?,?,?)",
        id,
        user_id,
        body.game_slug,
        label,
        body.local_path_hint,
        body.client_os
    )
    .execute(&state.pool)
    .await
    .map_err(|e| {
        if e.to_string().contains("UNIQUE constraint") {
            (
                StatusCode::CONFLICT,
                Json(serde_json::json!({
                    "error": "save with this game and label already exists"
                })),
            )
        } else {
            internal_err()
        }
    })?;

    // Create physical directory
    let save_dir = state
        .config
        .storage
        .data_dir
        .join("data")
        .join(&user_id)
        .join(&body.game_slug)
        .join(&label);
    tokio::fs::create_dir_all(&save_dir).await.ok();

    let row = fetch_save(&state.pool, &id, &user_id)
        .await
        .map_err(|_| internal_err())?
        .ok_or_else(internal_err)?;

    Ok((StatusCode::CREATED, Json(row)))
}

pub async fn list(
    State(state): State<Arc<ServerState>>,
    Extension(user): Extension<AuthUser>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<SaveResponse>>, StatusCode> {
    let user_id = user.user_id.to_string();

    let rows = if let Some(slug) = q.game_slug {
        sqlx::query!(
            r#"SELECT s.id, s.game_slug, s.label, s.local_path_hint, s.client_os,
                      s.latest_version_num, s.created_at, s.updated_at,
                      COALESCE(COUNT(sn.id), 0) as "snapshot_count: i64",
                      COALESCE(SUM(sn.total_size_bytes), 0) as "total_size_bytes: i64"
               FROM saves s
               LEFT JOIN snapshots sn ON sn.save_id = s.id AND sn.deleted_at IS NULL
               WHERE s.user_id = ? AND s.game_slug = ?
               GROUP BY s.id ORDER BY s.created_at"#,
            user_id,
            slug
        )
        .fetch_all(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(|r| SaveResponse {
            id: r.id,
            game_slug: r.game_slug,
            label: r.label,
            local_path_hint: r.local_path_hint,
            client_os: r.client_os,
            latest_version_num: r.latest_version_num,
            snapshot_count: r.snapshot_count.unwrap_or(0),
            total_size_bytes: r.total_size_bytes.unwrap_or(0),
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
        .collect()
    } else {
        sqlx::query!(
            r#"SELECT s.id, s.game_slug, s.label, s.local_path_hint, s.client_os,
                      s.latest_version_num, s.created_at, s.updated_at,
                      COALESCE(COUNT(sn.id), 0) as "snapshot_count: i64",
                      COALESCE(SUM(sn.total_size_bytes), 0) as "total_size_bytes: i64"
               FROM saves s
               LEFT JOIN snapshots sn ON sn.save_id = s.id AND sn.deleted_at IS NULL
               WHERE s.user_id = ?
               GROUP BY s.id ORDER BY s.created_at"#,
            user_id
        )
        .fetch_all(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(|r| SaveResponse {
            id: r.id,
            game_slug: r.game_slug,
            label: r.label,
            local_path_hint: r.local_path_hint,
            client_os: r.client_os,
            latest_version_num: r.latest_version_num,
            snapshot_count: r.snapshot_count.unwrap_or(0),
            total_size_bytes: r.total_size_bytes.unwrap_or(0),
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
        .collect()
    };

    Ok(Json(rows))
}

pub async fn get_one(
    State(state): State<Arc<ServerState>>,
    Extension(user): Extension<AuthUser>,
    Path(save_id): Path<String>,
) -> Result<Json<SaveResponse>, StatusCode> {
    let user_id = user.user_id.to_string();
    fetch_save(&state.pool, &save_id, &user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)
        .map(Json)
}

pub async fn patch(
    State(state): State<Arc<ServerState>>,
    Extension(user): Extension<AuthUser>,
    Path(save_id): Path<String>,
    Json(body): Json<PatchSaveRequest>,
) -> Result<Json<SaveResponse>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = user.user_id.to_string();

    // Verify ownership
    let count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM saves WHERE id=? AND user_id=?",
        save_id,
        user_id
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|_| internal_err())?;

    if count == 0 {
        return Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error":"not found"})),
        ));
    }

    if let Some(label) = &body.label {
        sqlx::query!("UPDATE saves SET label=? WHERE id=?", label, save_id)
            .execute(&state.pool)
            .await
            .map_err(|e| {
                if e.to_string().contains("UNIQUE") {
                    (
                        StatusCode::CONFLICT,
                        Json(serde_json::json!({"error":"label collision"})),
                    )
                } else {
                    internal_err()
                }
            })?;
    }
    if let Some(hint) = &body.local_path_hint {
        sqlx::query!(
            "UPDATE saves SET local_path_hint=? WHERE id=?",
            hint,
            save_id
        )
        .execute(&state.pool)
        .await
        .map_err(|_| internal_err())?;
    }
    if let Some(os) = &body.client_os {
        sqlx::query!("UPDATE saves SET client_os=? WHERE id=?", os, save_id)
            .execute(&state.pool)
            .await
            .map_err(|_| internal_err())?;
    }

    fetch_save(&state.pool, &save_id, &user_id)
        .await
        .map_err(|_| internal_err())?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error":"not found"})),
            )
        })
        .map(Json)
}

pub async fn delete(
    State(state): State<Arc<ServerState>>,
    Extension(user): Extension<AuthUser>,
    Path(save_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let user_id = user.user_id.to_string();

    let row = sqlx::query!(
        "SELECT game_slug, label FROM saves WHERE id=? AND user_id=?",
        save_id,
        user_id
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    sqlx::query!("DELETE FROM saves WHERE id=?", save_id)
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Remove physical directory
    let dir = state
        .config
        .storage
        .data_dir
        .join("data")
        .join(&user_id)
        .join(&row.game_slug)
        .join(&row.label);
    tokio::fs::remove_dir_all(&dir).await.ok();

    Ok(StatusCode::NO_CONTENT)
}

// ─── Helpers ────────────────────────────────────────────────────────────────

async fn fetch_save(
    pool: &sqlx::SqlitePool,
    save_id: &str,
    user_id: &str,
) -> Result<Option<SaveResponse>, sqlx::Error> {
    sqlx::query!(
        r#"SELECT s.id, s.game_slug, s.label, s.local_path_hint, s.client_os,
                  s.latest_version_num, s.created_at, s.updated_at,
                  COALESCE(COUNT(sn.id), 0) as "snapshot_count: i64",
                  COALESCE(SUM(sn.total_size_bytes), 0) as "total_size_bytes: i64"
           FROM saves s
           LEFT JOIN snapshots sn ON sn.save_id = s.id AND sn.deleted_at IS NULL
           WHERE s.id = ? AND s.user_id = ?
           GROUP BY s.id"#,
        save_id,
        user_id
    )
    .fetch_optional(pool)
    .await
    .map(|opt| {
        opt.map(|r| SaveResponse {
            id: r.id,
            game_slug: r.game_slug,
            label: r.label,
            local_path_hint: r.local_path_hint,
            client_os: r.client_os,
            latest_version_num: r.latest_version_num,
            snapshot_count: r.snapshot_count.unwrap_or(0),
            total_size_bytes: r.total_size_bytes.unwrap_or(0),
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
    })
}

fn internal_err() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({"error": "internal server error"})),
    )
}
