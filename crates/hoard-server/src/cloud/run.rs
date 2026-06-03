//! Cloud-mode entry point. Invoked by `main` when
//! `database.backend = "postgres"`.

use crate::cloud::{
    auth::{require_cloud_auth, JwksCache},
    bandwidth, db, polar, r2,
    routes::{logs as log_routes, me, saves, sync as sync_routes},
    state::CloudState,
    webhooks,
};
use crate::config::Config;
use anyhow::Result;
use axum::{
    extract::State,
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
    //     Spawned as a detached task — failures just `warn!` and the
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
    {
        let pool = pool.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_secs(3600));
            tick.tick().await; // skip the immediate first tick
            loop {
                tick.tick().await;
                let res = sqlx::query(
                    "DELETE FROM client_logs WHERE received_at < now() - interval '14 days'",
                )
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

    // 5. Build routers.
    let authed = Router::new()
        .route("/v1/me", get(me::get_me).delete(me::delete_me))
        .route("/v1/me/export", post(me::create_export_job))
        .route("/v1/cloud/saves", post(saves::init_upload))
        .route(
            "/v1/cloud/saves/:save_id/versions/:version/commit",
            post(saves::commit_upload),
        )
        .route(
            "/v1/cloud/saves/:save_id/versions/:version/download",
            get(saves::download),
        )
        .route("/v1/cloud/sync", get(sync_routes::manifest))
        // Client diagnostic-log ingest (INFO+ only). Smaller body cap than
        // save uploads — applied per-route.
        .route(
            "/v1/cloud/logs",
            post(log_routes::ingest).layer(axum::extract::DefaultBodyLimit::max(
                crate::routes::logs::MAX_BATCH_BYTES,
            )),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            require_cloud_auth,
        ));

    let public = Router::new()
        .route("/v1/webhooks/lemonsqueezy", post(webhooks::handle))
        .route("/v1/webhooks/polar", post(polar::handle))
        // Health is *also* available unauthed in cloud mode so Fly can probe it.
        .route("/v1/health", get(cloud_health));

    // Browser CORS: the marketing site (hoard.services) and the account
    // page fetch this API cross-origin. Without these headers the browser
    // blocks reading the response even on a healthy 200, which is what made
    // the status dot read "degraded". No cookies — auth is a Bearer token —
    // so credentials stay off.
    let cors = CorsLayer::new()
        .allow_origin([
            HeaderValue::from_static("https://hoard.services"),
            HeaderValue::from_static("http://localhost:5173"),
            HeaderValue::from_static("http://localhost:4173"),
        ])
        .allow_methods([Method::GET, Method::POST, Method::DELETE])
        .allow_headers([header::AUTHORIZATION, header::ACCEPT, header::CONTENT_TYPE]);

    let app = Router::new()
        .merge(public)
        .merge(authed)
        .with_state(state.clone())
        .layer(cors);

    let addr: SocketAddr = format!("{}:{}", cfg.server.host, cfg.server.port).parse()?;
    info!(%addr, "cloud mode listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.ok();
            info!("received ctrl-c, shutting down");
        })
        .await?;
    Ok(())
}

async fn cloud_health(State(state): State<CloudState>) -> axum::Json<HealthBody> {
    // Reaching this handler means the server process is up. "degraded" is
    // reserved for "up but a dependency is failing" — here, Postgres. If the
    // process itself were down the request wouldn't connect at all, which the
    // client reads as a hard outage (red), not degraded (amber).
    //
    // The DB probe is bounded by a 2s timeout: Fly's health check hits this
    // endpoint every 15s with a 5s budget, so a hung Postgres must never make
    // the handler block past that — otherwise Fly would crash-loop a machine
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
        log_min_level: "info",
    })
}

#[derive(serde::Serialize)]
struct HealthBody {
    status: &'static str,
    version: &'static str,
    mode: &'static str,
    /// Minimum log level cloud accepts for client-log ingest — INFO. The
    /// client reads this on connect and filters at source.
    log_min_level: &'static str,
}
