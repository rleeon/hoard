//! Deleting a version has to take its bytes with it, against a real Postgres.
//!
//! Two failures on the same path, found together in sep-2026 by reconciling the
//! bucket against the database.
//!
//! The first was live in production: `sha256` became `bytea` and the GC's
//! `DELETE` kept comparing it to hex text. Postgres refuses `bytea = text`
//! outright, so every freed blob lost its object and kept its row, and the only
//! trace was a `warn!` nobody was reading. 221 dead rows had piled up by the
//! time it was noticed. A test that runs the statement against a bytea column
//! is the only thing that catches that class, since the types only meet at
//! runtime.
//!
//! The second is the reverse leak: when the object refuses to delete, dropping
//! the row anyway strands the bytes, because nothing in the tree ever looks for
//! objects the database does not mention.
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

use hoard_server::cloud::routes::saves::{
    abandoned_blobs_to_drop, defer_blob_row, delete_blob_row, CasFileEntry,
};
use sqlx::{PgPool, Row};
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

async fn blob_exists(pool: &PgPool, user: Uuid, sha_hex: &str) -> bool {
    sqlx::query("SELECT 1 FROM cloud_blobs WHERE user_id = $1 AND sha256 = decode($2, 'hex')")
        .bind(user)
        .bind(sha_hex)
        .fetch_optional(pool)
        .await
        .expect("lookup")
        .is_some()
}

/// The production bug: hex text against a bytea column deletes nothing, and the
/// object is already gone by the time the statement runs.
#[tokio::test]
async fn freeing_a_blob_actually_removes_its_row() {
    let Some(pool) = pool().await else { return };
    let user = seed_user(&pool).await;
    let s = sha(0xab);
    add_blob(&pool, user, &s, 0).await;

    let removed = delete_blob_row(&pool, user, &s).await.expect("delete");
    assert_eq!(removed, 1, "the row has to actually go, not silently miss");
    assert!(!blob_exists(&pool, user, &s).await);

    cleanup(&pool, user).await;
}

/// A delete that matched nothing must say so rather than report success: that
/// is the difference between "already clean" and "the types do not line up".
#[tokio::test]
async fn deleting_a_blob_that_is_not_there_reports_zero() {
    let Some(pool) = pool().await else { return };
    let user = seed_user(&pool).await;
    let removed = delete_blob_row(&pool, user, &sha(0x11)).await.expect("delete");
    assert_eq!(removed, 0);
    cleanup(&pool, user).await;
}

/// When the object will not delete, the row stays and joins the retry queue.
#[tokio::test]
async fn a_blob_whose_object_survives_is_deferred_not_dropped() {
    let Some(pool) = pool().await else { return };
    let user = seed_user(&pool).await;
    let s = sha(0xcd);
    add_blob(&pool, user, &s, 0).await;

    let touched = defer_blob_row(&pool, user, &s).await.expect("defer");
    assert_eq!(touched, 1);
    assert!(
        blob_exists(&pool, user, &s).await,
        "deferring must keep the row: without it nothing looks for the object again"
    );
    let due: Option<time::OffsetDateTime> =
        sqlx::query("SELECT purge_after FROM cloud_blobs WHERE user_id = $1 AND sha256 = decode($2, 'hex')")
            .bind(user)
            .bind(&s)
            .fetch_one(&pool)
            .await
            .expect("row")
            .get(0);
    assert!(due.is_some(), "the retry queue is `purge_after`, so it has to be stamped");

    cleanup(&pool, user).await;
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
    let drop = abandoned_blobs_to_drop(&pool, user, &[], &[]).await.expect("check");
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
