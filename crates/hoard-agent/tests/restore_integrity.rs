//! A restore that fails must leave the save exactly as it found it.
//!
//! The restore used to open each destination file with `File::create`, stream
//! the bytes in and only then compare the SHA-256. A corrupt or cut-short body
//! was caught, but after the good save had already been emptied and
//! overwritten; on Cloud a failed blob also cancelled the three downloading
//! beside it, halfway through their own files. R2 cuts bodies short often
//! enough that this needed no attacker.
//!
//! Each test stands up a small HTTP server on loopback that talks like one of
//! the three download paths (self-hosted tar, Cloud blobs, Cloud's old
//! whole-archive versions), breaks the bytes one way or another, and checks the
//! folder byte for byte and mtime for mtime afterwards.

use std::collections::{BTreeMap, HashMap};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use async_compression::tokio::write::ZstdEncoder;
use hoard_agent::api::ApiClient;
use hoard_agent::restore::{download_snapshot, RestoreOptions};
use hoard_core::kernel::fileclass::RestoreGate;
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

const SAVE: &str = "8a4a2f1e-0000-4000-8000-000000000001";

#[derive(Clone)]
struct Route {
    body: Vec<u8>,
    /// Announce the whole length, send this much, hang up: what R2 does.
    cut_at: Option<usize>,
}

type Routes = Arc<Mutex<HashMap<String, Route>>>;

/// A server that answers GETs from a table, one connection per request.
fn spawn_stub() -> (String, Routes) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let base = format!("http://{}", listener.local_addr().expect("addr"));
    let routes: Routes = Arc::default();
    let table = routes.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { break };
            let table = table.clone();
            std::thread::spawn(move || {
                let _ = serve_one(stream, &table);
            });
        }
    });
    (base, routes)
}

fn serve_one(mut stream: TcpStream, routes: &Routes) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 || line.trim_end().is_empty() {
            break;
        }
    }
    let target = request_line.split_whitespace().nth(1).unwrap_or_default();
    let path = target.split('?').next().unwrap_or_default().to_string();
    let route = routes.lock().unwrap().get(&path).cloned();
    let Some(route) = route else {
        let body = format!("no route for {path}");
        write!(
            stream,
            "HTTP/1.1 404 Not Found\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )?;
        return Ok(());
    };
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        route.body.len()
    )?;
    let sent = route
        .cut_at
        .unwrap_or(route.body.len())
        .min(route.body.len());
    stream.write_all(&route.body[..sent])?;
    stream.flush()?;
    Ok(())
}

fn route(routes: &Routes, path: &str, body: impl Into<Vec<u8>>) {
    routes.lock().unwrap().insert(
        path.to_string(),
        Route {
            body: body.into(),
            cut_at: None,
        },
    );
}

fn cut_route(routes: &Routes, path: &str, body: impl Into<Vec<u8>>, cut_at: usize) {
    routes.lock().unwrap().insert(
        path.to_string(),
        Route {
            body: body.into(),
            cut_at: Some(cut_at),
        },
    );
}

fn sha(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// What a folder holds, file by file: bytes and mtime.
fn picture(dir: &Path) -> BTreeMap<PathBuf, (Vec<u8>, filetime::FileTime)> {
    let mut out = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in std::fs::read_dir(&d).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                stack.push(path);
            } else {
                let meta = std::fs::metadata(&path).unwrap();
                out.insert(
                    path.strip_prefix(dir).unwrap().to_path_buf(),
                    (
                        std::fs::read(&path).unwrap(),
                        filetime::FileTime::from_last_modification_time(&meta),
                    ),
                );
            }
        }
    }
    out
}

/// A save folder with a good local copy of everything, stamped with an old
/// mtime so a rewrite would show.
fn good_save(root: &Path) -> PathBuf {
    let dest = root.join("Saves");
    for (name, bytes) in [
        ("slot1.sav", b"slot 1, the good local save".as_slice()),
        ("slot2.sav", b"slot 2, also good".as_slice()),
        ("notes/journal.txt", b"only here".as_slice()),
    ] {
        let path = dest.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, bytes).unwrap();
        filetime::set_file_mtime(&path, filetime::FileTime::from_unix_time(1_600_000_000, 0))
            .unwrap();
    }
    dest
}

/// The version on the server: both slots changed.
fn version_files() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        (
            "slot1.sav",
            b"slot 1, as uploaded from the other PC".to_vec(),
        ),
        (
            "slot2.sav",
            b"slot 2, as uploaded from the other PC!!".to_vec(),
        ),
    ]
}

async fn tar_zst(entries: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut builder = tokio_tar::Builder::new(Vec::new());
    for (name, bytes) in entries {
        let mut header = tokio_tar::Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(0o644);
        header.set_mtime(1_700_000_000);
        header.set_entry_type(tokio_tar::EntryType::Regular);
        header.set_cksum();
        builder
            .append_data(&mut header, name, bytes.as_slice())
            .await
            .unwrap();
    }
    let tar = builder.into_inner().await.unwrap();
    let mut enc = ZstdEncoder::new(Vec::new());
    enc.write_all(&tar).await.unwrap();
    enc.shutdown().await.unwrap();
    enc.into_inner()
}

fn options(backup_root: &Path) -> RestoreOptions {
    RestoreOptions {
        skip_verify: false,
        // What `hoard restore --force` and the History dialog pass: the user
        // said to overwrite.
        force: true,
        reuse_from: None,
        gate: RestoreGate::permissive(),
        backup_root: Some(backup_root.to_path_buf()),
        // Beside it, in the test's own temp folder: never the real state folder
        // a running service stages in.
        staging_root: Some(backup_root.with_file_name("staging")),
        narrow: None,
    }
}

// ---- self-hosted

fn self_hosted(
    routes: &Routes,
    listed: &[(&str, Vec<u8>)],
    archive: Vec<u8>,
    cut_at: Option<usize>,
) {
    route(
        routes,
        "/v1/health",
        r#"{"status":"ok","version":"test","cas":true}"#,
    );
    let files: Vec<serde_json::Value> = listed
        .iter()
        .map(|(name, bytes)| {
            serde_json::json!({
                "relative_path": name,
                "size_bytes": bytes.len(),
                "sha256": sha(bytes),
            })
        })
        .collect();
    let detail = serde_json::json!({
        "id": "snap-1",
        "version_num": 1,
        "total_size_bytes": listed.iter().map(|(_, b)| b.len()).sum::<usize>(),
        "file_count": listed.len(),
        "is_pinned": false,
        "created_at": "2026-09-29T10:00:00Z",
        "files": files,
    });
    route(
        routes,
        &format!("/v1/saves/{SAVE}/snapshots/1"),
        detail.to_string(),
    );
    let download = format!("/v1/saves/{SAVE}/snapshots/1/download");
    match cut_at {
        Some(at) => cut_route(routes, &download, archive, at),
        None => route(routes, &download, archive),
    }
}

async fn assert_fails_untouched(base: &str, tmp: &Path, expect: &str) {
    let dest = tmp.join("Saves");
    let before = picture(&dest);
    let client = ApiClient::new(base, "token").unwrap();
    let err = download_snapshot(
        &client,
        SAVE,
        1,
        &dest,
        options(&tmp.join("kept")),
        |_, _| {},
    )
    .await
    .expect_err("the restore has to fail");
    assert!(
        format!("{err:#}").contains(expect),
        "expected {expect:?} in: {err:#}"
    );
    assert_eq!(picture(&dest), before, "the save folder changed");
}

#[tokio::test]
async fn self_hosted_corrupt_file_leaves_the_save_untouched() {
    let tmp = tempfile::tempdir().unwrap();
    good_save(tmp.path());
    let (base, routes) = spawn_stub();
    let listed = version_files();
    // Same length, other bytes: a blob that rotted on the server's disk.
    let mut served = listed.clone();
    served[1].1 = vec![b'X'; served[1].1.len()];
    self_hosted(&routes, &listed, tar_zst(&served).await, None);

    assert_fails_untouched(&base, tmp.path(), "sha256 mismatch").await;
}

#[tokio::test]
async fn self_hosted_download_cut_short_leaves_the_save_untouched() {
    let tmp = tempfile::tempdir().unwrap();
    good_save(tmp.path());
    let (base, routes) = spawn_stub();
    // Big and incompressible, so the cut lands after the first file is whole
    // and inside the second: the point where writing in place did its damage.
    let mut noise = Vec::with_capacity(512 * 1024);
    let mut x: u32 = 0x2545_f491;
    while noise.len() < 512 * 1024 {
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        noise.extend_from_slice(&x.to_le_bytes());
    }
    let listed = vec![("slot1.sav", noise.clone()), ("slot2.sav", noise)];
    let archive = tar_zst(&listed).await;
    let cut = archive.len() * 3 / 4;
    self_hosted(&routes, &listed, archive, Some(cut));

    assert_fails_untouched(
        &base,
        tmp.path(),
        "end of file before message length reached",
    )
    .await;
}

#[tokio::test]
async fn self_hosted_archive_short_of_a_file_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    good_save(tmp.path());
    let (base, routes) = spawn_stub();
    let listed = version_files();
    self_hosted(&routes, &listed, tar_zst(&listed[..1]).await, None);

    assert_fails_untouched(&base, tmp.path(), "ended without 1 of its files").await;
}

#[tokio::test]
async fn self_hosted_archive_with_a_stranger_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    good_save(tmp.path());
    let (base, routes) = spawn_stub();
    let listed = version_files();
    let mut served = listed.clone();
    served.push(("autoexec.cfg", b"bind x quit".to_vec()));
    self_hosted(&routes, &listed, tar_zst(&served).await, None);

    assert_fails_untouched(&base, tmp.path(), "not in v1's file list").await;
}

#[tokio::test]
async fn self_hosted_restore_lands_whole_and_keeps_the_originals() {
    let tmp = tempfile::tempdir().unwrap();
    let dest = good_save(tmp.path());
    let (base, routes) = spawn_stub();
    let listed = version_files();
    self_hosted(&routes, &listed, tar_zst(&listed).await, None);

    let client = ApiClient::new(&base, "token").unwrap();
    let kept_root = tmp.path().join("kept");
    let outcome = download_snapshot(&client, SAVE, 1, &dest, options(&kept_root), |_, _| {})
        .await
        .unwrap();

    for (name, bytes) in &listed {
        assert_eq!(&std::fs::read(dest.join(name)).unwrap(), bytes);
    }
    assert_eq!(
        std::fs::read(dest.join("notes/journal.txt")).unwrap(),
        b"only here"
    );
    let kept = outcome.kept_in.expect("it replaced two files");
    assert!(kept.starts_with(&kept_root));
    assert_eq!(
        std::fs::read(kept.join("slot1.sav")).unwrap(),
        b"slot 1, the good local save"
    );
}

// ---- Cloud, content-addressed

fn cloud(routes: &Routes, base: &str, listed: &[(&str, Vec<u8>)]) {
    route(
        routes,
        "/v1/health",
        r#"{"status":"ok","version":"test","mode":"cloud"}"#,
    );
    let files: Vec<serde_json::Value> = listed
        .iter()
        .enumerate()
        .map(|(i, (name, bytes))| {
            serde_json::json!({
                "relative_path": name,
                "sha256": sha(bytes),
                "size_bytes": bytes.len(),
                "modified_at": 1_700_000_000,
                "download": {"method": "GET", "url": format!("{base}/blob/{i}")},
            })
        })
        .collect();
    route(
        routes,
        &format!("/v1/cloud/saves/{SAVE}/versions/1/manifest"),
        serde_json::json!({"content_addressed": true, "files": files}).to_string(),
    );
    for (i, (_, bytes)) in listed.iter().enumerate() {
        route(routes, &format!("/blob/{i}"), bytes.clone());
    }
}

#[tokio::test]
async fn cloud_corrupt_blob_leaves_the_save_untouched() {
    let tmp = tempfile::tempdir().unwrap();
    good_save(tmp.path());
    let (base, routes) = spawn_stub();
    let listed = version_files();
    cloud(&routes, &base, &listed);
    route(&routes, "/blob/1", vec![b'X'; listed[1].1.len()]);

    assert_fails_untouched(&base, tmp.path(), "sha256 mismatch").await;
}

#[tokio::test]
async fn cloud_blob_cut_short_leaves_the_save_untouched() {
    let tmp = tempfile::tempdir().unwrap();
    good_save(tmp.path());
    let (base, routes) = spawn_stub();
    let listed = version_files();
    cloud(&routes, &base, &listed);
    cut_route(&routes, "/blob/0", listed[0].1.clone(), 5);

    assert_fails_untouched(
        &base,
        tmp.path(),
        "end of file before message length reached",
    )
    .await;
}

#[tokio::test]
async fn cloud_restore_reuses_local_bytes_and_lands_whole() {
    let tmp = tempfile::tempdir().unwrap();
    let dest = good_save(tmp.path());
    let (base, routes) = spawn_stub();
    let listed = version_files();
    cloud(&routes, &base, &listed);
    // The first file's new content is already on disk under another name, so
    // it is copied rather than fetched; the route for it answers garbage to
    // prove it.
    std::fs::write(dest.join("backup-of-slot1.sav"), &listed[0].1).unwrap();
    route(&routes, "/blob/0", vec![b'X'; listed[0].1.len()]);

    let client = ApiClient::new(&base, "token").unwrap();
    let mut opts = options(&tmp.path().join("kept"));
    opts.reuse_from = Some(dest.clone());
    let outcome = download_snapshot(&client, SAVE, 1, &dest, opts, |_, _| {})
        .await
        .unwrap();

    assert_eq!(outcome.files_reused, 1);
    for (name, bytes) in &listed {
        assert_eq!(&std::fs::read(dest.join(name)).unwrap(), bytes);
    }
    let mtime = filetime::FileTime::from_last_modification_time(
        &std::fs::metadata(dest.join("slot2.sav")).unwrap(),
    );
    assert_eq!(mtime.unix_seconds(), 1_700_000_000);
}

// ---- Cloud, old whole-archive versions

#[tokio::test]
async fn cloud_archive_with_a_bad_sha_leaves_the_save_untouched() {
    let tmp = tempfile::tempdir().unwrap();
    good_save(tmp.path());
    let (base, routes) = spawn_stub();
    route(
        &routes,
        "/v1/health",
        r#"{"status":"ok","version":"test","mode":"cloud"}"#,
    );
    route(
        &routes,
        &format!("/v1/cloud/saves/{SAVE}/versions/1/manifest"),
        r#"{"content_addressed":false}"#,
    );
    let archive = tar_zst(&version_files()).await;
    route(
        &routes,
        &format!("/v1/cloud/saves/{SAVE}/versions/1/download"),
        serde_json::json!({
            "save_id": SAVE,
            "version_num": 1,
            "sha256": sha(b"some other archive"),
            "size_bytes": archive.len(),
            "download": {"method": "GET", "url": format!("{base}/archive")},
        })
        .to_string(),
    );
    route(&routes, "/archive", archive);

    assert_fails_untouched(&base, tmp.path(), "sha256 mismatch for v1").await;
}

// ---- where the placement meets the disk

/// A version uploaded from Linux can carry two spellings of one name. Where
/// case folds (here a Wine prefix, which folds on any OS) both would be
/// written into one staged file at once; the restore refuses before a byte
/// moves.
#[tokio::test]
async fn two_spellings_of_one_file_are_refused_where_case_folds() {
    let tmp = tempfile::tempdir().unwrap();
    let prefix = tmp.path().join("pfx/drive_c");
    let dest = good_save(&prefix);
    let before = picture(&dest);
    let (base, routes) = spawn_stub();
    let listed = vec![
        ("slot1.sav", b"lower-case spelling".to_vec()),
        ("Slot1.sav", b"upper-case spelling".to_vec()),
    ];
    cloud(&routes, &base, &listed);

    let client = ApiClient::new(&base, "token").unwrap();
    let err = download_snapshot(
        &client,
        SAVE,
        1,
        &dest,
        options(&tmp.path().join("kept")),
        |_, _| {},
    )
    .await
    .expect_err("two names for one file");
    assert!(
        format!("{err:#}").contains("which are the same file here"),
        "{err:#}"
    );
    assert_eq!(picture(&dest), before, "the save folder changed");
}

/// The original is put aside as a hard link where the disk allows it: no
/// second copy of a large save, and it is the very file that was there.
#[cfg(unix)]
#[tokio::test]
async fn the_replaced_file_is_kept_as_itself() {
    use std::os::unix::fs::MetadataExt;
    let tmp = tempfile::tempdir().unwrap();
    let dest = good_save(tmp.path());
    let inode = std::fs::metadata(dest.join("slot1.sav")).unwrap().ino();
    let (base, routes) = spawn_stub();
    let listed = version_files();
    cloud(&routes, &base, &listed);

    let client = ApiClient::new(&base, "token").unwrap();
    let outcome = download_snapshot(
        &client,
        SAVE,
        1,
        &dest,
        options(&tmp.path().join("kept")),
        |_, _| {},
    )
    .await
    .unwrap();

    let kept = outcome
        .kept_in
        .expect("it replaced two files")
        .join("slot1.sav");
    assert_eq!(std::fs::metadata(&kept).unwrap().ino(), inode);
    assert_eq!(
        std::fs::read(&kept).unwrap(),
        b"slot 1, the good local save"
    );
    assert_eq!(std::fs::read(dest.join("slot1.sav")).unwrap(), listed[0].1);
}
