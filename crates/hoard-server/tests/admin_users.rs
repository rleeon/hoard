//! Account administration from the web panel, end to end (`routes::admin`).
//!
//! Same approach as `devices_presence.rs`: the real handlers against a real
//! database. What it pins down is the half of `hoard-admin` that a NAS operator
//! can now reach without a shell, create, rename, set a password, mint a
//! device token, delete, and, above all, the four refusals that keep an
//! operator from locking themselves out or losing saves by a stray click.

use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::response::Json;
use hoard_server::auth::AuthUser;
use hoard_server::routes::admin;
use hoard_server::routes::health::ServerState;
use hoard_server::routes::session::SESSION_DEVICE_NAME;
use sqlx::Row;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

const ADMIN_ID: &str = "11111111-2222-4333-8444-555555555555";

struct Harness {
    state: Arc<ServerState>,
    admin: AuthUser,
    _dir: tempfile::TempDir,
}

async fn harness() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = dir.path().to_path_buf();
    let cfg_path = data_dir.join("config.toml");
    // A `\` inside a TOML basic string is an escape sequence, so a Windows path
    // written verbatim fails to parse before the first test body runs.
    let toml_path = |p: &std::path::Path| p.display().to_string().replace('\\', "/");
    std::fs::write(
        &cfg_path,
        format!(
            r#"
[server]
host = "127.0.0.1"
port = 12421
public_url = "http://localhost:12421"

[storage]
data_dir = "{data}"
max_snapshot_size_mb = 64
upload_timeout_secs = 600

[database]
url = "sqlite://{db}"
max_connections = 1

[auth]
token_lifetime_days = 365
allow_registration = true

[retention]
trash_retention_days = 30
tmp_cleanup_hours = 24

[logging]
level = "warn"
format = "pretty"
"#,
            data = toml_path(&data_dir),
            db = toml_path(&data_dir.join("hoard.db")),
        ),
    )
    .unwrap();

    let config = hoard_server::config::Config::load(&cfg_path).unwrap();
    let pool = hoard_server::db::connect(&config.database.url, 1)
        .await
        .unwrap();
    hoard_server::db::run_migrations(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO users (id, username, password_hash, is_admin) VALUES (?,'root','x',1)",
    )
    .bind(ADMIN_ID)
    .execute(&pool)
    .await
    .unwrap();
    let store = hoard_server::store::build_store(&config).await.unwrap();

    Harness {
        state: Arc::new(ServerState {
            trusted_proxies: Default::default(),
            pool,
            config,
            start_time: Instant::now(),
            store,
            events: Default::default(),
        }),
        admin: AuthUser {
            user_id: Uuid::parse_str(ADMIN_ID).unwrap(),
            username: "root".into(),
            is_admin: true,
        },
        _dir: dir,
    }
}

async fn create(
    h: &Harness,
    username: &str,
    password: &str,
    is_admin: bool,
) -> Result<String, StatusCode> {
    admin::create_user(
        Extension(h.admin.clone()),
        State(h.state.clone()),
        Json(admin::NewUser {
            username: username.into(),
            password: password.into(),
            is_admin,
            storage_quota_bytes: None,
        }),
    )
    .await
    .map(|(_, Json(u))| u.id)
    .map_err(|(code, _)| code)
}

async fn patch(h: &Harness, id: &str, body: admin::UserPatch) -> Result<(), StatusCode> {
    admin::patch_user(
        Extension(h.admin.clone()),
        State(h.state.clone()),
        Path(id.to_string()),
        Json(body),
    )
    .await
    .map(|_| ())
    .map_err(|(code, _)| code)
}

fn patch_body() -> admin::UserPatch {
    admin::UserPatch {
        is_admin: None,
        storage_quota_bytes: None,
        username: None,
        password: None,
    }
}

async fn mint(h: &Harness, user_id: &str, device: Option<&str>) -> Result<String, StatusCode> {
    admin::create_token(
        Extension(h.admin.clone()),
        State(h.state.clone()),
        Json(admin::NewToken {
            user_id: user_id.into(),
            device_name: device.map(str::to_string),
        }),
    )
    .await
    .map(|(_, Json(m))| m.token)
    .map_err(|(code, _)| code)
}

async fn remove(h: &Harness, id: &str) -> Result<admin::DeletedUser, StatusCode> {
    admin::delete_user(
        Extension(h.admin.clone()),
        State(h.state.clone()),
        Path(id.to_string()),
    )
    .await
    .map(|Json(d)| d)
    .map_err(|(code, _)| code)
}

#[tokio::test]
async fn creates_an_account_that_can_actually_log_in() {
    let h = harness().await;
    let id = create(&h, "player-two", "hunter2hunter2", false)
        .await
        .expect("created");

    // The password has to survive the same verification the login route runs;
    // a hash written by a different function would only show up at first login.
    let hash: String = sqlx::query("SELECT password_hash FROM users WHERE id = ?")
        .bind(&id)
        .fetch_one(&h.state.pool)
        .await
        .unwrap()
        .get("password_hash");
    assert!(hoard_core::hashing::verify_password("hunter2hunter2", &hash).unwrap());
}

#[tokio::test]
async fn refuses_the_names_and_passwords_that_cause_support_tickets() {
    let h = harness().await;
    create(&h, "ok-name", "hunter2hunter2", false)
        .await
        .unwrap();

    // Taken.
    assert_eq!(
        create(&h, "ok-name", "hunter2hunter2", false).await,
        Err(StatusCode::CONFLICT)
    );
    // A space breaks `hoard-admin token create <name>` at the shell.
    assert_eq!(
        create(&h, "two words", "hunter2hunter2", false).await,
        Err(StatusCode::BAD_REQUEST)
    );
    assert_eq!(
        create(&h, "", "hunter2hunter2", false).await,
        Err(StatusCode::BAD_REQUEST)
    );
    assert_eq!(
        create(&h, "shorty", "1234567", false).await,
        Err(StatusCode::BAD_REQUEST)
    );
}

#[tokio::test]
async fn renaming_moves_the_name_and_nothing_else() {
    let h = harness().await;
    let id = create(&h, "old-name", "hunter2hunter2", false)
        .await
        .unwrap();
    let token = mint(&h, &id, Some("desktop")).await.unwrap();

    patch(
        &h,
        &id,
        admin::UserPatch {
            username: Some("new-name".into()),
            ..patch_body()
        },
    )
    .await
    .expect("renamed");

    let (name, tokens): (String, i64) = sqlx::query_as(
        "SELECT u.username, (SELECT COUNT(*) FROM api_tokens t \
                             WHERE t.user_id = u.id AND t.revoked_at IS NULL) \
         FROM users u WHERE u.id = ?",
    )
    .bind(&id)
    .fetch_one(&h.state.pool)
    .await
    .unwrap();
    assert_eq!(name, "new-name");
    // The PC that was syncing before the rename is still syncing after it.
    assert_eq!(tokens, 1, "a rename must not cost the user their devices");
    assert!(!token.is_empty());

    // Onto a name someone else holds.
    let other = create(&h, "taken", "hunter2hunter2", false).await.unwrap();
    assert_eq!(
        patch(
            &h,
            &other,
            admin::UserPatch {
                username: Some("new-name".into()),
                ..patch_body()
            }
        )
        .await,
        Err(StatusCode::CONFLICT)
    );
}

#[tokio::test]
async fn a_new_password_closes_browsers_but_leaves_the_pcs_syncing() {
    let h = harness().await;
    let id = create(&h, "player", "hunter2hunter2", false).await.unwrap();
    mint(&h, &id, Some("desktop")).await.unwrap();
    // A browser session is an `api_tokens` row like any other; only the device
    // name tells it apart.
    sqlx::query("INSERT INTO api_tokens (id, user_id, token_hash, device_name) VALUES (?,?,?,?)")
        .bind(Uuid::new_v4().to_string())
        .bind(&id)
        .bind("sessionhash")
        .bind(SESSION_DEVICE_NAME)
        .execute(&h.state.pool)
        .await
        .unwrap();

    patch(
        &h,
        &id,
        admin::UserPatch {
            password: Some("newpassword".into()),
            ..patch_body()
        },
    )
    .await
    .expect("password set");

    let live: Vec<(Option<String>,)> = sqlx::query_as(
        "SELECT device_name FROM api_tokens WHERE user_id = ? AND revoked_at IS NULL",
    )
    .bind(&id)
    .fetch_all(&h.state.pool)
    .await
    .unwrap();
    assert_eq!(live.len(), 1, "only the browser session should be gone");
    assert_eq!(live[0].0.as_deref(), Some("desktop"));

    assert_eq!(
        patch(
            &h,
            &id,
            admin::UserPatch {
                password: Some("short".into()),
                ..patch_body()
            }
        )
        .await,
        Err(StatusCode::BAD_REQUEST)
    );
}

#[tokio::test]
async fn a_minted_token_authenticates_and_cannot_impersonate_a_session() {
    let h = harness().await;
    let id = create(&h, "player", "hunter2hunter2", false).await.unwrap();

    let token = mint(&h, &id, Some("living-room PC")).await.unwrap();
    let stored: i64 = sqlx::query("SELECT COUNT(*) AS n FROM api_tokens WHERE token_hash = ?")
        .bind(hoard_core::hashing::hash_token(&token))
        .fetch_one(&h.state.pool)
        .await
        .unwrap()
        .get("n");
    assert_eq!(
        stored, 1,
        "the hash of the token we handed out must be the stored one"
    );

    // Only the hash is kept, so the plaintext must never appear in the table.
    let leaked: i64 = sqlx::query("SELECT COUNT(*) AS n FROM api_tokens WHERE token_hash = ?")
        .bind(&token)
        .fetch_one(&h.state.pool)
        .await
        .unwrap()
        .get("n");
    assert_eq!(leaked, 0);

    // A device called "web panel" would be indistinguishable from a session.
    assert_eq!(
        mint(&h, &id, Some(SESSION_DEVICE_NAME)).await,
        Err(StatusCode::BAD_REQUEST)
    );
    assert_eq!(
        mint(&h, "no-such-id", Some("desktop")).await,
        Err(StatusCode::NOT_FOUND)
    );
}

#[tokio::test]
async fn deleting_takes_the_stored_bytes_with_it() {
    let h = harness().await;
    let id = create(&h, "player", "hunter2hunter2", false).await.unwrap();

    // Two objects on disk with the index rows that point at them, which is what
    // a user with one save looks like from here.
    let sha = "a".repeat(64);
    let chunk_sha = "b".repeat(64);
    for (table, digest, size) in [("blobs", &sha, 400i64), ("chunks", &chunk_sha, 600)] {
        sqlx::query(&format!(
            "INSERT INTO {table} (user_id, sha256, size_bytes, refcount) VALUES (?,?,?,1)"
        ))
        .bind(&id)
        .bind(digest)
        .bind(size)
        .execute(&h.state.pool)
        .await
        .unwrap();
    }
    let root = h.state.config.storage.data_dir.clone();
    let blob = root.join(hoard_server::store::blob_key(&id, &sha));
    let chunk = root.join(hoard_server::store::chunk_key(&id, &chunk_sha));
    for path in [&blob, &chunk] {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b"x").unwrap();
    }

    let gone = remove(&h, &id).await.expect("deleted");
    assert_eq!(gone.username, "player");
    assert_eq!(gone.objects_removed, 2);
    assert_eq!(gone.bytes_removed, 1000);
    // The regression this guards: the old code removed `data_dir/<user_id>`,
    // a path nothing has written to since the content-addressed store landed,
    // and reported success with every byte still on disk.
    assert!(!blob.exists(), "the blob is still on disk");
    assert!(!chunk.exists(), "the chunk is still on disk");
    assert!(!root.join("blobs").join(&id).exists());

    let left: i64 = sqlx::query("SELECT COUNT(*) AS n FROM users WHERE id = ?")
        .bind(&id)
        .fetch_one(&h.state.pool)
        .await
        .unwrap()
        .get("n");
    assert_eq!(left, 0);
}

#[tokio::test]
async fn a_server_cannot_be_left_without_an_admin() {
    let h = harness().await;

    // Deleting yourself is refused, and that one check is the whole invariant:
    // the caller is an admin and the target is someone else, so an admin is
    // always left behind. Demoting needs an explicit count; this does not.
    assert_eq!(
        remove(&h, ADMIN_ID).await.map(|_| ()),
        Err(StatusCode::CONFLICT)
    );

    // Another admin, deleted by us, is allowed, we are still here.
    let second = create(&h, "second-admin", "hunter2hunter2", true)
        .await
        .unwrap();
    remove(&h, &second)
        .await
        .expect("an admin may delete another");

    let admins: i64 = sqlx::query("SELECT COUNT(*) AS n FROM users WHERE is_admin <> 0")
        .fetch_one(&h.state.pool)
        .await
        .unwrap()
        .get("n");
    assert_eq!(admins, 1);

    assert_eq!(
        remove(&h, "no-such-id").await.map(|_| ()),
        Err(StatusCode::NOT_FOUND)
    );
}

#[tokio::test]
async fn every_route_here_is_closed_to_a_non_admin() {
    let h = harness().await;
    let plain = AuthUser {
        user_id: Uuid::new_v4(),
        username: "player".into(),
        is_admin: false,
    };

    let created = admin::create_user(
        Extension(plain.clone()),
        State(h.state.clone()),
        Json(admin::NewUser {
            username: "sneaky".into(),
            password: "hunter2hunter2".into(),
            is_admin: true,
            storage_quota_bytes: None,
        }),
    )
    .await
    .map(|_| ())
    .map_err(|(code, _)| code);
    assert_eq!(created, Err(StatusCode::FORBIDDEN));

    let deleted = admin::delete_user(
        Extension(plain.clone()),
        State(h.state.clone()),
        Path(ADMIN_ID.to_string()),
    )
    .await
    .map(|_| ())
    .map_err(|(code, _)| code);
    assert_eq!(deleted, Err(StatusCode::FORBIDDEN));

    let minted = admin::create_token(
        Extension(plain),
        State(h.state.clone()),
        Json(admin::NewToken {
            user_id: ADMIN_ID.into(),
            device_name: None,
        }),
    )
    .await
    .map(|_| ())
    .map_err(|(code, _)| code);
    assert_eq!(minted, Err(StatusCode::FORBIDDEN));
}
