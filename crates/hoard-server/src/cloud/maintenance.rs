//! Maintenance mode, for moving the database.
//!
//! A server that starts while the file at `[cloud] maintenance_flag` exists
//! answers everything but `/v1/health` with a 429, and starts none of its
//! background tasks. The dump taken meanwhile is only the database if both
//! hold: no request writes to it, and no sweep records a change it already
//! made in R2 after the dump was taken (the compressor rewriting a blob as
//! zstd, a purge deleting objects). A copy that missed either would point at
//! bytes that are no longer there, or call compressed bytes raw.
//!
//! 429 and not 503, because of what clients do with each. A 503 is a server
//! error and the agent reports a failed backup. A 429 is a wait: 1.0.4 and
//! 1.1.4 show the save waiting on the bandwidth limit and retry once it is
//! over, and from 1.1.5, where the `code` marks it a budget, the upload is
//! parked without a word. Seen in the rehearsal of 16-sep-2026 with the
//! released binaries: every one landed its version within 33 s of reopening.
//!
//! Read once at start-up. Going in and out is a restart of the process, which
//! the supervisor turns around in a second, and that restart is also what
//! drains the requests in flight before the dump starts.

use axum::{
    extract::Request,
    http::{header, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};

/// Short enough that clients are back soon after the move, and under the
/// 300 s that every client caps a server's hint at.
pub const RETRY_AFTER_SECS: u64 = 120;

pub fn active(flag: &str) -> bool {
    !flag.is_empty() && std::path::Path::new(flag).exists()
}

/// Fly restarts a machine whose health check fails, and a machine restarting
/// in the middle of a dump is the one thing the maintenance is there to avoid.
pub async fn guard(req: Request, next: Next) -> Response {
    if req.uri().path() == "/v1/health" {
        return next.run(req).await;
    }
    let mut res = (
        StatusCode::TOO_MANY_REQUESTS,
        Json(serde_json::json!({
            "error": "Hoard Cloud is under maintenance, back in a few minutes",
            "code": "maintenance",
            "retry_after_seconds": RETRY_AFTER_SECS,
        })),
    )
        .into_response();
    res.headers_mut()
        .insert(header::RETRY_AFTER, HeaderValue::from(RETRY_AFTER_SECS));
    res
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, middleware, routing::get, Router};
    use tower::ServiceExt;

    fn app() -> Router {
        Router::new()
            .route("/v1/health", get(|| async { "ok" }))
            .route("/v1/cloud/sync", get(|| async { "manifest" }))
            .route("/v1/webhooks/polar", axum::routing::post(|| async { "ok" }))
            .layer(middleware::from_fn(guard))
    }

    async fn call(method: &str, path: &str) -> Response {
        app()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(path)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn health_still_answers() {
        assert_eq!(call("GET", "/v1/health").await.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn everything_else_is_a_coded_429() {
        for (method, path) in [
            ("GET", "/v1/cloud/sync"),
            ("POST", "/v1/webhooks/polar"),
            ("GET", "/v1/route/that/does/not/exist"),
        ] {
            let res = call(method, path).await;
            assert_eq!(
                res.status(),
                StatusCode::TOO_MANY_REQUESTS,
                "{method} {path}"
            );
            assert_eq!(res.headers()[header::RETRY_AFTER], "120");
            let bytes = axum::body::to_bytes(res.into_body(), 1 << 16)
                .await
                .unwrap();
            let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            // The client files a 429 as a budget only when it carries a code,
            // and reads the wait from the body before the header.
            assert_eq!(body["code"], "maintenance");
            assert_eq!(body["retry_after_seconds"], 120);
        }
    }

    #[test]
    fn the_flag_is_a_file_that_exists() {
        let dir = tempfile::tempdir().unwrap();
        let flag = dir.path().join("MAINTENANCE");
        assert!(!active(""), "no path configured means never");
        assert!(!active(flag.to_str().unwrap()));
        std::fs::write(&flag, "").unwrap();
        assert!(active(flag.to_str().unwrap()));
    }
}
