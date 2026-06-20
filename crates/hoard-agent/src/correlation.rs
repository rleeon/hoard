//! DETECCIÓN — correlación proceso↔escritura (fase 3, ADR 0020). LA JOYA.
//!
//! La señal más fiable de que una carpeta es un save: fue reescrita mientras
//! un proceso de JUEGO (no del sistema) estaba vivo. No depende del nombre
//! ni de la extensión, así que captura saves con nombres GUID o en idiomas
//! raros que el name-signal jamás atraparía (el benchmark del manifest mide
//! ~6% de recall sólo por nombre — ver `scoring::bench`).
//!
//! MECANISMO (este módulo): un store de observaciones persistido. Cuando el
//! `SaveWatcher` emite una escritura sobre un dir, el agente llama a
//! [`CorrelationStore::record`] con la foto de procesos vivos
//! ([`sample_game_processes`]); el store atribuye la escritura al proceso de
//! juego más probable y la guarda. El scoring consulta
//! [`CorrelationStore::signal_for`] y suma [`CORRELATION_BONUS`] (+0.50).
//!
//! NOTA DE INTEGRACIÓN: el bucle observador (cablear `SaveWatcher` sobre los
//! roots de `roots.rs` + muestreo de `sysinfo` en cada evento) vive en el
//! scheduler del agente y se cablea en un paso posterior. Este módulo provee
//! el store, el muestreo y la señal, todo testeable en aislamiento. En frío
//! (juego nunca observado) la señal vale 0 — es el límite conocido del ADR:
//! la correlación necesita al menos una sesión de juego observada.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sysinfo::System;

/// Bonus de score de ADR 0020 §2 para correlación proceso↔escritura. Es la
/// señal dominante: por sí sola más cualquier evidencia débil ya cruza el
/// cutoff de auto-confirmado (0.60).
pub const CORRELATION_BONUS: f32 = 0.50;

/// Procesos que NUNCA son el juego: sistema, shells, navegadores, el propio
/// Hoard y los launchers/overlays (que corren junto al juego pero no SON el
/// juego). Match por substring case-insensitive sobre el nombre del proceso.
const NON_GAME_PROCESS: &[&str] = &[
    // Sistema / shells.
    "svchost",
    "systemd",
    "kthreadd",
    "kworker",
    "gnome-shell",
    "plasmashell",
    "kwin",
    "xorg",
    "wayland",
    "pipewire",
    "pulseaudio",
    "dbus",
    "csrss",
    "winlogon",
    "services.exe",
    "lsass",
    "dwm.exe",
    "explorer.exe",
    "windowserver",
    "loginwindow",
    // Navegadores / runtimes genéricos.
    "chrome",
    "chromium",
    "firefox",
    "msedge",
    "brave",
    "safari",
    "electron",
    // Demonios del sistema observados colándose como "juego" (atribuían
    // escrituras a dockerd, avahi, etc.). Match por substring del nombre.
    "dockerd",
    "containerd",
    "multipathd",
    "avahi-daemon",
    "systemd-",
    "gvfsd",
    "gsd-",
    "pipewire-",
    "wireplumber",
    // Launchers / overlays / clientes (no son el juego en sí).
    "steamwebhelper",
    "steam.exe",
    "steam ",
    "epicgameslauncher",
    "galaxyclient",
    "battle.net",
    "origin.exe",
    "eadesktop",
    "ubisoftconnect",
    "uplay",
    // Infraestructura Wine/Proton/Steam-runtime en Linux: envuelve al juego
    // pero no es el juego. Sin esto la atribución `.first()` se queda con un
    // wrapper (`reaper`, `proton`, `wineserver`) en vez del ejecutable real.
    // Match por substring, así que `wineserver`, `wine64-preloader`,
    // `winedevice.exe` caen todos bajo "wine".
    "wine",
    "proton",
    "pressure-vessel",
    "pv-bwrap",
    "srt-bwrap",
    "reaper",
    "steam-runtime",
    "gameoverlayui",
    "steamerrorreporter",
    "winedevice",
    "plugplay.exe",
    "rpcss.exe",
    "conhost.exe",
    // El propio Hoard.
    "hoard",
];

/// Un proceso vivo candidato a juego.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GameProcess {
    pub name: String,
    pub exe: Option<PathBuf>,
}

/// Prefijos de ruta de SISTEMA: un ejecutable bajo ellos no es un juego
/// (demonios, librerías, runtimes de apps empaquetadas). Los juegos viven en
/// el home del usuario (Steam, Wine `~/.wine*`, Lutris, Heroic, Flatpak data)
/// o en `/opt/<juego>` — nunca en `/usr` ni `/lib`. Filtra el grueso de la
/// basura observada (hilos de Brave en `/opt/brave.com`, Electron en
/// `/usr/lib/...`, `gsd-*` en `/usr/libexec`).
const SYSTEM_EXE_PREFIXES: &[&str] = &["/usr/", "/lib", "/lib64", "/bin", "/sbin", "/run/"];

/// `true` si el proceso parece un juego (no sistema/launcher/navegador).
/// Usa el nombre y, cuando está, la ruta del ejecutable: un nombre de hilo
/// genérico (`ThreadPoolForeg`) no delata al navegador, pero su exe sí.
pub fn is_game_like(name: &str, exe: Option<&Path>) -> bool {
    let lower = name.trim().to_lowercase();
    if lower.is_empty() {
        return false;
    }
    if NON_GAME_PROCESS.iter().any(|bad| lower.contains(bad)) {
        return false;
    }
    if let Some(exe) = exe {
        let exe_str = exe.to_string_lossy().to_lowercase();
        if SYSTEM_EXE_PREFIXES.iter().any(|p| exe_str.starts_with(p)) {
            return false;
        }
        // El nombre del proceso puede ser un hilo genérico; mira también el
        // basename del ejecutable contra la lista negra.
        if let Some(base) = exe.file_name().and_then(|s| s.to_str()) {
            let base = base.to_lowercase();
            if NON_GAME_PROCESS.iter().any(|bad| base.contains(bad)) {
                return false;
            }
        }
    }
    true
}

/// Foto de los procesos vivos que parecen juegos. `sys` debe venir ya
/// refrescado por el caller — el agente ya mantiene y refresca un `System`
/// en su tick de actividad, así que la idea es reutilizarlo, no crear otro.
///
/// Dos filtros duros además de [`is_game_like`], aprendidos de basura real
/// observada en producción (hilos de kernel `cpuhp/0`, `nv_open_q`,
/// `ib_srv_wkr-2`, threads `tokio-runtime-w`… atribuidos como "juego"):
/// - se descartan los HILOS (kernel y userland): un juego es un PROCESO, no
///   un hilo de otro; `thread_kind().is_some()` los delata;
/// - se exige un EJECUTABLE en disco: un juego siempre tiene binario; los
///   hilos de kernel no (`exe()` es `None`). Esto es load-bearing: la
///   correlación alimenta el playtime, y atribuir un save a un worker de
///   kernel —que vive 24/7— acumularía horas para siempre.
pub fn sample_game_processes(sys: &System) -> Vec<GameProcess> {
    let mut out = Vec::new();
    for proc in sys.processes().values() {
        if proc.thread_kind().is_some() {
            continue;
        }
        let Some(exe) = proc.exe().map(|p| p.to_path_buf()) else {
            continue;
        };
        let name = proc.name().to_string_lossy().to_string();
        if is_game_like(&name, Some(&exe)) {
            out.push(GameProcess {
                name,
                exe: Some(exe),
            });
        }
    }
    out
}

/// Una escritura observada en un dir, atribuida a un proceso de juego.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteObservation {
    pub dir: PathBuf,
    pub process_name: String,
    pub exe: Option<PathBuf>,
    pub observed_at_ms: u64,
    /// Cuántas veces se ha re-observado (más hits ⇒ más confianza).
    pub hits: u32,
}

/// Store persistido de correlaciones, indexado por dir observado.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CorrelationStore {
    #[serde(default)]
    observations: HashMap<PathBuf, WriteObservation>,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

impl CorrelationStore {
    /// Ruta por defecto en disco, junto a `state.json`.
    pub fn default_path() -> Result<PathBuf> {
        Ok(crate::config::CliConfig::state_dir()?.join("correlation.json"))
    }

    /// Carga el store; un fichero ausente o corrupto produce uno vacío
    /// (las observaciones son recolectables de nuevo, no son críticas).
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        let text = serde_json::to_string_pretty(self).context("serializing correlation store")?;
        std::fs::write(path, text).with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }

    /// Registra una escritura en `dir` ocurrida mientras `processes` estaban
    /// vivos. Si no hay ningún proceso de juego, no hay correlación y no se
    /// guarda nada. Con varios juegos vivos se atribuye al primero (la
    /// atribución fina es best-effort; para la señal "esto es un save" basta
    /// con que CUALQUIER juego estuviera vivo).
    pub fn record(&mut self, dir: &Path, processes: &[GameProcess]) {
        let Some(primary) = processes.first() else {
            return;
        };
        let entry = self
            .observations
            .entry(dir.to_path_buf())
            .or_insert_with(|| WriteObservation {
                dir: dir.to_path_buf(),
                process_name: primary.name.clone(),
                exe: primary.exe.clone(),
                observed_at_ms: 0,
                hits: 0,
            });
        entry.process_name = primary.name.clone();
        entry.exe = primary.exe.clone();
        entry.observed_at_ms = now_ms();
        entry.hits = entry.hits.saturating_add(1);
    }

    /// Devuelve la observación que corrobora `dir`: coincidencia exacta o
    /// cualquier dir observado que sea `dir` o un ancestro suyo (el watcher
    /// es recursivo, así que la escritura puede registrarse en un padre).
    pub fn signal_for(&self, dir: &Path) -> Option<&WriteObservation> {
        let mut cur = Some(dir);
        while let Some(d) = cur {
            if let Some(o) = self.observations.get(d) {
                return Some(o);
            }
            cur = d.parent();
        }
        None
    }

    /// Nombre de proceso atribuido a `dir`, sin la extensión `.exe`. Útil
    /// para la atribución de fase 4 (nombrar el save en la librería).
    pub fn attributed_name(&self, dir: &Path) -> Option<String> {
        self.signal_for(dir).map(|o| {
            o.process_name
                .strip_suffix(".exe")
                .unwrap_or(&o.process_name)
                .to_string()
        })
    }

    pub fn is_empty(&self) -> bool {
        self.observations.is_empty()
    }

    pub fn len(&self) -> usize {
        self.observations.len()
    }
}

/// Scoring estático ([`crate::scoring::score_dir`]) más el bonus de
/// correlación si el store corrobora el dir. Mantiene `scoring.rs` libre de
/// dependencias de runtime; el bonus se suma aquí.
pub fn score_with_correlation(
    path: &Path,
    name: &str,
    store: &CorrelationStore,
) -> crate::scoring::ScoreBreakdown {
    let mut b = crate::scoring::score_dir(path, name);
    if let Some(obs) = store.signal_for(path) {
        b.score = (b.score + CORRELATION_BONUS).clamp(0.0, 1.0);
        b.reasons
            .push(format!("process correlation ({})", obs.process_name));
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_game_like_rejects_system_and_launchers() {
        assert!(!is_game_like("svchost.exe", None));
        assert!(!is_game_like("steamwebhelper", None));
        assert!(!is_game_like("hoard-agent", None));
        assert!(!is_game_like("chrome", None));
        assert!(!is_game_like("", None));
        // Un juego cualquiera pasa.
        assert!(is_game_like("eldenring.exe", None));
        assert!(is_game_like("Hades.exe", None));
    }

    #[test]
    fn is_game_like_rejects_wine_proton_infrastructure() {
        // Los wrappers de Linux conviven con el juego pero no SON el juego;
        // sin filtrarlos, `.first()` atribuía el save a un wrapper.
        for n in [
            "wineserver",
            "wine64-preloader",
            "winedevice.exe",
            "proton",
            "reaper",
            "pv-bwrap",
            "pressure-vessel-wrap",
            "steam-runtime-launcher-service",
            "gameoverlayui",
        ] {
            assert!(!is_game_like(n, None), "{n} debería filtrarse");
        }
        // El ejecutable real del juego (mismo nombre que vería sysinfo bajo
        // Proton) sigue pasando.
        assert!(is_game_like("eu5.exe", None));
    }

    #[test]
    fn is_game_like_rejects_by_exe_path_and_basename() {
        // Hilo genérico de Brave: el nombre no delata, el exe sí.
        assert!(!is_game_like(
            "ThreadPoolForeg",
            Some(Path::new("/opt/brave.com/brave/brave"))
        ));
        // Runtime de Electron / apps en /usr.
        assert!(!is_game_like(
            "electro:disk$0",
            Some(Path::new(
                "/usr/lib/claude-desktop/node_modules/electron/dist/electron"
            ))
        ));
        assert!(!is_game_like(
            "gmain",
            Some(Path::new("/usr/libexec/gsd-sound"))
        ));
        // Un juego de Wine en el home pasa.
        assert!(is_game_like(
            "factorio.exe",
            Some(Path::new(
                "/home/u/.wine64/drive_c/Factorio/bin/factorio.exe"
            ))
        ));
    }

    #[test]
    fn record_and_signal_with_ancestor_match() {
        let mut store = CorrelationStore::default();
        let save = PathBuf::from("/home/u/.local/share/Game/Saves");
        store.record(
            &save,
            &[GameProcess {
                name: "game.exe".into(),
                exe: None,
            }],
        );
        // Coincidencia exacta.
        assert!(store.signal_for(&save).is_some());
        // Un hijo del dir observado también corrobora (watcher recursivo).
        assert!(store.signal_for(&save.join("slot1")).is_some());
        // Un dir no relacionado, no.
        assert!(store.signal_for(Path::new("/etc")).is_none());
        assert_eq!(store.attributed_name(&save).as_deref(), Some("game"));
    }

    #[test]
    fn no_game_process_records_nothing() {
        let mut store = CorrelationStore::default();
        store.record(Path::new("/x"), &[]);
        assert!(store.is_empty());
    }

    #[test]
    fn correlation_rescues_invisible_folder() {
        // Carpeta con nombre opaco (GUID) y vacía: estáticamente es
        // INVISIBLE — por debajo de SCORE_POSSIBLE, el walker la descarta.
        let tmp = std::env::temp_dir().join("hoard-corr-test-guid-1234");
        let _ = std::fs::create_dir_all(&tmp);
        let static_only = crate::scoring::score_dir(&tmp, "guid-1234");
        assert!(static_only.score < crate::scoring::SCORE_POSSIBLE);

        // La correlación sola (+0.50) la sube a "posible": deja de ser
        // descartada. Para AUTO-confirmar (≥0.60) el ADR pide además una
        // señal débil — eso es deliberado.
        let mut store = CorrelationStore::default();
        store.record(
            &tmp,
            &[GameProcess {
                name: "weirdgame.exe".into(),
                exe: None,
            }],
        );
        let correlated = score_with_correlation(&tmp, "guid-1234", &store);
        assert!(correlated.score >= crate::scoring::SCORE_POSSIBLE);
        assert!(correlated.score < crate::scoring::SCORE_CONFIRMED);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn correlation_plus_content_auto_confirms() {
        // Camino dorado del ADR: nombre opaco + un save reciente +
        // correlación de proceso ⇒ auto-confirmado con margen holgado.
        let tmp = std::env::temp_dir().join(format!("hoard-corr-golden-{}", now_ms()));
        let _ = std::fs::create_dir_all(&tmp);
        std::fs::write(tmp.join("game.sav"), b"binary").unwrap();

        let mut store = CorrelationStore::default();
        store.record(
            &tmp,
            &[GameProcess {
                name: "weirdgame.exe".into(),
                exe: None,
            }],
        );
        let correlated = score_with_correlation(&tmp, "12345", &store);
        assert!(correlated.score >= crate::scoring::SCORE_CONFIRMED);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn store_round_trips_to_disk() {
        let tmp = std::env::temp_dir().join(format!("hoard-corr-{}.json", now_ms()));
        let mut store = CorrelationStore::default();
        store.record(
            Path::new("/a/b/Saves"),
            &[GameProcess {
                name: "g.exe".into(),
                exe: Some(PathBuf::from("/a/b/g.exe")),
            }],
        );
        store.save(&tmp).unwrap();
        let loaded = CorrelationStore::load(&tmp);
        assert_eq!(loaded.len(), 1);
        assert!(loaded.signal_for(Path::new("/a/b/Saves")).is_some());
        let _ = std::fs::remove_file(&tmp);
    }
}
