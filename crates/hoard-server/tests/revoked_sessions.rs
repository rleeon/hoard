//! Unlinking a device revokes its session, against a real Postgres.
//!
//! Skipped unless `HOARD_PG_TEST_URL` is set, like `device_prune`:
//!
//! ```sh
//! export HOARD_PG_TEST_URL=postgres://postgres:hoard@localhost:55432/hoard
//! cargo test -p hoard-server --features cloud --test revoked_sessions
//! ```
//!
//! **Never point it at production.** It runs migrations on whatever it is given.

#![cfg(feature = "cloud")]

use hoard_server::cloud::auth::SessionId;
use hoard_server::cloud::device_prune::prune_revoked_sessions;
use hoard_server::cloud::routes::me::revoke_device;
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

async fn device(pool: &PgPool, user_id: Uuid, session: Option<Uuid>) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO devices (user_id, device_name, device_kind, fingerprint, session_id)
         VALUES ($1, 'pc', 'desktop', $2, $3)
         RETURNING id",
    )
    .bind(user_id)
    .bind(Uuid::new_v4().to_string())
    .bind(session)
    .fetch_one(pool)
    .await
    .expect("insert device")
}

async fn revoked(pool: &PgPool, session: Uuid) -> bool {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM revoked_sessions WHERE session_id = $1)")
        .bind(session)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn device_exists(pool: &PgPool, id: Uuid) -> bool {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM devices WHERE id = $1)")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn unlinking_another_machine_revokes_its_session() {
    let Some(pool) = pool().await else { return };
    let u = user(&pool).await;
    let (theirs, mine) = (Uuid::new_v4(), Uuid::new_v4());
    let stolen = device(&pool, u, Some(theirs)).await;
    device(&pool, u, Some(mine)).await;

    revoke_device(&pool, u, stolen, Some(SessionId(mine)))
        .await
        .unwrap();

    assert!(!device_exists(&pool, stolen).await);
    assert!(revoked(&pool, theirs).await);
    assert!(!revoked(&pool, mine).await);
}

#[tokio::test]
async fn the_callers_own_session_is_spared() {
    let Some(pool) = pool().await else { return };
    let u = user(&pool).await;
    let mine = Uuid::new_v4();
    // The old row of a machine whose hostname changed: same session, new
    // fingerprint, and the machine frees its old slot.
    let old_row = device(&pool, u, Some(mine)).await;

    revoke_device(&pool, u, old_row, Some(SessionId(mine)))
        .await
        .unwrap();

    assert!(!device_exists(&pool, old_row).await);
    assert!(!revoked(&pool, mine).await);
}

#[tokio::test]
async fn a_session_another_row_still_holds_is_spared() {
    let Some(pool) = pool().await else { return };
    let u = user(&pool).await;
    let shared = Uuid::new_v4();
    let old_row = device(&pool, u, Some(shared)).await;
    let new_row = device(&pool, u, Some(shared)).await;

    // From the web, whose session is neither.
    revoke_device(&pool, u, old_row, Some(SessionId(Uuid::new_v4())))
        .await
        .unwrap();

    assert!(device_exists(&pool, new_row).await);
    assert!(!revoked(&pool, shared).await);
}

#[tokio::test]
async fn a_device_without_a_known_session_is_just_deleted() {
    let Some(pool) = pool().await else { return };
    let u = user(&pool).await;
    let old = device(&pool, u, None).await;

    revoke_device(&pool, u, old, None).await.unwrap();

    assert!(!device_exists(&pool, old).await);
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM revoked_sessions WHERE user_id = $1")
        .bind(u)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 0);
}

#[tokio::test]
async fn another_accounts_device_is_untouched() {
    let Some(pool) = pool().await else { return };
    let (victim, other) = (user(&pool).await, user(&pool).await);
    let session = Uuid::new_v4();
    let theirs = device(&pool, victim, Some(session)).await;

    revoke_device(&pool, other, theirs, None).await.unwrap();

    assert!(device_exists(&pool, theirs).await);
    assert!(!revoked(&pool, session).await);
}

#[tokio::test]
async fn ended_sessions_are_forgotten_after_a_week() {
    let Some(pool) = pool().await else { return };
    let u = user(&pool).await;
    let (old, recent, pending) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    for (sid, ended) in [
        (old, Some("8 days")),
        (recent, Some("1 day")),
        (pending, None),
    ] {
        sqlx::query(
            "INSERT INTO revoked_sessions (session_id, user_id, revoked_at, signed_out_at)
             VALUES ($1, $2, now() - interval '30 days', now() - $3::interval)",
        )
        .bind(sid)
        .bind(u)
        .bind(ended)
        .execute(&pool)
        .await
        .unwrap();
    }

    prune_revoked_sessions(&pool).await.unwrap();

    assert!(!revoked(&pool, old).await);
    assert!(revoked(&pool, recent).await);
    assert!(revoked(&pool, pending).await);
}
