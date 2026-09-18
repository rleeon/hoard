//! Service emails, fired and forgotten.
//!
//! Every function here is called from somewhere that must not slow down or
//! fail because of an email: an upload commit, the 402 path, a nightly sweep.
//! So they take what they need, spawn, and return. Nothing in a user request
//! ever waits on Resend.
//!
//! The order inside the task matters and is the same everywhere:
//!
//! 1. Is email even configured? If not, stop *before* claiming. Otherwise a
//!    server running with email off would quietly mark every user as told, and
//!    the day the key is added nobody would hear anything again.
//! 2. Claim the notice ([`notices::claim`]), which is what makes it once-only.
//! 3. Send. If that fails, release the claim so the next time the same
//!    condition comes round the user gets another chance. Nothing was
//!    delivered, so there is nothing to duplicate.

use crate::cloud::archive::ARCHIVE_GRACE_DAYS;
use crate::cloud::email;
use crate::cloud::notices::{self, Kind};
use crate::cloud::plans::{resolved_storage_limit, Plan};
use crate::cloud::purge::purge_threshold;
use crate::cloud::state::CloudState;
use crate::config::EmailConfig;
use std::time::Duration;
use uuid::Uuid;

/// How long before the hard delete an archived save is worth an email. Three
/// days out of seven: late enough that somebody who archived on purpose and
/// moved on is not nagged the same afternoon, early enough to act on.
const ARCHIVE_WARN_DAYS: i64 = 3;

/// Email config plus the address, or `None` when there is nothing to do:
/// email switched off, no profile, or a profile with no address.
async fn prepare(state: &CloudState, user_id: Uuid) -> Option<(EmailConfig, String)> {
    let cfg = state.config.cloud.as_ref().map(|c| c.email.clone())?;
    if !email::is_configured(&cfg) {
        return None;
    }
    let to: Option<String> = sqlx::query_scalar("SELECT email FROM profiles WHERE user_id = $1")
        .bind(user_id)
        .fetch_optional(&state.pool)
        .await
        .ok()
        .flatten();
    Some((cfg, to.filter(|e| !e.is_empty())?))
}

/// Log the outcome and, on failure, re-arm the notice.
async fn settle(
    state: &CloudState,
    user_id: Uuid,
    kind: Kind,
    scope: &str,
    result: anyhow::Result<bool>,
) {
    match result {
        Ok(true) => tracing::info!(%user_id, kind = kind.as_str(), scope, "notice emailed"),
        // `false` is "email is off", which `prepare` already ruled out.
        Ok(false) => {}
        Err(e) => {
            tracing::warn!(%user_id, kind = kind.as_str(), error = %e, "notice email failed");
            if let Err(e) = notices::clear(&state.pool, user_id, kind, scope).await {
                tracing::warn!(%user_id, error = %e, "could not re-arm notice after failure");
            }
        }
    }
}

/// True when this caller owns the send. Any database trouble answers `false`:
/// not knowing whether we already wrote is a reason to stay quiet.
async fn claimed(state: &CloudState, user_id: Uuid, kind: Kind, scope: &str) -> bool {
    match notices::claim(&state.pool, user_id, kind, scope).await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(%user_id, kind = kind.as_str(), error = %e, "notice claim failed");
            false
        }
    }
}

/// The purge just deleted history to make room.
pub fn storage_purge_started(
    state: &CloudState,
    user_id: Uuid,
    used: i64,
    limit: i64,
    deleted_versions: usize,
    deleted_games: usize,
) {
    let state = state.clone();
    tokio::spawn(async move {
        let Some((cfg, to)) = prepare(&state, user_id).await else {
            return;
        };
        let kind = Kind::StoragePurgeStarted;
        if !claimed(&state, user_id, kind, notices::ACCOUNT).await {
            return;
        }
        let r = email::send_storage_purge_started(
            &cfg,
            &to,
            used,
            limit,
            deleted_versions,
            deleted_games,
        )
        .await;
        settle(&state, user_id, kind, notices::ACCOUNT, r).await;
    });
}

/// An upload was rejected for want of space.
pub fn storage_full(state: &CloudState, user_id: Uuid, used: i64, limit: i64) {
    let state = state.clone();
    tokio::spawn(async move {
        let Some((cfg, to)) = prepare(&state, user_id).await else {
            return;
        };
        let kind = Kind::StorageFull;
        if !claimed(&state, user_id, kind, notices::ACCOUNT).await {
            return;
        }
        let r = email::send_storage_full(&cfg, &to, used, limit).await;
        settle(&state, user_id, kind, notices::ACCOUNT, r).await;
    });
}

/// One save is over the per-game cap, so only that game stopped syncing.
pub fn save_too_large(
    state: &CloudState,
    user_id: Uuid,
    save_id: String,
    game_slug: String,
    size: i64,
    limit: i64,
    plan: &'static str,
    pro_limit: i64,
) {
    let state = state.clone();
    tokio::spawn(async move {
        let Some((cfg, to)) = prepare(&state, user_id).await else {
            return;
        };
        let kind = Kind::SaveTooLarge;
        if !claimed(&state, user_id, kind, &save_id).await {
            return;
        }
        let r =
            email::send_save_too_large(&cfg, &to, &game_slug, size, limit, plan, pro_limit).await;
        settle(&state, user_id, kind, &save_id, r).await;
    });
}

/// An archived save is near the end of its grace window.
#[allow(clippy::too_many_arguments)]
pub fn archive_expiring(
    state: &CloudState,
    user_id: Uuid,
    save_id: String,
    game_slug: String,
    days: i64,
    archived_on: String,
    delete_on: String,
    versions: i64,
    size: i64,
) {
    let state = state.clone();
    tokio::spawn(async move {
        let Some((cfg, to)) = prepare(&state, user_id).await else {
            return;
        };
        let kind = Kind::ArchiveExpiring;
        if !claimed(&state, user_id, kind, &save_id).await {
            return;
        }
        let r = email::send_archive_expiring(
            &cfg,
            &to,
            &game_slug,
            days,
            &archived_on,
            &delete_on,
            versions,
            size,
        )
        .await;
        settle(&state, user_id, kind, &save_id, r).await;
    });
}

/// The account just filled its last device slot.
pub fn devices_full(
    state: &CloudState,
    user_id: Uuid,
    device_name: String,
    device_os: String,
    used: i64,
    limit: i64,
    plan: &'static str,
) {
    let state = state.clone();
    tokio::spawn(async move {
        let Some((cfg, to)) = prepare(&state, user_id).await else {
            return;
        };
        let kind = Kind::DevicesFull;
        if !claimed(&state, user_id, kind, notices::ACCOUNT).await {
            return;
        }
        let r =
            email::send_devices_full(&cfg, &to, &device_name, &device_os, used, limit, plan).await;
        settle(&state, user_id, kind, notices::ACCOUNT, r).await;
    });
}

// ---- daily housekeeping

/// Spawn the once-a-day pass: re-arm storage notices for accounts that are no
/// longer full, and warn about archived saves about to be deleted.
///
/// Both belong to conditions that change over days, never seconds, so a daily
/// cadence costs nothing in usefulness and keeps them off the upload path.
pub fn spawn_daily(state: CloudState) {
    tokio::spawn(async move {
        // A few minutes after boot, like the other sweepers: a task that first
        // fires 24h in never fires at all on a machine that idles to zero.
        tokio::time::sleep(Duration::from_secs(6 * 60)).await;
        let mut tick = tokio::time::interval(Duration::from_secs(24 * 60 * 60));
        tick.tick().await;
        loop {
            if let Err(e) = rearm_storage(&state).await {
                tracing::warn!(error = %e, "notice re-arm sweep failed");
            }
            if let Err(e) = warn_expiring_archives(&state).await {
                tracing::warn!(error = %e, "archive expiry warning sweep failed");
            }
            tick.tick().await;
        }
    });
}

/// Clear the storage notices of every account that is back under its purge
/// threshold. Driven off `email_notices` rather than off `profiles`, so the
/// work is proportional to the few users who were ever warned.
async fn rearm_storage(state: &CloudState) -> Result<(), sqlx::Error> {
    let warned: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT DISTINCT user_id FROM email_notices
          WHERE kind IN ('storage_purge_started', 'storage_full')",
    )
    .fetch_all(&state.pool)
    .await?;

    for (user_id,) in warned {
        let row: Option<(String, i64, Option<i64>, Option<time::OffsetDateTime>)> = sqlx::query_as(
            "SELECT plan, storage_bytes, storage_limit_bytes, storage_limit_change_at
               FROM profiles WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_optional(&state.pool)
        .await?;
        let Some((plan, used, limit_override, change_at)) = row else {
            continue;
        };
        let plan = Plan::from_str(&plan).unwrap_or(Plan::Free);
        let limit = resolved_storage_limit(
            plan,
            limit_override,
            change_at.map(|t| t.unix_timestamp()),
            time::OffsetDateTime::now_utc().unix_timestamp(),
        ) as i64;
        if used > (limit as f64 * purge_threshold(plan)) as i64 {
            continue;
        }
        for kind in [Kind::StoragePurgeStarted, Kind::StorageFull] {
            notices::clear(&state.pool, user_id, kind, notices::ACCOUNT).await?;
        }
    }
    rearm_oversized_saves(state).await
}

/// Clear the per-save notices of games that now fit. A user who trimmed a save
/// folder should hear about it again if it grows back past the cap, and the
/// only way to know it fits is to compare its newest version against the cap
/// that applied to it.
async fn rearm_oversized_saves(state: &CloudState) -> Result<(), sqlx::Error> {
    let warned: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT user_id, scope FROM email_notices WHERE kind = 'save_too_large' AND scope <> ''",
    )
    .fetch_all(&state.pool)
    .await?;

    for (user_id, save_id) in warned {
        let cap: Option<(String, Option<i64>)> =
            sqlx::query_as("SELECT plan, max_save_size_bytes FROM profiles WHERE user_id = $1")
                .bind(user_id)
                .fetch_optional(&state.pool)
                .await?;
        let Some((plan, cap_override)) = cap else {
            continue;
        };
        let plan = Plan::from_str(&plan).unwrap_or(Plan::Free);
        let cap = crate::cloud::plans::resolved_save_size_limit(plan, cap_override) as i64;
        let newest: Option<i64> = sqlx::query_scalar(
            "SELECT size_bytes FROM save_versions
              WHERE save_id = $1 AND deleted_at IS NULL
              ORDER BY version_num DESC LIMIT 1",
        )
        .bind(&save_id)
        .fetch_optional(&state.pool)
        .await?;
        // No versions left means the save was emptied or deleted; the scope
        // cleanup handles that case, so leave the row for it.
        if newest.is_some_and(|size| size <= cap) {
            notices::clear(&state.pool, user_id, Kind::SaveTooLarge, &save_id).await?;
        }
    }
    Ok(())
}

/// Email the owner of every archived save whose grace window runs out within
/// [`ARCHIVE_WARN_DAYS`]. Claiming is per save, so a user with three archived
/// games gets three warnings, each naming its own game.
async fn warn_expiring_archives(state: &CloudState) -> Result<(), sqlx::Error> {
    let due: Vec<(String, Uuid, String, time::OffsetDateTime)> = sqlx::query_as(
        "SELECT id, user_id, game_slug, archived_at FROM saves
          WHERE archived_at IS NOT NULL
            AND archived_at <= now() - make_interval(days => $1)
            AND archived_at >  now() - make_interval(days => $2)",
    )
    .bind((ARCHIVE_GRACE_DAYS - ARCHIVE_WARN_DAYS) as i32)
    .bind(ARCHIVE_GRACE_DAYS as i32)
    .fetch_all(&state.pool)
    .await?;

    for (save_id, user_id, game_slug, archived_at) in due {
        let versions: i64 =
            sqlx::query_scalar("SELECT count(*) FROM save_versions WHERE save_id = $1")
                .bind(&save_id)
                .fetch_one(&state.pool)
                .await?;
        // The bytes that actually disappear: this save's frozen blobs, the same
        // sum `reactivate_save` charges back against the quota.
        let size: Option<i64> = sqlx::query_scalar(
            r#"
            SELECT COALESCE(SUM(b.size_bytes), 0)::bigint
            FROM (SELECT DISTINCT sha256 FROM manifest_files WHERE save_id = $1) r
            JOIN cloud_blobs b ON b.user_id = $2 AND b.sha256 = r.sha256
            WHERE b.refcount = 0
            "#,
        )
        .bind(&save_id)
        .bind(user_id)
        .fetch_one(&state.pool)
        .await?;

        let delete_at = archived_at + time::Duration::days(ARCHIVE_GRACE_DAYS);
        let days = ((delete_at - time::OffsetDateTime::now_utc()).whole_hours() as f64 / 24.0)
            .ceil()
            .max(1.0) as i64;
        archive_expiring(
            state,
            user_id,
            save_id,
            game_slug,
            days,
            fmt_day(archived_at),
            fmt_day(delete_at),
            versions,
            size.unwrap_or(0),
        );
    }
    Ok(())
}

/// "18 September". Long enough to be unambiguous in a message that crosses
/// date formats, short enough to sit in a table cell.
fn fmt_day(t: time::OffsetDateTime) -> String {
    const MONTHS: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    format!("{} {}", t.day(), MONTHS[t.month() as usize - 1])
}
