//! Product events: the milestones and choices that say how people use Hoard
//! Cloud, stored in `product_events` (cloud only, never self-hosted).
//!
//! Until oct-2026 this lived in three tables with three conventions:
//! `audit_log.event_type`, `sync_log.kind` and free text in `client_logs`.
//! Every question about Pro meant ad hoc SQL against prod, joining the three
//! by hand, and half the answers were gone because each one was pruned on its
//! own clock. This list is the one place an event gets a name. The server
//! writes only names from here, and an app event whose name is not here is
//! dropped at ingest, so the table cannot fill up with spellings.
//!
//! Adding an event: a variant, its name, how often it may repeat, and whether
//! the app may send it. Names are `area.what`, lower case, and never change
//! once shipped: a renamed event is a new event with an empty history.

use serde_json::{Map, Value};

/// How often one event may be stored for the same account.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Repeat {
    /// Every occurrence is a row.
    Always,
    /// Once per account and scope, ever: `upload.first`, or a storage
    /// threshold, whose scope is the percentage.
    Once,
    /// Once per account, scope and UTC day. For things that repeat in bursts
    /// (a full account bounces every upload) where what matters is the day it
    /// happened, not how many times the client retried.
    Daily,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProductEvent {
    // ---- written by the server
    AccountCreated,
    AccountDeletionRequested,
    AccountReactivated,
    ActiveDay,
    DeviceRegistered,
    DeviceLimitHit,
    UploadFirst,
    DownloadFirst,
    SyncCrossDevice,
    StorageThreshold,
    StoragePurged,
    QuotaBlocked,
    SaveTooLarge,
    BandwidthBlocked,
    CheckoutRequested,
    CheckoutStatus,
    SubscriptionStarted,
    SubscriptionCanceled,
    SubscriptionEnded,
    ExportRequested,
    // ---- sent by the app
    AppOpened,
    PricingOpened,
    PricingContinue,
    CheckoutOpened,
    CheckoutFailed,
    UpsellShown,
    RestoreManual,
}

impl ProductEvent {
    pub const ALL: [ProductEvent; 27] = [
        Self::AccountCreated,
        Self::AccountDeletionRequested,
        Self::AccountReactivated,
        Self::ActiveDay,
        Self::DeviceRegistered,
        Self::DeviceLimitHit,
        Self::UploadFirst,
        Self::DownloadFirst,
        Self::SyncCrossDevice,
        Self::StorageThreshold,
        Self::StoragePurged,
        Self::QuotaBlocked,
        Self::SaveTooLarge,
        Self::BandwidthBlocked,
        Self::CheckoutRequested,
        Self::CheckoutStatus,
        Self::SubscriptionStarted,
        Self::SubscriptionCanceled,
        Self::SubscriptionEnded,
        Self::ExportRequested,
        Self::AppOpened,
        Self::PricingOpened,
        Self::PricingContinue,
        Self::CheckoutOpened,
        Self::CheckoutFailed,
        Self::UpsellShown,
        Self::RestoreManual,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::AccountCreated => "account.created",
            Self::AccountDeletionRequested => "account.deletion_requested",
            Self::AccountReactivated => "account.reactivated",
            Self::ActiveDay => "active.day",
            Self::DeviceRegistered => "device.registered",
            Self::DeviceLimitHit => "device.limit_hit",
            Self::UploadFirst => "upload.first",
            Self::DownloadFirst => "download.first",
            Self::SyncCrossDevice => "sync.cross_device",
            Self::StorageThreshold => "storage.threshold",
            Self::StoragePurged => "storage.purged",
            Self::QuotaBlocked => "quota.blocked",
            Self::SaveTooLarge => "save.too_large",
            Self::BandwidthBlocked => "bandwidth.blocked",
            Self::CheckoutRequested => "checkout.requested",
            Self::CheckoutStatus => "checkout.status",
            Self::SubscriptionStarted => "subscription.started",
            Self::SubscriptionCanceled => "subscription.canceled",
            Self::SubscriptionEnded => "subscription.ended",
            Self::ExportRequested => "export.requested",
            Self::AppOpened => "app.opened",
            Self::PricingOpened => "pricing.opened",
            Self::PricingContinue => "pricing.continue",
            Self::CheckoutOpened => "checkout.opened",
            Self::CheckoutFailed => "checkout.failed",
            Self::UpsellShown => "upsell.shown",
            Self::RestoreManual => "restore.manual",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|e| e.name() == name)
    }

    pub fn repeat(self) -> Repeat {
        match self {
            Self::AccountCreated
            | Self::UploadFirst
            | Self::DownloadFirst
            | Self::SyncCrossDevice
            | Self::StorageThreshold
            | Self::CheckoutStatus
            | Self::SubscriptionStarted
            | Self::SubscriptionCanceled
            | Self::SubscriptionEnded => Repeat::Once,
            // `upsell.shown` is daily because the storage dialog opens on every
            // bounced upload, and an account stuck in a loop bounced 840 times
            // a day (sep-2026).
            Self::ActiveDay
            | Self::AppOpened
            | Self::DeviceLimitHit
            | Self::StoragePurged
            | Self::QuotaBlocked
            | Self::SaveTooLarge
            | Self::BandwidthBlocked
            | Self::UpsellShown => Repeat::Daily,
            Self::AccountDeletionRequested
            | Self::AccountReactivated
            | Self::DeviceRegistered
            | Self::CheckoutRequested
            | Self::ExportRequested
            | Self::PricingOpened
            | Self::PricingContinue
            | Self::CheckoutOpened
            | Self::CheckoutFailed
            | Self::RestoreManual => Repeat::Always,
        }
    }

    /// Whether the app may send it. Everything else is something only the
    /// server can see happen, and a client claiming it would be believed.
    pub fn from_app(self) -> bool {
        matches!(
            self,
            Self::AppOpened
                | Self::PricingOpened
                | Self::PricingContinue
                | Self::CheckoutOpened
                | Self::CheckoutFailed
                | Self::UpsellShown
                | Self::RestoreManual
        )
    }

    /// The key a repeat collapses on, or `None` for an event where every
    /// occurrence counts. `day` is the UTC date (`2026-10-14`); `scope` is
    /// whatever narrows it, and may be empty.
    pub fn dedupe_key(self, day: &str, scope: &str) -> Option<String> {
        match self.repeat() {
            Repeat::Always => None,
            Repeat::Once => Some(scope.to_string()),
            Repeat::Daily if scope.is_empty() => Some(day.to_string()),
            Repeat::Daily => Some(format!("{day}|{scope}")),
        }
    }
}

/// The most properties one event keeps, and how long a key or a text value may
/// be. Properties are labels (`from = "sidebar"`, `interval = "year"`), never
/// free text, and nothing an app sends gets to be bigger than that.
const MAX_PROPS: usize = 12;
const MAX_KEY_LEN: usize = 32;
const MAX_TEXT_LEN: usize = 64;

/// Keeps what a property may be: a flat object of short labels, numbers and
/// booleans. Anything else (nesting, long strings, odd keys) is dropped rather
/// than truncated, because half a value reads as a real one.
pub fn clean_props(value: &Value) -> Value {
    let Value::Object(map) = value else {
        return Value::Object(Map::new());
    };
    let mut out = Map::new();
    for (key, v) in map {
        if out.len() == MAX_PROPS {
            break;
        }
        let key_ok = !key.is_empty()
            && key.len() <= MAX_KEY_LEN
            && key
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
        let value_ok = match v {
            Value::String(s) => s.chars().count() <= MAX_TEXT_LEN,
            Value::Number(_) | Value::Bool(_) => true,
            _ => false,
        };
        if key_ok && value_ok {
            out.insert(key.clone(), v.clone());
        }
    }
    Value::Object(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn every_name_is_unique_and_round_trips() {
        let mut seen = std::collections::HashSet::new();
        for e in ProductEvent::ALL {
            assert!(seen.insert(e.name()), "{} is used twice", e.name());
            assert_eq!(ProductEvent::from_name(e.name()), Some(e));
            let (area, what) = e.name().split_once('.').expect("area.what");
            assert!(!area.is_empty() && !what.is_empty());
            assert!(e
                .name()
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b == b'.' || b == b'_'));
        }
        assert_eq!(ProductEvent::from_name("account.Created"), None);
        assert_eq!(ProductEvent::from_name(""), None);
    }

    /// The app can only speak for itself. If it could send `subscription.started`
    /// the conversion count would be whatever a client says it is.
    #[test]
    fn the_app_cannot_claim_server_events() {
        for e in ProductEvent::ALL {
            let server_only = matches!(
                e,
                ProductEvent::AccountCreated
                    | ProductEvent::UploadFirst
                    | ProductEvent::QuotaBlocked
                    | ProductEvent::CheckoutRequested
                    | ProductEvent::SubscriptionStarted
                    | ProductEvent::SubscriptionEnded
            );
            if server_only {
                assert!(!e.from_app(), "{} must come from the server", e.name());
            }
        }
        assert!(ProductEvent::PricingOpened.from_app());
    }

    #[test]
    fn repeats_collapse_on_the_right_key() {
        let day = "2026-10-14";
        assert_eq!(ProductEvent::PricingOpened.dedupe_key(day, "x"), None);
        assert_eq!(
            ProductEvent::UploadFirst.dedupe_key(day, ""),
            Some(String::new())
        );
        assert_eq!(
            ProductEvent::StorageThreshold.dedupe_key(day, "80"),
            Some("80".into())
        );
        assert_eq!(
            ProductEvent::QuotaBlocked.dedupe_key(day, ""),
            Some(day.into())
        );
        assert_eq!(
            ProductEvent::SaveTooLarge.dedupe_key(day, "save-1"),
            Some(format!("{day}|save-1"))
        );
    }

    #[test]
    fn props_keep_labels_and_drop_the_rest() {
        let long = "x".repeat(MAX_TEXT_LEN + 1);
        let cleaned = clean_props(&json!({
            "from": "sidebar",
            "gb": 25,
            "yearly": true,
            "nested": { "a": 1 },
            "list": [1, 2],
            "long": long,
            "Bad-Key": "x",
            "": "x",
            "nothing": null,
        }));
        assert_eq!(
            cleaned,
            json!({ "from": "sidebar", "gb": 25, "yearly": true })
        );
        assert_eq!(clean_props(&json!("not an object")), json!({}));

        let many: Map<String, Value> = (0..40).map(|i| (format!("k{i}"), json!(i))).collect();
        let Value::Object(kept) = clean_props(&Value::Object(many)) else {
            unreachable!()
        };
        assert_eq!(kept.len(), MAX_PROPS);
    }
}
