//! Cloudflare R2 client (S3-compatible).
//!
//! R2 speaks the S3 API with a custom endpoint and inline access-key creds.
//! The object plumbing (put/get/head/delete + streaming) lives in the shared
//! [`crate::s3`] module; this wrapper adds only the R2-specific extras:
//! presigned URLs and the cloud key builders.
//!
//! Why presigned URLs: snapshots can be 50+ MB. Funneling those bytes
//! through Fly.io's machines wastes egress (cheap with R2 but not free for
//! us) and saturates a small node's network. The client uploads directly to
//! R2 via a short-lived presigned PUT URL, then calls a small `commit`
//! endpoint that records the version.

use anyhow::{Context, Result};
use aws_sdk_s3::presigning::PresigningConfig;
use std::collections::HashMap;
use std::time::Duration;

use crate::s3::{S3Params, S3};

pub struct R2Store {
    inner: S3,
    default_presign_ttl: Duration,
}

impl R2Store {
    /// Build a client from a `R2Config`. The endpoint URL is what makes this
    /// R2, not S3: without it the SDK would try to hit Amazon.
    pub async fn from_config(cfg: &crate::config::R2Config) -> Result<Self> {
        if cfg.endpoint.is_empty() || cfg.bucket.is_empty() {
            anyhow::bail!("cloud.r2.endpoint and cloud.r2.bucket are required");
        }
        let inner = S3::connect(S3Params {
            endpoint: cfg.endpoint.clone(),
            bucket: cfg.bucket.clone(),
            region: cfg.region.clone(),
            access_key_id: cfg.access_key_id.clone(),
            secret_access_key: cfg.secret_access_key.clone(),
            // R2 requires path-style addressing.
            force_path_style: true,
            // R2 implements the full modern S3 dialect (aws-chunked bodies,
            // flexible checksums), so this one client keeps the SDK defaults.
            compat: false,
        })
        .await?;
        Ok(Self {
            inner,
            default_presign_ttl: Duration::from_secs(cfg.presign_ttl_secs.max(60)),
        })
    }

    /// Direct PUT, useful for small objects (export ZIPs, manifests) the
    /// server constructs itself. For user-uploaded snapshots, prefer
    /// `presign_put`.
    pub async fn put_object(&self, key: &str, body: Vec<u8>) -> Result<()> {
        self.inner.put_object(key, body).await
    }

    /// Streaming PUT from a file on disk. Used for account-export ZIPs, which
    /// can be hundreds of MB, and streaming keeps the archive off the heap
    /// (unlike `put_object`, which buffers the whole body in a `Vec<u8>`).
    pub async fn put_file(&self, key: &str, path: &std::path::Path) -> Result<()> {
        self.inner.put_file(key, path).await
    }

    pub async fn get_object(&self, key: &str) -> Result<Vec<u8>> {
        self.inner.get_object(key).await
    }

    /// Bounded-memory async reader over the object body. For streaming a
    /// blob through the decompressing download proxy or the compression
    /// sweep without ever holding it whole in RAM.
    pub async fn get_reader(&self, key: &str) -> Result<impl tokio::io::AsyncBufRead + Unpin> {
        self.inner.get_reader(key).await
    }

    /// Streaming multipart PUT from a reader (atomic: the old object keeps
    /// serving until the multipart completes). Returns bytes written.
    pub async fn put_from_reader<R>(&self, key: &str, reader: R) -> Result<i64>
    where
        R: tokio::io::AsyncRead + Unpin,
    {
        self.inner.put_from_reader(key, reader).await
    }

    pub async fn delete_object(&self, key: &str) -> Result<()> {
        self.inner.delete(key).await
    }

    /// Returns `Some(size)` if the object exists, `None` if it doesn't.
    pub async fn head(&self, key: &str) -> Result<Option<i64>> {
        self.inner.head(key).await
    }

    /// Sizes of every blob this user has in the bucket, keyed by sha256.
    ///
    /// Keys are `blobs/<user>/<shard>/<sha>`, so the last path segment is the
    /// hash and one listing of the user's prefix answers a whole manifest.
    pub async fn blob_sizes(&self, user_id: uuid::Uuid) -> Result<HashMap<String, i64>> {
        let by_key = self.inner.list_sizes(&format!("blobs/{user_id}/")).await?;
        Ok(by_key
            .into_iter()
            .filter_map(|(k, size)| {
                let sha = k.rsplit('/').next()?;
                is_valid_sha256(sha).then(|| (sha.to_string(), size))
            })
            .collect())
    }

    pub async fn presign_put(&self, key: &str, ttl: Option<Duration>) -> Result<PresignedUrl> {
        let cfg = PresigningConfig::expires_in(ttl.unwrap_or(self.default_presign_ttl))?;
        let req = self
            .inner
            .client()
            .put_object()
            .bucket(self.inner.bucket())
            .key(key)
            .presigned(cfg)
            .await
            .with_context(|| format!("r2 presign_put {key}"))?;
        Ok(PresignedUrl {
            method: "PUT".to_string(),
            url: req.uri().to_string(),
            expires_in_secs: ttl.unwrap_or(self.default_presign_ttl).as_secs(),
        })
    }

    pub async fn presign_get(&self, key: &str, ttl: Option<Duration>) -> Result<PresignedUrl> {
        let cfg = PresigningConfig::expires_in(ttl.unwrap_or(self.default_presign_ttl))?;
        let req = self
            .inner
            .client()
            .get_object()
            .bucket(self.inner.bucket())
            .key(key)
            .presigned(cfg)
            .await
            .with_context(|| format!("r2 presign_get {key}"))?;
        Ok(PresignedUrl {
            method: "GET".to_string(),
            url: req.uri().to_string(),
            expires_in_secs: ttl.unwrap_or(self.default_presign_ttl).as_secs(),
        })
    }

    pub fn bucket(&self) -> &str {
        self.inner.bucket()
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PresignedUrl {
    pub method: String,
    pub url: String,
    pub expires_in_secs: u64,
}

/// Build a deterministic R2 key for a save version. Includes the user id
/// to avoid collisions and to make ad-hoc per-user listings cheap. The
/// suffix is the canonical `.tar.zst` produced by the agent.
pub fn key_for_snapshot(user_id: uuid::Uuid, save_id: &str, version: u64) -> String {
    format!("users/{user_id}/saves/{save_id}/v{version}.tar.zst")
}

/// Build a key for an export ZIP, with a distinct prefix so the cron sweep can
/// scope its work to that folder.
pub fn key_for_export(user_id: uuid::Uuid, job_id: uuid::Uuid) -> String {
    format!("exports/{user_id}/{job_id}.zip")
}

/// Build a deterministic R2 key for a content-addressed file blob. Keyed by
/// the whole-file SHA-256 under a per-user prefix, sharded by the first byte
/// so a single user's blobs spread across 256 folders. Distinct `blobs/`
/// prefix keeps them apart from the legacy `users/.../v{n}.tar.zst` archives.
pub fn key_for_blob(user_id: uuid::Uuid, sha256: &str) -> String {
    let shard = sha256.get(0..2).unwrap_or("00");
    format!("blobs/{user_id}/{shard}/{sha256}")
}

/// A client-supplied content hash must be exactly 64 lowercase hex chars before
/// it's ever interpolated into an R2 key. R2 treats keys literally (no `..`
/// traversal) and every key is already scoped under the authenticated user's
/// prefix, so this is defense in depth, but it keeps malformed or oversized
/// values out of the keyspace and out of `cloud_blobs`/`save_version_files`.
pub fn is_valid_sha256(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn snapshot_key_format_stable() {
        let u = Uuid::nil();
        let k = key_for_snapshot(u, "save-abc", 42);
        assert_eq!(
            k,
            "users/00000000-0000-0000-0000-000000000000/saves/save-abc/v42.tar.zst"
        );
    }

    /// `cloud_blobs.r2_key` used to store this string per row (migration 0050
    /// dropped it: ~111 bytes duplicating what these two arguments already
    /// say). That makes this function the only record of where a blob lives,
    /// so its format is now load-bearing for the 127k objects already in the
    /// bucket: change it and they all become unreachable. Pinned here on
    /// purpose. If this test fails, the fix is not to update the expectation.
    #[test]
    fn blob_key_format_stable() {
        let u = Uuid::nil();
        let sha = "ae2bcfee97e4b03b91985c5898168b7466dbd0c4da7e9cd6ca3ab8546ab0fd66";
        assert_eq!(
            key_for_blob(u, sha),
            format!("blobs/00000000-0000-0000-0000-000000000000/ae/{sha}")
        );
    }

    /// A sha shorter than its shard can't panic on the slice: the keyspace is
    /// validated upstream (`is_valid_sha256`), but this path used to be fed
    /// straight from a database column and now runs on every download.
    #[test]
    fn blob_key_survives_a_short_sha() {
        assert_eq!(
            key_for_blob(Uuid::nil(), "a"),
            "blobs/00000000-0000-0000-0000-000000000000/00/a"
        );
    }

    #[test]
    fn export_key_format_stable() {
        let u = Uuid::nil();
        let j = Uuid::nil();
        let k = key_for_export(u, j);
        assert_eq!(
            k,
            "exports/00000000-0000-0000-0000-000000000000/00000000-0000-0000-0000-000000000000.zip"
        );
    }

    #[test]
    fn sha256_validation() {
        let good = "a".repeat(64);
        assert!(is_valid_sha256(&good));
        assert!(is_valid_sha256(
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        ));
        assert!(!is_valid_sha256(&"a".repeat(63))); // too short
        assert!(!is_valid_sha256(&"a".repeat(65))); // too long
        assert!(!is_valid_sha256(&"A".repeat(64))); // uppercase rejected
        assert!(!is_valid_sha256(&"g".repeat(64))); // non-hex
        assert!(!is_valid_sha256("../../etc/passwd"));
        assert!(!is_valid_sha256(""));
    }
}
