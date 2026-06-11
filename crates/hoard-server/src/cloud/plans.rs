//! Plan limits — single source of truth for what each tier allows.
//!
//! Hardcoded on purpose. Pricing is an opinion (see ADR 0015); when it
//! changes, update here + the landing + the Lemon Squeezy products and
//! ship a new release. Reading these from the DB would invite drift.
//!
//! Two tiers post-1.6.1: Free and Pro. Pro+ was removed — the gap
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
            // Legacy values from the old enum — a user who paid for
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
                storage_bytes: 1 * GB,
                devices: 3,
                saves_tracked: None,
                version_history_forever: true,
                max_save_size_bytes: 200 * MB,
                bandwidth_window_secs: 15 * 60,
                bandwidth_quota_bytes: 1 * GB,
            },
            Plan::Pro => PlanLimits {
                plan: self,
                storage_bytes: 50 * GB,
                devices: u32::MAX,
                saves_tracked: None,
                version_history_forever: true,
                max_save_size_bytes: 2 * GB,
                bandwidth_window_secs: 15 * 60,
                // Single 15-min rolling window. Kept well above the 2 GB
                // max single-save size so a first-time upload of a large
                // save (whose `requested_bytes` ≈ its full size) can't be
                // permanently wedged behind the window, and roomy enough that
                // onboarding several games at once doesn't trip a 429.
                bandwidth_quota_bytes: 5 * GB,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_limits_match_spec() {
        let l = Plan::Free.limits();
        assert_eq!(l.storage_bytes, 1 * GB);
        assert_eq!(l.devices, 3);
        assert_eq!(l.saves_tracked, None);
        assert!(l.version_history_forever);
        assert_eq!(l.max_save_size_bytes, 200 * MB);
        assert_eq!(l.bandwidth_window_secs, 15 * 60);
        assert_eq!(l.bandwidth_quota_bytes, 1 * GB);
    }

    #[test]
    fn pro_limits_match_spec() {
        let l = Plan::Pro.limits();
        assert_eq!(l.storage_bytes, 50 * GB);
        assert_eq!(l.devices, u32::MAX);
        assert_eq!(l.saves_tracked, None);
        assert!(l.version_history_forever);
        assert_eq!(l.max_save_size_bytes, 2 * GB);
        assert_eq!(l.bandwidth_window_secs, 15 * 60);
        assert_eq!(l.bandwidth_quota_bytes, 5 * GB);
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
}
