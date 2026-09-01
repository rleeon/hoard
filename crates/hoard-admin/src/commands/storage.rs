//! `hoard-admin storage`: migrate, verify and status for the blob storage backends
//! (ADR 0020 phase 2).
//!
//! All object access goes through the `BlobStore` trait; this module never
//! touches the filesystem or a bucket directly (beyond staging temp files under
//! `data_dir/tmp/`, the same spool area the server uses).

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use clap::{Subcommand, ValueEnum};
use futures::stream::StreamExt;
use hoard_server::config::{Config, StorageBackend};
use hoard_server::db;
use hoard_server::store::{self, blob_key, chunk_key, BlobStore};
use indicatif::{ProgressBar, ProgressStyle};
use sha2::{Digest, Sha256};
use sqlx::Row;
use tokio::io::AsyncReadExt;
use uuid::Uuid;

#[derive(Subcommand)]
pub enum StorageCommand {
    /// Copy every blob/chunk object to another storage backend.
    ///
    /// STOP THE SERVER FIRST: writes that land during a migration would be
    /// missed. Idempotent and resumable: a re-run after a crash skips objects
    /// already copied and continues. Source data is never touched unless you
    /// pass --delete-source (and even then only after the whole pass verifies).
    /// This does NOT edit config.toml: flip `[storage] backend` yourself when
    /// it finishes.
    Migrate {
        /// Destination backend. The source is the other one.
        #[arg(long = "to", value_enum)]
        to: BackendArg,
        /// After a fully-verified pass, delete each copied object from the
        /// source backend. Skipped objects are hash-verified before deletion.
        #[arg(long)]
        delete_source: bool,
        /// Maximum objects copied in parallel.
        #[arg(long, default_value_t = 8)]
        concurrency: usize,
        /// Proceed even though a server may be running (skips the port check).
        #[arg(long)]
        yes: bool,
    },
    /// Re-download objects and check their bytes hash to their key.
    ///
    /// Detects missing or bit-rotted objects. Run it periodically as a
    /// self-hosted integrity check. Exit status is nonzero if anything is
    /// missing or corrupt.
    Verify {
        /// Only check a random sample of N objects. Default: check every one.
        #[arg(long)]
        sample: Option<usize>,
        /// Check every object (the default; accepted for explicitness).
        #[arg(long, conflicts_with = "sample")]
        all: bool,
    },
    /// Show the active backend, object counts/bytes per user, and reachability.
    Status,
}

/// CLI spelling of a backend.
#[derive(Clone, Copy, ValueEnum)]
pub enum BackendArg {
    Local,
    S3,
}

impl From<BackendArg> for StorageBackend {
    fn from(b: BackendArg) -> Self {
        match b {
            BackendArg::Local => StorageBackend::Local,
            BackendArg::S3 => StorageBackend::S3,
        }
    }
}

fn backend_name(b: StorageBackend) -> &'static str {
    match b {
        StorageBackend::Local => "local",
        StorageBackend::S3 => "s3",
    }
}

pub async fn run(cmd: StorageCommand, cfg: &Config) -> Result<()> {
    match cmd {
        StorageCommand::Migrate {
            to,
            delete_source,
            concurrency,
            yes,
        } => migrate(cfg, to.into(), delete_source, concurrency.max(1), yes).await,
        StorageCommand::Verify { sample, all: _ } => verify(cfg, sample).await,
        StorageCommand::Status => status(cfg).await,
    }
}

// ─── Shared helpers ──────────────────────────────────────────────────────────

/// One content-addressed object to move/check: its storage key, expected
/// sha256 (from the key), and byte size (from the DB row).
#[derive(Debug)]
struct ObjKey {
    key: String,
    sha: String,
    size: i64,
}

/// Enumerate object keys from the DB (`blobs` and `chunks`, refcount > 0), which is
/// the source of truth. Never lists the filesystem or bucket. With `sample`, draws
/// a random subset via SQL `RANDOM()`.
async fn enumerate_keys(pool: &sqlx::SqlitePool, sample: Option<usize>) -> Result<Vec<ObjKey>> {
    let (blob_sql, chunk_sql) = match sample {
        Some(n) => (
            format!("SELECT user_id, sha256, size_bytes FROM blobs WHERE refcount > 0 ORDER BY RANDOM() LIMIT {n}"),
            format!("SELECT user_id, sha256, size_bytes FROM chunks WHERE refcount > 0 ORDER BY RANDOM() LIMIT {n}"),
        ),
        None => (
            "SELECT user_id, sha256, size_bytes FROM blobs WHERE refcount > 0".to_string(),
            "SELECT user_id, sha256, size_bytes FROM chunks WHERE refcount > 0".to_string(),
        ),
    };

    let mut keys = Vec::new();
    for r in sqlx::query(&blob_sql).fetch_all(pool).await? {
        let user: String = r.get("user_id");
        let sha: String = r.get("sha256");
        keys.push(ObjKey {
            key: blob_key(&user, &sha),
            sha,
            size: r.get("size_bytes"),
        });
    }
    for r in sqlx::query(&chunk_sql).fetch_all(pool).await? {
        let user: String = r.get("user_id");
        let sha: String = r.get("sha256");
        keys.push(ObjKey {
            key: chunk_key(&user, &sha),
            sha,
            size: r.get("size_bytes"),
        });
    }
    if let Some(n) = sample {
        keys.truncate(n);
    }
    Ok(keys)
}

async fn hash_file(path: &Path) -> Result<String> {
    let mut f = tokio::fs::File::open(path)
        .await
        .with_context(|| format!("open {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 256 * 1024];
    loop {
        let n = f.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Read `key` back from `store`, hash it, and confirm it matches `expected_sha`.
/// Spools through `tmp_dir` (bounded memory); cleans up any spool file.
async fn verify_key(
    store: &Arc<dyn BlobStore>,
    key: &str,
    expected_sha: &str,
    tmp_dir: &Path,
) -> Result<()> {
    let r = store
        .local_ref(key, tmp_dir)
        .await
        .with_context(|| format!("read object {key}"))?;
    let got = hash_file(&r.path).await;
    if r.cleanup {
        let _ = tokio::fs::remove_file(&r.path).await;
    }
    let got = got?;
    if got != expected_sha {
        anyhow::bail!("hash mismatch (expected {expected_sha}, got {got})");
    }
    Ok(())
}

/// Reachability probe through the trait only: stage a file, put it under a
/// throwaway key, read it back and compare the bytes, then delete it.
///
/// The read-back is deliberate. A put-only probe is green against an endpoint
/// that stores mangled bytes (the `aws-chunked` case that made every blob
/// written through some S3 gateways unreadable), which is exactly the answer an
/// operator running `storage status` needs before trusting it with saves.
async fn reachability(store: &Arc<dyn BlobStore>, data_dir: &Path) -> Result<()> {
    let tmp = data_dir.join("tmp");
    tokio::fs::create_dir_all(&tmp).await.ok();
    let payload: Vec<u8> = (0..64 * 1024u32).map(|i| (i % 251) as u8).collect();
    let expected = hex::encode(Sha256::digest(&payload));
    let stage = tmp.join(format!("probe-{}", Uuid::new_v4()));
    tokio::fs::write(&stage, &payload)
        .await
        .context("stage probe file")?;
    let key = format!("_hoard_probe/{}", Uuid::new_v4());
    store
        .put_from_file(&key, &stage)
        .await
        .context("probe put")?;
    let present = store.exists(&key).await.context("probe head")?;
    let verified = if present {
        verify_key(store, &key, &expected, &tmp).await
    } else {
        Ok(())
    };
    let _ = store.delete(&key).await;
    if !present {
        anyhow::bail!("probe object missing after write");
    }
    verified.context("probe read-back: the endpoint did not return what was written")?;
    Ok(())
}

fn human_bytes(b: i64) -> String {
    const U: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut v = b as f64;
    let mut i = 0;
    while v >= 1024.0 && i < U.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{b} B")
    } else {
        format!("{v:.1} {}", U[i])
    }
}

// ─── migrate ─────────────────────────────────────────────────────────────────

enum MigOutcome {
    Copied(u64),
    Skipped,
}

/// Refuse to migrate while a server looks live: if we can't bind the configured
/// listen address, assume it's in use. Best-effort: other bind errors (permissions,
/// say) don't block.
fn ensure_server_stopped(cfg: &Config) -> Result<()> {
    let addr = format!("{}:{}", cfg.server.host, cfg.server.port);
    match std::net::TcpListener::bind(&addr) {
        Ok(l) => {
            drop(l);
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => anyhow::bail!(
            "a server appears to be running on {addr} (port in use). Stop it \
             before migrating, or writes during a migration would be lost. Pass \
             --yes to override this check."
        ),
        Err(_) => Ok(()),
    }
}

async fn migrate_one(
    src: &Arc<dyn BlobStore>,
    dest: &Arc<dyn BlobStore>,
    obj: &ObjKey,
    tmp_dir: &Path,
    verify_skips: bool,
) -> Result<MigOutcome> {
    // Idempotent skip: destination already has it at the right size. When we're
    // about to delete the source, hash-verify the destination copy first so
    // --delete-source never drops a source object we haven't proven landed.
    if let Some(sz) = dest.size(&obj.key).await? {
        if sz == obj.size {
            if verify_skips {
                verify_key(dest, &obj.key, &obj.sha, tmp_dir).await?;
            }
            return Ok(MigOutcome::Skipped);
        }
        // Size mismatch → a partial/incomplete destination object; re-copy.
    }

    // Copy source bytes into a fresh staged temp we own: never hand the real
    // source path to put_from_file (which consumes/moves it) or we'd destroy
    // the source. Spools through tmp/, bounded memory.
    let r = src
        .local_ref(&obj.key, tmp_dir)
        .await
        .with_context(|| format!("read source object {}", obj.key))?;
    let staged = tmp_dir.join(format!("mig-{}", Uuid::new_v4()));
    let copy_res = tokio::fs::copy(&r.path, &staged).await;
    if r.cleanup {
        let _ = tokio::fs::remove_file(&r.path).await;
    }
    copy_res.with_context(|| format!("stage {}", obj.key))?;

    dest.put_from_file(&obj.key, &staged)
        .await
        .with_context(|| format!("write destination object {}", obj.key))?;

    // Verify the copied bytes before counting it done.
    verify_key(dest, &obj.key, &obj.sha, tmp_dir)
        .await
        .with_context(|| format!("verify copied object {}", obj.key))?;

    Ok(MigOutcome::Copied(obj.size as u64))
}

async fn migrate_one_retry(
    src: &Arc<dyn BlobStore>,
    dest: &Arc<dyn BlobStore>,
    obj: &ObjKey,
    tmp_dir: &Path,
    verify_skips: bool,
) -> Result<MigOutcome> {
    const MAX_ATTEMPTS: u32 = 3;
    let mut attempt = 1;
    loop {
        match migrate_one(src, dest, obj, tmp_dir, verify_skips).await {
            Ok(o) => return Ok(o),
            Err(_) if attempt < MAX_ATTEMPTS => {
                let backoff = std::time::Duration::from_millis(200 * (1 << (attempt - 1)));
                tokio::time::sleep(backoff).await;
                attempt += 1;
            }
            Err(e) => return Err(e),
        }
    }
}

async fn migrate(
    cfg: &Config,
    to: StorageBackend,
    delete_source: bool,
    concurrency: usize,
    yes: bool,
) -> Result<()> {
    let from = match to {
        StorageBackend::Local => StorageBackend::S3,
        StorageBackend::S3 => StorageBackend::Local,
    };

    println!(
        "Migrating storage: {} → {}",
        backend_name(from),
        backend_name(to)
    );
    if !yes {
        ensure_server_stopped(cfg)?;
    } else {
        eprintln!("warning: --yes given, not checking whether a server is running");
    }

    let pool = db::connect(&cfg.database.url, cfg.database.max_connections).await?;

    // Build both stores. build_backend probes the s3 side (fail fast on a bad
    // endpoint). Direction-agnostic from here on.
    let dest = store::build_backend(cfg, to)
        .await
        .context("building destination backend")?;
    let src = store::build_backend(cfg, from)
        .await
        .context("building source backend")?;

    let tmp_dir = cfg.storage.data_dir.join("tmp");
    tokio::fs::create_dir_all(&tmp_dir).await.ok();

    let keys = enumerate_keys(&pool, None).await?;
    let total = keys.len();
    let total_bytes: i64 = keys.iter().map(|k| k.size).sum();
    if total == 0 {
        println!("Nothing to migrate: the database references no stored objects.");
        return Ok(());
    }
    println!(
        "{total} objects, {} to copy (already-present objects are skipped).",
        human_bytes(total_bytes)
    );

    let pb = ProgressBar::new(total as u64);
    pb.set_style(
        ProgressStyle::with_template("{bar:40} {pos}/{len} objects · {msg}")
            .unwrap()
            .progress_chars("=> "),
    );

    let copied = Arc::new(AtomicU64::new(0));
    let skipped = Arc::new(AtomicU64::new(0));
    let copied_bytes = Arc::new(AtomicU64::new(0));
    let verified: Arc<Mutex<Vec<ObjKey>>> = Arc::new(Mutex::new(Vec::new()));
    let failures: Arc<Mutex<Vec<(String, String)>>> = Arc::new(Mutex::new(Vec::new()));

    futures::stream::iter(keys)
        .map(|obj| {
            let (src, dest, tmp_dir, pb) = (src.clone(), dest.clone(), tmp_dir.clone(), pb.clone());
            let (copied, skipped, copied_bytes) =
                (copied.clone(), skipped.clone(), copied_bytes.clone());
            let (verified, failures) = (verified.clone(), failures.clone());
            async move {
                match migrate_one_retry(&src, &dest, &obj, &tmp_dir, delete_source).await {
                    Ok(MigOutcome::Copied(b)) => {
                        copied.fetch_add(1, Ordering::Relaxed);
                        copied_bytes.fetch_add(b, Ordering::Relaxed);
                        verified.lock().unwrap().push(obj);
                    }
                    Ok(MigOutcome::Skipped) => {
                        skipped.fetch_add(1, Ordering::Relaxed);
                        verified.lock().unwrap().push(obj);
                    }
                    Err(e) => {
                        failures
                            .lock()
                            .unwrap()
                            .push((obj.key.clone(), e.to_string()));
                    }
                }
                pb.set_message(format!(
                    "{} copied",
                    human_bytes(copied_bytes.load(Ordering::Relaxed) as i64)
                ));
                pb.inc(1);
            }
        })
        .buffer_unordered(concurrency)
        .collect::<Vec<()>>()
        .await;

    pb.finish_and_clear();

    let failures = Arc::try_unwrap(failures).unwrap().into_inner().unwrap();
    let copied_n = copied.load(Ordering::Relaxed);
    let skipped_n = skipped.load(Ordering::Relaxed);

    if !failures.is_empty() {
        eprintln!(
            "\nMigration INCOMPLETE: {} copied, {} skipped, {} failed.",
            copied_n,
            skipped_n,
            failures.len()
        );
        for (k, e) in failures.iter().take(20) {
            eprintln!("  {k}: {e}");
        }
        if failures.len() > 20 {
            eprintln!("  … and {} more", failures.len() - 20);
        }
        eprintln!("Source data was left untouched. Fix the cause and re-run; it resumes.");
        anyhow::bail!("{} object(s) failed to migrate", failures.len());
    }

    println!(
        "Copied {} new object(s) ({}), {} already present.",
        copied_n,
        human_bytes(copied_bytes.load(Ordering::Relaxed) as i64),
        skipped_n
    );

    // Only now, after a fully-verified pass, is it safe to delete the source.
    if delete_source {
        let verified = Arc::try_unwrap(verified).unwrap().into_inner().unwrap();
        println!(
            "Deleting {} object(s) from the source backend…",
            verified.len()
        );
        let del_pb = ProgressBar::new(verified.len() as u64);
        let del_failures: Arc<Mutex<Vec<(String, String)>>> = Arc::new(Mutex::new(Vec::new()));
        futures::stream::iter(verified)
            .map(|obj| {
                let (src, del_pb, del_failures) =
                    (src.clone(), del_pb.clone(), del_failures.clone());
                async move {
                    if let Err(e) = src.delete(&obj.key).await {
                        del_failures
                            .lock()
                            .unwrap()
                            .push((obj.key.clone(), e.to_string()));
                    }
                    del_pb.inc(1);
                }
            })
            .buffer_unordered(concurrency)
            .collect::<Vec<()>>()
            .await;
        del_pb.finish_and_clear();
        let del_failures = Arc::try_unwrap(del_failures).unwrap().into_inner().unwrap();
        if del_failures.is_empty() {
            println!("Source objects deleted.");
        } else {
            eprintln!(
                "warning: {} source object(s) could not be deleted (they're harmless leftovers):",
                del_failures.len()
            );
            for (k, e) in del_failures.iter().take(10) {
                eprintln!("  {k}: {e}");
            }
        }
    }

    println!("\nNext steps:");
    println!(
        "  1. Set  [storage] backend = \"{}\"  in your config.toml",
        backend_name(to)
    );
    if to == StorageBackend::S3 {
        println!("     (make sure the [storage.s3] block is filled in)");
    }
    println!("  2. Restart hoard-server");
    Ok(())
}

// ─── verify ──────────────────────────────────────────────────────────────────

async fn verify(cfg: &Config, sample: Option<usize>) -> Result<()> {
    let pool = db::connect(&cfg.database.url, cfg.database.max_connections).await?;
    let storeb = store::build_store(cfg)
        .await
        .context("building active storage backend")?;
    let tmp_dir = cfg.storage.data_dir.join("tmp");
    tokio::fs::create_dir_all(&tmp_dir).await.ok();

    let keys = enumerate_keys(&pool, sample).await?;
    let total = keys.len();
    if total == 0 {
        println!("No objects to verify.");
        return Ok(());
    }
    println!(
        "Verifying {total} object(s) on backend '{}'…",
        backend_name(cfg.storage.backend)
    );

    let pb = ProgressBar::new(total as u64);
    pb.set_style(
        ProgressStyle::with_template("{bar:40} {pos}/{len}")
            .unwrap()
            .progress_chars("=> "),
    );

    let missing: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let corrupt: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    futures::stream::iter(keys)
        .map(|obj| {
            let (store, tmp_dir, pb) = (storeb.clone(), tmp_dir.clone(), pb.clone());
            let (missing, corrupt) = (missing.clone(), corrupt.clone());
            async move {
                match store.exists(&obj.key).await {
                    Ok(true) => {
                        if let Err(e) = verify_key(&store, &obj.key, &obj.sha, &tmp_dir).await {
                            corrupt.lock().unwrap().push(format!("{} ({e})", obj.key));
                        }
                    }
                    Ok(false) => missing.lock().unwrap().push(obj.key.clone()),
                    Err(e) => missing.lock().unwrap().push(format!("{} ({e})", obj.key)),
                }
                pb.inc(1);
            }
        })
        .buffer_unordered(8)
        .collect::<Vec<()>>()
        .await;

    pb.finish_and_clear();

    let missing = Arc::try_unwrap(missing).unwrap().into_inner().unwrap();
    let corrupt = Arc::try_unwrap(corrupt).unwrap().into_inner().unwrap();

    let bad = missing.len() + corrupt.len();
    if bad == 0 {
        println!("OK: all {total} object(s) present and hash-verified.");
        return Ok(());
    }

    if !missing.is_empty() {
        eprintln!("\nMISSING ({}):", missing.len());
        for k in missing.iter().take(50) {
            eprintln!("  {k}");
        }
    }
    if !corrupt.is_empty() {
        eprintln!("\nCORRUPT ({}):", corrupt.len());
        for k in corrupt.iter().take(50) {
            eprintln!("  {k}");
        }
    }
    anyhow::bail!("{bad} of {total} object(s) missing or corrupt");
}

// ─── status ──────────────────────────────────────────────────────────────────

async fn status(cfg: &Config) -> Result<()> {
    let pool = db::connect(&cfg.database.url, cfg.database.max_connections).await?;

    println!("Active backend : {}", backend_name(cfg.storage.backend));
    if cfg.storage.backend == StorageBackend::S3 {
        if let Some(s3) = &cfg.storage.s3 {
            println!("  endpoint     : {}", s3.endpoint);
            println!("  bucket       : {}", s3.bucket);
            if !s3.key_prefix.is_empty() {
                println!("  key_prefix   : {}", s3.key_prefix);
            }
        }
    } else {
        println!("  data_dir     : {}", cfg.storage.data_dir.display());
    }

    // Per-user object counts + bytes across both stores (refcount > 0).
    let rows = sqlx::query(
        "SELECT u.username AS username,
            (SELECT COUNT(*) FROM blobs b WHERE b.user_id = u.id AND b.refcount > 0)
          + (SELECT COUNT(*) FROM chunks c WHERE c.user_id = u.id AND c.refcount > 0) AS objs,
            (SELECT COALESCE(SUM(size_bytes),0) FROM blobs b WHERE b.user_id = u.id AND b.refcount > 0)
          + (SELECT COALESCE(SUM(size_bytes),0) FROM chunks c WHERE c.user_id = u.id AND c.refcount > 0) AS bytes
         FROM users u ORDER BY u.username",
    )
    .fetch_all(&pool)
    .await?;

    let mut total_objs: i64 = 0;
    let mut total_bytes: i64 = 0;
    println!("\n{:<24} {:>10} {:>12}", "User", "Objects", "Size");
    for r in &rows {
        let username: String = r.get("username");
        let objs: i64 = r.get("objs");
        let bytes: i64 = r.get("bytes");
        total_objs += objs;
        total_bytes += bytes;
        if objs > 0 {
            println!("{:<24} {:>10} {:>12}", username, objs, human_bytes(bytes));
        }
    }
    println!(
        "{:<24} {:>10} {:>12}",
        "TOTAL",
        total_objs,
        human_bytes(total_bytes)
    );

    // Reachability of the active backend.
    print!("\nReachability   : ");
    match store::build_store(cfg).await {
        Ok(storeb) => match reachability(&storeb, &cfg.storage.data_dir).await {
            Ok(()) => println!("ok (write+read+delete probe passed)"),
            Err(e) => println!("FAILED: {e}"),
        },
        Err(e) => println!("FAILED: could not build backend: {e}"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local(dir: &Path) -> Arc<dyn BlobStore> {
        Arc::new(hoard_server::store::LocalFs::new(dir.to_path_buf()))
    }

    async fn seed(store: &Arc<dyn BlobStore>, tmp: &Path, content: &[u8]) -> ObjKey {
        let sha = hex::encode(Sha256::digest(content));
        let key = blob_key("u1", &sha);
        let staged = tmp.join(format!("seed-{}", Uuid::new_v4()));
        tokio::fs::write(&staged, content).await.unwrap();
        store.put_from_file(&key, &staged).await.unwrap();
        ObjKey {
            key,
            sha,
            size: content.len() as i64,
        }
    }

    /// Full copy + hash-verify, idempotent resume (skip), partial-destination
    /// recopy (size mismatch), source-preserved, then --delete-source.
    #[tokio::test]
    async fn migrate_copy_verify_resume_delete() {
        let root = std::env::temp_dir().join(format!("hoard-mig-{}", Uuid::new_v4()));
        let (src_dir, dst_dir, tmp) = (root.join("src"), root.join("dst"), root.join("tmp"));
        tokio::fs::create_dir_all(&tmp).await.unwrap();
        let src = local(&src_dir);
        let dst = local(&dst_dir);

        let obj = seed(&src, &tmp, b"hello world blob content here").await;

        // Full copy + verify.
        let o = migrate_one(&src, &dst, &obj, &tmp, false).await.unwrap();
        assert!(matches!(o, MigOutcome::Copied(_)));
        assert!(dst.exists(&obj.key).await.unwrap());
        verify_key(&dst, &obj.key, &obj.sha, &tmp).await.unwrap();
        // Source is never touched by a plain migrate.
        assert!(src.exists(&obj.key).await.unwrap());

        // Resume: destination already has it → idempotent skip.
        let o = migrate_one(&src, &dst, &obj, &tmp, false).await.unwrap();
        assert!(matches!(o, MigOutcome::Skipped));

        // Partial destination (truncated, wrong size) → forced recopy.
        let staged = tmp.join("partial");
        tokio::fs::write(&staged, b"short").await.unwrap();
        dst.put_from_file(&obj.key, &staged).await.unwrap();
        let o = migrate_one(&src, &dst, &obj, &tmp, false).await.unwrap();
        assert!(matches!(o, MigOutcome::Copied(_)), "size mismatch recopies");
        verify_key(&dst, &obj.key, &obj.sha, &tmp).await.unwrap();

        // --delete-source removes the (now-verified) source object.
        src.delete(&obj.key).await.unwrap();
        assert!(!src.exists(&obj.key).await.unwrap());
        assert!(dst.exists(&obj.key).await.unwrap());

        let _ = std::fs::remove_dir_all(&root);
    }

    /// A same-size but corrupt destination must NOT be skip-deleted: with
    /// verify_skips the hash check runs and the object errors instead.
    #[tokio::test]
    async fn delete_source_wont_skip_corrupt_dest() {
        let root = std::env::temp_dir().join(format!("hoard-mig-{}", Uuid::new_v4()));
        let (src_dir, dst_dir, tmp) = (root.join("src"), root.join("dst"), root.join("tmp"));
        tokio::fs::create_dir_all(&tmp).await.unwrap();
        let src = local(&src_dir);
        let dst = local(&dst_dir);

        let obj = seed(&src, &tmp, b"AAAAAAAAAAAAAAAA").await; // 16 bytes
                                                               // Destination holds a same-length but different-content object.
        let staged = tmp.join("corrupt");
        tokio::fs::write(&staged, b"BBBBBBBBBBBBBBBB")
            .await
            .unwrap();
        dst.put_from_file(&obj.key, &staged).await.unwrap();

        // verify_skips = true (as --delete-source sets) → hash mismatch → Err.
        assert!(migrate_one(&src, &dst, &obj, &tmp, true).await.is_err());

        let _ = std::fs::remove_dir_all(&root);
    }
}
