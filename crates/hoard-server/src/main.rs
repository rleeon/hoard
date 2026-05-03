use anyhow::Result;
use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use clap::Parser;
use hoard_server::{
    config::{Config, LogFormat},
    db,
};
use std::{net::SocketAddr, path::PathBuf, sync::Arc, time::Instant};
use tracing::info;

use hoard_server::auth::require_auth;
use hoard_server::cleanup;
use hoard_server::routes::{
    auth as auth_routes, games as game_routes, health, saves as save_routes,
    snapshots as snap_routes,
};

#[derive(Parser)]
#[command(name = "hoard-server", version, about = "Hoard save-sync server")]
struct Args {
    #[arg(long, default_value = "/etc/hoard/config.toml")]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let cfg = Config::load(&args.config)?;

    init_logging(&cfg);

    info!(
        version = env!("CARGO_PKG_VERSION"),
        host = %cfg.server.host,
        port = cfg.server.port,
        data_dir = %cfg.storage.data_dir.display(),
        "starting hoard-server"
    );

    // Ensure data subdirectories exist
    for dir in &["data", "tmp", "trash"] {
        tokio::fs::create_dir_all(cfg.storage.data_dir.join(dir)).await?;
    }

    let pool = db::connect(&cfg.database.url, cfg.database.max_connections).await?;
    db::run_migrations(&pool).await?;

    let state = Arc::new(health::ServerState {
        pool: pool.clone(),
        config: cfg.clone(),
        start_time: Instant::now(),
    });

    // Routes that require auth
    let authed = Router::new()
        .route("/v1/auth/whoami", get(auth_routes::whoami))
        // Games
        .route("/v1/games", get(game_routes::list))
        .route("/v1/games/:slug", get(game_routes::get_one))
        // Saves
        .route(
            "/v1/saves",
            get(save_routes::list).post(save_routes::create),
        )
        .route(
            "/v1/saves/:id",
            get(save_routes::get_one)
                .patch(save_routes::patch)
                .delete(save_routes::delete),
        )
        // Snapshots
        .route(
            "/v1/saves/:save_id/snapshots",
            get(snap_routes::list).post(snap_routes::create),
        )
        .route(
            "/v1/saves/:save_id/snapshots/:version",
            get(snap_routes::detail).delete(snap_routes::soft_delete),
        )
        .route(
            "/v1/saves/:save_id/snapshots/:version/download",
            get(snap_routes::download),
        )
        .route(
            "/v1/saves/:save_id/snapshots/:version/restore",
            post(snap_routes::restore),
        )
        .layer(axum::extract::DefaultBodyLimit::max(
            (cfg.storage.max_snapshot_size_mb as usize) * 1024 * 1024 + 16 * 1024 * 1024,
        ))
        .layer(middleware::from_fn_with_state(pool.clone(), require_auth));

    // Spawn periodic cleanup task
    let cleanup_pool = pool.clone();
    let cleanup_data = cfg.storage.data_dir.clone();
    let cleanup_tmp_h = cfg.retention.tmp_cleanup_hours;
    let cleanup_trash_d = cfg.retention.trash_retention_days;
    tokio::spawn(async move {
        cleanup::run_periodic(cleanup_pool, cleanup_data, cleanup_tmp_h, cleanup_trash_d).await;
    });

    let app = Router::new()
        .route("/v1/health", get(health::handler))
        .merge(authed)
        .with_state(state);

    let addr: SocketAddr = format!("{}:{}", cfg.server.host, cfg.server.port).parse()?;
    info!(%addr, "listening");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.ok();
            info!("received ctrl-c, shutting down");
        })
        .await?;

    Ok(())
}

fn init_logging(cfg: &Config) {
    use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
    let filter = EnvFilter::try_new(&cfg.logging.level).unwrap_or_else(|_| EnvFilter::new("info"));
    match cfg.logging.format {
        LogFormat::Json => tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().json())
            .init(),
        LogFormat::Pretty => tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().pretty())
            .init(),
    }
}
