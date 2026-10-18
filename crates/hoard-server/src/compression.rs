//! Response compression for the JSON API.

use axum::http::{header, Extensions, HeaderMap, StatusCode, Version};
use tower_http::compression::predicate::{Predicate, SizeAbove};
use tower_http::compression::CompressionLayer;

/// gzip or zstd for JSON bodies only: the sync poll and the restore manifests,
/// which compress 3-5x. Save bytes (blob proxy, tar.zst) and the SSE stream go
/// out untouched, and a client that doesn't ask gets what it always got.
pub fn json_only() -> CompressionLayer<impl Predicate> {
    CompressionLayer::new().compress_when(SizeAbove::new(256).and(
        |_: StatusCode, _: Version, headers: &HeaderMap, _: &Extensions| {
            headers
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .is_some_and(|ct| ct.starts_with("application/json"))
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use axum::routing::get;
    use axum::Router;
    use tower::ServiceExt;

    async fn encoding_of(path: &str, accept: &str) -> Option<String> {
        let app = Router::new()
            .route("/json", get(|| async { axum::Json(vec!["save"; 200]) }))
            .route(
                "/bytes",
                get(|| async {
                    (
                        [(header::CONTENT_TYPE, "application/octet-stream")],
                        vec![0u8; 4096],
                    )
                }),
            )
            .layer(json_only());
        let req = Request::get(path)
            .header(header::ACCEPT_ENCODING, accept)
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        resp.headers()
            .get(header::CONTENT_ENCODING)
            .map(|v| v.to_str().unwrap().to_string())
    }

    #[tokio::test]
    async fn only_json_is_compressed_and_only_when_asked() {
        assert_eq!(encoding_of("/json", "gzip").await.as_deref(), Some("gzip"));
        assert_eq!(encoding_of("/json", "zstd").await.as_deref(), Some("zstd"));
        assert_eq!(encoding_of("/json", "identity").await, None);
        assert_eq!(encoding_of("/bytes", "gzip, zstd").await, None);
    }
}
