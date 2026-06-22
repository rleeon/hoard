//! Per-IP request rate limiting, shared by the self-hosted and cloud routers.
//!
//! This is an in-process safety net against accidental request loops and cheap
//! brute force — not a substitute for a reverse-proxy / WAF limiter, which a
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

    let conf: Arc<RlConfig> = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(per_second)
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
