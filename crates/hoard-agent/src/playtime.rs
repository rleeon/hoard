//! PLAYTIME — horas reales jugadas, atribuidas por día local.
//!
//! El poll de procesos del agente ([`crate::agent::process_poll`]) ya sabe qué
//! saves rastreados tienen su proceso de juego vivo en cada tick. Este módulo
//! acumula ese tiempo: en cada tick suma el intervalo transcurrido a los juegos
//! que siguen vivos, agrupado por día local y por juego. Es la fuente de verdad
//! de "horas jugadas" que consume el recap (hoard-wrapple) vía
//! `list_playtime` — datos 100% locales, nada sale de la máquina.
//!
//! MODELO DE ACUMULACIÓN (anclas): por cada save vivo guardamos el instante de
//! la última atribución (`anchors`, sólo en memoria). En el tick siguiente, si
//! el juego sigue vivo, sumamos `now - ancla` a la cubeta del día y movemos el
//! ancla a `now`. Un juego que deja de estar vivo pierde su ancla (su cola
//! final, ≤ un intervalo de poll, no se cuenta — error acotado e irrelevante a
//! escala de horas). El salto se capa (`max_step_secs`) para que un suspend/
//! resume no cuente horas de máquina dormida como tiempo de juego.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Offset local capturado una sola vez. El desktop lo siembra desde `run()`
/// *antes* de arrancar hilos (ver `lib.rs`), porque `time`'s
/// `current_local_offset` se niega a leer el entorno en un proceso ya
/// multi-hilo. Si nadie lo siembra (CLI headless), caemos a UTC — el recap es
/// una función de escritorio, así que el CLI no pierde nada visible.
static LOCAL_OFFSET: OnceLock<time::UtcOffset> = OnceLock::new();

/// Siembra el offset local. Idempotente; sólo el primer valor cuenta.
pub fn set_local_offset(off: time::UtcOffset) {
    let _ = LOCAL_OFFSET.set(off);
}

fn local_offset() -> time::UtcOffset {
    // Sin sembrar (CLI headless, o el desktop no llamó a `set_local_offset`)
    // caemos a UTC. No usamos `current_local_offset` aquí: requiere la feature
    // `local-offset` de `time` y, además, falla en un proceso multi-hilo — por
    // eso el desktop captura el offset pre-hilos y lo siembra.
    *LOCAL_OFFSET.get_or_init(|| time::UtcOffset::UTC)
}

/// Clave de día local `YYYY-MM-DD` para un instante en epoch-ms. Usa el mismo
/// reloj local que la UI (misma máquina), así las cubetas casan con el binning
/// por día del heatmap.
fn local_day_key(now_ms: u64) -> String {
    let secs = (now_ms / 1000) as i64;
    let dt = time::OffsetDateTime::from_unix_timestamp(secs)
        .unwrap_or(time::OffsetDateTime::UNIX_EPOCH)
        .to_offset(local_offset());
    let d = dt.date();
    format!("{:04}-{:02}-{:02}", d.year(), u8::from(d.month()), d.day())
}

#[cfg(test)]
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Cubetas de tiempo jugado persistidas en disco, más anclas en memoria.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlaytimeStore {
    /// Día local `YYYY-MM-DD` → segundos jugados ese día (todos los juegos).
    #[serde(default)]
    days: BTreeMap<String, u64>,
    /// `game_slug` → segundos jugados acumulados (histórico).
    #[serde(default)]
    by_game: BTreeMap<String, u64>,
    /// Desglose cruzado día → (`game_slug` → segundos) — qué se jugó cada día y
    /// a qué. Campo nuevo: sólo se rellena de aquí en adelante, así que para
    /// días viejos la suma de sus juegos puede ser < `days[día]` (ver
    /// [`Self::upload_rows`], que añade una fila remanente para cuadrar).
    #[serde(default)]
    daily_by_game: BTreeMap<String, BTreeMap<String, u64>>,
    /// Segundos jugados acumulados (histórico).
    #[serde(default)]
    total_secs: u64,

    /// Sólo memoria: `save_id` → instante (epoch-ms) de la última atribución.
    #[serde(skip)]
    anchors: HashMap<String, u64>,
    #[serde(skip)]
    dirty: bool,
    #[serde(skip)]
    last_flush_ms: u64,
}

/// Forma serializable que el comando `list_playtime` devuelve a la UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaytimeSummary {
    pub days: BTreeMap<String, u64>,
    pub by_game: BTreeMap<String, u64>,
    /// Desglose cruzado día → (`game_slug` → segundos): qué se jugó cada día y
    /// cuánto. Alimenta el detalle por día del recap (clic en un cuadro).
    #[serde(default)]
    pub daily_by_game: BTreeMap<String, BTreeMap<String, u64>>,
    pub total_secs: u64,
}

/// Una fila atómica de tiempo jugado para subir al cloud: "este día, a este
/// juego, tantos segundos". El servidor las upserta por
/// `(user_id, device_fp, day, game_slug)`. El recap lee el agregado de vuelta.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaytimeRow {
    pub day: String,
    pub game_slug: String,
    pub secs: u64,
}

impl PlaytimeStore {
    /// Ruta por defecto en disco, junto a `state.json` / `correlation.json`.
    pub fn default_path() -> Result<PathBuf> {
        Ok(crate::config::CliConfig::state_dir()?.join("playtime.json"))
    }

    /// Carga el store; un fichero ausente o corrupto produce uno vacío (las
    /// horas son acumulables de nuevo, no son críticas).
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
        let text = serde_json::to_string_pretty(self).context("serializing playtime store")?;
        std::fs::write(path, text).with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }

    /// Acumula tiempo para los saves vivos este tick. `running` son pares
    /// `(save_id, game_slug)`. `max_step_secs` capa el salto por ancla (anti
    /// suspend/resume).
    pub fn accrue(&mut self, running: &[(String, String)], now: u64, max_step_secs: u64) {
        let live: HashSet<&str> = running.iter().map(|(id, _)| id.as_str()).collect();
        for (id, slug) in running {
            let prev = self.anchors.insert(id.clone(), now);
            let Some(prev) = prev else {
                // Primera observación: sólo ancla, sin atribuir (no sabemos
                // cuánto llevaba vivo antes de que lo viéramos).
                continue;
            };
            if now <= prev {
                continue;
            }
            let mut secs = (now - prev) / 1000;
            if secs == 0 {
                continue;
            }
            if secs > max_step_secs {
                secs = max_step_secs;
            }
            let day = local_day_key(now);
            *self.days.entry(day.clone()).or_insert(0) += secs;
            *self.by_game.entry(slug.clone()).or_insert(0) += secs;
            *self
                .daily_by_game
                .entry(day)
                .or_default()
                .entry(slug.clone())
                .or_insert(0) += secs;
            self.total_secs += secs;
            self.dirty = true;
        }
        // Olvida las anclas de juegos que ya no están vivos.
        self.anchors.retain(|id, _| live.contains(id.as_str()));
    }

    /// Persiste si hay cambios y han pasado al menos 30 s desde el último
    /// flush — evita escribir el JSON en cada tick.
    pub fn flush_if_due(&mut self, path: Option<&Path>, now: u64) {
        if !self.dirty {
            return;
        }
        if now.saturating_sub(self.last_flush_ms) < 30_000 {
            return;
        }
        self.flush(path, now);
    }

    /// Persiste ya (lo llama el agente al parar un juego, para que el recap
    /// esté fresco nada más salir).
    pub fn flush(&mut self, path: Option<&Path>, now: u64) {
        if !self.dirty {
            return;
        }
        if let Some(p) = path {
            if let Err(e) = self.save(p) {
                tracing::debug!(error = %e, "agent: failed to persist playtime store");
                return;
            }
        }
        self.dirty = false;
        self.last_flush_ms = now;
    }

    /// Copia serializable para la UI.
    pub fn summary(&self) -> PlaytimeSummary {
        PlaytimeSummary {
            days: self.days.clone(),
            by_game: self.by_game.clone(),
            daily_by_game: self.daily_by_game.clone(),
            total_secs: self.total_secs,
        }
    }

    /// Filas `(día, juego, segundos)` para subir al cloud. Emite el desglose
    /// real de `daily_by_game` y, por cada día, una fila remanente bajo el slug
    /// `__other__` con `days[día] − Σ juegos` cuando es positivo: así los días
    /// anteriores a este desglose (cuyo total vive sólo en `days`) siguen
    /// cuadrando en el agregado del servidor sin inventar a qué juego fueron.
    pub fn upload_rows(&self) -> Vec<PlaytimeRow> {
        let mut rows = Vec::new();
        for (day, total) in &self.days {
            let per_game = self.daily_by_game.get(day);
            let mut attributed = 0u64;
            if let Some(games) = per_game {
                for (slug, secs) in games {
                    if *secs == 0 {
                        continue;
                    }
                    attributed += *secs;
                    rows.push(PlaytimeRow {
                        day: day.clone(),
                        game_slug: slug.clone(),
                        secs: *secs,
                    });
                }
            }
            let remainder = total.saturating_sub(attributed);
            if remainder > 0 {
                rows.push(PlaytimeRow {
                    day: day.clone(),
                    game_slug: "__other__".into(),
                    secs: remainder,
                });
            }
        }
        rows
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> PlaytimeStore {
        PlaytimeStore::default()
    }

    #[test]
    fn first_observation_only_anchors() {
        let mut s = store();
        s.accrue(&[("a".into(), "game".into())], 10_000, 60);
        assert_eq!(s.total_secs, 0, "no time on the very first sighting");
        assert!(s.anchors.contains_key("a"));
    }

    #[test]
    fn accrues_between_ticks() {
        let mut s = store();
        s.accrue(&[("a".into(), "game".into())], 10_000, 60);
        // 12 s después.
        s.accrue(&[("a".into(), "game".into())], 22_000, 60);
        assert_eq!(s.total_secs, 12);
        assert_eq!(*s.by_game.get("game").unwrap(), 12);
        assert_eq!(s.days.values().sum::<u64>(), 12);
    }

    #[test]
    fn caps_implausible_jumps() {
        let mut s = store();
        s.accrue(&[("a".into(), "g".into())], 0, 40);
        // Una hora después (máquina dormida): sólo cuenta el cap.
        s.accrue(&[("a".into(), "g".into())], 3_600_000, 40);
        assert_eq!(s.total_secs, 40);
    }

    #[test]
    fn drops_anchor_when_game_stops() {
        let mut s = store();
        s.accrue(&[("a".into(), "g".into())], 0, 60);
        s.accrue(&[("a".into(), "g".into())], 10_000, 60);
        assert_eq!(s.total_secs, 10);
        // Tick sin juegos vivos: olvida el ancla.
        s.accrue(&[], 20_000, 60);
        assert!(!s.anchors.contains_key("a"));
        // Si vuelve, no atribuye el hueco (re-ancla).
        s.accrue(&[("a".into(), "g".into())], 30_000, 60);
        assert_eq!(s.total_secs, 10);
    }

    #[test]
    fn upload_rows_split_and_remainder() {
        let mut s = store();
        // Día con desglose real: 12 s a "g".
        s.accrue(&[("a".into(), "g".into())], 10_000, 60);
        s.accrue(&[("a".into(), "g".into())], 22_000, 60);
        // Simula un día histórico sin desglose (sólo en `days`).
        s.days.insert("2020-01-01".into(), 100);
        let rows = s.upload_rows();
        // La fila real "g".
        let g = rows
            .iter()
            .find(|r| r.game_slug == "g")
            .expect("row for g");
        assert_eq!(g.secs, 12);
        // El día histórico se vuelca entero como remanente.
        let other = rows
            .iter()
            .find(|r| r.day == "2020-01-01" && r.game_slug == "__other__")
            .expect("remainder row");
        assert_eq!(other.secs, 100);
        // El día con desglose completo no genera remanente.
        assert!(
            !rows
                .iter()
                .any(|r| r.day != "2020-01-01" && r.game_slug == "__other__"),
            "fully-attributed day must not emit a remainder"
        );
    }

    #[test]
    fn round_trips_to_disk() {
        let tmp = std::env::temp_dir().join(format!("hoard-playtime-{}.json", now_ms()));
        let mut s = store();
        s.accrue(&[("a".into(), "g".into())], 0, 60);
        s.accrue(&[("a".into(), "g".into())], 5_000, 60);
        s.flush(Some(&tmp), 5_000);
        let loaded = PlaytimeStore::load(&tmp);
        assert_eq!(loaded.total_secs, 5);
        assert_eq!(*loaded.by_game.get("g").unwrap(), 5);
        let _ = std::fs::remove_file(&tmp);
    }
}
