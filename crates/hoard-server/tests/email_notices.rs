//! The once-only gate behind every service email, against a real Postgres.
//!
//! `claim` is a primary-key conflict, so it cannot be tested without the
//! database that owns the constraint: a mock would just be asserting that
//! `INSERT ... ON CONFLICT DO NOTHING` was spelled right.
//!
//! Skipped unless `HOARD_PG_TEST_URL` is set, like `blob_sha_paths`:
//!
//! ```sh
//! docker run -d --name hoard-pg -p 55432:5432 \
//!   -e POSTGRES_PASSWORD=hoard -e POSTGRES_DB=hoard postgres:17
//! export HOARD_PG_TEST_URL=postgres://postgres:hoard@localhost:55432/hoard
//! cargo test -p hoard-server --features cloud --test email_notices
//! ```
//!
//! **Never point it at production.** It runs migrations on whatever it is given.

#![cfg(feature = "cloud")]

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

/// A profile to hang notices off, since `user_id` is a foreign key.
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

#[tokio::test]
async fn claim_gives_the_first_caller_and_nobody_else() {
    let Some(pool) = pool().await else { return };
    let u = user(&pool).await;

    assert!(notices::claim(&pool, u, Kind::StorageFull, ACCOUNT).await.unwrap());
    // The 402 comes back on every retry; only the first one may send.
    for _ in 0..5 {
        assert!(!notices::claim(&pool, u, Kind::StorageFull, ACCOUNT).await.unwrap());
    }

    // Clearing re-arms it: filling up again next month is worth telling.
    notices::clear(&pool, u, Kind::StorageFull, ACCOUNT).await.unwrap();
    assert!(notices::claim(&pool, u, Kind::StorageFull, ACCOUNT).await.unwrap());
}

#[tokio::test]
async fn scopes_and_kinds_do_not_shadow_each_other() {
    let Some(pool) = pool().await else { return };
    let u = user(&pool).await;

    // Two oversized games are two emails, not one.
    assert!(notices::claim(&pool, u, Kind::SaveTooLarge, "factorio").await.unwrap());
    assert!(notices::claim(&pool, u, Kind::SaveTooLarge, "cyberpunk").await.unwrap());
    assert!(!notices::claim(&pool, u, Kind::SaveTooLarge, "factorio").await.unwrap());

    // A different notice about the same game is still its own.
    assert!(notices::claim(&pool, u, Kind::ArchiveExpiring, "factorio").await.unwrap());

    // Deleting the save forgets everything said about it, both kinds.
    notices::clear_scope(&pool, "factorio").await.unwrap();
    assert!(notices::claim(&pool, u, Kind::SaveTooLarge, "factorio").await.unwrap());
    assert!(notices::claim(&pool, u, Kind::ArchiveExpiring, "factorio").await.unwrap());
    // ...and leaves the other game alone.
    assert!(!notices::claim(&pool, u, Kind::SaveTooLarge, "cyberpunk").await.unwrap());
}

#[tokio::test]
async fn clear_all_scopes_re_arms_every_game_at_once() {
    let Some(pool) = pool().await else { return };
    let u = user(&pool).await;

    notices::claim(&pool, u, Kind::SaveTooLarge, "a").await.unwrap();
    notices::claim(&pool, u, Kind::SaveTooLarge, "b").await.unwrap();
    notices::claim(&pool, u, Kind::DevicesFull, ACCOUNT).await.unwrap();

    notices::clear_all_scopes(&pool, u, Kind::SaveTooLarge).await.unwrap();
    assert!(notices::claim(&pool, u, Kind::SaveTooLarge, "a").await.unwrap());
    assert!(notices::claim(&pool, u, Kind::SaveTooLarge, "b").await.unwrap());
    // An unrelated kind is untouched.
    assert!(!notices::claim(&pool, u, Kind::DevicesFull, ACCOUNT).await.unwrap());
}

#[tokio::test]
async fn deleting_the_account_takes_its_notices() {
    let Some(pool) = pool().await else { return };
    let u = user(&pool).await;
    notices::claim(&pool, u, Kind::StoragePurgeStarted, ACCOUNT).await.unwrap();

    sqlx::query("DELETE FROM profiles WHERE user_id = $1")
        .bind(u)
        .execute(&pool)
        .await
        .expect("delete profile");

    let left: i64 = sqlx::query_scalar("SELECT count(*) FROM email_notices WHERE user_id = $1")
        .bind(u)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(left, 0, "notices should cascade with the profile");
}
