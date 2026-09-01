use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::Json,
};
use hoard_core::wire::{CreateSaveRequest, PatchSaveRequest, Save};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::routes::health::ServerState;
use crate::routes::{parse_save_id, repair_slug, repair_ts};

// ─── Request/Response types ─────────────────────────────────────────────────
//
// The shapes live in `hoard_core::wire` (ADR 0021 C.6): the client compiles
// against the same ones, so drift between the two ends is a compile
// compilación en vez de un 422 en producción.

#[derive(Deserialize)]
pub struct ListQuery {
    pub game_slug: Option<String>,
}

// ─── Handlers ───────────────────────────────────────────────────────────────

pub async fn create(
    State(state): State<Arc<ServerState>>,
    Extension(user): Extension<AuthUser>,
    Json(body): Json<CreateSaveRequest>,
) -> Result<(StatusCode, Json<Save>), (StatusCode, Json<serde_json::Value>)> {
    // `game_slug` already went through `GameSlug`'s gate when the body was
    // deserialised, so all that is needed here is the `&str` for SQL and for the
    // disk paths.
    let slug_str = body.game_slug.as_str();

    // Validate game exists
    let game_count: i64 =
        sqlx::query_scalar!("SELECT COUNT(*) as cnt FROM games WHERE slug = ?", slug_str)
            .fetch_one(&state.pool)
            .await
            .map_err(|e| internal_logged("reading a row", e))?;

    if game_count == 0 {
        // Self-heal path: if the client supplied a display_name (newer
        // desktops do; older CLI clients don't), insert a minimal games
        // stub. This unblocks tracking when the desktop's Ludusavi catalog
        // is fresher than the server's seed; the alternative is making
        // the user manually re-import the manifest on every server upgrade.
        if let Some(display) = body.display_name.as_deref().filter(|s| !s.is_empty()) {
            let display = display.to_string();
            let slug_for_insert = slug_str.to_string();
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
        slug_str,
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
        .join(slug_str)
        .join(&label);
    tokio::fs::create_dir_all(&save_dir).await.ok();

    let row = fetch_save(&state.pool, &id, &user_id)
        .await
        .map_err(|e| internal_logged("database access", e))?
        .ok_or_else(internal_err)?;

    Ok((StatusCode::CREATED, Json(row)))
}

pub async fn list(
    State(state): State<Arc<ServerState>>,
    Extension(user): Extension<AuthUser>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<Save>>, StatusCode> {
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
        .map_err(|e| internal_logged_status("listing rows", e))?
        .into_iter()
        .filter_map(|r| {
            Some(Save {
                id: parse_save_id(&r.id)?,
                user_id: None,
                game_slug: repair_slug(&r.game_slug),
                label: r.label,
                local_path_hint: r.local_path_hint,
                client_os: r.client_os,
                latest_version_num: Some(r.latest_version_num),
                snapshot_count: Some(r.snapshot_count.unwrap_or(0)),
                total_size_bytes: Some(r.total_size_bytes.unwrap_or(0)),
                created_at: repair_ts(&r.created_at),
                updated_at: repair_ts(&r.updated_at),
            })
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
        .map_err(|e| internal_logged_status("listing rows", e))?
        .into_iter()
        .filter_map(|r| {
            Some(Save {
                id: parse_save_id(&r.id)?,
                user_id: None,
                game_slug: repair_slug(&r.game_slug),
                label: r.label,
                local_path_hint: r.local_path_hint,
                client_os: r.client_os,
                latest_version_num: Some(r.latest_version_num),
                snapshot_count: Some(r.snapshot_count.unwrap_or(0)),
                total_size_bytes: Some(r.total_size_bytes.unwrap_or(0)),
                created_at: repair_ts(&r.created_at),
                updated_at: repair_ts(&r.updated_at),
            })
        })
        .collect()
    };

    Ok(Json(rows))
}

pub async fn get_one(
    State(state): State<Arc<ServerState>>,
    Extension(user): Extension<AuthUser>,
    Path(save_id): Path<String>,
) -> Result<Json<Save>, StatusCode> {
    let user_id = user.user_id.to_string();
    fetch_save(&state.pool, &save_id, &user_id)
        .await
        .map_err(|e| internal_logged_status("database access", e))?
        .ok_or(StatusCode::NOT_FOUND)
        .map(Json)
}

pub async fn patch(
    State(state): State<Arc<ServerState>>,
    Extension(user): Extension<AuthUser>,
    Path(save_id): Path<String>,
    Json(body): Json<PatchSaveRequest>,
) -> Result<Json<Save>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = user.user_id.to_string();

    // Verify ownership and fetch the current label so we can rename the
    // physical snapshot directory atomically when the label changes.
    let current = sqlx::query!(
        "SELECT game_slug, label FROM saves WHERE id=? AND user_id=?",
        save_id,
        user_id
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| internal_logged("reading a row", e))?
    .ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error":"not found"})),
        )
    })?;

    // ---- Label rename ------------------------------------------------------
    // The label is part of the on-disk path (data/<user>/<game>/<label>/v*/),
    // so a rename has to move two pieces in lockstep: the `saves.label`
    // row and the directory itself. We do the UPDATE inside a transaction,
    // then rename the directory, then commit. If the rename fails we roll
    // back the UPDATE so the user sees a clean error instead of a save
    // whose old snapshots are unreachable.
    if let Some(new_label) = body
        .label
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if new_label != current.label {
            let mut tx = state
                .pool
                .begin()
                .await
                .map_err(|e| internal_logged("opening a transaction", e))?;

            sqlx::query!("UPDATE saves SET label=? WHERE id=?", new_label, save_id)
                .execute(&mut *tx)
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

            let old_dir = state
                .config
                .storage
                .data_dir
                .join("data")
                .join(&user_id)
                .join(&current.game_slug)
                .join(&current.label);
            let new_dir = state
                .config
                .storage
                .data_dir
                .join("data")
                .join(&user_id)
                .join(&current.game_slug)
                .join(new_label);

            if old_dir.exists() {
                // Reject if the target dir already exists: the UNIQUE
                // constraint above should make this unreachable but we
                // double-check rather than trust two writers can't race.
                if new_dir.exists() {
                    tx.rollback().await.ok();
                    return Err((
                        StatusCode::CONFLICT,
                        Json(serde_json::json!({"error":"target directory already exists"})),
                    ));
                }
                if let Some(parent) = new_dir.parent() {
                    tokio::fs::create_dir_all(parent).await.ok();
                }
                if let Err(e) = tokio::fs::rename(&old_dir, &new_dir).await {
                    tracing::warn!(error = %e, old = ?old_dir, new = ?new_dir,
                        "rename failed during save label patch; rolling back DB");
                    tx.rollback().await.ok();
                    return Err(internal_err());
                }
            }

            if let Err(e) = tx.commit().await {
                // Commit failed after the rename succeeded, so try to undo
                // the rename so the world stays consistent.
                tracing::warn!(error = %e, "commit failed after rename; reverting rename");
                tokio::fs::rename(&new_dir, &old_dir).await.ok();
                return Err(internal_err());
            }
        }
    }

    if let Some(hint) = &body.local_path_hint {
        sqlx::query!(
            "UPDATE saves SET local_path_hint=? WHERE id=?",
            hint,
            save_id
        )
        .execute(&state.pool)
        .await
        .map_err(|e| internal_logged("writing to the database", e))?;
    }
    if let Some(os) = &body.client_os {
        sqlx::query!("UPDATE saves SET client_os=? WHERE id=?", os, save_id)
            .execute(&state.pool)
            .await
            .map_err(|e| internal_logged("writing to the database", e))?;
    }

    fetch_save(&state.pool, &save_id, &user_id)
        .await
        .map_err(|e| internal_logged("writing to the database", e))?
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
    .map_err(|e| internal_logged_status("reading a row", e))?
    .ok_or(StatusCode::NOT_FOUND)?;

    sqlx::query!("DELETE FROM saves WHERE id=?", save_id)
        .execute(&state.pool)
        .await
        .map_err(|e| internal_logged_status("writing to the database", e))?;

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
) -> Result<Option<Save>, sqlx::Error> {
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
        opt.and_then(|r| {
            Some(Save {
                id: parse_save_id(&r.id)?,
                user_id: None,
                game_slug: repair_slug(&r.game_slug),
                label: r.label,
                local_path_hint: r.local_path_hint,
                client_os: r.client_os,
                latest_version_num: Some(r.latest_version_num),
                snapshot_count: Some(r.snapshot_count.unwrap_or(0)),
                total_size_bytes: Some(r.total_size_bytes.unwrap_or(0)),
                created_at: repair_ts(&r.created_at),
                updated_at: repair_ts(&r.updated_at),
            })
        })
    })
}

fn internal_err() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({"error": "internal server error"})),
    )
}

/// Same 500 the client already gets, but the cause reaches the log.
///
/// Companion to [`internal_err`]. Mapping a fallible call with a closure that
/// ignores its argument silently deletes the one fact an operator needs; see
/// the note on `snapshots.rs::internal_logged` for what that cost.
fn internal_logged<E: std::fmt::Display>(
    what: &'static str,
    e: E,
) -> (StatusCode, Json<serde_json::Value>) {
    tracing::error!(error = %e, step = what, "saves request failed");
    internal_err()
}

/// Status-only sibling of [`internal_logged`], for the handlers in this module
/// that answer with a bare [`StatusCode`] instead of a JSON body. Same job:
/// the client learns nothing new, the operator learns everything.
fn internal_logged_status<E: std::fmt::Display>(what: &'static str, e: E) -> StatusCode {
    tracing::error!(error = %e, step = what, "saves request failed");
    StatusCode::INTERNAL_SERVER_ERROR
}
