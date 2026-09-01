//! `/v1/playtime`: self-hosted mirror of the agent's real-hours-played
//! tracker, attributed per local day and per game. SQLite/bearer counterpart of
//! the cloud route (`cloud::routes::playtime`); same wire shape so the recap
//! (hoard-wrapple) reads either source identically.
//!
//! Self-hosted model: one machine is one server user. Rows are scoped by the
//! bearer's `user_id`, so a user's recap is their OWN machine's history, not a
//! cross-machine merge (that's the cloud model, where many devices hang off one
//! account). `device_fp` is still recorded (and the aggregate sums over it), but
//! for a 1:1 machine↔user server that's just the single device.
//!
//! - `POST` replaces a device's breakdown of `(day, game, secs)` rows, only as
//!   wide as the client can vouch for; see the cloud route for the full story.
//!   The short version: this used to wipe every row for the device on the
//!   grounds that "the client's local playtime is monotonic", an invariant
//!   nothing enforced and that a reinstall breaks. Now a payload clears only the
//!   days it mentions, unless it sets `authoritative` (the agent does that only
//!   when its store came off disk).
//! - `GET` returns the device-merged aggregate in the same
//!   `{ days, by_game, daily_by_game, total_secs }` shape the recap reads
//!   locally. The synthetic `__other__` slug counts toward `days`/`total_secs`
//!   but is hidden from `by_game` and `daily_by_game`, since it isn't a real game.

use std::collections::BTreeMap;
use std::sync::Arc;

use axum::{
    extract::{Extension, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};

use crate::auth::AuthUser;
use crate::routes::health::ServerState;

/// Hard cap on rows per upload. A real history is days by games, a few thousand
/// at most. The cap only stops a malicious client from flooding the table.
const MAX_ROWS: usize = 50_000;

/// Synthetic slug for time attributed to a day but not to any specific game.
/// Mirrors the agent constant.
const OTHER_SLUG: &str = "__other__";

#[derive(Debug, Deserialize)]
pub struct PlaytimeUpload {
    /// Device fingerprint (the agent's logship identity). Scopes the rows so
    /// multiple machines accumulate independently instead of overwriting.
    pub device_fp: String,
    /// The client vouches for this device's *whole* history. Defaults to
    /// `false` so an older client gets the safe behaviour without an update.
    #[serde(default)]
    pub authoritative: bool,
    pub rows: Vec<PlaytimeRowIn>,
}

#[derive(Debug, Deserialize)]
pub struct PlaytimeRowIn {
    /// Local day `YYYY-MM-DD`.
    pub day: String,
    pub game_slug: String,
    pub secs: u64,
}

#[derive(Debug, Serialize)]
pub struct UploadResponse {
    pub accepted: usize,
}

/// Same shape the desktop's local `list_playtime` and the cloud route return,
/// so the recap reads any source identically.
#[derive(Debug, Serialize)]
pub struct PlaytimeAggregate {
    pub days: BTreeMap<String, u64>,
    pub by_game: BTreeMap<String, u64>,
    /// day → (game_slug → secs), merged across devices, real games only.
    pub daily_by_game: BTreeMap<String, BTreeMap<String, u64>>,
    pub total_secs: u64,
}

/// `YYYY-MM-DD` with all-digit year/month/day. Cheap guard against malformed
/// days polluting the store.
fn valid_day(day: &str) -> bool {
    let b = day.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
}

pub async fn upload(
    Extension(user): Extension<AuthUser>,
    State(state): State<Arc<ServerState>>,
    Json(body): Json<PlaytimeUpload>,
) -> Result<(StatusCode, Json<UploadResponse>), StatusCode> {
    let fp = body.device_fp.trim();
    if fp.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    if body.rows.len() > MAX_ROWS {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }

    let user_id = user.user_id.to_string();

    let mut tx = state.pool.begin().await.map_err(|e| {
        tracing::error!(error = %e, "playtime tx begin failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Clear before re-inserting, but only as wide as the client can vouch for.
    // Day by day rather than one statement with an `IN (...)`: SQLite has no
    // array binding, and a payload covers a handful of days inside a
    // transaction that is already doing one INSERT per row.
    if body.authoritative {
        sqlx::query("DELETE FROM playtime WHERE user_id = ? AND device_fp = ?")
            .bind(&user_id)
            .bind(fp)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "playtime delete failed");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
    } else {
        let mut days: Vec<&str> = body
            .rows
            .iter()
            .filter(|r| valid_day(&r.day))
            .map(|r| r.day.as_str())
            .collect();
        days.sort_unstable();
        days.dedup();
        for day in days {
            sqlx::query("DELETE FROM playtime WHERE user_id = ? AND device_fp = ? AND day = ?")
                .bind(&user_id)
                .bind(fp)
                .bind(day)
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    tracing::error!(error = %e, "playtime delete failed");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
        }
    }

    let mut accepted = 0usize;
    for row in &body.rows {
        if row.secs == 0 || row.game_slug.is_empty() || !valid_day(&row.day) {
            continue;
        }
        sqlx::query(
            "INSERT INTO playtime (user_id, device_fp, day, game_slug, secs, updated_at)
             VALUES (?, ?, ?, ?, ?, datetime('now'))
             ON CONFLICT (user_id, device_fp, day, game_slug)
             DO UPDATE SET secs = MAX(playtime.secs, excluded.secs),
                           updated_at = datetime('now')",
        )
        .bind(&user_id)
        .bind(fp)
        .bind(&row.day)
        .bind(&row.game_slug)
        .bind(row.secs as i64)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "playtime insert failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        accepted += 1;
    }
    tx.commit().await.map_err(|e| {
        tracing::error!(error = %e, "playtime tx commit failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok((StatusCode::OK, Json(UploadResponse { accepted })))
}

pub async fn aggregate(
    Extension(user): Extension<AuthUser>,
    State(state): State<Arc<ServerState>>,
) -> Result<Json<PlaytimeAggregate>, StatusCode> {
    let user_id = user.user_id.to_string();

    let err = |e: sqlx::Error| {
        tracing::error!(error = %e, "playtime aggregate query failed");
        StatusCode::INTERNAL_SERVER_ERROR
    };

    // Per-day total across every device and game (includes `__other__`).
    let day_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT day, CAST(SUM(secs) AS INTEGER) AS secs
           FROM playtime
          WHERE user_id = ?
          GROUP BY day
          ORDER BY day",
    )
    .bind(&user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(err)?;

    // Per-game total across every device and day, excluding the synthetic
    // remainder slug so the recap's "top games" stays real.
    let game_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT game_slug, CAST(SUM(secs) AS INTEGER) AS secs
           FROM playtime
          WHERE user_id = ? AND game_slug <> ?
          GROUP BY game_slug",
    )
    .bind(&user_id)
    .bind(OTHER_SLUG)
    .fetch_all(&state.pool)
    .await
    .map_err(err)?;

    // Per-(day, game) total across devices, real games only.
    let day_game_rows: Vec<(String, String, i64)> = sqlx::query_as(
        "SELECT day, game_slug, CAST(SUM(secs) AS INTEGER) AS secs
           FROM playtime
          WHERE user_id = ? AND game_slug <> ?
          GROUP BY day, game_slug",
    )
    .bind(&user_id)
    .bind(OTHER_SLUG)
    .fetch_all(&state.pool)
    .await
    .map_err(err)?;

    let mut days = BTreeMap::new();
    let mut total_secs = 0u64;
    for (day, secs) in day_rows {
        let secs = secs.max(0) as u64;
        total_secs += secs;
        days.insert(day, secs);
    }
    let by_game = game_rows
        .into_iter()
        .map(|(slug, secs)| (slug, secs.max(0) as u64))
        .collect();
    let mut daily_by_game: BTreeMap<String, BTreeMap<String, u64>> = BTreeMap::new();
    for (day, slug, secs) in day_game_rows {
        daily_by_game
            .entry(day)
            .or_default()
            .insert(slug, secs.max(0) as u64);
    }

    Ok(Json(PlaytimeAggregate {
        days,
        by_game,
        daily_by_game,
        total_secs,
    }))
}
