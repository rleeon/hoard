//! Hoard-help end to end: create, upload, complete, throttle, sweep.
//!
//! Needs `HOARD_PG_TEST_URL` (see `blob_sha_paths`). The upload half also needs
//! an S3 endpoint, because the bytes really go through `put_from_reader`:
//!
//! ```sh
//! docker run -d --name hoard-minio -p 9000:9000 minio/minio server /data
//! docker exec hoard-minio mc alias set l http://localhost:9000 minioadmin minioadmin
//! docker exec hoard-minio mc mb l/hoard-test
//! export HOARD_S3_TEST_ENDPOINT=http://localhost:9000
//! export HOARD_S3_TEST_KEY_ID=minioadmin HOARD_S3_TEST_SECRET=minioadmin
//! cargo test -p hoard-server --features cloud --test feedback
//! ```
//!
//! **Never point it at production.** It runs migrations on whatever it is given.

#![cfg(feature = "cloud")]

use axum::body::Body;
use axum::extract::{ConnectInfo, Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::Response;
use axum::Json;
use hoard_core::wire::{
    FeedbackCreate, FeedbackCreated, FeedbackFileDecl, FeedbackKind, FEEDBACK_TOKEN_HEADER,
};
use hoard_server::cloud::{feedback, routes::feedback as routes, state::CloudState};
use sqlx::PgPool;
use std::net::SocketAddr;
use uuid::Uuid;

async fn pool() -> Option<PgPool> {
    let url = std::env::var("HOARD_PG_TEST_URL").ok()?;
    let pool = hoard_server::cloud::db::connect(&url, 5)
        .await
        .expect("connect to the test database");
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

/// R2 at the test S3 endpoint when there is one, at the discard port when not.
async fn state_for(pool: PgPool, s3: bool) -> CloudState {
    let example = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../deploy/config.cloud.toml.example"
    );
    let base = std::fs::read_to_string(example).expect("read the cloud config example");
    let (endpoint, key, secret, bucket) = if s3 {
        (
            std::env::var("HOARD_S3_TEST_ENDPOINT").unwrap(),
            std::env::var("HOARD_S3_TEST_KEY_ID").unwrap_or_default(),
            std::env::var("HOARD_S3_TEST_SECRET").unwrap_or_default(),
            std::env::var("HOARD_S3_TEST_BUCKET").unwrap_or_else(|_| "hoard-test".into()),
        )
    } else {
        (
            "http://127.0.0.1:9".into(),
            "test".into(),
            "test".into(),
            "hoard-test".into(),
        )
    };
    let dir = std::env::temp_dir().join(format!("hoard-feedback-test-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("config.toml");
    std::fs::write(
        &path,
        base.replace(r#"endpoint = """#, &format!(r#"endpoint = "{endpoint}""#))
            .replace(
                r#"bucket = "hoard-snapshots-prod""#,
                &format!(r#"bucket = "{bucket}""#),
            )
            .replace(r#"region = "auto""#, r#"region = "us-east-1""#)
            .replace(
                r#"supabase_jwks_url = """#,
                r#"supabase_jwks_url = "http://127.0.0.1:9/jwks""#,
            )
            .replace(
                r#"access_key_id = """#,
                &format!(r#"access_key_id = "{key}""#),
            )
            .replace(
                r#"secret_access_key = """#,
                &format!(r#"secret_access_key = "{secret}""#),
            ),
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

/// Every test gets its own made-up address, so their throttles never meet.
fn from(ip: &str) -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert("x-forwarded-for", HeaderValue::from_str(ip).unwrap());
    h
}

fn random_ip() -> String {
    let b = Uuid::new_v4().into_bytes();
    format!("10.{}.{}.{}", b[0], b[1], b[2])
}

fn report(files: Vec<(&str, u64)>) -> FeedbackCreate {
    FeedbackCreate {
        kind: FeedbackKind::Bug,
        message: "the restore button does nothing".into(),
        contact: Some("  ".into()),
        app_version: Some("1.2.0".into()),
        os: Some("linux".into()),
        arch: Some("x86_64".into()),
        mode: Some("selfhosted".into()),
        files: files
            .into_iter()
            .map(|(n, s)| FeedbackFileDecl {
                name: n.into(),
                size: s,
            })
            .collect(),
    }
}

async fn create(state: &CloudState, ip: &str, body: FeedbackCreate) -> Response {
    let peer: Option<ConnectInfo<SocketAddr>> = None;
    routes::create(State(state.clone()), peer, from(ip), Json(body))
        .await
        .expect("create runs")
}

async fn created(resp: Response) -> FeedbackCreated {
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).expect("created json")
}

fn token(t: &str) -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert(FEEDBACK_TOKEN_HEADER, HeaderValue::from_str(t).unwrap());
    h
}

#[tokio::test]
async fn a_report_with_files_lands_and_is_swept_later() {
    let Some(pool) = pool().await else { return };
    if std::env::var("HOARD_S3_TEST_ENDPOINT").is_err() {
        eprintln!("skipped: no HOARD_S3_TEST_ENDPOINT");
        return;
    }
    let state = state_for(pool.clone(), true).await;

    // Over one 8 MiB part, so the multipart path runs too.
    let big: Vec<u8> = (0..9 * 1024 * 1024).map(|i| (i % 251) as u8).collect();
    let small = b"2026-10-02T10:00:00Z WARN restore failed".to_vec();
    let c = created(
        create(
            &state,
            &random_ip(),
            report(vec![
                ("C:\\Users\\angel\\Videos\\bug clip.mp4", big.len() as u64),
                ("hoard-logs.txt", small.len() as u64),
            ]),
        )
        .await,
    )
    .await;
    let id: Uuid = c.id.parse().unwrap();

    // Completing before the files are in is refused.
    let early = routes::complete(State(state.clone()), Path(id), token(&c.upload_token)).await;
    assert!(early.is_err());

    // A wrong token cannot upload.
    let bad = routes::upload(
        State(state.clone()),
        Path((id, 1)),
        token("nope"),
        Body::from(small.clone()),
    )
    .await;
    assert!(bad.is_err());

    // A body that does not match the declared size is refused.
    let short = routes::upload(
        State(state.clone()),
        Path((id, 1)),
        token(&c.upload_token),
        Body::from(small[..5].to_vec()),
    )
    .await;
    assert!(short.is_err());

    for (idx, bytes) in [(0i16, big.clone()), (1, small.clone())] {
        let r = routes::upload(
            State(state.clone()),
            Path((id, idx)),
            token(&c.upload_token),
            Body::from(bytes),
        )
        .await
        .expect("upload");
        assert_eq!(r.status(), StatusCode::NO_CONTENT);
    }
    let _ = routes::complete(State(state.clone()), Path(id), token(&c.upload_token))
        .await
        .expect("complete");

    // The token dies with completion.
    let late = routes::upload(
        State(state.clone()),
        Path((id, 1)),
        token(&c.upload_token),
        Body::from(small.clone()),
    )
    .await;
    assert!(late.is_err());

    let (contact, mode, has_ip): (Option<String>, String, bool) = sqlx::query_as(
        "SELECT contact, mode, ip_hash IS NOT NULL FROM feedback_reports WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(contact, None, "a blank contact is not stored");
    assert_eq!(mode, "selfhosted");
    assert!(has_ip);

    let key = feedback::object_key(id, 0, "C:\\Users\\angel\\Videos\\bug clip.mp4");
    assert!(key.ends_with("/0-bug_clip.mp4"));
    assert_eq!(state.r2.head(&key).await.unwrap(), Some(big.len() as i64));
    assert_eq!(state.r2.get_object(&key).await.unwrap(), big);

    // Ninety-one days later the sweep takes the rows and the objects.
    sqlx::query(
        "UPDATE feedback_reports SET created_at = now() - interval '91 days' WHERE id = $1",
    )
    .bind(id)
    .execute(&pool)
    .await
    .unwrap();
    assert!(feedback::sweep(&state).await.unwrap() >= 1);
    assert_eq!(state.r2.head(&key).await.unwrap(), None);
    let left: i64 = sqlx::query_scalar("SELECT count(*) FROM feedback_files WHERE report_id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(left, 0);
}

#[tokio::test]
async fn one_address_is_throttled_after_five_an_hour() {
    let Some(pool) = pool().await else { return };
    let state = state_for(pool, false).await;
    let ip = random_ip();
    for _ in 0..5 {
        created(create(&state, &ip, report(vec![])).await).await;
    }
    let sixth = create(&state, &ip, report(vec![])).await;
    assert_eq!(sixth.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(sixth.headers()["retry-after"], "3600");
    // Somebody else is not affected.
    created(create(&state, &random_ip(), report(vec![])).await).await;
}

#[tokio::test]
async fn limits_are_checked_before_anything_is_written() {
    let Some(pool) = pool().await else { return };
    let state = state_for(pool, false).await;
    let ip = random_ip();
    let peer: Option<ConnectInfo<SocketAddr>> = None;

    let mut empty = report(vec![]);
    empty.message = "   ".into();
    let too_big = report(vec![("a.mp4", 91 * 1024 * 1024)]);
    let too_many = report((0..11).map(|_| ("a.txt", 1)).collect());
    for body in [empty, too_big, too_many] {
        let r = routes::create(State(state.clone()), peer, from(&ip), Json(body)).await;
        assert!(r.is_err());
    }
    // None of those counted against the address.
    created(create(&state, &ip, report(vec![])).await).await;
}

#[tokio::test]
async fn a_report_without_files_completes_and_an_abandoned_one_is_dropped() {
    let Some(pool) = pool().await else { return };
    let state = state_for(pool.clone(), false).await;

    let done = created(create(&state, &random_ip(), report(vec![])).await).await;
    let done_id: Uuid = done.id.parse().unwrap();
    let _ = routes::complete(
        State(state.clone()),
        Path(done_id),
        token(&done.upload_token),
    )
    .await
    .expect("complete");

    let dead = created(create(&state, &random_ip(), report(vec![])).await).await;
    let dead_id: Uuid = dead.id.parse().unwrap();
    sqlx::query(
        "UPDATE feedback_reports SET created_at = now() - interval '25 hours' WHERE id = $1",
    )
    .bind(dead_id)
    .execute(&pool)
    .await
    .unwrap();

    feedback::sweep(&state).await.unwrap();
    let alive: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM feedback_reports WHERE id = ANY($1)")
        .bind(vec![done_id, dead_id])
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(alive, vec![done_id]);
}
