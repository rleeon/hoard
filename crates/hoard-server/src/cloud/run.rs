//! Cloud-mode entry point. Invoked by `main` when
//! `database.backend = "postgres"`.

use crate::cloud::{
    abandoned, account_purge, archive,
    auth::{require_active_account, require_cloud_auth, JwksCache},
    bandwidth, compress, db, export, polar, pollguard, r2,
    routes::{
        blob_proxy, checkout, device as device_routes, entitlements as ent_routes,
        logs as log_routes, me, notifications as notification_routes, playtime as playtime_routes,
        saves, sync as sync_routes,
    },
    state::CloudState,
};
use crate::config::Config;
use anyhow::Result;
use axum::{
    extract::{DefaultBodyLimit, State},
    http::{header, HeaderValue, Method},
    middleware,
    routing::{get, post},
    Router,
};
use std::{net::SocketAddr, sync::Arc, time::Duration, time::Instant};
use tower_http::cors::CorsLayer;
use tracing::info;

pub async fn run(cfg: Config) -> Result<()> {
    let cloud_cfg = cfg
        .cloud
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("cloud mode requires [cloud] config"))?;

    // 1. Connect Postgres.
    let pool = db::connect(&cfg.database.url, cfg.database.max_connections).await?;
    db::run_migrations(&pool).await?;

    // 2. Prime JWKS cache + schedule refresh.
    let jwks = JwksCache::new(
        cloud_cfg.supabase_jwks_url.clone(),
        cloud_cfg.supabase_audience.clone(),
        if cloud_cfg.supabase_issuer.is_empty() {
            None
        } else {
            Some(cloud_cfg.supabase_issuer.clone())
        },
    )
    .await?;
    jwks.clone()
        .spawn_refresh(Duration::from_secs(cloud_cfg.jwks_refresh_secs));

    // 3. R2 client.
    let r2_store = Arc::new(r2::R2Store::from_config(&cloud_cfg.r2).await?);

    // 4. Wire shared state.
    let state = CloudState {
        pool: pool.clone(),
        config: cfg.clone(),
        jwks,
        r2: r2_store,
        start_time: Instant::now(),
    };

    // 4b. Bandwidth bucket cleanup. 10-minute cadence is far below the
    //     1-hour cutoff, so a missed tick after a deploy can't let the
    //     table grow more than ~1.5h before the next run trims it back.
    //     Spawned as a detached task: failures just `warn!` and the
    //     next tick retries; not worth crashing the server over.
    {
        let pool = pool.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_secs(600));
            // The first tick fires immediately; skip it so we don't double
            // up with startup work.
            tick.tick().await;
            loop {
                tick.tick().await;
                match bandwidth::cleanup_old(&pool).await {
                    Ok(n) if n > 0 => {
                        tracing::debug!(rows = n, "bandwidth: cleaned old buckets");
                    }
                    Ok(_) => {}
                    Err(e) => {
                        tracing::warn!(error = %e, "bandwidth: cleanup failed");
                    }
                }
            }
        });
    }

    // 4c. Client-log retention. Diagnostic logs are kept 14 days; an hourly
    //     sweep deletes anything older. Detached task: failures `warn!` and
    //     the next tick retries.
    //
    //     The `EXEMPT_TARGETS` live 180 days rather than 14. They are not
    //     diagnostics: they are the two signals collected on purpose (detection
    //     contradictions and Hoard Screen usage) and the two questions they
    //     answer are longitudinal ("is detection getting better?", "is overlay
    //     use growing?"). With the short prune the panel showed a rolling
    //     two-week window and passed it off as the total, which is the worst way
    //     to be wrong: no error, no gap, and the trend erased. Volume is not the
    //     problem, at a couple of rows per session against the
    //     ~15.000 de log corriente.
    {
        let pool = pool.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_secs(3600));
            tick.tick().await; // skip the immediate first tick
            loop {
                tick.tick().await;
                let res = sqlx::query(
                    "DELETE FROM client_logs
                      WHERE received_at < now() - interval '14 days'
                        AND NOT (target = ANY($1) AND received_at > now() - interval '180 days')",
                )
                .bind(hoard_core::wire::EXEMPT_TARGETS)
                .execute(&pool)
                .await;
                match res {
                    Ok(r) if r.rows_affected() > 0 => {
                        tracing::debug!(rows = r.rows_affected(), "client logs: pruned expired");
                    }
                    Ok(_) => {}
                    Err(e) => {
                        tracing::warn!(error = %e, "client logs: prune failed");
                    }
                }
            }
        });
    }

    // 4d. Hard-purge of soft-deleted accounts past their 30-day grace. Daily
    //     cadence; deletes R2 objects then cascades the DB rows. Detached like
    //     the sweepers above.
    account_purge::spawn(state.clone());
    // Picks up after uploads that started and never committed: their manifest
    // rows, and the blobs they left in the bucket with nothing referencing them.
    abandoned::spawn(state.clone());

    // Fulfils `export_jobs` rows: builds the ZIP, uploads to R2, emails the
    // link, and expires old exports.
    export::spawn(state.clone());

    // At-rest blob compression sweep (no-op unless `[cloud.compression]`
    // enables it). Rewrites old raw blobs as zstd in place; quota and
    // everything user-visible keep counting raw bytes.
    compress::spawn(state.clone());

    // 4e. Hard-purge of archived games ("caja negra") past their 7-day grace:
    //     deletes the save rows and GCs the frozen R2 blobs whose window
    //     elapsed. Daily cadence, detached like the sweepers above.
    archive::spawn(state.clone());

    // 4f. Device-pairing sweep. Approved/expired rows are deleted inline on
    //     poll, but a pairing that's started and never polled (or approved and
    //     never collected) would linger. Drop anything past its expiry every
    //     10 minutes. Detached like the sweepers above.
    {
        let pool = pool.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_secs(600));
            tick.tick().await; // skip the immediate first tick
            loop {
                tick.tick().await;
                let res = sqlx::query("DELETE FROM device_pairings WHERE expires_at < now()")
                    .execute(&pool)
                    .await;
                match res {
                    Ok(r) if r.rows_affected() > 0 => {
                        tracing::debug!(
                            rows = r.rows_affected(),
                            "device pairings: pruned expired"
                        );
                    }
                    Ok(_) => {}
                    Err(e) => tracing::warn!(error = %e, "device pairings: prune failed"),
                }
            }
        });
    }

    // 5. Build routers.
    //
    // `authed_always` needs only a valid token: these routes must work even
    // while the account is frozen (soft-deleted) so the user can see the
    // countdown and reactivate. Everything else is on `authed`, which layers
    // `require_active_account` on top of auth so a scheduled-for-deletion
    // account can't read or write its data during the 30-day grace.

    // Per-device cap on the polling endpoints (see `pollguard`). Attached
    // with `route_layer` so it runs after `require_cloud_auth` and can key
    // by user. Respects the master rate-limit switch.
    let poll_guard = pollguard::PollGuard::new(
        if cfg.server.rate_limit.enabled {
            cfg.server.rate_limit.poll_per_minute
        } else {
            0
        },
        cfg.server.rate_limit.poll_burst,
    );
    let guarded = |class: &'static str| {
        let g = poll_guard.clone();
        middleware::from_fn(move |req, next| pollguard::guard(g.clone(), class, req, next))
    };
    let authed_always = Router::new()
        .route("/v1/me", get(me::get_me).delete(me::delete_me))
        .route("/v1/me/reactivate", post(me::reactivate_me))
        // It goes with the always-on routes rather than the guarded ones:
        // accepting the Terms is the first thing a client does on sign-in, and it
        // has to be able to even with the rest of the account frozen.
        .route("/v1/me/terms", get(me::get_terms).post(me::accept_terms))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            require_cloud_auth,
        ));

    let authed = Router::new()
        .route(
            "/v1/me/export",
            get(me::get_export_status).post(me::create_export_job),
        )
        // Per-user cap on stored versions per save (prunes immediately).
        .route(
            "/v1/me/max-versions",
            axum::routing::put(me::set_max_versions),
        )
        .route(
            "/v1/devices",
            get(me::list_devices).route_layer(guarded("devices")),
        )
        // Presence keepalive for the Eye panel. Lives off the `/devices/:id`
        // param path on purpose: matchit 0.7 (axum 0.7) panics on a static
        // segment colliding with a param at the same position, the same caveat
        // as `/saves/cas` below.
        .route(
            "/v1/presence/heartbeat",
            post(me::heartbeat).route_layer(guarded("heartbeat")),
        )
        .route("/v1/devices/:id", axum::routing::delete(me::delete_device))
        // Operator broadcasts for the bell panel. List is read-only; the
        // dismiss endpoint records a per-user, cross-device dismissal (see
        // cloud/routes/notifications.rs).
        .route(
            "/v1/notifications",
            get(notification_routes::list).route_layer(guarded("notifications")),
        )
        .route(
            "/v1/notifications/:id/dismiss",
            post(notification_routes::dismiss),
        )
        .route("/v1/cloud/checkout", post(checkout::create_checkout))
        // Phone confirms a CLI pairing. Authed: mints a session for *this*
        // signed-in user and hands it to the waiting headless device.
        .route("/v1/cloud/device/approve", post(device_routes::approve))
        .route("/v1/cloud/entitlements", get(ent_routes::get_entitlements))
        .route(
            "/v1/cloud/features/:feature/activate",
            post(ent_routes::activate_feature),
        )
        .route("/v1/cloud/saves", post(saves::init_upload))
        // Content-addressed init lives off the `/saves/:save_id` param path on
        // purpose: matchit 0.7 (axum 0.7) panics on a static `/saves/cas`
        // colliding with the `:save_id` param at the same position.
        .route("/v1/cloud/cas/init", post(saves::cas_init))
        .route(
            "/v1/cloud/saves/:save_id",
            axum::routing::delete(saves::delete_save).patch(saves::rename_save),
        )
        // "Caja negra": archive a game to free quota immediately (frozen,
        // downloadable for 7 days, then purged), and reactivate it (re-upload
        // from local) after upgrading.
        .route(
            "/v1/cloud/saves/:save_id/archive",
            post(archive::archive_handler),
        )
        .route(
            "/v1/cloud/saves/:save_id/reactivate",
            post(archive::reactivate_handler),
        )
        // Per-game freeable footprint + quota figures, for the "free space"
        // dialog.
        .route("/v1/cloud/storage/games", get(archive::storage_games))
        .route(
            "/v1/cloud/saves/:save_id/versions",
            get(saves::list_versions),
        )
        .route(
            "/v1/cloud/saves/:save_id/versions/:version",
            axum::routing::delete(saves::delete_version),
        )
        .route(
            "/v1/cloud/saves/:save_id/versions/:version/commit",
            post(saves::commit_upload),
        )
        .route(
            "/v1/cloud/saves/:save_id/versions/:version/cas/commit",
            post(saves::cas_commit),
        )
        .route(
            "/v1/cloud/saves/:save_id/versions/:version/manifest",
            get(saves::version_manifest),
        )
        .route(
            "/v1/cloud/saves/:save_id/versions/:version/download",
            get(saves::download),
        )
        .route(
            "/v1/cloud/sync",
            get(sync_routes::manifest).route_layer(guarded("sync")),
        )
        .route(
            "/v1/cloud/playtime",
            get(playtime_routes::aggregate).post(playtime_routes::upload),
        )
        // Client diagnostic-log ingest (INFO+ only). Smaller body cap than
        // save uploads, applied per route.
        .route(
            "/v1/cloud/logs",
            post(log_routes::ingest).layer(axum::extract::DefaultBodyLimit::max(
                crate::routes::logs::MAX_BATCH_BYTES,
            )),
        )
        // Bottom-up: `require_cloud_auth` (outer) runs first and inserts the
        // `CloudUser`, then `require_active_account` (inner) reads it and 403s
        // frozen accounts.
        .layer(middleware::from_fn_with_state(
            state.clone(),
            require_active_account,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            require_cloud_auth,
        ));

    let public = Router::new()
        .route("/v1/webhooks/polar", post(polar::handle))
        // Device pairing for browserless CLI login. Unauthenticated: the CLI
        // has no session yet and is keyed by the secret `device_code`. `start`
        // opens a pairing, `poll` collects the minted session once approved.
        .route("/v1/cloud/device/start", post(device_routes::start))
        .route("/v1/cloud/device/poll", post(device_routes::poll))
        // Decompressing blob download proxy: the HMAC token in the path is
        // the auth, same trust model as a presigned URL.
        .route("/v1/cloud/blob/:token", get(blob_proxy::download))
        // Health is *also* available unauthed in cloud mode so Fly can probe it.
        .route("/v1/health", get(cloud_health));

    // Browser CORS: the marketing site (hoard.services) and the account
    // page fetch this API cross-origin. Without these headers the browser
    // blocks reading the response even on a healthy 200, which is what made
    // the status dot read "degraded". No cookies, since auth is a Bearer token,
    // so credentials stay off.
    let cors = CorsLayer::new()
        .allow_origin([
            HeaderValue::from_static("https://hoard.services"),
            HeaderValue::from_static("http://localhost:5173"),
            HeaderValue::from_static("http://localhost:4173"),
        ])
        .allow_methods([Method::GET, Method::POST, Method::DELETE])
        .allow_headers([header::AUTHORIZATION, header::ACCEPT, header::CONTENT_TYPE]);

    let mut app = Router::new()
        .merge(public)
        .merge(authed_always)
        .merge(authed)
        .with_state(state.clone())
        // The self-hosted router has always set this (`main.rs`, derived from
        // `max_snapshot_size_mb`); the cloud one never did, so it ran on axum's
        // 2 MB default. That is not a limit on save *bytes*, which go straight
        // to R2 over a presigned PUT and never cross this process, but on the
        // `cas/init` manifest, which carries one JSON entry per file: path, its
        // 64-char sha256, size, mtime. At roughly 170 bytes an entry, 2 MB runs
        // out at ~12k files.
        //
        // A save that trips it can never land, and the failure reads as
        // anything but the truth: the extractor's 413 has a plain-text body, so
        // a client that expects Hoard's JSON parses zeros out of it and reports
        // "exceeds the server's per-save size limit", a per-save cap that
        // never ran, on a plan that was never consulted. Our first Pro user
        // spent a day watching Project Zomboid (one file per map chunk, tens of
        // thousands of them) retry hourly against that, upgrading mid-way in
        // the belief that a bigger plan would clear it.
        //
        // 32 MB is ~200k files, well past any real save, and stays under
        // Cloudflare's 100 MB request ceiling on the way in. Not higher: the
        // whole body is buffered and deserialized into owned Strings on a
        // 512 MB machine, so the headroom above is memory, not generosity.
        // `/v1/cloud/logs` keeps its own smaller cap, since a route-level limit is
        // applied inside this one and wins.
        .layer(DefaultBodyLimit::max(32 * 1024 * 1024));

    // Per-IP rate limiting. SmartIpKeyExtractor keys off X-Forwarded-For
    // (Fly/CDN set it), falling back to the connection peer, which the
    // `into_make_service_with_connect_info` below provides.
    //
    // Inside CORS, not outside it. Layers apply from the inside out, so with the
    // limiter outermost its 429s went out with no CORS headers: it short-circuits
    // before reaching the layer that adds them. A browser cannot read a response
    // like that, not even its status code (the web only sees "network error"), so
    // any 429 was undiagnosable from the client at exactly the moment knowing
    // matters most. With CORS outside, the preflight is answered by CORS (and
    // stops counting against the budget, which is also right) and the 429 arrives
    // readable.
    if let Some(rl) = crate::ratelimit::layer(&cfg.server.rate_limit) {
        info!(
            per_second = cfg.server.rate_limit.per_second,
            burst = cfg.server.rate_limit.burst,
            "rate limiting enabled"
        );
        app = app.layer(rl);
    }
    let app = app.layer(cors);

    let addr: SocketAddr = format!("{}:{}", cfg.server.host, cfg.server.port).parse()?;
    info!(%addr, "cloud mode listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        tokio::signal::ctrl_c().await.ok();
        info!("received ctrl-c, shutting down");
    })
    .await?;
    Ok(())
}

async fn cloud_health(State(state): State<CloudState>) -> axum::Json<HealthBody> {
    // Reaching this handler means the server process is up. "degraded" is
    // reserved for "up but a dependency is failing", here meaning Postgres. If the
    // process itself were down the request wouldn't connect at all, which the
    // client reads as a hard outage (red), not degraded (amber).
    //
    // The DB probe is bounded by a 2s timeout: Fly's health check hits this
    // endpoint every 15s with a 5s budget, so a hung Postgres must never make
    // the handler block past that, or Fly would crash-loop a machine
    // that a restart can't fix. Timeout or error → degraded.
    let db_ok = matches!(
        tokio::time::timeout(
            Duration::from_secs(2),
            sqlx::query("SELECT 1").execute(&state.pool),
        )
        .await,
        Ok(Ok(_))
    );
    axum::Json(HealthBody {
        status: if db_ok { "ok" } else { "degraded" },
        version: env!("CARGO_PKG_VERSION"),
        mode: "cloud",
        log_min_level: "warn",
    })
}

#[derive(serde::Serialize)]
struct HealthBody {
    status: &'static str,
    version: &'static str,
    mode: &'static str,
    /// Minimum log level cloud accepts for client-log ingest: WARN. The
    /// client reads this on connect and filters at source. Los
    /// `EXEMPT_TARGETS` (detection contradictions and Screen telemetry) are
    /// exempt on both sides.
    log_min_level: &'static str,
}
