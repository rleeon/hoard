//! Who has already been told what.
//!
//! `maybe_purge` runs on every upload and the 402 comes back on every retry, so
//! the triggers fire far more often than anyone wants to hear from us. This is
//! the gate, and it has two settings.
//!
//! [`claim`] is once: it returns `true` to exactly one caller, by letting the
//! database decide through a primary-key conflict rather than a read-then-write
//! nobody can make atomic. Its mirror is [`clear`], called where the condition
//! stops being true; without that a notice fires once in a user's lifetime,
//! which is worse than firing too often.
//!
//! [`claim_periodic`] is once a day while something keeps happening. The purge
//! is the case for it: history is being deleted again today, and saying so once
//! and never again would be the wrong kind of quiet. The reader ends it with
//! [`mute_by_token`], the link in the email, and the mute lifts by itself once
//! the daily sweep ([`expire_stale`]) drops a row nothing has refreshed.

use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;

/// The notices that keep state. `Kind::as_str` is stored verbatim, so these
/// strings are part of the schema: renaming one re-arms it for every user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Crossed the purge threshold and versions started disappearing.
    StoragePurgeStarted,
    /// Out of room; uploads are being rejected.
    StorageFull,
    /// One save is over the per-game cap. Scoped to the save.
    SaveTooLarge,
    /// An archived save is near the end of its grace window. Scoped to the save.
    ArchiveExpiring,
    /// Every device slot on the plan is taken.
    DevicesFull,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::StoragePurgeStarted => "storage_purge_started",
            Kind::StorageFull => "storage_full",
            Kind::SaveTooLarge => "save_too_large",
            Kind::ArchiveExpiring => "archive_expiring",
            Kind::DevicesFull => "devices_full",
        }
    }
}

/// Take the right to send `kind` to `user_id` about `scope`.
///
/// `true` means "you send it", and the row is already written before the
/// message leaves, so two concurrent uploads cannot both decide to send. A
/// caller whose send then fails is expected to [`clear`] the claim (see
/// `cloud::notify`): nothing was delivered, so there is nothing to duplicate,
/// and the next time the condition comes round the user gets another chance.
///
/// `scope` is a save id for the per-save notices and `""` for account-wide
/// ones. Errors are the caller's to swallow, not to propagate into a user
/// request: not knowing whether we already wrote is a reason to stay quiet.
pub async fn claim(
    pool: &PgPool,
    user_id: Uuid,
    kind: Kind,
    scope: &str,
) -> Result<bool, sqlx::Error> {
    let res = sqlx::query(
        "INSERT INTO email_notices (user_id, kind, scope) VALUES ($1, $2, $3)
         ON CONFLICT (user_id, kind, scope) DO NOTHING",
    )
    .bind(user_id)
    .bind(kind.as_str())
    .bind(scope)
    .execute(pool)
    .await?;
    Ok(res.rows_affected() == 1)
}

/// Re-arm one notice, so the next time the condition comes back the user hears
/// about it. A no-op when it was never sent.
pub async fn clear(
    pool: &PgPool,
    user_id: Uuid,
    kind: Kind,
    scope: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM email_notices WHERE user_id = $1 AND kind = $2 AND scope = $3")
        .bind(user_id)
        .bind(kind.as_str())
        .bind(scope)
        .execute(pool)
        .await?;
    Ok(())
}

/// Re-arm a notice for every scope it was sent about. Used by the account-wide
/// conditions that unblock several games at once, like dropping back under the
/// storage threshold.
pub async fn clear_all_scopes(pool: &PgPool, user_id: Uuid, kind: Kind) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM email_notices WHERE user_id = $1 AND kind = $2")
        .bind(user_id)
        .bind(kind.as_str())
        .execute(pool)
        .await?;
    Ok(())
}

/// Forget everything said about one save. Called when the save is deleted, so
/// a re-added game with the same id starts from silence rather than inheriting
/// warnings about a folder that no longer exists.
pub async fn clear_scope(pool: &PgPool, scope: &str) -> Result<(), sqlx::Error> {
    if scope.is_empty() {
        return Ok(());
    }
    sqlx::query("DELETE FROM email_notices WHERE scope = $1")
        .bind(scope)
        .execute(pool)
        .await?;
    Ok(())
}

/// Take the right to send `kind` again, at most once per `cooldown`.
///
/// Returns the row's mute token when the caller should send, `None` when the
/// notice is muted or was sent too recently. Unlike [`claim`] the row is
/// refreshed rather than blocked, so the message repeats while the condition
/// does, which for the purge is the point: every pass deletes more history.
pub async fn claim_periodic(
    pool: &PgPool,
    user_id: Uuid,
    kind: Kind,
    scope: &str,
    cooldown: Duration,
) -> Result<Option<Uuid>, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO email_notices (user_id, kind, scope) VALUES ($1, $2, $3)
         ON CONFLICT (user_id, kind, scope) DO UPDATE
            SET sent_at = now()
          WHERE email_notices.muted_at IS NULL
            AND email_notices.sent_at < now() - $4::interval
      RETURNING mute_token",
    )
    .bind(user_id)
    .bind(kind.as_str())
    .bind(scope)
    .bind(format!("{} seconds", cooldown.as_secs()))
    .fetch_optional(pool)
    .await
}

/// Silence the notice a mute link belongs to. Returns the kind it muted, or
/// `None` when the token is unknown or already muted, which is also what makes
/// the endpoint safe to call twice.
///
/// The token is the only thing the link carries: it identifies one row, never a
/// user, so a forwarded email cannot be used to learn whose account it was.
pub async fn mute_by_token(pool: &PgPool, token: Uuid) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar(
        "UPDATE email_notices SET muted_at = now()
          WHERE mute_token = $1 AND muted_at IS NULL
      RETURNING kind",
    )
    .bind(token)
    .fetch_optional(pool)
    .await
}

/// Drop periodic rows nothing has refreshed in `after`.
///
/// This is both halves of "it comes back when it stops": an account that has
/// not been purged in a fortnight loses its row, so the next purge is news
/// again, and a mute set during the last bad month expires with it instead of
/// being permanent.
pub async fn expire_stale(pool: &PgPool, kind: Kind, after: Duration) -> Result<u64, sqlx::Error> {
    let res = sqlx::query(
        "DELETE FROM email_notices
          WHERE kind = $1 AND sent_at < now() - $2::interval",
    )
    .bind(kind.as_str())
    .bind(format!("{} seconds", after.as_secs()))
    .execute(pool)
    .await?;
    Ok(res.rows_affected())
}

/// Account-wide notices carry no scope.
pub const ACCOUNT: &str = "";
