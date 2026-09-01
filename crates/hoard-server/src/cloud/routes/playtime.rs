//! `/v1/cloud/playtime`: cloud mirror of the agent's real-hours-played
//! tracker, attributed per local day and per game, per device.
//!
//! - `POST` replaces a device's breakdown of `(day, game, secs)` rows, scoped
//!   by how much the client can actually vouch for. `device_fp` keeps two
//!   machines that played the same game on the same day independent.
//!
//!   **How wide the replace goes is the client's call, and defaults to narrow.**
//!   This used to wipe every row for `(user_id, device_fp)` unconditionally,
//!   justified by "the client's local store is monotonic, so this never loses
//!   history". Nothing enforced that invariant, and it stops holding the moment
//!   the local store is gone: a reinstall, a wiped `AppData`, a fresh profile.
//!   Then the client came back with an empty breakdown, this route believed it,
//!   and the account's history was deleted server-side. That is exactly how a
//!   user lost theirs (2026-08-07): the app was reinstalled at 03:19 and its
//!   own routine push finished the job at 04:17.
//!
//!   So a payload now only clears the days it actually mentions, unless it sets
//!   `authoritative`, which the agent does only when its store came off disk.
//!   A client that lost its file says nothing about last month, so last month
//!   survives; a client that still has its file can retract a day (a game
//!   excluded from the count) and the retraction lands.
//!
//!   The old wide behaviour remains reachable for what it was for: a device
//!   that legitimately has *nothing* for an account (after the per-account
//!   playtime partition, a second account that never played here) pushes an
//!   authoritative empty set and its stale rows go.
//! - `GET` returns the device-merged aggregate in the same
//!   `{ days, by_game, total_secs }` shape the recap reads locally, so the UI
//!   can swap its source without reshaping. The synthetic `__other__` slug
//!   (time from days that predate the per-game breakdown) counts toward `days`
//!   and `total_secs` but is hidden from `by_game`, since it isn't a real game.

use std::collections::BTreeMap;

use axum::{extract::State, http::StatusCode, response::Json, Extension};
use serde::{Deserialize, Serialize};

use crate::cloud::auth::CloudUser;
use crate::cloud::errors::CloudError;
use crate::cloud::state::CloudState;

/// Hard cap on rows per upload. A real history is days by games, a few
/// thousand at most. The cap only stops a malicious client from flooding the
/// table; an honest agent never approaches it.
const MAX_ROWS: usize = 50_000;

/// Synthetic slug for time attributed to a day but not to any specific game
/// (days that predate the per-game breakdown). Mirrors the agent constant.
const OTHER_SLUG: &str = "__other__";

#[derive(Debug, Deserialize)]
pub struct PlaytimeUpload {
    /// Device fingerprint (the agent's logship identity). Scopes the rows so
    /// multiple machines accumulate independently instead of overwriting.
    pub device_fp: String,
    /// The client vouches for this device's *whole* history, so anything it
    /// doesn't send may be dropped.
    ///
    /// Defaults to `false`, and the default is the point: an older client, or any
    /// client that can't tell a real zero from a lost file, gets the safe
    /// behaviour without being updated first.
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

/// Same shape the desktop's local `list_playtime` returns, so the recap reads
/// either source identically.
#[derive(Debug, Serialize)]
pub struct PlaytimeAggregate {
    pub days: BTreeMap<String, u64>,
    pub by_game: BTreeMap<String, u64>,
    /// day → (game_slug → secs), merged across devices, real games only
    /// (`__other__` excluded). Powers the recap's per-day detail panel.
    pub daily_by_game: BTreeMap<String, BTreeMap<String, u64>>,
    pub total_secs: u64,
}

/// `YYYY-MM-DD` with all-digit year/month/day. Cheap guard so a malformed day
/// can't reach the `::date` cast and abort the whole batch with a 500.
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
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
    Json(body): Json<PlaytimeUpload>,
) -> Result<(StatusCode, Json<UploadResponse>), CloudError> {
    let fp = body.device_fp.trim();
    if fp.is_empty() {
        return Err(CloudError::BadRequest("device_fp is required".into()));
    }
    if body.rows.len() > MAX_ROWS {
        return Err(CloudError::BadRequest(format!(
            "too many rows: {} (max {MAX_ROWS})",
            body.rows.len()
        )));
    }

    let mut tx = state.pool.begin().await?;

    // Clear before re-inserting, but only as wide as the client can vouch for.
    // A day the payload mentions is fully restated by it (so a retracted game
    // disappears); a day it doesn't mention is none of this payload's business
    // unless the client says it knows its whole past.
    if body.authoritative {
        sqlx::query("DELETE FROM playtime WHERE user_id = $1 AND device_fp = $2")
            .bind(user.user_id)
            .bind(fp)
            .execute(&mut *tx)
            .await?;
    } else {
        let days: Vec<String> = {
            let mut d: Vec<String> = body
                .rows
                .iter()
                .filter(|r| valid_day(&r.day))
                .map(|r| r.day.clone())
                .collect();
            d.sort_unstable();
            d.dedup();
            d
        };
        if !days.is_empty() {
            sqlx::query(
                "DELETE FROM playtime
                  WHERE user_id = $1 AND device_fp = $2 AND day = ANY($3::date[])",
            )
            .bind(user.user_id)
            .bind(fp)
            .bind(&days)
            .execute(&mut *tx)
            .await?;
        }
    }

    let mut accepted = 0usize;
    for row in &body.rows {
        if row.secs == 0 || row.game_slug.is_empty() || !valid_day(&row.day) {
            continue;
        }
        sqlx::query(
            r#"
            INSERT INTO playtime (user_id, device_fp, day, game_slug, secs, updated_at)
            VALUES ($1, $2, $3::date, $4, $5, now())
            ON CONFLICT (user_id, device_fp, day, game_slug)
            DO UPDATE SET secs = GREATEST(playtime.secs, EXCLUDED.secs),
                          updated_at = now()
            "#,
        )
        .bind(user.user_id)
        .bind(fp)
        .bind(&row.day)
        .bind(&row.game_slug)
        .bind(row.secs as i64)
        .execute(&mut *tx)
        .await?;
        accepted += 1;
    }
    tx.commit().await?;

    Ok((StatusCode::OK, Json(UploadResponse { accepted })))
}

pub async fn aggregate(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
) -> Result<Json<PlaytimeAggregate>, CloudError> {
    // Per-day total across every device and game (includes `__other__`).
    let day_rows: Vec<(String, i64)> = sqlx::query_as(
        r#"
        SELECT to_char(day, 'YYYY-MM-DD') AS day, SUM(secs)::bigint AS secs
          FROM playtime
         WHERE user_id = $1
         GROUP BY day
         ORDER BY day
        "#,
    )
    .bind(user.user_id)
    .fetch_all(&state.pool)
    .await?;

    // Per-game total across every device and day, excluding the synthetic
    // remainder slug so the recap's "top games" stays real.
    let game_rows: Vec<(String, i64)> = sqlx::query_as(
        r#"
        SELECT game_slug, SUM(secs)::bigint AS secs
          FROM playtime
         WHERE user_id = $1 AND game_slug <> $2
         GROUP BY game_slug
        "#,
    )
    .bind(user.user_id)
    .bind(OTHER_SLUG)
    .fetch_all(&state.pool)
    .await?;

    // Per-(day, game) total across devices, real games only: the day-detail
    // breakdown the recap shows when a square is clicked.
    let day_game_rows: Vec<(String, String, i64)> = sqlx::query_as(
        r#"
        SELECT to_char(day, 'YYYY-MM-DD') AS day, game_slug, SUM(secs)::bigint AS secs
          FROM playtime
         WHERE user_id = $1 AND game_slug <> $2
         GROUP BY day, game_slug
        "#,
    )
    .bind(user.user_id)
    .bind(OTHER_SLUG)
    .fetch_all(&state.pool)
    .await?;

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
