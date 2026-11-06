//! Per-IP request rate limiting, shared by the self-hosted and cloud routers.
//!
//! This is an in-process safety net against accidental request loops and cheap
//! brute force, not a substitute for a reverse-proxy or WAF limiter, which a
//! production deployment should still run in front of the server.

use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use axum::http::{HeaderMap, HeaderName, Request};
use governor::middleware::NoOpMiddleware;
use tower_governor::governor::{GovernorConfig, GovernorConfigBuilder};
use tower_governor::key_extractor::{KeyExtractor, PeerIpKeyExtractor};
use tower_governor::{GovernorError, GovernorLayer};

use crate::clientip::TrustedProxies;
use crate::config::RateLimitConfig;

/// Concrete config type for our chosen key extractor + (no-op) middleware.
type RlConfig = GovernorConfig<ClientKey, NoOpMiddleware>;

/// Whose bucket a request drains.
///
/// With `client_ip_header`, that header alone, which must be one the proxy in
/// front overwrites on every request: `fly-client-ip` on Fly, which replaced a
/// client-sent `6.6.6.6` with the real address when checked on 13-10-2026. A
/// request without it (a health check from inside Fly) counts against its
/// peer. Fly's proxy keeps a client's `X-Forwarded-For: 7.7.7.7` as the
/// leftmost entry and passes `X-Real-Ip` through untouched, which is why Hoard
/// Cloud needs this setting and cannot rely on the fallback below.
///
/// Without it, the same answer the panel's login throttle gets from
/// [`crate::clientip::client_ip`]: `X-Forwarded-For` is believed only from a
/// peer listed in `server.trusted_proxies`, read from the right; from anyone
/// else the peer is the key. This used to be `tower_governor`'s
/// `SmartIpKeyExtractor`, which took the leftmost entry from whoever sent it,
/// so a client that changed the header on every request was never limited,
/// and one that wrote somebody else's address drained that person's bucket
/// instead of its own.
#[derive(Clone)]
pub struct ClientKey {
    header: Option<HeaderName>,
    trusted: TrustedProxies,
}

impl ClientKey {
    fn from_config(cfg: &RateLimitConfig, trusted: &TrustedProxies) -> Self {
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
                         keying on server.trusted_proxies as if it were unset"
                    );
                    None
                }
            });
        Self {
            header,
            trusted: trusted.clone(),
        }
    }
}

impl KeyExtractor for ClientKey {
    type Key = String;

    fn extract<T>(&self, req: &Request<T>) -> Result<Self::Key, GovernorError> {
        let ip = match &self.header {
            Some(name) => {
                let claimed = req
                    .headers()
                    .get(name)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.trim().parse::<IpAddr>().ok());
                match claimed {
                    Some(ip) => ip,
                    None => PeerIpKeyExtractor.extract(req)?,
                }
            }
            None => {
                let peer = PeerIpKeyExtractor.extract(req)?;
                if forwards_from_untrusted(req.headers(), peer, &self.trusted) {
                    warn_untrusted_forwarder_once(peer);
                }
                // `client_ip` only looks at the peer's address, never its port.
                crate::clientip::client_ip(req.headers(), SocketAddr::new(peer, 0), &self.trusted)
            }
        };
        // Per /64 for IPv6, as the login throttle does: a household owns the
        // whole prefix and would otherwise get a fresh bucket per address.
        Ok(crate::clientip::throttle_bucket(ip))
    }
}

/// Does this request name another client while coming from a peer we do not
/// take at its word?
///
/// That is either someone trying to pick their own bucket, which now fails,
/// or a reverse proxy the operator forgot to list in `server.trusted_proxies`,
/// whose clients now all share its bucket. The second one would otherwise be
/// invisible until the 429s start, so it is worth a line in the log.
fn forwards_from_untrusted(headers: &HeaderMap, peer: IpAddr, trusted: &TrustedProxies) -> bool {
    (headers.contains_key("x-forwarded-for") || headers.contains_key("x-real-ip"))
        && !trusted.contains(peer)
}

/// Set once [`warn_untrusted_forwarder_once`] has logged, so a busy proxy, or
/// a client spraying headers, costs one line and not one per request.
static WARNED_UNTRUSTED_FORWARDER: AtomicBool = AtomicBool::new(false);

fn warn_untrusted_forwarder_once(peer: IpAddr) {
    if WARNED_UNTRUSTED_FORWARDER.swap(true, Ordering::Relaxed) {
        return;
    }
    tracing::warn!(
        %peer,
        "a request names another client in X-Forwarded-For or X-Real-Ip, but its \
         peer is not in server.trusted_proxies, so the rate limit counts it as the \
         peer. If the peer is your reverse proxy, add it to server.trusted_proxies, \
         or every client behind it shares one bucket. Logged once."
    );
}

/// Build a [`GovernorLayer`] from config, or `None` when rate limiting is
/// disabled. Also spawns a background task that evicts idle per-IP buckets so a
/// flood of distinct source IPs can't grow the limiter map without bound.
///
/// Keys through [`ClientKey`], which needs the socket peer IP. Because of that
/// the router **must** be served with
/// `into_make_service_with_connect_info::<SocketAddr>()`, otherwise the peer IP
/// isn't available and extraction fails closed.
pub fn layer(
    cfg: &RateLimitConfig,
    trusted: &TrustedProxies,
) -> Option<GovernorLayer<ClientKey, NoOpMiddleware>> {
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
            .key_extractor(ClientKey::from_config(cfg, trusted))
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
pub fn apply(
    cfg: &RateLimitConfig,
    trusted: &TrustedProxies,
    app: axum::Router,
    exempt: axum::Router,
) -> axum::Router {
    let app = match layer(cfg, trusted) {
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

    fn trusting(entries: &[&str]) -> TrustedProxies {
        let owned: Vec<String> = entries.iter().map(|s| s.to_string()).collect();
        let (trusted, bad) = TrustedProxies::parse(&owned);
        assert!(bad.is_empty(), "{bad:?}");
        trusted
    }

    fn keyed_on(header: Option<&str>, trusted: &[&str]) -> ClientKey {
        ClientKey::from_config(
            &RateLimitConfig {
                client_ip_header: header.map(str::to_string),
                ..Default::default()
            },
            &trusting(trusted),
        )
    }

    /// A request as it arrives from `peer`, the address on the other end of
    /// the socket.
    fn request_from(peer: [u8; 4], headers: &[(&str, &str)]) -> Request<Body> {
        let mut b = Request::builder().uri("/v1/feedback");
        for (k, v) in headers {
            b = b.header(*k, *v);
        }
        let mut req = b.body(Body::empty()).unwrap();
        req.extensions_mut()
            .insert(axum::extract::ConnectInfo(SocketAddr::from((peer, 40000))));
        req
    }

    /// A request from a peer that is not a trusted proxy.
    fn request(headers: &[(&str, &str)]) -> Request<Body> {
        request_from([172, 16, 0, 9], headers)
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
        let k = keyed_on(Some("fly-client-ip"), &["loopback"]);
        assert_eq!(
            k.extract(&as_fly_delivers_it("203.0.113.7")).unwrap(),
            "203.0.113.7"
        );
    }

    /// The bypass this closes: without `client_ip_header`, a caller that is not
    /// a trusted proxy used to be keyed on its own leftmost `X-Forwarded-For`
    /// entry. Rotating it bought a fresh bucket on every request, and writing
    /// someone else's address drained theirs.
    #[test]
    fn a_direct_caller_cannot_pick_its_bucket() {
        let k = keyed_on(None, &["loopback"]);
        for spoof in ["7.7.7.7", "203.0.113.7", "not-an-ip"] {
            assert_eq!(
                k.extract(&request(&[
                    ("x-forwarded-for", spoof),
                    ("x-real-ip", spoof)
                ]))
                .unwrap(),
                "172.16.0.9",
                "the client chose its bucket with {spoof}"
            );
        }
    }

    /// The same request on Fly without `client_ip_header`: Fly's proxy is not
    /// a trusted peer, so its clients share its bucket rather than choosing one.
    #[test]
    fn unset_an_untrusted_proxy_counts_as_itself() {
        let k = keyed_on(None, &["loopback"]);
        assert_eq!(
            k.extract(&as_fly_delivers_it("203.0.113.7")).unwrap(),
            "172.16.0.9"
        );
    }

    /// nginx with `$proxy_add_x_forwarded_for` on the same host: the left end
    /// is still the client's to write, the right end is what nginx saw.
    #[test]
    fn a_trusted_proxy_is_believed_from_the_right() {
        let k = keyed_on(None, &["loopback"]);
        let req = request_from(
            [127, 0, 0, 1],
            &[("x-forwarded-for", "7.7.7.7, 203.0.113.7")],
        );
        assert_eq!(k.extract(&req).unwrap(), "203.0.113.7");
    }

    #[test]
    fn the_fallback_buckets_v6_per_64() {
        let k = keyed_on(None, &["loopback"]);
        let a = k
            .extract(&request_from(
                [127, 0, 0, 1],
                &[("x-forwarded-for", "2001:db8:f60a:9e00:d0fe:45c5:180e:c0ee")],
            ))
            .unwrap();
        let b = k
            .extract(&request_from(
                [127, 0, 0, 1],
                &[("x-forwarded-for", "2001:db8:f60a:9e00::1")],
            ))
            .unwrap();
        assert_eq!(a, b);
        assert_eq!(a, "2001:db8:f60a:9e00::/64");
    }

    #[test]
    fn only_an_untrusted_peer_that_forwards_is_reported() {
        let trusted = trusting(&["loopback"]);
        let mut xff = HeaderMap::new();
        xff.insert("x-forwarded-for", "203.0.113.7".parse().unwrap());
        let mut real_ip = HeaderMap::new();
        real_ip.insert("x-real-ip", "203.0.113.7".parse().unwrap());
        let docker_proxy: IpAddr = "172.18.0.3".parse().unwrap();
        let loopback: IpAddr = "127.0.0.1".parse().unwrap();

        assert!(forwards_from_untrusted(&xff, docker_proxy, &trusted));
        assert!(forwards_from_untrusted(&real_ip, docker_proxy, &trusted));
        assert!(!forwards_from_untrusted(&xff, loopback, &trusted));
        assert!(!forwards_from_untrusted(
            &HeaderMap::new(),
            docker_proxy,
            &trusted
        ));
    }

    #[test]
    fn a_v6_client_is_one_bucket_per_64() {
        let k = keyed_on(Some("fly-client-ip"), &["loopback"]);
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
        let k = keyed_on(Some("fly-client-ip"), &["loopback"]);
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
        assert!(keyed_on(Some("not a header"), &[]).header.is_none());
        assert!(keyed_on(Some("  "), &[]).header.is_none());
    }

    async fn hit(app: &Router, method: &str, uri: &str) -> StatusCode {
        let mut req = Request::builder()
            .method(method)
            .uri(uri)
            .body(Body::empty())
            .unwrap();
        // `oneshot` has no socket, so the peer the key falls back to is put
        // in by hand: one fixed address, so every request in the test shares
        // one bucket.
        req.extensions_mut()
            .insert(axum::extract::ConnectInfo(SocketAddr::from((
                [203, 0, 113, 7],
                40000,
            ))));
        app.clone().oneshot(req).await.unwrap().status()
    }

    fn app_under_test() -> Router {
        let limited = Router::new().route("/v1/saves", get(|| async { "ok" }));
        let exempt = Router::new().route("/v1/cas/blobs/:id/:sha", put(|| async { "ok" }));
        apply(
            &one_request_only(),
            &TrustedProxies::default(),
            limited,
            exempt,
        )
    }

    /// The bypass end to end, through the real limiter: the same peer with a
    /// fresh `X-Forwarded-For` on every request still has one bucket.
    #[tokio::test]
    async fn a_fresh_forwarded_for_does_not_buy_a_fresh_bucket() {
        let limited = Router::new().route("/v1/feedback", get(|| async { "ok" }));
        let app = apply(
            &one_request_only(),
            &trusting(&["loopback"]),
            limited,
            Router::new(),
        );
        let first = request(&[("x-forwarded-for", "198.51.100.1")]);
        assert_eq!(
            app.clone().oneshot(first).await.unwrap().status(),
            StatusCode::OK
        );
        let second = request(&[("x-forwarded-for", "198.51.100.2")]);
        assert_eq!(
            app.clone().oneshot(second).await.unwrap().status(),
            StatusCode::TOO_MANY_REQUESTS,
            "a new X-Forwarded-For from the same peer must not reset its limit"
        );
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
