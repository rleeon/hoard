//! The sync poll's fingerprint cache (`cloud/routes/sync.rs`), against a real
//! Postgres. The cache is only as good as the fingerprint: a change it misses
//! is a new version the other devices never hear about. So every column the
//! poll carries gets moved here, one at a time, and the fingerprint has to move
//! with it.
//!
//! Skipped unless `HOARD_PG_TEST_URL` is set, like `blob_sha_paths`:
//!
//! ```sh
//! docker run -d --name hoard-pg -p 55432:5432 \
//!   -e POSTGRES_PASSWORD=hoard -e POSTGRES_DB=hoard postgres:17
//! export HOARD_PG_TEST_URL=postgres://postgres:hoard@localhost:55432/hoard
//! cargo test -p hoard-server --features cloud --test sync_manifest
//! ```
//!
//! **Never point it at production.** It runs migrations on whatever it is given.

#![cfg(feature = "cloud")]

use hoard_server::cloud::routes::sync::{cached_fingerprint, fingerprint, saves_for};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool() -> Option<PgPool> {
    let url = std::env::var("HOARD_PG_TEST_URL").ok()?;
    let pool = hoard_server::cloud::db::connect(&url, 5)
        .await
        .expect("connect to the test database");
    // Same bootstrap as `blob_sha_paths`, lock taken on one connection so the
    // unlock lands on it too.
    let mut guard = pool.acquire().await.expect("bootstrap connection");
    sqlx::query("SELECT pg_advisory_lock(8_233_119_402)")
        .execute(&mut *guard)
        .await
        .expect("bootstrap lock");
    for role in ["anon", "authenticated", "service_role"] {
        let _ = sqlx::query(&format!("CREATE ROLE {role} NOLOGIN"))
            .execute(&pool)
            .await;
    }
    sqlx::query("CREATE SCHEMA IF NOT EXISTS auth")
        .execute(&pool)
        .await
        .expect("auth schema");
    sqlx::query("CREATE TABLE IF NOT EXISTS auth.users (id UUID PRIMARY KEY)")
        .execute(&pool)
        .await
        .expect("auth.users");
    sqlx::query(
        "CREATE OR REPLACE FUNCTION auth.uid() RETURNS UUID LANGUAGE sql STABLE AS $$ SELECT NULL::uuid $$",
    )
    .execute(&pool)
    .await
    .expect("auth.uid()");
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

/// One user with one save at version 1.
async fn seed(pool: &PgPool) -> (Uuid, String) {
    let user = Uuid::new_v4();
    let save_id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO auth.users (id) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(user)
        .execute(pool)
        .await
        .expect("auth user");
    sqlx::query(
        "INSERT INTO profiles (user_id, email, plan, storage_bytes) VALUES ($1, $2, 'free', 0)",
    )
    .bind(user)
    .bind(format!("{user}@test.invalid"))
    .execute(pool)
    .await
    .expect("profile");
    add_save(pool, user, &save_id).await;
    (user, save_id)
}

async fn add_save(pool: &PgPool, user: Uuid, save_id: &str) {
    sqlx::query(
        "INSERT INTO saves (id, user_id, game_slug, label, latest_version_num)
         VALUES ($1, $2, 'test-game', 'default', 1)",
    )
    .bind(save_id)
    .bind(user)
    .execute(pool)
    .await
    .expect("save");
    sqlx::query(
        "INSERT INTO save_versions
           (save_id, version_num, size_bytes, sha256, r2_key, file_count, content_addressed)
         VALUES ($1, 1, 1000, 'a', '', 1, TRUE)",
    )
    .bind(save_id)
    .execute(pool)
    .await
    .expect("version");
}

async fn cleanup(pool: &PgPool, user: Uuid) {
    let _ = sqlx::query("DELETE FROM saves WHERE user_id = $1")
        .bind(user)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM profiles WHERE user_id = $1")
        .bind(user)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM auth.users WHERE id = $1")
        .bind(user)
        .execute(pool)
        .await;
}

async fn run(pool: &PgPool, sql: &str, save_id: &str) {
    sqlx::query(sql)
        .bind(save_id)
        .execute(pool)
        .await
        .unwrap_or_else(|e| panic!("{sql}: {e}"));
}

#[tokio::test]
async fn every_column_the_poll_carries_moves_the_fingerprint() {
    let Some(pool) = pool().await else { return };
    let (user, save) = seed(&pool).await;

    let mut prev = fingerprint(&pool, user).await.expect("fingerprint");
    assert_eq!(
        prev,
        fingerprint(&pool, user).await.expect("fingerprint"),
        "nothing changed, so neither may the fingerprint"
    );

    // The version columns are the ones that matter: the `saves` trigger bumps
    // `updated_at` on any update of its own row, but nothing touches the save
    // when only its latest version changes.
    let changes = [
        "UPDATE save_versions SET size_bytes = 2000 WHERE save_id = $1",
        "UPDATE save_versions SET sha256 = 'b' WHERE save_id = $1",
        "UPDATE save_versions SET parent_version = 7 WHERE save_id = $1",
        "UPDATE save_versions SET file_count = 2 WHERE save_id = $1",
        "UPDATE saves SET label = 'with, a \"quote\" (and parens)' WHERE id = $1",
        "UPDATE saves SET game_slug = 'other-game' WHERE id = $1",
        "INSERT INTO save_versions
           (save_id, version_num, size_bytes, sha256, r2_key, file_count, content_addressed)
         VALUES ($1, 2, 1000, 'c', '', 1, TRUE)",
        "UPDATE saves SET latest_version_num = 2 WHERE id = $1",
        "UPDATE saves SET archived_at = now() WHERE id = $1",
        "UPDATE saves SET archived_at = NULL WHERE id = $1",
        "UPDATE saves SET backup_only = true WHERE id = $1",
    ];
    for sql in changes {
        run(&pool, sql, &save).await;
        let now = fingerprint(&pool, user).await.expect("fingerprint");
        // A second version that is not the latest is not in the list, so that
        // one step is the only one allowed to leave the fingerprint alone.
        if sql.starts_with("INSERT") {
            assert_eq!(now, prev, "a non-latest version is not part of the poll");
        } else {
            assert_ne!(now, prev, "missed: {sql}");
        }
        prev = now;
    }

    // Another save showing up moves it, and another user's saves do not.
    run(
        &pool,
        "UPDATE saves SET backup_only = false WHERE id = $1",
        &save,
    )
    .await;
    let before = fingerprint(&pool, user).await.expect("fingerprint");
    let (other, _) = seed(&pool).await;
    assert_eq!(
        before,
        fingerprint(&pool, user).await.expect("fingerprint"),
        "another user's saves leaked into the fingerprint"
    );
    add_save(&pool, user, &Uuid::new_v4().to_string()).await;
    assert_ne!(
        before,
        fingerprint(&pool, user).await.expect("fingerprint"),
        "a new save went unnoticed"
    );

    cleanup(&pool, user).await;
    cleanup(&pool, other).await;
}

#[tokio::test]
async fn the_poll_answers_fresh_after_a_change() {
    let Some(pool) = pool().await else { return };
    let (user, save) = seed(&pool).await;

    let first = saves_for(&pool, user).await.expect("first poll");
    assert_eq!(
        cached_fingerprint(user),
        Some(fingerprint(&pool, user).await.expect("fingerprint")),
        "the first poll fills the cache, so the next one skips the full read"
    );
    assert_eq!(first, saves_for(&pool, user).await.expect("second poll"));

    run(
        &pool,
        "UPDATE save_versions SET sha256 = 'fresh' WHERE save_id = $1",
        &save,
    )
    .await;
    let after = saves_for(&pool, user).await.expect("poll after the change");
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].latest_sha256, "fresh", "served a stale list");

    cleanup(&pool, user).await;
}
