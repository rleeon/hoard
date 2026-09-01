//! Per-IP request rate limiting, shared by the self-hosted and cloud routers.
//!
//! This is an in-process safety net against accidental request loops and cheap
//! brute force, not a substitute for a reverse-proxy or WAF limiter, which a
//! production deployment should still run in front of the server.

use std::sync::Arc;
use std::time::Duration;

use governor::middleware::NoOpMiddleware;
use tower_governor::governor::{GovernorConfig, GovernorConfigBuilder};
use tower_governor::key_extractor::SmartIpKeyExtractor;
use tower_governor::GovernorLayer;

use crate::config::RateLimitConfig;

/// Concrete config type for our chosen key extractor + (no-op) middleware.
type RlConfig = GovernorConfig<SmartIpKeyExtractor, NoOpMiddleware>;

/// Build a [`GovernorLayer`] from config, or `None` when rate limiting is
/// disabled. Also spawns a background task that evicts idle per-IP buckets so a
/// flood of distinct source IPs can't grow the limiter map without bound.
///
/// Uses [`SmartIpKeyExtractor`], which keys off `X-Forwarded-For` / `X-Real-Ip`
/// / `Forwarded` (the reverse-proxy case) and falls back to the socket peer IP.
/// Because of that fallback the router **must** be served with
/// `into_make_service_with_connect_info::<SocketAddr>()`, otherwise the peer IP
/// isn't available and extraction fails closed.
pub fn layer(cfg: &RateLimitConfig) -> Option<GovernorLayer<SmartIpKeyExtractor, NoOpMiddleware>> {
    if !cfg.enabled {
        return None;
    }
    // `governor` rejects zero rates; clamp so a misconfiguration degrades to a
    // very tight limit rather than panicking.
    let per_second = cfg.per_second.max(1);
    let burst = cfg.burst.max(1);

    // CAUTION: `GovernorConfigBuilder::per_second(n)` does NOT mean "n
    // requests per second": it sets the replenish PERIOD to n seconds (one
    // request every n seconds sustained). Shipping `.per_second(50)` here
    // granted 1 req/50s after the burst, so any IP with a couple of live
    // clients drained the bucket and then saw 429 ("Limitada") forever.
    // Convert our requests-per-second config into the period governor wants.
    let period = Duration::from_millis((1000 / per_second).max(1));

    let conf: Arc<RlConfig> = Arc::new(
        GovernorConfigBuilder::default()
            .period(period)
            .burst_size(burst)
            .key_extractor(SmartIpKeyExtractor)
            .finish()
            .expect("rate-limit config (per_second/burst are clamped > 0)"),
    );

    let limiter = conf.limiter().clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            limiter.retain_recent();
        }
    });

    Some(GovernorLayer { config: conf })
}

/// Mount the limiter over `app` and **then** merge `exempt`, which therefore
/// never sees it.
///
/// A function rather than three loose lines in `main` because the ordering IS
/// the fix and reads like nothing: `Router::layer` only wraps the routes
/// already mounted, so merging before the `layer` instead of after quietly
/// puts the exempt route back under the limiter, and nothing fails or warns.
/// This way that mistake has a test that catches it.
///
/// `exempt`'s only tenant is `PUT /v1/cas/blobs/:upload_id/:sha256`; the why is
/// in `main.rs`, where it is built.
pub fn apply(cfg: &RateLimitConfig, app: axum::Router, exempt: axum::Router) -> axum::Router {
    let app = match layer(cfg) {
        Some(rl) => app.layer(rl),
        None => app,
    };
    app.merge(exempt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::{get, put};
    use axum::Router;
    use tower::ServiceExt;

    /// A limiter tight enough that the second request no longer fits, so the
    /// test measures the exemption and not the bucket's arithmetic.
    fn one_request_only() -> RateLimitConfig {
        RateLimitConfig {
            enabled: true,
            per_second: 1,
            burst: 1,
            ..Default::default()
        }
    }

    async fn hit(app: &Router, method: &str, uri: &str) -> StatusCode {
        let req = Request::builder()
            .method(method)
            .uri(uri)
            // `SmartIpKeyExtractor` falls back to the peer, which `oneshot`
            // does not have; the header gives it a stable IP so every request
            // in the test shares one bucket.
            .header("x-real-ip", "203.0.113.7")
            .body(Body::empty())
            .unwrap();
        app.clone().oneshot(req).await.unwrap().status()
    }

    fn app_under_test() -> Router {
        let limited = Router::new().route("/v1/saves", get(|| async { "ok" }));
        let exempt = Router::new().route("/v1/cas/blobs/:id/:sha", put(|| async { "ok" }));
        apply(&one_request_only(), limited, exempt)
    }

    /// An ordinary route is still limited: that is what the limiter exists to
    /// do, and without this the test below would also pass with it switched
    /// off.
    #[tokio::test]
    async fn an_ordinary_route_is_still_limited() {
        let app = app_under_test();
        assert_eq!(hit(&app, "GET", "/v1/saves").await, StatusCode::OK);
        assert_eq!(
            hit(&app, "GET", "/v1/saves").await,
            StatusCode::TOO_MANY_REQUESTS,
            "the limiter must keep braking whatever is not exempt"
        );
    }

    /// Issue #17: the blob upload is never limited, however many there are.
    /// That is 173 PUTs for the issue's Teardown, and the count was fixed by
    /// the server itself when it answered `cas/init`, so limiting here is
    /// fighting the batch we authorised ourselves.
    #[tokio::test]
    async fn the_blob_upload_is_never_limited() {
        let app = app_under_test();
        // Drain the bucket first, so the exemption is the only thing that can
        // explain the PUTs getting through.
        let _ = hit(&app, "GET", "/v1/saves").await;
        assert_eq!(
            hit(&app, "GET", "/v1/saves").await,
            StatusCode::TOO_MANY_REQUESTS
        );

        for i in 0..200 {
            assert_eq!(
                hit(&app, "PUT", "/v1/cas/blobs/abc/def").await,
                StatusCode::OK,
                "the blob PUT was limited on request {i}"
            );
        }
    }
}
