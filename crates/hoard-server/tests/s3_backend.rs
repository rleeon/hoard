//! Integration test for the S3-compatible blob backend (ADR 0020).
//!
//! Skipped unless `HOARD_S3_TEST_ENDPOINT` (and the credential vars below) are
//! set, CI without a MinIO/S3 endpoint runs it as a no-op. To exercise it,
//! bring up MinIO and export:
//!
//! ```sh
//! docker run -d -p 9000:9000 -e MINIO_ROOT_USER=minioadmin \
//!   -e MINIO_ROOT_PASSWORD=minioadmin minio/minio server /data
//! # create the bucket (mc alias/mb, or the console), then:
//! export HOARD_S3_TEST_ENDPOINT=http://localhost:9000
//! export HOARD_S3_TEST_BUCKET=hoard-test
//! export HOARD_S3_TEST_KEY_ID=minioadmin
//! export HOARD_S3_TEST_SECRET=minioadmin
//! cargo test -p hoard-server --test s3_backend -- --nocapture
//! ```

#![cfg(feature = "s3-backend")]

use hoard_server::config::S3StorageConfig;
use hoard_server::store::{blob_key, BlobStore, S3Store};
use sha2::{Digest, Sha256};

fn test_config() -> Option<S3StorageConfig> {
    let endpoint = std::env::var("HOARD_S3_TEST_ENDPOINT").ok()?;
    Some(S3StorageConfig {
        endpoint,
        bucket: std::env::var("HOARD_S3_TEST_BUCKET").unwrap_or_else(|_| "hoard-test".into()),
        region: std::env::var("HOARD_S3_TEST_REGION").unwrap_or_default(),
        access_key_id: std::env::var("HOARD_S3_TEST_KEY_ID").unwrap_or_default(),
        secret_access_key: std::env::var("HOARD_S3_TEST_SECRET").unwrap_or_default(),
        key_prefix: format!("itest/{}", uuid::Uuid::new_v4()),
        force_path_style: true,
    })
}

/// Full backend contract against a live endpoint: probe, upload finalization,
/// existence (dedup negotiation), download round-trip with hash verification,
/// and delete (the trash-purge GC primitive).
#[tokio::test]
async fn s3_backend_roundtrip() {
    let Some(cfg) = test_config() else {
        eprintln!("HOARD_S3_TEST_ENDPOINT unset — skipping S3 integration test");
        return;
    };

    let store = S3Store::connect(&cfg)
        .await
        .expect("connect to S3 endpoint");
    store.probe().await.expect("bucket write+delete probe");

    // A staged upload file on local disk, as the snapshot commit path produces.
    let tmp = std::env::temp_dir().join(format!("hoard-s3-it-{}", uuid::Uuid::new_v4()));
    tokio::fs::create_dir_all(&tmp).await.unwrap();
    let payload: Vec<u8> = (0..1_500_000u32).map(|i| (i % 251) as u8).collect();
    let sha = hex::encode(Sha256::digest(&payload));
    let staged = tmp.join("staged.bin");
    tokio::fs::write(&staged, &payload).await.unwrap();

    let key = blob_key("itest-user", &sha);

    // Not there yet → dedup would treat it as new.
    assert!(!store.exists(&key).await.unwrap());

    // Upload finalization consumes the staged file.
    store.put_from_file(&key, &staged).await.unwrap();
    assert!(!staged.exists(), "staged file consumed by put_from_file");
    assert!(store.exists(&key).await.unwrap(), "dedup sees it now");

    // Download round-trip: spool into a local dir and verify the hash matches.
    let spool = tmp.join("spool");
    let r = store.local_ref(&key, &spool).await.unwrap();
    assert!(r.cleanup, "remote backend spools");
    let got = tokio::fs::read(&r.path).await.unwrap();
    assert_eq!(
        hex::encode(Sha256::digest(&got)),
        sha,
        "bytes survive round-trip"
    );

    // Trash-purge GC primitive.
    store.delete(&key).await.unwrap();
    assert!(!store.exists(&key).await.unwrap());
    // Double-delete is tolerated: GC and upload rollback both re-delete, and
    // some endpoints answer 404 where S3 answers 204.
    store.delete(&key).await.unwrap();
    // Deleting a key that never existed is equally a no-op.
    store
        .delete(&blob_key("itest-user", &"f".repeat(64)))
        .await
        .unwrap();

    // A miss is a miss, not an error, for both existence and size.
    let ghost = blob_key("itest-user", &"a".repeat(64));
    assert!(!store.exists(&ghost).await.unwrap());
    assert_eq!(store.size(&ghost).await.unwrap(), None);

    let _ = tokio::fs::remove_dir_all(&tmp).await;
}

/// Every object we store is written from a file on disk, which is the case
/// where the SDK switches to `aws-chunked` framing unless told otherwise,
/// the shape that silently corrupted uploads through S3 gateways that don't
/// unwrap it. Sizes here straddle the internal 8 MiB part size so the single
/// PUT and any multipart path are both exercised end to end.
#[tokio::test]
async fn s3_streaming_bodies_land_verbatim() {
    let Some(cfg) = test_config() else {
        eprintln!("HOARD_S3_TEST_ENDPOINT unset — skipping S3 integration test");
        return;
    };
    let store = S3Store::connect(&cfg).await.expect("connect");
    store.probe().await.expect("probe");

    let tmp = std::env::temp_dir().join(format!("hoard-s3-stream-{}", uuid::Uuid::new_v4()));
    tokio::fs::create_dir_all(&tmp).await.unwrap();

    for size in [0usize, 1, 8 * 1024 * 1024 - 1, 9 * 1024 * 1024] {
        let payload: Vec<u8> = (0..size).map(|i| (i % 251) as u8).collect();
        let sha = hex::encode(Sha256::digest(&payload));
        let staged = tmp.join(format!("staged-{size}.bin"));
        tokio::fs::write(&staged, &payload).await.unwrap();
        let key = blob_key("itest-stream", &sha);

        store.put_from_file(&key, &staged).await.unwrap();
        assert_eq!(
            store.size(&key).await.unwrap(),
            Some(size as i64),
            "{size}-byte object keeps its length (framing leaking in would grow it)"
        );

        let r = store.local_ref(&key, &tmp.join("spool")).await.unwrap();
        let got = tokio::fs::read(&r.path).await.unwrap();
        assert_eq!(
            hex::encode(Sha256::digest(&got)),
            sha,
            "{size}-byte object round-trips byte for byte"
        );
        let _ = tokio::fs::remove_file(&r.path).await;
        store.delete(&key).await.unwrap();
    }

    let _ = tokio::fs::remove_dir_all(&tmp).await;
}
