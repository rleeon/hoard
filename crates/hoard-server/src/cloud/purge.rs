//! Smart storage purge under quota pressure (cloud only).
//!
//! When a user's deduped footprint crosses a per-plan threshold (80% Free,
//! 90% Pro) we reclaim space by deleting *old* save versions, never the most
//! recent one of any game, never pinned ones, and never below a per-plan floor
//! of kept versions per game (1 Free, 5 Pro).
//!
//! Deletion is **staged by reclaimable size** (biggest first: 500MB → 200MB →
//! 100MB → 50MB → any) so a few deletes free the most space, and **weighted by
//! how much history each game hoards**: the game with the most deletable
//! versions cedes first, so 30 Victoria 3 saves shrink before 8 Factorio ones
//! (≈ "every 3 Victoria deleted, 1 Factorio").
//!
//! The decision ([`plan_quota_purge`]) is a pure function over version
//! metadata so it's exhaustively testable; [`maybe_purge`] does the DB/R2 work
//! and rechecks the *real* footprint between deletes (dedup means a version's
//! reclaimable size is only the bytes of blobs nothing else references).

use crate::cloud::errors::CloudError;
use crate::cloud::plans::{resolved_storage_limit, Plan};
use crate::cloud::routes::saves::release_blobs;
use crate::cloud::state::CloudState;
use uuid::Uuid;

#[allow(clippy::identity_op)]
const MB: i64 = 1024 * 1024;

/// Size tiers (reclaimable bytes) for staged purging: free the biggest
/// reclaimable versions first. The last tier (`0`) means "any size".
const SIZE_TIERS: [i64; 5] = [500 * MB, 200 * MB, 100 * MB, 50 * MB, 0];

/// Fraction of the plan's storage limit at which auto-purge starts (and the
/// UI goes "orange"). Below this, nothing is ever deleted.
pub fn purge_threshold(plan: Plan) -> f64 {
    match plan {
        Plan::Free => 0.80,
        Plan::Pro => 0.90,
    }
}

/// Minimum versions kept per game (the most recent ones), never purged.
pub fn floor_per_game(plan: Plan) -> usize {
    match plan {
        Plan::Free => 1,
        Plan::Pro => 5,
    }
}

/// One committed, live version considered for purging.
#[derive(Debug, Clone)]
pub struct PurgeCandidate {
    /// The save (game) this version belongs to.
    pub save_id: String,
    pub version_num: i64,
    pub created_unix: i64,
    /// Bytes freed if *this* version is deleted now: only blobs no other
    /// version references (refcount == 1). Shared blobs free nothing here.
    pub freeable_bytes: i64,
    /// Pinned versions are never purged and count toward the floor.
    pub is_pinned: bool,
    /// The save's head (latest) version. Never purged; counts toward the floor.
    pub is_head: bool,
}

/// Decide the ordered list of `(save_id, version_num)` to delete to bring
/// `used` back under `target`. Pure: no I/O. Order is the deletion order.
///
/// Invariants: never the head or a pinned version; never drop a game below
/// `floor` kept versions; within a game, always oldest-first.
pub fn plan_quota_purge(
    candidates: &[PurgeCandidate],
    floor: usize,
    used: i64,
    target: i64,
) -> Vec<(String, i64)> {
    use std::collections::{HashMap, VecDeque};

    if used <= target {
        return Vec::new();
    }

    // Group versions per save.
    let mut by_save: HashMap<&str, Vec<&PurgeCandidate>> = HashMap::new();
    for c in candidates {
        by_save.entry(c.save_id.as_str()).or_default().push(c);
    }

    // Per save, build the oldest-first queue of deletable versions: skip head
    // and pinned, and stop once only `floor` versions would remain.
    let mut deletable: HashMap<String, VecDeque<&PurgeCandidate>> = HashMap::new();
    for (save, mut vs) in by_save {
        vs.sort_by_key(|c| std::cmp::Reverse(c.version_num)); // newest first
        let mut retained = vs.len();
        let mut dq = VecDeque::new();
        for c in vs.iter().rev() {
            // oldest -> newest
            if c.is_head || c.is_pinned {
                continue;
            }
            if retained > floor {
                dq.push_back(*c);
                retained -= 1;
            }
        }
        if !dq.is_empty() {
            deletable.insert(save.to_string(), dq);
        }
    }

    let mut remaining = used;
    let mut plan = Vec::new();

    for &tier in SIZE_TIERS.iter() {
        while remaining > target {
            // Pick the save whose oldest deletable qualifies for this tier and
            // that has the most deletable versions left (hoarders cede first).
            let mut best: Option<&str> = None;
            let mut best_count = 0usize;
            for (save, dq) in deletable.iter() {
                let Some(front) = dq.front() else { continue };
                let qualifies = tier == 0 || front.freeable_bytes > tier;
                if qualifies && dq.len() > best_count {
                    best_count = dq.len();
                    best = Some(save.as_str());
                }
            }
            let Some(save) = best.map(str::to_string) else {
                break; // nothing qualifies at this tier
            };
            let dq = deletable.get_mut(&save).expect("save present");
            let c = dq.pop_front().expect("front present");
            plan.push((c.save_id.clone(), c.version_num));
            remaining -= c.freeable_bytes;
        }
        if remaining <= target {
            break;
        }
    }

    plan
}

/// Run a purge for `user_id` if their footprint is over the plan threshold.
/// Returns the number of versions deleted. Best-effort and idempotent: safe to
/// call after every commit.
pub async fn maybe_purge(state: &CloudState, user_id: Uuid) -> Result<usize, CloudError> {
    // Promote a downgrade whose grace window has elapsed before we read the
    // limit. Until then the user keeps their larger limit and nothing here
    // purges.
    crate::cloud::quota::apply_due_downgrade(&state.pool, user_id).await?;
    let row: Option<(String, i64, Option<i64>, Option<time::OffsetDateTime>)> = sqlx::query_as(
        "SELECT plan, storage_bytes, storage_limit_bytes, storage_limit_change_at \
         FROM profiles WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await?;
    let Some((plan_s, used, limit_override, change_at)) = row else {
        return Ok(0);
    };
    let plan = Plan::from_str(&plan_s).unwrap_or(Plan::Free);
    // Resolve against the *granted* limit, not the plan's. A user inside a
    // downgrade grace window keeps their old room, and deleting their history
    // during the very window we promised not to is the whole bug this guards.
    // (`apply_due_downgrade` above already retired any window that came due, so
    // a surviving `change_at` is genuinely still open.)
    let limit = resolved_storage_limit(
        plan,
        limit_override,
        change_at.map(|t| t.unix_timestamp()),
        time::OffsetDateTime::now_utc().unix_timestamp(),
    ) as i64;
    let target = (limit as f64 * purge_threshold(plan)) as i64;
    if used <= target {
        return Ok(0);
    }

    let candidates = load_candidates(state, user_id).await?;
    let plan_list = plan_quota_purge(&candidates, floor_per_game(plan), used, target);
    if plan_list.is_empty() {
        return Ok(0);
    }

    let mut deleted = 0usize;
    for (save_id, version) in plan_list {
        purge_one(state, user_id, &save_id, version).await?;
        deleted += 1;
        // Dedup means the planned freeable is only an estimate; stop as soon as
        // the real footprint is back under target.
        let now: Option<i64> =
            sqlx::query_scalar("SELECT storage_bytes FROM profiles WHERE user_id = $1")
                .bind(user_id)
                .fetch_optional(&state.pool)
                .await?;
        if now.map(|u| u <= target).unwrap_or(true) {
            break;
        }
    }

    tracing::info!(user_id = %user_id, deleted, "quota purge: reclaimed old versions");
    Ok(deleted)
}

/// Load every live, content-addressed version with its reclaimable size.
async fn load_candidates(
    state: &CloudState,
    user_id: Uuid,
) -> Result<Vec<PurgeCandidate>, CloudError> {
    // `freeable` sums the size of this version's distinct blobs that nothing
    // else references (refcount == 1). DISTINCT first so a blob used by several
    // files in the version isn't counted twice.
    let rows: Vec<(String, i64, i64, bool, bool, i64)> = sqlx::query_as(
        r#"
        WITH ver_blobs AS (
            SELECT DISTINCT f.save_id, f.version_num, f.sha256
            FROM manifest_files f
            JOIN saves s ON s.id = f.save_id
            WHERE s.user_id = $1
        ),
        freeable AS (
            SELECT vb.save_id, vb.version_num,
                   COALESCE(SUM(CASE WHEN b.refcount = 1 THEN b.size_bytes ELSE 0 END), 0)::bigint AS bytes
            FROM ver_blobs vb
            JOIN cloud_blobs b ON b.user_id = $1 AND b.sha256 = vb.sha256
            GROUP BY vb.save_id, vb.version_num
        )
        SELECT v.save_id,
               v.version_num,
               EXTRACT(EPOCH FROM v.created_at)::bigint AS created_unix,
               v.is_pinned,
               (v.version_num = s.latest_version_num) AS is_head,
               COALESCE(fr.bytes, 0) AS freeable
        FROM save_versions v
        JOIN saves s ON s.id = v.save_id
        LEFT JOIN freeable fr ON fr.save_id = v.save_id AND fr.version_num = v.version_num
        WHERE s.user_id = $1
          AND v.deleted_at IS NULL
          AND v.sha256 <> ''
          AND v.content_addressed = TRUE
        "#,
    )
    .bind(user_id)
    .fetch_all(&state.pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(save_id, version_num, created_unix, is_pinned, is_head, freeable_bytes)| {
                PurgeCandidate {
                    save_id,
                    version_num,
                    created_unix,
                    freeable_bytes,
                    is_pinned,
                    is_head,
                }
            },
        )
        .collect())
}

/// Count how many versions a hypothetical cap of `cap` would delete for this
/// user, without touching anything. Same predicate as
/// [`prune_version_caps`]; powers the confirmation dialog the panel shows
/// before lowering the cap.
///
/// `manual = true` asks about the deliberate copies' cap, `false` about the
/// automatic ones'. Each class is counted against its own, as in the prune, so the
/// dialog promises exactly what is going to happen.
pub async fn count_version_cap_excess(
    state: &CloudState,
    user_id: Uuid,
    cap: i64,
    manual: bool,
) -> Result<i64, CloudError> {
    let cap = cap.max(1);
    let n: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM save_versions v
          JOIN saves s ON s.id = v.save_id
         WHERE s.user_id = $1
           AND v.sha256 <> ''
           AND v.deleted_at IS NULL
           AND v.is_pinned = FALSE
           AND v.version_num <> s.latest_version_num
           AND (COALESCE(v.notes, '') IN ('manual', 'pre-restore')) = $3
           AND (SELECT COUNT(*) FROM save_versions w
                 WHERE w.save_id = v.save_id
                   AND w.sha256 <> ''
                   AND w.deleted_at IS NULL
                   AND (COALESCE(w.notes, '') IN ('manual', 'pre-restore')) = $3
                   AND w.version_num > v.version_num) >= $2
        "#,
    )
    .bind(user_id)
    .bind(cap)
    .bind(manual)
    .fetch_one(&state.pool)
    .await?;
    Ok(n.max(0))
}

/// Enforce the user's own "max versions per save" caps (`profiles.max_versions`
/// for the automatic ones, `profiles.max_manual_versions` for the ones asked for by
/// hand; NULL means no limit). Unlike the quota purge this is user-opt-in and
/// unconditional: for every save, the oldest committed live versions beyond
/// the newest `cap` are deleted, never the head and never pinned ones. Handles
/// both content-addressed versions (blob release, like [`purge_one`]) and
/// legacy whole-archive ones (R2 object drop, like the manual delete route).
/// Returns how many versions were deleted.
///
/// The two classes are counted separately, and that is the whole point. With a
/// single cap, a game autosaving every minute fills the entire history in one
/// session and takes out the copy somebody deliberately made before a boss.
/// Counted apart, an automatic burst can only displace other automatic ones. Which
/// class a version belongs to is what `notes` says; null means automatic, which is
/// what every row from before this carries (see [`VersionOrigin`]).
pub async fn prune_version_caps(state: &CloudState, user_id: Uuid) -> Result<usize, CloudError> {
    let caps: Option<(Option<i32>, Option<i32>)> =
        sqlx::query_as("SELECT max_versions, max_manual_versions FROM profiles WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(&state.pool)
            .await?;
    let Some((auto_cap, manual_cap)) = caps else {
        return Ok(0);
    };
    if auto_cap.is_none() && manual_cap.is_none() {
        return Ok(0);
    }
    // An unset cap means no limit. It translates to an unreachable number rather
    // than a separate branch, so the query stays a single one.
    let auto_cap = auto_cap.map_or(i64::MAX, |c| i64::from(c).max(1));
    let manual_cap = manual_cap.map_or(i64::MAX, |c| i64::from(c).max(1));

    // Victims: live committed versions with `cap`-or-more newer live committed
    // siblings **de su misma clase** (i.e. outside the newest-`cap` window),
    // excluding pinned rows and the head. The head is always inside the window
    // since cap >= 1, but the explicit guard keeps that invariant even if a
    // stale `latest_version_num` points elsewhere.
    let victims: Vec<(String, i64, bool, String)> = sqlx::query_as(
        r#"
        SELECT v.save_id, v.version_num, v.content_addressed, v.r2_key
          FROM save_versions v
          JOIN saves s ON s.id = v.save_id
         WHERE s.user_id = $1
           AND v.sha256 <> ''
           AND v.deleted_at IS NULL
           AND v.is_pinned = FALSE
           AND v.version_num <> s.latest_version_num
           AND (SELECT COUNT(*) FROM save_versions w
                 WHERE w.save_id = v.save_id
                   AND w.sha256 <> ''
                   AND w.deleted_at IS NULL
                   AND (COALESCE(w.notes, '') IN ('manual', 'pre-restore'))
                     = (COALESCE(v.notes, '') IN ('manual', 'pre-restore'))
                   AND w.version_num > v.version_num)
               >= (CASE WHEN COALESCE(v.notes, '') IN ('manual', 'pre-restore')
                        THEN $3 ELSE $2 END)
      ORDER BY v.save_id, v.version_num ASC
        "#,
    )
    .bind(user_id)
    .bind(auto_cap)
    .bind(manual_cap)
    .fetch_all(&state.pool)
    .await?;

    let mut deleted = 0usize;
    for (save_id, version, content_addressed, r2_key) in victims {
        if content_addressed {
            purge_one(state, user_id, &save_id, version).await?;
        } else {
            if let Err(e) = state.r2.delete_object(&r2_key).await {
                tracing::warn!(error = %e, r2_key = %r2_key, "version cap: R2 object delete failed");
            }
            sqlx::query("DELETE FROM save_versions WHERE save_id = $1 AND version_num = $2")
                .bind(&save_id)
                .bind(version)
                .execute(&state.pool)
                .await?;
        }
        deleted += 1;
    }

    if deleted > 0 {
        tracing::info!(
            user_id = %user_id,
            deleted,
            auto_cap,
            manual_cap,
            "version cap: pruned versions over the per-class caps"
        );
    }
    Ok(deleted)
}

/// Delete one content-addressed version (same path as the manual
/// `delete_version` handler's CA branch), releasing its blob references. We
/// never purge a head, so there's no head to repoint.
async fn purge_one(
    state: &CloudState,
    user_id: Uuid,
    save_id: &str,
    version: i64,
) -> Result<(), CloudError> {
    let shas: Vec<(String,)> = sqlx::query_as(
        "SELECT DISTINCT encode(sha256, 'hex') FROM manifest_files
          WHERE save_id = $1 AND version_num = $2",
    )
    .bind(save_id)
    .bind(version)
    .fetch_all(&state.pool)
    .await?;

    let res = sqlx::query("DELETE FROM save_versions WHERE save_id = $1 AND version_num = $2")
        .bind(save_id)
        .bind(version)
        .execute(&state.pool)
        .await?;

    // Concurrency guard: if another purge/delete path already removed this
    // version, our DELETE affects 0 rows and that other path already released
    // these blob references. Releasing them again would double-decrement the
    // refcount and could evict blobs a live version still points at. Bail.
    if res.rows_affected() == 0 {
        return Ok(());
    }

    release_blobs(state, user_id, shas.into_iter().map(|(s,)| (s, 1))).await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(save: &str, v: i64, freeable_mb: i64) -> PurgeCandidate {
        PurgeCandidate {
            save_id: save.to_string(),
            version_num: v,
            created_unix: v, // monotonic with version for these tests
            freeable_bytes: freeable_mb * MB,
            is_pinned: false,
            is_head: false,
        }
    }

    /// Mark the highest version_num of each save as head (what the DB query does).
    fn with_heads(mut cs: Vec<PurgeCandidate>) -> Vec<PurgeCandidate> {
        use std::collections::HashMap;
        let mut head: HashMap<&str, i64> = HashMap::new();
        for x in &cs {
            let e = head.entry(x.save_id.as_str()).or_insert(x.version_num);
            if x.version_num > *e {
                *e = x.version_num;
            }
        }
        let head: HashMap<String, i64> =
            head.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
        for x in &mut cs {
            x.is_head = head[&x.save_id] == x.version_num;
        }
        cs
    }

    #[test]
    fn nothing_when_under_target() {
        let cs = with_heads(vec![c("a", 1, 100), c("a", 2, 100)]);
        assert!(plan_quota_purge(&cs, 1, 100, 200).is_empty());
    }

    #[test]
    fn never_head_or_pinned_or_below_floor() {
        // One save, 3 versions, floor 1. Only the 2 oldest are deletable; head
        // (v3) survives. Big target forces deleting all it can.
        let cs = with_heads(vec![c("a", 1, 100), c("a", 2, 100), c("a", 3, 100)]);
        let plan = plan_quota_purge(&cs, 1, 10_000 * MB, 0);
        let ids: Vec<i64> = plan.iter().map(|(_, v)| *v).collect();
        assert_eq!(ids, vec![1, 2], "oldest-first, head v3 kept");
    }

    #[test]
    fn floor_five_keeps_five() {
        // 8 versions, floor 5 → only the 3 oldest deletable.
        let cs = with_heads((1..=8).map(|v| c("a", v, 100)).collect());
        let plan = plan_quota_purge(&cs, 5, 10_000 * MB, 0);
        let ids: Vec<i64> = plan.iter().map(|(_, v)| *v).collect();
        assert_eq!(ids, vec![1, 2, 3]);
    }

    #[test]
    fn hoarder_game_cedes_first() {
        // Victoria: 30 versions, Factorio: 8. Same size. Floor 1. The first
        // deletions must skew heavily to Victoria (it has more deletable).
        let mut cs: Vec<PurgeCandidate> = (1..=30).map(|v| c("victoria", v, 100)).collect();
        cs.extend((1..=8).map(|v| c("factorio", v, 100)));
        let cs = with_heads(cs);
        // Target after deleting ~10 of 100MB-each from a 3800MB footprint.
        let used = 38 * 100 * MB;
        let target = used - 10 * 100 * MB; // free ~10 versions
        let plan = plan_quota_purge(&cs, 1, used, target);
        let vic = plan.iter().filter(|(s, _)| s == "victoria").count();
        let fac = plan.iter().filter(|(s, _)| s == "factorio").count();
        assert!(vic > fac, "hoarder (victoria) cedes more: {vic} vs {fac}");
        assert!(
            fac <= 1,
            "factorio barely touched while victoria has surplus"
        );
    }

    #[test]
    fn big_versions_purged_first() {
        // Two saves: one with big old versions, one with small. Big ones should
        // be chosen first (higher size tier), even though counts are equal.
        let mut cs = vec![c("big", 1, 600), c("big", 2, 600)];
        cs.extend([c("small", 1, 10), c("small", 2, 10)]);
        let cs = with_heads(cs);
        let used = (600 + 600 + 10 + 10) * MB;
        let target = used - 500 * MB; // need ~one big deletion
        let plan = plan_quota_purge(&cs, 1, used, target);
        assert_eq!(plan.first().map(|(s, _)| s.as_str()), Some("big"));
    }
}
