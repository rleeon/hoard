//! `/v1/cloud/sync` — manifest endpoint. Clients call this on startup and
//! periodically to learn which versions exist on the server side.

use crate::cloud::auth::CloudUser;
use crate::cloud::errors::CloudError;
use crate::cloud::state::CloudState;
use axum::{extract::State, response::Json, Extension};
use serde::Serialize;
use time::OffsetDateTime;

#[derive(Debug, Serialize)]
pub struct ManifestEntry {
    pub save_id: String,
    pub game_slug: String,
    pub label: String,
    pub latest_version_num: i64,
    /// Parent of the latest version (`None` = root). Lets a syncing device
    /// see the DAG edge without a per-version round-trip.
    pub latest_parent_version: Option<i64>,
    pub latest_size_bytes: i64,
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
    // `backup_only` saves are hidden from the manifest pull: other devices
    // won't see them, so the agent won't auto-restore the file. The save
    // is still uploadable and downloadable through the explicit per-id
    // endpoints — that's the "modo ahorro" toggle.
    let rows: Vec<(
        String,
        String,
        String,
        i64,
        OffsetDateTime,
        Option<i64>,
        Option<String>,
        Option<i64>,
    )> = sqlx::query_as(
        r#"
            SELECT s.id, s.game_slug, s.label, s.latest_version_num, s.updated_at,
                   sv.size_bytes, sv.sha256, sv.parent_version
              FROM saves s
         LEFT JOIN save_versions sv
                ON sv.save_id = s.id AND sv.version_num = s.latest_version_num
             WHERE s.user_id = $1 AND s.backup_only = false
          ORDER BY s.updated_at DESC
            "#,
    )
    .bind(user.user_id)
    .fetch_all(&state.pool)
    .await?;

    let saves = rows
        .into_iter()
        .map(
            |(id, slug, label, ver, updated, size, sha, parent)| ManifestEntry {
                save_id: id,
                game_slug: slug,
                label,
                latest_version_num: ver,
                latest_parent_version: parent,
                latest_size_bytes: size.unwrap_or(0),
                latest_sha256: sha.unwrap_or_default(),
                updated_at: updated
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap_or_default(),
            },
        )
        .collect();

    Ok(Json(Manifest {
        generated_at: OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default(),
        saves,
    }))
}
