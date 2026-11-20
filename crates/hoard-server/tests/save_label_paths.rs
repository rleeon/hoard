//! A save's label never reaches past its own folder on disk.
//!
//! The label is free text and used to be joined straight into
//! `data/<user>/<game>/<label>`: a save labelled `../../..` had its delete land
//! on `remove_dir_all(data_dir)`, every user's blobs and the database with it,
//! and an absolute label replaced the whole path. Any account on a self-hosted
//! server could do it.

use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::response::Json;
use hoard_core::ids::GameSlug;
use hoard_core::wire::{CreateSaveRequest, PatchSaveRequest};
use hoard_server::auth::AuthUser;
use hoard_server::routes::health::ServerState;
use hoard_server::routes::saves;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

const USER: &str = "11111111-2222-4333-8444-555555555555";

struct Harness {
    state: Arc<ServerState>,
    user: AuthUser,
    dir: tempfile::TempDir,
}

impl Harness {
    fn data_dir(&self) -> &std::path::Path {
        self.dir.path()
    }
}

async fn harness() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = dir.path().to_path_buf();
    let db_path = data_dir.join("hoard.db");
    let cfg_path = data_dir.join("config.toml");
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
            db = toml_path(&db_path),
        ),
    )
    .unwrap();

    let config = hoard_server::config::Config::load(&cfg_path).unwrap();
    let pool = hoard_server::db::connect(&config.database.url, 1)
        .await
        .unwrap();
    hoard_server::db::run_migrations(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO users (id, username, password_hash, storage_quota_bytes, storage_used_bytes)
         VALUES (?,'jacka','x', 1073741824, 0)",
    )
    .bind(USER)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT OR IGNORE INTO games (slug, display_name) VALUES ('factorio','Factorio')")
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
        user: AuthUser {
            user_id: Uuid::parse_str(USER).unwrap(),
            username: "jacka".into(),
            is_admin: false,
        },
        dir,
    }
}

async fn create(h: &Harness, label: &str) -> String {
    let body = CreateSaveRequest {
        game_slug: GameSlug::parse("factorio").unwrap(),
        label: Some(label.into()),
        local_path_hint: None,
        client_os: None,
        display_name: None,
        steam_app_id: None,
    };
    let (status, Json(save)) = saves::create(
        State(h.state.clone()),
        Extension(h.user.clone()),
        Json(body),
    )
    .await
    .unwrap();
    assert_eq!(status, StatusCode::CREATED);
    save.id.to_string()
}

async fn delete(h: &Harness, id: &str) {
    let status = saves::delete(
        State(h.state.clone()),
        Extension(h.user.clone()),
        Path(id.to_string()),
    )
    .await
    .unwrap();
    assert_eq!(status, StatusCode::NO_CONTENT);
}

fn legacy_dir(h: &Harness, label: &str) -> std::path::PathBuf {
    h.data_dir()
        .join("data")
        .join(USER)
        .join("factorio")
        .join(label)
}

#[tokio::test]
async fn a_label_that_climbs_out_cannot_delete_the_data_folder() {
    let h = harness().await;
    let other_users_blob = h.data_dir().join("blobs").join("someone-else").join("ab");
    std::fs::create_dir_all(&other_users_blob).unwrap();
    std::fs::write(other_users_blob.join("abcd"), b"theirs").unwrap();

    let id = create(&h, "../../..").await;
    delete(&h, &id).await;

    assert!(other_users_blob.join("abcd").exists());
    assert!(h.data_dir().join("hoard.db").exists());
}

#[tokio::test]
async fn an_absolute_label_cannot_reach_outside() {
    let h = harness().await;
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("keep"), b"not hoard's").unwrap();
    let made_by_create = outside.path().join("made-by-create");

    let id = create(&h, &made_by_create.display().to_string()).await;
    assert!(!made_by_create.exists());
    let id_root = create(&h, &outside.path().display().to_string()).await;

    delete(&h, &id).await;
    delete(&h, &id_root).await;

    assert!(outside.path().join("keep").exists());
}

#[tokio::test]
async fn a_rename_to_a_climbing_label_moves_nothing() {
    let h = harness().await;
    let id = create(&h, "default").await;
    let old = legacy_dir(&h, "default");
    std::fs::create_dir_all(old.join("v1")).unwrap();
    std::fs::write(old.join("v1").join("save.dat"), b"v1").unwrap();

    let body = PatchSaveRequest {
        label: Some("../../../moved".into()),
        ..Default::default()
    };
    let Json(save) = saves::patch(
        State(h.state.clone()),
        Extension(h.user.clone()),
        Path(id),
        Json(body),
    )
    .await
    .unwrap();

    assert_eq!(save.label, "../../../moved");
    assert!(old.join("v1").join("save.dat").exists());
    assert!(!h.data_dir().join("moved").exists());
}

#[tokio::test]
async fn a_slot_name_with_a_slash_still_works() {
    let h = harness().await;
    let label = "2 - Mods/Vanilla";
    let id = create(&h, label).await;
    let dir = legacy_dir(&h, label);
    std::fs::create_dir_all(dir.join("v1")).unwrap();

    delete(&h, &id).await;

    assert!(!dir.exists());
    assert!(h.data_dir().join("hoard.db").exists());
}
