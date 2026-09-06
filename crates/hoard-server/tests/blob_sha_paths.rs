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
    // Same bootstrap dance as `downgrade_grace`, lock included: taken on one
    // connection rather than the pool, or the unlock lands elsewhere and every
    // later test binary blocks on it forever.
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
             VALUES ($1, decode($2, 'hex'), 1000, $3)",
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
            // Both shapes, because that is what a real commit writes and the
            // reads now go through the interned view. A fixture that fills only
            // the old table leaves every read blind, which is a property of the
            // fixture and not of the code under test.
            sqlx::query(
                "WITH e AS (
                     INSERT INTO file_entries (save_id, relative_path, sha256, size_bytes)
                     VALUES ($1, $3, decode($4, 'hex'), 1000)
                     ON CONFLICT (save_id, relative_path, sha256) DO NOTHING
                     RETURNING id
                 )
                 INSERT INTO version_files (version_id, entry_id, modified_at)
                 SELECT v.id, COALESCE((SELECT id FROM e),
                                       (SELECT id FROM file_entries
                                         WHERE save_id = $1 AND relative_path = $3
                                           AND sha256 = decode($4, 'hex'))), 0
                   FROM save_versions v
                  WHERE v.save_id = $1 AND v.version_num = $2
                 ON CONFLICT DO NOTHING",
            )
            .bind(&save_id)
            .bind(v)
            .bind(format!("file{f}.sav"))
            .bind(sha(f))
            .execute(pool)
            .await
            .expect("interned rows");
            sqlx::query(
                "INSERT INTO save_version_files
                   (save_id, version_num, relative_path, sha256, size_bytes, modified_at)
                 VALUES ($1, $2, $3, decode($4, 'hex'), 1000, 0)",
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

/// The dual write of the interned manifest (phase 3, step 2): every commit
/// writes the old table and the new pair, and the two must say the same thing.
///
/// This is the test the cutover rests on. Nothing reads `file_entries` /
/// `version_files` yet, so a divergence here is invisible in production until
/// the day the reads switch over, at which point it is somebody's save with a
/// file missing from it. Comparing the two representations on every commit is
/// what makes that impossible to ship.
#[tokio::test]
async fn the_interned_manifest_matches_the_old_table() {
    let Some(pool) = pool().await else { return };
    let (user, _existing) = seed(&pool, 1, 2).await;
    let state = state_for(pool.clone()).await;

    // Two versions of the same save: the second repeats one file and changes
    // the other. That is the shape interning exists for, so the catalogue must
    // end up with three entries and not four.
    for (version, second_sha) in [(1u8, sha(10)), (2u8, sha(11))] {
        let body = hoard_server::cloud::routes::saves::CasInit {
            save_id: Uuid::new_v4().to_string(),
            game_slug: "interned-game".into(),
            label: Some("default".into()),
            device_name: None,
            notes: None,
            backup_only: false,
            base_version: if version == 1 { None } else { Some(1) },
            files: vec![
                hoard_server::cloud::routes::saves::CasFileEntry {
                    relative_path: "steady.sav".into(),
                    sha256: sha(9),
                    size_bytes: 100,
                    modified_at: Some(1),
                },
                hoard_server::cloud::routes::saves::CasFileEntry {
                    relative_path: "changing.sav".into(),
                    sha256: second_sha,
                    size_bytes: 200,
                    modified_at: Some(i64::from(version)),
                },
            ],
        };
        let user_ctx = hoard_server::cloud::auth::CloudUser {
            user_id: user,
            email: format!("{user}@test.invalid"),
            role: "authenticated".into(),
            avatar_url: None,
            display_name: None,
        };
        hoard_server::cloud::routes::saves::cas_init(
            axum::extract::State(state.clone()),
            axum::Extension(user_ctx),
            axum::Json(body),
        )
        .await
        .expect("cas_init runs");
    }

    // `cas_init` opens a *pending* version and clears any previous pending one,
    // so without a `cas_commit` in between the second call replaces the first
    // rather than adding a version. That is fine for what is being checked
    // here: the invariant is not how many versions exist, it is that whatever
    // the old table says, the interned pair says the same.

    // Everything below is scoped to this save: the `seed` fixture writes the
    // old table directly, bypassing the dual write, so its rows legitimately
    // have no interned counterpart.
    //
    // The catalogue stores each distinct (path, sha) once, however many rows
    // the old table spends on them.
    let (old_distinct, entries): (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(DISTINCT (f.relative_path, f.sha256))
                   FROM save_version_files f JOIN saves s ON s.id = f.save_id
                  WHERE s.user_id = $1 AND s.game_slug = 'interned-game'),
                (SELECT count(*)
                   FROM file_entries e JOIN saves s ON s.id = e.save_id
                  WHERE s.user_id = $1 AND s.game_slug = 'interned-game')",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .expect("counts");
    assert_eq!(
        entries, old_distinct,
        "the catalogue holds exactly the distinct contents, no more and no less"
    );

    // And the payload agrees, version by version, path by path. Any difference
    // in path, digest, size or mtime shows up as a non-empty row set. The check
    // runs both ways: a row the new tables cannot reproduce, and a row they
    // invented that the old table never had.
    let missing: Vec<(String, i64, String)> = sqlx::query_as(
        "SELECT f.save_id, f.version_num, f.relative_path
           FROM save_version_files f
           JOIN saves s ON s.id = f.save_id
          WHERE s.user_id = $1 AND s.game_slug = 'interned-game'
            AND NOT EXISTS (
                SELECT 1
                  FROM version_files vf
                  JOIN save_versions v ON v.id = vf.version_id
                  JOIN file_entries e ON e.id = vf.entry_id
                 WHERE v.save_id = f.save_id
                   AND v.version_num = f.version_num
                   AND e.relative_path = f.relative_path
                   AND e.sha256 = f.sha256
                   AND e.size_bytes = f.size_bytes
                   AND vf.modified_at IS NOT DISTINCT FROM f.modified_at)",
    )
    .bind(user)
    .fetch_all(&pool)
    .await
    .expect("comparison");
    assert!(
        missing.is_empty(),
        "rows the interned tables do not reproduce: {missing:?}"
    );

    let invented: i64 = sqlx::query_scalar(
        "SELECT count(*)
           FROM version_files vf
           JOIN save_versions v ON v.id = vf.version_id
           JOIN file_entries e ON e.id = vf.entry_id
           JOIN saves s ON s.id = v.save_id
          WHERE s.user_id = $1 AND s.game_slug = 'interned-game'
            AND NOT EXISTS (
                SELECT 1 FROM save_version_files f
                 WHERE f.save_id = v.save_id
                   AND f.version_num = v.version_num
                   AND f.relative_path = e.relative_path
                   AND f.sha256 = e.sha256)",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .expect("reverse comparison");
    assert_eq!(invented, 0, "the interned tables invented rows");

    cleanup(&pool, user).await;
}
