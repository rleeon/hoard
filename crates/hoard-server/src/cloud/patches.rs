//! Blobs stored as zstd patches against an earlier blob of the same account.
//!
//! The client makes them (the base as zstd's reference prefix) and decodes
//! them; the server checks bases and keeps them alive. A Fallout 4 save that
//! uploads as 21 MB of zstd goes up as 0.7-1.9 MB (measured oct-2026), and a
//! Stellaris zip, patched inflated (`zstd-patch-inflated`), 8.5 MB as 0.7-1.0.
//! Depth is one: a base is always stored whole, never a patch itself.

use std::collections::HashSet;

use uuid::Uuid;

use crate::cloud::state::CloudState;

pub use crate::patch::ENCODING;

const DEFAULT_MIN_APP_VERSION: &str = "1.3.0";

pub fn enabled(state: &CloudState) -> bool {
    state.config.cloud.as_ref().is_some_and(|c| c.blob_patch)
}

/// May this account upload patches? Only if every device seen in the last 90
/// days can decode them: a version made of patches doesn't restore on one
/// that can't, and the server has no room to rebuild it for that device.
pub async fn account_ok(state: &CloudState, user: Uuid) -> Result<bool, sqlx::Error> {
    let Some(cfg) = state.config.cloud.as_ref().filter(|c| c.blob_patch) else {
        return Ok(false);
    };
    let min = if cfg.patch_min_app_version.is_empty() {
        DEFAULT_MIN_APP_VERSION
    } else {
        cfg.patch_min_app_version.as_str()
    };
    let versions: Vec<Option<String>> = sqlx::query_scalar(
        "SELECT app_version FROM devices
          WHERE user_id = $1 AND last_seen_at > now() - interval '90 days'",
    )
    .bind(user)
    .fetch_all(&state.pool)
    .await?;
    Ok(!versions.is_empty()
        && versions
            .iter()
            .all(|v| v.as_deref().is_some_and(|v| version_at_least(v, min))))
}

/// The shas in `wanted` this account can patch against: stored whole and
/// alive (referenced, or kept for a patch that needs it). With `lock`, the
/// rows stay locked for the caller's transaction, which keeps the GC off them.
pub async fn valid_bases(
    conn: &mut sqlx::PgConnection,
    user: Uuid,
    wanted: &[String],
    lock: bool,
) -> Result<HashSet<String>, sqlx::Error> {
    if wanted.is_empty() {
        return Ok(HashSet::new());
    }
    let sql = format!(
        "SELECT encode(b.sha256, 'hex') FROM cloud_blobs b
          WHERE b.user_id = $1
            AND b.sha256 = ANY(ARRAY(SELECT decode(u, 'hex') FROM unnest($2::text[]) AS u))
            AND b.base_sha256 IS NULL
            AND b.encoding IS DISTINCT FROM 'missing'
            AND (b.refcount > 0 OR EXISTS (
                  SELECT 1 FROM cloud_blobs p
                   WHERE p.user_id = b.user_id AND p.base_sha256 = b.sha256))
          ORDER BY b.sha256 {}",
        if lock { "FOR UPDATE" } else { "" }
    );
    let rows: Vec<String> = sqlx::query_scalar(&sql)
        .bind(user)
        .bind(wanted)
        .fetch_all(&mut *conn)
        .await?;
    Ok(rows.into_iter().collect())
}

pub use crate::patch::apply_files;

/// `1.3.0 >= 1.2.9`, by numbers. Anything that doesn't parse is too old.
pub fn version_at_least(v: &str, min: &str) -> bool {
    fn parts(s: &str) -> Option<[u64; 3]> {
        let core = s.trim().trim_start_matches('v').split(['-', '+']).next()?;
        let mut it = core.split('.');
        let mut out = [0u64; 3];
        for (i, slot) in out.iter_mut().enumerate() {
            match it.next() {
                Some(p) => *slot = p.parse().ok()?,
                None if i > 0 => break,
                None => return None,
            }
        }
        Some(out)
    }
    matches!((parts(v), parts(min)), (Some(a), Some(b)) if a >= b)
}

#[cfg(test)]
mod tests {
    use super::version_at_least;

    #[test]
    fn versions_compare_by_number() {
        assert!(version_at_least("1.3.0", "1.3.0"));
        assert!(version_at_least("1.10.0", "1.3.0"));
        assert!(version_at_least("2.0", "1.3.0"));
        assert!(version_at_least("1.3.1-beta", "1.3.0"));
        assert!(!version_at_least("1.2.9", "1.3.0"));
        assert!(!version_at_least("", "1.3.0"));
        assert!(!version_at_least("dev", "1.3.0"));
    }
}
