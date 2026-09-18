//! Who has already been told what.
//!
//! Every notice in `cloud::email` is worth sending once and unbearable sent
//! repeatedly: `maybe_purge` runs on every upload and the "you are full" 402
//! comes back on every retry. [`claim`] is the gate. It returns `true` to
//! exactly one caller, by letting the database decide through a primary-key
//! conflict rather than a read-then-write nobody can make atomic.
//!
//! The mirror image is [`clear`], called where the condition stops being true.
//! Without it a notice fires once in a user's lifetime, which is worse than
//! firing too often: the second time they fill their account, nobody warns them.

use sqlx::PgPool;
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
pub async fn claim(pool: &PgPool, user_id: Uuid, kind: Kind, scope: &str) -> Result<bool, sqlx::Error> {
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
pub async fn clear(pool: &PgPool, user_id: Uuid, kind: Kind, scope: &str) -> Result<(), sqlx::Error> {
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

/// Account-wide notices carry no scope.
pub const ACCOUNT: &str = "";
