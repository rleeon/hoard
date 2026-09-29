//! Blob refcounts and quota across every path that adds or drops a reference,
//! against a real Postgres.
//!
//! The invariant under test: a blob's `refcount` equals the number of committed
//! versions of live (not archived) saves that reference it, and
//! `profiles.storage_bytes` equals the size of the blobs with `refcount > 0`.
//! In September 2026 a save delete cut short by the client's timeout broke the
//! first half for ten accounts, and the second half followed: users were charged
//! for up to a gigabyte of files that no longer existed.
//!
//! Skipped unless `HOARD_PG_TEST_URL` is set:
//!
//! ```sh
//! docker run -d --name hoard-pg -p 55432:5432 \
//!   -e POSTGRES_PASSWORD=hoard -e POSTGRES_DB=hoard postgres:17
//! export HOARD_PG_TEST_URL=postgres://postgres:hoard@localhost:55432/hoard
//! cargo test -p hoard-server --features cloud --test blob_accounting
//! ```
//!
//! **Never point it at production.** It runs migrations on whatever it is given.

#![cfg(feature = "cloud")]

use hoard_server::cloud::state::CloudState;
use hoard_server::cloud::{integrity, r2, reconcile};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool() -> Option<PgPool> {
    let url = std::env::var("HOARD_PG_TEST_URL").ok()?;
    let pool = hoard_server::cloud::db::connect(&url, 5)
        .await
        .expect("connect to the test database");
    // Same bootstrap as `blob_sha_paths`, lock on one connection included.
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

/// A `CloudState` whose R2 points at the discard port, like `blob_sha_paths`:
/// every object call is refused at once, which is exactly the "R2 delete
/// failed" branch the deferral logic has to survive.
async fn state_for(pool: PgPool) -> CloudState {
    let example = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../deploy/config.cloud.toml.example"
    );
    let base = std::fs::read_to_string(example).expect("read the cloud config example");
    let dir = std::env::temp_dir().join(format!("hoard-acct-test-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("config.toml");
    std::fs::write(
        &path,
        base.replace(r#"endpoint = """#, r#"endpoint = "http://127.0.0.1:9""#)
            .replace(
                r#"supabase_jwks_url = """#,
                r#"supabase_jwks_url = "http://127.0.0.1:9/jwks""#,
            )
            .replace(r#"access_key_id = """#, r#"access_key_id = "test""#)
            .replace(r#"secret_access_key = """#, r#"secret_access_key = "test""#),
    )
    .expect("write config");
    let config = hoard_server::config::Config::load(&path).expect("load config");
    let r2 = hoard_server::cloud::r2::R2Store::from_config(
        &config.cloud.as_ref().expect("cloud section").r2,
    )
    .await
    .expect("r2 client");
    CloudState::for_test(pool, config, std::sync::Arc::new(r2))
}

/// A `CloudState` on a real S3 endpoint, for the tests that list and delete
/// objects. `None` (skip) unless `HOARD_R2_TEST_ENDPOINT` names one, e.g. an
/// ephemeral MinIO:
///
/// ```sh
/// docker run -d --rm --name hoard-minio-test -p 127.0.0.1:59000:9000 \
///   -e MINIO_ROOT_USER=hoardtest -e MINIO_ROOT_PASSWORD=hoardtest-secret \
///   quay.io/minio/minio server /data
/// curl --aws-sigv4 "aws:amz:us-east-1:s3" --user hoardtest:hoardtest-secret \
///   -X PUT http://127.0.0.1:59000/hoard-test
/// export HOARD_R2_TEST_ENDPOINT=http://127.0.0.1:59000
/// ```
async fn bucket_state(pool: PgPool) -> Option<CloudState> {
    let endpoint = std::env::var("HOARD_R2_TEST_ENDPOINT").ok()?;
    let example = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../deploy/config.cloud.toml.example"
    );
    let base = std::fs::read_to_string(example).expect("read the cloud config example");
    let dir = std::env::temp_dir().join(format!("hoard-acct-s3-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("config.toml");
    std::fs::write(
        &path,
        base.replace(r#"endpoint = """#, &format!(r#"endpoint = "{endpoint}""#))
            .replace(
                r#"bucket = "hoard-snapshots-prod""#,
                r#"bucket = "hoard-test""#,
            )
            .replace(r#"region = "auto""#, r#"region = "us-east-1""#)
            .replace(
                r#"supabase_jwks_url = """#,
                r#"supabase_jwks_url = "http://127.0.0.1:9/jwks""#,
            )
            .replace(r#"access_key_id = """#, r#"access_key_id = "hoardtest""#)
            .replace(
                r#"secret_access_key = """#,
                r#"secret_access_key = "hoardtest-secret""#,
            ),
    )
    .expect("write config");
    let config = hoard_server::config::Config::load(&path).expect("load config");
    let r2 = hoard_server::cloud::r2::R2Store::from_config(
        &config.cloud.as_ref().expect("cloud section").r2,
    )
    .await
    .expect("r2 client");
    Some(CloudState::for_test(pool, config, std::sync::Arc::new(r2)))
}

async fn put_blob(state: &CloudState, user: Uuid, seed: u16, size: usize) {
    state
        .r2
        .put_object(&r2::key_for_blob(user, &sha(seed)), vec![7u8; size])
        .await
        .expect("put object");
}

async fn in_bucket(state: &CloudState, user: Uuid, seed: u16) -> bool {
    state
        .r2
        .head(&r2::key_for_blob(user, &sha(seed)))
        .await
        .expect("head")
        .is_some()
}

/// Three days on: every object in these tests is past `reconcile::MIN_AGE`.
fn later() -> i64 {
    time::OffsetDateTime::now_utc().unix_timestamp() + 3 * 24 * 60 * 60
}

/// 64 hex chars, distinct per seed.
fn sha(seed: u16) -> String {
    format!("{seed:04x}{}", "a".repeat(60))
}

async fn new_user(pool: &PgPool) -> Uuid {
    let user = Uuid::new_v4();
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
    user
}

async fn new_save(pool: &PgPool, user: Uuid, slug: &str) -> String {
    let save_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO saves (id, user_id, game_slug, label, latest_version_num)
         VALUES ($1, $2, $3, 'main', 0)",
    )
    .bind(&save_id)
    .bind(user)
    .bind(slug)
    .execute(pool)
    .await
    .expect("save");
    save_id
}

/// Write a version and its manifest. `committed` decides whether it also takes
/// its blob references the way `cas_commit` does (+1 per distinct sha, a new
/// blob inserted at 1) and advances the head; a pending one takes none.
async fn add_version(
    pool: &PgPool,
    user: Uuid,
    save_id: &str,
    num: i64,
    files: &[(u16, i64)],
    committed: bool,
) {
    let mut tx = pool.begin().await.expect("tx");
    let (vid,): (i64,) = sqlx::query_as(
        "INSERT INTO save_versions
           (save_id, version_num, size_bytes, sha256, r2_key, file_count, content_addressed)
         VALUES ($1, $2, $3, $4, '', $5, TRUE) RETURNING id",
    )
    .bind(save_id)
    .bind(num)
    .bind(files.iter().map(|f| f.1).sum::<i64>())
    .bind(if committed { "digest" } else { "" })
    .bind(files.len() as i64)
    .fetch_one(&mut *tx)
    .await
    .expect("version");
    for (seed, size) in files {
        let (eid,): (i64,) = sqlx::query_as(
            "INSERT INTO file_entries (save_id, relative_path, sha256, size_bytes)
             VALUES ($1, $2, decode($3, 'hex'), $4)
             ON CONFLICT (save_id, relative_path, sha256) DO UPDATE SET size_bytes = EXCLUDED.size_bytes
             RETURNING id",
        )
        .bind(save_id)
        .bind(format!("f{seed}.sav"))
        .bind(sha(*seed))
        .bind(size)
        .fetch_one(&mut *tx)
        .await
        .expect("entry");
        sqlx::query(
            "INSERT INTO version_files (version_id, entry_id, modified_at) VALUES ($1, $2, 0)",
        )
        .bind(vid)
        .bind(eid)
        .execute(&mut *tx)
        .await
        .expect("version file");
    }
    if committed {
        let mut seen = std::collections::BTreeMap::new();
        for (seed, size) in files {
            seen.entry(*seed).or_insert(*size);
        }
        for (seed, size) in seen {
            sqlx::query(
                "INSERT INTO cloud_blobs (user_id, sha256, size_bytes, refcount)
                 VALUES ($1, decode($2, 'hex'), $3, 1)
                 ON CONFLICT (user_id, sha256)
                 DO UPDATE SET refcount = cloud_blobs.refcount + 1, purge_after = NULL",
            )
            .bind(user)
            .bind(sha(seed))
            .bind(size)
            .execute(&mut *tx)
            .await
            .expect("blob");
        }
        sqlx::query(
            "UPDATE saves SET latest_version_num = $1 WHERE id = $2 AND latest_version_num < $1",
        )
        .bind(num)
        .bind(save_id)
        .execute(&mut *tx)
        .await
        .expect("head");
    }
    tx.commit().await.expect("commit");
}

async fn refcount(pool: &PgPool, user: Uuid, seed: u16) -> Option<(i64, bool)> {
    sqlx::query_as(
        "SELECT refcount, purge_after IS NOT NULL FROM cloud_blobs
          WHERE user_id = $1 AND sha256 = decode($2, 'hex')",
    )
    .bind(user)
    .bind(sha(seed))
    .fetch_optional(pool)
    .await
    .expect("refcount")
}

async fn storage(pool: &PgPool, user: Uuid) -> i64 {
    sqlx::query_scalar("SELECT storage_bytes FROM profiles WHERE user_id = $1")
        .bind(user)
        .fetch_one(pool)
        .await
        .expect("storage")
}

async fn audit(pool: &PgPool, users: &[Uuid]) -> integrity::Report {
    integrity::audit(pool, Some(users)).await.expect("audit")
}

async fn cleanup(pool: &PgPool, users: &[Uuid]) {
    for user in users {
        let _ = sqlx::query("DELETE FROM saves WHERE user_id = $1")
            .bind(user)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM cloud_blobs WHERE user_id = $1")
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
}

// ---- level 1: the read-only audit

/// Two saves sharing a file, one of them with a pending attempt on top: the
/// shape every other test starts from, and it has to read as clean, or the
/// audit would page someone for ordinary use.
#[tokio::test]
async fn audit_is_clean_on_ordinary_use() {
    let Some(pool) = pool().await else { return };
    let user = new_user(&pool).await;
    let a = new_save(&pool, user, "game-a").await;
    let b = new_save(&pool, user, "game-b").await;
    add_version(&pool, user, &a, 1, &[(1, 100), (2, 200)], true).await;
    add_version(&pool, user, &a, 2, &[(1, 100), (3, 300)], true).await;
    add_version(&pool, user, &b, 1, &[(1, 100)], true).await;
    add_version(&pool, user, &b, 2, &[(1, 100), (4, 400)], false).await;

    assert_eq!(refcount(&pool, user, 1).await, Some((3, false)));
    assert_eq!(storage(&pool, user).await, 600);
    let r = audit(&pool, &[user]).await;
    assert!(r.is_clean(), "{r:?}");
    cleanup(&pool, &[user]).await;
}

/// The September 2026 leak, exactly: a blob still charged after the only
/// version that referenced it is gone.
#[tokio::test]
async fn audit_finds_blobs_charged_for_nothing() {
    let Some(pool) = pool().await else { return };
    let user = new_user(&pool).await;
    let a = new_save(&pool, user, "game-a").await;
    add_version(&pool, user, &a, 1, &[(1, 100), (2, 5000)], true).await;
    // The row goes, the release never runs.
    sqlx::query("DELETE FROM saves WHERE id = $1")
        .bind(&a)
        .execute(&pool)
        .await
        .unwrap();

    let r = audit(&pool, &[user]).await;
    assert_eq!(r.charged_for_nothing_blobs, 2);
    assert_eq!(r.charged_for_nothing_bytes, 5100);
    assert_eq!(r.charged_for_nothing_accounts, vec![user]);
    assert_eq!(r.under_counted_blobs, 0);
    // The trigger kept storage_bytes in step with the rows: this is not drift,
    // it is rows that should not be charged at all.
    assert!(r.drift_accounts.is_empty());
    cleanup(&pool, &[user]).await;
}

#[tokio::test]
async fn audit_finds_under_counted_stuck_and_drift() {
    let Some(pool) = pool().await else { return };
    let user = new_user(&pool).await;
    let a = new_save(&pool, user, "game-a").await;
    add_version(&pool, user, &a, 1, &[(1, 100)], true).await;
    add_version(&pool, user, &a, 2, &[(1, 100), (2, 200)], true).await;
    // Under: two versions reference sha 1, the row says one.
    sqlx::query(
        "UPDATE cloud_blobs SET refcount = 1 WHERE user_id = $1 AND sha256 = decode($2, 'hex')",
    )
    .bind(user)
    .bind(sha(1))
    .execute(&pool)
    .await
    .unwrap();
    // Stuck: an unreferenced row at 0 that nothing will ever sweep.
    sqlx::query(
        "INSERT INTO cloud_blobs (user_id, sha256, size_bytes, refcount) VALUES ($1, decode($2, 'hex'), 50, 0)",
    )
    .bind(user)
    .bind(sha(9))
    .execute(&pool)
    .await
    .unwrap();
    // Drift: someone wrote around the trigger.
    sqlx::query("UPDATE profiles SET storage_bytes = storage_bytes + 7 WHERE user_id = $1")
        .bind(user)
        .execute(&pool)
        .await
        .unwrap();

    let r = audit(&pool, &[user]).await;
    assert_eq!(r.under_counted_blobs, 1);
    assert_eq!(r.under_counted_accounts, vec![user]);
    assert_eq!(r.stuck_blobs, 1);
    assert_eq!(r.drift_accounts, vec![user]);
    assert_eq!(r.drift_bytes, 7);
    assert_eq!(r.charged_for_nothing_blobs, 0);
    cleanup(&pool, &[user]).await;
}

/// An archived save hands its references back and keeps its versions. That is
/// by design and must not read as under-counted.
#[tokio::test]
async fn audit_leaves_archived_saves_alone() {
    let Some(pool) = pool().await else { return };
    let user = new_user(&pool).await;
    let a = new_save(&pool, user, "game-a").await;
    add_version(&pool, user, &a, 1, &[(1, 100)], true).await;
    sqlx::query("UPDATE saves SET archived_at = now() WHERE id = $1")
        .bind(&a)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE cloud_blobs SET refcount = 0, purge_after = now() + interval '7 days' WHERE user_id = $1",
    )
    .bind(user)
    .execute(&pool)
    .await
    .unwrap();

    let r = audit(&pool, &[user]).await;
    assert!(r.is_clean(), "{r:?}");
    cleanup(&pool, &[user]).await;
}

// ---- level 2: the bucket against the rows, counting only

/// The four kinds of object an account can hold, side by side: a live file, a
/// file an upload in flight has promised, litter, and a live row whose object
/// is gone. Only the litter is an orphan, and nothing is deleted.
#[tokio::test]
async fn reconcile_counts_orphans_and_missing_without_deleting() {
    let Some(pool) = pool().await else { return };
    let Some(state) = bucket_state(pool.clone()).await else {
        return;
    };
    let user = new_user(&pool).await;
    let a = new_save(&pool, user, "game-a").await;
    add_version(&pool, user, &a, 1, &[(1, 10), (2, 20)], true).await;
    add_version(&pool, user, &a, 2, &[(1, 10), (3, 30)], false).await;
    put_blob(&state, user, 1, 10).await; // live
    put_blob(&state, user, 3, 30).await; // promised by the pending version
    put_blob(&state, user, 4, 40).await; // litter
                                         // sha 2 has a live row and no object.

    let f = reconcile::reconcile(&state, Some(&[user]), later(), false)
        .await
        .expect("reconcile");
    assert_eq!(f.orphan_objects, 1);
    assert_eq!(f.orphan_bytes, 40);
    assert_eq!(f.missing_objects, 1);
    assert_eq!(f.missing_accounts, vec![user]);
    assert_eq!(f.deleted_objects, 0);
    assert!(in_bucket(&state, user, 4).await);

    // Today, the same litter is too young to call.
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    let f = reconcile::reconcile(&state, Some(&[user]), now, false)
        .await
        .expect("reconcile");
    assert_eq!(f.orphan_objects, 0);
    cleanup(&pool, &[user]).await;
}

// ---- level 3: the blob GC

async fn due(pool: &PgPool, user: Uuid, seed: u16, size: i64) {
    sqlx::query(
        "INSERT INTO cloud_blobs (user_id, sha256, size_bytes, refcount, purge_after)
         VALUES ($1, decode($2, 'hex'), $3, 0, now() - interval '1 minute')",
    )
    .bind(user)
    .bind(sha(seed))
    .bind(size)
    .execute(pool)
    .await
    .expect("due blob");
}

/// Due rows lose their object and then their row; a row that is live, or not
/// due yet, is left alone. The row really has to go: in sep-2026 the GC's
/// `DELETE` compared the bytea `sha256` with hex text, matched nothing, and 221
/// rows whose objects were already gone had piled up before anyone noticed.
#[tokio::test]
async fn gc_deletes_due_blobs_and_nothing_else() {
    let Some(pool) = pool().await else { return };
    let Some(state) = bucket_state(pool.clone()).await else {
        return;
    };
    let user = new_user(&pool).await;
    due(&pool, user, 1, 10).await;
    put_blob(&state, user, 1, 10).await;
    let a = new_save(&pool, user, "game-a").await;
    add_version(&pool, user, &a, 1, &[(2, 20)], true).await;
    put_blob(&state, user, 2, 20).await;
    sqlx::query(
        "INSERT INTO cloud_blobs (user_id, sha256, size_bytes, refcount, purge_after)
         VALUES ($1, decode($2, 'hex'), 30, 0, now() + interval '1 day')",
    )
    .bind(user)
    .bind(sha(3))
    .execute(&pool)
    .await
    .unwrap();
    put_blob(&state, user, 3, 30).await;

    let n = hoard_server::cloud::archive::purge_due_blobs(&state, Some(&[user]))
        .await
        .expect("gc");
    assert_eq!(n, 1);
    assert_eq!(refcount(&pool, user, 1).await, None);
    assert!(!in_bucket(&state, user, 1).await);
    assert_eq!(refcount(&pool, user, 2).await, Some((1, false)));
    assert!(in_bucket(&state, user, 2).await);
    assert_eq!(refcount(&pool, user, 3).await, Some((0, true)));
    assert!(in_bucket(&state, user, 3).await);
    assert!(audit(&pool, &[user]).await.is_clean());
    cleanup(&pool, &[user]).await;
}

/// When the bucket refuses the delete, the row stays and comes back tomorrow.
/// It used to be dropped anyway, stranding the object with nothing pointing at
/// it.
#[tokio::test]
async fn gc_keeps_the_row_when_the_bucket_refuses() {
    let Some(pool) = pool().await else { return };
    let state = state_for(pool.clone()).await;
    let user = new_user(&pool).await;
    due(&pool, user, 1, 10).await;

    let n = hoard_server::cloud::archive::purge_due_blobs(&state, Some(&[user]))
        .await
        .expect("gc");
    assert_eq!(n, 0);
    let later: bool =
        sqlx::query_scalar("SELECT purge_after > now() FROM cloud_blobs WHERE user_id = $1")
            .bind(user)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(later, "pushed to tomorrow");
    cleanup(&pool, &[user]).await;
}

/// A row somebody else holds (an upload reviving it) is skipped, not waited on
/// and not deleted.
#[tokio::test]
async fn gc_skips_rows_another_transaction_holds() {
    let Some(pool) = pool().await else { return };
    let Some(state) = bucket_state(pool.clone()).await else {
        return;
    };
    let user = new_user(&pool).await;
    due(&pool, user, 1, 10).await;
    put_blob(&state, user, 1, 10).await;

    let mut holder = pool.begin().await.unwrap();
    sqlx::query("SELECT 1 FROM cloud_blobs WHERE user_id = $1 FOR UPDATE")
        .bind(user)
        .execute(&mut *holder)
        .await
        .unwrap();
    let n = hoard_server::cloud::archive::purge_due_blobs(&state, Some(&[user]))
        .await
        .expect("gc");
    assert_eq!(n, 0);
    // The holder revives it, as a commit re-referencing the file would.
    sqlx::query("UPDATE cloud_blobs SET refcount = 1, purge_after = NULL WHERE user_id = $1")
        .bind(user)
        .execute(&mut *holder)
        .await
        .unwrap();
    holder.commit().await.unwrap();

    let n = hoard_server::cloud::archive::purge_due_blobs(&state, Some(&[user]))
        .await
        .expect("gc");
    assert_eq!(n, 0);
    assert_eq!(refcount(&pool, user, 1).await, Some((1, false)));
    assert!(in_bucket(&state, user, 1).await);
    cleanup(&pool, &[user]).await;
}

// ---- level 4: archive and reactivate

/// Save `a` shares file 1 with save `b` and has an upload in flight naming file
/// 1 and a new file 5. Archiving `a` hands back exactly what its two committed
/// versions took: file 1 keeps `b`'s reference, `a`'s own files drop to 0 with
/// a week's grace, and the pending version takes nothing from anyone.
#[tokio::test]
async fn archive_hands_back_only_committed_references() {
    let Some(pool) = pool().await else { return };
    let state = state_for(pool.clone()).await;
    let user = new_user(&pool).await;
    let a = new_save(&pool, user, "game-a").await;
    let b = new_save(&pool, user, "game-b").await;
    add_version(&pool, user, &a, 1, &[(1, 100), (2, 200)], true).await;
    add_version(&pool, user, &a, 2, &[(1, 100), (3, 300)], true).await;
    add_version(&pool, user, &a, 3, &[(1, 100), (5, 500)], false).await;
    add_version(&pool, user, &b, 1, &[(1, 100)], true).await;
    assert_eq!(storage(&pool, user).await, 600);

    let out = hoard_server::cloud::archive::archive_save(&state, user, &a)
        .await
        .expect("archive");
    assert_eq!(out.freed_bytes, 500);
    assert_eq!(refcount(&pool, user, 1).await, Some((1, false)));
    assert_eq!(refcount(&pool, user, 2).await, Some((0, true)));
    assert_eq!(refcount(&pool, user, 3).await, Some((0, true)));
    assert_eq!(storage(&pool, user).await, 100);
    assert!(audit(&pool, &[user]).await.is_clean());

    hoard_server::cloud::archive::reactivate_save(&state, user, &a)
        .await
        .expect("reactivate");
    assert_eq!(refcount(&pool, user, 1).await, Some((3, false)));
    assert_eq!(refcount(&pool, user, 2).await, Some((1, false)));
    assert_eq!(refcount(&pool, user, 3).await, Some((1, false)));
    assert_eq!(storage(&pool, user).await, 600);
    assert!(audit(&pool, &[user]).await.is_clean());
    cleanup(&pool, &[user]).await;
}

/// Two archive requests racing used to both pass the "already archived?" check
/// and hand the references back twice.
#[tokio::test]
async fn concurrent_archives_release_once() {
    let Some(pool) = pool().await else { return };
    let state = state_for(pool.clone()).await;
    let user = new_user(&pool).await;
    let a = new_save(&pool, user, "game-a").await;
    let b = new_save(&pool, user, "game-b").await;
    add_version(&pool, user, &a, 1, &[(1, 100)], true).await;
    add_version(&pool, user, &b, 1, &[(1, 100)], true).await;

    let (s1, s2) = (state.clone(), state.clone());
    let (a1, a2) = (a.clone(), a.clone());
    let (r1, r2) = tokio::join!(
        tokio::spawn(
            async move { hoard_server::cloud::archive::archive_save(&s1, user, &a1).await }
        ),
        tokio::spawn(
            async move { hoard_server::cloud::archive::archive_save(&s2, user, &a2).await }
        ),
    );
    r1.unwrap().expect("first archive");
    r2.unwrap().expect("second archive");
    assert_eq!(refcount(&pool, user, 1).await, Some((1, false)));
    assert!(audit(&pool, &[user]).await.is_clean());
    cleanup(&pool, &[user]).await;
}

// ---- level 5: deleting saves and versions

fn ctx(user: Uuid) -> hoard_server::cloud::auth::CloudUser {
    hoard_server::cloud::auth::CloudUser {
        user_id: user,
        email: format!("{user}@test.invalid"),
        role: "authenticated".into(),
        avatar_url: None,
        display_name: None,
    }
}

async fn delete_save(state: &CloudState, user: Uuid, save_id: &str) -> u16 {
    use axum::response::IntoResponse;
    match hoard_server::cloud::routes::saves::delete_save(
        axum::extract::State(state.clone()),
        axum::Extension(ctx(user)),
        axum::extract::Path(save_id.to_string()),
    )
    .await
    {
        Ok(r) => r.status().as_u16(),
        Err(e) => e.into_response().status().as_u16(),
    }
}

async fn delete_version(state: &CloudState, user: Uuid, save_id: &str, v: i64) -> u16 {
    use axum::response::IntoResponse;
    match hoard_server::cloud::routes::saves::delete_version(
        axum::extract::State(state.clone()),
        axum::Extension(ctx(user)),
        axum::extract::Path((save_id.to_string(), v)),
    )
    .await
    {
        Ok(r) => r.status().as_u16(),
        Err(e) => e.into_response().status().as_u16(),
    }
}

/// Same shape as the archive test. The old delete counted the pending version
/// too, took three references off file 1, and left save `b` pointing at a blob
/// at 0 that the GC would have deleted.
#[tokio::test]
async fn deleting_a_save_hands_back_exactly_its_committed_references() {
    let Some(pool) = pool().await else { return };
    let state = state_for(pool.clone()).await;
    let user = new_user(&pool).await;
    let a = new_save(&pool, user, "game-a").await;
    let b = new_save(&pool, user, "game-b").await;
    add_version(&pool, user, &a, 1, &[(1, 100), (2, 200)], true).await;
    add_version(&pool, user, &a, 2, &[(1, 100), (3, 300)], true).await;
    add_version(&pool, user, &a, 3, &[(1, 100), (5, 500)], false).await;
    add_version(&pool, user, &b, 1, &[(1, 100)], true).await;

    // The bucket is unreachable: the delete must not need it.
    assert_eq!(delete_save(&state, user, &a).await, 204);
    assert_eq!(refcount(&pool, user, 1).await, Some((1, false)));
    assert_eq!(refcount(&pool, user, 2).await, Some((0, true)));
    assert_eq!(refcount(&pool, user, 3).await, Some((0, true)));
    assert_eq!(storage(&pool, user).await, 100);
    assert!(audit(&pool, &[user]).await.is_clean());
    assert_eq!(delete_save(&state, user, &a).await, 404);
    cleanup(&pool, &[user]).await;
}

/// An archived save already handed its references back; deleting it must not
/// take them a second time from the saves it shares files with.
#[tokio::test]
async fn deleting_an_archived_save_releases_nothing_twice() {
    let Some(pool) = pool().await else { return };
    let state = state_for(pool.clone()).await;
    let user = new_user(&pool).await;
    let a = new_save(&pool, user, "game-a").await;
    let b = new_save(&pool, user, "game-b").await;
    add_version(&pool, user, &a, 1, &[(1, 100), (2, 200)], true).await;
    add_version(&pool, user, &b, 1, &[(1, 100)], true).await;
    hoard_server::cloud::archive::archive_save(&state, user, &a)
        .await
        .expect("archive");

    assert_eq!(delete_save(&state, user, &a).await, 204);
    assert_eq!(refcount(&pool, user, 1).await, Some((1, false)));
    assert_eq!(refcount(&pool, user, 2).await, Some((0, true)));
    assert!(audit(&pool, &[user]).await.is_clean());
    cleanup(&pool, &[user]).await;
}

#[tokio::test]
async fn deleting_versions_one_by_one() {
    let Some(pool) = pool().await else { return };
    let state = state_for(pool.clone()).await;
    let user = new_user(&pool).await;
    let a = new_save(&pool, user, "game-a").await;
    add_version(&pool, user, &a, 1, &[(1, 100), (2, 200)], true).await;
    add_version(&pool, user, &a, 2, &[(1, 100), (3, 300)], true).await;
    add_version(&pool, user, &a, 3, &[(1, 100), (5, 500)], false).await;

    // The head goes: file 3 is free, file 1 keeps v1's reference.
    assert_eq!(delete_version(&state, user, &a, 2).await, 204);
    assert_eq!(refcount(&pool, user, 1).await, Some((1, false)));
    assert_eq!(refcount(&pool, user, 3).await, Some((0, true)));
    let head: i64 = sqlx::query_scalar("SELECT latest_version_num FROM saves WHERE id = $1")
        .bind(&a)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(head, 1);
    // A pending version took nothing and gives nothing back.
    assert_eq!(delete_version(&state, user, &a, 3).await, 204);
    assert_eq!(refcount(&pool, user, 1).await, Some((1, false)));
    // The last committed version takes the save with it.
    assert_eq!(delete_version(&state, user, &a, 1).await, 204);
    assert_eq!(refcount(&pool, user, 1).await, Some((0, true)));
    let gone: bool = sqlx::query_scalar("SELECT NOT EXISTS (SELECT 1 FROM saves WHERE id = $1)")
        .bind(&a)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(gone);
    assert_eq!(storage(&pool, user).await, 0);
    assert!(audit(&pool, &[user]).await.is_clean());
    cleanup(&pool, &[user]).await;
}

#[tokio::test]
async fn concurrent_deletes_of_one_save_release_once() {
    let Some(pool) = pool().await else { return };
    let state = state_for(pool.clone()).await;
    let user = new_user(&pool).await;
    let a = new_save(&pool, user, "game-a").await;
    let b = new_save(&pool, user, "game-b").await;
    add_version(&pool, user, &a, 1, &[(1, 100)], true).await;
    add_version(&pool, user, &b, 1, &[(1, 100)], true).await;

    let (s1, s2) = (state.clone(), state.clone());
    let (a1, a2) = (a.clone(), a.clone());
    let (r1, r2) = tokio::join!(
        tokio::spawn(async move { delete_save(&s1, user, &a1).await }),
        tokio::spawn(async move { delete_save(&s2, user, &a2).await }),
    );
    let mut codes = [r1.unwrap(), r2.unwrap()];
    codes.sort();
    assert_eq!(codes, [204, 404]);
    assert_eq!(refcount(&pool, user, 1).await, Some((1, false)));
    assert!(audit(&pool, &[user]).await.is_clean());
    cleanup(&pool, &[user]).await;
}

/// The September 2026 bug, as it happened: the client hangs up halfway. The
/// release used to die with the request, after the save was gone and before the
/// references were, leaving them charged forever. Now the whole delete is one
/// transaction that outlives the request: hanging up changes nothing about how
/// it ends.
#[tokio::test]
async fn a_delete_the_client_abandons_still_finishes_whole() {
    let Some(pool) = pool().await else { return };
    let state = state_for(pool.clone()).await;
    let user = new_user(&pool).await;
    let a = new_save(&pool, user, "game-a").await;
    add_version(&pool, user, &a, 1, &[(1, 100), (2, 200)], true).await;

    // Park the delete on a blob lock, then drop the handler the way hyper does
    // when the connection closes.
    let mut holder = pool.begin().await.unwrap();
    sqlx::query("SELECT 1 FROM cloud_blobs WHERE user_id = $1 FOR UPDATE")
        .bind(user)
        .execute(&mut *holder)
        .await
        .unwrap();
    let (s1, a1) = (state.clone(), a.clone());
    let handler = tokio::spawn(async move { delete_save(&s1, user, &a1).await });
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    handler.abort();
    let _ = handler.await;
    // Nothing is half done while it waits.
    assert_eq!(storage(&pool, user).await, 300);
    holder.rollback().await.unwrap();

    let mut gone = false;
    for _ in 0..50 {
        gone = sqlx::query_scalar("SELECT NOT EXISTS (SELECT 1 FROM saves WHERE id = $1)")
            .bind(&a)
            .fetch_one(&pool)
            .await
            .unwrap();
        if gone {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert!(gone, "the delete finished without anyone waiting for it");
    assert_eq!(storage(&pool, user).await, 0);
    assert!(audit(&pool, &[user]).await.is_clean());
    cleanup(&pool, &[user]).await;
}

/// The max-versions cap prunes through `purge_one`: same accounting.
#[tokio::test]
async fn pruning_to_the_version_cap_keeps_the_books_straight() {
    let Some(pool) = pool().await else { return };
    let state = state_for(pool.clone()).await;
    let user = new_user(&pool).await;
    let a = new_save(&pool, user, "game-a").await;
    add_version(&pool, user, &a, 1, &[(1, 100), (2, 200)], true).await;
    add_version(&pool, user, &a, 2, &[(1, 100), (3, 300)], true).await;
    add_version(&pool, user, &a, 3, &[(1, 100), (4, 400)], true).await;
    sqlx::query("UPDATE profiles SET max_versions = 1 WHERE user_id = $1")
        .bind(user)
        .execute(&pool)
        .await
        .unwrap();

    let pruned = hoard_server::cloud::purge::prune_version_caps(&state, user)
        .await
        .expect("prune");
    assert_eq!(pruned, 2);
    assert_eq!(refcount(&pool, user, 1).await, Some((1, false)));
    assert_eq!(refcount(&pool, user, 2).await, Some((0, true)));
    assert_eq!(refcount(&pool, user, 3).await, Some((0, true)));
    assert_eq!(storage(&pool, user).await, 500);
    assert!(audit(&pool, &[user]).await.is_clean());
    cleanup(&pool, &[user]).await;
}

// ---- level 6: a commit reusing blobs the GC is after

async fn commit(state: &CloudState, user: Uuid, save_id: &str, v: i64) -> u16 {
    use axum::response::IntoResponse;
    match hoard_server::cloud::routes::saves::cas_commit(
        axum::extract::State(state.clone()),
        axum::Extension(ctx(user)),
        axum::extract::Path((save_id.to_string(), v)),
        None,
    )
    .await
    {
        Ok(r) => r.status().as_u16(),
        Err(e) => e.into_response().status().as_u16(),
    }
}

#[tokio::test]
async fn an_ordinary_commit_takes_its_references() {
    let Some(pool) = pool().await else { return };
    let Some(state) = bucket_state(pool.clone()).await else {
        return;
    };
    let user = new_user(&pool).await;
    let a = new_save(&pool, user, "game-a").await;
    add_version(&pool, user, &a, 1, &[(1, 10), (2, 20)], false).await;
    put_blob(&state, user, 1, 10).await;
    put_blob(&state, user, 2, 20).await;

    assert_eq!(commit(&state, user, &a, 1).await, 200);
    assert_eq!(refcount(&pool, user, 1).await, Some((1, false)));
    assert_eq!(storage(&pool, user).await, 30);
    assert!(audit(&pool, &[user]).await.is_clean());
    cleanup(&pool, &[user]).await;
}

/// Delete a game and add it straight back: the rows are still there at 0, the
/// commit takes them back without a byte uploaded, and the quota returns.
#[tokio::test]
async fn re_adding_a_deleted_game_revives_its_blobs() {
    let Some(pool) = pool().await else { return };
    let Some(state) = bucket_state(pool.clone()).await else {
        return;
    };
    let user = new_user(&pool).await;
    let a = new_save(&pool, user, "game-a").await;
    add_version(&pool, user, &a, 1, &[(1, 10)], false).await;
    put_blob(&state, user, 1, 10).await;
    assert_eq!(commit(&state, user, &a, 1).await, 200);
    assert_eq!(delete_save(&state, user, &a).await, 204);
    assert_eq!(refcount(&pool, user, 1).await, Some((0, true)));
    assert_eq!(storage(&pool, user).await, 0);

    let b = new_save(&pool, user, "game-a-again").await;
    add_version(&pool, user, &b, 1, &[(1, 10)], false).await;
    assert_eq!(commit(&state, user, &b, 1).await, 200);
    assert_eq!(refcount(&pool, user, 1).await, Some((1, false)));
    assert_eq!(storage(&pool, user).await, 10);
    assert!(audit(&pool, &[user]).await.is_clean());
    cleanup(&pool, &[user]).await;
}

/// A row at 0 whose object is already gone must not be revived: the version
/// would point at bytes that don't exist. The commit refuses, drops the row, and
/// the retry is asked for the upload.
#[tokio::test]
async fn reviving_a_blob_whose_bytes_are_gone_is_refused_and_heals() {
    let Some(pool) = pool().await else { return };
    let Some(state) = bucket_state(pool.clone()).await else {
        return;
    };
    let user = new_user(&pool).await;
    sqlx::query(
        "INSERT INTO cloud_blobs (user_id, sha256, size_bytes, refcount, purge_after)
         VALUES ($1, decode($2, 'hex'), 10, 0, now() + interval '1 day')",
    )
    .bind(user)
    .bind(sha(1))
    .execute(&pool)
    .await
    .unwrap();
    let a = new_save(&pool, user, "game-a").await;
    add_version(&pool, user, &a, 1, &[(1, 10)], false).await;

    assert_eq!(commit(&state, user, &a, 1).await, 400);
    assert_eq!(refcount(&pool, user, 1).await, None, "the dead row is gone");
    let pending: bool =
        sqlx::query_scalar("SELECT sha256 = '' FROM save_versions WHERE save_id = $1")
            .bind(&a)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(pending, "nothing was committed");

    // The retry uploads the bytes, and lands.
    put_blob(&state, user, 1, 10).await;
    assert_eq!(commit(&state, user, &a, 1).await, 200);
    assert_eq!(refcount(&pool, user, 1).await, Some((1, false)));
    assert!(audit(&pool, &[user]).await.is_clean());
    cleanup(&pool, &[user]).await;
}

/// The GC takes a row between the commit's first look and its lock. The GC
/// deletes the object before the row, so the bytes are gone: refuse.
#[tokio::test]
async fn a_blob_collected_under_a_commit_is_not_referenced() {
    let Some(pool) = pool().await else { return };
    let Some(state) = bucket_state(pool.clone()).await else {
        return;
    };
    let user = new_user(&pool).await;
    sqlx::query(
        "INSERT INTO cloud_blobs (user_id, sha256, size_bytes, refcount, purge_after)
         VALUES ($1, decode($2, 'hex'), 10, 0, now() - interval '1 minute')",
    )
    .bind(user)
    .bind(sha(1))
    .execute(&pool)
    .await
    .unwrap();
    put_blob(&state, user, 1, 10).await;
    let a = new_save(&pool, user, "game-a").await;
    add_version(&pool, user, &a, 1, &[(1, 10)], false).await;

    // Play the GC: hold the row, let the commit queue behind it, then delete
    // the object and the row and let go.
    let mut gc = pool.begin().await.unwrap();
    sqlx::query("SELECT 1 FROM cloud_blobs WHERE user_id = $1 FOR UPDATE")
        .bind(user)
        .execute(&mut *gc)
        .await
        .unwrap();
    let (s1, a1) = (state.clone(), a.clone());
    let pending = tokio::spawn(async move { commit(&s1, user, &a1, 1).await });
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    state
        .r2
        .delete_object(&r2::key_for_blob(user, &sha(1)))
        .await
        .unwrap();
    sqlx::query("DELETE FROM cloud_blobs WHERE user_id = $1")
        .bind(user)
        .execute(&mut *gc)
        .await
        .unwrap();
    gc.commit().await.unwrap();

    assert_eq!(pending.await.unwrap(), 400);
    assert_eq!(refcount(&pool, user, 1).await, None);
    assert!(audit(&pool, &[user]).await.is_clean());
    cleanup(&pool, &[user]).await;
}

// ---- level 7: storage accounted once per statement (0064)

async fn lifetime(pool: &PgPool, user: Uuid) -> i64 {
    sqlx::query_scalar("SELECT lifetime_storage_bytes FROM profiles WHERE user_id = $1")
        .bind(user)
        .fetch_one(pool)
        .await
        .expect("lifetime")
}

/// One upsert that inserts some blobs and revives others is two statements'
/// worth of triggers (INSERT and UPDATE); both halves count, and only arriving
/// bytes add to the lifetime total.
#[tokio::test]
async fn an_upsert_that_inserts_and_revives_counts_both() {
    let Some(pool) = pool().await else { return };
    let user = new_user(&pool).await;
    sqlx::query(
        "INSERT INTO cloud_blobs (user_id, sha256, size_bytes, refcount)
         VALUES ($1, decode($2, 'hex'), 100, 0), ($1, decode($3, 'hex'), 200, 1)",
    )
    .bind(user)
    .bind(sha(1))
    .bind(sha(2))
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(storage(&pool, user).await, 200);
    assert_eq!(lifetime(&pool, user).await, 200);

    sqlx::query(
        "INSERT INTO cloud_blobs (user_id, sha256, size_bytes, refcount)
         SELECT $1, decode(s, 'hex'), 300, 1 FROM unnest($2::text[]) AS s
         ON CONFLICT (user_id, sha256) DO UPDATE SET refcount = cloud_blobs.refcount + 1",
    )
    .bind(user)
    .bind(vec![sha(1), sha(2), sha(3)])
    .execute(&pool)
    .await
    .unwrap();
    // sha 1 revived (+100), sha 2 already charged, sha 3 new (+300).
    assert_eq!(storage(&pool, user).await, 600);
    assert_eq!(lifetime(&pool, user).await, 600);

    sqlx::query("UPDATE cloud_blobs SET refcount = 0 WHERE user_id = $1")
        .bind(user)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(storage(&pool, user).await, 0);
    assert_eq!(
        lifetime(&pool, user).await,
        600,
        "leaving never lowers the lifetime total"
    );
    cleanup(&pool, &[user]).await;
}

/// One statement touching several accounts charges each its own share.
#[tokio::test]
async fn one_statement_across_accounts_splits_by_account() {
    let Some(pool) = pool().await else { return };
    let (u1, u2) = (new_user(&pool).await, new_user(&pool).await);
    sqlx::query(
        "INSERT INTO cloud_blobs (user_id, sha256, size_bytes, refcount)
         VALUES ($1, decode($3, 'hex'), 10, 1), ($1, decode($4, 'hex'), 20, 1),
                ($2, decode($3, 'hex'), 40, 1)",
    )
    .bind(u1)
    .bind(u2)
    .bind(sha(1))
    .bind(sha(2))
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(storage(&pool, u1).await, 30);
    assert_eq!(storage(&pool, u2).await, 40);

    sqlx::query("DELETE FROM cloud_blobs WHERE user_id = ANY($1) AND sha256 = decode($2, 'hex')")
        .bind(vec![u1, u2])
        .bind(sha(1))
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(storage(&pool, u1).await, 20);
    assert_eq!(storage(&pool, u2).await, 0);
    cleanup(&pool, &[u1, u2]).await;
}

/// A counter that had already drifted low stops at zero instead of going
/// negative, as the row triggers did.
#[tokio::test]
async fn the_counter_never_goes_below_zero() {
    let Some(pool) = pool().await else { return };
    let user = new_user(&pool).await;
    sqlx::query(
        "INSERT INTO cloud_blobs (user_id, sha256, size_bytes, refcount) VALUES ($1, decode($2, 'hex'), 100, 1)",
    )
    .bind(user)
    .bind(sha(1))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("UPDATE profiles SET storage_bytes = 30 WHERE user_id = $1")
        .bind(user)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM cloud_blobs WHERE user_id = $1")
        .bind(user)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(storage(&pool, user).await, 0);
    cleanup(&pool, &[user]).await;
}

/// Purging an account cascades from `profiles` into `cloud_blobs`; the
/// statement trigger then updates a profile row that is already gone, which
/// must be a no-op and not an error.
#[tokio::test]
async fn purging_an_account_cascades_cleanly() {
    let Some(pool) = pool().await else { return };
    let user = new_user(&pool).await;
    let a = new_save(&pool, user, "game-a").await;
    add_version(&pool, user, &a, 1, &[(1, 100), (2, 200)], true).await;
    sqlx::query("DELETE FROM profiles WHERE user_id = $1")
        .bind(user)
        .execute(&pool)
        .await
        .expect("cascade");
    let left: i64 = sqlx::query_scalar("SELECT count(*) FROM cloud_blobs WHERE user_id = $1")
        .bind(user)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(left, 0);
    cleanup(&pool, &[user]).await;
}

/// The reason for 0064: handing back the references of a save the size of the
/// largest in production. Row by row this was quadratic, 38 s for 20,000 on a
/// laptop; per statement it is one pass.
#[tokio::test]
async fn a_twenty_thousand_blob_release_is_quick() {
    let Some(pool) = pool().await else { return };
    let state = state_for(pool.clone()).await;
    let user = new_user(&pool).await;
    let a = new_save(&pool, user, "huge").await;
    let files: Vec<(u16, i64)> = (0..20_000u16).map(|i| (i, 1000)).collect();
    add_version(&pool, user, &a, 1, &files, true).await;
    assert_eq!(storage(&pool, user).await, 20_000_000);

    let started = std::time::Instant::now();
    assert_eq!(delete_save(&state, user, &a).await, 204);
    let took = started.elapsed();
    assert_eq!(storage(&pool, user).await, 0);
    assert!(took < std::time::Duration::from_secs(10), "took {took:?}");
    eprintln!("20,000-blob delete: {took:?}");
    cleanup(&pool, &[user]).await;
}
