//! **The service updates itself.**
//!
//! What checks for a new version is the service and not the window, for the same
//! reason the engine lives here (ADR 0021) and native notifications go out from
//! here (D.14.1): **it is the only thing that is always there**. The window was
//! closed, the terminal was not opened for two weeks, and the sync has still been
//! running for days with a bug that was fixed three releases ago.
//!
//! ## The split
//!
//! - The **policy** (when to download, when to apply, when it stops being
//!   optional) is pure and lives in `hoard_agent::install::auto`.
//! - The **mechanics** (which file, which signature, where it goes) live in
//!   `hoard_agent::install::{fetch, stage}`, shared with `hoard install`.
//! - What is left here is the loop: ask, decide, do, get relieved.
//!
//! ## What this loop never does
//!
//! **It opens no dialogs.** A background service that makes a polkit window appear
//! at three in the morning is worse than not updating. In the background cycle
//! everything runs `noninteractive`, so the routes that need a human (`.deb`,
//! `.rpm`, `.dmg`) only move when somebody asks from a client
//! ([`hoard_core::ipc::Request::ApplyUpdate`]), and then yes, with the dialog in
//! front of whoever just asked for it.
//!
//! Past the deadline it is tried anyway, but only down the routes that do not ask
//! (we are root already, or there is a `sudo` with a cached credential). Failing
//! that, the update stays marked mandatory and the first window to open resolves
//! it.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use hoard_agent::install::auto::{self, Hold, Ledger, Situation, Stance};
use hoard_agent::install::{stage, Manifest};
use hoard_agent::supervisor::Finished;
use hoard_core::ipc::{UpdateHold, UpdatePhase, UpdateState};
use time::OffsetDateTime;

use crate::engine::Engine;

/// How often GitHub is asked in normal running.
///
/// Half an hour was the cadence of the window's amber badge and it is more than
/// enough here: the service does not close, so over a day that is 24 unauthenticated
/// requests against a limit of 60/h. What matters is not finding out soon, it is
/// finding out **always**.
const POLL: Duration = Duration::from_secs(60 * 60);

/// The short cadence while something is pending that could not be applied yet (a
/// game open, an upload halfway through). It probes the brake, not GitHub: the
/// version is not asked about again until [`POLL`] comes round.
const RETRY: Duration = Duration::from_secs(60);

/// A breather before the first cycle. The service's start already competes with the
/// engine's, the login's and the session's; a 90 MB download the moment you sign in
/// is exactly what not to do.
const WARMUP: Duration = Duration::from_secs(90);

/// The cap on consecutive failures before the attempts get spaced out. Without it,
/// a release that publishes no package for this architecture retries every minute
/// for ever, which is the compression hot loop (6 blobs with no terminal state,
/// retrying since July) written all over again.
const MAX_FAILURES: u32 = 5;

// ---- what the updater shows

/// The updater's shared view: what [`hoard_core::ipc::Request::UpdateStatus`]
/// answers.
///
/// It is an `Arc<Mutex<...>>` and not a channel because clients ask whenever they
/// feel like it; there is nobody to push to when nobody is connected, which is half
/// the time.
#[derive(Clone)]
pub struct Updater {
    inner: Arc<Mutex<Live>>,
    /// A client asked to apply now. It wakes the loop, which is what applies:
    /// applying from an IPC connection's thread would leave two applications
    /// treading on each other if the user clicked twice.
    poke: Arc<tokio::sync::Notify>,
}

#[derive(Default)]
struct Live {
    phase: Phase,
    latest: Option<String>,
    staged: Option<String>,
    deadline: Option<OffsetDateTime>,
    mandatory: bool,
    unattended: bool,
    last_error: Option<String>,
    /// The version a client asked to apply, waiting for the loop to pick it up.
    requested: Option<Option<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Phase {
    #[default]
    UpToDate,
    Downloading,
    Ready,
    Waiting(Hold),
    Applying,
    Restarting,
    Failed,
    Managed,
}

impl Updater {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Live::default())),
            poke: Arc::new(tokio::sync::Notify::new()),
        }
    }

    /// Lo que ve un cliente.
    pub fn state(&self) -> UpdateState {
        let live = self.lock();
        UpdateState {
            current: env!("CARGO_PKG_VERSION").to_string(),
            latest: live.latest.clone(),
            staged: live.staged.clone(),
            phase: match live.phase {
                Phase::UpToDate => UpdatePhase::UpToDate,
                Phase::Downloading => UpdatePhase::Downloading,
                Phase::Ready => UpdatePhase::Ready,
                Phase::Waiting(hold) => UpdatePhase::Waiting {
                    hold: match hold {
                        Hold::GameRunning => UpdateHold::GameRunning,
                        Hold::TransferInFlight => UpdateHold::TransferInFlight,
                    },
                },
                Phase::Applying => UpdatePhase::Applying,
                Phase::Restarting => UpdatePhase::Restarting,
                Phase::Failed => UpdatePhase::Failed,
                Phase::Managed => UpdatePhase::Managed,
            },
            deadline: live.deadline,
            mandatory: live.mandatory,
            unattended: live.unattended,
            last_error: live.last_error.clone(),
        }
    }

    /// A client asks to apply now. It returns immediately: the loop is what
    /// applies, and what happened is read afterwards through [`Updater::state`].
    pub fn apply_now(&self, version: Option<String>) {
        self.lock().requested = Some(version);
        self.poke.notify_one();
    }

    /// "Not now", for `hours`. It does not move the deadline.
    pub fn snooze(&self, hours: u32) {
        let until = OffsetDateTime::now_utc() + time::Duration::hours(hours.min(24 * 7) as i64);
        let mut ledger = Ledger::load();
        ledger.snoozed_until = Some(until);
        if let Err(err) = ledger.save() {
            tracing::warn!(error = %format!("{err:#}"), "hoardd: couldn't record the update snooze");
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Live> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn take_request(&self) -> Option<Option<String>> {
        self.lock().requested.take()
    }

    fn set_phase(&self, phase: Phase) {
        self.lock().phase = phase;
    }

    fn fail(&self, error: String) {
        let mut live = self.lock();
        live.phase = Phase::Failed;
        live.last_error = Some(error);
    }
}

impl Default for Updater {
    fn default() -> Self {
        Self::new()
    }
}

// =======================================================================
// El bucle
// =======================================================================

/// Why the loop stops. There is only one reason: something was applied and the new
/// binary has to be started.
pub struct Relaunch {
    pub version: String,
}

/// Watches, downloads and applies. It runs under `supervisor::supervise` like
/// everything that outlives a request (D.12): a panic here is a logged incident and
/// a restart, not a service that quietly stops updating for ever.
pub async fn watch(
    updater: Updater,
    engine: Engine,
    notifier: Arc<crate::notify::Notifier>,
    relaunch: tokio::sync::mpsc::Sender<Relaunch>,
) -> Finished {
    tokio::time::sleep(WARMUP).await;
    let mut next_poll = Duration::ZERO;

    loop {
        if next_poll > Duration::ZERO {
            tokio::select! {
                _ = tokio::time::sleep(next_poll) => {}
                // A client asked to apply, so there is no waiting for the hour.
                _ = updater.poke.notified() => {}
            }
        }

        let requested = updater.take_request();
        next_poll = match tick(&updater, &engine, &notifier, requested, &relaunch).await {
            Cadence::Normal => POLL,
            Cadence::Soon => RETRY,
        };
    }
}

/// When to come back.
enum Cadence {
    Normal,
    /// Hay algo pendiente y frenado: se vuelve pronto a mirar el freno.
    Soon,
}

async fn tick(
    updater: &Updater,
    engine: &Engine,
    notifier: &crate::notify::Notifier,
    requested: Option<Option<String>>,
    relaunch: &tokio::sync::mpsc::Sender<Relaunch>,
) -> Cadence {
    let manifest = match Manifest::load_or_observe() {
        Ok(m) => m,
        Err(err) => {
            updater.fail(format!("{err:#}"));
            return Cadence::Normal;
        }
    };

    // Nada nuestro que tocar: lo mantiene el gestor de paquetes de la distro, un
    // Flatpak, un `nix`. Se dice y no se vuelve a mirar.
    if manifest.delivery.is_some_and(|d| !d.is_ours()) {
        updater.set_phase(Phase::Managed);
        return Cadence::Normal;
    }

    let unattended = manifest.applies_unattended();
    let mut ledger = Ledger::load();

    // What we staged is what we're running: the update landed, whoever got to
    // see it. This is the only place that can close the book on Windows, where
    // the installer kills the daemon that launched it: the process that applied
    // the update is never the process that returns from applying it, so
    // without this the deadline, the staged copy and the attempt counter all
    // survive an update that worked.
    let current = hoard_agent::update::current();
    if ledger.staged.as_deref() == Some(current) {
        tracing::info!(
            version = current,
            "hoardd: started on the version it had staged"
        );
        ledger.applied(current);
        let _ = ledger.save();
        stage::sweep(current);
    }

    // Freno de mano tras varios fallos seguidos: se sigue mirando, pero al ritmo
    // largo, no al corto.
    let burnt = ledger.failures >= MAX_FAILURES;

    // GitHub is only asked when it is due; a cycle that comes back early because a
    // game is open is looking at the brake, not at the version.
    let now = OffsetDateTime::now_utc();
    let stale = ledger
        .last_check_at
        .is_none_or(|at| now - at >= time::Duration::seconds(POLL.as_secs() as i64));
    if stale {
        if let Some(latest) = hoard_agent::update::fetch_latest().await {
            ledger.observe(&latest, now);
            let _ = ledger.save();
        }
    }

    let situation = Situation {
        current: hoard_agent::update::current().to_string(),
        latest: ledger.latest_seen.clone(),
        staged: ledger.staged.clone(),
        first_seen_at: ledger.first_seen_at,
        unattended,
        transfer_in_flight: engine.transfers_in_flight(),
        game_running: game_running(engine).await,
    };
    let stance = auto::decide(now, &situation);

    {
        let mut live = updater.lock();
        live.latest.clone_from(&ledger.latest_seen);
        live.staged.clone_from(&ledger.staged);
        live.deadline = ledger.deadline();
        live.mandatory = matches!(stance, Stance::Force { .. });
        live.unattended = unattended;
    }

    match stance {
        Stance::Idle => {
            updater.set_phase(Phase::UpToDate);
            Cadence::Normal
        }

        Stance::Stage { version } => {
            if burnt {
                tracing::debug!(
                    version = %version,
                    failures = ledger.failures,
                    "hoardd: update staging is backed off after repeated failures"
                );
                return Cadence::Normal;
            }
            updater.set_phase(Phase::Downloading);
            tracing::info!(version = %version, "hoardd: downloading the update");
            match stage::stage(&version, &manifest).await {
                Ok(staged) => {
                    ledger.staged = Some(staged.version.clone());
                    ledger.staged_at = Some(OffsetDateTime::now_utc());
                    ledger.failures = 0;
                    ledger.last_error = None;
                    let _ = ledger.save();
                    updater.lock().staged = Some(staged.version);
                    updater.set_phase(Phase::Ready);
                    // It is on disk now: the next cycle decides whether to apply it,
                    // and there is no reason to wait an hour to ask.
                    Cadence::Soon
                }
                Err(err) => {
                    let message = format!("{err:#}");
                    tracing::warn!(version = %version, error = %message, "hoardd: couldn't stage the update");
                    ledger.failures += 1;
                    ledger.last_error = Some(message.clone());
                    let _ = ledger.save();
                    updater.fail(message);
                    Cadence::Normal
                }
            }
        }

        Stance::Waiting { version, hold } => {
            // A client asking to apply **now** must not fall on deaf ears: without
            // this the request was lost in silence and the button did nothing, which
            // is worse than a disabled button.
            //
            // The two brakes are treated differently because they are not the same.
            // "A game is open" is a courtesy of ours, and the user can waive it, since
            // they asked. "An upload is halfway through" is no courtesy: swapping the
            // binaries there kills the process doing it, so the request is stored and
            // served as soon as it finishes, which is seconds.
            if let Some(asked) = requested {
                match hold {
                    Hold::GameRunning => {
                        tracing::info!(version = %version, "hoardd: a client asked to update with a game running, honouring it");
                        return apply(updater, &mut ledger, &manifest, &version, false, relaunch)
                            .await;
                    }
                    Hold::TransferInFlight => {
                        updater.lock().requested = Some(asked);
                    }
                }
            }
            tracing::debug!(version = %version, ?hold, "hoardd: update is staged and waiting");
            updater.set_phase(Phase::Waiting(hold));
            Cadence::Soon
        }

        Stance::Ask { version } => {
            updater.set_phase(Phase::Ready);
            // A client asked to apply, so somebody is in front of it and the routes
            // that need a dialog can open one.
            if let Some(asked) = requested {
                if asked.as_deref().is_none_or(|v| v == version) {
                    return apply(updater, &mut ledger, &manifest, &version, false, relaunch).await;
                }
            }
            if ledger
                .snoozed_until
                .is_none_or(|until| OffsetDateTime::now_utc() >= until)
            {
                tracing::info!(version = %version, "hoardd: an update is ready and needs someone to approve it");
                // **Once per version, and only in this case.** It is the only road
                // that does not finish on its own: without this notice, somebody who
                // installed from a `.deb` and does not open the app for a week hears
                // nothing until the deadline passes, and the deadline can only take
                // over their screen if they get around to opening it.
                if ledger.notified.as_deref() != Some(version.as_str()) {
                    notifier
                        .announce(crate::notify::Kind::UpdateReady {
                            version: version.clone(),
                        })
                        .await;
                    ledger.notified = Some(version.clone());
                    let _ = ledger.save();
                }
            }
            Cadence::Normal
        }

        Stance::ApplyQuietly { version } | Stance::Force { version } => {
            if burnt && requested.is_none() {
                return Cadence::Normal;
            }
            // `noninteractive` even here: the background cycle has no window to
            // paint a dialog in, so a `pkexec` launched from here would wait for ever
            // on somebody who will never see it. Only when a client asks explicitly is
            // asking allowed.
            let interactive = requested.is_some();
            apply(
                updater,
                &mut ledger,
                &manifest,
                &version,
                !interactive,
                relaunch,
            )
            .await
        }
    }
}

/// Aplica lo bajado y pide el relevo.
async fn apply(
    updater: &Updater,
    ledger: &mut Ledger,
    manifest: &Manifest,
    version: &str,
    noninteractive: bool,
    relaunch: &tokio::sync::mpsc::Sender<Relaunch>,
) -> Cadence {
    let Some(staged) = stage::already_staged(version, manifest) else {
        // What was downloaded is gone (a cache clean, a full disk). It is forgotten
        // and the next cycle downloads it again.
        ledger.staged = None;
        ledger.staged_at = None;
        let _ = ledger.save();
        return Cadence::Soon;
    };

    updater.set_phase(Phase::Applying);
    tracing::info!(version, noninteractive, "hoardd: applying the update");

    // The attempt is written down *before* it happens, which is backwards
    // everywhere except here: on Windows there is no after. The NSIS installer
    // stops `hoardd.exe` before overwriting it, so the run that applies an
    // update is killed while it waits for that installer and never reaches
    // either arm below. Left uncounted, an install that keeps failing is
    // retried every hour forever, and every retry force-closes the app the user
    // is looking at. A cycle that sees what we staged is what we're now
    // running clears this again.
    ledger.failures += 1;
    ledger.last_error = Some(format!("applying {version} never reported back"));
    let _ = ledger.save();

    let mut manifest = manifest.clone();
    match stage::apply(&staged, &mut manifest, noninteractive).await {
        Ok(()) => {
            ledger.applied(version);
            let _ = ledger.save();
            {
                let mut live = updater.lock();
                live.phase = Phase::Restarting;
                live.staged = None;
                live.mandatory = false;
                live.last_error = None;
            }
            tracing::info!(
                version,
                "hoardd: update applied, relaunching on the new binary"
            );
            // The relief does not happen here: there is an engine to stop and a
            // socket to release, and `run` owns those. A full or closed channel means
            // a relief is under way already.
            let _ = relaunch.try_send(Relaunch {
                version: version.to_string(),
            });
            Cadence::Normal
        }
        Err(err) => {
            let message = format!("{err:#}");
            tracing::warn!(version, error = %message, "hoardd: couldn't apply the update");
            // Already counted above; this only puts the real reason in place of
            // the placeholder.
            ledger.last_error = Some(message.clone());
            let _ = ledger.save();
            updater.fail(message);
            // A privilege failure is not the updater failing: it means somebody has
            // to be in front of it. The first window to open resolves it, so it is
            // left marked pending rather than silent.
            Cadence::Normal
        }
    }
}

/// Is any game open right now? The engine is asked, since it is what correlates
/// process to folder; with no engine there are no games to speak of.
async fn game_running(engine: &Engine) -> bool {
    crate::engine::slot_status(engine)
        .await
        .iter()
        .any(|s| s.process_running)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_updater_says_nothing_is_pending() {
        let u = Updater::new();
        let s = u.state();
        assert_eq!(s.phase, UpdatePhase::UpToDate);
        assert!(!s.mandatory);
        assert_eq!(s.latest, None);
        assert_eq!(s.current, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn a_client_request_is_taken_exactly_once() {
        let u = Updater::new();
        u.apply_now(Some("1.2.3".into()));
        assert_eq!(u.take_request(), Some(Some("1.2.3".into())));
        // The second read comes back empty: two presses of the button must not turn
        // into two applications treading on each other.
        assert_eq!(u.take_request(), None);
    }

    #[test]
    fn every_hold_survives_the_trip_to_the_wire() {
        for (hold, expected) in [
            (Hold::GameRunning, UpdateHold::GameRunning),
            (Hold::TransferInFlight, UpdateHold::TransferInFlight),
        ] {
            let u = Updater::new();
            u.set_phase(Phase::Waiting(hold));
            assert_eq!(u.state().phase, UpdatePhase::Waiting { hold: expected });
        }
    }

    #[test]
    fn a_failure_is_visible_to_clients() {
        let u = Updater::new();
        u.fail("no package for aarch64".into());
        let s = u.state();
        assert_eq!(s.phase, UpdatePhase::Failed);
        assert_eq!(s.last_error.as_deref(), Some("no package for aarch64"));
    }
}
