//! The content-addressed upload, end to end (`routes::cas`).
//!
//! It calls the real handlers, the same ones `main.rs` mounts, against a real
//! database and a real store. Axum's extractors are wrappers, so invoking them
//! directly exercises everything that matters (staging, hash verification, placement
//! in the store, the transaction, the accounting) without standing a server up.
//!
//! What is tested is the whole promise: **the second backup of a nearly identical
//! save does not retransmit what the server already has**. That is what separates
//! this from the multipart, and the one thing a unit test cannot show.

use axum::body::Body;
use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::response::Json;
use hoard_core::ids::Sha256 as Sha256Hex;
use hoard_core::wire::{CasCommit, CasFile, CasInit};
use hoard_server::auth::AuthUser;
use hoard_server::routes::cas;
use hoard_server::routes::health::ServerState;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

const USER: &str = "11111111-2222-4333-8444-555555555555";
const SAVE: &str = "66666666-7777-4888-8999-aaaaaaaaaaaa";

struct Harness {
    state: Arc<ServerState>,
    user: AuthUser,
    _dir: tempfile::TempDir,
}

async fn harness() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = dir.path().to_path_buf();
    let db_path = data_dir.join("hoard.db");
    let cfg_path = data_dir.join("config.toml");

    // A minimal but real config: it is loaded with the binary's own `Config::load`,
    // so a field that becomes mandatory breaks here and not in production.
    // `display()` writes the host's separators, and on Windows a `\` inside a
    // TOML basic string is an escape sequence, `C:\Users\...` fails to parse
    // before a single test body runs. Forward slashes are accepted by both
    // Windows APIs and SQLite's URL parser, so normalising here keeps one
    // fixture correct on every platform.
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

    let config = hoard_server::config::Config::load(&cfg_path).expect("la config de prueba carga");
    let pool = hoard_server::db::connect(&config.database.url, 1)
        .await
        .expect("base de prueba");
    hoard_server::db::run_migrations(&pool)
        .await
        .expect("migraciones");
    seed(&pool).await;
    let store = hoard_server::store::build_store(&config)
        .await
        .expect("local store");

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
        _dir: dir,
    }
}

async fn seed(pool: &SqlitePool) {
    sqlx::query(
        "INSERT INTO users (id, username, password_hash, storage_quota_bytes, storage_used_bytes)
         VALUES (?,'jacka','x', 1073741824, 0)",
    )
    .bind(USER)
    .execute(pool)
    .await
    .unwrap();
    // The migrations already seed a catalogue; the game may or may not be in it.
    sqlx::query("INSERT OR IGNORE INTO games (slug, display_name) VALUES ('factorio','Factorio')")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO saves (id, user_id, game_slug, label, latest_version_num)
         VALUES (?,?,'factorio','default',0)",
    )
    .bind(SAVE)
    .bind(USER)
    .execute(pool)
    .await
    .unwrap();
}

fn sha_of(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn manifest(files: &[(&str, &[u8])]) -> Vec<CasFile> {
    manifest_at(files, &[])
}

/// Like [`manifest`], with a per-file mtime: `mtimes[i]` is `files[i]`'s. With no
/// mtime every file ties and the history's headline file is picked by size, which is
/// what happens with an older client.
fn manifest_at(files: &[(&str, &[u8])], mtimes: &[i64]) -> Vec<CasFile> {
    files
        .iter()
        .enumerate()
        .map(|(i, (path, bytes))| CasFile {
            relative_path: (*path).into(),
            sha256: Sha256Hex::parse(&sha_of(bytes)).unwrap(),
            size_bytes: bytes.len() as i64,
            modified_at: mtimes.get(i).copied(),
        })
        .collect()
}

/// A full backup: init, upload what is missing, commit. Returns the version, the
/// blobs that had to be transmitted, and the bytes transmitted.
async fn backup(h: &Harness, files: &[(&str, &[u8])], base: Option<i64>) -> (i64, usize, i64) {
    backup_at(h, files, &[], base).await.0
}

/// A backup with declared mtimes, which also returns the whole `Snapshot` so what
/// the history's row will say can be inspected.
async fn backup_at(
    h: &Harness,
    files: &[(&str, &[u8])],
    mtimes: &[i64],
    base: Option<i64>,
) -> ((i64, usize, i64), hoard_core::wire::Snapshot) {
    let m = manifest_at(files, mtimes);
    let init = cas::init(
        State(h.state.clone()),
        Extension(h.user.clone()),
        Path(SAVE.to_string()),
        Json(CasInit {
            base_version: base,
            files: m.clone(),
        }),
    )
    .await
    .expect("init")
    .0;

    let asked = init.missing.len();
    let asked_bytes = init.missing_bytes;
    for missing in &init.missing {
        let bytes = files
            .iter()
            .find(|(_, b)| sha_of(b) == missing.sha256.as_str())
            .map(|(_, b)| *b)
            .expect("the server only asks for shas from the manifest");
        let code = cas::upload_blob(
            State(h.state.clone()),
            Extension(h.user.clone()),
            Path((init.upload_id.clone(), missing.sha256.as_str().to_string())),
            Body::from(bytes.to_vec()),
        )
        .await
        .expect("subida de blob");
        assert_eq!(code, StatusCode::NO_CONTENT);
    }

    let snap = cas::commit(
        State(h.state.clone()),
        Extension(h.user.clone()),
        Path(SAVE.to_string()),
        Json(CasCommit {
            upload_id: init.upload_id,
            base_version: base,
            device_name: Some("ubserver".into()),
            notes: None,
            files: m,
        }),
    )
    .await
    .expect("commit")
    .1
     .0;

    ((snap.version_num, asked, asked_bytes), snap)
}

async fn used_bytes(pool: &SqlitePool) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT storage_used_bytes FROM users WHERE id=?")
        .bind(USER)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// The case that started it (Aug 2026): a large save where one file changes between
/// backups. The first pays for everything; the second pays only for the file that
/// changed, and the version is still complete.
#[tokio::test]
async fn the_second_backup_only_transmits_what_changed() {
    let h = harness().await;
    let big = vec![7u8; 200_000];
    let level = vec![1u8; 40_000];
    let level_v2 = vec![2u8; 40_000];

    let (v1, asked1, bytes1) =
        backup(&h, &[("mods.zip", &big), ("level.dat", &level)], Some(0)).await;
    assert_eq!(v1, 1);
    assert_eq!(asked1, 2, "la primera copia no tiene nada que reutilizar");
    assert_eq!(bytes1, 240_000);
    assert_eq!(used_bytes(&h.state.pool).await, 240_000);

    let (v2, asked2, bytes2) =
        backup(&h, &[("mods.zip", &big), ("level.dat", &level_v2)], Some(1)).await;
    assert_eq!(v2, 2);
    assert_eq!(asked2, 1, "el zip de mods no vuelve a viajar");
    assert_eq!(bytes2, 40_000);
    assert_eq!(
        used_bytes(&h.state.pool).await,
        280_000,
        "only the new bytes are charged"
    );

    // Version 2 is whole even though half the content never travelled.
    let files: Vec<(String, i64)> = sqlx::query_as(
        "SELECT sf.relative_path, sf.size_bytes FROM snapshot_files sf
           JOIN snapshots s ON s.id = sf.snapshot_id
          WHERE s.save_id=? AND s.version_num=2 ORDER BY sf.relative_path",
    )
    .bind(SAVE)
    .fetch_all(&h.state.pool)
    .await
    .unwrap();
    assert_eq!(
        files,
        vec![
            ("level.dat".to_string(), 40_000),
            ("mods.zip".to_string(), 200_000),
        ]
    );

    // And the shared blob is referenced by both versions, which is what stops
    // deleting one from taking the other's bytes with it.
    let refcount: i64 =
        sqlx::query_scalar("SELECT refcount FROM blobs WHERE user_id=? AND sha256=?")
            .bind(USER)
            .bind(sha_of(&big))
            .fetch_one(&h.state.pool)
            .await
            .unwrap();
    assert_eq!(refcount, 2);
}

/// What goes up through CAS has to **come back**. The download route knows nothing
/// about the upload protocol: it rebuilds the tar from `snapshot_files`, so this
/// checks the commit leaves the rows exactly as that route expects them. Without this
/// test, a version uploaded through CAS could be unrecoverable and nobody would find
/// out until somebody tried to restore it.
#[tokio::test]
async fn what_cas_uploads_restores_byte_for_byte() {
    use tokio::io::AsyncReadExt;

    let h = harness().await;
    let a: Vec<u8> = (0..50_000u32).map(|i| (i % 251) as u8).collect();
    let b = b"blueprints".to_vec();
    let files: Vec<(&str, &[u8])> = vec![("saves/mundo.zip", &a), ("mods/lista.json", &b)];
    backup(&h, &files, Some(0)).await;

    let resp = hoard_server::routes::snapshots::download(
        State(h.state.clone()),
        Extension(h.user.clone()),
        Path((SAVE.to_string(), 1)),
    )
    .await
    .expect("descarga");

    let body = axum::body::to_bytes(resp.into_body(), 64 * 1024 * 1024)
        .await
        .expect("cuerpo");
    let reader = async_compression::tokio::bufread::ZstdDecoder::new(std::io::Cursor::new(body));
    let mut archive = tokio_tar::Archive::new(reader);
    let mut entries = archive.entries().unwrap();
    let mut got: Vec<(String, Vec<u8>)> = Vec::new();
    while let Some(entry) = futures::StreamExt::next(&mut entries).await {
        let mut entry = entry.unwrap();
        let path = entry.path().unwrap().to_string_lossy().to_string();
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf).await.unwrap();
        got.push((path, buf));
    }
    got.sort_by(|x, y| x.0.cmp(&y.0));

    assert_eq!(got.len(), 2);
    assert_eq!(got[0].0, "mods/lista.json");
    assert_eq!(got[0].1, b);
    assert_eq!(got[1].0, "saves/mundo.zip");
    assert_eq!(got[1].1, a);
}

/// Bytes that do not hash to what they promise: they are rejected and nothing is
/// left in staging. It is the defence against the silent corruption of Aug 2026,
/// where the game rotates the save between the client hashing it and sending it, and
/// without this the server would store new content under the old one's sha.
#[tokio::test]
async fn bytes_that_dont_match_their_sha_are_refused() {
    let h = harness().await;
    let good = b"el save de verdad".to_vec();
    let m = manifest(&[("save", &good)]);
    let init = cas::init(
        State(h.state.clone()),
        Extension(h.user.clone()),
        Path(SAVE.to_string()),
        Json(CasInit {
            base_version: Some(0),
            files: m.clone(),
        }),
    )
    .await
    .expect("init")
    .0;

    let err = cas::upload_blob(
        State(h.state.clone()),
        Extension(h.user.clone()),
        Path((
            init.upload_id.clone(),
            init.missing[0].sha256.as_str().to_string(),
        )),
        Body::from(b"otra partida distinta".to_vec()),
    )
    .await
    .expect_err("el sha no cuadra");
    assert_eq!(err.0, StatusCode::BAD_REQUEST);

    // Y el commit no puede rescatarlo: sin el blob en staging, el manifiesto
    // referencia contenido que el server no tiene.
    let err = cas::commit(
        State(h.state.clone()),
        Extension(h.user.clone()),
        Path(SAVE.to_string()),
        Json(CasCommit {
            upload_id: init.upload_id,
            base_version: Some(0),
            device_name: None,
            notes: None,
            files: m,
        }),
    )
    .await
    .expect_err("commit sin bytes");
    assert_eq!(err.0, StatusCode::BAD_REQUEST);

    let versions: i64 = sqlx::query_scalar("SELECT latest_version_num FROM saves WHERE id=?")
        .bind(SAVE)
        .fetch_one(&h.state.pool)
        .await
        .unwrap();
    assert_eq!(versions, 0, "no version was created");
}

/// The other face of the rejection above: a base that does not line up but whose
/// manifest carries the head **whole and then some**. There is nothing to bury, since
/// the head's content travels file by file in the version about to be written, so it
/// passes. It is the way out for a client that lost its place in the account (a
/// pruned history, a rebuilt row) and cannot read the head off the 409: without it,
/// it retries every ten minutes until it gives up.
#[tokio::test]
async fn a_diverged_base_passes_when_the_manifest_carries_the_whole_head() {
    let h = harness().await;
    let old = b"contenido".to_vec();
    backup(&h, &[("save", &old)], Some(0)).await;

    // La cabeza es 1. Subimos con base 0,desfasada, pero llevando el fichero
    // de la cabeza intacto y uno nuevo encima.
    let extra = b"nuevo".to_vec();
    let out = cas::init(
        State(h.state.clone()),
        Extension(h.user.clone()),
        Path(SAVE.to_string()),
        Json(CasInit {
            base_version: Some(0),
            files: manifest(&[("save", &old), ("otro", &extra)]),
        }),
    )
    .await
    .expect("el manifiesto contiene la cabeza entera")
    .0;
    assert_eq!(
        out.version_num, 2,
        "descendemos de la cabeza real, no de la base"
    );

    // Y quitando el fichero de la cabeza vuelve a ser el entierro de siempre.
    let err = cas::init(
        State(h.state.clone()),
        Extension(h.user.clone()),
        Path(SAVE.to_string()),
        Json(CasInit {
            base_version: Some(0),
            files: manifest(&[("otro", &extra)]),
        }),
    )
    .await
    .expect_err("perder el fichero de la cabeza sigue siendo non-fast-forward");
    assert_eq!(err.0, StatusCode::CONFLICT);
    assert_eq!(err.1["code"], "non_fast_forward");
}

/// Another machine pushed while we were uploading. The init would cut it off before
/// moving a byte; the commit checks again because minutes pass between the two.
#[tokio::test]
async fn a_diverged_head_is_refused_before_and_after_the_upload() {
    let h = harness().await;
    let data = b"contenido".to_vec();
    backup(&h, &[("save", &data)], Some(0)).await;

    // Init con una base que ya no es la cabeza.
    let m = manifest(&[("save", &data)]);
    let err = cas::init(
        State(h.state.clone()),
        Extension(h.user.clone()),
        Path(SAVE.to_string()),
        Json(CasInit {
            base_version: Some(0),
            files: m.clone(),
        }),
    )
    .await
    .expect_err("non-fast-forward");
    assert_eq!(err.0, StatusCode::CONFLICT);
    // The body has to name the row it rejected against, not just the versions.
    // On Cloud that id can be a row the client has never heard of (its local id
    // resolves by game+label), and a client that can't read it back has no way
    // to find the head it must reconcile with, it looks itself up by the id it
    // knows, finds nothing, and parks the conflict forever.
    assert_eq!(err.1["code"], "non_fast_forward");
    assert_eq!(err.1["head_version"], 1);
    assert_eq!(err.1["base_version"], 0);
    assert_eq!(err.1["save_id"], SAVE);

    // Init correcto, y entre medias otro equipo avanza la cabeza: el commit
    // tiene que negarse igual.
    let init = cas::init(
        State(h.state.clone()),
        Extension(h.user.clone()),
        Path(SAVE.to_string()),
        Json(CasInit {
            base_version: Some(1),
            files: m.clone(),
        }),
    )
    .await
    .expect("init")
    .0;
    sqlx::query("UPDATE saves SET latest_version_num=9 WHERE id=?")
        .bind(SAVE)
        .execute(&h.state.pool)
        .await
        .unwrap();
    let err = cas::commit(
        State(h.state.clone()),
        Extension(h.user.clone()),
        Path(SAVE.to_string()),
        Json(CasCommit {
            upload_id: init.upload_id,
            base_version: Some(1),
            device_name: None,
            notes: None,
            files: m,
        }),
    )
    .await
    .expect_err("non-fast-forward en el commit");
    assert_eq!(err.0, StatusCode::CONFLICT);
    assert_eq!(err.1["head_version"], 9);
    assert_eq!(err.1["save_id"], SAVE);
}

/// The per-version cap is measured on the save's logical size, and it is now known
/// **before** anything is transmitted. The multipart could only abort mid-upload,
/// which is what left the user with a 413 and no number.
#[tokio::test]
async fn the_snapshot_cap_is_answered_before_any_byte_moves() {
    let h = harness().await;
    // The harness sets the cap at 64 MB; more than that is declared without uploading.
    let files = vec![CasFile {
        relative_path: "enorme.bin".into(),
        sha256: Sha256Hex::parse(&sha_of(b"x")).unwrap(),
        size_bytes: 200 * 1024 * 1024,
        modified_at: None,
    }];
    let err = cas::init(
        State(h.state.clone()),
        Extension(h.user.clone()),
        Path(SAVE.to_string()),
        Json(CasInit {
            base_version: Some(0),
            files,
        }),
    )
    .await
    .expect_err("por encima del tope");
    assert_eq!(err.0, StatusCode::PAYLOAD_TOO_LARGE);
    let body = serde_json::to_value(&err.1 .0).unwrap();
    assert_eq!(body["code"], "snapshot_too_large");
    assert_eq!(body["limit_bytes"], 64 * 1024 * 1024);
    // The real size, which is why it goes in `actual_bytes`: `received_bytes` means
    // "how far the transmission got before it was cut", and nothing has been
    // transmitted here yet. Sending it under that name made the client tell the user
    // "3.6 GB sent before stopping" about an upload that sent not one byte (Aug
    // 2026).
    assert_eq!(body["actual_bytes"], 200 * 1024 * 1024);
    assert!(
        body.get("received_bytes").is_none(),
        "un rechazo antes de mover bytes no puede hablar de bytes recibidos: {body}"
    );
}

/// The staging area belongs to whoever opened it. Somebody else's id allows neither
/// upload nor commit, and answers the same as an invented one.
#[tokio::test]
async fn another_users_upload_area_is_not_reachable() {
    let h = harness().await;
    let data = b"mio".to_vec();
    let init = cas::init(
        State(h.state.clone()),
        Extension(h.user.clone()),
        Path(SAVE.to_string()),
        Json(CasInit {
            base_version: Some(0),
            files: manifest(&[("save", &data)]),
        }),
    )
    .await
    .expect("init")
    .0;

    let intruder = AuthUser {
        user_id: Uuid::new_v4(),
        username: "otro".into(),
        is_admin: false,
    };
    let err = cas::upload_blob(
        State(h.state.clone()),
        Extension(intruder),
        Path((init.upload_id.clone(), sha_of(&data))),
        Body::from(data.clone()),
    )
    .await
    .expect_err("no es su subida");
    assert_eq!(err.0, StatusCode::NOT_FOUND);

    // Y un upload_id que no es un UUID no llega ni a tocar el disco.
    let err = cas::upload_blob(
        State(h.state.clone()),
        Extension(h.user.clone()),
        Path(("../../escape".to_string(), sha_of(&data))),
        Body::from(data),
    )
    .await
    .expect_err("invalid id");
    assert_eq!(err.0, StatusCode::BAD_REQUEST);
}

/// What the history's row says about a version.
///
/// With 70 worlds in the same folder, the row has to name the one that was touched
/// and say how much changed; the version number and the date, which is all it used to
/// say, tell no backup apart from the next. It is checked end to end (the mtime the
/// client declares, the stored manifest, the diff against the previous version)
/// because each piece lives somewhere different.
#[tokio::test]
async fn the_history_row_says_which_save_moved() {
    let h = harness().await;
    let world = vec![1u8; 4_000];
    let world_v2 = vec![2u8; 5_000];
    let autosave = vec![3u8; 4_000];

    let (_, first) = backup_at(
        &h,
        &[("adwdaw.zip", &world), ("_autosave1.zip", &autosave)],
        &[1_000, 2_000],
        None,
    )
    .await;
    let i = first
        .insight
        .expect("the first version already carries an insight");
    // Nothing changed because there is no previous version, but the folder already
    // holds two saves and one of them has a name: the autosave cannot be the
    // headline.
    assert_eq!(i.title.as_deref(), Some("adwdaw"));
    assert_eq!(i.entries, 2);
    assert_eq!(i.changed_files, 0);
    assert_eq!(i.delta_bytes, 0);

    let (_, second) = backup_at(
        &h,
        &[("adwdaw.zip", &world_v2), ("_autosave1.zip", &autosave)],
        &[3_000, 2_000],
        Some(1),
    )
    .await;
    let i = second.insight.expect("the second one too");
    assert_eq!(i.title.as_deref(), Some("adwdaw"));
    assert_eq!(i.primary_path.as_deref(), Some("adwdaw.zip"));
    assert_eq!(i.changed_files, 1);
    assert_eq!(i.removed_files, 0);
    assert_eq!(i.delta_bytes, 1_000);
}
