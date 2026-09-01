//! Plan limits: the single source of truth for what each tier allows.
//!
//! Hardcoded on purpose. Pricing is an opinion (see ADR 0015); when it
//! changes, update here + the landing + the Polar products and ship a new
//! release. Reading these from the DB would invite drift.
//!
//! Two tiers post-1.6.1: Free and Pro. Pro+ was removed, because the gap
//! between "I play one game" and "I store every save I've ever made"
//! turned out to be smaller than originally guessed, and a third tier
//! complicated pricing copy without a clear customer to sell it to.
//!
//! Version history is forever on both tiers. The retention cron only
//! purges hard-deleted accounts and unreferenced R2 objects; it never
//! ages out a user's snapshots by date.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Plan {
    Free,
    Pro,
}

impl Plan {
    pub fn from_str(s: &str) -> Option<Plan> {
        match s {
            "free" => Some(Plan::Free),
            "pro" => Some(Plan::Pro),
            // Legacy values from the old enum: a user who paid for
            // Pro+ pre-1.6.1 is grandfathered onto Pro (same storage,
            // bandwidth shape; the difference was retention which is
            // now forever for everyone). The migration rewrites stored
            // rows to "pro" so this branch only fires on stale tokens
            // / cached JSON.
            "proplus" | "pro+" | "pro_plus" => Some(Plan::Pro),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Plan::Free => "free",
            Plan::Pro => "pro",
        }
    }

    pub fn limits(self) -> PlanLimits {
        match self {
            Plan::Free => PlanLimits {
                plan: self,
                storage_bytes: 2 * GB,
                devices: 3,
                saves_tracked: None,
                version_history_forever: true,
                max_save_size_bytes: 1 * GB,
                bandwidth_window_secs: 15 * 60,
                // Per-save cap raised 200 MB→1 GB and the 15-min window quota
                // 2→3 GB (2026-07-09). A single large monolithic save (e.g. a
                // ~144 MB Factorio `.zip` that barely dedups) rewritten by the
                // game's autosave a handful of times inside the window was
                // saturating the old quota and tripping a confusing 429 on what
                // the user saw as "one save".
                bandwidth_quota_bytes: 3 * GB,
            },
            Plan::Pro => PlanLimits {
                plan: self,
                // Base tier (Pro x1). The effective limit can be larger when
                // the user bought a higher storage tier; see
                // `effective_storage_limit` and `STORAGE_STEP_BYTES`.
                storage_bytes: 100 * GB,
                devices: u32::MAX,
                saves_tracked: None,
                version_history_forever: true,
                max_save_size_bytes: 10 * GB,
                bandwidth_window_secs: 15 * 60,
                // Single 15-min rolling window. Kept above the 10 GB max
                // single-save size so a first-time upload of a large save
                // (whose `requested_bytes` ≈ its full size) can't be
                // permanently wedged behind the window, and roomy enough that
                // onboarding several games at once doesn't trip a 429.
                // Raised to 15 GB (2026-07-09) alongside the Free bump.
                bandwidth_quota_bytes: 15 * GB,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct PlanLimits {
    pub plan: Plan,
    pub storage_bytes: u64,
    /// `u32::MAX` means unlimited devices. The wire shape on `/v1/me`
    /// normalises this to `-1`.
    pub devices: u32,
    /// `None` means unlimited tracked saves. Free is unlimited too as
    /// of 1.6.1.
    pub saves_tracked: Option<u32>,
    /// Always `true` post-1.6.1. Kept as a field on the limits struct so
    /// a future "rolling 30 days" tier could opt out without a wire
    /// rename.
    pub version_history_forever: bool,
    pub max_save_size_bytes: u64,
    pub bandwidth_window_secs: u32,
    pub bandwidth_quota_bytes: u64,
}

#[allow(clippy::identity_op)]
const MB: u64 = 1024 * 1024;
const GB: u64 = 1024 * MB;

/// Storage tier step. Pro sells in +25 GB increments (Pro x1 = 25 GB base,
/// x2 = 50, x3 = 75, …). Each step is its own Polar product; the resolved
/// size is stored per-user in `profiles.storage_limit_bytes` by the webhook.
/// Kept here so the landing/checkout copy and the server agree on one number.
pub const STORAGE_STEP_BYTES: u64 = 25 * GB;

/// The storage limit actually enforced for a user, given their plan and the
/// optional per-user override column (`profiles.storage_limit_bytes`,
/// NULL = use the plan default).
///
/// Free always gets the plan default regardless of any override left behind,
/// a guard so a stale value from a lapsed Pro tier can never grant a free
/// account extra room. Pro honours a positive override (the bought tier) and
/// falls back to the 100 GB base when it's NULL/0.
pub fn effective_storage_limit(plan: Plan, override_bytes: Option<i64>) -> u64 {
    match plan {
        Plan::Free => Plan::Free.limits().storage_bytes,
        Plan::Pro => match override_bytes {
            Some(b) if b > 0 => b as u64,
            _ => Plan::Pro.limits().storage_bytes,
        },
    }
}

/// The storage limit enforced *right now*, honouring an in-flight downgrade
/// grace window.
///
/// While a downgrade is scheduled (`change_at` still in the future) the
/// `profiles.storage_limit_bytes` column holds an absolute grant, the limit the
/// user had the moment the smaller tier landed, and it wins over the
/// plan default, **Free included**. That exception is the entire point of the
/// window: keep the room until the deadline so nothing is purged behind the
/// user's back, and so `/v1/me` can count down to a date instead of announcing
/// a deletion that already happened.
///
/// This is why `effective_storage_limit`'s "Free ignores any override" guard
/// isn't enough on its own: a Pro→Free drop flips the plan column immediately,
/// so without the grace grant the limit collapses to 2 GB the same second and
/// the auto-purge starts eating history. Once the deadline passes,
/// `quota::apply_due_downgrade` promotes the pending value into
/// `storage_limit_bytes` and this collapses back to `effective_storage_limit`.
pub fn resolved_storage_limit(
    plan: Plan,
    override_bytes: Option<i64>,
    change_at_unix: Option<i64>,
    now_unix: i64,
) -> u64 {
    let base = effective_storage_limit(plan, override_bytes);
    match (change_at_unix, override_bytes) {
        (Some(at), Some(grant)) if at > now_unix && grant > 0 => (grant as u64).max(base),
        _ => base,
    }
}

/// The device cap actually in force, given the plan and whether the account has
/// **ever** been Pro (`profiles.first_pro_at`).
///
/// Storage and devices come apart on a downgrade, on purpose. Storage is a
/// recurring cost, so Free means 2 GB and the grace window of
/// [`resolved_storage_limit`] is what makes that landing survivable. Devices are
/// not: a user who paired six machines while paying doesn't un-pair them by
/// letting the subscription lapse, and telling them "6 / 3" turns a working
/// account into one that reads as broken, or locks them out outright the day
/// the cap stops being merely informational (`register_device` counts but
/// deliberately doesn't gate).
///
/// So the Pro device allowance is kept for life. It's the one thing a lapsed
/// subscription doesn't take back.
pub fn resolved_devices_limit(plan: Plan, ever_pro: bool) -> u32 {
    if ever_pro {
        return Plan::Pro.limits().devices;
    }
    plan.limits().devices
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ex_pro_keeps_unlimited_devices_but_not_the_storage() {
        // The point of the split: a lapsed Pro is back on 2 GB…
        assert_eq!(effective_storage_limit(Plan::Free, None), 2 * GB);
        // …but keeps every machine they paired while paying.
        assert_eq!(
            resolved_devices_limit(Plan::Free, true),
            Plan::Pro.limits().devices
        );
        // Someone who was never Pro gets the plain Free cap.
        assert_eq!(resolved_devices_limit(Plan::Free, false), 3);
        // And Pro is Pro either way.
        assert_eq!(
            resolved_devices_limit(Plan::Pro, false),
            Plan::Pro.limits().devices
        );
    }

    #[test]
    fn free_limits_match_spec() {
        let l = Plan::Free.limits();
        assert_eq!(l.storage_bytes, 2 * GB);
        assert_eq!(l.devices, 3);
        assert_eq!(l.saves_tracked, None);
        assert!(l.version_history_forever);
        assert_eq!(l.max_save_size_bytes, 1 * GB);
        assert_eq!(l.bandwidth_window_secs, 15 * 60);
        assert_eq!(l.bandwidth_quota_bytes, 3 * GB);
    }

    #[test]
    fn pro_limits_match_spec() {
        let l = Plan::Pro.limits();
        assert_eq!(l.storage_bytes, 100 * GB);
        assert_eq!(l.devices, u32::MAX);
        assert_eq!(l.saves_tracked, None);
        assert!(l.version_history_forever);
        assert_eq!(l.max_save_size_bytes, 10 * GB);
        assert_eq!(l.bandwidth_window_secs, 15 * 60);
        assert_eq!(l.bandwidth_quota_bytes, 15 * GB);
    }

    #[test]
    fn from_str_roundtrip() {
        for p in [Plan::Free, Plan::Pro] {
            assert_eq!(Plan::from_str(p.as_str()), Some(p));
        }
        assert_eq!(Plan::from_str("bogus"), None);
        // Legacy tokens grandfather onto Pro.
        assert_eq!(Plan::from_str("proplus"), Some(Plan::Pro));
        assert_eq!(Plan::from_str("pro+"), Some(Plan::Pro));
        assert_eq!(Plan::from_str("pro_plus"), Some(Plan::Pro));
    }

    #[test]
    fn effective_storage_limit_honours_tier_and_guards_free() {
        // Free ignores any override.
        assert_eq!(
            effective_storage_limit(Plan::Free, Some(200 * GB as i64)),
            2 * GB
        );
        assert_eq!(effective_storage_limit(Plan::Free, None), 2 * GB);
        // Pro falls back to the 100 GB base when NULL/0/negative.
        assert_eq!(effective_storage_limit(Plan::Pro, None), 100 * GB);
        assert_eq!(effective_storage_limit(Plan::Pro, Some(0)), 100 * GB);
        assert_eq!(effective_storage_limit(Plan::Pro, Some(-5)), 100 * GB);
        // Pro honours a bought tier override above the base.
        assert_eq!(
            effective_storage_limit(Plan::Pro, Some(150 * GB as i64)),
            150 * GB
        );
        assert_eq!(STORAGE_STEP_BYTES, 25 * GB);
    }

    #[test]
    fn grace_grant_survives_the_plan_flip_to_free() {
        let now = 1_000_000;
        let grant = 100 * GB as i64;
        // Pro→Free with a live window: the plan column already says free, but
        // the absolute grant rules until the deadline.
        assert_eq!(
            resolved_storage_limit(Plan::Free, Some(grant), Some(now + 86_400), now),
            100 * GB
        );
        // Deadline passed → back to the plan default even if the column is
        // still populated (belt and braces; `apply_due_downgrade` clears it).
        assert_eq!(
            resolved_storage_limit(Plan::Free, Some(grant), Some(now - 1), now),
            2 * GB
        );
        // No window at all → plain effective limit, Free ignores the override.
        assert_eq!(
            resolved_storage_limit(Plan::Free, Some(grant), None, now),
            2 * GB
        );
    }

    #[test]
    fn grace_window_leaves_a_pro_tier_alone() {
        let now = 1_000_000;
        // On Pro the column already *is* the bought tier, so an open window
        // changes nothing: a Pro x6 → Pro x2 downgrade parks 150 GB in the
        // column and 50 GB in `pending`, and 150 GB is what's enforced.
        assert_eq!(
            resolved_storage_limit(Plan::Pro, Some(150 * GB as i64), Some(now + 10), now),
            150 * GB
        );
        // The plan base is the floor when the column is empty, window or not.
        assert_eq!(
            resolved_storage_limit(Plan::Pro, None, Some(now + 10), now),
            100 * GB
        );
    }
}
