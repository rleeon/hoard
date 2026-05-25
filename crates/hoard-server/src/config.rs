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
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StorageConfig {
    pub data_dir: PathBuf,
    pub max_snapshot_size_mb: u64,
    pub upload_timeout_secs: u64,
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RetentionConfig {
    pub trash_retention_days: u64,
    pub tmp_cleanup_hours: u64,
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
    /// JWKS URL for Supabase Auth — used to verify access tokens.
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
    /// JWKS refresh interval (seconds). Defaults to one hour.
    #[serde(default = "default_jwks_refresh_secs")]
    pub jwks_refresh_secs: u64,
    #[serde(default)]
    pub r2: R2Config,
    #[serde(default)]
    pub lemonsqueezy: LemonSqueezyConfig,
    /// Public-facing URL of the Hoard Cloud landing/checkout. Embedded in
    /// 402 responses so the client can offer an upgrade link.
    #[serde(default = "default_upgrade_url")]
    pub upgrade_url: String,
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
pub struct LemonSqueezyConfig {
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub webhook_secret: String,
    #[serde(default)]
    pub store_id: String,
    /// Map LS variant_id -> our plan tier and interval. Single tier
    /// post-1.6.1 (Pro) so `plan` is always `"pro"`; the field stays as
    /// a string instead of an enum so an old Pro+ variant in a deployed
    /// config doesn't fail validation — it just won't resolve into the
    /// runtime `Plan` enum and will return 400 from the webhook.
    #[serde(default)]
    pub variants: Vec<LemonSqueezyVariant>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LemonSqueezyVariant {
    pub variant_id: String,
    pub plan: String,     // 'pro'
    pub interval: String, // 'month' | 'year'
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
    /// the explicit `--config` path doesn't exist — letting users run
    /// `hoard-server` without sudo by dropping a config in their XDG config
    /// dir or the working directory.
    ///
    /// We deliberately use `server.toml` for the XDG path instead of
    /// `config.toml` because the CLI (`hoard-cli`) already uses
    /// `~/.config/hoard/config.toml` with a different schema — sharing the
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
                // Check write permission by attempting to create a temp file
                let probe = self.storage.data_dir.join(".hoard_write_probe");
                std::fs::write(&probe, b"").with_context(|| {
                    format!(
                        "storage.data_dir {:?} is not writable",
                        self.storage.data_dir
                    )
                })?;
                std::fs::remove_file(&probe).ok();
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
