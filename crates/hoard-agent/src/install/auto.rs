//! When Hoard updates itself, and when that stops being optional.
//!
//! Updating used to be a button: the app checked GitHub every half hour, drew an
//! amber badge and waited. Anybody who did not press it stayed on their version
//! forever, and since `hoard`, `hoardd` and the app move together or not at all
//! (see [`super`]), "forever" means a bug fixed three releases ago is still alive
//! on machines that have been switched on for months.
//!
//! This module is the policy that turns that button into what Steam does: it
//! downloads itself, applies itself, and when it cannot apply itself it applies on
//! open. The decision is pure ([`decide`] touches neither network nor disk)
//! because the case that matters (what happens on the second day, with a game
//! open, on a machine where the package needs root?) has to be testable without
//! such a machine in front of you.
//!
//! ## The two questions
//!
//! Everything comes from separating two things that used to be conflated:
//!
//! 1. Can it be applied without bothering anybody? Neither the user nor a
//!    preference decides that: the route the app arrived by does
//!    ([`super::Delivery`]). An AppImage and a per-user NSIS write into the home
//!    and nobody notices; a `.deb` needs a polkit dialog and a `.dmg` needs a hand
//!    dragging it. The first family really does update itself; the second can only
//!    do it with somebody present.
//! 2. Has the deadline passed? A clock starts the moment a new version is seen
//!    ([`GRACE`], 48 hours). Before it rings, an update that needs somebody is
//!    offered and can be postponed. Afterwards, it is not.
//!
//! The silent case never reaches the window at all: when the route applies itself
//! and the machine is idle, the user finds out from the version number. The
//! deadline exists for the other half.
//!
//! ## What the deadline does not run over
//!
//! "Mandatory" is not "right now whatever happens". Relieving the core restarts
//! `hoardd`, and doing that with an upload half done leaves a blob dangling. So
//! there are two brakes, and the deadline only lifts one:
//!
//! - A transfer in flight brakes always, deadline or not. It is seconds or
//!   minutes, and waiting for it costs nobody anything.
//! - An open game brakes the silent update (restarting the engine mid-game is
//!   exactly when sync matters) but not the mandatory one. Otherwise somebody who
//!   leaves a game open for a week does not update for a week, which is the
//!   problem we came to solve.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use super::{Component, Delivery, Manifest};

/// How long an old version is tolerated once the new one is seen. Two days: what
/// the user asked for, and enough to let a weekend away from the computer pass
/// without Monday's first session being a forced update.
pub const GRACE: Duration = Duration::from_secs(48 * 60 * 60);

/// Overrides [`GRACE`], in hours. For testing the deadline without waiting two
/// days, and for a machine that wants to be stricter; it is not documented as a
/// user option because the long deadline is the policy, not a preference.
pub const GRACE_ENV: &str = "HOARD_UPDATE_GRACE_HOURS";

/// El plazo efectivo de esta máquina.
pub fn grace() -> Duration {
    match std::env::var(GRACE_ENV)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
    {
        Some(hours) => Duration::from_secs(hours * 3600),
        None => GRACE,
    }
}

// =======================================================================
// Qué toca hacer ahora mismo
// =======================================================================

/// What to do about the new version at this exact moment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "stance", rename_all = "snake_case")]
pub enum Stance {
    /// Nothing: either there is no new version, or this install is not ours.
    Idle,
    /// There is a new version and it is not downloaded. Download it and verify its
    /// signature; nothing gets applied yet.
    ///
    /// Downloading before deciding is what makes "update on open" take as long as a
    /// `rename` rather than as long as a 90 MB bundle.
    Stage { version: String },
    /// Bajada, verificada, y esta máquina puede relevarse sin pedirle nada a
    /// nadie. Aplicar sin decir nada.
    ApplyQuietly { version: String },
    /// Downloaded, but applying it needs somebody present (polkit, a `.dmg`). It
    /// gets offered, and can be postponed.
    Ask { version: String },
    /// The deadline has passed. It gets applied, and if it needs somebody present
    /// the window does not let them carry on until it is.
    Force { version: String },
    /// An update is due and this is not the moment. The reason travels inside,
    /// because a mute brake is indistinguishable from a broken updater, which is
    /// exactly how 36 minutes were lost in D.12.
    Waiting { version: String, hold: Hold },
}

impl Stance {
    /// The version it points at, if it points at one.
    pub fn version(&self) -> Option<&str> {
        match self {
            Stance::Idle => None,
            Stance::Stage { version }
            | Stance::ApplyQuietly { version }
            | Stance::Ask { version }
            | Stance::Force { version }
            | Stance::Waiting { version, .. } => Some(version),
        }
    }

    /// ¿Esto se aplica sin preguntar?
    pub fn is_automatic(&self) -> bool {
        matches!(
            self,
            Stance::ApplyQuietly { .. } | Stance::Force { .. } | Stance::Stage { .. }
        )
    }
}

/// Why it is waiting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Hold {
    /// A backup or restore is half done. It brakes always.
    TransferInFlight,
    /// A game is open. It brakes the silent path, not the mandatory one.
    GameRunning,
}

/// The facts the decision is made from. The caller (the daemon) gathers them all;
/// nothing is inspected here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Situation {
    /// La versión que corre ahora mismo.
    pub current: String,
    /// The latest published one we know of. `None` means it could not be asked.
    pub latest: Option<String>,
    /// What is already downloaded and verified on disk.
    pub staged: Option<String>,
    /// When [`Situation::latest`] was first seen. The deadline comes from this.
    pub first_seen_at: Option<OffsetDateTime>,
    /// ¿Puede esta instalación relevarse entera sin privilegios ni manos?
    /// Sale de [`Manifest::applies_unattended`].
    pub unattended: bool,
    /// ¿Hay una copia o restauración en vuelo?
    pub transfer_in_flight: bool,
    /// ¿Hay un juego abierto?
    pub game_running: bool,
}

/// What to do now. Pure: the same inputs give the same answer.
///
/// The order of the branches IS the policy, and one of them surprises people: the
/// deadline is checked *after* downloading and *before* the idle check. Downloading
/// first because forcing what is not on disk is promising an instant update that is
/// going to take a minute; the deadline before the idle check because waiting for
/// idle is exactly what the deadline exists to stop doing.
pub fn decide(now: OffsetDateTime, s: &Situation) -> Stance {
    let Some(latest) = s.latest.as_deref() else {
        return Stance::Idle;
    };
    if !crate::update::is_newer(latest, &s.current) {
        return Stance::Idle;
    }
    let version = latest.to_string();

    // Downloaded is not the same as *this* being downloaded: a release published
    // while the previous one sat in the cache leaves `staged` pointing at the old
    // one, and applying that would knowingly install something that is no longer
    // the latest.
    if s.staged.as_deref() != Some(latest) {
        return Stance::Stage { version };
    }

    // Una transferencia a medias frena todo, incluido lo obligatorio.
    if s.transfer_in_flight {
        return Stance::Waiting {
            version,
            hold: Hold::TransferInFlight,
        };
    }

    let overdue = s
        .first_seen_at
        .is_some_and(|seen| now - seen >= grace().try_into().unwrap_or(time::Duration::ZERO));

    if overdue {
        return Stance::Force { version };
    }

    // An open game only brakes the silent path. Past the deadline, waiting for the
    // game to close is never updating.
    if s.game_running {
        return Stance::Waiting {
            version,
            hold: Hold::GameRunning,
        };
    }

    if s.unattended {
        Stance::ApplyQuietly { version }
    } else {
        Stance::Ask { version }
    }
}

// =======================================================================
// El registro en disco
// =======================================================================

/// What has to be remembered between starts: what was seen, when it was first seen
/// (the deadline's clock), and what is downloaded.
///
/// It lives in the state directory rather than in the preferences on purpose: it is
/// not something the user chooses, it is the updater's notebook. Mixing it in with
/// the preferences would make deleting preferences reset the deadline.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ledger {
    /// La última versión publicada que hemos visto.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_seen: Option<String>,
    /// When we first saw it. The deadline's clock.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub first_seen_at: Option<OffsetDateTime>,
    /// Qué versión está bajada y verificada en `staging_dir`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub staged: Option<String>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub staged_at: Option<OffsetDateTime>,
    /// The last time GitHub was asked (so it is not asked in a loop).
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub last_check_at: Option<OffsetDateTime>,
    /// "Not now": until when what can be postponed stays quiet. It does not affect
    /// the deadline, since postponing delays the question, not the due date.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub snoozed_until: Option<OffsetDateTime>,
    /// Which version the user has already been told about. Only used on the path
    /// that needs somebody present; without it the notice would appear every cycle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notified: Option<String>,
    /// What went wrong on the last attempt, and how many in a row. The second is
    /// what brakes the hot loop: a release whose asset does not exist for this
    /// architecture would fail every five minutes forever.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    #[serde(default)]
    pub failures: u32,
}

impl Ledger {
    /// `<state>/update.json`.
    pub fn path() -> anyhow::Result<std::path::PathBuf> {
        Ok(crate::config::CliConfig::state_dir()?.join("update.json"))
    }

    /// Reads the record. An unreadable or corrupt file is treated as "there is no
    /// record": the worst case is resetting the deadline, and that is infinitely
    /// better than an updater that will not start because a JSON was left half
    /// written.
    pub fn load() -> Self {
        let Ok(path) = Self::path() else {
            return Self::default();
        };
        std::fs::read_to_string(path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::path()?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    /// Notes what GitHub just said.
    ///
    /// The deadline's clock resets only when the version changes. Resetting it on
    /// every poll is the bug that would stop the deadline ever ringing; not
    /// resetting it when a newer version ships would force the new one on the
    /// previous one's deadline, which is the opposite.
    pub fn observe(&mut self, latest: &str, now: OffsetDateTime) {
        self.last_check_at = Some(now);
        if self.latest_seen.as_deref() != Some(latest) {
            self.latest_seen = Some(latest.to_string());
            self.first_seen_at = Some(now);
            // What was downloaded belonged to the previous version, so it is no
            // good now.
            if self.staged.as_deref() != Some(latest) {
                self.staged = None;
                self.staged_at = None;
            }
            self.snoozed_until = None;
            self.notified = None;
            self.failures = 0;
            self.last_error = None;
        }
    }

    /// Closes the cycle: this version is running now. It leaves the notebook blank
    /// for the next one.
    pub fn applied(&mut self, version: &str) {
        if self.latest_seen.as_deref() == Some(version) {
            self.first_seen_at = None;
        }
        self.staged = None;
        self.staged_at = None;
        self.snoozed_until = None;
        self.notified = None;
        self.failures = 0;
        self.last_error = None;
    }

    /// When it stops being optional, if anything is pending at all.
    pub fn deadline(&self) -> Option<OffsetDateTime> {
        let seen = self.first_seen_at?;
        let g: time::Duration = grace().try_into().ok()?;
        Some(seen + g)
    }
}

// =======================================================================
// ¿Se aplica sola esta instalación?
// =======================================================================

impl Delivery {
    /// Can this route be relieved without dialogs and without hands?
    ///
    /// It is not the negation of [`Delivery::needs_elevation`]: a `.dmg` asks for no
    /// privileges and still needs a person dragging it into the Finder. The
    /// question that matters here is "can this happen while nobody is looking?",
    /// and only two routes answer yes.
    pub fn applies_unattended(self) -> bool {
        matches!(self, Delivery::AppImage | Delivery::Nsis)
    }
}

impl Manifest {
    /// Can this machine relieve itself entirely without asking for privileges or
    /// hands?
    ///
    /// Entirely: [`super`]'s rule is that the pieces go to the same version or none
    /// of them moves, so one piece needing a dialog is enough for the whole update
    /// to need one. Applying the core silently and leaving the app waiting on a
    /// `pkexec` the user cancels is precisely the silent mismatch this module exists
    /// not to create.
    pub fn applies_unattended(&self) -> bool {
        if let Some(d) = self.delivery {
            if !d.is_ours() {
                return false;
            }
        }
        if self.has(Component::Desktop) && !self.delivery.is_some_and(|d| d.applies_unattended()) {
            return false;
        }
        // A core inside the bundle does not relieve itself: the app's installer
        // brings it, so it inherits that answer (which is already `true` if we got
        // this far with Desktop installed).
        if self.core_from_bundle {
            return self.has(Component::Desktop);
        }
        self.core_dir.as_deref().is_some_and(dir_is_writable)
    }
}

/// Can we write here without being root? It is tested by writing rather than
/// inferred from the path: `~/.local/bin` and `/usr/bin` are the usual cases, but
/// `HOARD_INSTALL_DIR` puts the core wherever the user likes and a list of known
/// paths would be silently wrong right there.
fn dir_is_writable(dir: &std::path::Path) -> bool {
    let probe = dir.join(".hoard-write-probe");
    match std::fs::write(&probe, b"") {
        Ok(()) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(days: i64) -> OffsetDateTime {
        OffsetDateTime::UNIX_EPOCH + time::Duration::days(days)
    }

    fn situation() -> Situation {
        Situation {
            current: "1.0.0".into(),
            latest: Some("1.1.0".into()),
            staged: Some("1.1.0".into()),
            first_seen_at: Some(at(0)),
            unattended: true,
            transfer_in_flight: false,
            game_running: false,
        }
    }

    #[test]
    fn nothing_to_do_without_a_newer_release() {
        let mut s = situation();
        s.latest = None;
        assert_eq!(decide(at(0), &s), Stance::Idle);

        s.latest = Some("1.0.0".into());
        assert_eq!(decide(at(0), &s), Stance::Idle);

        s.latest = Some("0.9.0".into());
        assert_eq!(decide(at(0), &s), Stance::Idle);
    }

    #[test]
    fn downloads_before_it_decides_anything_else() {
        let mut s = situation();
        s.staged = None;
        assert_eq!(
            decide(at(0), &s),
            Stance::Stage {
                version: "1.1.0".into()
            }
        );
        // Even past the deadline: forcing what is not on disk is promising a
        // `rename` and delivering a download.
        assert_eq!(
            decide(at(30), &s),
            Stance::Stage {
                version: "1.1.0".into()
            }
        );
    }

    #[test]
    fn stale_staging_is_not_good_enough() {
        let mut s = situation();
        s.staged = Some("1.0.5".into());
        assert!(matches!(decide(at(0), &s), Stance::Stage { .. }));
    }

    #[test]
    fn quiet_when_the_delivery_allows_it() {
        let s = situation();
        assert_eq!(
            decide(at(0), &s),
            Stance::ApplyQuietly {
                version: "1.1.0".into()
            }
        );
    }

    #[test]
    fn asks_when_someone_has_to_be_there() {
        let mut s = situation();
        s.unattended = false;
        assert_eq!(
            decide(at(0), &s),
            Stance::Ask {
                version: "1.1.0".into()
            }
        );
    }

    #[test]
    fn a_running_game_holds_the_quiet_path_but_not_the_deadline() {
        let mut s = situation();
        s.game_running = true;
        assert_eq!(
            decide(at(0), &s),
            Stance::Waiting {
                version: "1.1.0".into(),
                hold: Hold::GameRunning
            }
        );
        // Pasado el plazo, el juego deja de ser excusa.
        assert_eq!(
            decide(at(3), &s),
            Stance::Force {
                version: "1.1.0".into()
            }
        );
    }

    #[test]
    fn a_transfer_in_flight_holds_everything() {
        let mut s = situation();
        s.transfer_in_flight = true;
        assert_eq!(
            decide(at(30), &s),
            Stance::Waiting {
                version: "1.1.0".into(),
                hold: Hold::TransferInFlight
            }
        );
    }

    #[test]
    fn the_deadline_overrides_the_prompt() {
        let mut s = situation();
        s.unattended = false;
        assert!(matches!(decide(at(1), &s), Stance::Ask { .. }));
        assert_eq!(
            decide(at(2), &s),
            Stance::Force {
                version: "1.1.0".into()
            }
        );
    }

    #[test]
    fn the_clock_only_restarts_when_the_version_changes() {
        let mut l = Ledger::default();
        l.observe("1.1.0", at(0));
        assert_eq!(l.first_seen_at, Some(at(0)));

        // The same number four polls later: the clock does not move, which is the
        // only thing that lets the deadline ever ring.
        l.observe("1.1.0", at(1));
        assert_eq!(l.first_seen_at, Some(at(0)));
        assert_eq!(l.last_check_at, Some(at(1)));

        // Versión distinta: reloj nuevo, y lo bajado deja de valer.
        l.staged = Some("1.1.0".into());
        l.observe("1.2.0", at(5));
        assert_eq!(l.first_seen_at, Some(at(5)));
        assert_eq!(l.staged, None);
    }

    #[test]
    fn observing_the_version_already_staged_keeps_the_download() {
        let mut l = Ledger::default();
        l.observe("1.1.0", at(0));
        l.staged = Some("1.1.0".into());
        l.observe("1.1.0", at(1));
        assert_eq!(l.staged, Some("1.1.0".into()));
    }

    #[test]
    fn applying_clears_the_clock() {
        let mut l = Ledger::default();
        l.observe("1.1.0", at(0));
        l.staged = Some("1.1.0".into());
        l.applied("1.1.0");
        assert_eq!(l.first_seen_at, None);
        assert_eq!(l.staged, None);
        assert_eq!(l.deadline(), None);
    }

    #[test]
    fn a_managed_install_is_never_ours_to_touch() {
        let m = Manifest {
            version: "1.0.0".into(),
            components: vec![Component::Core, Component::Desktop],
            delivery: Some(Delivery::Managed),
            core_dir: None,
            desktop_path: None,
            core_from_bundle: false,
        };
        assert!(!m.applies_unattended());
    }

    #[test]
    fn a_native_package_needs_someone_there() {
        for d in [Delivery::Deb, Delivery::Rpm, Delivery::Dmg] {
            let m = Manifest {
                version: "1.0.0".into(),
                components: vec![Component::Core, Component::Desktop],
                delivery: Some(d),
                core_dir: None,
                desktop_path: None,
                core_from_bundle: true,
            };
            assert!(!m.applies_unattended(), "{d:?} should need a human");
        }
    }

    #[test]
    fn appimage_and_nsis_apply_themselves() {
        for d in [Delivery::AppImage, Delivery::Nsis] {
            assert!(d.applies_unattended(), "{d:?} should be silent");
        }
        for d in [
            Delivery::Deb,
            Delivery::Rpm,
            Delivery::Dmg,
            Delivery::Managed,
        ] {
            assert!(!d.applies_unattended(), "{d:?} should not be silent");
        }
    }

    #[test]
    fn a_headless_core_in_a_user_dir_applies_itself() {
        let tmp = std::env::temp_dir().join("hoard-auto-test-core");
        std::fs::create_dir_all(&tmp).unwrap();
        let m = Manifest {
            version: "1.0.0".into(),
            components: vec![Component::Core],
            delivery: None,
            core_dir: Some(tmp.clone()),
            desktop_path: None,
            core_from_bundle: false,
        };
        assert!(m.applies_unattended());
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
