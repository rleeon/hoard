//! A retried upload cleans up after the attempt it replaces, against a real
//! Postgres: only the blobs that attempt promised and the retry dropped, and
//! only those nothing else in the account holds.
//!
//! Skipped unless `HOARD_PG_TEST_URL` is set, like `orphaned_cursor`:
//!
//! ```sh
//! docker run -d --name hoard-pg -p 55432:5432 \
//!   -e POSTGRES_PASSWORD=hoard -e POSTGRES_DB=hoard postgres:17
//! export HOARD_PG_TEST_URL=postgres://postgres:hoard@localhost:55432/hoard
//! cargo test -p hoard-server --features cloud --test version_delete
//! ```
//!
//! **Never point it at production.**

#![cfg(feature = "cloud")]

use hoard_server::cloud::routes::saves::{abandoned_blobs_to_drop, CasFileEntry};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool() -> Option<PgPool> {
    let url = std::env::var("HOARD_PG_TEST_URL").ok()?;
    let pool = hoard_server::cloud::db::connect(&url, 5)
        .await
        .expect("connect to the test database");
    sqlx::query("SELECT pg_advisory_lock(8_233_119_404)")
        .execute(&pool)
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
    sqlx::query("SELECT pg_advisory_unlock(8_233_119_404)")
        .execute(&pool)
        .await
        .expect("bootstrap unlock");
    Some(pool)
}

async fn seed_user(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO auth.users (id) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(id)
        .execute(pool)
        .await
        .expect("auth user");
    sqlx::query("INSERT INTO profiles (user_id, email, plan) VALUES ($1, $2, 'pro')")
        .bind(id)
        .bind(format!("{id}@test.invalid"))
        .execute(pool)
        .await
        .expect("profile");
    id
}

async fn cleanup(pool: &PgPool, id: Uuid) {
    let _ = sqlx::query("DELETE FROM cloud_blobs WHERE user_id = $1")
        .bind(id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM saves WHERE user_id = $1")
        .bind(id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM profiles WHERE user_id = $1")
        .bind(id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM auth.users WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await;
}

fn sha(seed: u8) -> String {
    format!("{:02x}", seed).repeat(32)
}

async fn add_blob(pool: &PgPool, user: Uuid, sha_hex: &str, refcount: i64) {
    sqlx::query(
        "INSERT INTO cloud_blobs (user_id, sha256, size_bytes, refcount)
         VALUES ($1, decode($2, 'hex'), 1024, $3)",
    )
    .bind(user)
    .bind(sha_hex)
    .bind(refcount)
    .execute(pool)
    .await
    .expect("blob row");
}

/// The abandoned-attempt cleanup: only what the retry dropped, and only what
/// nothing else in the account holds.
#[tokio::test]
async fn a_retried_upload_only_drops_what_it_orphaned() {
    let Some(pool) = pool().await else { return };
    let user = seed_user(&pool).await;

    let kept_by_manifest = sha(0x01); // still named by the new attempt
    let held_by_a_blob_row = sha(0x02); // committed earlier, so a live file
    let orphaned = sha(0x03); // uploaded, then dropped from the manifest
    add_blob(&pool, user, &held_by_a_blob_row, 1).await;

    let stale = vec![
        kept_by_manifest.clone(),
        held_by_a_blob_row.clone(),
        orphaned.clone(),
    ];
    let keeping = vec![CasFileEntry {
        relative_path: "save.dat".into(),
        sha256: kept_by_manifest.clone(),
        size_bytes: 1024,
        modified_at: None,
    }];

    let drop = abandoned_blobs_to_drop(&pool, user, &stale, &keeping)
        .await
        .expect("check");
    assert_eq!(
        drop,
        vec![orphaned],
        "only the sha the retry stopped naming and nothing else references"
    );

    cleanup(&pool, user).await;
}

/// Nothing stale means nothing to do, and no query worth running.
#[tokio::test]
async fn a_first_attempt_has_nothing_to_clean_up() {
    let Some(pool) = pool().await else { return };
    let user = seed_user(&pool).await;
    let drop = abandoned_blobs_to_drop(&pool, user, &[], &[])
        .await
        .expect("check");
    assert!(drop.is_empty());
    cleanup(&pool, user).await;
}

/// One account's abandoned blob is not another's to delete, even for the same
/// bytes: keys are per-user, and dedup does not cross accounts.
#[tokio::test]
async fn another_accounts_copy_is_not_a_reference() {
    let Some(pool) = pool().await else { return };
    let mine = seed_user(&pool).await;
    let theirs = seed_user(&pool).await;
    let s = sha(0x04);
    add_blob(&pool, theirs, &s, 1).await;

    let drop = abandoned_blobs_to_drop(&pool, mine, std::slice::from_ref(&s), &[])
        .await
        .expect("check");
    assert_eq!(
        drop,
        vec![s],
        "their live copy says nothing about whether mine is still wanted"
    );

    cleanup(&pool, mine).await;
    cleanup(&pool, theirs).await;
}
