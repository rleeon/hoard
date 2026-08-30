use anyhow::{Context, Result};
use hoard_core::ids::{GameSlug, Repair, SaveId};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use time::OffsetDateTime;

/// Process-wide identifier of the sync context whose `saves` map is live.
///
/// The `saves` map (save_id → version cursor + local path) only means anything
/// for the account/server that owns those saves: a `save_id` and its
/// `last_version_num` are minted server-side, so replaying one account's cursors
/// against another (or against a self-hosted server) makes the next upload claim
/// a `base_version` the target never had → `non_fast_forward`, and surfaces
/// "saves from another account/self-hosted" residue. So each context keeps its
/// `saves` in its own file (`contexts/<id>.json`); device-level prefs
/// (`manual_paths`, `ignored_slugs`, `playtime_excluded`) stay global in
/// `device.json`.
///
/// The desktop sets this at boot and on every login/logout/account switch
/// (mirroring `current_client`'s self-hosted-wins-else-cloud selection). When
/// unset — the headless CLI, which is self-hosted only — the id is derived from
/// the configured server URL.
static ACTIVE_CONTEXT: RwLock<Option<String>> = RwLock::new(None);

/// Override the active sync context. `None` clears the override so the id falls
/// back to the self-hosted server URL from [`crate::config::CliConfig`].
pub fn set_active_context(ctx: Option<String>) {
    *ACTIVE_CONTEXT.write().unwrap() = ctx;
}

/// Context id for a Hoard Cloud account, keyed by its Supabase `user_id` (a
/// UUID, so already filesystem-safe).
pub fn cloud_context(user_id: &str) -> String {
    format!("cloud-{user_id}")
}

/// Context id for a self-hosted server. The URL can carry `:` and `/`, so we
/// key on a short stable SHA-256 prefix of the normalised URL instead of the
/// raw string.
pub fn selfhosted_context(server_url: &str) -> String {
    let mut h = Sha256::new();
    h.update(server_url.trim_end_matches('/').as_bytes());
    format!("selfhosted-{}", hex::encode(&h.finalize()[..8]))
}

/// Resolve the id of the context whose `saves` file should be loaded now.
pub fn current_context_id() -> String {
    if let Some(ctx) = ACTIVE_CONTEXT.read().unwrap().clone() {
        return ctx;
    }
    match crate::config::CliConfig::load_default() {
        Ok((cfg, _)) => selfhosted_context(&cfg.server.url),
        Err(_) => "default".to_string(),
    }
}

/// Per-save local metadata: which directory on disk maps to which remote save.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveState {
    pub local_path: PathBuf,
    pub game_slug: String,
    pub label: String,
    #[serde(with = "time::serde::rfc3339::option", default)]
    pub last_backup_at: Option<OffsetDateTime>,
    pub last_version_num: Option<i64>,
    /// User-toggled pause. When true the agent skips this save (no process
    /// matching, no FS watch) but the row stays in `state.json` so flipping
    /// it back on doesn't lose the path mapping. `default` lets us read
    /// older state files without migration.
    #[serde(default)]
    pub paused: bool,
    /// Sync preset id for this save (see [`crate::presets`]). Resolves into a
    /// [`crate::presets::SavePolicy`] of overrides layered on the global
    /// config. `None`/absent = the implicit `standard` preset (inherit
    /// everything). Auto-assigned from [`crate::presets::builtin_preset_for`]
    /// on track for known-quirky games; user-overridable. `default` keeps
    /// older `state.json` files loading without migration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
    /// Skip-by-set-hash cache (ADR 0019). A cheap signature over the save's
    /// `(relative_path, size, mtime)` set as of the last successful upload.
    /// Before backing up, the agent recomputes the signature; if it's
    /// unchanged the watcher fired on a settle that touched nothing, so the
    /// upload is a no-op and skipped. `default` keeps older state files
    /// loading without migration.
    #[serde(default)]
    pub set_hash: Option<String>,
    /// Manually-pinned process executable names that mark this save as
    /// "playing" (case-insensitive exact match in the agent's process poll).
    /// Empty = derive from the slug via [`crate::presets::builtin_processes_for`]
    /// as before. This is how a manually-added emulator save (whose slug isn't
    /// in any catalog) keeps its "is the user playing" signal across restarts:
    /// the user-picked emulator exe is persisted here instead of being
    /// recomputed from the slug. `default` keeps older `state.json` files
    /// loading without migration.
    #[serde(default)]
    pub processes: Vec<String>,
    /// Estos nombres de proceso los comparten VARIOS saves rastreados, así que
    /// ver el proceso no dice cuál de ellos se está jugando.
    ///
    /// Es el caso de una consola emulada partida en una carpeta por juego:
    /// diez títulos de la misma máquina listan el mismo ejecutable, y contarlos
    /// todos como "jugando" en cuanto arranca el emulador inventaría horas para
    /// nueve de ellos y vetaría el sync de nueve partidas que nadie ha tocado.
    /// Cuando esto está puesto, el nombre de proceso deja de bastar por sí solo
    /// y hace falta que ADEMÁS haya actividad en esta carpeta concreta (ver
    /// `sample_running` en `agent.rs`). `default` mantiene cargables los
    /// `state.json` anteriores.
    #[serde(default)]
    pub shared_processes: bool,
    /// Does a restore write this game's config?
    ///
    /// The files [`hoard_core::kernel::fileclass`] classifies as `DeviceLocal`
    /// — `graphics.ini`, `settings.cfg`, whatever carries THIS monitor's
    /// resolution — are always uploaded but by default never written back:
    /// restoring them from one PC onto another is the short road to a game that
    /// boots to a black screen. The switch in the restore dialog skips that
    /// once; this settles it for the game.
    ///
    /// It is **per game** because the answer is. In one game the config and the
    /// save live in the same file and it has to be written; in another it is the
    /// resolution and it must not be touched. A single global switch would have
    /// to be right for both at once, which it cannot be.
    ///
    /// `None` = undecided: not written, and the dialog keeps asking.
    /// `Some(true)` writes it **on automatic restores too**, which is what makes
    /// the setting worth having; `Some(false)` is an explicit no.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_device_local: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CliState {
    /// keyed by save_id (UUID string)
    #[serde(default)]
    pub saves: HashMap<String, SaveState>,
    /// User-supplied save-path overrides, keyed by game slug. When detection
    /// runs, any entry here wins over every heuristic (filesystem, Steam,
    /// Proton prefix, refinement). The detection pipeline tags the resulting
    /// row with `DetectionSource::ManualOverride` so the UI can show "manual"
    /// in the source badge. Set via [`Self::set_manual_path`], cleared via
    /// [`Self::clear_manual_path`]. `default` lets older `state.json` files
    /// load without migration.
    #[serde(default)]
    pub manual_paths: HashMap<String, PathBuf>,
    /// Slugs the user has explicitly blacklisted from the Library page. The
    /// detection pipeline runs to completion as usual; the filter happens at
    /// the edge of `list_detected_games` so the walker still benefits from
    /// install dirs we'd otherwise miss. Reactivatable from
    /// Settings → "Juegos ignorados". `default` keeps older `state.json`
    /// files loading without migration.
    #[serde(default)]
    pub ignored_slugs: HashSet<String>,
    /// Slugs the user has dropped from playtime-only tracking (the amber
    /// "Jugados, sin copia" list that feeds the recap). Playtime-only games
    /// are auto-enrolled from the installed-game scan + catalog; this set is
    /// the opt-out so a game the user doesn't want counted stops being
    /// re-added on the next scan. Distinct from [`Self::ignored_slugs`] (which
    /// is about save detection) so excluding one from the recap doesn't hide
    /// the other. `default` keeps older `state.json` files loading.
    #[serde(default)]
    pub playtime_excluded: HashSet<String>,
    /// Carpetas que el usuario ha descartado: la detección no vuelve a
    /// ofrecerlas, ni ellas ni nada por debajo.
    ///
    /// Complementa a [`Self::ignored_slugs`], que no basta para esto: el
    /// nombre de un hallazgo de fase 4 sale de la atribución, y la atribución
    /// cambia entre escaneos (la carpeta de Planet S pasó por ChatGPT,
    /// opencode y code). Con un slug distinto cada vez, ignorar por slug no
    /// sujeta nada — la misma carpeta reaparece con nombre nuevo. Por ruta sí.
    ///
    /// `default` mantiene cargando los `state.json` antiguos.
    #[serde(default)]
    pub excluded_paths: Vec<PathBuf>,
    /// Slugs que [`cleanse`] marcó al cargar: bien formados pero **degenerados**
    /// (tokens de fontanería o el nombre de usuario del sistema). No se
    /// persiste (`skip`): se recalcula en cada carga a partir de lo que hay en
    /// disco, así que el fichero de estado no cambia de forma y el Slice 5
    /// —dueño de la limpieza durable— no hereda un campo que migrar.
    ///
    /// **Derivado**: lo recalcula [`cleanse`] en cada carga; escribirlo a mano
    /// no tiene efecto duradero. Consultable con [`Self::is_slug_quarantined`].
    #[serde(skip)]
    pub quarantined_slugs: HashSet<String>,
    /// Ids de save de `saves` que no son UUID canónicos. Mismo trato: se marcan
    /// y se dejan donde están (borrarlos dejaría el save sin rastrear y sin
    /// forma de recuperar su ruta local). **Derivado**, como
    /// [`Self::quarantined_slugs`].
    #[serde(skip)]
    pub quarantined_save_ids: HashSet<String>,
}

/// On-disk shape of `device.json`: the machine-level prefs that are identical
/// across every account and self-hosted server. Split out of [`CliState`] so
/// switching context never disturbs them.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct DevicePrefs {
    #[serde(default)]
    manual_paths: HashMap<String, PathBuf>,
    #[serde(default)]
    ignored_slugs: HashSet<String>,
    #[serde(default)]
    playtime_excluded: HashSet<String>,
    /// Carpetas descartadas por el usuario. Es preferencia del DISPOSITIVO,
    /// no de la cuenta: una carpeta que aquí es basura puede ser legítima en
    /// otra máquina.
    #[serde(default)]
    excluded_paths: Vec<PathBuf>,
}

/// On-disk shape of `contexts/<id>.json`: the per-context `saves` map (save_id
/// → local metadata + server version cursor).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ContextSaves {
    #[serde(default)]
    saves: HashMap<String, SaveState>,
}

/// Read + deserialize a JSON state file, self-healing a corrupt one instead of
/// aborting. These files are rebuildable caches (every tracked save re-adopts
/// on the next detection pass), so a half-written file (crash, disk gremlin,
/// hand-edit gone wrong) is moved aside for forensics and we start clean.
fn load_json<T: serde::de::DeserializeOwned + Default>(path: &Path) -> Result<T> {
    if !path.exists() {
        return Ok(T::default());
    }
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    match serde_json::from_str::<T>(&text) {
        Ok(v) => Ok(v),
        Err(e) => {
            let backup = path.with_extension(format!(
                "json.corrupt-{}",
                OffsetDateTime::now_utc().unix_timestamp()
            ));
            match std::fs::rename(path, &backup) {
                Ok(()) => tracing::warn!(
                    error = %e, backup = %backup.display(),
                    "state file was corrupt; backed it up and started fresh"
                ),
                Err(re) => tracing::warn!(
                    error = %re, path = %path.display(),
                    "state file is corrupt and couldn't be moved aside; ignoring it"
                ),
            }
            Ok(T::default())
        }
    }
}

/// Serialize `value` to `path` (pretty JSON), creating the parent dir.
///
/// The write is atomic (see [`crate::atomic_write`]) because the recovery above
/// is expensive here: a plain `fs::write` truncates before it writes, and a
/// process that dies in that window leaves a 0-byte file that [`load_json`]
/// reads as corrupt. For `device.json` that costs the user their manual paths
/// and exclusions; for `contexts/<id>.json` it costs them the entire list of
/// tracked saves.
fn save_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let text = serde_json::to_string_pretty(value).context("serializing state")?;
    crate::atomic_write::write_atomic(path, text.as_bytes())
}

/// One-time migration of the pre-split monolithic `state.json`.
///
/// Before contexts, one `state.json` held both the device prefs and the `saves`
/// map for whatever account/server was last used. On first load after the split
/// we route that file's prefs into `device.json` and its `saves` into the file
/// for the *currently active* context (which is exactly the context those saves
/// belonged to, since it was the only one). Then we rename the legacy file aside
/// so we neither re-migrate nor let a downgrade resurrect stale cursors.
///
/// Guarded so it runs at most once: if `device.json` already exists we've
/// migrated (or started fresh post-split) and leave any lingering legacy file
/// untouched.
fn migrate_legacy_state(device_path: &Path, context_path: &Path) -> Result<()> {
    let legacy = crate::config::CliConfig::state_dir()?.join("state.json");
    if !legacy.exists() || device_path.exists() {
        return Ok(());
    }
    let old: CliState = load_json(&legacy)?;
    old.save_split(device_path, context_path)?;
    let archived = legacy.with_extension("json.migrated");
    match std::fs::rename(&legacy, &archived) {
        Ok(()) => tracing::info!(
            saves = old.saves.len(),
            context = %context_path.display(),
            "state: migrated monolithic state.json into device.json + per-context saves"
        ),
        Err(e) => tracing::warn!(
            error = %e,
            "state: migrated state.json but couldn't archive the old file"
        ),
    }
    Ok(())
}

/// Un slug del que ya nos hemos quejado en este proceso.
///
/// `cleanse` corre en **cada** carga de `state.json` — unas veinticinco veces
/// por hora — y el estado no se reescribe, así que el mismo slug envenenado se
/// redescubre íntegro cada vez. Dos usuarios con un save llamado `user`
/// generaron 3.669 avisos idénticos en tres días: eso no son 3.669 incidentes,
/// es uno visto 3.669 veces, y enterró el resto del log.
///
/// La primera vez se cuenta entera; las siguientes son `debug`. El conjunto es
/// por proceso: un reinicio del daemon vuelve a avisar una vez, que es
/// exactamente lo que se quiere de un problema que sigue ahí.
fn warn_once(slug: &str, emit: impl FnOnce()) {
    use std::sync::{Mutex, OnceLock};
    static SEEN: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let seen = SEEN.get_or_init(|| Mutex::new(HashSet::new()));
    let first = match seen.lock() {
        Ok(mut g) => g.insert(slug.to_string()),
        // Un lock envenenado no puede silenciar un aviso: se cuenta.
        Err(_) => true,
    };
    if first {
        emit();
    } else {
        tracing::debug!(slug, "state: slug still quarantined (already reported)");
    }
}

/// Pasa el estado recién leído por la **puerta indulgente** de
/// `hoard_core::ids` (ADR 0021, C.3).
///
/// El veneno ya está en disco —el save de `GSE Saves` quedó con el slug igual al
/// nombre de usuario de Windows, y eso convirtió cualquier app del perfil en
/// señal de "estás jugando"—, así que un `try_from` estricto aquí dejaría el
/// motor sin arrancar. Tres desenlaces por valor, ninguno es un error:
///
/// - **Válido** → intacto.
/// - **Recuperable** (mayúsculas, espacios, basura) → se re-deriva con el mismo
///   `slugify` que lo mintó, se avisa, y se usa el reparado. Un slug así hoy ni
///   siquiera podría subirse (la puerta del wire lo rechaza), así que repararlo
///   es lo único que devuelve ese save al sync.
/// - **Degenerado** (token de fontanería, nombre de usuario) → **no se toca**:
///   ya está bien formado y es la identidad `(user, game_slug, label)` que el
///   server conoce; renombrarlo crearía un save nuevo en la nube. Se marca en
///   [`CliState::is_slug_quarantined`] para que la correlación lo ignore.
///
/// La limpieza durable (reescribir el estado migrado) es del Slice 5; esto es la
/// reparación en memoria que hace que el motor arranque mientras tanto.
///
/// **El veredicto es `GameSlug::repair` y sólo él.** Antes se recomprobaba
/// además con `agent::is_generic_identity_token`, que amplía la lista estática
/// con los componentes del home de ESTA máquina. Eso es lo correcto para casar
/// procesos vivos (bajo `C:\Users\<user>\` cuelga todo) y lo incorrecto aquí:
/// la identidad de un save es `(user, game_slug, label)` en el servidor y tiene
/// que significar lo mismo en todos los equipos. Con el criterio local, un save
/// llamado como el usuario quedaba en cuarentena en su portátil y limpio en el
/// de al lado, para el mismo `state.json` sincronizado.
fn cleanse(state: &mut CliState) {
    let mut quarantined: HashSet<String> = HashSet::new();

    let mut triage = |raw: &str| -> Option<String> {
        match GameSlug::repair(raw) {
            Repair::Clean(_) => None,
            Repair::Repaired { value, .. } => {
                warn_once(
                    raw,
                    || tracing::warn!(raw, repaired = %value, "state: slug inválido reparado al cargar"),
                );
                Some(value.into_inner())
            }
            Repair::Quarantined { reason, .. } => {
                // El motivo va en el campo, no en el texto: `Degenerate` y
                // `Unrecoverable` son cosas distintas y el mensaje llamaba
                // "irrecuperable" a los degenerados, que son la inmensa mayoría.
                warn_once(
                    raw,
                    || tracing::warn!(slug = raw, %reason, "state: slug quarantined at load"),
                );
                quarantined.insert(raw.to_string());
                None
            }
        }
    };

    for save in state.saves.values_mut() {
        if let Some(fixed) = triage(&save.game_slug) {
            save.game_slug = fixed;
        }
    }
    // Las prefs de dispositivo van keyed por slug; el mismo veneno vale.
    let manual: Vec<(String, PathBuf)> = state.manual_paths.drain().collect();
    for (slug, path) in manual {
        let key = triage(&slug).unwrap_or(slug);
        state.manual_paths.insert(key, path);
    }
    for set in [&mut state.ignored_slugs, &mut state.playtime_excluded] {
        let old: Vec<String> = set.drain().collect();
        for slug in old {
            set.insert(triage(&slug).unwrap_or(slug));
        }
    }

    // Un id de save que no es UUID nunca existirá server-side (el churn de
    // DOOM). Se marca, no se borra: la fila guarda la ruta local del save.
    let bad_ids: HashSet<String> = state
        .saves
        .keys()
        .filter(|id| SaveId::parse(id).is_err())
        .cloned()
        .collect();
    for id in &bad_ids {
        tracing::warn!(save_id = %id, "state: save_id que no es UUID; marcado");
    }

    state.quarantined_slugs = quarantined;
    state.quarantined_save_ids = bad_ids;
}

impl CliState {
    /// Global `device.json` path: machine-level prefs shared across contexts.
    pub fn device_path() -> Result<PathBuf> {
        Ok(crate::config::CliConfig::state_dir()?.join("device.json"))
    }

    /// Per-context saves file: `contexts/<id>.json`.
    pub fn context_path_for(ctx: &str) -> Result<PathBuf> {
        Ok(crate::config::CliConfig::state_dir()?
            .join("contexts")
            .join(format!("{ctx}.json")))
    }

    fn device_prefs(&self) -> DevicePrefs {
        DevicePrefs {
            manual_paths: self.manual_paths.clone(),
            ignored_slugs: self.ignored_slugs.clone(),
            excluded_paths: self.excluded_paths.clone(),
            playtime_excluded: self.playtime_excluded.clone(),
        }
    }

    /// Load by merging the global device prefs with one context's saves. Used
    /// by [`Self::load_default`]; exposed for tests with explicit paths.
    ///
    /// Lo cargado pasa por [`cleanse`] antes de devolverse: el estado en disco
    /// es anterior a la puerta de `hoard_core::ids` y puede llevar veneno (así
    /// entró la correlación fantasma), pero cargar **nunca** puede fallar por
    /// eso — se repara o se marca (ADR 0021, C.3).
    pub fn load_split(device_path: &Path, context_path: &Path) -> Result<Self> {
        let prefs: DevicePrefs = load_json(device_path)?;
        let ctx: ContextSaves = load_json(context_path)?;
        let mut state = Self {
            saves: ctx.saves,
            manual_paths: prefs.manual_paths,
            ignored_slugs: prefs.ignored_slugs,
            playtime_excluded: prefs.playtime_excluded,
            excluded_paths: prefs.excluded_paths,
            quarantined_slugs: HashSet::new(),
            quarantined_save_ids: HashSet::new(),
        };
        cleanse(&mut state);
        Ok(state)
    }

    /// ¿Este slug quedó marcado al cargar? Un slug en cuarentena está bien
    /// formado pero significa cualquier cosa (`users`, el nombre de usuario del
    /// perfil…). Para todo lo demás (rutas, subidas, identidad server-side)
    /// sigue siendo el slug de ese save y se usa tal cual.
    ///
    /// **No es lo que protege la correlación**, aunque el doc lo prometiera:
    /// quien casa procesos vivos con saves es `agent::game_identity_tokens`, y
    /// ése ya descarta el mismo token por su cuenta —con el criterio ampliado
    /// al home local, que ahí sí corresponde—. Cablear además esta consulta
    /// sería duplicar la defensa con un criterio más flojo. Vive como
    /// diagnóstico: qué slugs de este estado no significan nada, para la UI y
    /// para el log.
    pub fn is_slug_quarantined(&self, slug: &str) -> bool {
        self.quarantined_slugs.contains(slug)
    }

    /// Los slugs marcados en la última carga (diagnóstico / UI).
    pub fn quarantined_slugs(&self) -> &HashSet<String> {
        &self.quarantined_slugs
    }

    /// Los ids de save marcados en la última carga: presentes en `saves` pero
    /// sin forma de UUID canónico, así que el server nunca los reconocerá.
    pub fn quarantined_save_ids(&self) -> &HashSet<String> {
        &self.quarantined_save_ids
    }

    /// Write device prefs and the context's saves to their two files.
    pub fn save_split(&self, device_path: &Path, context_path: &Path) -> Result<()> {
        save_json(device_path, &self.device_prefs())?;
        save_json(
            context_path,
            &ContextSaves {
                saves: self.saves.clone(),
            },
        )?;
        Ok(())
    }

    /// Load the state for the currently-active context (see [`ACTIVE_CONTEXT`]),
    /// migrating a pre-split monolithic `state.json` on first run. Returns the
    /// merged state plus the context's saves-file path, which the caller threads
    /// straight back into [`Self::save`].
    pub fn load_default() -> Result<(Self, PathBuf)> {
        let device_path = Self::device_path()?;
        let context_path = Self::context_path_for(&current_context_id())?;
        migrate_legacy_state(&device_path, &context_path)?;
        Ok((Self::load_split(&device_path, &context_path)?, context_path))
    }

    /// Persist the state. `context_path` is the per-context saves file returned
    /// by [`Self::load_default`]; the device prefs always go to the single
    /// global `device.json`.
    pub fn save(&self, context_path: &Path) -> Result<()> {
        self.save_split(&Self::device_path()?, context_path)
    }

    /// Record a manual save-folder override for `slug`. Subsequent calls to
    /// [`crate::detection::detect_all`] return a row whose `found_paths` is
    /// exactly `[path]` and whose source is `ManualOverride`, regardless of
    /// what the heuristics produced.
    pub fn set_manual_path(&mut self, slug: &str, path: PathBuf) {
        // Fijar la ruta a mano ES la desmentida de la heurística: lo que
        // propuso no valía y ésta es la respuesta. Se emite aquí, que es por
        // donde pasan los dos frontends, y no en cada command.
        crate::telemetry::manual_path(slug, &path);
        self.manual_paths.insert(slug.to_string(), path);
    }

    /// Drop the manual override for `slug` (if any). After this the next
    /// detect_all pass returns whatever the heuristics find.
    pub fn clear_manual_path(&mut self, slug: &str) {
        self.manual_paths.remove(slug);
    }

    /// True when `slug` has been blacklisted via
    /// [`Self::add_ignored_slug`]. The Library page filters detected games
    /// against this set so they stop reappearing in the grid until the user
    /// reactivates them from Settings.
    pub fn is_ignored(&self, slug: &str) -> bool {
        self.ignored_slugs.contains(slug)
    }

    /// Persistently blacklist a detected slug. After this call any
    /// `list_detected_games` invocation drops the row before returning it to
    /// the UI. Idempotent: re-adding an existing slug is a no-op.
    pub fn add_ignored_slug(&mut self, slug: String) {
        self.ignored_slugs.insert(slug);
    }

    /// `true` si `path` está en una carpeta descartada por el usuario.
    ///
    /// La comparación es **por frontera de segmento**, así que descartar
    /// `…/Games` tapa `…/Games/X` pero no `…/GamesOther`. En Windows y macOS
    /// ignora mayúsculas, como hacen esos sistemas de ficheros.
    pub fn is_path_excluded(&self, path: &Path) -> bool {
        if self.excluded_paths.is_empty() {
            return false;
        }
        let norm = |p: &Path| -> PathBuf {
            if cfg!(any(windows, target_os = "macos")) {
                PathBuf::from(p.to_string_lossy().to_lowercase())
            } else {
                p.to_path_buf()
            }
        };
        let target = norm(path);
        // `starts_with` compara por COMPONENTES, que es justo la semántica de
        // frontera que queremos (y por eso bajar a minúsculas la cadena entera
        // no altera nada: los separadores siguen donde estaban).
        self.excluded_paths
            .iter()
            .any(|root| target.starts_with(norm(root)))
    }

    /// Descarta una carpeta. Idempotente, y absorbe lo que ya cubriera:
    /// excluir un padre deja sin sentido a sus hijas ya excluidas.
    pub fn add_excluded_path(&mut self, path: PathBuf) {
        if self.is_path_excluded(&path) {
            return;
        }
        self.excluded_paths.retain(|p| !p.starts_with(&path));
        self.excluded_paths.push(path);
    }

    /// Deja de descartar exactamente esta carpeta (no las que la contengan).
    pub fn remove_excluded_path(&mut self, path: &Path) {
        self.excluded_paths.retain(|p| p != path);
    }

    /// Drop the blacklist entry for `slug` so the next detection pass
    /// re-surfaces it. Mirrors `add_ignored_slug`. Idempotent.
    pub fn remove_ignored_slug(&mut self, slug: &str) {
        self.ignored_slugs.remove(slug);
    }

    /// True when `slug` was dropped from playtime-only tracking via
    /// [`Self::exclude_playtime`]. The desktop's playtime-game derivation
    /// filters auto-enroll candidates against this so they stop coming back.
    pub fn is_playtime_excluded(&self, slug: &str) -> bool {
        self.playtime_excluded.contains(slug)
    }

    /// Stop counting `slug` toward the recap. After this the next agent seed
    /// no longer enrols a playtime-only slot for it. Idempotent.
    pub fn exclude_playtime(&mut self, slug: String) {
        self.playtime_excluded.insert(slug);
    }

    /// Re-allow `slug` for playtime-only tracking. Mirrors
    /// [`Self::exclude_playtime`]. Idempotent.
    pub fn include_playtime(&mut self, slug: &str) {
        self.playtime_excluded.remove(slug);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn save_state(slug: &str) -> SaveState {
        SaveState {
            local_path: PathBuf::from(format!("/saves/{slug}")),
            game_slug: slug.to_string(),
            label: "default".to_string(),
            last_backup_at: None,
            last_version_num: Some(34),
            paused: false,
            preset: None,
            allow_device_local: None,
            set_hash: None,
            processes: vec![],
            shared_processes: false,
        }
    }

    /// The core partition invariant: two contexts sharing one `device.json`
    /// keep their `saves` maps fully separate (no cross-account/self-hosted
    /// residue), while device-level prefs are shared. Mirrors production, where
    /// every write goes through a state that was first loaded via `load_split`.
    #[test]
    fn contexts_isolate_saves_but_share_device_prefs() {
        let tmp = tempfile::tempdir().unwrap();
        let device = tmp.path().join("device.json");
        let ctx_a = tmp.path().join("contexts/cloud-A.json");
        let ctx_b = tmp.path().join("contexts/cloud-B.json");

        // Account A: one save + device-level prefs.
        let mut a = CliState::default();
        a.saves.insert("save-a".into(), save_state("factorio"));
        a.set_manual_path("stellaris", PathBuf::from("/data/stellaris"));
        a.add_ignored_slug("dwarf-fortress".into());
        a.save_split(&device, &ctx_a).unwrap();

        // Account B loads the shared device.json (inheriting A's prefs), adds
        // its own save, and persists — exactly the load→mutate→save cycle the
        // app runs.
        let mut b = CliState::load_split(&device, &ctx_b).unwrap();
        assert!(b.saves.is_empty(), "B's context starts with no saves");
        assert_eq!(b.manual_paths.len(), 1, "B shares A's device prefs");
        assert!(b.is_ignored("dwarf-fortress"));
        b.saves.insert("save-b".into(), save_state("ck3"));
        b.save_split(&device, &ctx_b).unwrap();

        // Neither context can see the other's saves.
        let a_loaded = CliState::load_split(&device, &ctx_a).unwrap();
        assert!(a_loaded.saves.contains_key("save-a"));
        assert!(!a_loaded.saves.contains_key("save-b"));
        let b_loaded = CliState::load_split(&device, &ctx_b).unwrap();
        assert!(b_loaded.saves.contains_key("save-b"));
        assert!(!b_loaded.saves.contains_key("save-a"));
        // A's device prefs survived B's write.
        assert_eq!(a_loaded.manual_paths.len(), 1);
        assert!(a_loaded.is_ignored("dwarf-fortress"));
    }

    /// Distinct context ids per cloud account and per self-hosted URL.
    #[test]
    fn context_ids_are_distinct_per_account_and_server() {
        assert_ne!(cloud_context("user-a"), cloud_context("user-b"));
        assert_ne!(
            selfhosted_context("https://a.example"),
            selfhosted_context("https://b.example")
        );
        // A trailing slash is normalised away, so it doesn't fork the context.
        assert_eq!(
            selfhosted_context("https://a.example"),
            selfhosted_context("https://a.example/")
        );
        // Cloud and self-hosted never collide.
        assert!(cloud_context("x").starts_with("cloud-"));
        assert!(selfhosted_context("x").starts_with("selfhosted-"));
    }

    /// `manual_paths` survives a device.json round-trip; a missing context file
    /// yields empty saves rather than an error.
    #[test]
    fn device_prefs_round_trip_and_missing_context_is_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let device = tmp.path().join("device.json");
        let ctx = tmp.path().join("contexts/selfhosted-x.json");

        let mut state = CliState::default();
        state.set_manual_path("stellaris", PathBuf::from("/home/x/Stellaris/save games"));
        state.set_manual_path("ck3", PathBuf::from("/data/ck3"));
        state.save_split(&device, &ctx).unwrap();

        let loaded = CliState::load_split(&device, &ctx).unwrap();
        assert_eq!(loaded.manual_paths.len(), 2);
        assert_eq!(
            loaded.manual_paths.get("stellaris"),
            Some(&PathBuf::from("/home/x/Stellaris/save games")),
        );

        // A brand-new context (no file yet) loads clean, keeping shared prefs.
        let fresh_ctx = tmp.path().join("contexts/cloud-new.json");
        let fresh = CliState::load_split(&device, &fresh_ctx).unwrap();
        assert!(fresh.saves.is_empty());
        assert_eq!(fresh.manual_paths.len(), 2);
    }

    /// A pre-split monolithic `state.json` (saves + prefs in one file)
    /// deserialises into `CliState` and splits cleanly into the two files.
    #[test]
    fn legacy_monolithic_state_splits_cleanly() {
        let tmp = tempfile::tempdir().unwrap();
        let device = tmp.path().join("device.json");
        let ctx = tmp.path().join("contexts/cloud-legacy.json");

        // Old format, including the now-removed `last_cloud_account_id` key,
        // which serde must ignore rather than reject.
        let legacy = r#"{
            "saves": { "s1": {
                "local_path": "/saves/factorio", "game_slug": "factorio",
                "label": "default", "last_version_num": 7
            }},
            "manual_paths": { "ck3": "/data/ck3" },
            "ignored_slugs": ["dwarf-fortress"],
            "last_cloud_account_id": "old-account"
        }"#;
        let old: CliState = serde_json::from_str(legacy).unwrap();
        old.save_split(&device, &ctx).unwrap();

        let loaded = CliState::load_split(&device, &ctx).unwrap();
        assert!(loaded.saves.contains_key("s1"));
        assert_eq!(
            loaded.manual_paths.get("ck3"),
            Some(&PathBuf::from("/data/ck3"))
        );
        assert!(loaded.is_ignored("dwarf-fortress"));
    }

    /// **El test de no-brickeo (ADR 0021, C.3).** Un `state.json` con el veneno
    /// que llegó a producción tiene que **cargar**, no reventar: el motor
    /// arranca, los saves sanos siguen intactos, el slug recuperable sale
    /// reparado y el degenerado sale marcado sin que le cambien la identidad.
    ///
    /// Si algún día alguien pone `#[serde(try_from)]` sobre el estado
    /// persistido, este test cae — que es justo el punto.
    #[test]
    fn poisoned_state_json_loads_and_is_repaired() {
        let tmp = tempfile::tempdir().unwrap();
        let device = tmp.path().join("device.json");
        let ctx = tmp.path().join("contexts/cloud-poisoned.json");
        std::fs::create_dir_all(ctx.parent().unwrap()).unwrap();

        // Estado real de julio 2026: un save sano, uno con el slug sin
        // slugificar ("GSE Saves"), uno con el slug degenerado (el caso
        // `slug == username`, aquí un token de fontanería que no depende del
        // entorno de test) y un id local que no es UUID (el churn de DOOM).
        std::fs::write(
            &ctx,
            r#"{ "saves": {
                "3f2504e0-4f89-41d3-9a0c-0305e82c3301": {
                    "local_path": "/saves/stardew", "game_slug": "stardew-valley",
                    "label": "default", "last_version_num": 7
                },
                "7c9e6679-7425-40de-944b-e07fc1f90ae7": {
                    "local_path": "/saves/gse", "game_slug": "GSE Saves",
                    "label": "default", "last_version_num": 2
                },
                "9d1b2c3e-1111-4222-8333-444455556666": {
                    "local_path": "/saves/venom", "game_slug": "savedgames",
                    "label": "default", "last_version_num": 1
                },
                "local-doom-4": {
                    "local_path": "/saves/doom", "game_slug": "base",
                    "label": "default", "last_version_num": null
                }
            } }"#,
        )
        .unwrap();
        std::fs::write(
            &device,
            r#"{ "manual_paths": { "Stardew Valley!": "/data/sdv" },
                 "ignored_slugs": ["Dwarf Fortress"] }"#,
        )
        .unwrap();

        // 1. Carga. Esto es lo que no puede fallar nunca.
        let state = CliState::load_split(&device, &ctx).expect("el estado envenenado debe cargar");
        assert_eq!(state.saves.len(), 4, "no se pierde ninguna fila");

        // 2. El save sano sigue igual.
        assert_eq!(
            state.saves["3f2504e0-4f89-41d3-9a0c-0305e82c3301"].game_slug,
            "stardew-valley"
        );

        // 3. El slug recuperable se re-deriva con el mismo `slugify` que lo
        //    mintó — sin él ese save ni siquiera podría subirse hoy.
        assert_eq!(
            state.saves["7c9e6679-7425-40de-944b-e07fc1f90ae7"].game_slug,
            "gse-saves"
        );

        // 4. El degenerado NO se renombra (es la identidad que el server ya
        //    conoce), pero queda marcado para que la correlación lo ignore.
        assert_eq!(
            state.saves["9d1b2c3e-1111-4222-8333-444455556666"].game_slug,
            "savedgames"
        );
        assert!(state.is_slug_quarantined("savedgames"));
        assert!(!state.is_slug_quarantined("stardew-valley"));
        assert!(!state.is_slug_quarantined("gse-saves"));

        // 5. El id que no es UUID se marca, pero la fila se queda (guarda la
        //    ruta local del save).
        assert!(state.quarantined_save_ids().contains("local-doom-4"));
        assert_eq!(state.quarantined_save_ids().len(), 1);

        // 6. Las prefs de dispositivo van keyed por slug: mismo tratamiento.
        assert!(state.manual_paths.contains_key("stardew-valley"));
        assert!(state.is_ignored("dwarf-fortress"));

        // 7. Y el estado reparado persiste y vuelve a cargar sin más avisos.
        state.save_split(&device, &ctx).unwrap();
        let again = CliState::load_split(&device, &ctx).unwrap();
        assert_eq!(
            again.saves["7c9e6679-7425-40de-944b-e07fc1f90ae7"].game_slug,
            "gse-saves"
        );
        assert!(again.is_slug_quarantined("savedgames"));
    }

    /// Un estado sano no se toca: `cleanse` es un no-op sobre lo que ya es
    /// canónico (si no, cada carga movería datos buenos).
    #[test]
    fn clean_state_is_untouched_by_the_cleanse() {
        let tmp = tempfile::tempdir().unwrap();
        let device = tmp.path().join("device.json");
        let ctx = tmp.path().join("contexts/cloud-clean.json");

        let mut state = CliState::default();
        state.saves.insert(
            "3f2504e0-4f89-41d3-9a0c-0305e82c3301".into(),
            save_state("factorio"),
        );
        state.set_manual_path("stellaris", PathBuf::from("/data/stellaris"));
        state.add_ignored_slug("dwarf-fortress".into());
        state.save_split(&device, &ctx).unwrap();

        let loaded = CliState::load_split(&device, &ctx).unwrap();
        assert_eq!(
            loaded.saves["3f2504e0-4f89-41d3-9a0c-0305e82c3301"].game_slug,
            "factorio"
        );
        assert!(loaded.manual_paths.contains_key("stellaris"));
        assert!(loaded.is_ignored("dwarf-fortress"));
        assert!(loaded.quarantined_slugs().is_empty());
        assert!(loaded.quarantined_save_ids().is_empty());
    }

    /// `clear_manual_path` removes the entry; subsequent saves no longer
    /// emit the slug.
    #[test]
    fn clear_manual_path_removes_entry() {
        let mut state = CliState::default();
        state.set_manual_path("stardew-valley", PathBuf::from("/x"));
        assert_eq!(state.manual_paths.len(), 1);
        state.clear_manual_path("stardew-valley");
        assert!(state.manual_paths.is_empty());
        // Idempotent: clearing an unknown slug doesn't panic.
        state.clear_manual_path("not-there");
    }

    /// Default `CliState` has no blacklisted slugs — the field is purely
    /// opt-in.
    #[test]
    fn ignored_slugs_default_empty() {
        assert!(CliState::default().ignored_slugs.is_empty());
    }

    /// Round-trip the blacklist API: add a slug, see it via `is_ignored`,
    /// drop it, see it gone. Idempotent on both ends.
    #[test]
    fn add_and_remove_ignored_slug() {
        let mut state = CliState::default();
        assert!(!state.is_ignored("lethal-company"));

        state.add_ignored_slug("lethal-company".to_string());
        assert!(state.is_ignored("lethal-company"));
        assert_eq!(state.ignored_slugs.len(), 1);

        // Idempotent: re-adding doesn't grow the set.
        state.add_ignored_slug("lethal-company".to_string());
        assert_eq!(state.ignored_slugs.len(), 1);

        state.remove_ignored_slug("lethal-company");
        assert!(!state.is_ignored("lethal-company"));
        assert!(state.ignored_slugs.is_empty());

        // Idempotent: removing an unknown slug doesn't panic.
        state.remove_ignored_slug("not-there");
    }

    /// `ignored_slugs` survives a JSON round-trip and pre-1.5.3 state files
    /// (no `ignored_slugs` key) deserialise as an empty set.
    #[test]
    fn serialize_with_empty_ignored_does_not_emit_field_explicitly_or_does_emit_consistently() {
        // Round-trip with a populated set: every slug survives.
        let mut state = CliState::default();
        state.add_ignored_slug("lethal-company".to_string());
        state.add_ignored_slug("terraforming-mars".to_string());
        let json = serde_json::to_string(&state).unwrap();
        let parsed: CliState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.ignored_slugs.len(), 2);
        assert!(parsed.is_ignored("lethal-company"));
        assert!(parsed.is_ignored("terraforming-mars"));

        // Pre-1.5.3 files without the key load with an empty set thanks to
        // `#[serde(default)]`.
        let legacy: CliState = serde_json::from_str("{\"saves\":{}}").unwrap();
        assert!(legacy.ignored_slugs.is_empty());

        // Empty set round-trips back to empty.
        let empty = CliState::default();
        let empty_json = serde_json::to_string(&empty).unwrap();
        let parsed_empty: CliState = serde_json::from_str(&empty_json).unwrap();
        assert!(parsed_empty.ignored_slugs.is_empty());
    }

    #[test]
    fn excluding_a_folder_covers_everything_under_it_but_not_a_sibling() {
        let mut s = CliState::default();
        s.add_excluded_path(PathBuf::from("/home/u/Games"));
        assert!(s.is_path_excluded(Path::new("/home/u/Games")));
        assert!(s.is_path_excluded(Path::new("/home/u/Games/X/saves")));
        // Frontera de segmento: `GamesOther` NO cae dentro de `Games`.
        assert!(!s.is_path_excluded(Path::new("/home/u/GamesOther")));
        assert!(!s.is_path_excluded(Path::new("/home/u")));
    }

    #[test]
    fn excluding_a_parent_absorbs_its_children_and_is_idempotent() {
        let mut s = CliState::default();
        s.add_excluded_path(PathBuf::from("/a/b/c"));
        s.add_excluded_path(PathBuf::from("/a/b"));
        assert_eq!(s.excluded_paths, vec![PathBuf::from("/a/b")]);
        // Re-excluir algo ya cubierto no añade nada.
        s.add_excluded_path(PathBuf::from("/a/b/d"));
        assert_eq!(s.excluded_paths, vec![PathBuf::from("/a/b")]);
    }

    #[test]
    fn unexcluding_removes_only_the_exact_folder() {
        let mut s = CliState::default();
        s.add_excluded_path(PathBuf::from("/a/b"));
        s.remove_excluded_path(Path::new("/a/b/c"));
        assert!(s.is_path_excluded(Path::new("/a/b/c")), "sólo la exacta");
        s.remove_excluded_path(Path::new("/a/b"));
        assert!(!s.is_path_excluded(Path::new("/a/b/c")));
    }

    #[test]
    fn an_empty_exclude_list_costs_nothing_and_excludes_nothing() {
        let s = CliState::default();
        assert!(!s.is_path_excluded(Path::new("/anything/at/all")));
    }

    /// A 0-byte `contexts/<id>.json` is what the old truncate-then-write left
    /// after a crash, and it is the expensive one: `load_json` moves it aside
    /// and hands back `Default`, so the user's tracked-save list is gone.
    #[test]
    fn a_zero_byte_context_file_costs_the_saves_and_is_kept_for_forensics() {
        let tmp = tempfile::tempdir().unwrap();
        let device = tmp.path().join("device.json");
        let ctx = tmp.path().join("contexts").join("cloud-a.json");
        std::fs::create_dir_all(ctx.parent().unwrap()).unwrap();
        std::fs::write(&ctx, b"").unwrap();

        let state =
            CliState::load_split(&device, &ctx).expect("load must not fail on a 0-byte file");

        assert!(state.saves.is_empty());
        let backups: Vec<_> = std::fs::read_dir(ctx.parent().unwrap())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains(".corrupt-"))
            .collect();
        assert_eq!(
            backups.len(),
            1,
            "the corrupt file must be moved aside, not deleted"
        );
        assert!(!ctx.exists(), "the corrupt file must not be left in place");
    }

    /// The other half of a torn write, on the device-level file: some bytes
    /// made it, the closing brace didn't.
    #[test]
    fn a_truncated_device_file_falls_back_to_empty_prefs() {
        let tmp = tempfile::tempdir().unwrap();
        let device = tmp.path().join("device.json");
        let ctx = tmp.path().join("contexts").join("cloud-a.json");
        std::fs::write(&device, br#"{"manual_paths":{"factorio":"/sav"#).unwrap();

        let state =
            CliState::load_split(&device, &ctx).expect("load must not fail on a truncated file");

        assert!(state.manual_paths.is_empty());
        assert!(state.saves.is_empty());
    }

    /// The fix proper: both files are replaced in one step each, so a reload
    /// sees the whole thing and no temp file is left behind in the state dir.
    #[test]
    fn saving_over_corrupt_files_writes_them_whole() {
        let tmp = tempfile::tempdir().unwrap();
        let device = tmp.path().join("device.json");
        let ctx = tmp.path().join("contexts").join("cloud-a.json");
        std::fs::create_dir_all(ctx.parent().unwrap()).unwrap();
        std::fs::write(&device, b"").unwrap();
        std::fs::write(&ctx, b"{\"saves\": {").unwrap();

        let mut state = CliState::default();
        state.saves.insert("s1".to_string(), save_state("factorio"));
        state.set_manual_path("factorio", PathBuf::from("/saves/factorio"));
        state.save_split(&device, &ctx).unwrap();

        let back = CliState::load_split(&device, &ctx).unwrap();
        assert_eq!(back.saves.len(), 1);
        assert_eq!(
            back.manual_paths.get("factorio"),
            Some(&PathBuf::from("/saves/factorio"))
        );

        let leftovers: Vec<_> = std::fs::read_dir(ctx.parent().unwrap())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains(".tmp-"))
            .collect();
        assert!(
            leftovers.is_empty(),
            "temp files left behind: {leftovers:?}"
        );
    }
}
