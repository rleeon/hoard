//! Detection, automatic mode's scheduler and the game catalogue, in the service
//! (ADR 0021, Slice 8).
//!
//! They lived in the desktop, and that cost twice. With the window closed nothing
//! looked for new games, swept the tracked saves or refreshed the catalogue,
//! however long the machine ran. And the window loaded the whole catalogue to do
//! it, about 50 MB it then kept for as long as it sat in the tray, when this
//! process loads it anyway for its own backups.
//!
//! A desktop from before this still runs its own scheduler, and two of them would
//! track the same game twice, so while one is connected the passes here stand
//! aside (see [`Detect::note_client`]). What a client asks for directly runs
//! either way.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use hoard_agent::library::{self, CachedDetection};
use hoard_agent::prefs::Prefs;
use hoard_agent::supervisor::Finished;
use hoard_core::ipc::{CatalogInfo, Hello, ScanNote, CAP_DETECTION};
use tokio::sync::{broadcast, Notify};

use crate::engine::Engine;

/// Notes queued per subscriber before a slow one starts missing some. They are
/// progress, so a lagging window loses a tick of a bar, never data.
const NOTES: usize = 64;

/// How often the scheduler wakes to see what is due. The intervals themselves
/// are the user's, in `prefs.json`.
const TICK: Duration = Duration::from_secs(30);

/// Floors, so a hand-edited `prefs.json` cannot spin a tight loop. Not defaults:
/// those live in `prefs.rs` (5 min scan, 1 h sweep).
const MIN_SCAN_INTERVAL: Duration = Duration::from_secs(30);
const MIN_SWEEP_INTERVAL: Duration = Duration::from_secs(60);

/// A burst of heavy-game signals (a game launch, or a process whose CPU flaps
/// around the threshold across an engine restart) is one scan, not one each.
const EVENT_SCAN_DEBOUNCE: Duration = Duration::from_secs(60);

/// With automatic mode off the Library's cache is still refreshed once a day, as
/// the desktop always did; this is how often that is checked.
const STALE_CHECK: Duration = Duration::from_secs(30 * 60);
const STALE_AFTER_SECS: i64 = 24 * 60 * 60;

/// The catalogue is looked at this often and only downloaded when a day old.
const CATALOG_RECHECK: Duration = Duration::from_secs(60 * 60);
/// Not in the first minutes after starting: a login is busy enough already.
const CATALOG_WARMUP: Duration = Duration::from_secs(2 * 60);

pub struct Detect {
    notes: broadcast::Sender<ScanNote>,
    /// One sweep at a time: the timer and a button at once would walk the disk
    /// twice for the same answer.
    gate: tokio::sync::Mutex<()>,
    /// And one catalogue download: the keeper and the Settings button would write
    /// the same file twice.
    catalog_gate: tokio::sync::Mutex<()>,
    legacy_desktops: AtomicUsize,
    kick: Notify,
    /// A scan is owed now, not on the timer: a heavy game appeared.
    scan_now: AtomicBool,
    /// The preferences changed: both halves run at once, as flipping the toggle
    /// always did.
    rearm: AtomicBool,
    last_event_scan: Mutex<Option<Instant>>,
}

impl Default for Detect {
    fn default() -> Self {
        Self::new()
    }
}

impl Detect {
    pub fn new() -> Self {
        Self {
            notes: broadcast::channel(NOTES).0,
            gate: tokio::sync::Mutex::new(()),
            catalog_gate: tokio::sync::Mutex::new(()),
            legacy_desktops: AtomicUsize::new(0),
            kick: Notify::new(),
            scan_now: AtomicBool::new(false),
            rearm: AtomicBool::new(false),
            last_event_scan: Mutex::new(None),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ScanNote> {
        self.notes.subscribe()
    }

    fn phase(&self, kind: &str, done: Option<usize>, total: Option<usize>) {
        let _ = self.notes.send(ScanNote::Phase {
            kind: kind.to_string(),
            done: done.map(|d| d as u32),
            total: total.map(|t| t as u32),
        });
    }

    /// Tells every window a pass finished, so it reads the fresh cache.
    pub fn finished(&self, tracked: usize) {
        let _ = self.notes.send(ScanNote::Finished {
            tracked: tracked as u32,
        });
    }

    /// Counts a desktop from before Slice 8 for as long as its connection lives:
    /// it runs its own scheduler, so ours stands aside meanwhile. `None` for every
    /// other client, the CLI included.
    pub fn note_client(self: &Arc<Self>, hello: &Hello) -> Option<LegacyDesktop> {
        let legacy = hello.client.starts_with("hoard-desktop")
            && !hello.caps.iter().any(|c| c == CAP_DETECTION);
        if !legacy {
            return None;
        }
        if self.legacy_desktops.fetch_add(1, Ordering::SeqCst) == 0 {
            tracing::info!(
                client = %hello.client,
                "hoardd: a desktop that scans on its own is connected; automatic mode stays with it"
            );
        }
        Some(LegacyDesktop(self.clone()))
    }

    fn standing_aside(&self) -> bool {
        self.legacy_desktops.load(Ordering::SeqCst) > 0
    }

    pub fn automatic_changed(&self) {
        self.rearm.store(true, Ordering::SeqCst);
        self.kick.notify_one();
    }

    /// A heavy untracked game appeared: scan now rather than on the timer.
    pub fn heavy_process(&self) {
        {
            let mut last = self.last_event_scan.lock().unwrap();
            if last.is_some_and(|t| t.elapsed() < EVENT_SCAN_DEBOUNCE) {
                tracing::debug!("event scan: debounced, a scan ran recently");
                return;
            }
            *last = Some(Instant::now());
        }
        self.scan_now.store(true, Ordering::SeqCst);
        self.kick.notify_one();
    }

    /// One sweep, its progress out to the subscribers and its result in the cache
    /// on disk. The heap goes back to the system afterwards (see [`trim_heap`]).
    pub async fn scan(&self, deep: bool) -> anyhow::Result<CachedDetection> {
        let _one_at_a_time = self.gate.lock().await;
        let notes = self.notes.clone();
        // The sweep reports once per catalogue task, tens of thousands a pass: only
        // whole percents go out.
        let shown = AtomicU32::new(u32::MAX);
        let progress = move |done: usize, total: usize| {
            let pct = if total == 0 {
                100
            } else {
                (done * 100 / total) as u32
            };
            if shown.swap(pct, Ordering::Relaxed) != pct {
                let _ = notes.send(ScanNote::Progress {
                    done: done as u32,
                    total: total as u32,
                });
            }
        };
        let result = library::detect_filtered(deep, progress)
            .await
            .map(library::remember_detection);
        trim_heap();
        result
    }

    /// Downloads the catalogue again, its stages out to the subscribers.
    pub async fn refresh_catalog(&self) -> anyhow::Result<CatalogInfo> {
        let _one_at_a_time = self.catalog_gate.lock().await;
        let notes = self.notes.clone();
        let result = hoard_agent::catalog::refresh(move |stage| {
            let _ = notes.send(ScanNote::Catalog {
                stage: stage.to_string(),
            });
        })
        .await;
        trim_heap();
        let update = result?;
        Ok(CatalogInfo {
            games: update.games as u64,
            has_runtime_override: true,
            updated_at: Some(update.updated_at),
            size_bytes: Some(update.size_bytes),
        })
    }
}

/// A desktop from before Slice 8, counted while its connection lasts.
pub struct LegacyDesktop(Arc<Detect>);

impl Drop for LegacyDesktop {
    fn drop(&mut self) {
        if self.0.legacy_desktops.fetch_sub(1, Ordering::SeqCst) == 1 {
            tracing::info!(
                "hoardd: no desktop that scans on its own is left; automatic mode is ours again"
            );
        }
    }
}

pub fn catalog_info() -> CatalogInfo {
    let status = hoard_agent::catalog::status();
    CatalogInfo {
        games: status.games as u64,
        has_runtime_override: status.has_runtime_override,
        updated_at: status.updated_at,
        size_bytes: None,
    }
}

pub async fn diagnose(slug: &str) -> anyhow::Result<hoard_agent::detection::DetectionTrace> {
    let (state, _) = hoard_agent::state::CliState::load_default()?;
    Ok(hoard_agent::detection::diagnose(slug, hoard_agent::manifest::Os::current(), &state).await)
}

/// Automatic mode's timer, and the daily refresh of the Library's cache when it
/// is off. It runs one round right away, as flipping the toggle on always did,
/// then wakes every [`TICK`], or at once on a kick.
pub async fn scheduler(detect: Arc<Detect>, engine: Engine) -> Finished {
    let mut last_scan: Option<Instant> = None;
    let mut last_sweep: Option<Instant> = None;
    let mut last_stale_check: Option<Instant> = None;
    loop {
        if detect.rearm.swap(false, Ordering::SeqCst) {
            last_scan = None;
            last_sweep = None;
        }
        match Prefs::load_default() {
            Err(e) => tracing::warn!(error = %e, "automatic mode: couldn't read prefs"),
            Ok(_) if detect.standing_aside() => {}
            // Tracking goes through the engine's client, so the pass waits for one:
            // the first after a sign-in, or after this service starts, then runs on
            // the next tick rather than a whole interval later.
            Ok((prefs, _)) if prefs.automatic_mode && engine.client().is_some() => {
                let scan_every =
                    Duration::from_secs(prefs.automatic_scan_interval_secs).max(MIN_SCAN_INTERVAL);
                let owed = detect.scan_now.swap(false, Ordering::SeqCst);
                if owed || last_scan.map_or(true, |t| t.elapsed() >= scan_every) {
                    automatic_pass(&detect, &engine).await;
                    last_scan = Some(Instant::now());
                }
                let sweep_every = Duration::from_secs(prefs.automatic_backup_interval_secs)
                    .max(MIN_SWEEP_INTERVAL);
                if last_sweep.map_or(true, |t| t.elapsed() >= sweep_every) {
                    // No engine yet is not a sweep missed: the next tick tries again.
                    if let Some(handle) = engine.handle() {
                        match handle.sweep_all(prefs.automatic_backup_interval_secs).await {
                            Ok(()) => last_sweep = Some(Instant::now()),
                            Err(e) => tracing::warn!(
                                error = %format!("{e:#}"),
                                "automatic backup sweep failed"
                            ),
                        }
                    }
                }
            }
            // Off, or on and waiting for an engine: the Library's cache still gets
            // its daily refresh.
            Ok((prefs, _)) => {
                // Heavy-game scans belong to automatic mode: with it off, the user
                // never asked for games to be tracked behind their back. With it on
                // the scan stays owed until there is an engine to track with.
                if !prefs.automatic_mode {
                    detect.scan_now.store(false, Ordering::SeqCst);
                }
                if last_stale_check.map_or(true, |t| t.elapsed() >= STALE_CHECK) {
                    last_stale_check = Some(Instant::now());
                    if cache_is_stale() {
                        tracing::info!("detection cache older than 24h, refreshing in background");
                        match detect.scan(false).await {
                            Ok(_) => detect.finished(0),
                            Err(e) => tracing::warn!(
                                error = %format!("{e:#}"),
                                "background detection refresh failed"
                            ),
                        }
                    }
                }
            }
        }
        tokio::select! {
            _ = tokio::time::sleep(TICK) => {}
            _ = detect.kick.notified() => {}
        }
    }
}

/// The Library's cache is a day old. No cache at all is not stale: a fresh
/// install waits for the user's first scan rather than walking the disk unasked.
fn cache_is_stale() -> bool {
    library::load_detection_from_disk().is_some_and(|cached| {
        (time::OffsetDateTime::now_utc() - cached.scanned_at).whole_seconds() >= STALE_AFTER_SECS
    })
}

/// One automatic pass: sweep, track what is new, tell the engine, and hand it the
/// folders to probe.
async fn automatic_pass(detect: &Detect, engine: &Engine) {
    detect.phase("detecting", None, None);
    let cached = match detect.scan(false).await {
        Ok(cached) => cached,
        Err(e) => {
            tracing::warn!(error = %format!("{e:#}"), "automatic scan: detection failed");
            detect.phase("idle", None, None);
            return;
        }
    };
    let Some(client) = engine.client() else {
        // Signed out, or the engine still starting: the cache is fresh for the
        // Library all the same, and the next pass does the tracking.
        tracing::info!("automatic scan: no engine to track with yet");
        detect.phase("idle", None, None);
        detect.finished(0);
        return;
    };
    let notes = detect.notes.clone();
    let tracking = library::run_auto_track(&client, cached.report.games, move |done, total| {
        let _ = notes.send(ScanNote::Phase {
            kind: "tracking".to_string(),
            done: Some(done as u32),
            total: Some(total as u32),
        });
    })
    .await;
    let run = match tracking {
        Ok(run) => run,
        Err(e) => {
            tracing::warn!(error = %format!("{e:#}"), "automatic scan: couldn't list tracked saves");
            detect.phase("idle", None, None);
            detect.finished(0);
            return;
        }
    };

    detect.phase("starting_agent", None, None);
    if run.tracked > 0 || !run.pruned.is_empty() {
        if let Err(e) = crate::engine::reload(engine).await {
            tracing::warn!(error = %format!("{e:#}"), "automatic scan: couldn't reload the watch list");
        }
    }
    if let Some(handle) = engine.handle() {
        let count = run.probe.len();
        match handle.set_probe_candidates(run.probe).await {
            Ok(()) => tracing::debug!(count, "automatic scan: probe candidates handed to the engine"),
            Err(e) => tracing::warn!(error = %format!("{e:#}"), "automatic scan: couldn't hand over the probe candidates"),
        }
    }
    detect.phase("idle", None, None);
    detect.finished(run.tracked);
    tracing::info!(tracked = run.tracked, "automatic scan: done");
}

/// Refreshes the catalogue once a day. The check is a metadata read; the 17 MB
/// download only happens when the copy is a day old.
pub async fn catalog_keeper(detect: Arc<Detect>) -> Finished {
    tokio::time::sleep(CATALOG_WARMUP).await;
    loop {
        if !detect.standing_aside() && hoard_agent::catalog::is_stale() {
            tracing::info!("Ludusavi catalog stale; running background refresh");
            match detect.refresh_catalog().await {
                Ok(info) => tracing::info!(games = info.games, "auto-refresh complete"),
                Err(e) => tracing::warn!(
                    error = %format!("{e:#}"),
                    "auto-refresh failed; keeping the catalogue in use"
                ),
            }
        }
        tokio::time::sleep(CATALOG_RECHECK).await;
    }
}

/// Hands the heap glibc freed back to the system. A sweep or a catalogue refresh is
/// a burst in a process that then idles for hours, and glibc keeps what a burst
/// freed: after a refresh that was 125 MB, never returned.
fn trim_heap() {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    {
        // SAFETY: `malloc_trim` only walks glibc's own arenas and takes no pointer
        // of ours.
        unsafe {
            libc::malloc_trim(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hello(client: &str, caps: &[&str]) -> Hello {
        Hello {
            protocol: hoard_core::ipc::PROTOCOL_VERSION,
            client: client.into(),
            caps: caps.iter().map(|c| c.to_string()).collect(),
        }
    }

    #[test]
    fn only_a_desktop_without_the_cap_makes_automatic_mode_stand_aside() {
        let detect = Arc::new(Detect::new());
        assert!(detect.note_client(&hello("hoard 1.2.0 (sync)", &[])).is_none());
        assert!(detect
            .note_client(&hello("hoard-desktop 1.2.0 (events)", &[CAP_DETECTION]))
            .is_none());
        assert!(!detect.standing_aside());

        let old = detect.note_client(&hello("hoard-desktop 1.1.6 (events)", &[]));
        assert!(detect.standing_aside());
        drop(old);
        assert!(!detect.standing_aside(), "its connection closing gives it back");
    }

    #[test]
    fn a_burst_of_heavy_games_owes_one_scan() {
        let detect = Detect::new();
        detect.heavy_process();
        assert!(detect.scan_now.swap(false, Ordering::SeqCst));
        detect.heavy_process();
        assert!(!detect.scan_now.load(Ordering::SeqCst), "debounced");
    }
}
