//! Shared S3-compatible client.
//!
//! Generalizes what used to live only in `cloud/r2.rs`: a thin wrapper over
//! `aws-sdk-s3` bound to one bucket, with an explicit endpoint URL and inline
//! access-key credentials, the shape every S3-compatible service speaks: MinIO,
//! Backblaze B2, Cloudflare R2, Garage, Wasabi, `rclone serve s3` and the rest.
//!
//! Compiled whenever `s3-backend` is on (which `cloud` implies), so both the
//! self-hosted S3 blob backend (`store::S3Store`) and the cloud R2 client
//! (`cloud::r2::R2Store`) build on the same object primitives instead of
//! duplicating the SDK plumbing. `cloud/r2.rs` keeps only the R2-specific
//! extras on top (presigned URLs, key builders).

use anyhow::{Context, Result};
use aws_credential_types::Credentials;
use aws_sdk_s3::{
    config::{BehaviorVersion, Region, RequestChecksumCalculation, ResponseChecksumValidation},
    primitives::ByteStream,
    Client,
};
use std::path::Path;
use std::time::Duration;

/// Connection parameters for an S3-compatible endpoint. Deliberately a plain
/// struct (not tied to any config type) so both the self-hosted
/// `[storage.s3]` block and the cloud `[cloud.r2]` block can build one.
#[derive(Clone)]
pub struct S3Params {
    pub endpoint: String,
    pub bucket: String,
    pub region: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    /// MinIO, Garage and `rclone serve s3` need path-style addressing
    /// (`endpoint/bucket/key`); most others accept it too. R2 forces it.
    pub force_path_style: bool,
    /// Talk the most conservative dialect of S3 the SDK can speak, for
    /// endpoints that are not AWS and not R2 (see [`S3::connect`]). On for the
    /// self-hosted `[storage.s3]` backend, off for the cloud R2 client.
    pub compat: bool,
}

/// Whether a failed operation means "the object isn't there". Checks the HTTP
/// status and the S3 error code rather than the message, because every
/// implementation words it differently (and some answer a bare 404 with no
/// body at all).
fn is_not_found<E>(
    err: &aws_sdk_s3::error::SdkError<E, aws_sdk_s3::config::http::HttpResponse>,
) -> bool
where
    aws_sdk_s3::error::SdkError<E, aws_sdk_s3::config::http::HttpResponse>:
        aws_sdk_s3::error::ProvideErrorMetadata,
{
    use aws_sdk_s3::error::ProvideErrorMetadata;
    if err.raw_response().map(|r| r.status().as_u16()) == Some(404) {
        return true;
    }
    matches!(err.code(), Some("NoSuchKey") | Some("NotFound"))
}

/// An `aws-sdk-s3` client bound to a single bucket, plus the small set of
/// object operations Hoard needs. Everything higher-level (presigning, key
/// schemes) lives in the callers.
pub struct S3 {
    client: Client,
    bucket: String,
}

impl S3 {
    /// Build a client. The explicit `endpoint_url` is what makes this speak to
    /// an arbitrary S3-compatible host rather than Amazon.
    ///
    /// `compat` exists because "S3-compatible" is a spectrum. Since the SDK
    /// defaults flipped to `RequestChecksumCalculation::WhenSupported`, every
    /// upload with a *streaming* body (which is every blob we store, since the body
    /// comes off a file, not a `Vec`) is sent as `aws-chunked`: the payload is
    /// wrapped in `<len>;chunk-signature=…` frames with the checksum trailing
    /// after it. AWS, R2 and MinIO unwrap that. An implementation that doesn't,
    /// `rclone serve s3`'s gofakes3 among them, which is the bridge our own guide
    /// points Mega, OneDrive and Drive users at, stores the frames verbatim, so
    /// every blob lands corrupt and only surfaces on restore, months later.
    /// With `compat` on we ask for checksums only where the S3 API *requires*
    /// them (nothing we call does), which drops `aws-chunked` entirely and puts
    /// a plain `Content-Length` body on the wire. No integrity is lost: blobs
    /// are content-addressed by sha256 and verified on the way back
    /// (`hoard-admin storage verify`, and the agent re-hashes every download).
    ///
    /// `compat` also widens the SDK's tight AWS-shaped defaults (3.1 s to
    /// connect, 3 attempts), because the endpoint may be an rclone process
    /// forwarding to a consumer cloud drive.
    pub async fn connect(p: S3Params) -> Result<Self> {
        if p.endpoint.is_empty() || p.bucket.is_empty() {
            anyhow::bail!("s3 endpoint and bucket are required");
        }
        // Without a scheme the SDK fails deep inside the first request with an
        // opaque "invalid URI", so catch the common `127.0.0.1:9000` typo here.
        if !p.endpoint.starts_with("http://") && !p.endpoint.starts_with("https://") {
            anyhow::bail!(
                "s3 endpoint must start with http:// or https:// (got {:?})",
                p.endpoint
            );
        }
        let creds = Credentials::new(
            p.access_key_id,
            p.secret_access_key,
            None,
            None,
            "hoard-s3-static",
        );
        let region = if p.region.is_empty() {
            // Many S3-compatibles ignore region but the SDK requires one.
            "auto".to_string()
        } else {
            p.region
        };
        let mut loader = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(region))
            .credentials_provider(creds)
            .endpoint_url(p.endpoint);
        if p.compat {
            loader = loader
                .timeout_config(
                    aws_config::timeout::TimeoutConfig::builder()
                        .connect_timeout(Duration::from_secs(15))
                        .build(),
                )
                // Retries cover a remote that rate-limits or drops a request.
                // Every operation we issue is idempotent (content-addressed
                // PUT, GET, HEAD, DELETE), so a replay is always safe.
                .retry_config(aws_config::retry::RetryConfig::standard().with_max_attempts(5));
        }
        let sdk_conf = loader.load().await;

        let mut s3_conf =
            aws_sdk_s3::config::Builder::from(&sdk_conf).force_path_style(p.force_path_style);
        if p.compat {
            s3_conf = s3_conf
                .request_checksum_calculation(RequestChecksumCalculation::WhenRequired)
                .response_checksum_validation(ResponseChecksumValidation::WhenRequired);
        }

        Ok(Self {
            client: Client::from_conf(s3_conf.build()),
            bucket: p.bucket,
        })
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    pub fn bucket(&self) -> &str {
        &self.bucket
    }

    /// Direct PUT of an in-memory body, for small objects the server builds
    /// itself (export ZIPs, manifests, the write probe). Buffers the whole
    /// body; use [`Self::put_file`] for anything large.
    pub async fn put_object(&self, key: &str, body: Vec<u8>) -> Result<()> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(body.into())
            .send()
            .await
            .with_context(|| format!("s3 put_object {key}"))?;
        Ok(())
    }

    /// Streaming PUT from a file on disk. Keeps the body off the heap: the SDK
    /// reads the file incrementally. Used for blob/chunk finalization and
    /// account-export ZIPs, which can be GB-sized.
    ///
    /// A single PUT is the most widely supported way to store an object, and
    /// everything Hoard stores fits in one: files above `CHUNK_THRESHOLD`
    /// (128 MB) are split into chunks before they ever reach a backend. Should
    /// that invariant ever change, the 5 GiB single-PUT ceiling would turn into
    /// a confusing provider error, so anything approaching it falls back to
    /// multipart instead.
    pub async fn put_file(&self, key: &str, path: &Path) -> Result<()> {
        const SINGLE_PUT_MAX: u64 = 4 * 1024 * 1024 * 1024;
        if let Ok(meta) = tokio::fs::metadata(path).await {
            if meta.len() > SINGLE_PUT_MAX {
                let file = tokio::fs::File::open(path)
                    .await
                    .with_context(|| format!("s3 put_file open {}", path.display()))?;
                self.put_from_reader(key, file).await?;
                return Ok(());
            }
        }
        let body = ByteStream::from_path(path)
            .await
            .with_context(|| format!("s3 put_file open {}", path.display()))?;
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(body)
            .send()
            .await
            .with_context(|| format!("s3 put_file {key}"))?;
        Ok(())
    }

    /// GET the whole object into memory. Only for objects known to be small
    /// (export ZIPs re-read by the cloud worker). Snapshot downloads must use
    /// [`Self::get_to_file`] so a multi-GB save never lands on the heap.
    pub async fn get_object(&self, key: &str) -> Result<Vec<u8>> {
        let out = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .with_context(|| format!("s3 get_object {key}"))?;
        let bytes = out
            .body
            .collect()
            .await
            .with_context(|| format!("s3 read body {key}"))?
            .into_bytes()
            .to_vec();
        Ok(bytes)
    }

    /// Stream the object to a local file, bounded memory (the SDK body is
    /// piped straight to disk, never fully buffered). This is the primitive
    /// the S3 blob backend uses to spool a remote blob/chunk into `tmp/`
    /// before it's fed into a snapshot download tarball.
    pub async fn get_to_file(&self, key: &str, dest: &Path) -> Result<()> {
        let out = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .with_context(|| format!("s3 get_object {key}"))?;
        let declared = out.content_length();
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("s3 spool mkdir {}", parent.display()))?;
        }
        let mut reader = out.body.into_async_read();
        let mut file = tokio::fs::File::create(dest)
            .await
            .with_context(|| format!("s3 spool create {}", dest.display()))?;
        let copied = tokio::io::copy(&mut reader, &mut file)
            .await
            .with_context(|| format!("s3 spool {key}"))?;
        tokio::io::AsyncWriteExt::flush(&mut file).await.ok();
        // A body cut short mid-stream is not an error for `copy`; it just
        // stops. Silently spooling a truncated blob would put the wrong bytes
        // in a restore tarball, so compare against the length the endpoint
        // declared and let the caller fail (and the client retry) instead.
        if let Some(len) = declared {
            if len >= 0 && copied != len as u64 {
                let _ = tokio::fs::remove_file(dest).await;
                anyhow::bail!("s3 get {key}: truncated body ({copied} of {len} bytes)");
            }
        }
        Ok(())
    }

    /// Open the object as a bounded-memory async reader. The caller streams
    /// it wherever it needs (a decompressor, a response body) without the
    /// object ever landing whole on the heap.
    pub async fn get_reader(&self, key: &str) -> Result<impl tokio::io::AsyncBufRead + Unpin> {
        let out = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .with_context(|| format!("s3 get_object {key}"))?;
        Ok(out.body.into_async_read())
    }

    /// Streaming PUT from an arbitrary reader, bounded memory (one part
    /// buffer at a time). Bodies that fit in one part go out as a single
    /// PUT (one Class A op); larger ones use multipart. Returns the total
    /// bytes written. Either way the upload is atomic: the key serves its
    /// previous content until the PUT/`complete` succeeds, and an error
    /// path aborts the multipart so no half-written object ever becomes
    /// visible.
    pub async fn put_from_reader<R>(&self, key: &str, mut reader: R) -> Result<i64>
    where
        R: tokio::io::AsyncRead + Unpin,
    {
        use aws_sdk_s3::types::{CompletedMultipartUpload, CompletedPart};
        use tokio::io::AsyncReadExt;

        // S3 minimum part size is 5 MiB (except the last part); 8 MiB keeps
        // part counts low without hurting the 512 MB machine.
        const PART_SIZE: usize = 8 * 1024 * 1024;

        // Buffer the first part up front: a body that fits in one part goes
        // out as a single PUT, one Class A op instead of multipart's three
        // (create + part + complete), and most save files are far below the
        // part size. Only genuinely multi-part bodies pay for multipart.
        let mut first = Vec::with_capacity(PART_SIZE);
        while first.len() < PART_SIZE {
            let n = (&mut reader)
                .take((PART_SIZE - first.len()) as u64)
                .read_to_end(&mut first)
                .await
                .context("s3 put_from_reader read")?;
            if n == 0 {
                break;
            }
        }
        if first.len() < PART_SIZE {
            let total = first.len() as i64;
            self.client
                .put_object()
                .bucket(&self.bucket)
                .key(key)
                .body(first.into())
                .send()
                .await
                .with_context(|| format!("s3 put_from_reader single put {key}"))?;
            return Ok(total);
        }

        let mp = self
            .client
            .create_multipart_upload()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .with_context(|| format!("s3 create_multipart {key}"))?;
        let upload_id = mp
            .upload_id()
            .context("s3 create_multipart returned no upload id")?
            .to_string();

        let upload = async {
            let mut parts: Vec<CompletedPart> = Vec::new();
            let mut total: i64 = 0;
            let mut part_number = 1i32;
            // Part 1 is the buffer already read above, a full part; smaller
            // bodies (including empty) took the single-PUT path.
            let mut buf = first;
            loop {
                // An empty part means the reader drained on a part boundary.
                if buf.is_empty() {
                    break;
                }
                let done = buf.len() < PART_SIZE;
                total += buf.len() as i64;
                let out = self
                    .client
                    .upload_part()
                    .bucket(&self.bucket)
                    .key(key)
                    .upload_id(&upload_id)
                    .part_number(part_number)
                    .body(buf.into())
                    .send()
                    .await
                    .with_context(|| format!("s3 upload_part {part_number} {key}"))?;
                parts.push(
                    CompletedPart::builder()
                        .part_number(part_number)
                        .set_e_tag(out.e_tag().map(str::to_string))
                        .build(),
                );
                part_number += 1;
                if done {
                    break;
                }
                buf = Vec::with_capacity(PART_SIZE);
                while buf.len() < PART_SIZE {
                    let n = (&mut reader)
                        .take((PART_SIZE - buf.len()) as u64)
                        .read_to_end(&mut buf)
                        .await
                        .context("s3 multipart read")?;
                    if n == 0 {
                        break;
                    }
                }
            }
            self.client
                .complete_multipart_upload()
                .bucket(&self.bucket)
                .key(key)
                .upload_id(&upload_id)
                .multipart_upload(
                    CompletedMultipartUpload::builder()
                        .set_parts(Some(parts))
                        .build(),
                )
                .send()
                .await
                .with_context(|| format!("s3 complete_multipart {key}"))?;
            Ok(total)
        }
        .await;

        if upload.is_err() {
            // Best-effort: leave no dangling multipart charging storage.
            let _ = self
                .client
                .abort_multipart_upload()
                .bucket(&self.bucket)
                .key(key)
                .upload_id(&upload_id)
                .send()
                .await;
        }
        upload
    }

    /// `Some(size)` if the object exists, `None` on a clean 404.
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
                // 404 is the expected miss. Detect it via the typed service
                // error rather than the Display string: MinIO, R2 and S3 all
                // render the message differently, but `is_not_found()` is
                // uniform. Anything else (auth, outage) must bubble up: treating
                // a transient error as "absent" would let callers re-create
                // objects the user already has.
                if e.as_service_error().map(|se| se.is_not_found()) == Some(true) {
                    Ok(None)
                } else {
                    Err(e).context("s3 head_object")
                }
            }
        }
    }

    /// Every key under `prefix`, mapped to its size.
    ///
    /// One request per 1000 keys instead of one per key. `head` is the right
    /// call for a single object and the wrong one for a whole manifest: a
    /// version with 24,784 blobs is 24,784 round trips even fanned out, and
    /// `cas_commit` has to answer inside the client's 60 s timeout. The same
    /// answer arrives here in ~26.
    ///
    /// Pagination is sequential because each page's continuation token is only
    /// known once the previous page lands. That is the whole cost: ~26 serial
    /// round trips rather than tens of thousands.
    pub async fn list_sizes(&self, prefix: &str) -> Result<std::collections::HashMap<String, i64>> {
        let mut out = std::collections::HashMap::new();
        let mut token: Option<String> = None;
        loop {
            let mut req = self
                .client
                .list_objects_v2()
                .bucket(&self.bucket)
                .prefix(prefix);
            if let Some(t) = token.take() {
                req = req.continuation_token(t);
            }
            let page = req.send().await.context("s3 list_objects_v2")?;
            for obj in page.contents() {
                if let (Some(k), Some(sz)) = (obj.key(), obj.size()) {
                    out.insert(k.to_string(), sz);
                }
            }
            if page.is_truncated() == Some(true) {
                token = page.next_continuation_token().map(|t| t.to_string());
                if token.is_none() {
                    break;
                }
            } else {
                break;
            }
        }
        Ok(out)
    }

    /// Delete an object. Missing is success: S3 itself answers 204 for a key
    /// that was never there, but several compatibles (gofakes3 behind `rclone
    /// serve s3`, some gateways) answer 404 instead. GC and upload rollback
    /// both double-delete by design, so a 404 must not fail the sweep and
    /// strand the rest of the trash.
    pub async fn delete(&self, key: &str) -> Result<()> {
        match self
            .client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
        {
            Ok(_) => Ok(()),
            Err(e) if is_not_found(&e) => Ok(()),
            Err(e) => Err(e).with_context(|| format!("s3 delete_object {key}")),
        }
    }
}
