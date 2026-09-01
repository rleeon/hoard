use anyhow::{Context, Result};
use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub server: ServerConfig,
    pub storage: StorageConfig,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
    pub retention: RetentionConfig,
    pub logging: LoggingConfig,
    /// Browser panel served by this same binary. Self-hosted only; the cloud
    /// router never mounts it.
    #[serde(default)]
    pub panel: PanelConfig,
    /// Cloud mode configuration. Required when `database.backend = "postgres"`.
    /// Ignored in self-hosted mode.
    #[serde(default)]
    pub cloud: Option<CloudConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub public_url: String,
    /// When true (default), admins can trigger a self-upgrade remotely via
    /// `POST /v1/admin/upgrade`. The request only drops a marker file in
    /// `data_dir`; a root systemd oneshot does the privileged work (ADR
    /// 0017). Set to false to disable the remote path entirely.
    #[serde(default = "default_allow_remote_upgrade")]
    pub allow_remote_upgrade: bool,
    /// Per-IP request rate limiting. On by default; tune or disable via the
    /// `[server.rate_limit]` table. This is an in-process safety net (cheap
    /// brute-force / accidental-loop protection); production deployments should
    /// still rate-limit at the reverse proxy.
    #[serde(default)]
    pub rate_limit: RateLimitConfig,
    /// Which peers are allowed to tell the server who the client is, meaning
    /// whose `X-Forwarded-For` we believe. Addresses, CIDRs, or the shorthands
    /// `loopback` / `private` / `any`; see
    /// [`crate::clientip::TrustedProxies::parse`].
    ///
    /// Defaults to `loopback`, which covers the usual single-box shape (nginx
    /// or Caddy on the same host proxying to `127.0.0.1:12421`) and nothing
    /// else. A proxy that reaches the server from another address (a container
    /// on a Docker network, a box elsewhere on the LAN) has to be
    /// named here, or its clients all count as one.
    ///
    /// It is not a nicety: the panel's login throttle counts wrong passwords
    /// per client address, so believing an unvouched header let a direct caller
    /// rotate it and never hit the limit at all.
    #[serde(default = "default_trusted_proxies")]
    pub trusted_proxies: Vec<String>,
}

fn default_trusted_proxies() -> Vec<String> {
    vec!["loopback".to_string()]
}

fn default_allow_remote_upgrade() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RateLimitConfig {
    /// Master switch. Defaults on.
    #[serde(default = "default_rate_limit_enabled")]
    pub enabled: bool,
    /// Sustained allowance in requests per second per client IP (one cell
    /// replenishes every `1/per_second` seconds). NOTE: this is our unit, not
    /// tower_governor's, whose builder method of the same name takes a period
    /// in seconds; `ratelimit::layer` does the conversion.
    #[serde(default = "default_rate_limit_per_second")]
    pub per_second: u64,
    /// Burst capacity: how many requests can arrive back-to-back before the
    /// sustained rate kicks in.
    #[serde(default = "default_rate_limit_burst")]
    pub burst: u32,
    /// Per-device cap on the cheap polling endpoints (`/v1/cloud/sync`,
    /// `/v1/devices`, `/v1/notifications`, `/v1/presence/heartbeat`), in
    /// sustained requests per minute per (user, device, endpoint). `0`
    /// disables the guard. Separate from the per-IP limit above: that one
    /// is loose enough (50/s on Fly) to let a whole re-sync burst through,
    /// which a single runaway poller also fits under.
    #[serde(default = "default_rate_limit_poll_per_minute")]
    pub poll_per_minute: u32,
    /// Burst capacity for the poll guard. Must absorb the legitimate spikes
    /// of the official client: app startup fires one `/v1/cloud/sync` per
    /// save being auto-restored (`hoard-agent::restore` fetches the
    /// manifest per game) on top of the login pull and feed kicks. The
    /// sustained rate above is what actually stops a hammering client,
    /// after the burst drains it converges to `poll_per_minute`.
    #[serde(default = "default_rate_limit_poll_burst")]
    pub poll_burst: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: default_rate_limit_enabled(),
            per_second: default_rate_limit_per_second(),
            burst: default_rate_limit_burst(),
            poll_per_minute: default_rate_limit_poll_per_minute(),
            poll_burst: default_rate_limit_poll_burst(),
        }
    }
}

fn default_rate_limit_enabled() -> bool {
    true
}
fn default_rate_limit_per_second() -> u64 {
    20
}
fn default_rate_limit_burst() -> u32 {
    60
}
fn default_rate_limit_poll_per_minute() -> u32 {
    10
}
fn default_rate_limit_poll_burst() -> u32 {
    60
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StorageConfig {
    pub data_dir: PathBuf,
    pub max_snapshot_size_mb: u64,
    pub upload_timeout_secs: u64,
    /// Where blob/chunk bytes live: local disk (`"local"`, the default and the
    /// zero-config upgrade path) or an S3-compatible bucket (`"s3"`, ADR 0020).
    /// The SQLite index, `tmp/` upload staging and the upgrade marker always
    /// stay on local disk regardless.
    #[serde(default)]
    pub backend: StorageBackend,
    /// S3 endpoint settings. Required when `backend = "s3"`, ignored otherwise.
    #[serde(default)]
    pub s3: Option<S3StorageConfig>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum StorageBackend {
    #[default]
    Local,
    S3,
}

/// Self-hosted S3-compatible storage (MinIO, Backblaze B2, R2, Garage, Wasabi,
/// or `rclone serve s3` fronting Mega/Dropbox/Drive). Distinct from the
/// cloud-mode `[cloud.r2]` block; this is for a self-hosted server that wants
/// its blobs off local disk. Secrets can come from `HOARD__STORAGE__S3__*`.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct S3StorageConfig {
    /// Full endpoint URL, e.g. `http://localhost:9000` (MinIO) or
    /// `https://s3.us-west-002.backblazeb2.com`.
    #[serde(default)]
    pub endpoint: String,
    #[serde(default)]
    pub bucket: String,
    /// Some services ignore region; leave empty to default to `auto`.
    #[serde(default)]
    pub region: String,
    #[serde(default)]
    pub access_key_id: String,
    #[serde(default)]
    pub secret_access_key: String,
    /// Optional prefix prepended to every object key (share one bucket across
    /// deployments). Empty by default.
    #[serde(default)]
    pub key_prefix: String,
    /// Path-style addressing (`endpoint/bucket/key`). Default true, because MinIO,
    /// Garage and `rclone serve s3` require it; leave true unless your provider
    /// mandates virtual-host style.
    #[serde(default = "default_force_path_style")]
    pub force_path_style: bool,
}

fn default_force_path_style() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    /// Connection URL. `sqlite://...` for self-hosted, `postgres://...` for cloud.
    pub url: String,
    pub max_connections: u32,
    /// Selects which backend the server boots into.
    /// Cloud routes (Supabase/R2/LS) only exist on `postgres`.
    #[serde(default = "default_backend")]
    pub backend: DbBackend,
}

fn default_backend() -> DbBackend {
    DbBackend::Sqlite
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DbBackend {
    Sqlite,
    Postgres,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthConfig {
    pub token_lifetime_days: u64,
    pub allow_registration: bool,
}

/// The web panel (`GET /panel`).
///
/// Off-by-config rather than off-by-default: a self-hoster who never opens a
/// browser still pays only three static routes, and the ones who do want it
/// shouldn't have to discover a config key first. Turning it off removes the
/// panel routes *and* the password login with them, which is the point: an
/// instance that only ever talks to the desktop app has no reason to expose a
/// second way in.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PanelConfig {
    #[serde(default = "default_panel_enabled")]
    pub enabled: bool,
    /// How long a browser session lasts before the cookie's token expires.
    /// Short by API standards (`auth.token_lifetime_days` defaults to a year)
    /// because a browser is a shared, long-lived, extension-infested place and
    /// this token is as powerful as the one in the desktop app.
    #[serde(default = "default_panel_session_days")]
    pub session_days: u64,
    /// How long the login door stays shut after too many wrong passwords, in
    /// seconds. Read through [`PanelConfig::login_throttle`], which enforces
    /// the floor. Never read this field directly.
    ///
    /// Ten by default, which is short for a login form and right for a box on
    /// your own network: long enough that an argon2id verify (19 MiB, tens of
    /// milliseconds a go) stops being a free CPU lever, short enough that
    /// fumbling your own password three times doesn't lock you out of your own
    /// server. Raise it towards a minute or more if the panel is exposed to the
    /// open internet, where what protects the account is the password, and a
    /// slow door buys it time.
    #[serde(default = "default_panel_login_throttle_secs")]
    pub login_throttle_secs: u64,
}

/// Floor on [`PanelConfig::login_throttle_secs`]. There is no "off": with no
/// pause at all, each guess costs the server a full password hash and costs the
/// attacker one request, which is the wrong way round. Two seconds is small
/// enough to be invisible to a human who mistyped and large enough that the
/// hashing can't be used as a lever.
pub const MIN_LOGIN_THROTTLE_SECS: u64 = 2;

impl PanelConfig {
    /// The throttle window, with the floor applied. The one place the number is
    /// decided, so a config that asks for less can't leak past a caller that
    /// forgot to clamp.
    pub fn login_throttle(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.login_throttle_secs.max(MIN_LOGIN_THROTTLE_SECS))
    }

    /// True when the configured value was raised to the floor, so the caller logs
    /// it once at boot so the operator isn't left thinking their `1` took.
    pub fn login_throttle_was_raised(&self) -> bool {
        self.login_throttle_secs < MIN_LOGIN_THROTTLE_SECS
    }
}

impl Default for PanelConfig {
    fn default() -> Self {
        Self {
            enabled: default_panel_enabled(),
            session_days: default_panel_session_days(),
            login_throttle_secs: default_panel_login_throttle_secs(),
        }
    }
}

fn default_panel_enabled() -> bool {
    true
}
fn default_panel_session_days() -> u64 {
    14
}
fn default_panel_login_throttle_secs() -> u64 {
    10
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RetentionConfig {
    pub trash_retention_days: u64,
    pub tmp_cleanup_hours: u64,
    /// Age-weighted snapshot pruning (ADR 0018, eje B). When true, the
    /// periodic cleanup thins old snapshots per the `data_saving` policy,
    /// soft-deleting the losers into `trash/` (recoverable, purged later by
    /// `trash_retention_days`). Defaults on. `#[serde(default)]` so existing
    /// configs without this key keep parsing.
    #[serde(default = "default_snapshot_pruning")]
    pub snapshot_pruning: bool,
    /// "Ahorro de datos" knob `k ∈ [0,1]`: higher keeps fewer snapshots.
    /// Maps onto concrete keep-counts via
    /// `RetentionPolicy::from_data_saving`. Default 0.3.
    #[serde(default = "default_data_saving")]
    pub data_saving: f64,
}

fn default_snapshot_pruning() -> bool {
    true
}

fn default_data_saving() -> f64 {
    0.3
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: LogFormat,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    Json,
    Pretty,
}

/// Cloud-mode configuration. Most fields can also come from env vars (and
/// usually do in production, since secrets shouldn't live in config files).
/// Figment merges `HOARD__CLOUD__*` over the TOML, so leaving these empty in
/// `config.toml` and exporting via env is the standard production path.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct CloudConfig {
    /// JWKS URL for Supabase Auth, used to verify access tokens.
    /// Typical value: `https://<project>.supabase.co/auth/v1/.well-known/jwks.json`.
    #[serde(default)]
    pub supabase_jwks_url: String,
    /// Audience claim expected in JWTs. Supabase issues `authenticated` by
    /// default; override if you've changed it.
    #[serde(default = "default_aud")]
    pub supabase_audience: String,
    /// Optional issuer claim check. Empty = skip.
    #[serde(default)]
    pub supabase_issuer: String,
    /// Supabase project base URL (`https://<project>.supabase.co`). Only the
    /// device-pairing flow needs it: on approval the server calls the GoTrue
    /// admin API here to mint a fresh session for the CLI. Empty = the
    /// `POST /v1/cloud/device/*` endpoints refuse (feature off).
    /// From `HOARD__CLOUD__SUPABASE_URL`.
    #[serde(default)]
    pub supabase_url: String,
    /// Supabase **service-role** key. Privileged: used only server-side to
    /// mint device-pairing sessions (admin `generate_link` + `verify`). Never
    /// sent to clients. From `HOARD__CLOUD__SUPABASE_SERVICE_ROLE_KEY`.
    #[serde(default)]
    pub supabase_service_role_key: String,
    /// JWKS refresh interval (seconds). Defaults to one hour.
    #[serde(default = "default_jwks_refresh_secs")]
    pub jwks_refresh_secs: u64,
    #[serde(default)]
    pub r2: R2Config,
    /// Polar (Merchant of Record) billing configuration. Webhooks upsert
    /// into the `subscriptions` table keyed on the provider's subscription
    /// id. Fields usually come from `HOARD__CLOUD__POLAR__*`.
    #[serde(default)]
    pub polar: PolarConfig,
    /// Public-facing URL of the Hoard Cloud landing/checkout. Embedded in
    /// 402 responses so the client can offer an upgrade link.
    #[serde(default = "default_upgrade_url")]
    pub upgrade_url: String,
    /// Grace window (days) a user keeps their old, larger storage limit after
    /// dropping to a smaller tier whose size is below their current footprint.
    /// Nothing is purged during the window and `/v1/me` advertises the pending
    /// change. Defaults to 30. Cloud only; self-hosted has no quotas.
    #[serde(default = "default_storage_downgrade_grace_days")]
    pub storage_downgrade_grace_days: u64,
    /// Transactional email (account-export "your download is ready"). Optional:
    /// with no API key the export worker still runs and produces a downloadable
    /// ZIP surfaced in-app; it just skips the email leg.
    #[serde(default)]
    pub email: EmailConfig,
    /// At-rest zstd compression of content-addressed blobs (cost saver:
    /// R2 bills physical bytes, quota keeps charging raw bytes). Off by
    /// default; enable in dev first. Fields from `HOARD__CLOUD__COMPRESSION__*`.
    #[serde(default)]
    pub compression: CompressionConfig,
}

/// Background blob-compression sweep settings. Purely server-side: clients
/// keep uploading raw bytes to presigned PUTs and keep receiving raw bytes
/// on download (compressed blobs are served through the decompressing
/// `/v1/cloud/blob/:token` proxy instead of a direct presigned GET).
/// Deliberately undocumented in the example config: it is internal storage
/// maintenance, not an operator feature; override via
/// `HOARD__CLOUD__COMPRESSION__*` env vars if ever needed.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CompressionConfig {
    /// Master switch. On by default: steady state is "every blob older than
    /// `min_age_hours` is compressed".
    #[serde(default = "default_compression_enabled")]
    pub enabled: bool,
    /// Only compress blobs at least this old. New blobs are the ones a
    /// freshly-synced device is most likely to pull; leave them on the
    /// direct presigned path for a while.
    #[serde(default = "default_compression_min_age_hours")]
    pub min_age_hours: u64,
    /// Skip blobs that had a download URL minted within this window, so an
    /// in-flight direct GET can never race the object overwrite.
    #[serde(default = "default_compression_idle_hours")]
    pub idle_hours: u64,
    /// Blobs compressed per sweep tick.
    #[serde(default = "default_compression_batch")]
    pub batch: u32,
    /// Seconds between sweep ticks.
    #[serde(default = "default_compression_sweep_secs")]
    pub sweep_secs: u64,
    /// zstd level (1-21). 9 is a good size/CPU balance for a background job.
    #[serde(default = "default_compression_level")]
    pub level: i32,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            enabled: default_compression_enabled(),
            min_age_hours: default_compression_min_age_hours(),
            idle_hours: default_compression_idle_hours(),
            batch: default_compression_batch(),
            sweep_secs: default_compression_sweep_secs(),
            level: default_compression_level(),
        }
    }
}

fn default_compression_enabled() -> bool {
    true
}
fn default_compression_min_age_hours() -> u64 {
    72
}
fn default_compression_idle_hours() -> u64 {
    24
}
fn default_compression_batch() -> u32 {
    50
}
fn default_compression_sweep_secs() -> u64 {
    300
}
fn default_compression_level() -> i32 {
    9
}

/// Resend transactional-email settings. Fields usually come from
/// `HOARD__CLOUD__EMAIL__*`. Empty `api_key` disables email delivery.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct EmailConfig {
    /// Resend API key (`re_...`). Empty = email delivery disabled.
    #[serde(default)]
    pub api_key: String,
    /// From header, e.g. `Hoard <noreply@hoard.services>`. Required for sends.
    #[serde(default)]
    pub from: String,
}

fn default_aud() -> String {
    "authenticated".to_string()
}
fn default_jwks_refresh_secs() -> u64 {
    3600
}
fn default_upgrade_url() -> String {
    "https://hoard.services/upgrade".to_string()
}
fn default_storage_downgrade_grace_days() -> u64 {
    30
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct R2Config {
    #[serde(default)]
    pub endpoint: String,
    #[serde(default)]
    pub bucket: String,
    #[serde(default)]
    pub region: String,
    #[serde(default)]
    pub access_key_id: String,
    #[serde(default)]
    pub secret_access_key: String,
    /// Default TTL for presigned URLs, in seconds. 1h is a sane default.
    #[serde(default = "default_presign_ttl")]
    pub presign_ttl_secs: u64,
}

fn default_presign_ttl() -> u64 {
    3600
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PolarConfig {
    /// Organization access token (OAT). Used for server-initiated Polar API
    /// calls, specifically `POST /v1/checkouts/` to create a checkout
    /// session. Webhook verification itself only needs `webhook_secret`.
    #[serde(default)]
    pub access_token: String,
    /// Standard Webhooks signing secret (the `polar_whs_...` value). Used
    /// verbatim as the HMAC key; see `cloud::polar::verify_signature`.
    #[serde(default)]
    pub webhook_secret: String,
    /// Polar API base URL. Empty falls back to production
    /// (`https://api.polar.sh`); set to `https://sandbox-api.polar.sh` to
    /// test against the sandbox.
    #[serde(default)]
    pub base_url: String,
    /// Where Polar redirects the browser after a successful payment. Empty
    /// falls back to `https://hoard.services/account?upgraded=1`. Polar
    /// substitutes `{CHECKOUT_ID}` if present.
    #[serde(default)]
    pub success_url: String,
    /// Map Polar product UUID -> our plan tier and interval. Two products:
    /// Pro Monthly and Pro Yearly.
    #[serde(default)]
    pub products: Vec<PolarProduct>,
}

impl PolarConfig {
    /// Production base unless an explicit override is configured.
    pub fn base(&self) -> &str {
        if self.base_url.is_empty() {
            "https://api.polar.sh"
        } else {
            self.base_url.trim_end_matches('/')
        }
    }

    /// Success redirect, with a production-account fallback.
    pub fn success(&self) -> &str {
        if self.success_url.is_empty() {
            "https://hoard.services/account?upgraded=1"
        } else {
            &self.success_url
        }
    }

    /// Resolve our (plan, interval) pair → the Polar product UUID to buy.
    /// With storage tiers there are several Pro products per interval; this
    /// returns the smallest-storage one (the base, Pro x1) so callers that
    /// don't care about a tier get a deterministic default. Use
    /// `product_for_storage` to pick a specific tier.
    pub fn product_for(&self, plan: &str, interval: &str) -> Option<&str> {
        self.products
            .iter()
            .filter(|p| p.plan == plan && p.interval == interval)
            .min_by_key(|p| p.storage_gb.unwrap_or(0))
            .map(|p| p.product_id.as_str())
    }

    /// Resolve (plan, interval, storage_gb) → the exact tier's product UUID.
    /// `storage_gb == None` matches the base product (Pro x1).
    pub fn product_for_storage(
        &self,
        plan: &str,
        interval: &str,
        storage_gb: Option<u64>,
    ) -> Option<&str> {
        self.products
            .iter()
            .find(|p| p.plan == plan && p.interval == interval && p.storage_gb == storage_gb)
            .map(|p| p.product_id.as_str())
    }

    /// Storage size (bytes) sold by a given Polar product UUID. `None` if the
    /// product isn't configured or carries no `storage_gb` (treated as the
    /// plan default by `effective_storage_limit`).
    pub fn storage_bytes_for_product(&self, product_id: &str) -> Option<u64> {
        self.products
            .iter()
            .find(|p| p.product_id == product_id)
            .and_then(|p| p.storage_gb)
            .map(|gb| gb * 1024 * 1024 * 1024)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PolarProduct {
    pub product_id: String, // UUID
    pub plan: String,       // 'pro'
    pub interval: String,   // 'month' | 'year'
    /// Storage this tier grants, in GB (Pro x1 = 25, x2 = 50, …). Written to
    /// `profiles.storage_limit_bytes` by the webhook. `None`/omitted means the
    /// plan default (so an old single-tier config keeps working).
    #[serde(default)]
    pub storage_gb: Option<u64>,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            let mut tried = vec![path.display().to_string()];
            if let Some(found) = Self::search_fallbacks(&mut tried) {
                return Self::load_from(&found);
            }
            anyhow::bail!(
                "Config file not found. Tried:\n  - {}\n\n\
                 Create one from the example, e.g.:\n    \
                 mkdir -p ~/.config/hoard && cp deploy/config.toml.example ~/.config/hoard/server.toml\n  \
                 or for a system-wide install:\n    \
                 sudo mkdir -p /etc/hoard && sudo cp deploy/config.toml.example /etc/hoard/config.toml",
                tried.join("\n  - ")
            );
        }
        Self::load_from(path)
    }

    fn load_from(path: &Path) -> Result<Self> {
        let config: Config = Figment::new()
            .merge(Toml::file(path))
            .merge(Env::prefixed("HOARD__").split("__"))
            .extract()
            .with_context(|| format!("Failed to parse config from {}", path.display()))?;

        config.validate()?;
        Ok(config)
    }

    /// Search the standard fallback locations for a config file. Used when
    /// the explicit `--config` path doesn't exist, letting users run
    /// `hoard-server` without sudo by dropping a config in their XDG config
    /// dir or the working directory.
    ///
    /// We deliberately use `server.toml` for the XDG path instead of
    /// `config.toml` because the CLI (`hoard-cli`) already uses
    /// `~/.config/hoard/config.toml` with a different schema, and sharing the
    /// filename would cause confusing parse errors when the server tries
    /// to read CLI credentials.
    ///
    /// Order, first-found wins:
    ///   1. `$XDG_CONFIG_HOME/hoard/server.toml`
    ///      (or `$HOME/.config/hoard/server.toml`)
    ///   2. `./hoard-server.toml` in the current working directory
    ///   3. `./server.toml` in the current working directory
    fn search_fallbacks(tried: &mut Vec<String>) -> Option<PathBuf> {
        let mut candidates: Vec<PathBuf> = Vec::new();

        if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
            candidates.push(PathBuf::from(xdg).join("hoard").join("server.toml"));
        } else if let Some(home) = std::env::var_os("HOME") {
            candidates.push(
                PathBuf::from(home)
                    .join(".config")
                    .join("hoard")
                    .join("server.toml"),
            );
        }

        if let Ok(cwd) = std::env::current_dir() {
            candidates.push(cwd.join("hoard-server.toml"));
            candidates.push(cwd.join("server.toml"));
        }

        for c in candidates {
            tried.push(c.display().to_string());
            if c.exists() {
                return Some(c);
            }
        }
        None
    }

    fn validate(&self) -> Result<()> {
        if self.server.port == 0 {
            anyhow::bail!("server.port must be > 0");
        }

        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.logging.level.as_str()) {
            anyhow::bail!(
                "logging.level must be one of {:?}, got {:?}",
                valid_levels,
                self.logging.level
            );
        }

        match self.database.backend {
            DbBackend::Sqlite => {
                // Self-hosted: storage lives on disk under data_dir.
                if !self.storage.data_dir.exists() {
                    anyhow::bail!(
                        "storage.data_dir {:?} does not exist. Create it with: \
                         mkdir -p {}",
                        self.storage.data_dir,
                        self.storage.data_dir.display()
                    );
                }
                // Check write permission by attempting to create a temp file.
                // `data_dir` is still required for the s3 backend, since tmp
                // upload staging, the SQLite DB and the upgrade marker live here, so
                // this applies regardless of `storage.backend`.
                let probe = self.storage.data_dir.join(".hoard_write_probe");
                std::fs::write(&probe, b"").with_context(|| {
                    format!(
                        "storage.data_dir {:?} is not writable",
                        self.storage.data_dir
                    )
                })?;
                std::fs::remove_file(&probe).ok();

                // S3 blob backend (ADR 0020): validate config shape here; the
                // async reachability probe (bucket write+delete) runs at
                // startup in `store::build_store`.
                if self.storage.backend == StorageBackend::S3 {
                    #[cfg(not(feature = "s3-backend"))]
                    anyhow::bail!(
                        "storage.backend = \"s3\" requires building with --features s3-backend"
                    );
                    #[cfg(feature = "s3-backend")]
                    {
                        let s3 = self.storage.s3.as_ref().ok_or_else(|| {
                            anyhow::anyhow!(
                                "storage.backend = \"s3\" requires an [storage.s3] section in config"
                            )
                        })?;
                        if s3.endpoint.is_empty() || s3.bucket.is_empty() {
                            anyhow::bail!(
                                "storage.s3.endpoint and storage.s3.bucket are required for the s3 backend"
                            );
                        }
                    }
                }
            }
            DbBackend::Postgres => {
                // Cloud mode: storage is R2, not disk. data_dir may still be
                // present in the TOML (it's not optional in the schema) but
                // we don't require it to exist.
                #[cfg(not(feature = "cloud"))]
                anyhow::bail!(
                    "database.backend = \"postgres\" requires building with --features cloud"
                );
                #[cfg(feature = "cloud")]
                {
                    let cloud = self.cloud.as_ref().ok_or_else(|| {
                        anyhow::anyhow!(
                            "cloud mode requires [cloud] section in config (see deploy/config.cloud.toml.example)"
                        )
                    })?;
                    if cloud.supabase_jwks_url.is_empty() {
                        anyhow::bail!(
                            "cloud.supabase_jwks_url is required when database.backend = \"postgres\""
                        );
                    }
                    if cloud.r2.bucket.is_empty() || cloud.r2.endpoint.is_empty() {
                        anyhow::bail!("cloud.r2.bucket and cloud.r2.endpoint are required");
                    }
                }
            }
        }

        Ok(())
    }
}
