use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CliConfig {
    pub server: ServerSection,
    #[serde(default)]
    pub auth: AuthSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerSection {
    pub url: String,
}

impl Default for ServerSection {
    fn default() -> Self {
        Self {
            url: "http://localhost:8080".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthSection {
    /// Bearer token in plaintext (`hoard_v1_<hex>`).
    /// Stored in the user's config dir; permissions tightened to 0600 on Unix.
    pub token: Option<String>,
}

impl CliConfig {
    pub fn project_dirs() -> Result<ProjectDirs> {
        ProjectDirs::from("dev", "hoard", "hoard")
            .context("could not determine user config directory")
    }

    pub fn default_path() -> Result<PathBuf> {
        let pd = Self::project_dirs()?;
        Ok(pd.config_dir().join("config.toml"))
    }

    pub fn state_dir() -> Result<PathBuf> {
        let pd = Self::project_dirs()?;
        Ok(pd.data_local_dir().to_path_buf())
    }

    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("reading {}", path.display()))?;
        let cfg: CliConfig = toml::from_str(&text)
            .with_context(|| format!("parsing {}", path.display()))?;
        Ok(cfg)
    }

    pub fn load_default() -> Result<(Self, PathBuf)> {
        let path = Self::default_path()?;
        let cfg = Self::load(&path)?;
        Ok((cfg, path))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        let text = toml::to_string_pretty(self).context("serializing config")?;
        std::fs::write(path, text)
            .with_context(|| format!("writing {}", path.display()))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(path)?.permissions();
            perms.set_mode(0o600);
            std::fs::set_permissions(path, perms)?;
        }
        Ok(())
    }

    pub fn require_token(&self) -> Result<&str> {
        self.auth
            .token
            .as_deref()
            .filter(|s| !s.is_empty())
            .context("not logged in: run `hoard login --token <TOKEN>` first")
    }
}
