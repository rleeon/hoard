//! A client cursor that outlived the save it points at, against a real Postgres.
//!
//! The report of ago-2026: a Fallout 4 save was archived, then dropped from the
//! cloud, while the desktop kept `last_version_num = 398` in its local state.
//! Every backup after that re-minted the empty row through `resolve_save_row`,
//! was refused as a non-fast-forward against its own head of 0, and the refusal
//! rolled the fresh row back with the transaction. So the account showed no
//! Fallout 4 at all, the row never survived a request, while the client burned
//! its conflict budget and parked the save for good, told each time that
//! "another device advanced this save", of which there was none.
//!
//! The trap is that it has no exit on the client's side: a base only moves when
//! an upload lands or a reconcile pulls a head, and an empty remote offers
//! neither. So the server has to be the one that gives, and what it gives up is
//! nothing: an empty row has no version to bury.
//!
//! What these pin down is the shape of that concession, that it applies to a
//! row with no history, and *only* to one.
//!
//! Skipped unless `HOARD_PG_TEST_URL` is set, like `downgrade_grace`:
//!
//! ```sh
//! docker run -d --name hoard-pg -p 55432:5432 \
//!   -e POSTGRES_PASSWORD=hoard -e POSTGRES_DB=hoard postgres:17
//! export HOARD_PG_TEST_URL=postgres://postgres:hoard@localhost:55432/hoard
//! cargo test -p hoard-server --features cloud --test orphaned_cursor
//! ```
//!
//! **Never point it at production.**

#![cfg(feature = "cloud")]

use hoard_server::cloud::routes::saves::{
    manifest_covers_head, resolve_save_row, save_has_no_history, CasFileEntry,
};
use sqlx::PgPool;
use uuid::Uuid;

/// Connect + migrate, or `None` when the env var isn't set (CI's no-op path).
///
/// Same Supabase stand-ins as `downgrade_grace`, same advisory lock around them:
/// `cargo test` runs the tests in this binary in parallel and they would
/// otherwise race each other on `CREATE OR REPLACE FUNCTION`.
async fn pool() -> Option<PgPool> {
    let url = std::env::var("HOARD_PG_TEST_URL").ok()?;
    let pool = hoard_server::cloud::db::connect(&url, 5)
        .await
        .expect("connect to the test database");
    sqlx::query("SELECT pg_advisory_lock(8_233_119_403)")
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
    sqlx::query("SELECT pg_advisory_unlock(8_233_119_403)")
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

/// Fallout 4, exactly as reported: the row is gone, the client still says 398.
#[tokio::test]
async fn a_deleted_save_does_not_diverge_from_its_own_replacement() {
    let Some(pool) = pool().await else { return };
    let user = seed_user(&pool).await;
    let mut conn = pool.acquire().await.unwrap();

    // Nothing in the cloud carries this id, nor this (game, label): the row
    // below is the empty one minted to answer the push.
    let client_save_id = Uuid::new_v4().to_string();
    let row = resolve_save_row(&mut conn, &client_save_id, user, "fallout-4", &None, false)
        .await
        .unwrap();
    assert_eq!(row.1, 0, "a freshly minted row starts at head 0");

    assert!(
        save_has_no_history(&mut conn, &row.0, row.1).await.unwrap(),
        "a base of 398 against an empty row is a dead cursor, not a divergence"
    );

    drop(conn);
    cleanup(&pool, user).await;
}

/// The concession is scoped to *no history*, not to a head that reads 0. If the
/// bookkeeping column ever lags its versions, the versions win and the push is
/// still refused, that column is the thing this check refuses to trust.
#[tokio::test]
async fn history_under_a_stale_head_still_diverges() {
    let Some(pool) = pool().await else { return };
    let user = seed_user(&pool).await;
    let mut conn = pool.acquire().await.unwrap();

    let row = resolve_save_row(
        &mut conn,
        &Uuid::new_v4().to_string(),
        user,
        "factorio",
        &None,
        false,
    )
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO save_versions (save_id, version_num, size_bytes, sha256, r2_key, file_count)
         VALUES ($1, 1, 10, 'deadbeef', 'k', 1)",
    )
    .bind(&row.0)
    .execute(&mut *conn)
    .await
    .unwrap();

    assert!(
        !save_has_no_history(&mut conn, &row.0, 0).await.unwrap(),
        "a version exists under this head; refusing the push is what protects it"
    );

    drop(conn);
    cleanup(&pool, user).await;
}

/// And a save with a real head is untouched by any of this: the ordinary
/// two-devices divergence still rejects, which is the whole point of the check.
#[tokio::test]
async fn a_real_head_never_takes_the_escape_hatch() {
    let Some(pool) = pool().await else { return };
    let user = seed_user(&pool).await;
    let mut conn = pool.acquire().await.unwrap();

    let row = resolve_save_row(
        &mut conn,
        &Uuid::new_v4().to_string(),
        user,
        "stellaris",
        &None,
        false,
    )
    .await
    .unwrap();
    sqlx::query("UPDATE saves SET latest_version_num = 7 WHERE id = $1")
        .bind(&row.0)
        .execute(&mut *conn)
        .await
        .unwrap();

    assert!(
        !save_has_no_history(&mut conn, &row.0, 7).await.unwrap(),
        "head 7 vs base 4 is the divergence the 409 exists for"
    );

    drop(conn);
    cleanup(&pool, user).await;
}

/// Expand a short readable label into a real 64-hex digest. `sha256` is bytea
/// since migration 0052 and the inserts decode it, so "aaa" no longer goes in
/// as itself; the tests keep their labels and this turns them into something
/// `decode(..., 'hex')` accepts, distinct per label.
fn sha_of(label: &str) -> String {
    let mut h: String = label.bytes().map(|b| format!("{b:02x}")).collect();
    h.truncate(64);
    format!("{h:0<64}")
}

/// A file as the client declares it in a CAS manifest.
fn f(path: &str, sha: &str) -> CasFileEntry {
    CasFileEntry {
        relative_path: path.to_string(),
        sha256: sha_of(sha),
        size_bytes: 1,
        modified_at: None,
    }
}

/// Seeds `save_versions` + the interned manifest for one version.
async fn seed_version(pool: &PgPool, save_id: &str, num: i64, files: &[(&str, &str)]) {
    sqlx::query(
        "INSERT INTO save_versions (save_id, version_num, size_bytes, sha256, r2_key, file_count, content_addressed)
         VALUES ($1, $2, 1, 'x', '', $3, TRUE)",
    )
    .bind(save_id)
    .bind(num)
    .bind(files.len() as i64)
    .execute(pool)
    .await
    .expect("version");
    for (path, sha) in files {
        // The catalogue entry, then the version's reference to it.
        sqlx::query(
            "WITH e AS (
                 INSERT INTO file_entries (save_id, relative_path, sha256, size_bytes)
                 VALUES ($1, $3, decode($4, 'hex'), 1)
                 ON CONFLICT (save_id, relative_path, sha256) DO NOTHING
                 RETURNING id
             )
             INSERT INTO version_files (version_id, entry_id, modified_at)
             SELECT v.id, COALESCE((SELECT id FROM e),
                                   (SELECT id FROM file_entries
                                     WHERE save_id = $1 AND relative_path = $3
                                       AND sha256 = decode($4, 'hex'))), NULL
               FROM save_versions v
              WHERE v.save_id = $1 AND v.version_num = $2
             ON CONFLICT DO NOTHING",
        )
        .bind(save_id)
        .bind(num)
        .bind(path)
        .bind(sha_of(sha))
        .execute(pool)
        .await
        .expect("interned row");
    }
    sqlx::query("UPDATE saves SET latest_version_num = $2 WHERE id = $1")
        .bind(save_id)
        .bind(num)
        .execute(pool)
        .await
        .expect("head");
}

/// Sandfall, as reported: the client sits ahead of a head it already contains,
/// and every push is refused for ten days. Carrying the whole head plus new
/// content of its own means the version about to be written loses none of it.
#[tokio::test]
async fn a_manifest_holding_the_whole_head_may_fast_forward() {
    let Some(pool) = pool().await else { return };
    let user = seed_user(&pool).await;
    let mut conn = pool.acquire().await.unwrap();
    let row = resolve_save_row(
        &mut conn,
        &Uuid::new_v4().to_string(),
        user,
        "sandfall",
        &None,
        false,
    )
    .await
    .unwrap();
    drop(conn);
    seed_version(
        &pool,
        &row.0,
        2,
        &[("save.dat", "aaa"), ("meta.json", "bbb")],
    )
    .await;
    let mut conn = pool.acquire().await.unwrap();

    // The same two files, plus one the head never had.
    let push = vec![
        f("save.dat", "aaa"),
        f("meta.json", "bbb"),
        f("new.dat", "ccc"),
    ];
    assert!(
        manifest_covers_head(&mut conn, &row.0, 2, &push)
            .await
            .unwrap(),
        "a push that carries the head whole buries nothing"
    );

    drop(conn);
    cleanup(&pool, user).await;
}

/// The protection itself: a push that drops a file the head has, or brings an
/// older copy of one, is the burial the 409 exists to stop.
#[tokio::test]
async fn a_manifest_missing_part_of_the_head_is_still_refused() {
    let Some(pool) = pool().await else { return };
    let user = seed_user(&pool).await;
    let mut conn = pool.acquire().await.unwrap();
    let row = resolve_save_row(
        &mut conn,
        &Uuid::new_v4().to_string(),
        user,
        "victoria-3",
        &None,
        false,
    )
    .await
    .unwrap();
    drop(conn);
    seed_version(&pool, &row.0, 4, &[("a.sav", "aaa"), ("b.sav", "bbb")]).await;
    let mut conn = pool.acquire().await.unwrap();

    assert!(
        !manifest_covers_head(&mut conn, &row.0, 4, &[f("a.sav", "aaa")])
            .await
            .unwrap(),
        "dropping b.sav is exactly what the rejection protects"
    );
    // Same paths, one of them changed underneath us: the other device's edit
    // would be lost.
    assert!(
        !manifest_covers_head(
            &mut conn,
            &row.0,
            4,
            &[f("a.sav", "aaa"), f("b.sav", "different")]
        )
        .await
        .unwrap(),
        "an older copy of a file the head advanced is not coverage"
    );

    drop(conn);
    cleanup(&pool, user).await;
}

/// A head whose manifest rows aren't there (pending, or pre-CAS) is not
/// something to reason about, so the ordinary rejection stands.
#[tokio::test]
async fn a_head_without_a_manifest_grants_nothing() {
    let Some(pool) = pool().await else { return };
    let user = seed_user(&pool).await;
    let mut conn = pool.acquire().await.unwrap();
    let row = resolve_save_row(
        &mut conn,
        &Uuid::new_v4().to_string(),
        user,
        "peak",
        &None,
        false,
    )
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO save_versions (save_id, version_num, size_bytes, sha256, r2_key, file_count)
         VALUES ($1, 3, 1, '', '', 0)",
    )
    .bind(&row.0)
    .execute(&mut *conn)
    .await
    .unwrap();

    assert!(
        !manifest_covers_head(&mut conn, &row.0, 3, &[f("a.sav", "aaa")])
            .await
            .unwrap(),
        "no manifest to compare against means no concession"
    );

    drop(conn);
    cleanup(&pool, user).await;
}

/// Carrying the head and nothing more is not a push worth minting a version
/// for: the agent settles on the head instead, and an identical version would
/// only pad the history of a device that lost its place.
#[tokio::test]
async fn a_manifest_equal_to_the_head_is_not_a_fast_forward() {
    let Some(pool) = pool().await else { return };
    let user = seed_user(&pool).await;
    let mut conn = pool.acquire().await.unwrap();
    let row = resolve_save_row(
        &mut conn,
        &Uuid::new_v4().to_string(),
        user,
        "openttd",
        &None,
        false,
    )
    .await
    .unwrap();
    drop(conn);
    seed_version(&pool, &row.0, 5, &[("a.sav", "aaa"), ("b.sav", "bbb")]).await;
    let mut conn = pool.acquire().await.unwrap();

    assert!(
        !manifest_covers_head(
            &mut conn,
            &row.0,
            5,
            &[f("a.sav", "aaa"), f("b.sav", "bbb")]
        )
        .await
        .unwrap(),
        "identical to the head is a settle, not an upload"
    );

    drop(conn);
    cleanup(&pool, user).await;
}
