//! The failures worth a human's attention, counted over the last 24 hours for
//! the Discord status embed.
//!
//! What counts is decided by *where* the count happens, not by log level:
//!
//!   * any 5xx the API answered, sorted by what the request was doing
//!     (upload, download, delete, anything else), via the [`track`]
//!     middleware;
//!   * a background job that deletes or moves user data failing, via
//!     [`record`] next to the `warn!`/`error!` it already logs.
//!
//! 4xx never count. Quota refusals, pacing and non-fast-forward conflicts are
//! the server doing its job and they are most of the log; counting them would
//! leave the number permanently high, and a number that is always high is one
//! nobody reads.
//!
//! Kept in memory in hourly buckets: a few hundred bytes, no table, and no
//! write on the error path, which is exactly when the database may be the
//! thing that is down. The price is that a restart zeroes it, so the embed
//! says "since boot" while the process is younger than the window.

use axum::{
    extract::{MatchedPath, Request},
    http::Method,
    middleware::Next,
    response::Response,
};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// What the failed operation was doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Upload = 0,
    Download = 1,
    Delete = 2,
    Other = 3,
}

pub const KINDS: usize = 4;
const HOURS: u64 = 24;
/// Cap on the remembered "last failure" line.
const LAST_MAX_CHARS: usize = 120;

/// The most recent failure of one kind: when (unix seconds) and what.
pub type Last = Option<(u64, String)>;

pub struct Window {
    /// The unix hour each slot currently holds. A slot stamped with an hour
    /// outside the window reads as zero, which is how old hours fall off
    /// without anything sweeping them.
    stamp: [u64; HOURS as usize],
    counts: [[u32; KINDS]; HOURS as usize],
    last: [Last; KINDS],
}

impl Window {
    pub const fn new() -> Self {
        Self {
            stamp: [0; HOURS as usize],
            counts: [[0; KINDS]; HOURS as usize],
            last: [None, None, None, None],
        }
    }

    fn record(&mut self, now: u64, kind: Kind, what: &str) {
        let hour = now / 3600;
        let slot = (hour % HOURS) as usize;
        if self.stamp[slot] != hour {
            self.stamp[slot] = hour;
            self.counts[slot] = [0; KINDS];
        }
        let n = &mut self.counts[slot][kind as usize];
        *n = n.saturating_add(1);
        self.last[kind as usize] = Some((now, what.chars().take(LAST_MAX_CHARS).collect()));
    }

    fn snapshot(&self, now: u64) -> Snapshot {
        let hour = now / 3600;
        let mut counts = [0u32; KINDS];
        for (slot, &stamp) in self.stamp.iter().enumerate() {
            // The window is the current hour and the 23 before it.
            if stamp <= hour && hour - stamp < HOURS {
                for (total, n) in counts.iter_mut().zip(self.counts[slot]) {
                    *total = total.saturating_add(n);
                }
            }
        }
        let last = std::array::from_fn(|k| {
            self.last[k]
                .clone()
                .filter(|(at, _)| now.saturating_sub(*at) < HOURS * 3600)
        });
        Snapshot { counts, last }
    }
}

impl Default for Window {
    fn default() -> Self {
        Self::new()
    }
}

/// Counts over the last 24 hours by [`Kind`], and the latest failure of each.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Snapshot {
    pub counts: [u32; KINDS],
    pub last: [Last; KINDS],
}

impl Snapshot {
    pub fn count(&self, kind: Kind) -> u32 {
        self.counts[kind as usize]
    }

    /// The most recent failure of any kind.
    pub fn latest(&self) -> Option<&(u64, String)> {
        self.last.iter().flatten().max_by_key(|(at, _)| *at)
    }
}

static WINDOW: Mutex<Window> = Mutex::new(Window::new());

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Count one failure. Cheap enough for any error path: one uncontended lock
/// and at most one short string.
pub fn record(kind: Kind, what: &str) {
    // A poisoned lock only means some thread panicked halfway through bumping
    // a counter. The data is still counters; losing the count, or panicking
    // again on an error path, would both be worse.
    WINDOW
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .record(now_secs(), kind, what);
}

/// The current 24-hour picture.
pub fn snapshot() -> Snapshot {
    WINDOW
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .snapshot(now_secs())
}

/// Sort a request into a [`Kind`] by method and route template.
pub fn classify(method: &Method, route: &str) -> Kind {
    if *method == Method::DELETE {
        return Kind::Delete;
    }
    // `…/versions/:version/commit` and `…/versions/:version/cas/commit` both
    // end in `/commit`.
    if route == "/v1/cloud/cas/init"
        || (route == "/v1/cloud/saves" && *method == Method::POST)
        || route.ends_with("/commit")
    {
        return Kind::Upload;
    }
    if route == "/v1/cloud/sync"
        || route.starts_with("/v1/cloud/blob/")
        || route.ends_with("/download")
        || route.ends_with("/manifest")
    {
        return Kind::Download;
    }
    Kind::Other
}

/// Counts every 5xx the API answers. Mounted once over the whole router, so a
/// route added later is covered without anyone having to remember.
pub async fn track(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    // The template (`/v1/cloud/saves/:save_id`), not the concrete path: it is
    // what classifies, and it keeps ids out of a line posted to Discord.
    // Cloning it is a refcount bump, so the happy path allocates nothing.
    let route = req.extensions().get::<MatchedPath>().cloned();
    let resp = next.run(req).await;
    if resp.status().is_server_error() {
        let route = route.as_ref().map_or("(unmatched)", MatchedPath::as_str);
        record(
            classify(&method, route),
            &format!("{method} {route} → {}", resp.status().as_u16()),
        );
    }
    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: u64 = 1_788_000_000; // some hour in 2026

    #[test]
    fn counts_by_kind_inside_the_window() {
        let mut w = Window::new();
        w.record(T0, Kind::Upload, "a");
        w.record(T0 + 60, Kind::Upload, "b");
        w.record(T0 + 5 * 3600, Kind::Delete, "c");
        let s = w.snapshot(T0 + 6 * 3600);
        assert_eq!(s.counts, [2, 0, 1, 0]);
        assert_eq!(s.latest().map(|(_, w)| w.as_str()), Some("c"));
    }

    #[test]
    fn old_hours_fall_off() {
        let mut w = Window::new();
        w.record(T0, Kind::Download, "gone by tomorrow");
        assert_eq!(w.snapshot(T0 + 23 * 3600).count(Kind::Download), 1);
        let later = w.snapshot(T0 + 25 * 3600);
        assert_eq!(later.count(Kind::Download), 0);
        assert!(later.latest().is_none(), "a day-old 'last' is not news");
    }

    #[test]
    fn a_reused_slot_starts_from_zero() {
        let mut w = Window::new();
        w.record(T0, Kind::Other, "yesterday");
        // Same slot (24 h later), different hour: the old count must not leak in.
        w.record(T0 + 24 * 3600, Kind::Other, "today");
        assert_eq!(w.snapshot(T0 + 24 * 3600).count(Kind::Other), 1);
    }

    #[test]
    fn last_line_is_capped() {
        let mut w = Window::new();
        w.record(T0, Kind::Other, &"x".repeat(1000));
        let s = w.snapshot(T0);
        assert_eq!(s.latest().unwrap().1.chars().count(), LAST_MAX_CHARS);
    }

    #[test]
    fn routes_sort_into_the_right_kind() {
        use Kind::*;
        let cases = [
            (Method::DELETE, "/v1/cloud/saves/:save_id", Delete),
            (
                Method::DELETE,
                "/v1/cloud/saves/:save_id/versions/:version",
                Delete,
            ),
            (Method::DELETE, "/v1/me", Delete),
            (Method::POST, "/v1/cloud/cas/init", Upload),
            (Method::POST, "/v1/cloud/saves", Upload),
            (
                Method::POST,
                "/v1/cloud/saves/:save_id/versions/:version/commit",
                Upload,
            ),
            (
                Method::POST,
                "/v1/cloud/saves/:save_id/versions/:version/cas/commit",
                Upload,
            ),
            (Method::GET, "/v1/cloud/sync", Download),
            (Method::GET, "/v1/cloud/blob/:token", Download),
            (
                Method::GET,
                "/v1/cloud/saves/:save_id/versions/:version/download",
                Download,
            ),
            (
                Method::GET,
                "/v1/cloud/saves/:save_id/versions/:version/manifest",
                Download,
            ),
            (Method::PATCH, "/v1/cloud/saves/:save_id", Other),
            (Method::POST, "/v1/presence/heartbeat", Other),
            (Method::GET, "/v1/cloud/saves/:save_id/versions", Other),
        ];
        for (m, route, want) in cases {
            assert_eq!(classify(&m, route), want, "{m} {route}");
        }
    }

    /// Through a real router: a 5xx is counted under its route template (no
    /// ids), and a 4xx is not counted at all. The only test that touches the
    /// global window, so the before/after deltas are its own.
    #[tokio::test]
    async fn the_middleware_counts_5xx_and_ignores_4xx() {
        use axum::{
            body::Body,
            http::{Request as HttpRequest, StatusCode},
            routing::{delete, post},
            Router,
        };
        use tower::ServiceExt;

        let app = Router::new()
            .route(
                "/v1/cloud/saves/:save_id",
                delete(|| async { StatusCode::INTERNAL_SERVER_ERROR }),
            )
            .route(
                "/v1/cloud/cas/init",
                post(|| async { StatusCode::PAYMENT_REQUIRED }),
            )
            .layer(axum::middleware::from_fn(track));

        let before = snapshot();
        let req = |m: &str, uri: &str| {
            HttpRequest::builder()
                .method(m)
                .uri(uri)
                .body(Body::empty())
                .unwrap()
        };
        let r = app
            .clone()
            .oneshot(req("DELETE", "/v1/cloud/saves/abc-123"))
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let r = app
            .oneshot(req("POST", "/v1/cloud/cas/init"))
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::PAYMENT_REQUIRED);

        let after = snapshot();
        assert_eq!(after.count(Kind::Delete), before.count(Kind::Delete) + 1);
        assert_eq!(after.count(Kind::Upload), before.count(Kind::Upload));
        let (_, what) = after.last[Kind::Delete as usize].clone().unwrap();
        assert_eq!(what, "DELETE /v1/cloud/saves/:save_id → 500");
        assert!(!what.contains("abc-123"));
    }
}
