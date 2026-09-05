//! Every cloud path that reads or writes a `sha256`, against a real Postgres.
//!
//! The cloud module talks to the database through runtime `sqlx::query()`, not
//! the `query!` macro, so **nothing here is checked at compile time**. Bind a
//! `String` where the column is `bytea` and it builds, ships, and fails on the
//! first user who touches that route. `cargo check --features cloud` will not
//! save you, and neither will the unit tests: the whole surface is SQL.
//!
//! So this file exists to be the thing that fails first, on a laptop, when the
//! storage type of a sha changes. It is deliberately about *coverage of the
//! query surface* rather than about behaviour, most of which is already pinned
//! elsewhere. Each test drives a real entry point and asserts something small;
//! the point is that the SQL executed at all.
//!
//! The functions that take a `CloudState` get one pointed at a dead R2
//! endpoint. Object deletion is best-effort by design (a failed delete only
//! leaks a blob for a later sweep, it never fails the request), so the SQL runs
//! to completion and only the network call gives up. That is the half being
//! tested.
//!
//! Skipped unless `HOARD_PG_TEST_URL` is set, like `downgrade_grace`:
//!
//! ```sh
//! docker run -d --name hoard-pg -p 55432:5432 \
//!   -e POSTGRES_PASSWORD=hoard -e POSTGRES_DB=hoard postgres:17
//! export HOARD_PG_TEST_URL=postgres://postgres:hoard@localhost:55432/hoard
//! cargo test -p hoard-server --features cloud --test blob_sha_paths
//! ```
//!
//! **Never point it at production.** It runs migrations on whatever it is given.

#![cfg(feature = "cloud")]

use hoard_server::cloud::state::CloudState;
use sqlx::PgPool;
use uuid::Uuid;

/// A sha-shaped string: 64 lowercase hex. `seed` varies the tail so a test can
/// mint several distinct ones without thinking about it.
fn sha(seed: u8) -> String {
    format!("{:02x}{}", seed, "0".repeat(62))
}

async fn pool() -> Option<PgPool> {
    let url = std::env::var("HOARD_PG_TEST_URL").ok()?;
    let pool = hoard_server::cloud::db::connect(&url, 5)
        .await
        .expect("connect to the test database");
    // Same bootstrap dance as `downgrade_grace`: the migrations assume Supabase
    // objects a bare Postgres has never heard of, and the test binaries race
    // each other creating them.
    sqlx::query("SELECT pg_advisory_lock(8_233_119_402)")
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
    sqlx::query("SELECT pg_advisory_unlock(8_233_119_402)")
        .execute(&pool)
        .await
        .expect("bootstrap unlock");
    Some(pool)
}

/// A `CloudState` whose R2 points nowhere reachable. Building the client does
/// not open a connection, so this is free until something actually calls out.
async fn state_for(pool: PgPool) -> CloudState {
    // The committed example, with the R2 endpoint pointed somewhere dead. It
    // is used as the base instead of a hand-written stub because it is the one
    // file guaranteed to carry every field the loader demands; a stub goes
    // stale the day somebody adds a setting, and it fails as a confusing
    // parse error in an unrelated test.
    //
    // Port 9 is discard, so the best-effort R2 deletes these paths make are
    // refused at once instead of hanging the test on a connect timeout.
    let example = concat!(env!("CARGO_MANIFEST_DIR"), "/../../deploy/config.cloud.toml.example");
    let base = std::fs::read_to_string(example).expect("read the cloud config example");
    let dir = std::env::temp_dir().join(format!("hoard-sha-test-{}", Uuid::new_v4()));
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
    .expect("r2 client (building one opens no connection)");
    CloudState::for_test(pool, config, std::sync::Arc::new(r2))
}

/// One user, one save, `versions` committed versions of `files` files each.
/// Every file of every version points at the same shas, which is the shape that
/// matters here: the manifest repeats, the blobs are shared and refcounted.
async fn seed(pool: &PgPool, versions: i64, files: u8) -> (Uuid, String) {
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
    sqlx::query(
        "INSERT INTO saves (id, user_id, game_slug, label, latest_version_num)
         VALUES ($1, $2, 'test-game', 'default', $3)",
    )
    .bind(&save_id)
    .bind(user)
    .bind(versions)
    .execute(pool)
    .await
    .expect("save");

    for f in 0..files {
        sqlx::query(
            "INSERT INTO cloud_blobs (user_id, sha256, size_bytes, refcount)
             VALUES ($1, $2, 1000, $3)",
        )
        .bind(user)
        .bind(sha(f))
        .bind(versions)
        .execute(pool)
        .await
        .expect("blob");
    }

    for v in 1..=versions {
        sqlx::query(
            "INSERT INTO save_versions
               (save_id, version_num, size_bytes, sha256, r2_key, file_count, content_addressed)
             VALUES ($1, $2, $3, 'x', '', $4, TRUE)",
        )
        .bind(&save_id)
        .bind(v)
        .bind(1000 * i64::from(files))
        .bind(i64::from(files))
        .execute(pool)
        .await
        .expect("version");
        for f in 0..files {
            sqlx::query(
                "INSERT INTO save_version_files
                   (save_id, version_num, relative_path, sha256, size_bytes, modified_at)
                 VALUES ($1, $2, $3, $4, 1000, 0)",
            )
            .bind(&save_id)
            .bind(v)
            .bind(format!("file{f}.sav"))
            .bind(sha(f))
            .execute(pool)
            .await
            .expect("file row");
        }
    }
    (user, save_id)
}

async fn cleanup(pool: &PgPool, user: Uuid) {
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

/// `prune_version_caps` is the widest of these: it selects victims, reads
/// `DISTINCT sha256` off their manifests, and drives the refcount decrement
/// that frees a blob. Three separate sha-typed queries in one call.
#[tokio::test]
async fn version_cap_prune_walks_the_manifest_and_frees_blobs() {
    let Some(pool) = pool().await else { return };
    let (user, _save) = seed(&pool, 5, 3).await;
    sqlx::query("UPDATE profiles SET max_versions = 2 WHERE user_id = $1")
        .bind(user)
        .execute(&pool)
        .await
        .expect("cap");

    let state = state_for(pool.clone()).await;
    let pruned = hoard_server::cloud::purge::prune_version_caps(&state, user)
        .await
        .expect("prune runs");

    // Five versions, cap of two, and the head is never a victim.
    assert_eq!(pruned, 3, "versions over the cap");
    let left: i64 =
        sqlx::query_scalar("SELECT count(*) FROM save_versions WHERE save_id IN (SELECT id FROM saves WHERE user_id = $1)")
            .bind(user)
            .fetch_one(&pool)
            .await
            .expect("count");
    assert_eq!(left, 2);
    // The manifests went with them, and the refcounts came down by three.
    let refs: i64 = sqlx::query_scalar("SELECT min(refcount) FROM cloud_blobs WHERE user_id = $1")
        .bind(user)
        .fetch_one(&pool)
        .await
        .expect("refcount");
    assert_eq!(refs, 2, "one decrement per pruned version");

    cleanup(&pool, user).await;
}

/// The dry-run half of the same feature, and the one the confirmation dialog
/// shows a number from.
#[tokio::test]
async fn version_cap_preview_counts_without_touching_anything() {
    let Some(pool) = pool().await else { return };
    let (user, _save) = seed(&pool, 4, 2).await;
    let state = state_for(pool.clone()).await;

    let n = hoard_server::cloud::purge::count_version_cap_excess(&state, user, 2, false)
        .await
        .expect("preview runs");
    assert_eq!(n, 2);

    let left: i64 =
        sqlx::query_scalar("SELECT count(*) FROM save_versions WHERE save_id IN (SELECT id FROM saves WHERE user_id = $1)")
            .bind(user)
            .fetch_one(&pool)
            .await
            .expect("count");
    assert_eq!(left, 4, "a dry run writes nothing");

    cleanup(&pool, user).await;
}

/// `shared_groups` joins the two sha columns directly
/// (`cloud_blobs.sha256 = save_version_files.sha256`). If one side changes
/// storage type and the other does not, this is where Postgres says
/// "operator does not exist".
#[tokio::test]
async fn shared_groups_joins_manifest_against_blobs() {
    let Some(pool) = pool().await else { return };
    let (user, _save) = seed(&pool, 2, 2).await;

    let groups = hoard_server::cloud::archive::shared_groups(&pool, user)
        .await
        .expect("shared_groups runs");
    // A single save shares nothing with anybody, so the join must run and come
    // back empty rather than error.
    assert!(groups.is_empty());

    cleanup(&pool, user).await;
}

/// Archiving walks the manifest to work out which blobs to freeze, and
/// reactivating walks it again to re-reference them. Both group by sha.
#[tokio::test]
async fn archive_and_reactivate_round_trip_the_refcounts() {
    let Some(pool) = pool().await else { return };
    let (user, save_id) = seed(&pool, 3, 2).await;
    let state = state_for(pool.clone()).await;

    hoard_server::cloud::archive::archive_save(&state, user, &save_id)
        .await
        .expect("archive runs");
    let frozen: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM cloud_blobs WHERE user_id = $1 AND purge_after IS NOT NULL",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .expect("frozen");
    assert_eq!(frozen, 2, "every blob of the only save is frozen");

    hoard_server::cloud::archive::reactivate_save(&state, user, &save_id)
        .await
        .expect("reactivate runs");
    let thawed: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM cloud_blobs WHERE user_id = $1 AND purge_after IS NULL",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .expect("thawed");
    assert_eq!(thawed, 2, "and thawed again on the way back");

    cleanup(&pool, user).await;
}

/// The daily sweep reads `save_version_files` to size what it would drop.
#[tokio::test]
async fn the_abandoned_sweep_reads_the_manifest() {
    let Some(pool) = pool().await else { return };
    let (user, _save) = seed(&pool, 2, 2).await;
    let state = state_for(pool.clone()).await;

    // Nothing here is abandoned; what is being tested is that the query runs.
    hoard_server::cloud::abandoned::sweep(&state)
        .await
        .expect("sweep runs");

    cleanup(&pool, user).await;
}

/// `record_cloud` diffs a version's manifest against the one before it, which
/// is two manifest reads keyed by sha.
#[tokio::test]
async fn version_insight_diffs_two_manifests() {
    let Some(pool) = pool().await else { return };
    let (user, save_id) = seed(&pool, 2, 2).await;

    let insight = hoard_server::insight::record_cloud(&pool, &save_id, 2)
        .await
        .expect("insight runs");
    // Both versions hold the same shas, so the diff is real but empty of
    // changes. What matters is that it produced one at all.
    assert!(insight.is_some(), "a manifest of two files yields an insight");

    cleanup(&pool, user).await;
}
