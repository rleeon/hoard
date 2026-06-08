//! Cloud-pull poller — the live "is anything new?" loop.
//!
//! Pairs with `commands::automatic`. The automatic scheduler does the heavy
//! lifting (scan-library + backup-stale sweep) on the hourly scale; this
//! poller hits `/v1/cloud/sync` every `prefs.cloud_poll_interval_secs`
//! (default 10 s) so the LiveStatus widget feels instant and ActivityFeed
//! shows real-time pull events the moment another device uploads.
//!
//! Decoupling the two cadences was an ADR-0016 call. The manifest endpoint
//! returns <5 KB and is explicitly excluded from the bandwidth quota
//! (`hoard-server::cloud::routes::sync` — no `bandwidth::check` call), so a
//! 10-second cadence is free in money and bytes.
//!
//! What this poller deliberately does **not** do: it never overwrites a
//! local save file. The "remote is newer, pull it" pathway still goes
//! through the agent's auto-restore sweep (triggered by the automatic
//! scheduler on its hourly tick, or by flipping the toggle off→on). The
//! poller's role is to make the UI honest about server state — not to
//! race the user's keyboard. Forcing pulls from a 10-second loop would
//! risk overwriting an active edit; ADR 0016 spells out the trade.
//!
//! Events emitted (Tauri):
//! - `agent://cloud-pull-started`  — fired right before the HTTP GET.
//! - `agent://cloud-pull-completed { count, new_versions, bytes }`
//!   `count` = total saves in manifest, `new_versions` = number where the
//!   remote version_num is strictly greater than the last manifest seen
//!   in-memory, `bytes` = sum of `latest_size_bytes` of the new ones
//!   (informational only — nothing was downloaded).
//! - `agent://quota-reached { reset_in_seconds, plan }` — emitted on a
//!   429 response from the manifest endpoint. Today the manifest is
//!   excluded from the limiter so this should be a no-op in practice;
//!   we keep the branch in case the policy changes.
//! - `agent://offline` — emitted on transport errors (DNS, TCP, TLS).
//!   The LiveStatus widget downgrades to the red "Server unreachable"
//!   dot until the next successful pull.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use tokio::task::JoinHandle;
use tokio::time::interval;

/// Managed singleton holding the currently-active poller task, if any.
/// Mirrors `AutomaticScheduler`. We mutate the inner `Option<JoinHandle>`
/// to swap tasks on prefs changes.
#[derive(Default)]
pub struct CloudPullScheduler {
    handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    /// Latest seen `(save_id → version_num)` map. Used to detect deltas
    /// between polls. Lives in the scheduler (not on disk) because a
    /// fresh session is correct: the first poll just emits "0 new" and
    /// subsequent polls show real deltas.
    seen: Arc<Mutex<Vec<ManifestSeenEntry>>>,
    /// Single-flight coalescing gate shared by the timed poller and every
    /// Realtime `kick()`. Without it, a catch-up backup sweep that touches
    /// N saves makes Supabase push N near-simultaneous `saves` UPDATEs, and
    /// each one used to spawn its own `/v1/cloud/sync` — N concurrent pulls
    /// that race on token refresh, so a single transient timeout among them
    /// emitted `agent://offline` and the LiveStatus dot flapped to "agente
    /// apagado". With the gate, at most one pull runs at a time and a burst
    /// of kicks collapses into a single follow-up pull.
    gate: Arc<Mutex<PullGate>>,
}

/// Coalescing state for [`CloudPullScheduler::gate`].
#[derive(Default)]
struct PullGate {
    /// A pull is currently executing.
    running: bool,
    /// A pull was requested while one was already running; run exactly one
    /// more pass when the current one finishes (collapses any number of
    /// concurrent kicks into a single re-run).
    rerun: bool,
}

#[derive(Debug, Clone)]
struct ManifestSeenEntry {
    save_id: String,
    version_num: i64,
}

#[derive(Debug, Deserialize)]
struct ManifestEntry {
    save_id: String,
    #[allow(dead_code)]
    game_slug: String,
    #[allow(dead_code)]
    label: String,
    latest_version_num: i64,
    latest_size_bytes: i64,
}

#[derive(Debug, Deserialize)]
struct Manifest {
    saves: Vec<ManifestEntry>,
}

#[derive(Debug, Serialize, Clone)]
struct CloudPullCompleted {
    /// Total saves in the manifest. The watcher_count for cloud.
    count: usize,
    /// How many of those have a `latest_version_num` strictly greater
    /// than what we saw last time. First poll always reports 0.
    new_versions: usize,
    /// Sum of `latest_size_bytes` across the newly-versioned saves —
    /// informational only, nothing was downloaded.
    bytes: i64,
}

#[derive(Debug, Serialize, Clone)]
struct QuotaReached {
    reset_in_seconds: u32,
    plan: String,
}

/// Cancel any in-flight poller and start a fresh one ticking every
/// `interval_secs`. Safe to call repeatedly. The poller reads the cloud
/// session from disk on each tick (cheap) so a logout / login round-trip
/// without a restart picks up the new token transparently.
pub fn start(app: &AppHandle, interval_secs: u32) {
    let scheduler = app.state::<CloudPullScheduler>();
    {
        let mut slot = scheduler.handle.lock().unwrap();
        if let Some(prev) = slot.take() {
            prev.abort();
        }
    }

    let secs = interval_secs.clamp(5, 300) as u64;
    let period = Duration::from_secs(secs);
    let app_for_task = app.clone();
    let seen = scheduler.seen.clone();
    let gate = scheduler.gate.clone();
    let new_handle = tokio::task::spawn(async move {
        tracing::info!(interval_secs = secs, "cloud-pull poller: started");

        // First tick fires immediately so the user sees activity on
        // sign-in without waiting `interval_secs`. The built-in
        // zero-delay first tick of `tokio::time::interval` is the right
        // shape — we don't manually emit before the loop.
        let mut ticker = interval(period);
        loop {
            ticker.tick().await;
            guarded_pull(&app_for_task, &seen, &gate).await;
        }
    });

    let mut slot = scheduler.handle.lock().unwrap();
    *slot = Some(new_handle);
}

/// Fire a single manifest pull immediately, off the regular cadence.
///
/// Used by the Realtime push (`cloud_realtime`): when another device commits
/// a new save version, Supabase pushes a `saves` UPDATE and we refresh state
/// in ~1 s instead of waiting for the next `interval_secs` tick. Reuses the
/// scheduler's `seen` map so delta detection stays consistent with the timed
/// poll. No-op when signed out (`run_one_pull` bails on missing creds).
pub fn kick(app: &AppHandle) {
    let scheduler = app.state::<CloudPullScheduler>();
    let seen = scheduler.seen.clone();
    let gate = scheduler.gate.clone();
    let app = app.clone();
    tokio::task::spawn(async move {
        guarded_pull(&app, &seen, &gate).await;
    });
}

/// Run a pull behind the single-flight [`PullGate`]. If a pull is already in
/// flight this only flags a re-run and returns immediately; the in-flight
/// caller drains that flag with exactly one extra pass when it finishes. A
/// burst of kicks (e.g. a backup sweep touching every save) therefore costs
/// at most two `/v1/cloud/sync` requests instead of one per save.
async fn guarded_pull(
    app: &AppHandle,
    seen: &Arc<Mutex<Vec<ManifestSeenEntry>>>,
    gate: &Arc<Mutex<PullGate>>,
) {
    {
        let mut g = gate.lock().unwrap();
        if g.running {
            // Someone is already pulling; ask them to do one more pass with
            // the freshest server state and bail.
            g.rerun = true;
            return;
        }
        g.running = true;
    }

    loop {
        run_one_pull(app, seen).await;
        let mut g = gate.lock().unwrap();
        if g.rerun {
            g.rerun = false;
            // Loop again to honour the kick(s) that arrived mid-flight.
        } else {
            g.running = false;
            return;
        }
    }
}

/// Abort the running poller. No-op when nothing is scheduled.
pub fn stop(app: &AppHandle) {
    let scheduler = app.state::<CloudPullScheduler>();
    let mut slot = scheduler.handle.lock().unwrap();
    if let Some(prev) = slot.take() {
        prev.abort();
        tracing::info!("cloud-pull poller: stopped");
    }
    // Clear the seen-map too: a fresh login starts from 0 known versions.
    scheduler.seen.lock().unwrap().clear();
}

/// Restart the poller if a cloud session exists. Used by
/// `set_cloud_poll_interval` and by `cloud_complete_login` so the cadence
/// adjusts live. No-op when the user is signed out.
pub fn restart_if_signed_in(app: &AppHandle, interval_secs: u32) {
    let signed_in = {
        let st = app.state::<crate::state::AppState>();
        let has = st.cloud_account.lock().unwrap().is_some();
        has
    };
    if signed_in {
        start(app, interval_secs);
    }
}

/// Boot-time rehydration: if a cloud session is present on disk, start
/// the poller using the saved interval. Called from `lib.rs::setup`.
pub async fn restart_if_enabled(app: &AppHandle) -> anyhow::Result<()> {
    let signed_in = {
        let st = app.state::<crate::state::AppState>();
        let has = st.cloud_account.lock().unwrap().is_some();
        has
    };
    if !signed_in {
        return Ok(());
    }
    let (prefs, _) = hoard_agent::prefs::Prefs::load_default()?;
    start(app, prefs.cloud_poll_interval_secs);
    Ok(())
}

async fn run_one_pull(app: &AppHandle, seen: &Arc<Mutex<Vec<ManifestSeenEntry>>>) {
    let _ = app.emit("agent://cloud-pull-started", ());

    let creds = match crate::commands::cloud::load_active_creds() {
        Ok(Some(c)) => c,
        Ok(None) => {
            // Session disappeared between polls (user logged out). Quiet
            // exit; the logout path called `stop()` separately.
            return;
        }
        Err(e) => {
            tracing::warn!(error = %e, "cloud-pull: couldn't read session");
            let _ = app.emit("agent://offline", ());
            return;
        }
    };

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent(concat!("hoard-desktop/", env!("CARGO_PKG_VERSION")))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "cloud-pull: HTTP client build failed");
            return;
        }
    };

    let url = format!("{}/v1/cloud/sync", creds.server_url);
    let mut access_token = creds.access_token.clone();
    let mut resp = match client.get(&url).bearer_auth(&access_token).send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::debug!(error = %e, "cloud-pull: network error");
            let _ = app.emit("agent://offline", ());
            return;
        }
    };

    // The access token is a short-lived Supabase JWT. When it expires the
    // sync endpoint answers 401 — which previously surfaced as a permanent
    // "server down" dot. Renew it with the refresh token and retry once so the
    // poller keeps working across the token's lifetime (and across restarts).
    if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
        match crate::commands::cloud::refresh_active_session().await {
            Ok(fresh) => {
                access_token = fresh.access_token;
                match client.get(&url).bearer_auth(&access_token).send().await {
                    Ok(r) => resp = r,
                    Err(e) => {
                        tracing::debug!(error = %e, "cloud-pull: network error after refresh");
                        let _ = app.emit("agent://offline", ());
                        return;
                    }
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "cloud-pull: token refresh failed");
                let _ = app.emit("agent://offline", ());
                return;
            }
        }
    }

    let status = resp.status();
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        let retry_after = resp
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(60);
        let plan = creds.plan.clone().unwrap_or_else(|| "free".to_string());
        let _ = app.emit(
            "agent://quota-reached",
            QuotaReached {
                reset_in_seconds: retry_after,
                plan,
            },
        );
        return;
    }
    if !status.is_success() {
        tracing::warn!(status = %status, "cloud-pull: non-2xx response");
        let _ = app.emit("agent://offline", ());
        return;
    }

    let body = match resp.text().await {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!(error = %e, "cloud-pull: couldn't read response body");
            return;
        }
    };
    let manifest: Manifest = match serde_json::from_str(&body) {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!(error = %e, "cloud-pull: couldn't parse manifest");
            return;
        }
    };

    let (new_versions, new_bytes, advanced_ids) = {
        let mut seen_guard = seen.lock().unwrap();
        let mut new_versions: usize = 0;
        let mut new_bytes: i64 = 0;
        let mut advanced_ids: Vec<String> = Vec::new();
        for entry in &manifest.saves {
            let prev_version = seen_guard
                .iter()
                .find(|s| s.save_id == entry.save_id)
                .map(|s| s.version_num);
            match prev_version {
                Some(prev) if entry.latest_version_num > prev => {
                    new_versions += 1;
                    new_bytes += entry.latest_size_bytes.max(0);
                    advanced_ids.push(entry.save_id.clone());
                }
                None => {
                    // First time we see this save in this session. We
                    // intentionally do *not* count it as "new" — the
                    // user might have logged in to a populated account
                    // and reporting "247 new versions!" right after
                    // sign-in is noisy. The seed pass just records the
                    // baseline; deltas from the next poll onward are
                    // what surface to LiveStatus.
                }
                _ => {}
            }
        }
        // Replace the seen-map wholesale: saves removed server-side
        // (other-device deletes) shouldn't linger.
        seen_guard.clear();
        for entry in &manifest.saves {
            seen_guard.push(ManifestSeenEntry {
                save_id: entry.save_id.clone(),
                version_num: entry.latest_version_num,
            });
        }
        (new_versions, new_bytes, advanced_ids)
    };

    let _ = app.emit(
        "agent://cloud-pull-completed",
        CloudPullCompleted {
            count: manifest.saves.len(),
            new_versions,
            bytes: new_bytes,
        },
    );

    // Sync global: the poller is the cheap "is this device outdated?" detector.
    // When it's on and a save advanced server-side, ask the agent to pull it
    // right now — "en el momento", even if the game is running. This is the
    // low-latency complement to the agent's own sweep (which would catch the
    // same delta within its cooldown). Off by default and version-gated on the
    // agent side, so this is a no-op cost when nothing actually changed.
    if !advanced_ids.is_empty() {
        let global_sync = hoard_agent::prefs::Prefs::load_default()
            .map(|(p, _)| p.global_sync)
            .unwrap_or(false);
        if global_sync {
            let handle = app
                .try_state::<crate::state::AppState>()
                .and_then(|s| s.agent.lock().unwrap().clone());
            if let Some(h) = handle {
                for id in advanced_ids {
                    if let Err(e) = h.force_restore(id).await {
                        tracing::warn!(
                            error = %e,
                            "cloud-pull: couldn't request force-restore for sync global"
                        );
                    }
                }
            }
        }
    }
}
