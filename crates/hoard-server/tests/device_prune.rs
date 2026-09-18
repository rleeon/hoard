//! The 90-day device sweep, against a real Postgres.
//!
//! This one has to be exercised against the database because the whole point is
//! the interval arithmetic and the cached count staying honest afterwards;
//! neither is visible from Rust.
//!
//! Skipped unless `HOARD_PG_TEST_URL` is set, like `blob_sha_paths`:
//!
//! ```sh
//! export HOARD_PG_TEST_URL=postgres://postgres:hoard@localhost:55432/hoard
//! cargo test -p hoard-server --features cloud --test device_prune
//! ```
//!
//! **Never point it at production.** It runs migrations on whatever it is given.

#![cfg(feature = "cloud")]

use hoard_server::cloud::device_prune::prune;
use hoard_server::cloud::notices::{self, Kind, ACCOUNT};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool() -> Option<PgPool> {
    let url = std::env::var("HOARD_PG_TEST_URL").ok()?;
    let pool = PgPool::connect(&url).await.expect("connect");
    sqlx::migrate!("./migrations/postgres")
        .set_ignore_missing(true)
        .run(&pool)
        .await
        .expect("migrate");
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

/// A device last seen `days_ago`, with its own fingerprint.
async fn device(pool: &PgPool, user_id: Uuid, name: &str, days_ago: i32) {
    sqlx::query(
        "INSERT INTO devices (user_id, device_name, device_kind, fingerprint, last_seen_at)
         VALUES ($1, $2, 'desktop', $3, now() - make_interval(days => $4))",
    )
    .bind(user_id)
    .bind(name)
    .bind(format!("fp-{name}-{user_id}"))
    .bind(days_ago)
    .execute(pool)
    .await
    .expect("insert device");
}

/// `profiles.devices_count` is INT4: read it as anything else and sqlx fails
/// at runtime, which is the whole reason this test talks to a real database.
async fn count_of(pool: &PgPool, user_id: Uuid) -> i32 {
    sqlx::query_scalar("SELECT devices_count FROM profiles WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn only_devices_past_ninety_days_go() {
    let Some(pool) = pool().await else { return };
    let u = user(&pool).await;
    device(&pool, u, "ancient", 200).await;
    device(&pool, u, "stale", 91).await;
    // 89 days is inside the window, and it is the edge that matters: the cutoff
    // being off by one would silently reclaim a slot somebody still uses.
    device(&pool, u, "borderline", 89).await;
    device(&pool, u, "daily", 1).await;

    prune(&pool).await.unwrap();

    let left: Vec<(String,)> =
        sqlx::query_as("SELECT device_name FROM devices WHERE user_id = $1 ORDER BY device_name")
            .bind(u)
            .fetch_all(&pool)
            .await
            .unwrap();
    let left: Vec<String> = left.into_iter().map(|(n,)| n).collect();
    assert_eq!(left, vec!["borderline".to_string(), "daily".to_string()]);
}

#[tokio::test]
async fn the_cached_count_is_repaired_and_the_notice_re_armed() {
    let Some(pool) = pool().await else { return };
    let u = user(&pool).await;
    device(&pool, u, "old-laptop", 120).await;
    device(&pool, u, "desktop", 2).await;
    sqlx::query("UPDATE profiles SET devices_count = 2 WHERE user_id = $1")
        .bind(u)
        .execute(&pool)
        .await
        .unwrap();
    // The account was told it was full while the dead laptop held a slot.
    assert!(notices::claim(&pool, u, Kind::DevicesFull, ACCOUNT)
        .await
        .unwrap());

    prune(&pool).await.unwrap();

    assert_eq!(count_of(&pool, u).await, 1);
    // A slot opened with no action from the user, so the warning must be able
    // to fire again the next time they really are full.
    assert!(notices::claim(&pool, u, Kind::DevicesFull, ACCOUNT)
        .await
        .unwrap());
}

#[tokio::test]
async fn an_account_that_loses_nothing_is_left_alone() {
    let Some(pool) = pool().await else { return };
    let untouched = user(&pool).await;
    device(&pool, untouched, "fresh", 3).await;
    sqlx::query("UPDATE profiles SET devices_count = 1 WHERE user_id = $1")
        .bind(untouched)
        .execute(&pool)
        .await
        .unwrap();
    notices::claim(&pool, untouched, Kind::DevicesFull, ACCOUNT)
        .await
        .unwrap();

    let other = user(&pool).await;
    device(&pool, other, "gone", 300).await;

    prune(&pool).await.unwrap();

    assert_eq!(count_of(&pool, untouched).await, 1);
    // Its notice survives: nothing about that account changed.
    assert!(!notices::claim(&pool, untouched, Kind::DevicesFull, ACCOUNT)
        .await
        .unwrap());
}
