//! Cloudflare R2 client (S3-compatible).
//!
//! R2 speaks the S3 API with a custom endpoint and inline access-key creds.
//! We use `aws-sdk-s3` and force the endpoint to R2's URL. The bucket lives
//! in `cfg.cloud.r2.bucket`.
//!
//! Why presigned URLs: snapshots can be 50+ MB. Funneling those bytes
//! through Fly.io's machines wastes egress (cheap with R2 but not free for
//! us) and saturates a small node's network. The client uploads directly to
//! R2 via a short-lived presigned PUT URL, then calls a small `commit`
//! endpoint that records the version.

use anyhow::{Context, Result};
use aws_credential_types::Credentials;
use aws_sdk_s3::{
    config::{BehaviorVersion, Region},
    presigning::PresigningConfig,
    Client,
};
use std::time::Duration;

pub struct R2Store {
    client: Client,
    bucket: String,
    default_presign_ttl: Duration,
}

impl R2Store {
    /// Build a client from a `R2Config`. The endpoint URL is what makes this
    /// R2-not-S3 — without it, the SDK would try to hit Amazon.
    pub async fn from_config(cfg: &crate::config::R2Config) -> Result<Self> {
        if cfg.endpoint.is_empty() || cfg.bucket.is_empty() {
            anyhow::bail!("cloud.r2.endpoint and cloud.r2.bucket are required");
        }
        let creds = Credentials::new(
            cfg.access_key_id.clone(),
            cfg.secret_access_key.clone(),
            None,
            None,
            "hoard-r2-static",
        );
        let region = if cfg.region.is_empty() {
            // R2 ignores region for the most part but the SDK requires one.
            "auto".to_string()
        } else {
            cfg.region.clone()
        };
        let sdk_conf = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(region))
            .credentials_provider(creds)
            .endpoint_url(cfg.endpoint.clone())
            .load()
            .await;

        let s3_conf = aws_sdk_s3::config::Builder::from(&sdk_conf)
            .force_path_style(true)
            .build();
        let client = Client::from_conf(s3_conf);

        Ok(Self {
            client,
            bucket: cfg.bucket.clone(),
            default_presign_ttl: Duration::from_secs(cfg.presign_ttl_secs.max(60)),
        })
    }

    /// Direct PUT — useful for small objects (export ZIPs, manifests) the
    /// server constructs itself. For user-uploaded snapshots, prefer
    /// `presign_put`.
    pub async fn put_object(&self, key: &str, body: Vec<u8>) -> Result<()> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(body.into())
            .send()
            .await
            .with_context(|| format!("r2 put_object {key}"))?;
        Ok(())
    }

    pub async fn get_object(&self, key: &str) -> Result<Vec<u8>> {
        let out = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .with_context(|| format!("r2 get_object {key}"))?;
        let bytes = out
            .body
            .collect()
            .await
            .with_context(|| format!("r2 read body {key}"))?
            .into_bytes()
            .to_vec();
        Ok(bytes)
    }

    pub async fn delete_object(&self, key: &str) -> Result<()> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .with_context(|| format!("r2 delete_object {key}"))?;
        Ok(())
    }

    /// Returns `Some(size)` if the object exists, `None` if it doesn't.
    pub async fn head(&self, key: &str) -> Result<Option<i64>> {
        match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
        {
            Ok(out) => Ok(out.content_length()),
            Err(e) => {
                // 404 → not found is the expected miss path. Anything else
                // bubbles up — we'd rather surface a real error than treat
                // a transient outage as "object doesn't exist" and silently
                // re-create something the user already has.
                if e.to_string().contains("NotFound") || e.to_string().contains("status code: 404")
                {
                    Ok(None)
                } else {
                    Err(e).context("r2 head_object")
                }
            }
        }
    }

    pub async fn presign_put(&self, key: &str, ttl: Option<Duration>) -> Result<PresignedUrl> {
        let cfg = PresigningConfig::expires_in(ttl.unwrap_or(self.default_presign_ttl))?;
        let req = self
            .client
            .put_object()
            .bucket(&self.bucket)
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
            .client
            .get_object()
            .bucket(&self.bucket)
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
        &self.bucket
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

/// Build a key for an export ZIP — distinct prefix so the cron sweep can
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
}
