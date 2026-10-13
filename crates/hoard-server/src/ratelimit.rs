//! Per-IP request rate limiting, shared by the self-hosted and cloud routers.
//!
//! This is an in-process safety net against accidental request loops and cheap
//! brute force, not a substitute for a reverse-proxy or WAF limiter, which a
//! production deployment should still run in front of the server.

use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::http::{HeaderName, Request};
use governor::middleware::NoOpMiddleware;
use tower_governor::governor::{GovernorConfig, GovernorConfigBuilder};
use tower_governor::key_extractor::{KeyExtractor, PeerIpKeyExtractor, SmartIpKeyExtractor};
use tower_governor::{GovernorError, GovernorLayer};

use crate::config::RateLimitConfig;

/// Concrete config type for our chosen key extractor + (no-op) middleware.
type RlConfig = GovernorConfig<ClientKey, NoOpMiddleware>;

/// Whose bucket a request drains.
///
/// Without `client_ip_header`, [`SmartIpKeyExtractor`]: the leftmost
/// `X-Forwarded-For` entry, then `X-Real-Ip`, then the peer. Any client writes
/// those, which is enough for a loop guard and nothing more. Fly's proxy keeps
/// a client's `X-Forwarded-For: 7.7.7.7` as the leftmost entry and passes
/// `X-Real-Ip` through untouched, so on Hoard Cloud every anonymous route
/// (`/v1/feedback`, `device/start`) had no limit at all for anyone who changed
/// the header per request.
///
/// With it, that header alone, which must be one the proxy in front overwrites
/// on every request: `fly-client-ip` on Fly, which replaced a client-sent
/// `6.6.6.6` with the real address when checked on 13-10-2026. A request
/// without it (a health check from inside Fly) counts against its peer.
#[derive(Clone)]
pub struct ClientKey {
    header: Option<HeaderName>,
}

impl ClientKey {
    fn from_config(cfg: &RateLimitConfig) -> Self {
        let header = cfg
            .client_ip_header
            .as_deref()
            .map(str::trim)
            .filter(|h| !h.is_empty())
            .and_then(|h| match HeaderName::from_bytes(h.as_bytes()) {
                Ok(name) => Some(name),
                Err(_) => {
                    tracing::error!(
                        header = h,
                        "server.rate_limit.client_ip_header is not a valid header name; \
                         keying on X-Forwarded-For as if it were unset"
                    );
                    None
                }
            });
        Self { header }
    }
}

impl KeyExtractor for ClientKey {
    type Key = String;

    fn extract<T>(&self, req: &Request<T>) -> Result<Self::Key, GovernorError> {
        let Some(name) = &self.header else {
            return SmartIpKeyExtractor.extract(req).map(|ip| ip.to_string());
        };
        let claimed = req
            .headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.trim().parse::<IpAddr>().ok());
        let ip = match claimed {
            Some(ip) => ip,
            None => PeerIpKeyExtractor.extract(req)?,
        };
        // Per /64 for IPv6, as the login throttle does: a household owns the
        // whole prefix and would otherwise get a fresh bucket per address.
        Ok(crate::clientip::throttle_bucket(ip))
    }
}

/// Build a [`GovernorLayer`] from config, or `None` when rate limiting is
/// disabled. Also spawns a background task that evicts idle per-IP buckets so a
/// flood of distinct source IPs can't grow the limiter map without bound.
///
/// Keys through [`ClientKey`], which falls back to the socket peer IP. Because
/// of that fallback the router **must** be served with
/// `into_make_service_with_connect_info::<SocketAddr>()`, otherwise the peer IP
/// isn't available and extraction fails closed.
pub fn layer(cfg: &RateLimitConfig) -> Option<GovernorLayer<ClientKey, NoOpMiddleware>> {
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
            .key_extractor(ClientKey::from_config(cfg))
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

    fn keyed_on(header: Option<&str>) -> ClientKey {
        ClientKey::from_config(&RateLimitConfig {
            client_ip_header: header.map(str::to_string),
            ..Default::default()
        })
    }

    fn request(headers: &[(&str, &str)]) -> Request<Body> {
        let mut b = Request::builder().uri("/v1/feedback");
        for (k, v) in headers {
            b = b.header(*k, *v);
        }
        let mut req = b.body(Body::empty()).unwrap();
        req.extensions_mut()
            .insert(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                [172, 16, 0, 9],
                40000,
            ))));
        req
    }

    /// What Fly delivered for a client that sent `X-Forwarded-For: 7.7.7.7`
    /// and `X-Real-Ip: 8.8.8.8`: both kept, the real address only in
    /// `Fly-Client-IP` and second from the right.
    fn as_fly_delivers_it(real: &str) -> Request<Body> {
        let xff = format!("7.7.7.7, {real}, 2001:db8:ffff::2");
        request(&[
            ("fly-client-ip", real),
            ("x-forwarded-for", &xff),
            ("x-real-ip", "8.8.8.8"),
        ])
    }

    #[test]
    fn behind_fly_the_spoofed_headers_are_ignored() {
        let k = keyed_on(Some("fly-client-ip"));
        assert_eq!(
            k.extract(&as_fly_delivers_it("203.0.113.7")).unwrap(),
            "203.0.113.7"
        );
    }

    /// The bypass this closes: without the setting, the client's own header
    /// is the key.
    #[test]
    fn unset_it_keeps_trusting_the_leftmost_entry() {
        let k = keyed_on(None);
        assert_eq!(
            k.extract(&as_fly_delivers_it("203.0.113.7")).unwrap(),
            "7.7.7.7"
        );
    }

    #[test]
    fn a_v6_client_is_one_bucket_per_64() {
        let k = keyed_on(Some("fly-client-ip"));
        let a = k
            .extract(&as_fly_delivers_it(
                "2001:db8:f60a:9e00:d0fe:45c5:180e:c0ee",
            ))
            .unwrap();
        let b = k
            .extract(&as_fly_delivers_it("2001:db8:f60a:9e00::1"))
            .unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn without_the_header_the_peer_is_the_key() {
        let k = keyed_on(Some("fly-client-ip"));
        assert_eq!(
            k.extract(&request(&[("x-forwarded-for", "7.7.7.7")]))
                .unwrap(),
            "172.16.0.9"
        );
        assert_eq!(
            k.extract(&request(&[("fly-client-ip", "not-an-ip")]))
                .unwrap(),
            "172.16.0.9"
        );
    }

    #[test]
    fn a_bad_header_name_degrades_to_the_default() {
        assert!(keyed_on(Some("not a header")).header.is_none());
        assert!(keyed_on(Some("  ")).header.is_none());
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
