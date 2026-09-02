//! Blob/chunk storage backend abstraction (ADR 0020).
//!
//! Self-hosted `hoard-server` keeps its content-addressed bytes either on local
//! disk (the default, unchanged) or on any S3-compatible endpoint, selected by
//! `[storage] backend`. Everything else, the SQLite index, auth, dedup and
//! refcounts, retention, the client API, is identical between the two: the
//! bucket only ever holds opaque zstd blob/chunk bytes, addressed by the same
//! per-user, sha-sharded key scheme the on-disk layout uses.
//!
//! The one key scheme (`blobs/<user>/<ab>/<sha>` and `chunks/<user>/<ab>/<sha>`)
//! mirrors `blobs::blob_path` / `chunking::chunk_path`, so `LocalFs` maps a key
//! straight onto `data_dir/<key>`, byte-identical to the pre-abstraction layout,
//! and `S3Store` uses it verbatim as the object key (optionally under
//! a `key_prefix`).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;

/// Object key for a whole-file blob. First two hex chars of the sha shard the
/// keyspace so one user's blobs spread across 256 folders/prefixes.
pub fn blob_key(user_id: &str, sha256: &str) -> String {
    let shard = if sha256.len() >= 2 {
        &sha256[..2]
    } else {
        "00"
    };
    format!("blobs/{user_id}/{shard}/{sha256}")
}

/// Object key for a content-defined chunk (ADR 0019). Same sharding as blobs
/// under a distinct `chunks/` prefix.
pub fn chunk_key(user_id: &str, sha256: &str) -> String {
    let shard = if sha256.len() >= 2 {
        &sha256[..2]
    } else {
        "00"
    };
    format!("chunks/{user_id}/{shard}/{sha256}")
}

/// A local filesystem path from which an object's bytes can be read directly.
/// For the local backend this is the object's real path (`cleanup = false`,
/// zero-copy). For a remote backend the object has been streamed into a spool
/// file the caller must delete once done (`cleanup = true`).
pub struct LocalRef {
    pub path: PathBuf,
    pub cleanup: bool,
}

/// A place to store and retrieve content-addressed blob/chunk bytes. Keys come
/// from [`blob_key`] / [`chunk_key`]; the store is agnostic to which is which.
#[async_trait]
pub trait BlobStore: Send + Sync {
    /// Finalize an upload: move the staged file at `local_path` into the store
    /// under `key`. Consumes the staged file. The local backend renames it
    /// (same filesystem, since `tmp/` and the blob store share `data_dir`) with a
    /// copy fallback on EXDEV; the S3 backend streams it to the bucket.
    async fn put_from_file(&self, key: &str, local_path: &Path) -> Result<()>;

    /// Whether an object exists. Used as a HEAD fallback only; the snapshot
    /// path prefers the `blobs`/`chunks` tables as the source of truth.
    async fn exists(&self, key: &str) -> Result<bool>;

    /// Object size in bytes, or `None` if absent. A cheap stat/HEAD (no
    /// download). Migration uses it to skip keys the destination already has
    /// at the right size.
    async fn size(&self, key: &str) -> Result<Option<i64>>;

    /// Remove an object. A missing object is not an error (GC and rollback both
    /// tolerate a double-delete).
    async fn delete(&self, key: &str) -> Result<()>;

    /// Resolve `key` to a readable local path, spooling remote bytes into
    /// `spool_dir` if the backend isn't local. Never buffers the whole object
    /// in RAM. The caller deletes the returned path iff `cleanup` is set.
    async fn local_ref(&self, key: &str, spool_dir: &Path) -> Result<LocalRef>;

    /// The directory keys resolve under, for the one backend that has one.
    /// `None` everywhere else: a remote bucket has no directories, only keys
    /// that happen to contain slashes. Only for tidying up empty directories
    /// after a purge; never for reading or writing an object, which always
    /// goes through the methods above.
    fn local_root(&self) -> Option<&Path> {
        None
    }
}

// ─── Local filesystem backend ───────────────────────────────────────────────

/// Stores objects under `data_dir/<key>`, byte-identical to the historical
/// on-disk layout. This is the default and the only backend when built with
/// `--no-default-features`.
pub struct LocalFs {
    data_dir: PathBuf,
}

impl LocalFs {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    fn resolve(&self, key: &str) -> PathBuf {
        self.data_dir.join(key)
    }
}

#[async_trait]
impl BlobStore for LocalFs {
    async fn put_from_file(&self, key: &str, local_path: &Path) -> Result<()> {
        let dst = self.resolve(key);
        if let Some(parent) = dst.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("mkdir {}", parent.display()))?;
        }
        // Same filesystem (tmp + blobs share data_dir) → rename; fall back to
        // copy on the off chance of EXDEV, then drop the staged source.
        match tokio::fs::rename(local_path, &dst).await {
            Ok(()) => Ok(()),
            Err(_) => {
                tokio::fs::copy(local_path, &dst)
                    .await
                    .with_context(|| format!("copy blob into {}", dst.display()))?;
                let _ = tokio::fs::remove_file(local_path).await;
                Ok(())
            }
        }
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        Ok(self.resolve(key).exists())
    }

    async fn size(&self, key: &str) -> Result<Option<i64>> {
        match tokio::fs::metadata(self.resolve(key)).await {
            Ok(m) => Ok(Some(m.len() as i64)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e).with_context(|| format!("stat {key}")),
        }
    }

    async fn delete(&self, key: &str) -> Result<()> {
        match tokio::fs::remove_file(self.resolve(key)).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e).with_context(|| format!("delete {key}")),
        }
    }

    async fn local_ref(&self, key: &str, _spool_dir: &Path) -> Result<LocalRef> {
        Ok(LocalRef {
            path: self.resolve(key),
            cleanup: false,
        })
    }

    fn local_root(&self) -> Option<&Path> {
        Some(&self.data_dir)
    }
}

// ─── S3-compatible backend ──────────────────────────────────────────────────

#[cfg(feature = "s3-backend")]
pub struct S3Store {
    inner: crate::s3::S3,
    /// Optional prefix prepended to every key (lets one bucket host several
    /// deployments). Empty by default.
    prefix: String,
}

#[cfg(feature = "s3-backend")]
impl S3Store {
    pub async fn connect(cfg: &crate::config::S3StorageConfig) -> Result<Self> {
        let inner = crate::s3::S3::connect(crate::s3::S3Params {
            endpoint: cfg.endpoint.clone(),
            bucket: cfg.bucket.clone(),
            region: cfg.region.clone(),
            access_key_id: cfg.access_key_id.clone(),
            secret_access_key: cfg.secret_access_key.clone(),
            force_path_style: cfg.force_path_style,
            // A self-hoster's endpoint can be anything from MinIO to an rclone
            // bridge in front of a consumer drive, so speak the plainest S3
            // possible here (see `S3::connect`).
            compat: true,
        })
        .await?;
        Ok(Self {
            inner,
            prefix: cfg.key_prefix.trim_matches('/').to_string(),
        })
    }

    fn full_key(&self, key: &str) -> String {
        if self.prefix.is_empty() {
            key.to_string()
        } else {
            format!("{}/{}", self.prefix, key)
        }
    }

    /// Fail-fast check, run once at startup: store a probe object *the same way
    /// a blob is stored* (streaming PUT from a file on disk), read it back and
    /// compare the bytes, then delete it.
    ///
    /// The round-trip is the point. A write-only probe passes on an endpoint
    /// that mangles streaming bodies. The historical failure here was
    /// `aws-chunked` framing being stored verbatim by `rclone serve s3`, which
    /// corrupts every blob while the server logs nothing. Comparing bytes turns
    /// that into a refusal to boot. Reading also proves the credentials can GET,
    /// not just PUT, which restore needs.
    pub async fn probe(&self) -> Result<()> {
        let key = self.full_key(".hoard_write_probe");
        // Big enough to cross the SDK's in-memory/streaming body split and to
        // survive nothing but an exact round-trip; small enough to be free.
        let payload: Vec<u8> = (0..64 * 1024u32).map(|i| (i % 251) as u8).collect();
        let staged = std::env::temp_dir().join(format!("hoard-s3-probe-{}", uuid::Uuid::new_v4()));
        tokio::fs::write(&staged, &payload)
            .await
            .with_context(|| format!("probe: staging {}", staged.display()))?;

        let result = self.probe_roundtrip(&key, &staged, &payload).await;
        let _ = tokio::fs::remove_file(&staged).await;
        // Leaving the probe object behind on a failed run is harmless; deleting
        // it on the way out keeps the bucket clean when it worked.
        let _ = self.inner.delete(&key).await;
        result
    }

    async fn probe_roundtrip(&self, key: &str, staged: &Path, expected: &[u8]) -> Result<()> {
        self.inner
            .put_file(key, staged)
            .await
            .context("probe write failed (check endpoint, bucket, credentials)")?;
        let got = self
            .inner
            .get_object(key)
            .await
            .context("probe read-back failed (the credentials can write but not read?)")?;
        if got != expected {
            anyhow::bail!(
                "probe read-back returned different bytes than were written \
                 ({} bytes out, {} back). This endpoint is not storing uploads \
                 verbatim — blobs written to it would be silently corrupt. If it \
                 sits behind a proxy, check that the proxy isn't rewriting request \
                 bodies.",
                expected.len(),
                got.len()
            );
        }
        Ok(())
    }
}

#[cfg(feature = "s3-backend")]
#[async_trait]
impl BlobStore for S3Store {
    async fn put_from_file(&self, key: &str, local_path: &Path) -> Result<()> {
        self.inner.put_file(&self.full_key(key), local_path).await?;
        // The staged source is the caller's tmp file; remove it so the S3 path
        // matches the local path's move-semantics (staging is cleaned anyway).
        let _ = tokio::fs::remove_file(local_path).await;
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        Ok(self.inner.head(&self.full_key(key)).await?.is_some())
    }

    async fn size(&self, key: &str) -> Result<Option<i64>> {
        self.inner.head(&self.full_key(key)).await
    }

    async fn delete(&self, key: &str) -> Result<()> {
        self.inner.delete(&self.full_key(key)).await
    }

    async fn local_ref(&self, key: &str, spool_dir: &Path) -> Result<LocalRef> {
        let dest = spool_dir.join(uuid::Uuid::new_v4().to_string());
        self.inner.get_to_file(&self.full_key(key), &dest).await?;
        Ok(LocalRef {
            path: dest,
            cleanup: true,
        })
    }
}

// ─── Factory ────────────────────────────────────────────────────────────────

/// Build a specific storage backend from config, independent of which one is
/// active. `hoard-admin storage migrate` uses this to hold source and
/// destination stores at once. Async because the S3 backend probes the bucket
/// (write+delete) to fail fast on a bad endpoint; local construction is
/// infallible.
pub async fn build_backend(
    cfg: &crate::config::Config,
    backend: crate::config::StorageBackend,
) -> Result<Arc<dyn BlobStore>> {
    use crate::config::StorageBackend;
    match backend {
        StorageBackend::Local => Ok(Arc::new(LocalFs::new(cfg.storage.data_dir.clone()))),
        StorageBackend::S3 => {
            #[cfg(feature = "s3-backend")]
            {
                let s3cfg = cfg.storage.s3.as_ref().ok_or_else(|| {
                    anyhow::anyhow!("the s3 backend requires an [storage.s3] section in config")
                })?;
                let store = S3Store::connect(s3cfg)
                    .await
                    .context("connecting to S3-compatible endpoint")?;
                store.probe().await.context(
                    "S3 bucket is not reachable/writable (check endpoint, bucket, credentials)",
                )?;
                Ok(Arc::new(store))
            }
            #[cfg(not(feature = "s3-backend"))]
            {
                anyhow::bail!("the s3 backend requires building with --features s3-backend")
            }
        }
    }
}

/// Build the *active* backend (`cfg.storage.backend`). Returns a trait object
/// so the rest of the server is backend-agnostic.
pub async fn build_store(cfg: &crate::config::Config) -> Result<Arc<dyn BlobStore>> {
    build_backend(cfg, cfg.storage.backend).await
}

/// Delete every stored object belonging to one user, driven off the index
/// rather than off the filesystem.
///
/// Deleting the `users` row cascades the `blobs`/`chunks` rows away, so this
/// has to run *first*: afterwards there is nothing left to say which keys
/// were theirs. Going through [`BlobStore`] instead of `remove_dir_all` is
/// what makes it work on an S3 bucket too, where there are no directories to
/// remove and the objects would otherwise stay (and keep costing) forever.
///
/// It replaces `data_dir/<user_id>`, which is where user data lived before the
/// content-addressed store landed. Nothing has been written there since, so
/// `hoard-admin user delete` was reporting that it had removed a user's data
/// while leaving every byte of it on disk.
///
/// Returns how many objects went and how many bytes they held. A key that is
/// already gone is not an error: the point is that it is not there afterwards.
pub async fn purge_user_objects(
    pool: &sqlx::SqlitePool,
    store: &Arc<dyn BlobStore>,
    user_id: &str,
) -> Result<(u64, i64)> {
    use sqlx::Row;

    let mut keys: Vec<(String, i64)> = Vec::new();
    for (table, is_blob) in [("blobs", true), ("chunks", false)] {
        let rows = sqlx::query(&format!(
            "SELECT sha256, size_bytes FROM {table} WHERE user_id = ?"
        ))
        .bind(user_id)
        .fetch_all(pool)
        .await?;
        keys.extend(rows.iter().map(|r| {
            let sha: String = r.get("sha256");
            let key = if is_blob {
                blob_key(user_id, &sha)
            } else {
                chunk_key(user_id, &sha)
            };
            (key, r.get::<i64, _>("size_bytes"))
        }));
    }

    let mut removed = 0u64;
    let mut bytes = 0i64;
    for (key, size) in &keys {
        match store.delete(key).await {
            Ok(()) => {
                removed += 1;
                bytes += size;
            }
            // One unreadable object must not strand the rest: the row is about
            // to disappear either way, so a failure here only means an orphan
            // left behind, which the operator can see in the admin overview.
            Err(e) => tracing::warn!(key, error = %e, "purge: could not delete object"),
        }
    }

    // The empty per-user shard directories the local backend leaves behind.
    // Harmless, but they make `du` and a file browser lie about who still has
    // an account on the box.
    if let Some(local) = store.local_root() {
        for prefix in ["blobs", "chunks"] {
            let dir = local.join(prefix).join(user_id);
            if dir.exists() {
                if let Err(e) = tokio::fs::remove_dir_all(&dir).await {
                    tracing::warn!(dir = %dir.display(), error = %e, "purge: could not remove directory");
                }
            }
        }
    }

    Ok((removed, bytes))
}

/// Startup guard (self-host): data on disk but an **empty database**.
///
/// The inverse of [`sanity_check`], and the one that really bites. If the
/// `data_dir` holds content from an earlier deployment but the database has not a
/// single user, the database has been lost: almost always a `docker compose` with
/// no persistent volume for `/var/lib/hoard`, or a `down -v`. The server used to
/// boot away happily, with migrations "applied" over a freshly created database,
/// and the operator found out when their client could not log in, with the
/// snapshots sitting there intact. Reported in aug-2026: a self-hoster updated,
/// lost users and clients, and had to recreate all of it by hand.
///
/// Booting like that is not recoverable from the client: the `save_id`s each
/// machine holds no longer exist here, and its uploads answer 404 forever.
/// Mejor negarse a servir y decirlo.
pub async fn guard_against_lost_database(
    pool: &sqlx::SqlitePool,
    data_dir: &std::path::Path,
) -> Result<()> {
    let users: i64 = sqlx::query_scalar("SELECT count(*) FROM users")
        .fetch_one(pool)
        .await?;
    if users > 0 {
        return Ok(());
    }
    // No users: is there any trace of an earlier deployment on disk? `blobs/` and
    // `chunks/` belong to the local store; `snapshots/` is the legacy layout.
    let leftovers = ["blobs", "chunks", "snapshots"]
        .iter()
        .filter(|d| std::fs::read_dir(data_dir.join(d)).is_ok_and(|mut r| r.next().is_some()))
        .count();
    if leftovers == 0 {
        // A genuinely fresh install: nothing to protect.
        return Ok(());
    }
    anyhow::bail!(
        "refusing to start: the database has no users, but {} holds data from a previous \
         deployment. The database was lost, not the storage — almost always a container \
         recreated without a persistent volume for the data directory (or a `docker compose \
         down -v`). Starting anyway would look healthy while every client gets 404s for save \
         ids this database has never seen. Restore the database file, or move the leftover \
         data aside if you really mean to start fresh.",
        data_dir.display()
    )
}

/// Startup guard (self-host): if the DB references content-addressed objects
/// but the active store holds none of a random sample, the operator almost
/// certainly flipped `[storage] backend` without running the migration. Fail
/// loudly at boot rather than serving a 500 on the first restore. A fresh DB
/// with nothing stored yet passes (no sample to check).
pub async fn sanity_check(pool: &sqlx::SqlitePool, store: &Arc<dyn BlobStore>) -> Result<()> {
    use sqlx::Row;

    // Sample live keys, blobs first, then chunks if the user only has chunked
    // saves. `refcount > 0` = still referenced (live or trashed).
    let mut keys: Vec<String> = sqlx::query(
        "SELECT user_id, sha256 FROM blobs WHERE refcount > 0 ORDER BY RANDOM() LIMIT 8",
    )
    .fetch_all(pool)
    .await?
    .iter()
    .map(|r| {
        blob_key(
            &r.get::<String, _>("user_id"),
            &r.get::<String, _>("sha256"),
        )
    })
    .collect();

    if keys.is_empty() {
        keys = sqlx::query(
            "SELECT user_id, sha256 FROM chunks WHERE refcount > 0 ORDER BY RANDOM() LIMIT 8",
        )
        .fetch_all(pool)
        .await?
        .iter()
        .map(|r| {
            chunk_key(
                &r.get::<String, _>("user_id"),
                &r.get::<String, _>("sha256"),
            )
        })
        .collect();
    }

    if keys.is_empty() {
        return Ok(());
    }

    for k in &keys {
        if store.exists(k).await.unwrap_or(false) {
            return Ok(());
        }
    }

    anyhow::bail!(
        "storage backend has no data for this database — did you run \
         `hoard-admin storage migrate`?"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn mem_pool() -> sqlx::SqlitePool {
        use sqlx::sqlite::SqliteConnectOptions;
        use std::str::FromStr;
        let opts = SqliteConnectOptions::from_str("sqlite::memory:")
            .unwrap()
            .pragma("foreign_keys", "ON");
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    /// The case that took out a self-hoster's deployment: a freshly created
    /// database (zero users) next to a `data_dir` full of the previous one.
    /// Booting like that looks healthy and leaves every client with permanent
    /// 404s, so the boot has to refuse.
    #[tokio::test]
    async fn refuses_to_boot_with_an_empty_database_next_to_leftover_data() {
        let pool = mem_pool().await;
        let tmp = tempfile::tempdir().unwrap();

        // A genuinely fresh install: no users and no data. It has to pass,
        // since a false positive here would break EVERY first start.
        guard_against_lost_database(&pool, tmp.path())
            .await
            .expect("a fresh install has to start");

        // Now with leftover blobs and no users: the database was lost.
        std::fs::create_dir_all(tmp.path().join("blobs/user-1/ab")).unwrap();
        std::fs::write(tmp.path().join("blobs/user-1/ab/abcd"), b"x").unwrap();
        let err = guard_against_lost_database(&pool, tmp.path())
            .await
            .expect_err("an empty database with data on disk has to abort");
        let msg = err.to_string();
        assert!(
            msg.contains("no users") && msg.contains("previous deployment"),
            "el mensaje debe nombrar el problema: {msg}"
        );
    }

    #[test]
    fn key_scheme_matches_disk_layout() {
        let uid = "user-1";
        let sha = "abcd".to_string() + &"0".repeat(60);
        // The local path is exactly data_dir joined with the key, so the S3
        // key and the on-disk layout can never drift.
        let data_dir = Path::new("/var/lib/hoard");
        assert_eq!(
            LocalFs::new(data_dir.to_path_buf()).resolve(&blob_key(uid, &sha)),
            crate::blobs::blob_path(data_dir, uid, &sha)
        );
        assert_eq!(
            LocalFs::new(data_dir.to_path_buf()).resolve(&chunk_key(uid, &sha)),
            crate::chunking::chunk_path(data_dir, uid, &sha)
        );
    }

    /// LocalFs round-trips a staged file: put moves it in, local_ref points at
    /// the real path (no copy), delete removes it, a missing delete is a no-op.
    #[tokio::test]
    async fn local_fs_put_ref_delete() {
        let root = std::env::temp_dir().join(format!("hoard-store-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&root).await.unwrap();
        let store = LocalFs::new(root.clone());

        let staged = root.join("staged.bin");
        tokio::fs::write(&staged, b"hello").await.unwrap();
        let key = blob_key("u1", &("aa".to_string() + &"0".repeat(62)));

        store.put_from_file(&key, &staged).await.unwrap();
        assert!(!staged.exists(), "staged file was moved, not copied");
        assert!(store.exists(&key).await.unwrap());

        let r = store.local_ref(&key, &root).await.unwrap();
        assert!(!r.cleanup, "local backend never spools");
        assert_eq!(tokio::fs::read(&r.path).await.unwrap(), b"hello");

        store.delete(&key).await.unwrap();
        assert!(!store.exists(&key).await.unwrap());
        // Double-delete is tolerated.
        store.delete(&key).await.unwrap();

        let _ = std::fs::remove_dir_all(&root);
    }
}
