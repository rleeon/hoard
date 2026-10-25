//! `product_events` against a real Postgres: the once and daily keys hold, the
//! app cannot write what only the server may, the backfill in 0067 reads the
//! tables it says it reads, and the admin function runs.
//!
//! Skipped unless `HOARD_PG_TEST_URL` is set, like `blob_sha_paths`.
//!
//! **Never point it at production.** It runs migrations on whatever it is given.

#![cfg(feature = "cloud")]

use hoard_core::product::ProductEvent;
use hoard_core::wire::{LogEntry, EVENT_TARGET};
use hoard_server::cloud::product_events::{ingest_app_events, Event};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

const STUB: &str = include_str!("../../../deploy/fly/pg/supabase-stub.sql");
const MIGRATION: &str = include_str!("../migrations/postgres/0067_product_events.sql");
const ADMIN_UID: &str = "f08eb80b-cbe8-4997-b69a-f2b9a5d6630a";

async fn pool() -> Option<PgPool> {
    let url = std::env::var("HOARD_PG_TEST_URL").ok()?;
    let pool = hoard_server::cloud::db::connect(&url, 5)
        .await
        .expect("connect to the test database");
    let mut guard = pool.acquire().await.expect("bootstrap connection");
    sqlx::query("SELECT pg_advisory_lock(8_233_119_402)")
        .execute(&mut *guard)
        .await
        .expect("bootstrap lock");
    sqlx::raw_sql(STUB)
        .execute(&mut *guard)
        .await
        .expect("the stub applies over whatever is there");
    hoard_server::cloud::db::run_migrations(&pool)
        .await
        .expect("migrations");
    sqlx::query("SELECT pg_advisory_unlock(8_233_119_402)")
        .execute(&mut *guard)
        .await
        .expect("bootstrap unlock");
    drop(guard);
    Some(pool)
}

async fn user(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO profiles (user_id, email) VALUES ($1, $2)")
        .bind(id)
        .bind(format!("{id}@test.invalid"))
        .execute(pool)
        .await
        .expect("insert profile");
    id
}

async fn count(pool: &PgPool, user: Uuid, name: &str) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM product_events WHERE user_id = $1 AND name = $2")
        .bind(user)
        .bind(name)
        .fetch_one(pool)
        .await
        .unwrap()
}

fn app_line(event: &str, props: serde_json::Value) -> LogEntry {
    LogEntry {
        level: "info".into(),
        target: Some(EVENT_TARGET.into()),
        message: "event".into(),
        fields: Some(json!({ "event": event, "props": props.to_string() })),
        ts: None,
    }
}

#[tokio::test]
async fn once_daily_and_always_keep_their_promises() {
    let Some(pool) = pool().await else { return };
    let u = user(&pool).await;

    assert!(Event::new(u, ProductEvent::UploadFirst).record(&pool).await);
    assert!(!Event::new(u, ProductEvent::UploadFirst).record(&pool).await);
    assert_eq!(count(&pool, u, "upload.first").await, 1);

    // Daily, narrowed by save: two saves are two rows, the same save again is not.
    for save in ["a", "b", "a"] {
        Event::new(u, ProductEvent::SaveTooLarge)
            .scope(save)
            .record(&pool)
            .await;
    }
    assert_eq!(count(&pool, u, "save.too_large").await, 2);

    // The index holds on its own, without this process's memory of the key.
    let raw = sqlx::query(
        "INSERT INTO product_events (user_id, name, source, dedupe_key)
         VALUES ($1, 'upload.first', 'server', '') ON CONFLICT DO NOTHING",
    )
    .bind(u)
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(raw.rows_affected(), 0);

    for _ in 0..3 {
        Event::new(u, ProductEvent::CheckoutRequested)
            .props(json!({ "interval": "month", "storage_gb": null }))
            .record(&pool)
            .await;
    }
    assert_eq!(count(&pool, u, "checkout.requested").await, 3);
    let props: serde_json::Value = sqlx::query_scalar(
        "SELECT props FROM product_events WHERE user_id = $1 AND name = 'checkout.requested' LIMIT 1",
    )
    .bind(u)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(props, json!({ "interval": "month" }));
}

#[tokio::test]
async fn the_app_writes_its_own_events_and_nothing_else() {
    let Some(pool) = pool().await else { return };
    let u = user(&pool).await;
    let lines = vec![
        app_line("pricing.opened", json!({ "from": "sidebar" })),
        app_line(
            "pricing.opened",
            json!({ "from": "storage", "nested": { "x": 1 } }),
        ),
        app_line("app.opened", json!({})),
        app_line("app.opened", json!({})),
        app_line("subscription.started", json!({ "plan": "pro" })),
        app_line("made.up", json!({})),
        LogEntry {
            level: "warn".into(),
            target: Some("hoard_agent::agent".into()),
            message: "a log line".into(),
            fields: None,
            ts: None,
        },
    ];
    let taken = ingest_app_events(&pool, u, Some("fp-1"), Some("1.2.3"), &lines).await;
    assert_eq!(
        taken, 6,
        "every event line leaves the batch, the log line stays"
    );

    assert_eq!(count(&pool, u, "pricing.opened").await, 2);
    assert_eq!(
        count(&pool, u, "app.opened").await,
        1,
        "once a day per machine"
    );
    assert_eq!(count(&pool, u, "subscription.started").await, 0);
    assert_eq!(count(&pool, u, "made.up").await, 0);

    let rows: Vec<(String, serde_json::Value, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT source, props, device_fingerprint, app_version FROM product_events
          WHERE user_id = $1 AND name = 'pricing.opened' ORDER BY id",
    )
    .bind(u)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(rows[0].0, "app");
    assert_eq!(rows[1].1, json!({ "from": "storage" }));
    assert_eq!(rows[0].2.as_deref(), Some("fp-1"));
    assert_eq!(rows[0].3.as_deref(), Some("1.2.3"));
}

/// The backfill half of 0067, run again over a user seeded with the history it
/// reads. It ran once already, on an empty database, during the bootstrap.
#[tokio::test]
async fn the_backfill_reads_the_old_tables() {
    let Some(pool) = pool().await else { return };
    let u = user(&pool).await;
    sqlx::raw_sql(&format!(
        "INSERT INTO devices (user_id, device_name, os, fingerprint)
              VALUES ('{u}', 'pc', 'linux', 'fp-a'), ('{u}', 'deck', 'linux', 'fp-b');
         INSERT INTO sync_log (user_id, kind, bytes, at) VALUES
              ('{u}', 'upload', 10, '2026-09-01 10:00Z'),
              ('{u}', 'upload', 10, '2026-09-02 10:00Z'),
              ('{u}', 'download', 10, '2026-09-03 10:00Z'),
              ('{u}', 'quota_block', 10, '2026-09-04 10:00Z'),
              ('{u}', 'quota_block', 10, '2026-09-04 11:00Z'),
              ('{u}', 'quota_block', 10, '2026-09-05 10:00Z');
         INSERT INTO sync_log (user_id, kind, bytes, metadata, at) VALUES
              ('{u}', 'save_too_large', 5, '{{\"save_id\": \"s1\", \"game_slug\": \"cyberpunk\"}}', '2026-09-06 10:00Z');
         INSERT INTO audit_log (user_id, actor, event_type, metadata, created_at) VALUES
              ('{u}', 'user', 'checkout.requested', '{{\"interval\": \"year\", \"storage_gb\": null}}', '2026-09-07 10:00Z');
         INSERT INTO subscriptions (user_id, ls_customer_id, ls_subscription_id, ls_product_id,
                                    plan, interval, status, created_at)
              VALUES ('{u}', 'c', 'sub-{u}', 'p', 'pro', 'month', 'active', '2026-09-08 10:00Z');"
    ))
    .execute(&pool)
    .await
    .expect("seed");

    let backfill = MIGRATION
        .split("-- ---- backfill")
        .nth(1)
        .and_then(|rest| rest.split("-- ---- the admin panel").next())
        .expect("the migration has a backfill section");
    sqlx::raw_sql(backfill)
        .execute(&pool)
        .await
        .expect("backfill");

    assert_eq!(count(&pool, u, "account.created").await, 1);
    assert_eq!(count(&pool, u, "device.registered").await, 2);
    assert_eq!(count(&pool, u, "upload.first").await, 1);
    assert_eq!(count(&pool, u, "download.first").await, 1);
    assert_eq!(count(&pool, u, "quota.blocked").await, 2, "one per day");
    assert_eq!(count(&pool, u, "save.too_large").await, 1);
    assert_eq!(count(&pool, u, "checkout.requested").await, 1);
    assert_eq!(count(&pool, u, "subscription.started").await, 1);

    let first_upload: String = sqlx::query_scalar(
        "SELECT to_char(occurred_at AT TIME ZONE 'UTC', 'YYYY-MM-DD')
           FROM product_events WHERE user_id = $1 AND name = 'upload.first'",
    )
    .bind(u)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(first_upload, "2026-09-01");
    let blocks: serde_json::Value = sqlx::query_scalar(
        "SELECT props FROM product_events
          WHERE user_id = $1 AND name = 'quota.blocked' AND dedupe_key = '2026-09-04'",
    )
    .bind(u)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(blocks, json!({ "blocks": 2 }));
    let checkout: serde_json::Value = sqlx::query_scalar(
        "SELECT props FROM product_events WHERE user_id = $1 AND name = 'checkout.requested'",
    )
    .bind(u)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(checkout, json!({ "interval": "year" }));

    // A second run adds nothing to what is keyed.
    sqlx::raw_sql(backfill)
        .execute(&pool)
        .await
        .expect("backfill again");
    assert_eq!(count(&pool, u, "upload.first").await, 1);
    assert_eq!(count(&pool, u, "quota.blocked").await, 2);
}

#[tokio::test]
async fn the_admin_function_answers_the_admin_and_nobody_else() {
    let Some(pool) = pool().await else { return };
    let u = user(&pool).await;
    Event::new(u, ProductEvent::AccountCreated)
        .record(&pool)
        .await;
    Event::new(u, ProductEvent::PricingOpened)
        .props(json!({ "from": "sidebar" }))
        .record(&pool)
        .await;

    // Other test binaries swap auth.uid() during their bootstrap, so the admin
    // is impersonated inside a transaction that is rolled back.
    let mut tx = pool.begin().await.unwrap();
    sqlx::raw_sql(&format!(
        "CREATE OR REPLACE FUNCTION auth.uid() RETURNS uuid LANGUAGE sql STABLE
             AS $$ SELECT '{ADMIN_UID}'::uuid $$"
    ))
    .execute(&mut *tx)
    .await
    .unwrap();
    let out: serde_json::Value = sqlx::query_scalar("SELECT public.admin_product_events()")
        .fetch_one(&mut *tx)
        .await
        .expect("the admin function runs");
    tx.rollback().await.unwrap();

    for key in [
        "funnel",
        "cohorts",
        "weekly",
        "daily_active",
        "catalog",
        "pricing_from",
        "conversions",
    ] {
        assert!(out.get(key).is_some(), "{key} missing");
    }
    let steps = out["funnel"].as_array().unwrap();
    assert_eq!(steps[0]["step"], "account.created");
    assert!(steps[0]["users"].as_i64().unwrap() >= 1);
    assert!(out["pricing_from"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["source"] == "sidebar"));

    let mut tx = pool.begin().await.unwrap();
    sqlx::raw_sql(
        "CREATE OR REPLACE FUNCTION auth.uid() RETURNS uuid LANGUAGE sql STABLE
             AS $$ SELECT gen_random_uuid() $$",
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    let refused =
        sqlx::query_scalar::<_, serde_json::Value>("SELECT public.admin_product_events()")
            .fetch_one(&mut *tx)
            .await
            .unwrap_err();
    assert_eq!(
        refused
            .as_database_error()
            .and_then(|d| d.code())
            .as_deref(),
        Some("42501")
    );
}
