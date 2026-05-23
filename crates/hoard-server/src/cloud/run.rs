//! Cloud-mode entry point. Invoked by `main` when
//! `database.backend = "postgres"`.

use crate::cloud::{
    auth::{require_cloud_auth, JwksCache},
    db, r2,
    routes::{me, saves, sync as sync_routes},
    state::CloudState,
    webhooks,
};
use crate::config::Config;
use anyhow::Result;
use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use std::{net::SocketAddr, sync::Arc, time::Duration, time::Instant};
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
        .layer(middleware::from_fn_with_state(
            state.clone(),
            require_cloud_auth,
        ));

    let public = Router::new()
        .route("/v1/webhooks/lemonsqueezy", post(webhooks::handle))
        // Health is *also* available unauthed in cloud mode so Fly can probe it.
        .route("/v1/health", get(cloud_health));

    let app = Router::new()
        .merge(public)
        .merge(authed)
        .with_state(state.clone());

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

async fn cloud_health() -> axum::Json<HealthBody> {
    axum::Json(HealthBody {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        mode: "cloud",
    })
}

#[derive(serde::Serialize)]
struct HealthBody {
    status: &'static str,
    version: &'static str,
    mode: &'static str,
}
