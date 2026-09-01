//! `/v1/me*`: account-facing endpoints for the desktop client.

use crate::cloud::abuse;
use crate::cloud::auth::CloudUser;
use crate::cloud::errors::CloudError;
use crate::cloud::plans::Plan;
use crate::cloud::quota;
use crate::cloud::state::CloudState;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::Json,
    Extension,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Wire shape for `GET /v1/me`. We expose used/limit pairs in bytes for
/// the storage bar and counts for devices + saves so the desktop's
/// /account page can render a coherent usage view without doing math.
/// Unlimited tiers send `-1` as the limit (clean to detect on the client
/// side without parsing string sentinels).
#[derive(Debug, Serialize)]
pub struct Me {
    pub user_id: Uuid,
    pub email: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub plan: String,
    /// RFC3339 account creation time. The desktop derives the
    /// "premium features unlocked for the first 30 days" trial from this.
    pub created_at: Option<String>,
    pub subscription_status: Option<String>,
    pub renews_at: Option<String>,
    pub cancel_at: Option<String>,
    pub storage_used_bytes: i64,
    pub storage_limit_bytes: i64,
    /// Total bytes ever stored, monotonic: never credited back on delete or
    /// purge. Powers the recap's lifetime "Atesorado". Backfilled to the
    /// current footprint at migration time, so it's only exact going forward.
    pub lifetime_storage_bytes: i64,
    pub devices_used: i32,
    pub devices_limit: i32,
    /// RFC3339 stamp of the first time this account went Pro, or `null` if it
    /// never did. One-way: a downgrade doesn't clear it. It's why an account on
    /// Free can legitimately report an unlimited `devices_limit`, so the UI can
    /// say *why* instead of looking like a bug.
    pub first_pro_at: Option<String>,
    pub saves_used: i32,
    pub saves_limit: i32,
    /// True on every tier post-1.6.1, kept on the wire as a bool so
    /// "rolling N days" tiers in the future flip this to `false` instead
    /// of forcing every client to start reading a `retention_days` int
    /// that used to be missing.
    pub version_history_forever: bool,
    pub max_save_size_bytes: i64,
    pub bandwidth_window_secs: i32,
    pub bandwidth_quota_bytes: i64,
    /// Storage pressure state for the UI gauge:
    /// - `"ok"`: under the auto-purge threshold (green).
    /// - `"purging"`: at or over threshold; old versions are auto-deleted to make
    ///   room (orange: the user is losing old history).
    /// - `"full"`: at the hard limit; nothing left to reclaim, sync uploads
    ///   are rejected (red).
    pub storage_status: &'static str,
    /// Set when a storage downgrade is scheduled but not yet in effect: the
    /// limit the account will drop to. The client warns the user to export
    /// before the deadline. `null` when no change is pending.
    pub pending_storage_limit_bytes: Option<i64>,
    /// RFC3339 instant the pending downgrade takes effect (end of the grace
    /// window). `null` when nothing is pending.
    pub storage_limit_change_at: Option<String>,
    /// Set when the account is soft-deleted and inside its 30-day grace: the
    /// RFC3339 instant deletion was requested. The desktop uses its presence to
    /// swap the whole app for a "scheduled for deletion, reactivate?" screen,
    /// since every data route is frozen (403 `account_scheduled_deletion`)
    /// until the user reactivates or the grace elapses. `null` for live
    /// accounts.
    pub deleted_at: Option<String>,
    /// RFC3339 instant the account is hard-purged if not reactivated
    /// (`deleted_at` + 30 days). `null` for live accounts.
    pub purges_at: Option<String>,
    /// User-chosen cap on stored versions per save. `null` = unlimited.
    /// Enforced server-side after every commit (oldest non-pinned versions
    /// beyond the cap are deleted). Set via `PUT /v1/me/max-versions`.
    ///
    /// Counts the automatic copies only. The ones the user asked for by hand have
    /// their own cap ([`Self::max_manual_versions`]) so a burst of autosaves
    /// cannot push them out of the history.
    pub max_versions: Option<i64>,
    /// Cap on the deliberate copies: the ones the user asked for, plus the safety
    /// net taken before a restore. `null` means unlimited, which is the default,
    /// because there are few of them and they are the ones worth keeping.
    pub max_manual_versions: Option<i64>,
    /// The Terms version this account last accepted, or `null` if none was ever
    /// recorded (accounts from before this existed). One half of the pair the
    /// client compares.
    pub terms_version_accepted: Option<String>,
    /// The version in force today. It travels even though the client has it
    /// compiled in: an older binary stuck on the previous literal would still
    /// believe it was up to date, and it is the other way round, the server
    /// decides.
    pub terms_version_current: &'static str,
}

/// Derive the storage gauge state from the deduped footprint vs. the limit
/// actually in force and the per-plan auto-purge threshold.
///
/// `grace` wins over everything: while a downgrade window is open the user is
/// still on their old, larger limit and **nothing is deleted**, so painting the
/// bar amber for "purging" would be a lie. The client shows the countdown
/// instead.
fn storage_status(plan: Plan, used: i64, limit: u64, grace: bool) -> &'static str {
    let limit = limit as i64;
    if limit <= 0 {
        return "ok";
    }
    if grace {
        return "grace";
    }
    if used >= limit {
        "full"
    } else if used as f64 >= limit as f64 * crate::cloud::purge::purge_threshold(plan) {
        "purging"
    } else {
        "ok"
    }
}

/// GET /v1/me: the current user's profile, plan and usage. Auto-creates the
/// profile row on first call (idempotent), so the client doesn't need a
/// separate "registration" step after Supabase OAuth.
pub async fn get_me(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
    headers: HeaderMap,
) -> Result<Json<Me>, CloudError> {
    upsert_profile_for(&state, &user, &headers).await?;
    // Register/refresh this machine in `devices` so the account page's
    // "Dispositivos N/M" reflects reality. Runs before the profile SELECT so
    // the recomputed `devices_count` is the one we return. Best-effort: a
    // client that sends no fingerprint (older builds) leaves the count alone.
    register_device(&state, &user, &headers).await?;

    // Sync any subscriptions that actually expired but status wasn't updated
    // (retroactive fix for webhooks that didn't fire or were delayed).
    // Check both renews_at (renewal date) and cancel_at (cancellation scheduled date).
    // Runs before the profile SELECT so a downgrade settled here is visible in
    // the very response that reports it, not one poll later.
    let expired_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM subscriptions
         WHERE user_id = $1 AND status IN ('active','grace')
         AND ((renews_at IS NOT NULL AND renews_at <= now())
              OR (cancel_at IS NOT NULL AND cancel_at <= now()))",
    )
    .bind(user.user_id)
    .fetch_one(&state.pool)
    .await?;
    if expired_count > 0 {
        let _ = sqlx::query(
            "UPDATE subscriptions SET status = 'expired', updated_at = now()
             WHERE user_id = $1 AND status IN ('active','grace')
             AND ((renews_at IS NOT NULL AND renews_at <= now())
                  OR (cancel_at IS NOT NULL AND cancel_at <= now()))",
        )
        .bind(user.user_id)
        .execute(&state.pool)
        .await;
        // Grace window first, plan flip second: `settle_storage_limit` reads the
        // old plan off the row to size the grant (same ordering as the webhook).
        if let Some(ref cloud) = state.config.cloud {
            let _ = quota::settle_storage_limit(
                &state.pool,
                user.user_id,
                Plan::Free,
                None,
                cloud.storage_downgrade_grace_days as i64,
            )
            .await;
        }
        let _ =
            sqlx::query("UPDATE profiles SET plan = 'free', updated_at = now() WHERE user_id = $1")
                .bind(user.user_id)
                .execute(&state.pool)
                .await;
    }

    // Promote any downgrade whose grace window elapsed, so the limit + status
    // we report are the live ones.
    quota::apply_due_downgrade(&state.pool, user.user_id).await?;
    // Purge snapshots if new limit is below current usage (e.g., downgrade from Pro to Free).
    let _ = crate::cloud::purge::maybe_purge(&state, user.user_id).await;

    let row: (
        String,
        Option<String>,
        Option<String>,
        String,
        i64,
        i32,
        time::OffsetDateTime,
        i64,
        Option<i64>,
        Option<i64>,
        Option<time::OffsetDateTime>,
        Option<time::OffsetDateTime>,
        Option<i32>,
        Option<i32>,
    ) = sqlx::query_as(
        "SELECT email, display_name, avatar_url, plan, storage_bytes, devices_count, created_at, lifetime_storage_bytes, storage_limit_bytes, pending_storage_limit_bytes, storage_limit_change_at, deleted_at, max_versions, max_manual_versions
           FROM profiles WHERE user_id = $1",
    )
    .bind(user.user_id)
    .fetch_one(&state.pool)
    .await?;

    // Read on its own rather than widening the tuple above: this is the one
    // fact that outlives the plan, and a positional 15-tuple is already the
    // most fragile line in this handler.
    let first_pro_at: Option<time::OffsetDateTime> =
        sqlx::query_scalar("SELECT first_pro_at FROM profiles WHERE user_id = $1")
            .bind(user.user_id)
            .fetch_one(&state.pool)
            .await?;

    let sub: Option<(
        String,
        Option<time::OffsetDateTime>,
        Option<time::OffsetDateTime>,
    )> = sqlx::query_as(
        "SELECT status, renews_at, cancel_at FROM subscriptions
              WHERE user_id = $1 AND status IN ('active','grace')
              ORDER BY updated_at DESC LIMIT 1",
    )
    .bind(user.user_id)
    .fetch_optional(&state.pool)
    .await?;

    // Saves count comes from a separate query, which keeps profile reads cheap
    // for endpoints that don't need this and avoids a join when the saves
    // count would be `0` on day one anyway. `saves` has no soft-delete
    // column today (only `save_versions` does); a plain COUNT is the right
    // shape. Propagate the SQL error rather than swallowing it so schema
    // drift surfaces here instead of a silent `0`.
    let saves_used: i64 =
        sqlx::query_scalar("SELECT COUNT(*)::bigint FROM saves WHERE user_id = $1")
            .bind(user.user_id)
            .fetch_one(&state.pool)
            .await?;

    // The last recorded acceptance. It goes on its own rather than in the big
    // SELECT above because its table is append-only and lives apart from
    // `profiles`: a JOIN here would only let a silly failure in this addition take
    // down the whole account response.
    let accepted_terms: Option<String> = sqlx::query_scalar(
        "SELECT version FROM terms_acceptances
          WHERE user_id = $1 ORDER BY accepted_at DESC LIMIT 1",
    )
    .bind(user.user_id)
    .fetch_optional(&state.pool)
    .await
    .unwrap_or_else(|e| {
        // And if it fails, it stays quiet: `/v1/me` is the call the whole account
        // screen hangs off. A piece of paperwork cannot be what stops somebody
        // looking at their plan. The client reads `null` and asks for the checkbox
        // again, which is the safe side of the failure.
        tracing::warn!("terms acceptance lookup failed: {e}");
        None
    });

    let plan = Plan::from_str(&row.3).unwrap_or(Plan::Free);
    let mut limits = plan.limits();
    // Devices bought on Pro are kept for life; only the storage goes back. See
    // `plans::resolved_devices_limit` for why the two part ways.
    limits.devices = crate::cloud::plans::resolved_devices_limit(plan, first_pro_at.is_some());
    // A pending downgrade exists iff a change instant is set; while it's in the
    // future `storage_limit_bytes` is the absolute grant the user keeps and the
    // plan column may already say "free", so resolve rather than just read the tier.
    let pending_change_at = row.10;
    limits.storage_bytes = crate::cloud::plans::resolved_storage_limit(
        plan,
        row.8,
        pending_change_at.map(|t| t.unix_timestamp()),
        time::OffsetDateTime::now_utc().unix_timestamp(),
    );
    // Where the limit is headed when the window closes (NULL pending = the plan
    // base).
    let pending_limit =
        pending_change_at.map(|_| crate::cloud::plans::effective_storage_limit(plan, row.9) as i64);
    let deleted_at = row.11;
    let purges_at = deleted_at.map(|d| d + time::Duration::days(GRACE_DAYS as i64));

    Ok(Json(Me {
        user_id: user.user_id,
        email: row.0,
        display_name: row.1,
        avatar_url: row.2,
        plan: plan.as_str().to_string(),
        created_at: Some(format_dt(row.6)),
        subscription_status: sub.as_ref().map(|s| s.0.clone()),
        renews_at: sub.as_ref().and_then(|s| s.1).map(format_dt),
        cancel_at: sub.as_ref().and_then(|s| s.2).map(format_dt),
        storage_used_bytes: row.4,
        storage_limit_bytes: bytes_or_unlimited(limits.storage_bytes),
        lifetime_storage_bytes: row.7,
        devices_used: row.5,
        devices_limit: devices_or_unlimited(limits.devices),
        first_pro_at: first_pro_at.map(format_dt),
        saves_used: saves_used as i32,
        saves_limit: limits.saves_tracked.map(|n| n as i32).unwrap_or(-1),
        version_history_forever: limits.version_history_forever,
        max_save_size_bytes: bytes_or_unlimited(limits.max_save_size_bytes),
        storage_status: storage_status(
            plan,
            row.4,
            limits.storage_bytes,
            pending_change_at.is_some(),
        ),
        bandwidth_window_secs: limits.bandwidth_window_secs as i32,
        bandwidth_quota_bytes: bytes_or_unlimited(limits.bandwidth_quota_bytes),
        pending_storage_limit_bytes: pending_limit,
        storage_limit_change_at: pending_change_at.map(format_dt),
        deleted_at: deleted_at.map(format_dt),
        purges_at: purges_at.map(format_dt),
        max_versions: row.12.map(|n| n as i64),
        max_manual_versions: row.13.map(|n| n as i64),
        terms_version_accepted: accepted_terms,
        terms_version_current: hoard_core::wire::TERMS_VERSION,
    }))
}

#[derive(Debug, Deserialize)]
pub struct MaxVersionsBody {
    /// `null` clears the cap (unlimited).
    pub max_versions: Option<i64>,
    /// Which cap this is about: `true` for the deliberate copies, `false` (the
    /// default) for the automatic ones. One endpoint for both, because the
    /// validation, the prune and the `dry_run` are identical.
    #[serde(default)]
    pub manual: bool,
    /// When true, nothing is written or deleted: `pruned` reports how many
    /// versions the given cap WOULD delete. The client shows that number in
    /// a confirmation dialog before committing the real call.
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Serialize)]
pub struct MaxVersionsOut {
    pub max_versions: Option<i64>,
    /// Which cap was just touched, echoed back from the request.
    pub manual: bool,
    /// Versions deleted right away because they were over the new cap (or,
    /// on `dry_run`, how many would be).
    pub pruned: usize,
}

/// `PUT /v1/me/max-versions`: set, or clear with `null`, the per-user cap
/// on stored versions per save, then prune immediately so the effect (and
/// the freed storage) is visible without waiting for the next backup. With
/// `dry_run: true` it only previews the prune count.
pub async fn set_max_versions(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
    Json(body): Json<MaxVersionsBody>,
) -> Result<Json<MaxVersionsOut>, CloudError> {
    if let Some(n) = body.max_versions {
        if !(1..=10_000).contains(&n) {
            return Err(CloudError::BadRequest(
                "max_versions must be between 1 and 10000".into(),
            ));
        }
    }

    if body.dry_run {
        // Clearing the cap never prunes; only a concrete number needs a count.
        let pruned = match body.max_versions {
            Some(n) => {
                crate::cloud::purge::count_version_cap_excess(&state, user.user_id, n, body.manual)
                    .await?
            }
            None => 0,
        };
        return Ok(Json(MaxVersionsOut {
            max_versions: body.max_versions,
            manual: body.manual,
            pruned: pruned.max(0) as usize,
        }));
    }

    // The column is picked by the flag, not by interpolation: two literal queries
    // rather than stitching a column name into the SQL.
    let sql = if body.manual {
        "UPDATE profiles SET max_manual_versions = $1, updated_at = now() WHERE user_id = $2"
    } else {
        "UPDATE profiles SET max_versions = $1, updated_at = now() WHERE user_id = $2"
    };
    sqlx::query(sql)
        .bind(body.max_versions.map(|n| n as i32))
        .bind(user.user_id)
        .execute(&state.pool)
        .await?;

    let pruned = crate::cloud::purge::prune_version_caps(&state, user.user_id).await?;
    Ok(Json(MaxVersionsOut {
        max_versions: body.max_versions,
        manual: body.manual,
        pruned,
    }))
}

/// Map u64 byte caps to the `-1 = unlimited` convention the client uses.
/// We never overflow i64 here in practice (quotas are GB-scale) but
/// the `try_into` guard keeps the intent obvious.
fn bytes_or_unlimited(n: u64) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// `u32::MAX` is the sentinel for unlimited devices (Pro); surface it
/// to the wire as `-1` so the desktop UI can render `∞` without a magic
/// large number drifting through the front-end format helpers.
fn devices_or_unlimited(n: u32) -> i32 {
    if n == u32::MAX {
        -1
    } else {
        n as i32
    }
}

fn format_dt(dt: OffsetDateTime) -> String {
    dt.format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}

/// Create the row if missing. Same effect as the old `POST /v1/profiles/sync`
/// from the handoff but folded into `GET /v1/me` to keep the client API
/// flat: the first authenticated GET is always the bootstrap.
///
/// First provisioning runs the anti-abuse gates (disposable domain, one live
/// account per canonical email, per-device free-account cap). Returning users
/// (any row already present) skip every gate: a miscount or a newly added
/// rule must never lock someone out of an account they already hold.
async fn upsert_profile_for(
    state: &CloudState,
    user: &CloudUser,
    headers: &HeaderMap,
) -> Result<(), CloudError> {
    let canonical = abuse::canonicalize_email(&user.email);

    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM profiles WHERE user_id = $1)")
            .bind(user.user_id)
            .fetch_one(&state.pool)
            .await?;

    if !exists {
        if abuse::is_disposable_email(&user.email) {
            return Err(CloudError::Forbidden(
                "disposable email addresses aren't allowed for Hoard Cloud",
            ));
        }
        if let Some(canon) = canonical.as_deref() {
            let dup: Option<Uuid> = sqlx::query_scalar(
                "SELECT user_id FROM profiles
                  WHERE email_canonical = $1 AND deleted_at IS NULL
                  LIMIT 1",
            )
            .bind(canon)
            .fetch_optional(&state.pool)
            .await?;
            if dup.is_some() {
                return Err(CloudError::Forbidden(
                    "an account already exists for this email address",
                ));
            }
        }
        enforce_device_account_cap(state, headers).await?;
    }

    // Persist the OAuth provider's name + avatar so /v1/me can return them
    // (this is why the desktop "account photo" was always blank: we only
    // ever wrote the email). COALESCE keeps any value already on the row when
    // a later token happens to omit the metadata, so the picture (and the
    // canonical email) doesn't flicker away on a refresh whose token lacks it.
    let res = sqlx::query(
        "INSERT INTO profiles (user_id, email, email_canonical, display_name, avatar_url)
             VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (user_id) DO UPDATE SET
             email           = EXCLUDED.email,
             email_canonical = COALESCE(EXCLUDED.email_canonical, profiles.email_canonical),
             display_name    = COALESCE(EXCLUDED.display_name, profiles.display_name),
             avatar_url      = COALESCE(EXCLUDED.avatar_url, profiles.avatar_url)",
    )
    .bind(user.user_id)
    .bind(&user.email)
    .bind(&canonical)
    .bind(&user.display_name)
    .bind(&user.avatar_url)
    .execute(&state.pool)
    .await;

    // The pre-check above closes the common case; this catches the tiny race
    // where two new accounts claim the same canonical concurrently. Surface it
    // as a clean 403 instead of a 500.
    if let Err(sqlx::Error::Database(ref db)) = res {
        if db.constraint() == Some("uq_profiles_email_canonical_live") {
            return Err(CloudError::Forbidden(
                "an account already exists for this email address",
            ));
        }
    }
    res?;
    Ok(())
}

/// Reject creation of a *new* free account when this device already hosts the
/// per-device cap of distinct live free accounts. Pro accounts aren't counted
/// and are never blocked. A client that sends no fingerprint (older builds, or
/// a machine with neither `/etc/machine-id` nor a hostname) is not gated: the
/// cap is a speed bump for casual multi-accounting, and we'd rather under-block
/// than lock out a real user whose machine reports no stable id.
async fn enforce_device_account_cap(
    state: &CloudState,
    headers: &HeaderMap,
) -> Result<(), CloudError> {
    let fp = headers
        .get("x-hoard-device-fp")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let fp = match fp {
        Some(fp) => fp,
        None => return Ok(()),
    };

    let count: i64 = sqlx::query_scalar(
        "SELECT count(DISTINCT d.user_id)
           FROM devices d
           JOIN profiles p ON p.user_id = d.user_id
          WHERE d.fingerprint = $1
            AND p.deleted_at IS NULL
            AND p.plan = 'free'",
    )
    .bind(fp)
    .fetch_one(&state.pool)
    .await?;

    if count >= abuse::MAX_FREE_ACCOUNTS_PER_DEVICE {
        // Coded so the client can localize it; the message is the English
        // fallback older clients render verbatim.
        return Err(CloudError::ForbiddenCode {
            code: "device_free_cap",
            message: "Sorry — it's not you. Hoard's Free plan is a gift, and to keep \
                      giving it away we can't allow several accounts on the same device. \
                      Upgrade to Pro to add another account here.",
        });
    }
    Ok(())
}

/// Upsert the calling machine into `devices` and recompute the cached
/// `profiles.devices_count`. Keyed on `(user_id, fingerprint)` so re-opening
/// the app on the same machine bumps `last_seen_at` instead of inflating the
/// count. No-op when the client sends no fingerprint header (older builds, or
/// a machine with neither `/etc/machine-id` nor a hostname). The device limit
/// is *not* enforced here. We only keep the count truthful; gating uploads on
/// it is a separate decision so a miscount can never lock a user out.
async fn register_device(
    state: &CloudState,
    user: &CloudUser,
    headers: &HeaderMap,
) -> Result<(), CloudError> {
    let header = |k: &str| headers.get(k).and_then(|v| v.to_str().ok()).map(str::trim);
    let fingerprint = match header("x-hoard-device-fp").filter(|s| !s.is_empty()) {
        Some(fp) => fp,
        None => return Ok(()),
    };
    let name = header("x-hoard-device-name")
        .filter(|s| !s.is_empty())
        .unwrap_or("Unknown device");
    let os = header("x-hoard-device-os").filter(|s| !s.is_empty());
    let app_version = header("x-hoard-app-version").filter(|s| !s.is_empty());

    sqlx::query(
        "INSERT INTO devices (user_id, device_name, device_kind, os, fingerprint, app_version)
             VALUES ($1, $2, 'desktop', $3, $4, $5)
         ON CONFLICT (user_id, fingerprint)
         DO UPDATE SET last_seen_at = now(),
                       device_name  = EXCLUDED.device_name,
                       os           = EXCLUDED.os,
                       app_version  = COALESCE(EXCLUDED.app_version, devices.app_version)",
    )
    .bind(user.user_id)
    .bind(name)
    .bind(os)
    .bind(fingerprint)
    .bind(app_version)
    .execute(&state.pool)
    .await?;

    sqlx::query(
        "UPDATE profiles
            SET devices_count = (SELECT count(*) FROM devices WHERE user_id = $1)
          WHERE user_id = $1",
    )
    .bind(user.user_id)
    .execute(&state.pool)
    .await?;
    Ok(())
}

/// A device counts as online while its last heartbeat is younger than this.
/// The agent beats every ~30s, so 90s = three missed beats before the dot
/// goes grey: tight enough to feel live, loose enough to ride out a hiccup.
const PRESENCE_ONLINE_WINDOW_SECS: i32 = 90;

/// One game a device is running right now: `{ slug, since }`, `since` in
/// RFC3339. Wire shape of both the `devices.playing` JSONB elements and the
/// `DeviceOut.playing` entries, stored and served identically on purpose.
#[derive(Debug, Serialize, serde::Deserialize)]
pub struct PlayingOut {
    pub slug: String,
    pub since: Option<String>,
}

/// One row of the account page's device list / the Eye panel.
#[derive(Debug, Serialize)]
pub struct DeviceOut {
    pub id: Uuid,
    pub device_name: String,
    pub device_kind: Option<String>,
    pub os: Option<String>,
    pub last_seen_at: Option<String>,
    pub created_at: Option<String>,
    /// Live presence: heartbeat fresh and no closing beat received.
    pub online: bool,
    /// Games the device is running right now, most recently started first.
    /// Only ever populated while `online`: a stale "playing" on a dead
    /// machine is worse than none. Empty = idle.
    pub playing: Vec<PlayingOut>,
    /// True for the row matching the caller's `x-hoard-device-fp`, so the UI
    /// can split "this machine" from the rest without knowing its own UUID.
    pub this_device: bool,
}

#[derive(Debug, Serialize)]
pub struct DeviceListOut {
    pub devices: Vec<DeviceOut>,
}

/// GET /v1/devices: the machines registered to this account, newest-seen
/// first. Powers the account page's "Dispositivos" list + unlink buttons.
pub async fn list_devices(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
    headers: HeaderMap,
) -> Result<Json<DeviceListOut>, CloudError> {
    let caller_fp = headers
        .get("x-hoard-device-fp")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .unwrap_or("");

    // `online` is computed here, at read time, so a client that died without
    // its closing beat ages out on its own. `playing` is masked for offline
    // devices in the same breath. The JSONB comes out as text and is parsed
    // in Rust, so no sqlx `json` feature is needed.
    let rows: Vec<(
        Uuid,
        String,
        Option<String>,
        Option<String>,
        Option<time::OffsetDateTime>,
        Option<time::OffsetDateTime>,
        bool,
        Option<String>,
        String,
    )> = sqlx::query_as(
        "SELECT id, device_name, device_kind, os, last_seen_at, created_at,
                (closed_at IS NULL
                 AND last_seen_at > now() - make_interval(secs => $2)) AS online,
                playing::text, fingerprint
           FROM devices WHERE user_id = $1
          ORDER BY last_seen_at DESC NULLS LAST",
    )
    .bind(user.user_id)
    .bind(f64::from(PRESENCE_ONLINE_WINDOW_SECS))
    .fetch_all(&state.pool)
    .await?;

    let devices = rows
        .into_iter()
        .map(
            |(
                id,
                device_name,
                device_kind,
                os,
                last_seen_at,
                created_at,
                online,
                playing,
                fingerprint,
            )| DeviceOut {
                id,
                device_name,
                device_kind,
                os,
                last_seen_at: last_seen_at.map(format_dt),
                created_at: created_at.map(format_dt),
                online,
                playing: if online {
                    playing
                        .as_deref()
                        .and_then(|j| serde_json::from_str(j).ok())
                        .unwrap_or_default()
                } else {
                    Vec::new()
                },
                this_device: !caller_fp.is_empty() && fingerprint == caller_fp,
            },
        )
        .collect();
    Ok(Json(DeviceListOut { devices }))
}

/// DELETE /v1/devices/:id: unlink a device. Scoped to the caller's user_id so
/// you can never delete another account's row even with a guessed UUID. After
/// the delete we recompute the cached `profiles.devices_count`. Deleting a
/// machine that's still running just means it re-registers on its next
/// `GET /v1/me`; that's intended (it's an "unlink", not a permanent ban).
pub async fn delete_device(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
    Path(device_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<DeviceListOut>, CloudError> {
    sqlx::query("DELETE FROM devices WHERE id = $1 AND user_id = $2")
        .bind(device_id)
        .bind(user.user_id)
        .execute(&state.pool)
        .await?;

    sqlx::query(
        "UPDATE profiles
            SET devices_count = (SELECT count(*) FROM devices WHERE user_id = $1)
          WHERE user_id = $1",
    )
    .bind(user.user_id)
    .execute(&state.pool)
    .await?;

    list_devices(State(state), Extension(user), headers).await
}

/// One game in a heartbeat: slug + how long it's been running. A duration
/// rather than a timestamp on purpose: the server anchors it to its own clock
/// (`now() - secs`), so a client with a skewed clock can never report
/// "playing since three minutes from now".
#[derive(Debug, Deserialize)]
pub struct PlayingIn {
    pub slug: String,
    #[serde(default)]
    pub for_secs: Option<i64>,
}

/// Body of `POST /v1/presence/heartbeat`. Everything is optional so an idle
/// keepalive is just `{}`.
#[derive(Debug, Deserialize)]
pub struct HeartbeatIn {
    /// Games running right now, most recently started first. Empty = idle.
    #[serde(default)]
    pub playing: Vec<PlayingIn>,
    /// Final beat on graceful shutdown: marks the device offline immediately
    /// instead of letting it age out of the 90s window.
    #[serde(default)]
    pub closing: bool,
}

/// Hard caps on what a beat may claim. Presence is cosmetic, shown only to the
/// user's own other devices, so these just stop a rogue client from
/// stuffing megabytes into the row.
const MAX_PLAYING_GAMES: usize = 8;
const MAX_SLUG_CHARS: usize = 128;

/// POST /v1/presence/heartbeat: presence keepalive from the agent. Sent every
/// ~30s while it runs, immediately on any game start/stop, and once with
/// `closing: true` on quit. Bumps `last_seen_at` and the `playing` JSONB;
/// `GET /v1/devices` turns that into the Eye panel's online dots. Best-effort
/// by design: a client without a fingerprint header (older builds) is a
/// no-op, never an error.
pub async fn heartbeat(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
    headers: HeaderMap,
    Json(body): Json<HeartbeatIn>,
) -> Result<StatusCode, CloudError> {
    let fp = match headers
        .get("x-hoard-device-fp")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(fp) => fp.to_owned(),
        None => return Ok(StatusCode::NO_CONTENT),
    };

    if body.closing {
        sqlx::query(
            "UPDATE devices
                SET last_seen_at = now(), closed_at = now(), playing = NULL
              WHERE user_id = $1 AND fingerprint = $2",
        )
        .bind(user.user_id)
        .bind(&fp)
        .execute(&state.pool)
        .await?;
        return Ok(StatusCode::NO_CONTENT);
    }

    // Each game's `since` is anchored once, on the beat where its slug first
    // appears; keepalives for a game already in the stored array keep its
    // stored `since`, so the elapsed time in the Eye panel doesn't jitter
    // with every beat. Read-modify-write is race-free in practice: exactly
    // one agent (and thus one presence task) runs per machine.
    let stored: Option<Option<String>> = sqlx::query_scalar(
        "SELECT playing::text FROM devices WHERE user_id = $1 AND fingerprint = $2",
    )
    .bind(user.user_id)
    .bind(&fp)
    .fetch_optional(&state.pool)
    .await?;

    // First beat from a machine the server has never met (e.g. a headless
    // CLI that authenticated via device pairing and never hit `GET /v1/me`):
    // register it the same way `/v1/me` would; name and os come from the same
    // headers. The register path also keeps `profiles.devices_count` truthful.
    if stored.is_none() {
        register_device(&state, &user, &headers).await?;
    }
    let old: Vec<PlayingOut> = stored
        .flatten()
        .as_deref()
        .and_then(|j| serde_json::from_str(j).ok())
        .unwrap_or_default();

    let now = time::OffsetDateTime::now_utc();
    let playing: Vec<PlayingOut> = body
        .playing
        .iter()
        .take(MAX_PLAYING_GAMES)
        .filter(|g| !g.slug.trim().is_empty())
        .map(|g| {
            let slug: String = g.slug.trim().chars().take(MAX_SLUG_CHARS).collect();
            let since = old
                .iter()
                .find(|o| o.slug == slug)
                .and_then(|o| o.since.clone())
                .unwrap_or_else(|| {
                    let secs = g.for_secs.unwrap_or(0).clamp(0, 365 * 86_400);
                    format_dt(now - time::Duration::seconds(secs))
                });
            PlayingOut {
                slug,
                since: Some(since),
            }
        })
        .collect();
    let playing_json = if playing.is_empty() {
        None
    } else {
        serde_json::to_string(&playing).ok()
    };

    sqlx::query(
        "UPDATE devices
            SET last_seen_at = now(), closed_at = NULL, playing = $3::jsonb
          WHERE user_id = $1 AND fingerprint = $2",
    )
    .bind(user.user_id)
    .bind(&fp)
    .bind(&playing_json)
    .execute(&state.pool)
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Cuerpo de `POST /v1/me/terms`.
#[derive(Debug, Deserialize)]
pub struct AcceptTermsBody {
    /// The version the client showed the user. We do not take it on trust: if an
    /// older binary sends a previous one, recording that as "accepted what is in
    /// force" would be manufacturing false evidence.
    pub version: String,
    /// `desktop` | `web` | `cli`.
    pub source: String,
    /// The app version that collected the click, when the client sends it.
    #[serde(default)]
    pub app_version: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TermsStatusOut {
    /// Última versión aceptada, o `null` si no consta ninguna.
    pub accepted_version: Option<String>,
    pub accepted_at: Option<String>,
    /// The one in force today.
    pub current_version: &'static str,
    /// `true` when the checkbox has to be asked for again.
    pub needs_acceptance: bool,
}

/// Accepted sources. Closed on purpose: the field ends up in a record a judge may
/// read, and "whatever the client sent" is not an answer.
const TERMS_SOURCES: [&str; 3] = ["desktop", "web", "cli"];

/// POST /v1/me/terms: records that this account accepted the Terms.
///
/// Idempotent on `(user_id, version)`: clients call it on every sign-in and the
/// first acceptance is the one that counts, so an `ON CONFLICT DO NOTHING` keeps
/// the original date instead of pushing it forward every time somebody opens the
/// app.
pub async fn accept_terms(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
    Json(body): Json<AcceptTermsBody>,
) -> Result<Json<TermsStatusOut>, CloudError> {
    if body.version != hoard_core::wire::TERMS_VERSION {
        return Err(CloudError::BadRequest(format!(
            "terms version mismatch: client sent {}, current is {}",
            body.version,
            hoard_core::wire::TERMS_VERSION
        )));
    }
    if !TERMS_SOURCES.contains(&body.source.as_str()) {
        return Err(CloudError::BadRequest(format!(
            "unknown terms source: {}",
            body.source
        )));
    }

    sqlx::query(
        "INSERT INTO terms_acceptances (user_id, version, source, app_version)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (user_id, version) DO NOTHING",
    )
    .bind(user.user_id)
    .bind(&body.version)
    .bind(&body.source)
    .bind(body.app_version.as_deref())
    .execute(&state.pool)
    .await?;

    terms_status(&state, user.user_id).await.map(Json)
}

/// GET /v1/me/terms: what this account accepted and whether it has to be asked
/// again. The user can request this record under GDPR article 15, and this is what
/// they are handed.
pub async fn get_terms(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
) -> Result<Json<TermsStatusOut>, CloudError> {
    terms_status(&state, user.user_id).await.map(Json)
}

async fn terms_status(state: &CloudState, user_id: Uuid) -> Result<TermsStatusOut, CloudError> {
    let row: Option<(String, OffsetDateTime)> = sqlx::query_as(
        "SELECT version, accepted_at FROM terms_acceptances
          WHERE user_id = $1 ORDER BY accepted_at DESC LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await?;
    let accepted_version = row.as_ref().map(|r| r.0.clone());
    Ok(TermsStatusOut {
        needs_acceptance: accepted_version.as_deref() != Some(hoard_core::wire::TERMS_VERSION),
        accepted_at: row.map(|r| format_dt(r.1)),
        accepted_version,
        current_version: hoard_core::wire::TERMS_VERSION,
    })
}

#[derive(Debug, Serialize)]
pub struct ExportJobOut {
    pub job_id: Uuid,
    pub status: String,
}

/// POST /v1/me/export: enqueue an export job. Returns immediately with a
/// `job_id`; the cron / background worker writes the ZIP to R2 and updates
/// the row. Client polls (future endpoint) or watches via realtime.
pub async fn create_export_job(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
) -> Result<Json<ExportJobOut>, CloudError> {
    // Reuse an in-flight job instead of stacking duplicates: tapping "export"
    // twice, or a status poll racing a click, must not spawn N ZIP builds.
    let existing: Option<(Uuid, String)> = sqlx::query_as(
        "SELECT id, status FROM export_jobs
           WHERE user_id = $1 AND status IN ('pending','running')
           ORDER BY requested_at DESC LIMIT 1",
    )
    .bind(user.user_id)
    .fetch_optional(&state.pool)
    .await?;
    if let Some((job_id, status)) = existing {
        return Ok(Json(ExportJobOut { job_id, status }));
    }

    let row: (Uuid, String) = sqlx::query_as(
        "INSERT INTO export_jobs (user_id, status) VALUES ($1, 'pending')
         RETURNING id, status",
    )
    .bind(user.user_id)
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(ExportJobOut {
        job_id: row.0,
        status: row.1,
    }))
}

/// Wire shape for `GET /v1/me/export`: the latest export job's state, so the
/// account page can show a spinner then a download button without email. All
/// fields are `null` when the user has never requested an export.
#[derive(Debug, Serialize)]
pub struct ExportStatusOut {
    pub job_id: Option<Uuid>,
    /// `pending` | `running` | `done` | `failed` | `expired`, or `null` if none.
    pub status: Option<String>,
    pub requested_at: Option<String>,
    pub size_bytes: Option<i64>,
    pub expires_at: Option<String>,
    /// Presigned R2 GET, only present when the job is `done` and unexpired.
    pub download_url: Option<String>,
    pub error: Option<String>,
}

/// GET /v1/me/export: status of the most recent export, with a fresh
/// presigned download link when one is ready. Polled by the client after it
/// enqueues an export.
pub async fn get_export_status(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
) -> Result<Json<ExportStatusOut>, CloudError> {
    let row: Option<(
        Uuid,
        String,
        OffsetDateTime,
        Option<i64>,
        Option<OffsetDateTime>,
        Option<String>,
        Option<String>,
    )> = sqlx::query_as(
        "SELECT id, status, requested_at, size_bytes, expires_at, r2_key, error
           FROM export_jobs WHERE user_id = $1
          ORDER BY requested_at DESC LIMIT 1",
    )
    .bind(user.user_id)
    .fetch_optional(&state.pool)
    .await?;

    let Some((job_id, status, requested_at, size_bytes, expires_at, r2_key, error)) = row else {
        return Ok(Json(ExportStatusOut {
            job_id: None,
            status: None,
            requested_at: None,
            size_bytes: None,
            expires_at: None,
            download_url: None,
            error: None,
        }));
    };

    // Presign a short-lived link only for a ready, unexpired object.
    let download_url = if status == "done" {
        match (r2_key.filter(|k| !k.is_empty()), expires_at) {
            (Some(key), Some(exp)) if exp > OffsetDateTime::now_utc() => state
                .r2
                .presign_get(&key, Some(std::time::Duration::from_secs(3600)))
                .await
                .ok()
                .map(|u| u.url),
            _ => None,
        }
    } else {
        None
    };

    Ok(Json(ExportStatusOut {
        job_id: Some(job_id),
        status: Some(status),
        requested_at: Some(format_dt(requested_at)),
        size_bytes,
        expires_at: expires_at.map(format_dt),
        download_url,
        error,
    }))
}

/// Days a soft-deleted account is kept, frozen, before the purge cron
/// hard-deletes it. Single-sourced here so `delete_me`, the `Me` payload's
/// `purges_at`, and `account_purge`'s cutoff can't drift apart.
pub const GRACE_DAYS: u32 = 30;

#[derive(Debug, Serialize)]
pub struct DeleteAccountOut {
    pub deleted_at: String,
    pub purges_after: String,
    pub grace_days: u32,
}

/// DELETE /v1/me: soft-delete the account. The account is *frozen* immediately
/// (every data route 403s via `require_active_account`), so unlike before this
/// no longer behaves like a plain logout. `account_purge`'s daily cron
/// hard-deletes it `GRACE_DAYS` later; `POST /v1/me/reactivate` cancels it in
/// the meantime. Mounted on the auth-only router so the freeze doesn't block
/// the user from un-deleting.
pub async fn delete_me(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
) -> Result<Json<DeleteAccountOut>, CloudError> {
    let now = OffsetDateTime::now_utc();
    let purge_at = now + time::Duration::days(GRACE_DAYS as i64);
    // Idempotent: re-deleting an already-deleted account keeps the *original*
    // `deleted_at` (COALESCE), so tapping delete twice can't quietly extend the
    // grace window and postpone the purge.
    sqlx::query(
        "UPDATE profiles SET deleted_at = COALESCE(deleted_at, now()), updated_at = now()
             WHERE user_id = $1",
    )
    .bind(user.user_id)
    .execute(&state.pool)
    .await?;
    sqlx::query(
        "INSERT INTO audit_log (user_id, actor, event_type, metadata)
             VALUES ($1, 'user', 'account.soft_deleted', NULL)",
    )
    .bind(user.user_id)
    .execute(&state.pool)
    .await?;
    Ok(Json(DeleteAccountOut {
        deleted_at: format_dt(now),
        purges_after: format_dt(purge_at),
        grace_days: GRACE_DAYS,
    }))
}

#[derive(Debug, Serialize)]
pub struct ReactivateOut {
    pub reactivated: bool,
}

/// POST /v1/me/reactivate: cancel a pending soft-delete. Clears `deleted_at`
/// (the purge cron only ever touches rows where it's non-NULL, so this is what
/// actually saves the account) and lifts the freeze. Explicit on purpose: a
/// mere re-login must NOT silently un-delete: that was the old bug that made
/// "delete" indistinguishable from a logout. `reactivated` is `false` when the
/// account wasn't scheduled for deletion (nothing to do), so the client can
/// tell an idempotent no-op from a real reactivation.
pub async fn reactivate_me(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
) -> Result<Json<ReactivateOut>, CloudError> {
    let res = sqlx::query(
        "UPDATE profiles SET deleted_at = NULL, updated_at = now()
             WHERE user_id = $1 AND deleted_at IS NOT NULL",
    )
    .bind(user.user_id)
    .execute(&state.pool)
    .await?;
    let reactivated = res.rows_affected() > 0;
    if reactivated {
        sqlx::query(
            "INSERT INTO audit_log (user_id, actor, event_type, metadata)
                 VALUES ($1, 'user', 'account.reactivated', NULL)",
        )
        .bind(user.user_id)
        .execute(&state.pool)
        .await?;
    }
    Ok(Json(ReactivateOut { reactivated }))
}

/// Used by tests on the `quota::QuotaInfo` shape. Keeps the symbol in
/// scope so the file compiles cleanly without unused-import lints in the
/// release build.
#[allow(dead_code)]
fn _quota_shape_check(_i: &quota::QuotaInfo) {}
