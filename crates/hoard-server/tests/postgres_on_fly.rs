//! What moving the cloud database off Supabase asks of the schema and the
//! server, checked against a real Postgres that is not Supabase: the stub
//! stands in for its objects, signups no longer need an auth.users row, the
//! admin functions are reachable without Supabase's REST layer, the account
//! copy lands, and the compression sweep still picks exactly the blobs it
//! picked before its query was split.
//!
//! Skipped unless `HOARD_PG_TEST_URL` is set, like `blob_sha_paths`.

use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use hoard_server::cloud::auth::CloudUser;
use hoard_server::cloud::routes::admin;
use hoard_server::cloud::state::CloudState;
use hoard_server::cloud::supabase_admin::AdminUser;
use hoard_server::cloud::{auth_mirror, compress};
use hoard_server::config::CompressionConfig;
use sha2::Digest;
use sqlx::PgPool;
use uuid::Uuid;

const STUB: &str = include_str!("../../../deploy/fly/pg/supabase-stub.sql");

async fn pool() -> Option<PgPool> {
    let url = std::env::var("HOARD_PG_TEST_URL").ok()?;
    let pool = hoard_server::cloud::db::connect(&url, 5)
        .await
        .expect("connect to the test database");
    // The lock every Postgres test binary takes around its bootstrap, on one
    // connection so the unlock lands where the lock was taken.
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

async fn state_for(pool: PgPool) -> CloudState {
    let example = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../deploy/config.cloud.toml.example"
    );
    let base = std::fs::read_to_string(example).expect("read the cloud config example");
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("config.toml");
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

async fn profile(pool: &PgPool) -> Uuid {
    let user = Uuid::new_v4();
    sqlx::query("INSERT INTO profiles (user_id, email, plan) VALUES ($1, $2, 'free')")
        .bind(user)
        .bind(format!("{user}@test.invalid"))
        .execute(pool)
        .await
        .expect("a profile with no auth.users row behind it");
    user
}

#[tokio::test]
async fn a_signup_needs_no_auth_users_row() {
    let Some(pool) = pool().await else { return };
    let fk: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'profiles_user_id_fkey')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!fk, "0060 drops the foreign key to auth.users");
    let user = profile(&pool).await;
    sqlx::query("DELETE FROM profiles WHERE user_id = $1")
        .bind(user)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn the_stub_uid_reads_the_claims_like_supabase() {
    let Some(pool) = pool().await else { return };
    // Other test binaries swap auth.uid() for a NULL stub during their own
    // bootstrap, so the body is checked under a session-private name.
    let body = STUB
        .split("CREATE OR REPLACE FUNCTION auth.uid()")
        .nth(1)
        .expect("the stub defines auth.uid()");
    let body = body.trim_end().trim_end_matches(';');
    let mut conn = pool.acquire().await.unwrap();
    sqlx::raw_sql(&format!("CREATE FUNCTION pg_temp.stub_uid(){body}"))
        .execute(&mut *conn)
        .await
        .expect("the stub body compiles");

    let none: Option<Uuid> = sqlx::query_scalar("SELECT pg_temp.stub_uid()")
        .fetch_one(&mut *conn)
        .await
        .unwrap();
    assert_eq!(none, None, "no claims, no caller");

    let user = Uuid::new_v4();
    let mut tx = sqlx::Connection::begin(&mut *conn).await.unwrap();
    sqlx::query("SELECT set_config('request.jwt.claims', $1, true)")
        .bind(serde_json::json!({ "sub": user }).to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    let got: Option<Uuid> = sqlx::query_scalar("SELECT pg_temp.stub_uid()")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(got, Some(user));
}

#[tokio::test]
async fn an_admin_call_runs_as_the_caller_and_leaves_no_trace() {
    let Some(pool) = pool().await else { return };
    sqlx::raw_sql(
        "CREATE OR REPLACE FUNCTION public.hoard_test_claimed_sub() RETURNS jsonb
             LANGUAGE sql STABLE
             AS $$ SELECT jsonb_build_object('sub',
                     nullif(current_setting('request.jwt.claims', true), '')::jsonb ->> 'sub') $$",
    )
    .execute(&pool)
    .await
    .unwrap();

    let user = Uuid::new_v4();
    // More calls than the pool has connections, so a setting that outlived its
    // transaction would show up on the plain query that follows each one.
    for _ in 0..8 {
        let json = admin::call_as(&pool, user, "hoard_test_claimed_sub")
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["sub"], user.to_string());
        let after: Option<String> =
            sqlx::query_scalar("SELECT nullif(current_setting('request.jwt.claims', true), '')")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(after, None, "claims must die with the transaction");
    }
}

#[tokio::test]
async fn the_admin_route_refuses_unknown_names_and_non_admins() {
    let Some(pool) = pool().await else { return };
    let state = state_for(pool.clone()).await;
    let caller = CloudUser {
        user_id: Uuid::new_v4(),
        email: "nobody@test.invalid".into(),
        role: "authenticated".into(),
        avatar_url: None,
        display_name: None,
    };

    let res = admin::rpc(
        State(state.clone()),
        Extension(caller.clone()),
        Path("hoard_test_claimed_sub".into()),
    )
    .await
    .into_response();
    assert_eq!(
        res.status(),
        StatusCode::NOT_FOUND,
        "only the three admin functions"
    );

    // The real functions come from migrations that stay out of git, so a clone
    // without them has nothing to call.
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM pg_proc WHERE proname = 'admin_metrics_screen')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    if exists {
        let res = admin::rpc(
            State(state),
            Extension(caller),
            Path("admin_metrics_screen".into()),
        )
        .await
        .into_response();
        assert_eq!(
            res.status(),
            StatusCode::FORBIDDEN,
            "42501 from the function"
        );
    }
}

fn account(n: u32) -> AdminUser {
    serde_json::from_value(serde_json::json!({
        "id": Uuid::from_u128(0xf1f1_0000_0000_4000_8000_0000_0000_0000 + n as u128),
        "email": format!("acct{n}@test.invalid"),
        "email_confirmed_at": if n % 2 == 0 { Some("2026-08-02T10:11:12.123456Z") } else { None },
        "created_at": "2026-08-01T00:00:00Z",
        "app_metadata": { "provider": if n % 3 == 0 { "github" } else { "google" } },
    }))
    .unwrap()
}

#[tokio::test]
async fn the_account_copy_lands_and_prunes_only_what_is_safe() {
    let Some(pool) = pool().await else { return };
    assert!(
        !auth_mirror::supabase_owns_auth(&pool).await.unwrap(),
        "a plain Postgres is never mistaken for Supabase"
    );

    let mut tx = pool.begin().await.unwrap();
    // Everything below is rolled back, the rows of other tests included.
    sqlx::query("DELETE FROM auth.users")
        .execute(&mut *tx)
        .await
        .unwrap();

    let ten: Vec<AdminUser> = (0..10).map(account).collect();
    let (written, removed) = auth_mirror::sync(&mut tx, &ten).await.unwrap();
    assert_eq!((written, removed), (10, 0));
    let (confirmed, github): (i64, i64) = sqlx::query_as(
        "SELECT count(email_confirmed_at),
                count(*) FILTER (WHERE raw_app_meta_data ->> 'provider' = 'github')
           FROM auth.users",
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert_eq!(
        (confirmed, github),
        (5, 4),
        "timestamps and provider parsed"
    );

    let (_, removed) = auth_mirror::sync(&mut tx, &ten[..9]).await.unwrap();
    assert_eq!(removed, 1, "an account deleted in Supabase leaves the copy");

    let (_, removed) = auth_mirror::sync(&mut tx, &ten[..3]).await.unwrap();
    assert_eq!(
        removed, 0,
        "a list a third the size is a bad answer, not a purge"
    );
    let left: i64 = sqlx::query_scalar("SELECT count(*) FROM auth.users")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(left, 9);

    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn the_split_sweep_picks_what_the_or_picked() {
    let Some(pool) = pool().await else { return };
    let user = profile(&pool).await;

    // created_at in 2000 so these sort ahead of any blob other tests left.
    let blobs = [
        ("raw", "NULL", "NULL", 0, 1, "NULL", "NULL", true),
        ("claimed", "'zstd'", "NULL", 0, 1, "NULL", "NULL", true),
        ("done", "'zstd'", "100", 0, 1, "NULL", "NULL", false),
        ("missing", "'missing'", "NULL", 0, 1, "NULL", "NULL", false),
        ("gave_up", "NULL", "NULL", 5, 1, "NULL", "NULL", false),
        ("orphan", "NULL", "NULL", 0, 0, "NULL", "NULL", false),
        ("archived", "NULL", "NULL", 0, 1, "now()", "NULL", false),
        ("just_served", "NULL", "NULL", 0, 1, "NULL", "now()", false),
        (
            "served_long_ago",
            "NULL",
            "NULL",
            0,
            1,
            "NULL",
            "'2000-01-02'",
            true,
        ),
    ];
    let mut expected = Vec::new();
    for (i, (name, encoding, stored, attempts, refcount, purge_after, presigned, eligible)) in
        blobs.iter().enumerate()
    {
        let sha = hex::encode(sha2::Sha256::digest(name.as_bytes()));
        sqlx::raw_sql(&format!(
            "INSERT INTO cloud_blobs (user_id, sha256, size_bytes, refcount, created_at, encoding,
                                      stored_bytes, compress_attempts, purge_after, last_presigned_at)
             VALUES ('{user}', decode('{sha}', 'hex'), 65536, {refcount},
                     timestamptz '2000-01-01' + interval '{i} minutes', {encoding}, {stored},
                     {attempts}, {purge_after}, {presigned})"
        ))
        .execute(&pool)
        .await
        .unwrap();
        if *eligible {
            expected.push(sha);
        }
    }
    // Too small to be worth compressing, in an otherwise eligible state.
    sqlx::query(
        "INSERT INTO cloud_blobs (user_id, sha256, size_bytes, refcount, created_at)
         VALUES ($1, decode(repeat('ab', 32), 'hex'), 100, 1, timestamptz '2000-01-01')",
    )
    .bind(user)
    .execute(&pool)
    .await
    .unwrap();

    let cfg = CompressionConfig {
        min_age_hours: 1,
        idle_hours: 1,
        batch: 50,
        ..Default::default()
    };
    let picked: Vec<String> = compress::eligible(&pool, &cfg)
        .await
        .unwrap()
        .into_iter()
        .filter(|(u, _, _)| *u == user)
        .map(|(_, sha, _)| sha)
        .collect();
    assert_eq!(picked, expected, "same blobs, oldest first");

    sqlx::query("DELETE FROM profiles WHERE user_id = $1")
        .bind(user)
        .execute(&pool)
        .await
        .unwrap();
}
