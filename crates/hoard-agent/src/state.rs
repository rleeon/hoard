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
/// The `saves` map (save_id to version cursor plus local path) only means
/// anything for the account or server that owns those saves: a `save_id` and its
/// `last_version_num` are minted server-side, so replaying one account's cursors
/// against another, or against a self-hosted server, makes the next upload claim
/// a `base_version` the target never had, which is a `non_fast_forward`, and
/// surfaces "saves from another account or self-hosted" residue. So each context
/// keeps its `saves` in its own file (`contexts/<id>.json`); device-level prefs
/// (`manual_paths`, `ignored_slugs`, `playtime_excluded`) stay global in
/// `device.json`.
///
/// The desktop sets this at boot and on every login, logout and account switch
/// (mirroring `current_client`'s self-hosted-wins-else-cloud selection). When it
/// is unset, on the headless CLI, which is self-hosted only, the id is derived
/// from the configured server URL.
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
    /// These process names are shared by SEVERAL tracked saves, so seeing the
    /// process does not say which of them is being played.
    ///
    /// It is the case of an emulated console split into one folder per game: ten
    /// titles from the same machine list the same executable, and counting them
    /// all as "playing" the moment the emulator starts would invent hours for
    /// nine of them and veto the sync of nine saves nobody has touched. When this
    /// is set, the process name stops being enough on its own and there also has
    /// to be activity in this particular folder (see `sample_running` in
    /// `agent.rs`). `default` keeps older `state.json` files loadable.
    #[serde(default)]
    pub shared_processes: bool,
    /// Does a restore write this game's config?
    ///
    /// The files [`hoard_core::kernel::fileclass`] classifies as `DeviceLocal`,
    /// `graphics.ini`, `settings.cfg`, whatever carries THIS monitor's
    /// resolution, are always uploaded but by default never written back:
    /// restoring them from one PC onto another is the short road to a game that
    /// boots to a black screen. The switch in the restore dialog skips that once;
    /// this settles it for the game.
    ///
    /// It is per game because the answer is. In one game the config and the save
    /// live in the same file and it has to be written; in another it is the
    /// resolution and it must not be touched. A single global switch would have to
    /// be right for both at once, which it cannot be.
    ///
    /// `None` is undecided: not written, and the dialog keeps asking.
    /// `Some(true)` writes it on automatic restores too, which is what makes the
    /// setting worth having; `Some(false)` is an explicit no.
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
    /// Folders the user has discarded: detection never offers them again, nor
    /// anything below them.
    ///
    /// It complements [`Self::ignored_slugs`], which is not enough for this: a
    /// phase-4 find's name comes from attribution, and attribution changes
    /// between scans (one game's folder came out as ChatGPT, then opencode, then
    /// code). With a different slug every time, ignoring by slug holds nothing
    /// down and the same folder reappears under a new name. By path it does.
    ///
    /// `default` keeps older `state.json` files loading.
    #[serde(default)]
    pub excluded_paths: Vec<PathBuf>,
    /// Slugs [`cleanse`] flagged on load: well formed but degenerate (plumbing
    /// tokens, or the system account name). Not persisted (`skip`): it is
    /// recomputed on every load from what is on disk, so the state file's shape
    /// does not change and Slice 5, which owns the durable cleanup, inherits no
    /// field to migrate.
    ///
    /// Derived: [`cleanse`] recomputes it on every load, and writing it by hand
    /// has no lasting effect. Queried with [`Self::is_slug_quarantined`].
    #[serde(skip)]
    pub quarantined_slugs: HashSet<String>,
    /// Save ids in `saves` that are not canonical UUIDs. Same treatment: they get
    /// flagged and left where they are (deleting them would leave the save
    /// untracked with no way to recover its local path). Derived, like
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
    /// Folders the user discarded. A DEVICE preference rather than an account
    /// one: a folder that is junk here can be legitimate on another machine.
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

/// A slug we have already complained about in this process.
///
/// `cleanse` runs on every load of `state.json`, some twenty-five times an hour,
/// and the state is not rewritten, so the same poisoned slug is rediscovered whole
/// each time. Two users with a save called `user` generated 3,669 identical
/// warnings in three days: that is not 3,669 incidents, it is one seen 3,669
/// times, and it buried the rest of the log.
///
/// The first time is reported in full; the rest are `debug`. The set is per
/// process, so restarting the daemon warns once again, which is exactly what you
/// want from a problem that is still there.
fn warn_once(slug: &str, emit: impl FnOnce()) {
    use std::sync::{Mutex, OnceLock};
    static SEEN: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let seen = SEEN.get_or_init(|| Mutex::new(HashSet::new()));
    let first = match seen.lock() {
        Ok(mut g) => g.insert(slug.to_string()),
        // A poisoned lock must not silence a warning: it gets reported.
        Err(_) => true,
    };
    if first {
        emit();
    } else {
        tracing::debug!(slug, "state: slug still quarantined (already reported)");
    }
}

/// Puts freshly read state through `hoard_core::ids`' lenient gate (ADR 0021,
/// C.3).
///
/// The poison is already on disk (one save ended up with its slug equal to the
/// Windows account name, which turned every app in the profile into a "you are
/// playing" signal) so a strict `try_from` here would leave the engine unable to
/// start. Three outcomes per value, none of them an error:
///
/// - Valid: untouched.
/// - Recoverable (uppercase, spaces, junk): re-derived with the same `slugify`
///   that minted it, warned about, and the repaired one used. A slug like that
///   could not even be uploaded today, since the wire's gate rejects it, so
///   repairing it is the only thing that gets that save back into sync.
/// - Degenerate (a plumbing token, an account name): left alone. It is already
///   well formed and it is the `(user, game_slug, label)` identity the server
///   knows; renaming it would create a new save in the cloud. It gets flagged in
///   [`CliState::is_slug_quarantined`] so correlation ignores it.
///
/// The durable cleanup, rewriting the migrated state, belongs to Slice 5; this is
/// the in-memory repair that lets the engine start meanwhile.
///
/// The verdict is `GameSlug::repair` and nothing else. It used to be re-checked
/// against `agent::is_generic_identity_token`, which widens the static list with
/// THIS machine's home components. That is right for matching live processes
/// (everything hangs under `C:\Users\<user>\`) and wrong here: a save's
/// identity is `(user, game_slug, label)` on the server and has to mean the same
/// thing on every machine. With the local criterion, a save named after the user
/// was quarantined on their laptop and clean on the one next to it, for the same
/// synced `state.json`.
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
                // The reason goes in the field rather than the text: `Degenerate`
                // and `Unrecoverable` are different things, and the message
                // called the degenerate ones, which are the vast majority,
                // "unrecoverable".
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
    // Device prefs are keyed by slug; the same poison applies.
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

    // A save id that is not a UUID will never exist server-side (the DOOM churn).
    // It gets flagged, not deleted: the row holds the save's local path.
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

    /// Load by merging the global device prefs with one context's saves. Used by
    /// [`Self::load_default`]; exposed for tests with explicit paths.
    ///
    /// What is loaded goes through [`cleanse`] before being returned: the on-disk
    /// state predates `hoard_core::ids`' gate and can carry poison (that is how
    /// the phantom correlation got in), but loading can never fail because of it.
    /// It is repaired or flagged (ADR 0021, C.3).
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

    /// Was this slug flagged on load? A quarantined slug is well formed but means
    /// anything at all (`users`, the profile's account name). For everything else
    /// (paths, uploads, server-side identity) it is still that save's slug and is
    /// used as-is.
    ///
    /// It is not what protects correlation, whatever the doc used to promise:
    /// what matches live processes to saves is `agent::game_identity_tokens`, and
    /// that already discards the same token on its own, with the criterion
    /// widened to the local home, which is right there. Wiring this query in as
    /// well would duplicate the defence with a weaker criterion. It lives on as
    /// diagnostics: which slugs in this state mean nothing, for the UI and for
    /// the log.
    pub fn is_slug_quarantined(&self, slug: &str) -> bool {
        self.quarantined_slugs.contains(slug)
    }

    /// Los slugs marcados en la última carga (diagnóstico / UI).
    pub fn quarantined_slugs(&self) -> &HashSet<String> {
        &self.quarantined_slugs
    }

    /// The save ids flagged on the last load: present in `saves` but with no
    /// canonical UUID shape, so the server will never recognise them.
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
        // Pinning the path by hand IS the contradiction of the heuristic: what it
        // proposed was no good and this is the answer. It is emitted here, where
        // both frontends pass through, rather than in each command.
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

    /// `true` when `path` is inside a folder the user discarded.
    ///
    /// The comparison is by segment boundary, so discarding `.../Games` covers
    /// `.../Games/X` but not `.../GamesOther`. On Windows and macOS it ignores
    /// case, as those filesystems do.
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
        // `starts_with` compares by COMPONENT, which is exactly the boundary
        // semantics we want (and why lowercasing the whole string changes
        // nothing: the separators are still where they were).
        self.excluded_paths
            .iter()
            .any(|root| target.starts_with(norm(root)))
    }

    /// Discards a folder. Idempotent, and it absorbs whatever it already covered:
    /// excluding a parent makes its already-excluded children pointless.
    pub fn add_excluded_path(&mut self, path: PathBuf) {
        if self.is_path_excluded(&path) {
            return;
        }
        self.excluded_paths.retain(|p| !p.starts_with(&path));
        self.excluded_paths.push(path);
    }

    /// Stops discarding exactly this folder, not the ones containing it.
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
        // its own save, and persists: exactly the load, mutate, save cycle the
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

    /// The no-bricking test (ADR 0021, C.3). A `state.json` carrying the poison
    /// that reached production has to load, not blow up: the engine starts, the
    /// healthy saves stay untouched, the recoverable slug comes out repaired and
    /// the degenerate one comes out flagged with its identity unchanged.
    ///
    /// If somebody ever puts a `#[serde(try_from)]` over the persisted state,
    /// this test fails, which is exactly the point.
    #[test]
    fn poisoned_state_json_loads_and_is_repaired() {
        let tmp = tempfile::tempdir().unwrap();
        let device = tmp.path().join("device.json");
        let ctx = tmp.path().join("contexts/cloud-poisoned.json");
        std::fs::create_dir_all(ctx.parent().unwrap()).unwrap();

        // Real state from July 2026: one healthy save, one with an unslugified
        // slug ("GSE Saves"), one with a degenerate slug (the `slug == username`
        // case, here a plumbing token that does not depend on the test
        // environment) and one local id that is not a UUID (the DOOM churn).
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

        // 1. Load. This is what can never fail.
        let state = CliState::load_split(&device, &ctx).expect("el estado envenenado debe cargar");
        assert_eq!(state.saves.len(), 4, "no se pierde ninguna fila");

        // 2. El save sano sigue igual.
        assert_eq!(
            state.saves["3f2504e0-4f89-41d3-9a0c-0305e82c3301"].game_slug,
            "stardew-valley"
        );

        // 3. The recoverable slug is re-derived with the same `slugify` that
        //    minted it; without that, the save could not even upload today.
        assert_eq!(
            state.saves["7c9e6679-7425-40de-944b-e07fc1f90ae7"].game_slug,
            "gse-saves"
        );

        // 4. The degenerate one is NOT renamed (it is the identity the server
        //    already knows), but it gets flagged so correlation ignores it.
        assert_eq!(
            state.saves["9d1b2c3e-1111-4222-8333-444455556666"].game_slug,
            "savedgames"
        );
        assert!(state.is_slug_quarantined("savedgames"));
        assert!(!state.is_slug_quarantined("stardew-valley"));
        assert!(!state.is_slug_quarantined("gse-saves"));

        // 5. The non-UUID id is flagged, but the row stays (it holds the save's
        //    local path).
        assert!(state.quarantined_save_ids().contains("local-doom-4"));
        assert_eq!(state.quarantined_save_ids().len(), 1);

        // 6. Device prefs are keyed by slug: same treatment.
        assert!(state.manual_paths.contains_key("stardew-valley"));
        assert!(state.is_ignored("dwarf-fortress"));

        // 7. And the repaired state persists and reloads with no further warnings.
        state.save_split(&device, &ctx).unwrap();
        let again = CliState::load_split(&device, &ctx).unwrap();
        assert_eq!(
            again.saves["7c9e6679-7425-40de-944b-e07fc1f90ae7"].game_slug,
            "gse-saves"
        );
        assert!(again.is_slug_quarantined("savedgames"));
    }

    /// Healthy state is untouched: `cleanse` is a no-op over what is already
    /// canonical (otherwise every load would move good data).
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

    /// Default `CliState` has no blacklisted slugs; the field is purely
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
