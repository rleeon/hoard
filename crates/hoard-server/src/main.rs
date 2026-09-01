use anyhow::Result;
use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use clap::{Parser, Subcommand};
use hoard_server::{
    config::{Config, DbBackend, LogFormat},
    db, upgrade,
};
use std::{net::SocketAddr, path::PathBuf, sync::Arc, time::Instant};
use tracing::{info, warn};

use hoard_server::auth::require_auth;
use hoard_server::cleanup;
use hoard_server::routes::{
    admin as admin_routes, auth as auth_routes, cas as cas_routes, devices as device_routes,
    events as event_routes, games as game_routes, health, logs as log_routes,
    overview as overview_routes, panel as panel_routes, playtime as playtime_routes,
    saves as save_routes, session as session_routes, snapshots as snap_routes,
};

#[derive(Parser)]
#[command(name = "hoard-server", version, about = "Hoard save-sync server")]
struct Args {
    /// Path to the TOML config file. Used by `serve` (and by default when no
    /// subcommand is given). Ignored by `upgrade`.
    #[arg(long, default_value = "/etc/hoard/config.toml", global = true)]
    config: PathBuf,

    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run the HTTP server. Default if no subcommand is given.
    Serve,
    /// Download the latest release from GitHub and replace this binary.
    /// Does not restart the systemd service; see the printed hint.
    Upgrade {
        /// Override the install destination. Defaults to the path of the
        /// currently-running binary (resolved via /proc/self/exe).
        #[arg(long)]
        target: Option<PathBuf>,
    },
    /// Apply pending database migrations and exit.
    ///
    /// Meant to run as a deploy step (Fly's `release_command`), not by hand:
    /// migrations that fail there abort the deploy and leave the version
    /// that's already serving untouched. Applied from inside `serve` instead,
    /// the same failure takes the server down and keeps it down, because the
    /// supervisor restarts it straight back into the migration that failed.
    Migrate,

    /// Re-read every cloud blob and check that its bytes hash to its name.
    ///
    /// The forensic half of the rotation-corruption fix: the client can no
    /// longer upload a blob whose contents don't match its sha256, but the
    /// ones committed before that fix are indistinguishable from healthy ones
    /// until someone restores them. Reads and records a verdict; never
    /// deletes. Cloud (Postgres + R2) only.
    #[cfg(feature = "cloud")]
    VerifyBlobs {
        /// Stop after N blobs. Default: every blob that has no verdict yet.
        #[arg(long)]
        limit: Option<i64>,
        /// Re-check blobs that already have a verdict.
        #[arg(long)]
        recheck: bool,
        /// Objects downloaded in parallel.
        #[arg(long, default_value_t = 4)]
        concurrency: usize,
        /// Report without writing verdicts to the database.
        #[arg(long)]
        dry_run: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // `upgrade` is self-contained: no config load, no DB, no logging init.
    if let Some(Cmd::Upgrade { target }) = args.cmd {
        return upgrade::run(target).await;
    }

    let cfg = Config::load(&args.config)?;

    init_logging(&cfg);

    if matches!(args.cmd, Some(Cmd::Migrate)) {
        match cfg.database.backend {
            DbBackend::Postgres => {
                #[cfg(feature = "cloud")]
                {
                    // `db::` here is the self-hosted SQLite module; the cloud
                    // pool lives in its own.
                    use hoard_server::cloud::db as cloud_db;
                    let pool =
                        cloud_db::connect(&cfg.database.url, cfg.database.max_connections).await?;
                    cloud_db::run_migrations(&pool).await?;
                    pool.close().await;
                    return Ok(());
                }
                #[cfg(not(feature = "cloud"))]
                {
                    anyhow::bail!(
                        "database.backend = \"postgres\" requires building with --features cloud"
                    )
                }
            }
            DbBackend::Sqlite => {
                anyhow::bail!("`migrate` is cloud-only; self-hosted migrates on start-up")
            }
        }
    }

    #[cfg(feature = "cloud")]
    if let Some(Cmd::VerifyBlobs {
        limit,
        recheck,
        concurrency,
        dry_run,
    }) = args.cmd
    {
        let opts = hoard_server::cloud::verify::Options {
            limit,
            recheck,
            concurrency: concurrency.max(1),
            dry_run,
        };
        let report = hoard_server::cloud::verify::run(&cfg, opts).await?;
        hoard_server::cloud::verify::print_report(&report, dry_run);
        // Salida distinta de cero si hay daño: así un cron lo nota.
        if !report.damaged.is_empty() {
            std::process::exit(2);
        }
        return Ok(());
    }

    info!(
        version = env!("CARGO_PKG_VERSION"),
        host = %cfg.server.host,
        port = cfg.server.port,
        backend = ?cfg.database.backend,
        "starting hoard-server"
    );

    match cfg.database.backend {
        DbBackend::Sqlite => run_self_hosted(cfg).await,
        DbBackend::Postgres => {
            #[cfg(feature = "cloud")]
            {
                hoard_server::cloud::run(cfg).await
            }
            #[cfg(not(feature = "cloud"))]
            {
                anyhow::bail!(
                    "database.backend = \"postgres\" requires building with --features cloud"
                )
            }
        }
    }
}

async fn run_self_hosted(cfg: Config) -> Result<()> {
    info!(data_dir = %cfg.storage.data_dir.display(), "self-hosted mode");

    // Ensure data subdirectories exist
    for dir in &["data", "tmp", "trash"] {
        tokio::fs::create_dir_all(cfg.storage.data_dir.join(dir)).await?;
    }

    let pool = db::connect(&cfg.database.url, cfg.database.max_connections).await?;
    db::run_migrations(&pool).await?;

    // Blob/chunk storage backend (ADR 0020): local disk or S3-compatible.
    // For s3 this probes the bucket (write+delete) and fails fast on a bad
    // endpoint before we accept any traffic.
    let store = hoard_server::store::build_store(&cfg).await?;

    // One-time migration of legacy folder snapshots into the blob store
    // (ADR 0018, eje C). No-op on fresh installs and once already migrated.
    // Local-layout only: skip it on the s3 backend (there are no legacy
    // on-disk `v<n>/` folders to migrate when blobs never lived on disk).
    if cfg.storage.backend == hoard_server::config::StorageBackend::Local {
        hoard_server::blobs::backfill_from_folders(&pool, &cfg.storage.data_dir).await?;
    }

    // Guard against flipping `[storage] backend` without migrating: if the DB
    // references objects the active store doesn't have, refuse to boot with a
    // pointer to `hoard-admin storage migrate` (ADR 0020, phase 2).
    hoard_server::store::sanity_check(&pool, &store).await?;

    // And the inverse, which is the one that loses data: a store holding an
    // earlier deployment's content and a database without a single user. Booting
    // like that looks healthy and leaves every client holding `save_id`s that no
    // longer exist here, so 404s forever.
    hoard_server::store::guard_against_lost_database(&pool, &cfg.storage.data_dir).await?;

    // Who may speak for someone else. A bad entry is named out loud: failing
    // open here means "your proxy isn't trusted", which is the safe direction
    // but an invisible one: every client behind it would silently share one
    // throttle bucket.
    let (trusted_proxies, bad_proxies) =
        hoard_server::clientip::TrustedProxies::parse(&cfg.server.trusted_proxies);
    for bad in &bad_proxies {
        warn!(
            entry = %bad.entry,
            why = bad.why,
            "server.trusted_proxies: ignoring an entry that isn't an address or CIDR"
        );
    }
    info!(trusted_proxies = %trusted_proxies, "trusting X-Forwarded-For from");

    let state = Arc::new(health::ServerState {
        pool: pool.clone(),
        config: cfg.clone(),
        start_time: Instant::now(),
        events: Default::default(),
        store: store.clone(),
        trusted_proxies,
    });

    // Routes that require auth
    let authed = Router::new()
        .route("/v1/auth/whoami", get(auth_routes::whoami))
        // Browser session teardown and self-service password change. Both need
        // an authenticated caller, so they live here; the login that mints the
        // session is public and sits on the router below.
        .route("/v1/auth/session", post(session_routes::exchange_token))
        .route("/v1/auth/logout", post(session_routes::logout))
        .route("/v1/auth/password", post(session_routes::change_password))
        // Rollups for the panel's account view (`routes::overview`).
        .route("/v1/me/overview", get(overview_routes::overview))
        .route("/v1/me/activity", get(overview_routes::activity))
        // Per-user cap on stored versions per save. Same path shape as the
        // cloud router so the agent hits one URL for both modes.
        .route(
            "/v1/me/max-versions",
            axum::routing::put(auth_routes::set_max_versions),
        )
        // Server→app push: long-lived SSE stream of this user's save changes
        // so other devices pull within ~1s instead of waiting for the sweep.
        .route("/v1/events", get(event_routes::stream))
        // Client diagnostic-log ingest. Smaller body cap than snapshots,
        // applied per-route so it overrides the large snapshot limit below.
        .route(
            "/v1/logs",
            post(log_routes::ingest).layer(axum::extract::DefaultBodyLimit::max(
                log_routes::MAX_BATCH_BYTES,
            )),
        )
        // Admin ops (self-hosted only, see ADR 0017). Gated on is_admin
        // inside the handler; the cloud router never mounts this.
        .route("/v1/admin/upgrade", post(admin_routes::upgrade))
        // Operator views behind the panel's server section. Each handler
        // checks `is_admin` itself; see the comment in `routes::admin`.
        .route("/v1/admin/overview", get(admin_routes::overview))
        .route("/v1/admin/users", post(admin_routes::create_user))
        .route(
            "/v1/admin/users/:id",
            axum::routing::patch(admin_routes::patch_user).delete(admin_routes::delete_user),
        )
        .route(
            "/v1/admin/tokens",
            get(admin_routes::tokens).post(admin_routes::create_token),
        )
        .route(
            "/v1/admin/tokens/:id/revoke",
            post(admin_routes::revoke_token),
        )
        .route("/v1/admin/logs", get(admin_routes::logs))
        // Games
        .route("/v1/games", get(game_routes::list))
        .route("/v1/games/:slug", get(game_routes::get_one))
        .route("/v1/games/:slug/known-paths", get(game_routes::known_paths))
        .route("/v1/manifest/version", get(game_routes::manifest_version))
        // Playtime mirror (hoard-wrapple recap source in self-hosted mode).
        .route(
            "/v1/playtime",
            get(playtime_routes::aggregate).post(playtime_routes::upload),
        )
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
        // Content-addressed upload: declare the manifest, upload only the
        // missing blobs, commit (see `routes::cas`). The multipart above stays
        // for clients that do not advertise it; `/v1/health` has carried
        // `cas: true` since 1.1.3.
        .route("/v1/saves/:save_id/cas/init", post(cas_routes::init))
        .route("/v1/saves/:save_id/cas/commit", post(cas_routes::commit))
        // The blob PUT lives in `blob_upload` below: it is the one route that
        // does not go through the per-IP limiter.
        // Censo de dispositivos + presencia en vivo (ver `routes::devices`).
        // The same routes as cloud, on purpose: the client already spoke them,
        // so there are not two protocols for one thing.
        .route("/v1/devices", get(device_routes::list))
        .route(
            "/v1/devices/:id",
            axum::routing::delete(device_routes::delete),
        )
        .route("/v1/presence/heartbeat", post(device_routes::heartbeat))
        .layer(axum::extract::DefaultBodyLimit::max(
            (cfg.storage.max_snapshot_size_mb as usize) * 1024 * 1024 + 16 * 1024 * 1024,
        ))
        .layer(middleware::from_fn_with_state(pool.clone(), require_auth));

    // `PUT /v1/cas/blobs/:upload_id/:sha256`, held apart from the router above
    // so the per-IP limiter does **not** cover it. Outside the `:save_id` tree
    // on purpose: a blob belongs to the user, not to one save, and the same
    // content can end up referenced by several.
    //
    // Why this one is exempt when everything else is limited: the number of
    // PUTs in an upload is not the client's choice, it was fixed by **this
    // server** when `cas/init` answered with the list of blobs it is missing.
    // Limiting them is fighting the batch we just authorised ourselves, and the
    // count is the save's file count: 173 for the Teardown in issue #17,
    // thousands for an emulator library. Any ceiling picked here is too low for
    // someone, and one high enough never to break is already no ceiling.
    //
    // Worse, the limiter's 429 goes out without draining the request body: the
    // client keeps writing a PUT nobody is reading, Windows sends RST and
    // **discards the response already sitting in its buffer**, so the 429 never
    // arrives and the client's pacer cannot react. See `put_blob_paced` in
    // `hoard-agent`.
    //
    // What actually needs braking (login, panel, polling) stays inside.
    let blob_upload = Router::new()
        .route(
            "/v1/cas/blobs/:upload_id/:sha256",
            axum::routing::put(cas_routes::upload_blob),
        )
        .layer(axum::extract::DefaultBodyLimit::max(
            (cfg.storage.max_snapshot_size_mb as usize) * 1024 * 1024 + 16 * 1024 * 1024,
        ))
        .layer(middleware::from_fn_with_state(pool.clone(), require_auth));

    // Spawn periodic cleanup task
    let cleanup_pool = pool.clone();
    let cleanup_data = cfg.storage.data_dir.clone();
    let cleanup_store = store.clone();
    let cleanup_tmp_h = cfg.retention.tmp_cleanup_hours;
    let cleanup_trash_d = cfg.retention.trash_retention_days;
    // Age-weighted snapshot pruning policy (ADR 0018). `None` disables it.
    let prune_policy = if cfg.retention.snapshot_pruning {
        Some(hoard_server::retention::RetentionPolicy::from_data_saving(
            cfg.retention.data_saving,
        ))
    } else {
        None
    };
    tokio::spawn(async move {
        cleanup::run_periodic(
            cleanup_pool,
            cleanup_data,
            cleanup_store,
            cleanup_tmp_h,
            cleanup_trash_d,
            prune_policy,
        )
        .await;
    });

    let mut public = Router::new().route("/v1/health", get(health::handler));

    // The panel and the password login it needs are one switch: an instance
    // that only ever talks to the desktop app has no reason to expose a second
    // way in, and leaving `/v1/auth/login` mounted with the pages gone would be
    // exactly that. Nothing here is authenticated: the login screen is the
    // one screen that by definition has no session yet.
    if cfg.panel.enabled {
        if cfg.panel.login_throttle_was_raised() {
            warn!(
                asked = cfg.panel.login_throttle_secs,
                using = hoard_server::config::MIN_LOGIN_THROTTLE_SECS,
                "panel: login_throttle_secs is below the minimum; using the minimum"
            );
        }
        info!(
            session_days = cfg.panel.session_days,
            login_throttle_secs = cfg.panel.login_throttle().as_secs(),
            "web panel enabled at /panel"
        );
        public = public
            .route("/", get(panel_routes::root))
            .route("/panel", get(panel_routes::index))
            .route("/panel/panel.css", get(panel_routes::css))
            .route("/panel/panel.js", get(panel_routes::js))
            .route("/panel/i18n/:lang", get(panel_routes::i18n))
            .route("/v1/auth/login", post(session_routes::login));
    }

    let app = public.merge(authed).with_state(state.clone());

    if cfg.server.rate_limit.enabled {
        info!(
            per_second = cfg.server.rate_limit.per_second,
            burst = cfg.server.rate_limit.burst,
            "rate limiting enabled (the CAS blob upload is exempt)"
        );
    }
    // Per-IP rate limiting (covers /v1/health and every authed route *except*
    // the CAS blob upload). Opt-out via [server.rate_limit].
    // SmartIpKeyExtractor needs ConnectInfo, which the
    // `into_make_service_with_connect_info` below supplies.
    //
    // The order (limit first, *then* merge the exempt route) is the whole
    // fix, and it lives inside `apply`, with a test that catches an inversion.
    let app =
        hoard_server::ratelimit::apply(&cfg.server.rate_limit, app, blob_upload.with_state(state));

    let addr: SocketAddr = format!("{}:{}", cfg.server.host, cfg.server.port).parse()?;
    info!(%addr, "listening");

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
