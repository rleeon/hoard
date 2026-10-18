//! A big save that changes a little goes up the second time as a patch, and
//! comes back byte for byte, with the base on this machine and without it.
//!
//! Loopback servers play Cloud (patches stored as such, the manifest hands out
//! each blob with its base) and self-hosted (patches rebuilt on arrival).

use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use hoard_agent::api::ApiClient;
use hoard_agent::backup::upload_directory;
use hoard_agent::restore::{download_snapshot, RestoreOptions};
use hoard_core::kernel::fileclass::RestoreGate;
use hoard_core::wire::VersionOrigin;
use sha2::{Digest, Sha256};

const SAVE: &str = "8a4a2f1e-0000-4000-8000-0000000000aa";

#[derive(Default)]
struct Cloud {
    base: String,
    objects: HashMap<String, Vec<u8>>,
    /// sha -> (encoding, base) as the commits declared them.
    blobs: HashMap<String, (Option<String>, Option<String>)>,
    versions: Vec<Vec<(String, String, usize)>>,
    pending: Vec<(String, String, usize)>,
    puts: Vec<(String, usize)>,
}

struct Req {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

type Handler<S> = fn(&Req, &mut S) -> (&'static str, Vec<u8>);

/// A loopback HTTP server over `state`, whose own URL `state` is built with.
fn spawn<S: Send + 'static>(
    state: impl FnOnce(String) -> S,
    handle: Handler<S>,
) -> (String, Arc<Mutex<S>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let shared = Arc::new(Mutex::new(state(base.clone())));
    let s = shared.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let s = s.clone();
            std::thread::spawn(move || {
                let _ = serve(stream, &s, handle);
            });
        }
    });
    (base, shared)
}

fn spawn_cloud() -> (String, Arc<Mutex<Cloud>>) {
    spawn(
        |base| Cloud {
            base,
            ..Default::default()
        },
        handle,
    )
}

fn sha(b: &[u8]) -> String {
    hex::encode(Sha256::digest(b))
}

fn serve<S>(mut stream: TcpStream, state: &Mutex<S>, handle: Handler<S>) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let mut headers = HashMap::new();
    loop {
        let mut h = String::new();
        if reader.read_line(&mut h)? == 0 || h.trim_end().is_empty() {
            break;
        }
        if let Some((k, v)) = h.split_once(':') {
            headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
        }
    }
    let len = headers
        .get("content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let mut body = vec![0u8; len];
    reader.read_exact(&mut body)?;
    let mut parts = line.split_whitespace();
    let req = Req {
        method: parts.next().unwrap_or_default().to_string(),
        path: parts
            .next()
            .unwrap_or_default()
            .split('?')
            .next()
            .unwrap_or_default()
            .to_string(),
        headers,
        body,
    };
    let (status, out) = handle(&req, &mut state.lock().unwrap());
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        out.len()
    )?;
    stream.write_all(&out)?;
    Ok(())
}

fn handle(r: &Req, c: &mut Cloud) -> (&'static str, Vec<u8>) {
    let ok = |v: serde_json::Value| ("200 OK", v.to_string().into_bytes());
    let (method, path, body) = (r.method.as_str(), r.path.as_str(), r.body.as_slice());
    match (method, path) {
        ("GET", "/v1/health") => ok(serde_json::json!({
            "status": "ok", "version": "test", "mode": "cloud", "blob_zstd": true, "blob_patch": true
        })),
        ("POST", "/v1/cloud/cas/init") => {
            let req: serde_json::Value = serde_json::from_slice(body).unwrap();
            c.pending = req["files"]
                .as_array()
                .unwrap()
                .iter()
                .map(|f| {
                    (
                        f["relative_path"].as_str().unwrap().to_string(),
                        f["sha256"].as_str().unwrap().to_string(),
                        f["size_bytes"].as_u64().unwrap() as usize,
                    )
                })
                .collect();
            let patch_from: HashMap<String, String> = req["patch_from"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .map(|p| {
                            (
                                p["sha256"].as_str().unwrap().to_string(),
                                p["base"].as_str().unwrap().to_string(),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default();
            let put = |key: &str| serde_json::json!({"method": "PUT", "url": format!("{}/put/{key}", c.base)});
            let missing: Vec<serde_json::Value> = c
                .pending
                .iter()
                .filter(|(_, s, _)| !c.blobs.contains_key(s))
                .map(|(_, s, z)| {
                    let mut m = serde_json::json!({"sha256": s, "size_bytes": z, "upload": put(s)});
                    // Offered only against a base held whole, at keys of its own.
                    if let Some(b) = patch_from
                        .get(s)
                        .filter(|b| matches!(c.blobs.get(*b), Some((_, None))))
                    {
                        m["patch"] = serde_json::json!({
                            "base": b,
                            "upload": put(&format!("{s}.zp.{b}")),
                            "inflated_upload": put(&format!("{s}.zpi.{b}")),
                        });
                    }
                    m
                })
                .collect();
            ok(serde_json::json!({
                "save_id": SAVE, "version_num": c.versions.len() + 1, "missing": missing,
                "quota": {"plan": "pro", "used_bytes": 0, "limit_bytes": 1u64 << 40},
            }))
        }
        ("PUT", p) if p.starts_with("/put/") => {
            let s = p.trim_start_matches("/put/").to_string();
            c.puts.push((s.clone(), body.len()));
            c.objects.insert(s, body.to_vec());
            ("200 OK", Vec::new())
        }
        ("POST", p) if p.ends_with("/cas/commit") => {
            let req: serde_json::Value = serde_json::from_slice(body).unwrap();
            let list = |k: &str| -> HashSet<String> {
                req[k]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default()
            };
            let zstd = list("zstd");
            let patches: HashMap<String, (String, String)> = req["patches"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .map(|p| {
                            (
                                p["sha256"].as_str().unwrap().to_string(),
                                (
                                    p["base"].as_str().unwrap().to_string(),
                                    p["encoding"].as_str().unwrap().to_string(),
                                ),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default();
            for (_, s, _) in &c.pending {
                if c.blobs.contains_key(s) {
                    continue;
                }
                let entry = match patches.get(s) {
                    Some((b, e)) => (Some(e.clone()), Some(b.clone())),
                    None if zstd.contains(s) => (Some("zstd".to_string()), None),
                    None => (None, None),
                };
                c.blobs.insert(s.clone(), entry);
            }
            let v = std::mem::take(&mut c.pending);
            c.versions.push(v);
            ok(
                serde_json::json!({"save_id": SAVE, "version_num": c.versions.len(), "committed": true}),
            )
        }
        ("GET", p) if p.ends_with("/manifest") => {
            let v: usize = p.rsplit('/').nth(1).unwrap().parse().unwrap();
            let files: Vec<serde_json::Value> = c.versions[v - 1]
                .iter()
                .map(|(rel, s, z)| {
                    let (enc, base) = c.blobs[s].clone();
                    let key = match (&enc, &base) {
                        (Some(e), Some(b)) if e == "zstd-patch-inflated" => format!("{s}.zpi.{b}"),
                        (_, Some(b)) => format!("{s}.zp.{b}"),
                        _ => s.clone(),
                    };
                    let mut f = serde_json::json!({
                        "relative_path": rel, "sha256": s, "size_bytes": z, "modified_at": 1_700_000_000,
                        "download": {"method": "GET", "url": format!("{}/get/{key}", c.base)},
                    });
                    if let Some(e) = enc {
                        f["encoding"] = e.into();
                    }
                    if let Some(b) = base {
                        f["base_download"] = serde_json::json!({"method": "GET", "url": format!("{}/get/{b}", c.base)});
                        if c.blobs[&b].0.as_deref() == Some("zstd") {
                            f["base_encoding"] = "zstd".into();
                        }
                        f["base_sha256"] = b.into();
                    }
                    f
                })
                .collect();
            ok(serde_json::json!({"content_addressed": true, "files": files}))
        }
        ("GET", p) if p.starts_with("/get/") => {
            match c.objects.get(p.trim_start_matches("/get/")) {
                Some(b) => ("200 OK", b.clone()),
                None => ("404 Not Found", Vec::new()),
            }
        }
        _ => ("404 Not Found", b"{}".to_vec()),
    }
}

/// Self-hosted: stores what it gets raw, rebuilding patches against the base
/// it holds, as `routes/cas.rs` does.
#[derive(Default)]
struct SelfHosted {
    blobs: HashMap<String, Vec<u8>>,
    /// (sha, encoding, base, bytes sent) per PUT.
    puts: Vec<(String, Option<String>, Option<String>, usize)>,
    versions: i64,
    /// Answers every patch with `patch_base_gone`, as if the base was purged
    /// between the init and the PUT.
    lost_bases: bool,
}

fn handle_self_hosted(r: &Req, s: &mut SelfHosted) -> (&'static str, Vec<u8>) {
    let ok = |v: serde_json::Value| ("200 OK", v.to_string().into_bytes());
    match (r.method.as_str(), r.path.as_str()) {
        ("GET", "/v1/health") => ok(serde_json::json!({
            "status": "ok", "version": "test", "cas": true, "blob_zstd": true, "blob_patch": true
        })),
        ("POST", p) if p.ends_with("/cas/init") => {
            let req: serde_json::Value = serde_json::from_slice(&r.body).unwrap();
            let missing: Vec<serde_json::Value> = req["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|f| !s.blobs.contains_key(f["sha256"].as_str().unwrap()))
                .map(|f| serde_json::json!({"sha256": f["sha256"], "size_bytes": f["size_bytes"]}))
                .collect();
            let bases_ok: Vec<&str> = req["bases"]
                .as_array()
                .map(|a| a.iter().filter_map(|b| b.as_str()).collect())
                .unwrap_or_default();
            let bases_ok: Vec<&str> = bases_ok
                .into_iter()
                .filter(|b| s.blobs.contains_key(*b))
                .collect();
            ok(serde_json::json!({
                "upload_id": "u1", "version_num": s.versions + 1, "missing": missing,
                "missing_bytes": 0, "bases_ok": bases_ok,
            }))
        }
        ("PUT", p) if p.starts_with("/v1/cas/blobs/") => {
            let sha_in = p.rsplit('/').next().unwrap().to_string();
            let enc = r.headers.get("x-hoard-blob-encoding").cloned();
            let base = r.headers.get("x-hoard-blob-base").cloned();
            s.puts
                .push((sha_in.clone(), enc.clone(), base.clone(), r.body.len()));
            let raw = match enc.as_deref() {
                Some("zstd-patch") => {
                    let base = base.unwrap_or_default();
                    let Some(b) = s.blobs.get(&base).filter(|_| !s.lost_bases) else {
                        let gone = serde_json::json!({
                            "error": "gone", "code": "patch_base_gone", "base": base
                        });
                        return ("409 Conflict", gone.to_string().into_bytes());
                    };
                    rebuild(b, &r.body)
                }
                Some("zstd-patch-inflated") => {
                    let base = base.unwrap_or_default();
                    let b = s.blobs.get(&base).expect("the test keeps its bases");
                    let (streams, frame) = hoard_deflate::unpack(&r.body).unwrap();
                    let mut reference = Vec::new();
                    hoard_deflate::reference(b, &streams, &mut reference).unwrap();
                    let mut raw = Vec::new();
                    hoard_deflate::deflate(&rebuild(&reference, frame), &mut raw).unwrap();
                    raw
                }
                Some("zstd") => zstd::decode_all(&r.body[..]).unwrap(),
                _ => r.body.clone(),
            };
            if sha(&raw) != sha_in {
                return ("400 Bad Request", b"{}".to_vec());
            }
            s.blobs.insert(sha_in, raw);
            ("204 No Content", Vec::new())
        }
        ("POST", p) if p.ends_with("/cas/commit") => {
            s.versions += 1;
            ok(serde_json::json!({
                "id": "v", "version_num": s.versions, "total_size_bytes": 0, "file_count": 1,
                "is_pinned": false, "created_at": "2026-10-18T00:00:00Z",
            }))
        }
        _ => ("404 Not Found", b"{}".to_vec()),
    }
}

fn rebuild(base: &[u8], patch: &[u8]) -> Vec<u8> {
    use zstd::zstd_safe::{get_frame_content_size, DCtx, DParameter};
    let size = get_frame_content_size(patch).unwrap().unwrap() as usize;
    let mut dctx = DCtx::create();
    dctx.set_parameter(DParameter::WindowLogMax(31)).unwrap();
    dctx.ref_prefix(base).unwrap();
    let mut out = vec![0u8; size];
    let n = dctx.decompress(&mut out[..], patch).unwrap();
    out.truncate(n);
    out
}

/// A zip like a Stellaris save: one deflated entry of text, ~1.6 MB.
fn zip_save(text: &[u8]) -> Vec<u8> {
    use hoard_deflate::testing::{deflated, zip_of};
    zip_of(&[(
        "gamestate",
        8,
        deflated(text, 6, 8, hoard_deflate::Wrap::Raw),
    )])
}

fn gamestate(seed: u64) -> Vec<u8> {
    let mut x = seed;
    let mut out = Vec::with_capacity(5 << 20);
    while out.len() < 5 << 20 {
        x = x.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        out.extend_from_slice(
            format!("fleet={} planet={}\n", x >> 40, (x >> 20) & 0xfff).as_bytes(),
        );
    }
    out
}

/// Compressible like a real save, distinct per seed.
fn save_bytes(len: usize, seed: u64) -> Vec<u8> {
    let mut x = seed;
    (0..len)
        .map(|i| {
            x = x.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            if i % 5 == 0 {
                (x >> 33) as u8
            } else {
                (i / 64 % 256) as u8
            }
        })
        .collect()
}

fn options(tmp: &std::path::Path) -> RestoreOptions {
    RestoreOptions {
        skip_verify: false,
        force: true,
        reuse_from: None,
        gate: RestoreGate::permissive(),
        backup_root: Some(tmp.join("kept")),
        staging_root: Some(tmp.join("staging")),
        narrow: None,
    }
}

/// The env is per process: both tests run under one lock, each with its own home.
static ENV: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn isolate(home: &std::path::Path) {
    std::env::set_var("XDG_CACHE_HOME", home.join("cache"));
    std::env::set_var("XDG_DATA_HOME", home.join("data"));
    std::env::set_var("XDG_CONFIG_HOME", home.join("config"));
    std::env::set_var("HOARD_PROFILE", format!("patch-{}", std::process::id()));
}

async fn backup(
    client: &ApiClient,
    root: &std::path::Path,
    base: Option<i64>,
) -> anyhow::Result<()> {
    upload_directory(
        client,
        SAVE,
        "fallout-4",
        "main",
        root,
        base,
        None,
        VersionOrigin::Automatic,
        |_, _| {},
    )
    .await
    .map(|_| ())
}

#[tokio::test]
async fn a_changed_big_save_goes_up_as_a_patch_and_comes_back() {
    let _env = ENV.lock().await;
    let home = tempfile::tempdir().unwrap();
    isolate(home.path());

    let (base, cloud) = spawn_cloud();
    let client = ApiClient::new(&base, "token").unwrap();
    let root = home.path().join("save");
    std::fs::create_dir_all(&root).unwrap();
    let file = root.join("slot.fos");
    let v1 = save_bytes(3 << 20, 7);
    std::fs::write(&file, &v1).unwrap();

    backup(&client, &root, None).await.unwrap();
    assert_eq!(
        cloud.lock().unwrap().blobs[&sha(&v1)].0.as_deref(),
        Some("zstd")
    );

    let mut v2 = v1.clone();
    v2[1_500_000..1_500_200].fill(0x42);
    std::fs::write(&file, &v2).unwrap();
    backup(&client, &root, Some(1)).await.unwrap();
    {
        let c = cloud.lock().unwrap();
        assert_eq!(
            c.blobs[&sha(&v2)],
            (Some("zstd-patch".to_string()), Some(sha(&v1))),
            "the second copy goes as a patch against the first"
        );
        let key = format!("{}.zp.{}", sha(&v2), sha(&v1));
        let (_, sent) = c.puts.iter().find(|(s, _)| *s == key).unwrap();
        assert!(
            *sent * 20 < v2.len(),
            "{sent} bytes sent for a {} byte file",
            v2.len()
        );
        assert!(
            !c.puts.iter().any(|(s, _)| *s == sha(&v2)),
            "never under the whole blob's key"
        );
    }

    // This machine still has the base.
    let here = home.path().join("restore-here");
    std::fs::create_dir_all(&here).unwrap();
    download_snapshot(&client, SAVE, 2, &here, options(home.path()), |_, _| {})
        .await
        .unwrap();
    assert_eq!(std::fs::read(here.join("slot.fos")).unwrap(), v2);

    // Another machine doesn't: it fetches the base and checks it.
    std::fs::remove_dir_all(home.path().join("cache")).unwrap();
    let there = home.path().join("restore-there");
    std::fs::create_dir_all(&there).unwrap();
    download_snapshot(&client, SAVE, 2, &there, options(home.path()), |_, _| {})
        .await
        .unwrap();
    assert_eq!(std::fs::read(there.join("slot.fos")).unwrap(), v2);

    // A base that comes back damaged fails the restore instead of landing junk.
    std::fs::remove_dir_all(home.path().join("cache")).unwrap();
    cloud
        .lock()
        .unwrap()
        .objects
        .insert(sha(&v1), zstd::encode_all(&b"not the base"[..], 3).unwrap());
    let broken = home.path().join("restore-broken");
    std::fs::create_dir_all(&broken).unwrap();
    let err = download_snapshot(&client, SAVE, 2, &broken, options(home.path()), |_, _| {})
        .await
        .unwrap_err();
    assert!(format!("{err:#}").contains("base"), "{err:#}");
    assert!(!broken.join("slot.fos").exists());
}

#[tokio::test]
async fn self_hosted_rebuilds_the_patch_and_a_lost_base_goes_whole() {
    let _env = ENV.lock().await;
    let home = tempfile::tempdir().unwrap();
    isolate(home.path());

    let (base, server) = spawn(|_| SelfHosted::default(), handle_self_hosted);
    let client = ApiClient::new(&base, "token").unwrap();
    let root = home.path().join("save");
    std::fs::create_dir_all(&root).unwrap();
    let file = root.join("slot.fos");
    let last_put = |sha: &str| {
        let s = server.lock().unwrap();
        let (_, enc, base, sent) = s.puts.iter().rev().find(|p| p.0 == sha).unwrap().clone();
        (enc, base, sent)
    };

    let v1 = save_bytes(3 << 20, 7);
    std::fs::write(&file, &v1).unwrap();
    backup(&client, &root, None).await.unwrap();
    assert_eq!(last_put(&sha(&v1)).0.as_deref(), Some("zstd"));

    let mut v2 = v1.clone();
    v2[1_500_000..1_500_200].fill(0x42);
    std::fs::write(&file, &v2).unwrap();
    backup(&client, &root, Some(1)).await.unwrap();
    let (enc, base, sent) = last_put(&sha(&v2));
    assert_eq!(enc.as_deref(), Some("zstd-patch"));
    assert_eq!(base, Some(sha(&v1)));
    assert!(sent * 20 < v2.len(), "{sent} bytes sent");
    assert_eq!(server.lock().unwrap().blobs[&sha(&v2)], v2, "stored whole");

    // The base goes between init and PUT: that copy fails, and the next one
    // sends the file whole instead of offering the same base again.
    let mut v3 = v2.clone();
    v3[100..300].fill(0x17);
    std::fs::write(&file, &v3).unwrap();
    server.lock().unwrap().lost_bases = true;
    let err = backup(&client, &root, Some(2)).await.unwrap_err();
    assert!(format!("{err:#}").contains("base"), "{err:#}");
    server.lock().unwrap().lost_bases = false;
    backup(&client, &root, Some(2)).await.unwrap();
    let (enc, base, _) = last_put(&sha(&v3));
    assert_eq!((enc.as_deref(), base), (Some("zstd"), None));

    // And what went whole is the next base.
    let mut v4 = v3.clone();
    v4[2_000_000..2_000_050].fill(0x99);
    std::fs::write(&file, &v4).unwrap();
    backup(&client, &root, Some(3)).await.unwrap();
    assert_eq!(last_put(&sha(&v4)).1, Some(sha(&v3)));
}

#[tokio::test]
async fn a_zip_save_goes_up_inflated_and_comes_back() {
    let _env = ENV.lock().await;
    let home = tempfile::tempdir().unwrap();
    isolate(home.path());

    let (base, cloud) = spawn_cloud();
    let client = ApiClient::new(&base, "token").unwrap();
    let root = home.path().join("save");
    std::fs::create_dir_all(&root).unwrap();
    let file = root.join("autosave.sav");
    let text = gamestate(3);
    let v1 = zip_save(&text);
    assert!(v1.len() > 1 << 20, "{} bytes", v1.len());
    std::fs::write(&file, &v1).unwrap();
    backup(&client, &root, None).await.unwrap();
    assert_eq!(
        cloud.lock().unwrap().blobs[&sha(&v1)].0,
        None,
        "a zip goes up raw"
    );

    let mut changed = text.clone();
    changed[2_000_000..2_000_300].fill(b'#');
    let v2 = zip_save(&changed);
    std::fs::write(&file, &v2).unwrap();
    backup(&client, &root, Some(1)).await.unwrap();
    {
        let c = cloud.lock().unwrap();
        assert_eq!(
            c.blobs[&sha(&v2)],
            (Some("zstd-patch-inflated".to_string()), Some(sha(&v1)))
        );
        let key = format!("{}.zpi.{}", sha(&v2), sha(&v1));
        let (_, sent) = c.puts.iter().find(|(s, _)| *s == key).unwrap();
        assert!(*sent * 20 < v2.len(), "{sent} bytes sent for {}", v2.len());
    }

    for (name, keep_cache) in [("here", true), ("there", false)] {
        if !keep_cache {
            std::fs::remove_dir_all(home.path().join("cache")).unwrap();
        }
        let dest = home.path().join(name);
        std::fs::create_dir_all(&dest).unwrap();
        download_snapshot(&client, SAVE, 2, &dest, options(home.path()), |_, _| {})
            .await
            .unwrap();
        assert_eq!(
            std::fs::read(dest.join("autosave.sav")).unwrap(),
            v2,
            "{name}"
        );
    }

    // The restore that fetched the base kept it: this machine patches next.
    let mut again = changed.clone();
    again[4_000_000..4_000_100].fill(b'%');
    let v3 = zip_save(&again);
    let there = home.path().join("there");
    std::fs::write(there.join("autosave.sav"), &v3).unwrap();
    backup(&client, &there, Some(2)).await.unwrap();
    assert_eq!(cloud.lock().unwrap().blobs[&sha(&v3)].1, Some(sha(&v1)));
}

#[tokio::test]
async fn self_hosted_rebuilds_an_inflated_patch() {
    let _env = ENV.lock().await;
    let home = tempfile::tempdir().unwrap();
    isolate(home.path());

    let (base, server) = spawn(|_| SelfHosted::default(), handle_self_hosted);
    let client = ApiClient::new(&base, "token").unwrap();
    let root = home.path().join("save");
    std::fs::create_dir_all(&root).unwrap();
    let file = root.join("autosave.sav");
    let text = gamestate(5);
    let v1 = zip_save(&text);
    std::fs::write(&file, &v1).unwrap();
    backup(&client, &root, None).await.unwrap();

    let mut changed = text.clone();
    changed[1_000_000..1_000_400].fill(b'@');
    let v2 = zip_save(&changed);
    std::fs::write(&file, &v2).unwrap();
    backup(&client, &root, Some(1)).await.unwrap();
    let s = server.lock().unwrap();
    let (_, enc, base, sent) = s.puts.iter().find(|p| p.0 == sha(&v2)).unwrap();
    assert_eq!(enc.as_deref(), Some("zstd-patch-inflated"));
    assert_eq!(base.as_deref(), Some(sha(&v1).as_str()));
    assert!(sent * 20 < v2.len(), "{sent} bytes sent");
    assert_eq!(s.blobs[&sha(&v2)], v2);
}
