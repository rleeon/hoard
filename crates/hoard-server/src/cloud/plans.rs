//! Plan limits — single source of truth for what each tier allows.
//!
//! Hardcoded on purpose. Pricing is an opinion (see ADR 0015); when it
//! changes, update here + the landing + the Lemon Squeezy products and
//! ship a new release. Reading these from the DB would invite drift.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Plan {
    Free,
    Pro,
    ProPlus,
}

impl Plan {
    pub fn from_str(s: &str) -> Option<Plan> {
        match s {
            "free" => Some(Plan::Free),
            "pro" => Some(Plan::Pro),
            "proplus" | "pro+" => Some(Plan::ProPlus),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Plan::Free => "free",
            Plan::Pro => "pro",
            Plan::ProPlus => "proplus",
        }
    }

    pub fn limits(self) -> PlanLimits {
        match self {
            Plan::Free => PlanLimits {
                plan: self,
                storage_bytes: 500 * MB,
                devices: 1,
                saves_tracked: Some(3),
                retention_days: 7,
            },
            Plan::Pro => PlanLimits {
                plan: self,
                storage_bytes: 50 * GB,
                devices: 5,
                saves_tracked: None,
                retention_days: 90,
            },
            Plan::ProPlus => PlanLimits {
                plan: self,
                storage_bytes: 200 * GB,
                devices: u32::MAX,
                saves_tracked: None,
                retention_days: 365,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct PlanLimits {
    pub plan: Plan,
    pub storage_bytes: u64,
    pub devices: u32,
    /// `None` means unlimited tracked saves.
    pub saves_tracked: Option<u32>,
    pub retention_days: u32,
}

const MB: u64 = 1024 * 1024;
const GB: u64 = 1024 * MB;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_limits_match_spec() {
        let l = Plan::Free.limits();
        assert_eq!(l.storage_bytes, 500 * MB);
        assert_eq!(l.devices, 1);
        assert_eq!(l.saves_tracked, Some(3));
        assert_eq!(l.retention_days, 7);
    }

    #[test]
    fn pro_limits_match_spec() {
        let l = Plan::Pro.limits();
        assert_eq!(l.storage_bytes, 50 * GB);
        assert_eq!(l.devices, 5);
        assert_eq!(l.saves_tracked, None);
        assert_eq!(l.retention_days, 90);
    }

    #[test]
    fn proplus_limits_match_spec() {
        let l = Plan::ProPlus.limits();
        assert_eq!(l.storage_bytes, 200 * GB);
        assert!(l.devices >= 1000);
        assert_eq!(l.retention_days, 365);
    }

    #[test]
    fn from_str_roundtrip() {
        for p in [Plan::Free, Plan::Pro, Plan::ProPlus] {
            assert_eq!(Plan::from_str(p.as_str()), Some(p));
        }
        assert_eq!(Plan::from_str("bogus"), None);
        assert_eq!(Plan::from_str("pro+"), Some(Plan::ProPlus));
    }
}
