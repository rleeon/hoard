//! Writes `product_events` (migration 0067, names in `hoard_core::product`).
//!
//! Every write is best-effort and off the answer's critical path: a lost event
//! is a gap in a chart, and a request that fails because a chart could not be
//! fed is a bug. Errors are logged and swallowed.
//!
//! Once and once-a-day events are kept unique by the table's index, but some of
//! them sit on hot routes (`active.day` on `/v1/me`, which every agent polls
//! once a minute). [`SEEN`] remembers the keys this process has already
//! written, so after the first time the route pays a hash lookup and no round
//! trip.

use crate::cloud::quota;
use crate::cloud::state::CloudState;
use hoard_core::product::{clean_props, ProductEvent};
use hoard_core::wire::{LogEntry, EVENT_TARGET};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use uuid::Uuid;

/// Two years: long enough to compare a month with the same month a year
/// before, which is the comparison a growing product keeps wanting.
const RETENTION_DAYS: i32 = 730;

/// Past this many remembered keys the set is cleared and refilled. A cleared
/// key costs one INSERT that the index turns into a no-op.
const SEEN_CAP: usize = 200_000;

/// The most events one log batch may carry. The app sends a handful a day; a
/// batch with hundreds is a client gone wrong, not a busy user.
const MAX_APP_EVENTS_PER_BATCH: usize = 50;

/// Storage thresholds, in percent of the account's limit. 80 is where the
/// cleanup starts deleting old versions on Free (`purge::purge_threshold`), and
/// 100 is the wall.
const STORAGE_THRESHOLDS: [u64; 3] = [50, 80, 100];

type SeenKey = (Uuid, ProductEvent, String);

static SEEN: OnceLock<Mutex<HashSet<SeenKey>>> = OnceLock::new();

fn seen() -> &'static Mutex<HashSet<SeenKey>> {
    SEEN.get_or_init(Default::default)
}

fn already_written(key: &SeenKey) -> bool {
    seen().lock().map(|s| s.contains(key)).unwrap_or(false)
}

fn remember(key: SeenKey) {
    if let Ok(mut s) = seen().lock() {
        if s.len() >= SEEN_CAP {
            s.clear();
        }
        s.insert(key);
    }
}

fn utc_day() -> String {
    let d = time::OffsetDateTime::now_utc().date();
    format!("{:04}-{:02}-{:02}", d.year(), u8::from(d.month()), d.day())
}

#[derive(Debug, Clone)]
pub struct Event {
    user_id: Uuid,
    event: ProductEvent,
    scope: String,
    props: Value,
    source: &'static str,
    fingerprint: Option<String>,
    app_version: Option<String>,
}

impl Event {
    pub fn new(user_id: Uuid, event: ProductEvent) -> Self {
        Self {
            user_id,
            event,
            scope: String::new(),
            props: json!({}),
            source: "server",
            fingerprint: None,
            app_version: None,
        }
    }

    /// What narrows a once or daily event: the percentage of a threshold, the
    /// save that was too large, the machine that checked in.
    pub fn scope(mut self, scope: impl Into<String>) -> Self {
        self.scope = scope.into();
        self
    }

    /// Nulls are dropped, so an optional field that is absent leaves no key
    /// behind instead of a `null` every query has to step around.
    pub fn props(mut self, props: Value) -> Self {
        self.props = match props {
            Value::Object(map) => {
                Value::Object(map.into_iter().filter(|(_, v)| !v.is_null()).collect())
            }
            other => other,
        };
        self
    }

    pub fn device(mut self, fingerprint: Option<&str>, app_version: Option<&str>) -> Self {
        self.fingerprint = fingerprint.filter(|s| !s.is_empty()).map(str::to_owned);
        self.app_version = app_version.filter(|s| !s.is_empty()).map(str::to_owned);
        self
    }

    /// Writes the row. `true` when a new one went in, `false` when it was a
    /// repeat or the write failed.
    pub async fn record(self, pool: &PgPool) -> bool {
        let key = self.event.dedupe_key(&utc_day(), &self.scope);
        let seen_key = key.clone().map(|k| (self.user_id, self.event, k));
        if seen_key.as_ref().is_some_and(already_written) {
            return false;
        }
        let res = sqlx::query(
            "INSERT INTO product_events
                (user_id, name, source, dedupe_key, props, device_fingerprint, app_version)
             VALUES ($1, $2, $3, $4, $5::jsonb, $6, $7)
             ON CONFLICT DO NOTHING",
        )
        .bind(self.user_id)
        .bind(self.event.name())
        .bind(self.source)
        .bind(&key)
        .bind(self.props.to_string())
        .bind(&self.fingerprint)
        .bind(&self.app_version)
        .execute(pool)
        .await;
        match res {
            Ok(done) => {
                if let Some(k) = seen_key {
                    remember(k);
                }
                done.rows_affected() > 0
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    user_id = %self.user_id,
                    event = self.event.name(),
                    "product event not recorded"
                );
                false
            }
        }
    }

    /// [`Event::record`] on its own task, for request handlers.
    pub fn spawn(self, pool: &PgPool) {
        let pool = pool.clone();
        tokio::spawn(async move {
            self.record(&pool).await;
        });
    }
}

// ---- events from the app

/// The product event a log line carries, if it is one the app may send, with
/// its properties cleaned. Anything else on [`EVENT_TARGET`] is dropped: a name
/// that is not in the catalogue, or one only the server may write.
pub fn app_event(entry: &LogEntry) -> Option<(ProductEvent, Value)> {
    if entry.target.as_deref() != Some(EVENT_TARGET) {
        return None;
    }
    let fields = entry.fields.as_ref()?;
    let event = fields
        .get("event")
        .and_then(Value::as_str)
        .and_then(ProductEvent::from_name)
        .filter(|e| e.from_app())?;
    // The shipper renders every field as text, so the properties arrive as a
    // JSON string inside the JSON.
    let props = match fields.get("props") {
        Some(Value::String(s)) => serde_json::from_str(s).unwrap_or(Value::Null),
        Some(other) => other.clone(),
        None => Value::Null,
    };
    Some((event, clean_props(&props)))
}

/// Stores the product events in a log batch and returns how many lines were
/// product events (taken out of the batch whether or not they were stored).
pub async fn ingest_app_events(
    pool: &PgPool,
    user_id: Uuid,
    fingerprint: Option<&str>,
    app_version: Option<&str>,
    entries: &[LogEntry],
) -> usize {
    let mut taken = 0usize;
    for entry in entries.iter().filter(|e| hoard_core::wire::cloud_only(e)) {
        taken += 1;
        if taken > MAX_APP_EVENTS_PER_BATCH {
            continue;
        }
        let Some((event, props)) = app_event(entry) else {
            continue;
        };
        // `app.opened` counts machines; `upsell.shown` counts which dialog.
        let scope = match event {
            ProductEvent::UpsellShown => props
                .get("modal")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            _ => fingerprint.unwrap_or_default().to_string(),
        };
        let mut e = Event::new(user_id, event)
            .scope(scope)
            .props(props)
            .device(fingerprint, app_version);
        e.source = "app";
        e.record(pool).await;
    }
    taken
}

// ---- derived from what the server sees

/// After a version lands: the account's first upload, and any storage
/// threshold it has just crossed.
pub async fn after_upload(state: &CloudState, user_id: Uuid) {
    Event::new(user_id, ProductEvent::UploadFirst)
        .record(&state.pool)
        .await;

    let all_seen = STORAGE_THRESHOLDS.iter().all(|pct| {
        already_written(&(user_id, ProductEvent::StorageThreshold, pct.to_string()))
    });
    if all_seen {
        return;
    }
    let info = match quota::load(&state.pool, user_id).await {
        Ok(Some((_, info))) => info,
        Ok(None) => return,
        Err(e) => {
            tracing::warn!(error = ?e, %user_id, "product events: quota not loaded");
            return;
        }
    };
    for pct in crossed(info.used_bytes, info.limit_bytes) {
        Event::new(user_id, ProductEvent::StorageThreshold)
            .scope(pct.to_string())
            .props(json!({
                "pct": pct,
                "used_bytes": info.used_bytes,
                "limit_bytes": info.limit_bytes,
                "plan": info.plan,
            }))
            .record(&state.pool)
            .await;
    }
}

fn crossed(used: u64, limit: u64) -> impl Iterator<Item = u64> {
    STORAGE_THRESHOLDS.into_iter().filter(move |pct| {
        limit > 0 && u128::from(used) * 100 >= u128::from(limit) * u128::from(*pct)
    })
}

/// After a version is handed out: the account's first download and, when the
/// version came from another machine, the first time sync between two of
/// them actually did something. Both machines are compared by name, which is
/// what `save_versions` keeps (migration 0041); a client too old to send its
/// fingerprint is never counted as either.
pub fn after_download(
    state: &CloudState,
    user_id: Uuid,
    fingerprint: Option<String>,
    save_id: String,
    version: i64,
) {
    let pool = state.pool.clone();
    tokio::spawn(async move {
        Event::new(user_id, ProductEvent::DownloadFirst)
            .device(fingerprint.as_deref(), None)
            .record(&pool)
            .await;

        let Some(fp) = fingerprint else { return };
        if already_written(&(user_id, ProductEvent::SyncCrossDevice, String::new())) {
            return;
        }
        let names: Result<Option<(Option<String>, Option<String>)>, _> = sqlx::query_as(
            "SELECT sv.device_name,
                    (SELECT d.device_name FROM devices d
                      WHERE d.user_id = $1 AND d.fingerprint = $2)
               FROM save_versions sv
              WHERE sv.save_id = $3 AND sv.version_num = $4",
        )
        .bind(user_id)
        .bind(&fp)
        .bind(&save_id)
        .bind(version)
        .fetch_optional(&pool)
        .await;
        let (made_on, asked_from) = match names {
            Ok(Some((Some(a), Some(b)))) => (a, b),
            Ok(_) => return,
            Err(e) => {
                tracing::warn!(error = %e, %user_id, "product events: provenance not read");
                return;
            }
        };
        if made_on != asked_from {
            Event::new(user_id, ProductEvent::SyncCrossDevice)
                .device(Some(&fp), None)
                .record(&pool)
                .await;
        }
    });
}

/// The once-a-day record of a refused sync, from the `sync_log` kind that
/// recorded it. `restore_paced` is the server pacing a loop, not something the
/// person ran into, and is left out.
pub fn sync_block(
    state: &CloudState,
    user_id: Uuid,
    kind: &str,
    save_id: &str,
    game_slug: Option<&str>,
    bytes: u64,
) {
    let (event, scope) = match kind {
        "quota_block" => (ProductEvent::QuotaBlocked, String::new()),
        "bandwidth_block" => (ProductEvent::BandwidthBlocked, String::new()),
        "save_too_large" => (ProductEvent::SaveTooLarge, save_id.to_string()),
        _ => return,
    };
    Event::new(user_id, event)
        .scope(scope)
        .props(json!({ "game_slug": game_slug, "bytes": bytes }))
        .spawn(&state.pool);
}

// ---- retention

pub fn spawn(state: CloudState) {
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(9 * 60)).await;
        let mut tick = tokio::time::interval(Duration::from_secs(24 * 60 * 60));
        tick.tick().await;
        loop {
            match prune(&state.pool).await {
                Ok(0) => {}
                Ok(n) => tracing::info!(rows = n, "product events: pruned expired"),
                Err(e) => tracing::warn!(error = %e, "product events: prune failed"),
            }
            tick.tick().await;
        }
    });
}

pub async fn prune(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let done = sqlx::query(
        "DELETE FROM product_events WHERE occurred_at < now() - make_interval(days => $1)",
    )
    .bind(RETENTION_DAYS)
    .execute(pool)
    .await?;
    Ok(done.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(target: &str, fields: Value) -> LogEntry {
        LogEntry {
            level: "info".into(),
            target: Some(target.into()),
            message: "event".into(),
            fields: Some(fields),
            ts: None,
        }
    }

    #[test]
    fn app_events_arrive_as_text_and_come_out_clean() {
        let entry = line(
            EVENT_TARGET,
            json!({ "event": "pricing.opened", "props": r#"{"from":"sidebar","x":{"y":1}}"# }),
        );
        let (event, props) = app_event(&entry).expect("a pricing event");
        assert_eq!(event, ProductEvent::PricingOpened);
        assert_eq!(props, json!({ "from": "sidebar" }));
    }

    /// A client that claims a payment, or makes up a name, is not believed.
    #[test]
    fn the_app_cannot_write_server_events_or_new_names() {
        for name in ["subscription.started", "upload.first", "pricing.hacked"] {
            let entry = line(EVENT_TARGET, json!({ "event": name }));
            assert!(app_event(&entry).is_none(), "{name} got through");
        }
        let other_target = line("hoard::telemetry", json!({ "event": "app.opened" }));
        assert!(app_event(&other_target).is_none());
    }

    #[test]
    fn thresholds_cross_on_the_percentage() {
        let gib = 1u64 << 30;
        assert_eq!(crossed(gib - 1, 2 * gib).collect::<Vec<_>>(), Vec::<u64>::new());
        assert_eq!(crossed(gib, 2 * gib).collect::<Vec<_>>(), vec![50]);
        assert_eq!(
            crossed(2 * gib + 1, 2 * gib).collect::<Vec<_>>(),
            vec![50, 80, 100]
        );
        // An account without a limit crosses nothing rather than dividing by it.
        assert_eq!(crossed(5, 0).count(), 0);
    }

    #[test]
    fn props_drop_nulls() {
        let e = Event::new(Uuid::nil(), ProductEvent::QuotaBlocked)
            .props(json!({ "game_slug": null, "bytes": 3 }));
        assert_eq!(e.props, json!({ "bytes": 3 }));
    }
}
