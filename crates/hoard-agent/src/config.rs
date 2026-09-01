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
            url: "http://localhost:12421".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthSection {
    /// Bearer token in plaintext (`hoard_v1_<hex>`).
    /// Stored in the user's config dir; permissions tightened to 0600 on Unix.
    pub token: Option<String>,
}

/// Resuelve el directorio de estado en Windows, mudando el antiguo la primera
/// time somebody asks.
///
/// The rules, in order, all with the same criterion: never return an empty
/// folder while the data is in another one.
///
/// 1. If the destination already exists, it is the right one (already migrated,
///    or the install was born there).
/// 2. If the source does not exist, there is nothing to move either.
/// 3. If the `rename` fails, the source keeps being used. A move that does not
///    happen is a nuisance; losing sight of the data is the bug this fixes.
///
/// The `rename` is atomic, since source and destination both hang off `AppData`
/// on the same volume, so there is no intermediate state with the files half
/// moved. If two processes race (the app and the service starting together, which
/// is normal) whoever loses sees their `rename` fail and finds the destination
/// already created, which is the right answer.
///
/// Only called on Windows, but compiled everywhere on purpose: it is what
/// decides where the user's data lives. Compiling and testing it always is what
/// stops a silly mistake travelling all the way to the one place it runs.
#[cfg_attr(not(windows), allow(dead_code))]
fn relocated_state_dir(old: &Path, new: &Path) -> PathBuf {
    if new.is_dir() {
        return new.to_path_buf();
    }
    if !old.is_dir() {
        return new.to_path_buf();
    }
    if let Some(parent) = new.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            tracing::warn!(
                error = %e,
                path = %parent.display(),
                "no se pudo preparar el nuevo directorio de estado; se sigue con el antiguo"
            );
            return old.to_path_buf();
        }
    }
    match std::fs::rename(old, new) {
        Ok(()) => {
            tracing::info!(
                from = %old.display(),
                to = %new.display(),
                "estado movido fuera de la carpeta de instalación"
            );
            new.to_path_buf()
        }
        Err(e) => {
            // Puede ser la carrera de arranque: si el otro proceso ya lo movió,
            // el destino existe y es la respuesta buena.
            if new.is_dir() {
                return new.to_path_buf();
            }
            tracing::warn!(
                error = %e,
                from = %old.display(),
                to = %new.display(),
                "no se pudo mover el estado; se sigue usando el antiguo"
            );
            old.to_path_buf()
        }
    }
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

    /// Where the user's state lives: watched saves, hours played,
    /// caché de detección, preferencias.
    ///
    /// En Windows **no** es `data_local_dir()`, y el motivo le costó a un
    /// usuario su historial. `ProjectDirs` lo resuelve a
    /// `%LOCALAPPDATA%\hoard\hoard\data`, and the NSIS installer (`productName`
    /// "Hoard", `installMode` `currentUser`) installs into `%LOCALAPPDATA%\Hoard`.
    /// Windows is case-insensitive, so the user's data ended up inside the
    /// install folder, which meant reinstalling or updating could take it away.
    /// And it did not stay local: the client started with its hours at zero and
    /// its next upload propagated that emptiness to the cloud.
    ///
    /// It moves to `%APPDATA%` (Roaming), where the configuration already lived
    /// and where no installer digs. The cache stays in Local, which is exactly the
    /// split Windows asks for: Roaming for the user's small state, Local for what
    /// can be rebuilt. On Linux and macOS `data_dir()` and `data_local_dir()` are
    /// the same path, so outside Windows nothing
    /// cambia nada.
    pub fn state_dir() -> Result<PathBuf> {
        let pd = Self::project_dirs()?;
        #[cfg(not(windows))]
        {
            Ok(pd.data_local_dir().to_path_buf())
        }
        #[cfg(windows)]
        {
            Ok(relocated_state_dir(pd.data_local_dir(), pd.data_dir()))
        }
    }

    /// Where rotating log files live. Distinct from `state_dir` so the user
    /// (or a packager's `clean cache` step) can wipe logs without nuking
    /// their tracked-saves mapping.
    pub fn cache_dir() -> Result<PathBuf> {
        let pd = Self::project_dirs()?;
        Ok(pd.cache_dir().to_path_buf())
    }

    pub fn logs_dir() -> Result<PathBuf> {
        Ok(Self::cache_dir()?.join("logs"))
    }

    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let cfg: CliConfig =
            toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
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
        std::fs::write(path, text).with_context(|| format!("writing {}", path.display()))?;

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

#[cfg(test)]
mod tests {
    use super::*;

    /// A directory with a file inside, to tell "it moved" from "it was created
    /// empty", which is exactly the confusion that cost a history.
    fn seeded(dir: &Path, name: &str) -> PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        let f = dir.join(name);
        std::fs::write(&f, b"x").unwrap();
        f
    }

    #[test]
    fn el_destino_existente_manda_y_el_origen_no_se_toca() {
        let tmp = tempfile::tempdir().unwrap();
        let old = tmp.path().join("local/hoard/data");
        let new = tmp.path().join("roaming/hoard/data");
        seeded(&old, "viejo.json");
        seeded(&new, "bueno.json");

        assert_eq!(relocated_state_dir(&old, &new), new);
        // Already migrated: moving again would overwrite the good with the old.
        assert!(new.join("bueno.json").exists());
        assert!(old.join("viejo.json").exists());
    }

    #[test]
    fn sin_origen_se_usa_el_destino_aunque_no_exista_todavia() {
        let tmp = tempfile::tempdir().unwrap();
        let old = tmp.path().join("local/hoard/data");
        let new = tmp.path().join("roaming/hoard/data");

        // Instalación limpia: nadie ha escrito nada aún. El llamante creará el
        // directorio cuando le toque.
        assert_eq!(relocated_state_dir(&old, &new), new);
    }

    #[test]
    fn el_estado_se_muda_con_su_contenido() {
        let tmp = tempfile::tempdir().unwrap();
        let old = tmp.path().join("local/hoard/data");
        let new = tmp.path().join("roaming/hoard/data");
        seeded(&old, "playtime.json");
        std::fs::create_dir_all(old.join("contexts")).unwrap();
        std::fs::write(old.join("contexts/cloud-x.json"), b"{}").unwrap();

        assert_eq!(relocated_state_dir(&old, &new), new);
        assert!(new.join("playtime.json").exists());
        assert!(new.join("contexts/cloud-x.json").exists());
        assert!(!old.exists());
    }

    #[test]
    fn si_la_mudanza_no_sale_se_sigue_leyendo_del_sitio_viejo() {
        let tmp = tempfile::tempdir().unwrap();
        let old = tmp.path().join("local/hoard/data");
        seeded(&old, "playtime.json");
        // An impossible destination: its parent is a file, so neither
        // `create_dir_all` nor `rename` can manage it.
        let blocker = tmp.path().join("roaming");
        std::fs::write(&blocker, b"no soy un directorio").unwrap();
        let new = blocker.join("hoard/data");

        // What must NOT happen is returning `new`: the data is still in `old` and
        // the caller would create an empty folder on top of the history.
        assert_eq!(relocated_state_dir(&old, &new), old);
        assert!(old.join("playtime.json").exists());
    }
}
