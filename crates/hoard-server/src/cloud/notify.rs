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
/// email switched off, no profile, a profile with no address, or a paying one.
///
/// **Pro accounts get none of these.** Every notice here ends in a pitch for
/// the plan they already bought, and the numbers around it read as nonsense to
/// them ("100 GB instead of 100 GB"). Their copy of the same information is in
/// the app, where it belongs. The one mail Pro still receives is the export
/// link, which is a reply to something they asked for and does not come
/// through here.
///
/// The third value is the reader's offers token, or `None` when they turned
/// offers off: then the notice still goes out, without the pitch for Pro.
async fn prepare(state: &CloudState, user_id: Uuid) -> Option<(EmailConfig, String, Option<Uuid>)> {
    let cfg = state.config.cloud.as_ref().map(|c| c.email.clone())?;
    if !email::is_configured(&cfg) {
        return None;
    }
    let row: Option<(String, String, Option<time::OffsetDateTime>, Uuid)> = sqlx::query_as(
        "SELECT email, plan, offers_opt_out_at, offers_token FROM profiles WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();
    let (to, plan, opted_out, token) = row?;
    if Plan::from_str(&plan).unwrap_or(Plan::Free) != Plan::Free {
        return None;
    }
    if to.is_empty() {
        return None;
    }
    let offers = if opted_out.is_none() {
        Some(token)
    } else {
        None
    };
    Some((cfg, to, offers))
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

/// Shortest gap between two "we deleted more of your history" emails.
///
/// This notice repeats on purpose: unlike the others it describes something
/// that happened *again* today, and a purge that runs every day is deleting
/// history every day. One a day is the ceiling, and the link in the message
/// ends it for good.
const PURGE_COOLDOWN: Duration = Duration::from_secs(24 * 60 * 60);

/// How long without a purge before the notice, and any mute on it, expire.
/// A fortnight of quiet means the situation passed; the next purge is news.
pub const PURGE_NOTICE_TTL: Duration = Duration::from_secs(14 * 24 * 60 * 60);

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
        let Some((cfg, to, offers)) = prepare(&state, user_id).await else {
            return;
        };
        let kind = Kind::StoragePurgeStarted;
        let token = match notices::claim_periodic(
            &state.pool,
            user_id,
            kind,
            notices::ACCOUNT,
            PURGE_COOLDOWN,
        )
        .await
        {
            Ok(Some(t)) => t,
            // Muted, or already sent today.
            Ok(None) => return,
            Err(e) => {
                tracing::warn!(%user_id, error = %e, "purge notice claim failed");
                return;
            }
        };
        let r = email::send_storage_purge_started(
            &cfg,
            &to,
            offers,
            used,
            limit,
            deleted_versions,
            deleted_games,
            token,
        )
        .await;
        // No `settle`: a failed send must not delete the row, or the next
        // upload in the same minute would try again and the daily ceiling would
        // stop meaning anything. It just waits for tomorrow.
        if let Err(e) = r {
            tracing::warn!(%user_id, error = %e, "purge notice email failed");
        }
    });
}

/// An upload was rejected for want of space.
///
/// It says which backup was turned away, because that is the true statement:
/// the quota check rejects an upload that does not *fit*, which happens long
/// before the account is literally full. "Your storage is full" over a table
/// reading 600 MB of 2 GB is how you lose someone's trust in one message.
pub fn storage_full(
    state: &CloudState,
    user_id: Uuid,
    save_id: String,
    requested: i64,
    used: i64,
    limit: i64,
) {
    let state = state.clone();
    tokio::spawn(async move {
        let Some((cfg, to, offers)) = prepare(&state, user_id).await else {
            return;
        };
        let kind = Kind::StorageFull;
        if !claimed(&state, user_id, kind, notices::ACCOUNT).await {
            return;
        }
        // Resolved here rather than at the call site: this runs once per
        // account per fill, while `check_storage` runs on every upload.
        let slug: Option<String> =
            sqlx::query_scalar("SELECT game_slug FROM saves WHERE id = $1 AND user_id = $2")
                .bind(&save_id)
                .bind(user_id)
                .fetch_optional(&state.pool)
                .await
                .ok()
                .flatten();
        let game = slug.unwrap_or_else(|| save_id.clone());
        let r = email::send_storage_full(&cfg, &to, offers, &game, requested, used, limit).await;
        settle(&state, user_id, kind, notices::ACCOUNT, r).await;
    });
}

/// One save is over the per-game cap, so only that game stopped syncing.
#[allow(clippy::too_many_arguments)]
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
        let Some((cfg, to, offers)) = prepare(&state, user_id).await else {
            return;
        };
        // The cap check runs before `init_upload` resolves the save, so the id
        // is whatever the client sent. Without this, N invented ids are N
        // emails and N rows nothing ever cleans up.
        let owned: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM saves WHERE id = $1 AND user_id = $2)",
        )
        .bind(&save_id)
        .bind(user_id)
        .fetch_one(&state.pool)
        .await
        .unwrap_or(false);
        if !owned {
            return;
        }
        let kind = Kind::SaveTooLarge;
        if !claimed(&state, user_id, kind, &save_id).await {
            return;
        }
        let r =
            email::send_save_too_large(&cfg, &to, offers, &game_slug, size, limit, plan, pro_limit)
                .await;
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
        let Some((cfg, to, offers)) = prepare(&state, user_id).await else {
            return;
        };
        let kind = Kind::ArchiveExpiring;
        if !claimed(&state, user_id, kind, &save_id).await {
            return;
        }
        let r = email::send_archive_expiring(
            &cfg,
            &to,
            offers,
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
        let Some((cfg, to, offers)) = prepare(&state, user_id).await else {
            return;
        };
        let kind = Kind::DevicesFull;
        if !claimed(&state, user_id, kind, notices::ACCOUNT).await {
            return;
        }
        let r = email::send_devices_full(
            &cfg,
            &to,
            offers,
            &device_name,
            &device_os,
            used,
            limit,
            plan,
        )
        .await;
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
    let warned: Vec<(Uuid,)> =
        sqlx::query_as("SELECT DISTINCT user_id FROM email_notices WHERE kind = 'storage_full'")
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
        notices::clear(&state.pool, user_id, Kind::StorageFull, notices::ACCOUNT).await?;
    }
    // The purge notice is not re-armed by going under the threshold: it is
    // periodic, and its row expires on its own once nothing refreshes it. That
    // also lifts any mute the reader set while it was happening.
    let expired =
        notices::expire_stale(&state.pool, Kind::StoragePurgeStarted, PURGE_NOTICE_TTL).await?;
    if expired > 0 {
        tracing::info!(
            rows = expired,
            "purge notices expired after a quiet fortnight"
        );
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
        // Live versions only. Counting soft-deleted or half-uploaded rows would
        // put a number next to a size that filters them, and the two figures
        // sit in the same table row of the email.
        let versions: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM save_versions
              WHERE save_id = $1 AND deleted_at IS NULL AND sha256 <> ''",
        )
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
