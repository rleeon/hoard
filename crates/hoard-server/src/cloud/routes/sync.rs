//! `/v1/cloud/sync`: manifest endpoint. Clients call this on startup and
//! periodically to learn which versions exist on the server side.

use crate::cloud::auth::CloudUser;
use crate::cloud::errors::CloudError;
use crate::cloud::state::CloudState;
use axum::{extract::State, response::Json, Extension};
use serde::Serialize;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ManifestEntry {
    pub save_id: String,
    pub game_slug: String,
    pub label: String,
    pub latest_version_num: i64,
    /// Parent of the latest version (`None` = root). Lets a syncing device
    /// see the DAG edge without a per-version round-trip.
    pub latest_parent_version: Option<i64>,
    pub latest_size_bytes: i64,
    /// Files inside the latest version's archive (0 = unknown, e.g. uploaded
    /// by a client predating file-count reporting). Lets the History view show
    /// "N archivos" for cloud saves.
    pub latest_file_count: i64,
    pub latest_sha256: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct Manifest {
    pub generated_at: String,
    pub saves: Vec<ManifestEntry>,
}

pub async fn manifest(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
) -> Result<Json<Manifest>, CloudError> {
    let saves = saves_for(&state.pool, user.user_id).await?;
    Ok(Json(Manifest {
        generated_at: OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default(),
        saves,
    }))
}

// ---- poll cache
//
// Every installed client polls this route once a minute, and almost every
// answer is the list it got the minute before. Sending that list whole each
// time came to ~200 MB a day out of the Supabase pooler (61.7k polls a day,
// 16 rows each, ~200 B a row), on its own more than the free plan's 5 GB of
// monthly egress, which the org overran (101%) on 2026-09-13. So the database
// first answers with a fingerprint of exactly the rows the list would carry,
// one short row, and the list itself only travels when the fingerprint moves.
//
// Nothing has to remember to invalidate this: the fingerprint is recomputed
// from the rows on every poll. The list is read after the fingerprint, so a
// cached pair can hold rows newer than its fingerprint (one wasted read on the
// next poll) but never older ones.

/// Backstop in case the fingerprint ever misses a change: one full read per
/// user per half hour, whatever it says.
const MAX_CACHE_AGE: Duration = Duration::from_secs(30 * 60);

struct Cached {
    fingerprint: String,
    fetched_at: Instant,
    saves: Vec<ManifestEntry>,
}

/// Per process, which is per machine: the cloud runs on exactly one.
static CACHE: LazyLock<Mutex<HashMap<Uuid, Cached>>> = LazyLock::new(Default::default);

// `backup_only` saves are hidden from the manifest pull: other devices won't
// see them, so the agent won't auto-restore the file. The save is still
// uploadable and downloadable through the explicit per-id endpoints; that's
// the data-saver toggle.
const SCOPE: &str = "
      FROM saves s
 LEFT JOIN save_versions sv
        ON sv.save_id = s.id AND sv.version_num = s.latest_version_num
     WHERE s.user_id = $1 AND s.backup_only = false
       AND s.archived_at IS NULL";

/// The list this route serves, from the cache when nothing has changed.
pub async fn saves_for(pool: &PgPool, user_id: Uuid) -> Result<Vec<ManifestEntry>, CloudError> {
    // A fingerprint that fails costs the saving, never the answer.
    let fingerprint = match fingerprint(pool, user_id).await {
        Ok(fp) => Some(fp),
        Err(e) => {
            tracing::warn!(error = %e, "sync fingerprint failed, sending the full list");
            None
        }
    };
    if let Some(fp) = &fingerprint {
        let cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(c) = cache.get(&user_id) {
            if c.fingerprint == *fp && c.fetched_at.elapsed() < MAX_CACHE_AGE {
                return Ok(c.saves.clone());
            }
        }
    }

    let saves = load(pool, user_id).await?;
    if let Some(fp) = fingerprint {
        let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
        // Users who stopped polling fall out, so the map holds who is online
        // rather than everyone ever seen.
        cache.retain(|_, c| c.fetched_at.elapsed() < MAX_CACHE_AGE);
        cache.insert(
            user_id,
            Cached {
                fingerprint: fp,
                fetched_at: Instant::now(),
                saves: saves.clone(),
            },
        );
    }
    Ok(saves)
}

/// One short row standing for the whole list: every column it carries, row by
/// row. A row value cast to text quotes anything holding a comma, a quote or a
/// paren and leaves NULL empty, so two different lists never print the same.
pub async fn fingerprint(pool: &PgPool, user_id: Uuid) -> Result<String, sqlx::Error> {
    let sql = format!(
        "SELECT COALESCE(md5(string_agg(
                    (s.id, s.game_slug, s.label, s.latest_version_num, s.updated_at,
                     sv.size_bytes, sv.sha256, sv.parent_version, sv.file_count)::text,
                    ',' ORDER BY s.id)), '')
         {SCOPE}"
    );
    sqlx::query_scalar(&sql).bind(user_id).fetch_one(pool).await
}

#[doc(hidden)]
pub fn cached_fingerprint(user_id: Uuid) -> Option<String> {
    CACHE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&user_id)
        .map(|c| c.fingerprint.clone())
}

async fn load(pool: &PgPool, user_id: Uuid) -> Result<Vec<ManifestEntry>, CloudError> {
    let sql = format!(
        "SELECT s.id, s.game_slug, s.label, s.latest_version_num, s.updated_at,
                sv.size_bytes, sv.sha256, sv.parent_version, sv.file_count
         {SCOPE}
         ORDER BY s.updated_at DESC"
    );
    let rows: Vec<(
        String,
        String,
        String,
        i64,
        OffsetDateTime,
        Option<i64>,
        Option<String>,
        Option<i64>,
        Option<i64>,
    )> = sqlx::query_as(&sql).bind(user_id).fetch_all(pool).await?;

    Ok(rows
        .into_iter()
        .map(
            |(id, slug, label, ver, updated, size, sha, parent, file_count)| ManifestEntry {
                save_id: id,
                game_slug: slug,
                label,
                latest_version_num: ver,
                latest_parent_version: parent,
                latest_size_bytes: size.unwrap_or(0),
                latest_file_count: file_count.unwrap_or(0),
                latest_sha256: sha.unwrap_or_default(),
                updated_at: updated
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap_or_default(),
            },
        )
        .collect())
}
