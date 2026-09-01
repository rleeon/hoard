//! Aggregates for the panel's own account view.
//!
//! Everything here is a rollup of tables the server already writes; nothing new
//! is collected. Two of those tables had never been read by anything:
//! `audit_log`, written on every snapshot create/delete/restore/prune since the
//! first migration, and `client_logs`, which clients upload and only the
//! retention sweep ever touched. The panel is the first reader either one has
//! had; see [`crate::routes::admin`] for the log side.
//!
//! The per-request cost is a handful of grouped scans over one user's rows. On
//! a self-hosted instance that is nothing; if it ever stops being nothing, the
//! fix is a cached rollup table, not a slimmer page.

use std::sync::Arc;

use axum::{
    extract::{Extension, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use time::{Duration as TimeDuration, OffsetDateTime};

use crate::auth::AuthUser;
use crate::routes::health::ServerState;

/// Days of history behind the activity strip and the playtime rollup.
const WINDOW_DAYS: i64 = 30;

#[derive(Serialize)]
pub struct Overview {
    pub account: Account,
    pub storage: Storage,
    pub counts: Counts,
    pub games: Vec<GameRow>,
    pub playtime: Playtime,
    pub server: ServerInfo,
}

#[derive(Serialize)]
pub struct Account {
    pub username: String,
    pub is_admin: bool,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct Storage {
    /// What the saves weigh laid end to end, before deduplication.
    pub logical_bytes: i64,
    /// What they actually occupy: one copy of each distinct blob or chunk.
    pub stored_bytes: i64,
    /// Soft-deleted versions still recoverable from trash.
    pub trash_bytes: i64,
    /// The number the quota is enforced against (`users.storage_used_bytes`).
    pub used_bytes: i64,
    pub quota_bytes: i64,
}

#[derive(Serialize)]
pub struct Counts {
    pub games: i64,
    pub saves: i64,
    pub versions: i64,
    pub trashed_versions: i64,
    pub files: i64,
    pub devices: i64,
    pub devices_online: i64,
}

#[derive(Serialize)]
pub struct GameRow {
    pub slug: String,
    pub display_name: String,
    pub saves: i64,
    pub versions: i64,
    pub bytes: i64,
    pub last_backup_at: Option<String>,
    pub playtime_secs: i64,
}

#[derive(Serialize)]
pub struct Playtime {
    pub window_days: i64,
    pub total_secs: i64,
    /// One entry per day in the window, oldest first, zero-filled. The panel
    /// draws it as a strip; zero-filling here keeps that logic out of eight
    /// translated front-ends' worth of edge cases.
    pub days: Vec<DayRow>,
}

#[derive(Serialize)]
pub struct DayRow {
    pub day: String,
    pub secs: i64,
}

#[derive(Serialize)]
pub struct ServerInfo {
    pub version: String,
    pub uptime_secs: u64,
    pub storage_backend: String,
    /// Surfaced because "where did my old versions go" is the question the
    /// retention policy answers, and until now the only place the answer lived
    /// was the operator's `config.toml`.
    pub snapshot_pruning: bool,
    pub data_saving: f64,
    pub trash_retention_days: u64,
    pub max_snapshot_size_mb: u64,
    pub max_versions: Option<i64>,
    pub max_manual_versions: Option<i64>,
}

#[derive(Serialize)]
pub struct ActivityRow {
    pub at: String,
    pub event: String,
    pub game_slug: Option<String>,
    pub label: Option<String>,
    pub version_num: Option<i64>,
    pub device_name: Option<String>,
    pub bytes: Option<i64>,
    /// Bytes that were not already stored: the honest cost of that version.
    pub new_bytes: Option<i64>,
}

#[derive(Deserialize)]
pub struct ActivityQuery {
    #[serde(default)]
    pub limit: Option<i64>,
}

/// `(slug, display_name, saves, versions, bytes, last_backup_at)`.
type GameRollup = (String, Option<String>, i64, i64, i64, Option<String>);

/// `(at, event, metadata, version_num, device_name, game_slug, label, bytes)`
/// as the join returns it, before it becomes an [`ActivityRow`].
type ActivityRecord = (
    String,
    String,
    Option<String>,
    Option<i64>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<i64>,
);

type ApiError = (StatusCode, Json<serde_json::Value>);

fn internal(e: sqlx::Error, what: &str) -> ApiError {
    tracing::error!(error = %e, "{what} failed");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({ "error": "internal" })),
    )
}

/// `GET /v1/me/overview`
pub async fn overview(
    State(state): State<Arc<ServerState>>,
    Extension(user): Extension<AuthUser>,
) -> Result<Json<Overview>, ApiError> {
    let uid = user.user_id.to_string();
    let pool = &state.pool;

    let acct: (String, i64, i64, Option<i64>, Option<i64>) = sqlx::query_as(
        "SELECT created_at, storage_used_bytes, storage_quota_bytes, max_versions, \
         max_manual_versions FROM users WHERE id = ?",
    )
    .bind(&uid)
    .fetch_one(pool)
    .await
    .map_err(|e| internal(e, "overview account"))?;

    let (logical_bytes, versions, files): (i64, i64, i64) = sqlx::query_as(
        "SELECT COALESCE(SUM(sn.total_size_bytes),0), COUNT(sn.id), \
                COALESCE(SUM(sn.file_count),0) \
         FROM snapshots sn JOIN saves s ON s.id = sn.save_id \
         WHERE s.user_id = ? AND sn.deleted_at IS NULL",
    )
    .bind(&uid)
    .fetch_one(pool)
    .await
    .map_err(|e| internal(e, "overview snapshots"))?;

    let (trash_bytes, trashed_versions): (i64, i64) = sqlx::query_as(
        "SELECT COALESCE(SUM(sn.total_size_bytes),0), COUNT(sn.id) \
         FROM snapshots sn JOIN saves s ON s.id = sn.save_id \
         WHERE s.user_id = ? AND sn.deleted_at IS NOT NULL",
    )
    .bind(&uid)
    .fetch_one(pool)
    .await
    .map_err(|e| internal(e, "overview trash"))?;

    // Blobs and chunks are two generations of the same idea living side by
    // side, so the physical footprint is the sum of both. A row with
    // refcount 0 is an orphan the cleanup sweep hasn't collected yet: it is
    // still occupying the disk, so it still counts here.
    let stored_bytes: i64 = sqlx::query_as::<_, (i64,)>(
        "SELECT (SELECT COALESCE(SUM(size_bytes),0) FROM blobs WHERE user_id = ?1) \
              + (SELECT COALESCE(SUM(size_bytes),0) FROM chunks WHERE user_id = ?1)",
    )
    .bind(&uid)
    .fetch_one(pool)
    .await
    .map_err(|e| internal(e, "overview stored bytes"))?
    .0;

    let (games_count, saves_count): (i64, i64) =
        sqlx::query_as("SELECT COUNT(DISTINCT game_slug), COUNT(*) FROM saves WHERE user_id = ?")
            .bind(&uid)
            .fetch_one(pool)
            .await
            .map_err(|e| internal(e, "overview save counts"))?;

    let (devices, devices_online): (i64, i64) = sqlx::query_as(
        "SELECT COUNT(*), \
                COALESCE(SUM(CASE WHEN closed_at IS NULL \
                     AND last_seen_at >= strftime('%Y-%m-%dT%H:%M:%SZ','now','-5 minutes') \
                     THEN 1 ELSE 0 END),0) \
         FROM devices WHERE user_id = ?",
    )
    .bind(&uid)
    .fetch_one(pool)
    .await
    .map_err(|e| internal(e, "overview devices"))?;

    let since = window_start();
    let play_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT game_slug, SUM(secs) FROM playtime \
         WHERE user_id = ? AND day >= ? GROUP BY game_slug",
    )
    .bind(&uid)
    .bind(&since)
    .fetch_all(pool)
    .await
    .map_err(|e| internal(e, "overview playtime by game"))?;

    let game_rows: Vec<GameRollup> = sqlx::query_as(
        "SELECT s.game_slug, g.display_name, COUNT(DISTINCT s.id), \
                COUNT(sn.id), COALESCE(SUM(sn.total_size_bytes),0), MAX(sn.created_at) \
         FROM saves s \
         LEFT JOIN games g ON g.slug = s.game_slug \
         LEFT JOIN snapshots sn ON sn.save_id = s.id AND sn.deleted_at IS NULL \
         WHERE s.user_id = ? \
         GROUP BY s.game_slug, g.display_name \
         ORDER BY 5 DESC, s.game_slug",
    )
    .bind(&uid)
    .fetch_all(pool)
    .await
    .map_err(|e| internal(e, "overview games"))?;

    let games = game_rows
        .into_iter()
        .map(|(slug, name, saves, versions, bytes, last)| {
            let playtime_secs = play_rows
                .iter()
                .find(|(g, _)| *g == slug)
                .map(|(_, s)| *s)
                .unwrap_or(0);
            GameRow {
                display_name: name.unwrap_or_else(|| slug.clone()),
                slug,
                saves,
                versions,
                bytes,
                last_backup_at: last,
                playtime_secs,
            }
        })
        .collect();

    let day_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT day, SUM(secs) FROM playtime WHERE user_id = ? AND day >= ? \
         GROUP BY day ORDER BY day",
    )
    .bind(&uid)
    .bind(&since)
    .fetch_all(pool)
    .await
    .map_err(|e| internal(e, "overview playtime by day"))?;

    Ok(Json(Overview {
        account: Account {
            username: user.username.clone(),
            is_admin: user.is_admin,
            created_at: acct.0,
        },
        storage: Storage {
            logical_bytes,
            stored_bytes,
            trash_bytes,
            used_bytes: acct.1,
            quota_bytes: acct.2,
        },
        counts: Counts {
            games: games_count,
            saves: saves_count,
            versions,
            trashed_versions,
            files,
            devices,
            devices_online,
        },
        games,
        playtime: Playtime {
            window_days: WINDOW_DAYS,
            total_secs: day_rows.iter().map(|(_, s)| *s).sum(),
            days: zero_filled(&day_rows),
        },
        server: ServerInfo {
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_secs: state.start_time.elapsed().as_secs(),
            storage_backend: match state.config.storage.backend {
                crate::config::StorageBackend::Local => "local".into(),
                crate::config::StorageBackend::S3 => "s3".into(),
            },
            snapshot_pruning: state.config.retention.snapshot_pruning,
            data_saving: state.config.retention.data_saving,
            trash_retention_days: state.config.retention.trash_retention_days,
            max_snapshot_size_mb: state.config.storage.max_snapshot_size_mb,
            max_versions: acct.3,
            max_manual_versions: acct.4,
        },
    }))
}

/// `GET /v1/me/activity?limit=`
///
/// The join to `snapshots` is a LEFT JOIN because an audit row outlives its
/// snapshot: a purge from trash deletes the row and the cascade takes the
/// version with it, but the record that it once existed is exactly what a
/// history is for.
pub async fn activity(
    State(state): State<Arc<ServerState>>,
    Extension(user): Extension<AuthUser>,
    Query(q): Query<ActivityQuery>,
) -> Result<Json<Vec<ActivityRow>>, ApiError> {
    let uid = user.user_id.to_string();
    let limit = q.limit.unwrap_or(60).clamp(1, 500);

    let rows: Vec<ActivityRecord> = sqlx::query_as(
        "SELECT a.created_at, a.event_type, a.metadata, sn.version_num, sn.device_name, \
                s.game_slug, s.label, sn.total_size_bytes \
         FROM audit_log a \
         LEFT JOIN snapshots sn ON sn.id = a.entity_id \
         LEFT JOIN saves s ON s.id = sn.save_id \
         WHERE a.user_id = ? \
         ORDER BY a.created_at DESC, a.rowid DESC LIMIT ?",
    )
    .bind(&uid)
    .bind(limit)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| internal(e, "activity"))?;

    let out = rows
        .into_iter()
        .map(|(at, event, meta, version, device, slug, label, bytes)| {
            // `snapshot.created` carries its own numbers, and they are better
            // than the joined ones: `new_bytes` (what deduplication actually
            // cost) exists nowhere else, and the version survives a purge that
            // took the snapshot row with it.
            let parsed: Option<serde_json::Value> =
                meta.as_deref().and_then(|m| serde_json::from_str(m).ok());
            let num = |k: &str| {
                parsed
                    .as_ref()
                    .and_then(|v| v.get(k))
                    .and_then(|v| v.as_i64())
            };
            ActivityRow {
                at,
                event,
                version_num: version.or_else(|| num("version_num")),
                bytes: bytes.or_else(|| num("bytes")),
                new_bytes: num("new_bytes"),
                device_name: device,
                game_slug: slug,
                label,
            }
        })
        .collect();

    Ok(Json(out))
}

fn window_start() -> String {
    let d = OffsetDateTime::now_utc() - TimeDuration::days(WINDOW_DAYS - 1);
    format!("{:04}-{:02}-{:02}", d.year(), d.month() as u8, d.day())
}

/// Turn the sparse `(day, secs)` rows into one entry per day in the window.
/// The dates come from the client's *local* day (that is how `playtime` is
/// keyed), so a row can legitimately sit a day either side of the UTC window;
/// anything outside is dropped rather than stretching the strip.
fn zero_filled(rows: &[(String, i64)]) -> Vec<DayRow> {
    let start = OffsetDateTime::now_utc() - TimeDuration::days(WINDOW_DAYS - 1);
    (0..WINDOW_DAYS)
        .map(|i| {
            let d = start + TimeDuration::days(i);
            let day = format!("{:04}-{:02}-{:02}", d.year(), d.month() as u8, d.day());
            let secs = rows
                .iter()
                .find(|(k, _)| *k == day)
                .map(|(_, v)| *v)
                .unwrap_or(0);
            DayRow { day, secs }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_is_dense_and_ends_today() {
        let today = {
            let d = OffsetDateTime::now_utc();
            format!("{:04}-{:02}-{:02}", d.year(), d.month() as u8, d.day())
        };
        let filled = zero_filled(&[(today.clone(), 42)]);
        assert_eq!(filled.len() as i64, WINDOW_DAYS);
        assert_eq!(filled.last().unwrap().day, today);
        assert_eq!(filled.last().unwrap().secs, 42);
        assert_eq!(filled.first().unwrap().secs, 0);
    }

    #[test]
    fn a_day_outside_the_window_is_dropped_not_stretched() {
        let filled = zero_filled(&[("1999-01-01".to_string(), 999)]);
        assert!(filled.iter().all(|d| d.secs == 0));
    }
}
