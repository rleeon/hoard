//! Detection: correlating a process with a write (phase 3, ADR 0020). The
//! strongest signal there is.
//!
//! The most reliable sign that a folder is a save: it was rewritten while a GAME
//! process, not a system one, was alive. It depends on neither the name nor the
//! extension, so it catches saves with GUID names or in languages the name signal
//! would never touch (the manifest benchmark measures about 6% recall from the
//! name alone; see `scoring::bench`).
//!
//! The mechanism here is a persisted store of observations. When the
//! `SaveWatcher` emits a write on a directory, the agent calls
//! [`CorrelationStore::record`] with a snapshot of the live processes
//! ([`sample_game_processes`]); the store attributes the write to the most likely
//! game process and saves it. Scoring asks [`CorrelationStore::signal_for`] and
//! adds [`CORRELATION_BONUS`] (+0.50).
//!
//! Integration note: the observer loop (wiring `SaveWatcher` over `roots.rs`'
//! roots plus a `sysinfo` sample on each event) lives in the agent's scheduler and
//! gets wired up in a later step. This module provides the store, the sampling and
//! the signal, all testable in isolation. Cold, meaning a game never observed, the
//! signal is 0. That is the ADR's known limit: correlation needs at least one
//! observed play session.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sysinfo::System;

/// ADR 0020 §2's score bonus for process-to-write correlation. It is the dominant
/// signal: on its own, plus any weak evidence, it already clears the
/// auto-confirmed cutoff (0.60).
pub const CORRELATION_BONUS: f32 = 0.50;

/// Processes that are NEVER the game: system, shells, browsers, Hoard itself, and
/// the launchers and overlays that run alongside a game without being it. Matched
/// by case-insensitive substring against the process name.
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
    // System daemons seen slipping through as "game" (they were attributing
    // writes to dockerd, avahi and the like). Matched by name substring.
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
    // The real Ubisoft client that Ubisoft-on-Steam games launch inside the
    // prefix: neither "ubisoftconnect" nor "uplay" is a substring of
    // `UbisoftGameLauncher.exe` or `UbisoftGameLauncher64.exe` (its `upc.exe`
    // sibling goes in the EXACT list). It rewrites
    // `.../Ubisoft Game Launcher/savegames/...` with its own cloud sync and
    // outlives the game, so the correlation never ends. See the PoP 2008
    // incident, jul-2026.
    "ubisoftgamelauncher",
    // Wine, Proton and Steam-runtime plumbing on Linux: it wraps the game without
    // being it. Without this the `.first()` attribution keeps a wrapper
    // (`reaper`, `proton`, `wineserver`) instead of the real executable. Matched
    // by substring, so `wineserver`, `wine64-preloader` and `winedevice.exe` all
    // fall under "wine".
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
    // Overlays and background utilities seen correlating wrongly with save folders
    // (RivaTuner, AMD, Windows tools). They touch and watch folders (Steam Cloud
    // rewrites the save while they run) without being the game. Matched by name
    // substring.
    "ctfmon",
    "taskhost",
    "rtss",
    "encoderserver",
    "rivatuner",
    "radeonsoftware",
    "windhawk",
    // GPU vendor overlays and helpers on Windows: they live in Program Files, so
    // the system-path rule does not reach them and, unlike the System32 ones, they
    // have to be named. They idle at about 0% CPU, so `sample_game_processes`'
    // CPU ordering already demotes them; this is only a safety net for the rare
    // case of a paused game. "NVIDIA Overlay.exe", "nvcontainer.exe".
    "nvidia",
    "nvcontainer",
    // Chat and dev apps that live alongside the game and correlated wrongly with
    // save folders (a chat client alive while Steam Cloud rewrote one game's save,
    // attributed to the chat client). They live in AppData/Local on Windows or
    // /usr on Linux, so the path does not always reach them and they have to be
    // named. Matched by substring.
    "discord",
    "slack",
    "claude",
    "anthropic",
    "node",
    "opencode",
    // OneDrive (sync daemon), Xbox / GamingServices (Windows gaming
    // infrastructure, not the game itself).
    "onedrive",
    "xbox",
    "gamingservices",
    "gamebar",
    // Desktop tools seen firing "heavy untracked game-like process" in production
    // (jul-2026 log): launchers and managers, scripting, office software, editors
    // and utilities. They live outside system paths, so they have to be named.
    "playnite",
    "autohotkey",
    "mspaint",
    "topaz",
    "winrar",
    "quicklook",
    "microsoft.media.player",
    "sdxhelper",
    "devenv",
    "winword",
    "crossdeviceservice",
    "logipluginservice",
    "generate_emu_config",
    // AI and desktop apps with their own installer in the user profile
    // (`%LOCALAPPDATA%\Programs\...`, `/opt/...`): neither the system-path rule
    // nor "electron" (the process name is the app's, not the runtime's) reached
    // them. Reported jul-2026: one of them inherited another game's folder and
    // auto-track tracked it under its name.
    "chatgpt",
    "copilot",
    "windsurf",
    "ollama",
    "lmstudio",
    "notion",
    "obsidian",
    // Capture and streaming: they run alongside the game and burn CPU, so
    // `sample_game_processes`' CPU ordering does not demote them and they can win
    // the attribution off the real game.
    "obs64",
    "obs32",
    "obs-studio",
    "streamlabs",
    // File sync clients: not games and, worse, they REWRITE the folder we watch,
    // which is the very signal correlation is attributed from. OneDrive was
    // already above; these are the others.
    "dropbox",
    "nextcloud",
    "megasync",
    "syncthing",
    "pcloud",
    "googledrive",
    // El propio Hoard.
    "hoard",
];

/// Processes that are never a game but whose name is too short for a substring
/// match without false positives (`"code"` collided with `codename.exe` and
/// `decode.exe`). Applied as an EXACT, case-insensitive match on the process and
/// executable basename.
///
/// `"upc"` is the Ubisoft client (sibling of `UbisoftGameLauncher.exe`, which does
/// go by substring); as a substring it would catch legitimate exes like
/// `upcoming.exe`. `"setup"` is installers (as a substring it would catch games
/// with "setup" in the folder); `"achievements"` is the GSE/Goldberg achievement
/// watcher, which runs alongside the emulated game without being it. `"cursor"`
/// and `"zed"` are editors: as substrings they would eat `Precursor.exe` or any
/// game with "zed" inside it (`Fuzed.exe`).
const NON_GAME_PROCESS_EXACT: &[&str] = &[
    "code",
    "code.exe",
    "upc",
    "upc.exe",
    "setup",
    "setup.exe",
    "achievements",
    "achievements.exe",
    "cursor",
    "cursor.exe",
    "zed",
    "zed.exe",
];

/// A process born inside this window after the SYSTEM booted is autostart
/// infrastructure (chat clients, trays, GPU overlays, `node` daemons), not a game
/// the user launched. It is vetoed as a correlation SOURCE rather than as a game
/// outright: `is_game_like` is untouched, so a game auto-launched at boot is still
/// detected by name or handle; it just is not used to ATTRIBUTE a save folder,
/// which is where the poison came from (a chat client alive since boot inheriting
/// a game's save).
const BOOT_AUTOSTART_GRACE_SECS: u64 = 120;

/// How many consecutive ticks a process other than the attributed one has to win
/// before it steals an already-settled correlation. Without this, `record` was
/// last-writer-wins: one unlucky tick (a chat client primary while Steam Cloud
/// rewrote the save) clobbered the game's real exe.
const ATTRIBUTION_SWITCH_STREAK: u32 = 3;

/// Minimum CPU for a process to count as the SOURCE of a correlation. Whatever
/// wrote the folder was executing; a resident at about 0% cannot have been, and
/// that is exactly how the store got poisoned: something else rewrote the folder
/// (Steam Cloud, GSE) with the game not alive, and the attribution fell on the
/// first "game-like" process on the system even after hours asleep (the MOUSE
/// case: an hourly task inherited the folder and fired GameStarted every hour for
/// days). The threshold is low on purpose: the sample arrives just after the
/// write, when the real writer always shows some CPU.
const CORRELATION_SOURCE_MIN_CPU_PCT: f32 = 0.5;

/// Consecutive phantom sessions (started and stopped on a weak signal without a
/// single write to the folder) that bring the observation down. A real game writes
/// its save while being played, and every write re-records the observation and
/// resets the strikes ([`CorrelationStore::record`]), so only a poisoned
/// attribution accumulates. See [`CorrelationStore::strike_phantom`].
pub const PHANTOM_SESSION_STRIKES: u32 = 2;

/// Un proceso vivo candidato a juego.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GameProcess {
    pub name: String,
    pub exe: Option<PathBuf>,
}

/// SYSTEM path prefixes: an executable under them is not a game (daemons,
/// libraries, packaged-app runtimes). Games live in the user's home (Steam, Wine
/// `~/.wine*`, Lutris, Heroic, Flatpak data) or in `/opt/<game>`, never in `/usr`
/// or `/lib`. This filters the bulk of the observed junk (Brave threads in
/// `/opt/brave.com`, Electron in `/usr/lib/...`, `gsd-*` in `/usr/libexec`).
/// It covers Linux, SteamOS and macOS (`/System/` is system daemons; mac games
/// live in `/Applications/*.app`, not here).
const SYSTEM_EXE_PREFIXES: &[&str] = &[
    "/usr/", "/lib", "/lib64", "/bin", "/sbin", "/run/", "/System/",
];

/// The same for Windows: an exe under the Windows directory (`C:\Windows\`,
/// System32, SysWOW64, WinSxS) is the system's, never a game. Matched by the
/// substring `:\windows\` to cover any drive letter. This filters
/// `RuntimeBroker.exe`, `ssh.exe`, `conhost.exe` and the rest, which on Windows
/// poisoned correlation because `SYSTEM_EXE_PREFIXES` held only Linux paths.
fn is_windows_system_exe(exe_str: &str) -> bool {
    exe_str.contains(":\\windows\\")
}

/// `true` when the process looks like a game rather than system, launcher or
/// browser. It uses the name and, when available, the executable's path: a generic
/// thread name (`ThreadPoolForeg`) does not give the browser away, but its exe
/// does.
pub fn is_game_like(name: &str, exe: Option<&Path>) -> bool {
    let lower = name.trim().to_lowercase();
    if lower.is_empty() {
        return false;
    }
    if NON_GAME_PROCESS.iter().any(|bad| lower.contains(bad)) {
        return false;
    }
    if NON_GAME_PROCESS_EXACT.iter().any(|bad| lower == *bad) {
        return false;
    }
    if is_installer_like(&lower) {
        return false;
    }
    if let Some(exe) = exe {
        let exe_str = exe.to_string_lossy().to_lowercase();
        if SYSTEM_EXE_PREFIXES.iter().any(|p| exe_str.starts_with(p)) {
            return false;
        }
        if is_windows_system_exe(&exe_str) {
            return false;
        }
        // The process name can be a generic thread's, so also check the
        // executable's basename against the blacklist.
        if let Some(base) = exe.file_name().and_then(|s| s.to_str()) {
            let base = base.to_lowercase();
            if NON_GAME_PROCESS.iter().any(|bad| base.contains(bad)) {
                return false;
            }
            if NON_GAME_PROCESS_EXACT.iter().any(|bad| base == *bad) {
                return false;
            }
            if is_installer_like(&base) {
                return false;
            }
        }
    }
    true
}

/// Multi-word installers and uninstallers that [`NON_GAME_PROCESS_EXACT`]'s EXACT
/// veto (only `setup` and `setup.exe`) does not catch: an exe like
/// `Codex Windows Sandbox Setup.exe` or `Elden Ring Installer.exe` passed
/// `is_game_like`, poisoned the correlation of the folder it rewrote, and in
/// `attribute_game_name` christened the save with its name. A "setup" substring
/// cannot be used, since it would catch games with the word inside; here we demand
/// it be the LAST token of the basename (or the name itself start with `unins`),
/// which tells an installer from a game that happens to mention the word.
pub(crate) fn is_installer_like(name: &str) -> bool {
    let stem = name.strip_suffix(".exe").unwrap_or(name).trim();
    if stem.starts_with("unins") {
        return true;
    }
    let last = stem.rsplit([' ', '_', '-']).next().unwrap_or(stem);
    matches!(last, "setup" | "installer" | "install" | "uninstall")
}

/// A snapshot of the live processes that look like games. `sys` must arrive
/// already refreshed by the caller: the agent maintains and refreshes a `System`
/// on its activity tick, so the idea is to reuse it rather than create another.
///
/// Two hard filters beyond [`is_game_like`], learned from real junk observed in
/// production (kernel threads `cpuhp/0`, `nv_open_q`, `ib_srv_wkr-2`, and
/// `tokio-runtime-w` threads, all attributed as "game"):
/// - THREADS are discarded, kernel and userland alike: a game is a PROCESS, not
///   somebody else's thread, and `thread_kind().is_some()` gives them away;
/// - an EXECUTABLE on disk is required: a game always has a binary, and kernel
///   threads do not (`exe()` is `None`). This is load-bearing, because correlation
///   feeds playtime, and attributing a save to a kernel worker, which lives 24/7,
///   would accumulate hours forever.
pub fn sample_game_processes(sys: &System) -> Vec<GameProcess> {
    let boot = System::boot_time();
    let mut scored: Vec<(f32, GameProcess)> = Vec::new();
    for proc in sys.processes().values() {
        if proc.thread_kind().is_some() {
            continue;
        }
        let Some(exe) = proc.exe().map(|p| p.to_path_buf()) else {
            continue;
        };
        // The autostart veto: a resident born alongside the system is not the
        // source of a save correlation. `start_time` and `boot_time` are in epoch
        // seconds, so the subtraction is its age after boot.
        if proc.start_time().saturating_sub(boot) < BOOT_AUTOSTART_GRACE_SECS {
            continue;
        }
        // The sleeping-resident veto: whatever wrote the folder was executing, so
        // it shows CPU over the sampling interval. A process at about 0% wrote
        // nothing, and attributing the save to it is the poison.
        if proc.cpu_usage() < CORRELATION_SOURCE_MIN_CPU_PCT {
            continue;
        }
        let name = proc.name().to_string_lossy().to_string();
        if is_game_like(&name, Some(&exe)) {
            scored.push((
                proc.cpu_usage(),
                GameProcess {
                    name,
                    exe: Some(exe),
                },
            ));
        }
    }
    // Ordered by descending CPU: the real game burns CPU while the background
    // helpers (overlays, brokers, Steam Cloud) sit near zero. `record()` attributes
    // by `.first()`, so this ordering points the correlation at the game rather
    // than at whichever process the map happened to yield. Ties keep a stable
    // order.
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.into_iter().map(|(_, p)| p).collect()
}

/// A write observed on a directory, attributed to a game process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteObservation {
    pub dir: PathBuf,
    pub process_name: String,
    pub exe: Option<PathBuf>,
    pub observed_at_ms: u64,
    /// How many times it has been re-observed; more hits mean more confidence.
    pub hits: u32,
    /// A candidate process other than the attributed one, and its run of
    /// consecutive ticks as primary. Stops a single tick stealing the attribution;
    /// it only wins after `ATTRIBUTION_SWITCH_STREAK`. `default` for reading older
    /// stores.
    #[serde(default)]
    challenger: Option<GameProcess>,
    #[serde(default)]
    challenger_streak: u32,
    /// Accumulated phantom sessions: the attributed process "started" and
    /// "stopped" without the folder receiving a single write. At
    /// [`PHANTOM_SESSION_STRIKES`] the observation is dropped, since the
    /// attribution is poisoned. Any real write ([`CorrelationStore::record`])
    /// resets the counter. `default` for reading older stores.
    #[serde(default)]
    phantom_strikes: u32,
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

    /// Loads the store; a missing or corrupt file produces an empty one (the
    /// observations can be collected again, they are not critical).
    pub fn load(path: &Path) -> Self {
        let mut store: Self = std::fs::read_to_string(path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        store.prune_invalid();
        store
    }

    /// Drops observations that no longer pass `is_game_like`'s CURRENT rules, or
    /// that have no exe on disk. It cleans a store poisoned by earlier versions
    /// with looser filters: background utilities (`ctfmon`, `RTSS`, `taskhostw`,
    /// `RadeonSoftware`) and kernel threads with no exe (`System`) that slipped
    /// through as "game" and fired "it started". Self-healing: as soon as a real
    /// session rewrites the folder, the right correlation is learned again.
    fn prune_invalid(&mut self) {
        self.observations
            .retain(|_, o| o.exe.is_some() && is_game_like(&o.process_name, o.exe.as_deref()));
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

    /// Records a write on `dir` that happened while `processes` were alive. With
    /// no game process there is no correlation and nothing is stored. With several
    /// games alive it is attributed to the first (fine-grained attribution is
    /// best-effort; for the "this is a save" signal it is enough that ANY game was
    /// alive).
    ///
    /// The attribution is NOT last-writer-wins: once settled, a different process
    /// only replaces it after being primary for `ATTRIBUTION_SWITCH_STREAK`
    /// consecutive ticks. That way an isolated hit from a resident (a chat client
    /// while something rewrites the save) does not clobber the game's real exe.
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
                challenger: None,
                challenger_streak: 0,
                phantom_strikes: 0,
            });
        entry.observed_at_ms = now_ms();
        entry.hits = entry.hits.saturating_add(1);
        // A real write absolves: the folder is alive and the attribution is being
        // re-earned right now.
        entry.phantom_strikes = 0;

        if primary.name == entry.process_name {
            // Confirma la atribución vigente; refresca el exe y borra retador.
            entry.exe = primary.exe.clone();
            entry.challenger = None;
            entry.challenger_streak = 0;
            return;
        }
        // Proceso distinto: acumula racha en vez de machacar.
        if entry
            .challenger
            .as_ref()
            .is_some_and(|c| c.name == primary.name)
        {
            entry.challenger_streak = entry.challenger_streak.saturating_add(1);
        } else {
            entry.challenger = Some(primary.clone());
            entry.challenger_streak = 1;
        }
        if entry.challenger_streak >= ATTRIBUTION_SWITCH_STREAK {
            entry.process_name = primary.name.clone();
            entry.exe = primary.exe.clone();
            entry.challenger = None;
            entry.challenger_streak = 0;
        }
    }

    /// Returns the observation that corroborates `dir`: an exact match, or any
    /// observed dir that is `dir` or an ancestor of it (the watcher is recursive,
    /// so the write can be recorded on a parent).
    pub fn signal_for(&self, dir: &Path) -> Option<&WriteObservation> {
        self.observed_key(dir)
            .and_then(|k| self.observations.get(&k))
    }

    /// The store's real key that corroborates `dir` (the same ancestor resolution
    /// as [`signal_for`], but returning the key so it can be mutated).
    fn observed_key(&self, dir: &Path) -> Option<PathBuf> {
        let mut cur = Some(dir);
        while let Some(d) = cur {
            if self.observations.contains_key(d) {
                return Some(d.to_path_buf());
            }
            cur = d.parent();
        }
        None
    }

    /// Records a phantom session against the observation corroborating `dir`: its
    /// attributed process was born and died without a single write to the folder.
    /// A real game writes while being played, so only a poisoned attribution (an
    /// hourly task, a resident) accumulates strikes; at
    /// [`PHANTOM_SESSION_STRIKES`] the observation is dropped and the weak signal
    /// dies with it, to be re-learned on its own in the next real session. Returns
    /// `Some(true)` when the observation fell, `Some(false)` when it took a strike
    /// and survived, `None` when `dir` had no observation.
    pub fn strike_phantom(&mut self, dir: &Path) -> Option<bool> {
        let key = self.observed_key(dir)?;
        let obs = self.observations.get_mut(&key)?;
        obs.phantom_strikes = obs.phantom_strikes.saturating_add(1);
        if obs.phantom_strikes >= PHANTOM_SESSION_STRIKES {
            self.observations.remove(&key);
            return Some(true);
        }
        Some(false)
    }

    /// Clears the strikes on the observation corroborating `dir`: the session that
    /// just closed DID write the folder, so the attribution is legitimate.
    pub fn absolve(&mut self, dir: &Path) {
        if let Some(key) = self.observed_key(dir) {
            if let Some(obs) = self.observations.get_mut(&key) {
                obs.phantom_strikes = 0;
            }
        }
    }

    /// The process name attributed to `dir`, without the `.exe` extension. Useful
    /// for phase 4's attribution (naming the save in the library).
    pub fn attributed_name(&self, dir: &Path) -> Option<String> {
        self.signal_for(dir).map(|o| {
            o.process_name
                .strip_suffix(".exe")
                .unwrap_or(&o.process_name)
                .to_string()
        })
    }

    /// Iterates the raw observations (dir to observation). The detection
    /// diagnostics trace uses it to show which watched dirs the store attributes
    /// to a slug, which is phase 4's signal.
    pub fn iter(&self) -> impl Iterator<Item = (&PathBuf, &WriteObservation)> {
        self.observations.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.observations.is_empty()
    }

    pub fn len(&self) -> usize {
        self.observations.len()
    }
}

/// Static scoring ([`crate::scoring::score_dir`]) plus the correlation bonus when
/// the store corroborates the dir. It keeps `scoring.rs` free of runtime
/// dependencies; the bonus is added here.
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
    fn is_game_like_rejects_background_overlays_and_tools() {
        // Background utilities seen correlating wrongly with save folders (the
        // real poisoned-store case).
        for n in [
            "ctfmon.exe",
            "taskhostw.exe",
            "RTSS.exe",
            "EncoderServer.exe",
            "RadeonSoftware.exe",
            "windhawk.exe",
        ] {
            assert!(!is_game_like(n, None), "{n} debería filtrarse");
        }
    }

    #[test]
    fn load_prunes_poisoned_observations() {
        // A store with the bug's real entries: background utils and no exe.
        let json = r#"{"observations":{
            "/saves/ark":{"dir":"/saves/ark","process_name":"RTSS.exe","exe":"C:/RivaTuner/RTSS.exe","observed_at_ms":1,"hits":9},
            "/saves/repo":{"dir":"/saves/repo","process_name":"ctfmon.exe","exe":"C:/Windows/System32/ctfmon.exe","observed_at_ms":1,"hits":9},
            "/saves/ksp":{"dir":"/saves/ksp","process_name":"windhawk.exe","exe":null,"observed_at_ms":1,"hits":9},
            "/saves/eu5":{"dir":"/saves/eu5","process_name":"System","exe":null,"observed_at_ms":1,"hits":9},
            "/saves/real":{"dir":"/saves/real","process_name":"eldenring.exe","exe":"D:/Games/eldenring.exe","observed_at_ms":1,"hits":9}
        }}"#;
        let tmp = std::env::temp_dir().join(format!("hoard-corr-prune-{}.json", now_ms()));
        std::fs::write(&tmp, json).unwrap();
        let store = CorrelationStore::load(&tmp);
        let _ = std::fs::remove_file(&tmp);
        // Sólo sobrevive el juego de verdad.
        assert_eq!(store.len(), 1);
        assert!(store.signal_for(Path::new("/saves/real")).is_some());
        assert!(store.signal_for(Path::new("/saves/ark")).is_none());
        assert!(store.signal_for(Path::new("/saves/repo")).is_none());
        assert!(store.signal_for(Path::new("/saves/ksp")).is_none());
    }

    #[test]
    fn is_game_like_rejects_wine_proton_infrastructure() {
        // The Linux wrappers live alongside the game without BEING it; without
        // filtering them, `.first()` attributed the save to a wrapper.
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
        // The game's real executable (the same name sysinfo would see under
        // Proton) still passes.
        assert!(is_game_like("eu5.exe", None));
    }

    #[test]
    fn is_game_like_rejects_by_exe_path_and_basename() {
        // A generic Brave thread: the name does not give it away, the exe does.
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
    fn is_game_like_rejects_windows_system_and_overlays() {
        // Procesos de fondo de Windows que envenenaban la correlación real:
        // ssh.exe / RuntimeBroker.exe bajo C:\Windows\ (ruta de sistema)…
        assert!(!is_game_like(
            "ssh.exe",
            Some(Path::new("C:\\Windows\\System32\\OpenSSH\\ssh.exe"))
        ));
        assert!(!is_game_like(
            "RuntimeBroker.exe",
            Some(Path::new("C:\\Windows\\System32\\RuntimeBroker.exe"))
        ));
        // ...and an NVIDIA overlay in Program Files (the path misses it, the name
        // does not).
        assert!(!is_game_like(
            "NVIDIA Overlay.exe",
            Some(Path::new(
                "C:\\Program Files\\NVIDIA Corporation\\NVIDIA app\\NVIDIA Overlay.exe"
            ))
        ));
        // Otra unidad distinta de C: también cuenta como sistema.
        assert!(!is_game_like(
            "conhost.exe",
            Some(Path::new("D:\\Windows\\System32\\conhost.exe"))
        ));
        // eu5.exe instalado en la biblioteca de Steam pasa el filtro.
        assert!(is_game_like(
            "eu5.exe",
            Some(Path::new(
                "C:\\Program Files (x86)\\Steam\\steamapps\\common\\Europa Universalis V\\eu5.exe"
            ))
        ));
    }

    #[test]
    fn is_game_like_rejects_onedrive_xbox_gamingservices_opencode() {
        // OneDrive sync daemon, Xbox infrastructure, GamingServices, opencode.
        for n in [
            "OneDrive.exe",
            "xbox.exe",
            "GamingServices.exe",
            "opencode",
            "opencode.exe",
        ] {
            assert!(!is_game_like(n, None), "{n} should be filtered");
        }
        // Real games still pass.
        assert!(is_game_like("eldenring.exe", None));
        assert!(is_game_like("Hades.exe", None));
    }

    #[test]
    fn is_game_like_rejects_desktop_ai_and_sync_apps() {
        // The real case (jul-2026 report): the app lives in
        // `%LOCALAPPDATA%\Programs\...`, so no PATH rule reached it and its
        // process name is not "electron". It passed the filter and inherited
        // another game's folder.
        for n in [
            "ChatGPT.exe",
            "chatgpt",
            "Cursor.exe",
            "zed",
            "ollama.exe",
            "obs64.exe",
            "Streamlabs OBS.exe",
            "Dropbox.exe",
            "syncthing",
            "nextcloud.exe",
        ] {
            assert!(!is_game_like(n, None), "{n} should be filtered");
        }
        // The EXACT matches must not eat a game that contains them.
        assert!(is_game_like("Precursor.exe", None));
        assert!(is_game_like("Fuzed.exe", None));
    }

    #[test]
    fn is_game_like_rejects_multiword_installers() {
        // The real case: an installer the EXACT "setup" veto did not catch, which
        // poisoned one game's folder and renamed it.
        for n in [
            "Codex Windows Sandbox Setup.exe",
            "Codex Windows Sandbox Setup",
            "Elden Ring Installer.exe",
            "vcredist_x64 setup",
            "unins000.exe",
            "MyGame-Uninstall.exe",
        ] {
            assert!(
                !is_game_like(n, None),
                "{n} should be filtered as installer"
            );
        }
        // False positives it must NOT bring down: a game that mentions the word in
        // the middle, rather than as the last token, still passes.
        assert!(is_game_like("Setup Simulator.exe", None));
        assert!(is_game_like("Installer Tycoon.exe", None));
    }

    #[test]
    fn is_game_like_rejects_code_by_exact_basename_not_substring() {
        // "code" as a substring would clobber codename.exe, decode.exe, etc.
        // The exact match catches VSCode without false positives.
        assert!(!is_game_like("code", None));
        assert!(!is_game_like("Code.exe", None));
        assert!(!is_game_like("code.exe", None));
        // Substring matches that look like "code" but aren't exact still pass.
        assert!(is_game_like("codename.exe", None));
        assert!(is_game_like("decode.exe", None));
        // By the exe's basename too, which is the case the basename exists to
        // cover: a generic thread name (which gives nothing away) plus an exe
        // OUTSIDE a system path (VSCode installed in the home, which
        // `SYSTEM_EXE_PREFIXES` does not reach). Mind how these pins are written:
        // with `/usr/...` the path rule decides and with `C:\...` the name does,
        // and neither reaches the basename, so both would pass with the basename
        // block deleted.
        assert!(!is_game_like(
            "ThreadPoolForeg",
            Some(Path::new("/home/u/.local/share/apps/vscode/code"))
        ));
        // A game whose basename is NOT "code" still passes from the same path.
        assert!(is_game_like(
            "ThreadPoolForeg",
            Some(Path::new(
                "/home/u/.local/share/apps/eldenring/eldenring.exe"
            ))
        ));
    }

    #[test]
    fn is_game_like_rejects_ubisoft_client() {
        // The client Ubisoft-on-Steam games launch inside the prefix: it rewrites
        // the save folder with its own cloud sync and outlives the game (the PoP
        // 2008 incident, jul-2026).
        assert!(!is_game_like("upc.exe", None));
        assert!(!is_game_like("UPC.exe", None));
        assert!(!is_game_like("upc", None));
        assert!(!is_game_like("UbisoftGameLauncher.exe", None));
        assert!(!is_game_like("UbisoftGameLauncher64.exe", None));
        // By the exe's basename too (the name can be a generic thread's). The path
        // is as the agent sees it in the Deck's Proton prefix: the basename is only
        // extracted with the native separator, so a Windows "Z:\..." path would
        // not work as a pin here.
        let prefix = "/home/deck/.steam/steam/steamapps/compatdata/19900/pfx/drive_c\
            /Program Files (x86)/Ubisoft/Ubisoft Game Launcher";
        assert!(!is_game_like(
            "ThreadPoolForeg",
            Some(&Path::new(prefix).join("upc.exe"))
        ));
        assert!(!is_game_like(
            "ThreadPoolForeg",
            Some(&Path::new(prefix).join("UbisoftGameLauncher64.exe"))
        ));
        // "upc" is in the EXACT list for precisely this reason: as a substring it
        // would eat legitimate exes.
        assert!(is_game_like("upcoming.exe", None));
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
        // An unrelated dir does not.
        assert!(store.signal_for(Path::new("/etc")).is_none());
        assert_eq!(store.attributed_name(&save).as_deref(), Some("game"));
    }

    #[test]
    fn record_attribution_survives_single_intruder_tick() {
        // Correlación asentada con el juego real.
        let mut store = CorrelationStore::default();
        let dir = PathBuf::from("/home/u/.local/share/Game/Saves");
        let game = GameProcess {
            name: "game.exe".into(),
            exe: Some(PathBuf::from("/games/game.exe")),
        };
        let intruder = GameProcess {
            name: "discord.exe".into(),
            exe: Some(PathBuf::from("/apps/discord.exe")),
        };
        for _ in 0..5 {
            store.record(&dir, std::slice::from_ref(&game));
        }
        assert_eq!(store.attributed_name(&dir).as_deref(), Some("game"));
        // A single tick from an intruder does NOT steal the attribution...
        store.record(&dir, std::slice::from_ref(&intruder));
        assert_eq!(store.attributed_name(&dir).as_deref(), Some("game"));
        // ...and the real game, on reappearing, resets the intruder's run.
        store.record(&dir, std::slice::from_ref(&game));
        store.record(&dir, std::slice::from_ref(&intruder));
        store.record(&dir, std::slice::from_ref(&game));
        assert_eq!(store.attributed_name(&dir).as_deref(), Some("game"));
        // Sólo tras ATTRIBUTION_SWITCH_STREAK ticks seguidos gana el retador.
        for _ in 0..ATTRIBUTION_SWITCH_STREAK {
            store.record(&dir, std::slice::from_ref(&intruder));
        }
        assert_eq!(store.attributed_name(&dir).as_deref(), Some("discord"));
    }

    #[test]
    fn is_game_like_rejects_desktop_tools_and_managers() {
        // Real tools from the jul-2026 log that passed the filter and fired "heavy
        // untracked game-like process", or worse, stayed on as a correlation
        // source.
        for n in [
            "Playnite.DesktopApp.exe",
            "AutoHotkey64.exe",
            "mspaint.exe",
            "Topaz Photo AI.exe",
            "WinRAR.exe",
            "QuickLook.exe",
            "Microsoft.Media.Player.exe",
            "GameBar.exe",
            "GameBarFTServer.exe",
            "sdxhelper.exe",
            "devenv.exe",
            "WINWORD.EXE",
            "CrossDeviceService.exe",
            "LogiPluginService.exe",
            "generate_emu_config.exe",
            "setup.exe",
            "achievements.exe",
        ] {
            assert!(!is_game_like(n, None), "{n} debería filtrarse");
        }
        // "setup" and "achievements" go by EXACT match: they eat no legitimate
        // names.
        assert!(is_game_like("setupgame.exe", None));
        assert!(is_game_like("myachievements2.exe", None));
        // Juegos reales siguen pasando.
        assert!(is_game_like("eldenring.exe", None));
        assert!(is_game_like("doomx64.exe", None));
    }

    #[test]
    fn phantom_strikes_drop_poisoned_observation() {
        let mut store = CorrelationStore::default();
        let dir = PathBuf::from("/home/u/AppData/LocalLow/Fumi Games/MOUSE/Save");
        let task = GameProcess {
            name: "hourlytask.exe".into(),
            exe: Some(PathBuf::from("/apps/hourlytask.exe")),
        };
        store.record(&dir, std::slice::from_ref(&task));
        // Primera sesión fantasma: strike, pero la observación sobrevive.
        assert_eq!(store.strike_phantom(&dir), Some(false));
        assert!(store.signal_for(&dir).is_some());
        // Segunda seguida: cae. La señal débil muere con ella.
        assert_eq!(store.strike_phantom(&dir), Some(true));
        assert!(store.signal_for(&dir).is_none());
        // With no observation, the strike is a no-op.
        assert_eq!(store.strike_phantom(&dir), None);
    }

    #[test]
    fn real_write_resets_phantom_strikes() {
        let mut store = CorrelationStore::default();
        let dir = PathBuf::from("/home/u/.local/share/EU5/save games");
        let game = GameProcess {
            name: "eu5.exe".into(),
            exe: Some(PathBuf::from("/games/eu5.exe")),
        };
        store.record(&dir, std::slice::from_ref(&game));
        assert_eq!(store.strike_phantom(&dir), Some(false));
        // A real write (record) absolves; the next strike is the first again and
        // the observation survives.
        store.record(&dir, std::slice::from_ref(&game));
        assert_eq!(store.strike_phantom(&dir), Some(false));
        assert!(store.signal_for(&dir).is_some());
        // `absolve` explícito también resetea (sesión con had_pending).
        store.absolve(&dir);
        assert_eq!(store.strike_phantom(&dir), Some(false));
        assert!(store.signal_for(&dir).is_some());
        // The strike resolves by ancestor just as `signal_for` does (the watcher
        // is recursive, so the observation can live on a parent).
        assert_eq!(store.strike_phantom(&dir.join("slot1")), Some(true));
        assert!(store.signal_for(&dir).is_none());
    }

    #[test]
    fn no_game_process_records_nothing() {
        let mut store = CorrelationStore::default();
        store.record(Path::new("/x"), &[]);
        assert!(store.is_empty());
    }

    #[test]
    fn correlation_rescues_invisible_folder() {
        // A folder with an opaque name (a GUID) and nothing in it: statically it
        // is INVISIBLE, below SCORE_POSSIBLE, and the walker discards it.
        let tmp = std::env::temp_dir().join("hoard-corr-test-guid-1234");
        let _ = std::fs::create_dir_all(&tmp);
        let static_only = crate::scoring::score_dir(&tmp, "guid-1234");
        assert!(static_only.score < crate::scoring::SCORE_POSSIBLE);

        // Correlation alone (+0.50) lifts it to "possible", so it stops being
        // discarded. To AUTO-confirm (0.60 or more) the ADR also asks for a weak
        // signal, which is deliberate.
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
        // The ADR's golden path: an opaque name plus one recent save plus process
        // correlation gives an auto-confirm with room to spare.
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
