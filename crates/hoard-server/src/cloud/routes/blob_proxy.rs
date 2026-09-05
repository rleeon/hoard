//! `/v1/cloud/blob/:token`: decompressing download proxy for blobs the
//! compression sweep rewrote as zstd (`cloud::compress`).
//!
//! Clients treat download URLs as opaque: `version_manifest` hands them a
//! URL and they GET it with no auth header (presigned-style). For raw blobs
//! that URL is a presigned R2 GET as always; for compressed blobs it points
//! here instead, with an HMAC token standing in for the presigned
//! signature. The response body is the RAW bytes, decompressed on the fly and
//! streamed with bounded memory, so the client's per-file SHA-256 check
//! passes exactly as if R2 had served the object directly. No client code
//! knows compression exists.
//!
//! The route is on the public router: the token is the whole auth, same
//! trust model as a presigned URL (unguessable, expiring, scoped to one
//! user+blob). The signing key is derived from the R2 secret so there's no
//! new secret to provision.

use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::cloud::state::CloudState;

type HmacSha256 = Hmac<Sha256>;

/// Signing key: derived from the R2 secret with a domain separator, so the
/// deployment needs no extra config and a leaked token can't be replayed
/// against anything else.
fn signing_key(state: &CloudState) -> Vec<u8> {
    let r2_secret = state
        .config
        .cloud
        .as_ref()
        .map(|c| c.r2.secret_access_key.as_str())
        .unwrap_or("");
    format!("hoard-blob-proxy:{r2_secret}").into_bytes()
}

/// Mint a token for one (user, blob) pair expiring at unix time `exp`.
/// Shape: `base64url(user_id:sha256:exp_unix)` + `.` + `base64url(mac)`.
fn mint_with(key: &[u8], user_id: Uuid, sha256: &str, exp: i64) -> String {
    let payload = format!("{user_id}:{sha256}:{exp}");
    let mut mac = HmacSha256::new_from_slice(key).expect("hmac accepts any key");
    mac.update(payload.as_bytes());
    let tag = mac.finalize().into_bytes();
    format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(payload.as_bytes()),
        URL_SAFE_NO_PAD.encode(tag)
    )
}

/// Verify a token; `None` on any mismatch, tamper or expiry.
fn verify_with(key: &[u8], token: &str) -> Option<(Uuid, String)> {
    let (payload_b64, tag_b64) = token.split_once('.')?;
    let payload = URL_SAFE_NO_PAD.decode(payload_b64).ok()?;
    let tag = URL_SAFE_NO_PAD.decode(tag_b64).ok()?;
    let mut mac = HmacSha256::new_from_slice(key).ok()?;
    mac.update(&payload);
    mac.verify_slice(&tag).ok()?;

    let payload = String::from_utf8(payload).ok()?;
    let mut parts = payload.splitn(3, ':');
    let user_id: Uuid = parts.next()?.parse().ok()?;
    let sha256 = parts.next()?.to_string();
    let exp: i64 = parts.next()?.parse().ok()?;
    if OffsetDateTime::now_utc().unix_timestamp() > exp {
        return None;
    }
    Some((user_id, sha256))
}

/// Mint a proxy token for one (user, blob) pair, valid for `ttl_secs`.
pub fn mint_token(state: &CloudState, user_id: Uuid, sha256: &str, ttl_secs: u64) -> String {
    let exp = OffsetDateTime::now_utc().unix_timestamp() + ttl_secs as i64;
    mint_with(&signing_key(state), user_id, sha256, exp)
}

fn verify_token(state: &CloudState, token: &str) -> Option<(Uuid, String)> {
    verify_with(&signing_key(state), token)
}

/// GET handler. Streams the blob, decompressing when (and only when) the
/// row says the compressed overwrite completed (`stored_bytes` set); a
/// claimed-but-still-raw blob passes through untouched.
pub async fn download(
    State(state): State<CloudState>,
    Path(token): Path<String>,
) -> Result<Response, StatusCode> {
    let Some((user_id, sha256)) = verify_token(&state, &token) else {
        return Err(StatusCode::FORBIDDEN);
    };

    let row: Option<(Option<String>, Option<i64>, i64)> = sqlx::query_as(
        "SELECT encoding, stored_bytes, size_bytes
           FROM cloud_blobs WHERE user_id = $1 AND sha256 = decode($2, 'hex')",
    )
    .bind(user_id)
    .bind(&sha256)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        tracing::warn!(error = %e, "blob proxy: row lookup failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let Some((encoding, stored_bytes, size_bytes)) = row else {
        return Err(StatusCode::NOT_FOUND);
    };
    let r2_key = crate::cloud::r2::key_for_blob(user_id, &sha256);

    let reader = state.r2.get_reader(&r2_key).await.map_err(|e| {
        tracing::warn!(error = %e, r2_key, "blob proxy: R2 get failed");
        StatusCode::BAD_GATEWAY
    })?;

    let body = if encoding.as_deref() == Some("zstd") && stored_bytes.is_some() {
        let decoder = async_compression::tokio::bufread::ZstdDecoder::new(reader);
        Body::from_stream(tokio_util::io::ReaderStream::new(decoder))
    } else {
        Body::from_stream(tokio_util::io::ReaderStream::new(reader))
    };

    // Either way the client receives the raw bytes, whose length we know.
    let mut resp = body.into_response();
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );
    resp.headers_mut()
        .insert(header::CONTENT_LENGTH, HeaderValue::from(size_bytes.max(0)));
    Ok(resp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_round_trips() {
        let key = b"test-key";
        let user = Uuid::new_v4();
        let exp = OffsetDateTime::now_utc().unix_timestamp() + 60;
        let t = mint_with(key, user, "abc123", exp);
        assert_eq!(verify_with(key, &t), Some((user, "abc123".to_string())));
    }

    #[test]
    fn expired_token_rejected() {
        let key = b"test-key";
        let user = Uuid::new_v4();
        let exp = OffsetDateTime::now_utc().unix_timestamp() - 1;
        let t = mint_with(key, user, "abc123", exp);
        assert_eq!(verify_with(key, &t), None);
    }

    #[test]
    fn tampered_token_rejected() {
        let key = b"test-key";
        let user = Uuid::new_v4();
        let exp = OffsetDateTime::now_utc().unix_timestamp() + 60;
        let t = mint_with(key, user, "abc123", exp);
        // Flip the sha inside the payload: re-encode with a different sha
        // but keep the original tag.
        let (_, tag) = t.split_once('.').unwrap();
        let forged_payload = URL_SAFE_NO_PAD.encode(format!("{user}:evil:{exp}").as_bytes());
        let forged = format!("{forged_payload}.{tag}");
        assert_eq!(verify_with(key, &forged), None);
        // Wrong key entirely.
        assert_eq!(verify_with(b"other-key", &t), None);
    }
}
