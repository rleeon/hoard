//! Snapshot retention policy: age-weighted (GFS) pruning.
//!
//! Implements eje B of [ADR 0018](../../docs/decisions/0018-storage-efficiency-dedup-retention.md):
//! instead of keeping every snapshot forever, thin them so recent history
//! stays dense and old history gets sparse. The policy is driven by the
//! "ahorro de datos" knob (`data_saving ∈ [0,1]`): higher = keep fewer.
//!
//! The decision logic ([`plan_prune`]) is a pure function over snapshot
//! metadata so it's exhaustively testable; the actual soft-delete (move to
//! `trash/`, decrement quota) happens in `cleanup.rs` and reuses the
//! existing snapshot deletion path. Nothing is hard-deleted here: pruned
//! snapshots go to the trash and are purged later by `trash_retention_days`.

/// Per-snapshot metadata needed to decide retention. Storefront/path details
/// are irrelevant to the decision, so they're left out.
#[derive(Debug, Clone)]
pub struct SnapshotMeta {
    pub id: String,
    /// `created_at` as a Unix timestamp (seconds). Caller parses the stored
    /// RFC3339 string.
    pub created_unix: i64,
    /// Pinned snapshots are never pruned and don't count against `byte_cap`.
    pub is_pinned: bool,
    pub size_bytes: i64,
}

/// Age-weighted retention policy. See [`RetentionPolicy::from_data_saving`]
/// for how the "ahorro de datos" slider maps onto these numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetentionPolicy {
    /// Always keep the N most recent snapshots verbatim.
    pub keep_recent: usize,
    /// Keep one snapshot per hour for the most recent N distinct hours.
    pub keep_hourly: usize,
    /// Keep one snapshot per day for the most recent N distinct days.
    pub keep_daily: usize,
    /// Keep one snapshot per week for the most recent N distinct weeks.
    pub keep_weekly: usize,
    /// Optional hard ceiling on total retained bytes per save. After the
    /// age-weighted pass, the oldest non-pinned survivors are pruned until
    /// the kept set fits under this. `None` disables the cap.
    pub byte_cap: Option<i64>,
}

/// Linear interpolation: `k=0 → a`, `k=1 → b`.
fn lerp(k: f64, a: f64, b: f64) -> f64 {
    a + (b - a) * k
}

impl RetentionPolicy {
    /// Map the user-facing `data_saving` knob `k ∈ [0,1]` onto concrete
    /// keep-counts (ADR 0018, decision 4). `k=0` is roughly keep a lot (close to the
    /// pre-0018 "keep everything" feel); `k=1` is roughly keep only the essentials.
    /// `byte_cap` stays `None` here; it is opt-in via config or plan.
    pub fn from_data_saving(k: f64) -> Self {
        let k = k.clamp(0.0, 1.0);
        let r = |a: f64, b: f64| lerp(k, a, b).round().max(1.0) as usize;
        RetentionPolicy {
            keep_recent: r(20.0, 3.0),
            keep_hourly: r(24.0, 6.0),
            keep_daily: r(14.0, 3.0),
            keep_weekly: r(8.0, 2.0),
            byte_cap: None,
        }
    }
}

impl Default for RetentionPolicy {
    /// Default knob position is `k = 0.3`, a little saving out of the box,
    /// because "keep everything" surprises users badly (the OpenTTD case
    /// that motivated ADR 0018).
    fn default() -> Self {
        Self::from_data_saving(0.3)
    }
}

/// Decide which snapshots to prune for a single save.
///
/// Borg-style grandfather-father-son: a snapshot is kept if it's among the
/// `keep_recent` newest, or it's the newest snapshot within one of the most
/// recent `keep_hourly` hours / `keep_daily` days / `keep_weekly` weeks.
/// Everything else is returned for pruning, **except**:
///
/// - Pinned snapshots are never pruned.
/// - The single newest snapshot is never pruned (belt-and-suspenders; with
///   any sane `keep_recent ≥ 1` it's already covered).
///
/// If `byte_cap` is set and the kept set still exceeds it, the oldest
/// non-pinned survivors are pruned until it fits (never the newest).
///
/// Input order doesn't matter; the function sorts internally. Returns the
/// ids to soft-delete.
pub fn plan_prune(snapshots: &[SnapshotMeta], policy: &RetentionPolicy) -> Vec<String> {
    if snapshots.len() <= 1 {
        return Vec::new();
    }

    // Newest first.
    let mut ordered: Vec<&SnapshotMeta> = snapshots.iter().collect();
    ordered.sort_by_key(|s| std::cmp::Reverse(s.created_unix));

    let newest_id = ordered[0].id.clone();

    use std::collections::HashSet;
    let mut kept: HashSet<&str> = HashSet::new();

    // Pinned + the absolute newest are always kept.
    for s in &ordered {
        if s.is_pinned {
            kept.insert(s.id.as_str());
        }
    }
    kept.insert(newest_id.as_str());

    // keep_recent: the N newest verbatim.
    for s in ordered.iter().take(policy.keep_recent) {
        kept.insert(s.id.as_str());
    }

    // Period rules: walking newest→oldest, the first snapshot seen in each
    // distinct period is the one we keep, up to `count` distinct periods.
    const HOUR: i64 = 3600;
    const DAY: i64 = 86_400;
    const WEEK: i64 = 7 * 86_400;
    for (bucket, count) in [
        (HOUR, policy.keep_hourly),
        (DAY, policy.keep_daily),
        (WEEK, policy.keep_weekly),
    ] {
        let mut seen: HashSet<i64> = HashSet::new();
        for s in &ordered {
            let period = s.created_unix.div_euclid(bucket);
            if seen.insert(period) {
                if seen.len() <= count {
                    kept.insert(s.id.as_str());
                } else {
                    break;
                }
            }
        }
    }

    // Optional byte cap: drop oldest non-pinned survivors until under cap.
    if let Some(cap) = policy.byte_cap {
        let mut kept_total: i64 = ordered
            .iter()
            .filter(|s| kept.contains(s.id.as_str()))
            .map(|s| s.size_bytes)
            .sum();
        if kept_total > cap {
            // Oldest first among the kept, skipping pinned and the newest.
            for s in ordered.iter().rev() {
                if kept_total <= cap {
                    break;
                }
                if s.is_pinned || s.id == newest_id {
                    continue;
                }
                if kept.remove(s.id.as_str()) {
                    kept_total -= s.size_bytes;
                }
            }
        }
    }

    ordered
        .iter()
        .filter(|s| !kept.contains(s.id.as_str()))
        .map(|s| s.id.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(id: &str, created_unix: i64) -> SnapshotMeta {
        SnapshotMeta {
            id: id.to_string(),
            created_unix,
            is_pinned: false,
            size_bytes: 1_600_000,
        }
    }

    #[test]
    fn empty_and_single_never_prune() {
        let p = RetentionPolicy::default();
        assert!(plan_prune(&[], &p).is_empty());
        assert!(plan_prune(&[snap("a", 100)], &p).is_empty());
    }

    #[test]
    fn keeps_recent_burst_within_keep_recent() {
        // 10 snapshots one minute apart; keep_recent covers all of them.
        let p = RetentionPolicy {
            keep_recent: 20,
            keep_hourly: 0,
            keep_daily: 0,
            keep_weekly: 0,
            byte_cap: None,
        };
        let snaps: Vec<_> = (0..10).map(|i| snap(&format!("v{i}"), i * 60)).collect();
        assert!(plan_prune(&snaps, &p).is_empty());
    }

    #[test]
    fn openttd_like_thinning() {
        // 33 snapshots ~1 min apart (all inside ~1 hour), like the OpenTTD
        // report. Default policy should thin them sharply but keep the
        // newest ones dense.
        let p = RetentionPolicy::default();
        let snaps: Vec<_> = (0..33).map(|i| snap(&format!("v{i}"), i * 60)).collect();
        let pruned = plan_prune(&snaps, &p);
        let kept = snaps.len() - pruned.len();
        assert!(kept < snaps.len(), "should prune something");
        assert!(kept >= p.keep_recent, "keeps at least the recent window");
        // The very newest (highest timestamp) must survive.
        assert!(!pruned.contains(&"v32".to_string()));
    }

    #[test]
    fn never_prunes_pinned_or_newest() {
        let p = RetentionPolicy {
            keep_recent: 1,
            keep_hourly: 0,
            keep_daily: 0,
            keep_weekly: 0,
            byte_cap: None,
        };
        let mut snaps: Vec<_> = (0..5)
            .map(|i| snap(&format!("v{i}"), i * DAY_SECS))
            .collect();
        snaps[1].is_pinned = true; // an old pinned one
        let pruned = plan_prune(&snaps, &p);
        assert!(!pruned.contains(&"v1".to_string()), "pinned kept");
        assert!(!pruned.contains(&"v4".to_string()), "newest kept");
        // Non-pinned middles get pruned.
        assert!(pruned.contains(&"v2".to_string()));
    }

    #[test]
    fn daily_rule_keeps_one_per_day() {
        // 6 snapshots, one per day, older than keep_recent. With keep_daily=3
        // only the 3 most recent days survive (plus newest already in those).
        let p = RetentionPolicy {
            keep_recent: 1,
            keep_hourly: 0,
            keep_daily: 3,
            keep_weekly: 0,
            byte_cap: None,
        };
        let snaps: Vec<_> = (0..6)
            .map(|i| snap(&format!("d{i}"), i * DAY_SECS))
            .collect();
        let pruned = plan_prune(&snaps, &p);
        let kept = snaps.len() - pruned.len();
        assert_eq!(kept, 3, "one per day for 3 days");
    }

    #[test]
    fn byte_cap_drops_oldest_until_under() {
        let p = RetentionPolicy {
            keep_recent: 10,
            keep_hourly: 0,
            keep_daily: 0,
            keep_weekly: 0,
            byte_cap: Some(3_500_000), // ~2 snapshots of 1.6MB
        };
        let snaps: Vec<_> = (0..5).map(|i| snap(&format!("v{i}"), i * 60)).collect();
        let pruned = plan_prune(&snaps, &p);
        // 5 * 1.6MB = 8MB kept by recency, cap allows ~2 → prune the 3 oldest.
        assert_eq!(pruned.len(), 3);
        assert!(
            !pruned.contains(&"v4".to_string()),
            "newest survives the cap"
        );
        assert!(pruned.contains(&"v0".to_string()), "oldest pruned first");
    }

    const DAY_SECS: i64 = 86_400;
}
