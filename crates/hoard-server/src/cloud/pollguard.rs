//! Per-device rate limit on the cheap polling endpoints.
//!
//! The official client polls `/v1/cloud/sync`, `/v1/devices`,
//! `/v1/notifications` (≤1/min each) and `/v1/presence/heartbeat` (2/min);
//! Realtime kicks collapse into at most a couple of extra pulls. The per-IP
//! limiter in `crate::ratelimit` has to stay loose (a re-sync after install
//! bursts hundreds of requests), which also lets a single runaway poller (a
//! modified client, or the pre-1.0.4 `prefs.json` knob set to 2 s) hammer
//! these endpoints forever. This guard caps each one per (user, device,
//! endpoint) instead, so abuse hits a wall without touching legit bursts on
//! the save endpoints.
//!
//! Device identity is the `x-hoard-device-fp` header (same one `/v1/me`
//! uses for the devices row). Requests without it, from older builds or browser
//! calls on the account page, share the user's single no-fp bucket,
//! which the generous limit absorbs. `/v1/health` is deliberately *not*
//! guarded: Fly probes it every 15 s to decide machine health, and a 429
//! there would flap the deployment.

use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::Request;
use axum::http::{HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use governor::clock::{Clock, DefaultClock};
use governor::state::keyed::DefaultKeyedStateStore;
use governor::{Quota, RateLimiter};
use uuid::Uuid;

use crate::cloud::auth::CloudUser;

/// Bucket key. The endpoint class is part of the key so a runaway sync poll
/// can't starve the same device's presence heartbeat.
type Key = (Uuid, String, &'static str);

type Keyed = RateLimiter<Key, DefaultKeyedStateStore<Key>, DefaultClock>;

pub struct PollGuard {
    /// `None` = guard disabled (`poll_per_minute = 0` or rate limiting off).
    limiter: Option<Keyed>,
    clock: DefaultClock,
}

impl PollGuard {
    /// `per_minute = 0` builds a disabled guard that lets everything through.
    /// Spawns the idle-bucket eviction task, so call it from inside the
    /// runtime.
    pub fn new(per_minute: u32, burst: u32) -> Arc<Self> {
        let limiter = NonZeroU32::new(per_minute).map(|n| {
            // Sustained one request every 60/n seconds. The burst must be
            // large enough for the official client's legitimate spikes,
            // app startup fires one `/v1/cloud/sync` per auto-restored save
            // on top of the login pull, while the sustained rate is what
            // walls off a hammering client once the burst drains.
            let burst = NonZeroU32::new(burst.max(1)).expect("clamped > 0");
            let quota = Quota::with_period(Duration::from_secs_f64(60.0 / n.get() as f64))
                .expect("non-zero period")
                .allow_burst(burst.max(n));
            RateLimiter::keyed(quota)
        });
        let guard = Arc::new(Self {
            limiter,
            clock: DefaultClock::default(),
        });
        if guard.limiter.is_some() {
            let g = guard.clone();
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(60)).await;
                    if let Some(l) = &g.limiter {
                        l.retain_recent();
                    }
                }
            });
        }
        guard
    }

    /// `Ok` = request may pass; `Err` = how long the caller should wait.
    fn check(&self, key: Key) -> Result<(), Duration> {
        match &self.limiter {
            None => Ok(()),
            Some(l) => l
                .check_key(&key)
                .map_err(|not_until| not_until.wait_time_from(self.clock.now())),
        }
    }
}

/// Middleware for one guarded route. Attach with `route_layer` so it runs
/// *after* `require_cloud_auth` (router-level layers wrap route layers) and
/// sees the authenticated `CloudUser`.
pub async fn guard(
    guard: Arc<PollGuard>,
    class: &'static str,
    req: Request,
    next: Next,
) -> Response {
    // No CloudUser means auth didn't run (mis-wired route), so fail open: the
    // per-IP limiter still applies.
    let Some(user) = req.extensions().get::<CloudUser>() else {
        return next.run(req).await;
    };
    let fp = req
        .headers()
        .get("x-hoard-device-fp")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    match guard.check((user.user_id, fp, class)) {
        Ok(()) => next.run(req).await,
        Err(wait) => {
            let retry_secs = wait.as_secs().max(1);
            let mut resp = (
                StatusCode::TOO_MANY_REQUESTS,
                axum::Json(serde_json::json!({
                    "error": format!("polling {class} too fast; retry in {retry_secs}s"),
                    "code": "rate_limited",
                })),
            )
                .into_response();
            resp.headers_mut()
                .insert("retry-after", HeaderValue::from(retry_secs));
            resp
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn burst_allowed_then_denied() {
        let g = PollGuard::new(3, 3);
        let user = Uuid::new_v4();
        let key = || (user, "fp-a".to_string(), "sync");
        for _ in 0..3 {
            assert!(g.check(key()).is_ok());
        }
        let wait = g.check(key()).expect_err("4th request within a minute");
        assert!(wait > Duration::ZERO);
    }

    #[tokio::test]
    async fn other_device_and_class_have_own_buckets() {
        let g = PollGuard::new(2, 2);
        let user = Uuid::new_v4();
        for _ in 0..2 {
            assert!(g.check((user, "fp-a".into(), "sync")).is_ok());
        }
        assert!(g.check((user, "fp-a".into(), "sync")).is_err());
        // Same user, different device: fresh bucket.
        assert!(g.check((user, "fp-b".into(), "sync")).is_ok());
        // Same device, different endpoint: fresh bucket.
        assert!(g.check((user, "fp-a".into(), "heartbeat")).is_ok());
    }

    #[tokio::test]
    async fn startup_burst_passes_beyond_sustained_rate() {
        // Regression: app startup fires one /v1/cloud/sync per auto-restored
        // save; a burst well above the sustained per-minute rate must pass.
        let g = PollGuard::new(2, 40);
        let user = Uuid::new_v4();
        for i in 0..40 {
            assert!(
                g.check((user, "fp".into(), "sync")).is_ok(),
                "burst request {i} should pass"
            );
        }
        assert!(g.check((user, "fp".into(), "sync")).is_err());
    }

    #[tokio::test]
    async fn zero_disables_the_guard() {
        let g = PollGuard::new(0, 0);
        let user = Uuid::new_v4();
        for _ in 0..100 {
            assert!(g.check((user, "fp".into(), "sync")).is_ok());
        }
    }
}
