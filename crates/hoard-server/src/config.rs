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
    pub url: String,
    pub max_connections: u32,
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

        Ok(())
    }
}
