//! Self-hosted HTTP routes.
//!
//! # The lenient gate for values coming out of the DB (ADR 0021, C.3)
//!
//! The types in [`hoard_core::wire`] carry the strict gate in `serde`, so nothing
//! poisoned gets *in* over the network. But these responses are built by reading
//! the server's SQLite, which is persisted state with years on it: a row with a
//! slug that would not pass the gate today must be repaired or flagged, never take
//! the request down. Hard rejection here would be exactly the bricking the ADR
//! forbids, only server-side and for all of the user's saves at once.

pub mod admin;
pub mod auth;
pub mod cas;
pub mod devices;
pub mod events;
pub mod games;
pub mod health;
pub mod logs;
pub mod overview;
pub mod panel;
pub mod playtime;
pub mod saves;
pub mod session;
pub mod snapshots;

use hoard_core::ids::{GameSlug, Repair, SaveId, Username};
use time::OffsetDateTime;

/// A slug from a DB row, ready for the wire.
///
/// Valid passes through. Recoverable is re-derived with `slugify`, with a warning
/// so the operator can see they have an old row. Unrecoverable becomes the
/// [`GameSlug::unknown`] marker, which matches no game.
///
/// Note that only what the strict gate rejects goes through [`GameSlug::repair`].
/// A *degenerate* slug (`users`, a Windows account name) is syntactically valid
/// and passes through as-is: it is the client that decides to ignore it for
/// correlation, and changing it here would break the `(user_id, game_slug, label)`
/// identity that save already has in the DB.
pub(crate) fn repair_slug(raw: &str) -> GameSlug {
    match GameSlug::parse(raw) {
        Ok(v) => v,
        Err(e) => match GameSlug::repair(raw) {
            Repair::Clean(v) | Repair::Repaired { value: v, .. } => {
                tracing::warn!(raw, repaired = %v, error = %e, "db: slug reparado al servirlo");
                v
            }
            Repair::Quarantined { reason, .. } => {
                tracing::warn!(raw, %reason, "db: slug irrecuperable, se sirve marcado");
                GameSlug::unknown()
            }
        },
    }
}

/// A username from the `users` table, ready for the wire. Same contract as
/// [`repair_slug`]: repaired or flagged, never a 500 (see [`Username::unknown`]).
pub(crate) fn repair_username(raw: &str) -> Username {
    match Username::parse(raw) {
        Ok(v) => v,
        Err(e) => match Username::repair(raw) {
            Repair::Clean(v) | Repair::Repaired { value: v, .. } => {
                tracing::warn!(error = %e, "db: username reparado al servirlo");
                v
            }
            Repair::Quarantined { reason, .. } => {
                tracing::warn!(%reason, "db: username irrecuperable, se sirve marcado");
                Username::unknown()
            }
        },
    }
}

/// A save id read from the DB. Unlike a slug it can be neither repaired nor
/// flagged: an invented id points at a different save. Ids are PKs the server
/// mints itself (`Uuid::new_v4()`), so a `None` here means a hand-edited DB. It
/// gets logged and the row is skipped, which is the only safe thing to do.
pub(crate) fn parse_save_id(raw: &str) -> Option<SaveId> {
    match SaveId::repair(raw).into_value() {
        Some(v) => Some(v),
        None => {
            tracing::error!(raw, "db: save_id que no es un UUID; fila omitida");
            None
        }
    }
}

/// A timestamp from a TEXT column. The server writes them with
/// `strftime('%Y-%m-%dT%H:%M:%SZ')`, so this only fails on a hand-edited DB: it
/// warns and falls back to the epoch rather than taking the response down.
pub(crate) fn repair_ts(raw: &str) -> OffsetDateTime {
    OffsetDateTime::parse(raw, &time::format_description::well_known::Rfc3339).unwrap_or_else(|e| {
        tracing::warn!(raw, error = %e, "db: timestamp ilegible, se sirve el epoch");
        OffsetDateTime::UNIX_EPOCH
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An old row with a dirty slug gets served repaired rather than taking the
    /// user's listing down.
    #[test]
    fn poisoned_db_rows_are_repaired_not_rejected() {
        assert_eq!(repair_slug("stardew-valley").as_str(), "stardew-valley");
        assert_eq!(repair_slug("GSE Saves").as_str(), "gse-saves");
        assert_eq!(repair_slug("   ").as_str(), "unknown-game");
        // Degenerate but well formed: respected, because it is the identity the
        // save already has in the DB.
        assert_eq!(repair_slug("users").as_str(), "users");

        assert_eq!(repair_username("jacka").as_str(), "jacka");
        assert_eq!(repair_username("  jacka  ").as_str(), "jacka");
        assert_eq!(repair_username("").as_str(), "unknown");
    }
}
