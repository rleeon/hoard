//! The OS's native notifications, sent **by the service** (ADR 0021 D.14.1).
//!
//! The desktop used to send them from its Svelte store, so they only existed with
//! the app open, which is exactly when they are least needed, since the window is
//! already saying the same thing. The engine lives here now and outlives the app
//! being closed, so the notification has to come from here too: it is the only way
//! to learn that a backup failed while you were playing full-screen, or that the
//! machine has not synced for an hour.
//!
//! ## The shape
//!
//! Three pieces, and only the last one knows about dbus:
//!
//! - [`Notice`]: *what* has to be said, derived from the event and the prefs by
//!   [`notice_for`]. A pure function, so the gate's tests (what gets notified and
//!   what does not) touch neither the bus nor the disk.
//! - [`text`]: how it is said, in the language the user picked in the app.
//! - [`Sink`]: where it goes out. `platform::sink()` returns this platform's, or
//!   the reason there is none.
//!
//! **Linux first, and the rest behind the same interface.** On Linux it goes out
//! over the session bus (`org.freedesktop.Notifications`, through `notify-rust`,
//! the same road the Tauri plugin takes in the desktop, so the notification looks
//! identical). On Windows and macOS `platform::sink()` returns the reason there is
//! no backend yet, the daemon says so in the log, and **the frontend keeps
//! notifying as it always has**: that is what
//! [`hoard_core::ipc::DaemonStatus::notifications`] announces, so the app neither
//! doubles the notification where we do notify nor goes quiet where we do not.

pub mod text;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use hoard_agent::agent::AgentEvent;
use hoard_agent::prefs::Prefs;
use hoard_agent::state::CliState;
use hoard_core::ipc::events::TooLargeKind;

use crate::notify::text::{Lang, Note};

/// How long the notification server gets to accept the notice. It is a call to a
/// local bus: taking longer than this means it is hung, and the event pump has
/// better things to do than wait for it.
const DELIVERY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// Can this build send native notifications? The IPC's `Status` announces it
/// ([`hoard_core::ipc::DaemonStatus::notifications`]) so the frontend goes quiet
/// where we speak. It is a platform constant, not a preference: the prefs decide
/// *whether* to notify, this decides *who* notifies.
pub const SUPPORTED: bool = platform::SUPPORTED;

/// Lo que hay que contarle al usuario. No sabe de idiomas ni de transporte.
#[derive(Debug, Clone, PartialEq)]
pub struct Notice {
    /// The game's name, when the event carried one. `None` means it has to be
    /// looked up in `state.json` (`BackupSuccess` only carries the `save_id`).
    pub name: Option<String>,
    /// Which save it is about, for resolving the name and for the logs.
    pub save_id: String,
    pub kind: Kind,
}

/// The notices the service sends. Deliberately the same four the desktop used to
/// send: what changed is **who** notifies, not what about.
#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    /// Backup uploaded. `bytes` is what travelled.
    BackupSaved {
        version: i64,
        bytes: u64,
    },
    BackupFailed {
        error: String,
        retrying: bool,
    },
    /// The save does not fit: not transient, not retried. `kind` says **who**
    /// rejected it (the plan, the user's own server, or a proxy in front of it),
    /// which is what decides the sentence and therefore where the user is sent to
    /// fix it.
    BackupTooLarge {
        kind: TooLargeKind,
        limit_bytes: u64,
        actual_bytes: u64,
    },
    /// The restore has failed N times in a row on the same version: this one does
    /// not fix itself.
    RestoreStuck {
        failures: u32,
    },
    /// There is a downloaded update **this machine cannot install on its own** (a
    /// native package that wants polkit, a `.dmg` that wants a hand).
    ///
    /// It does not come out of the event pump: [`crate::updater`] sends it, being
    /// the only one that knows. And it is the only update notification there is, on
    /// purpose: where it applies itself there is nothing to ask for, so notifying
    /// would be telling the user about a job that is already done.
    UpdateReady {
        version: String,
    },
}

/// What to notify about for this event, or nothing.
///
/// **Pure, and where the prefs gate lives.** The two that already existed rule just
/// as they did when the frontend notified: `notify_on_success` for the saved
/// backup and `notify_on_failure` for the three problem notices. No new preferences
/// are invented.
pub fn notice_for(event: &AgentEvent, prefs: &Prefs) -> Option<Notice> {
    match event {
        AgentEvent::BackupSuccess {
            save_id,
            version_num,
            total_bytes,
            already_landed,
            deliberate,
            ..
        } => {
            // `already_landed` is a no-op: the content was already up there and not
            // one byte travelled. Notifying about a backup that never happened is the
            // same lie as chiming while replaying the journal (ADR 0021 D.18).
            //
            // `deliberate` skips the preference: it is off by default because the
            // engine narrating every autosave is background noise, and that is not the
            // same as swallowing the answer to a button. Pressing "back up now" and
            // getting no signal at all leaves the user not knowing whether it happened
            // (Aug 2026).
            if (!prefs.notify_on_success && !*deliberate) || *already_landed {
                return None;
            }
            Some(Notice {
                name: None,
                save_id: save_id.clone(),
                kind: Kind::BackupSaved {
                    version: *version_num,
                    bytes: *total_bytes,
                },
            })
        }
        AgentEvent::BackupFailed {
            save_id,
            game_slug,
            error,
            will_retry,
        } => prefs.notify_on_failure.then(|| Notice {
            name: Some(game_slug.clone()),
            save_id: save_id.clone(),
            kind: Kind::BackupFailed {
                error: error.clone(),
                retrying: *will_retry,
            },
        }),
        AgentEvent::BackupTooLarge {
            save_id,
            game_slug,
            label,
            kind,
            limit_bytes,
            actual_bytes,
            ..
        } => prefs.notify_on_failure.then(|| Notice {
            name: Some(pick_name(label, game_slug)),
            save_id: save_id.clone(),
            kind: Kind::BackupTooLarge {
                kind: *kind,
                limit_bytes: *limit_bytes,
                actual_bytes: *actual_bytes,
            },
        }),
        AgentEvent::SaveAutoRestoreStuck {
            save_id,
            game_slug,
            failures,
            ..
        } => prefs.notify_on_failure.then(|| Notice {
            name: Some(game_slug.clone()),
            save_id: save_id.clone(),
            kind: Kind::RestoreStuck {
                failures: *failures,
            },
        }),
        _ => None,
    }
}

/// The label the user gave it beats the slug; when it is empty, the slug wins.
fn pick_name(label: &str, game_slug: &str) -> String {
    if label.trim().is_empty() {
        game_slug.to_string()
    } else {
        label.to_string()
    }
}

/// Where a notice goes out. One implementation per platform; in the tests, one
/// that only writes down what it is handed.
pub trait Sink: Send + Sync + 'static {
    fn deliver(&self, note: &Note) -> anyhow::Result<()>;
}

/// El que avisa. Vive en el daemon y lo alimenta la bomba de eventos.
pub struct Notifier {
    sink: Option<Arc<dyn Sink>>,
    /// We have complained once already that delivery fails. On a machine with no
    /// notification server (a NAS, a session with no desktop) **all** of them fail,
    /// and one WARN line per backup would be a log full of the same thing. The first
    /// goes out in full; the rest go to `debug`.
    complained: AtomicBool,
}

impl Notifier {
    /// El de esta plataforma. Si no hay backend lo dice **una vez, en voz alta**:
    /// un canal de avisos que no existe tiene que verse en el log, no deducirse
    /// del silencio (D.11).
    pub fn for_this_platform() -> Self {
        match platform::sink() {
            Ok(sink) => {
                tracing::info!(
                    transport = platform::TRANSPORT,
                    "hoardd: native notifications enabled"
                );
                Self::with_sink(sink)
            }
            Err(reason) => {
                tracing::info!(
                    reason = %reason,
                    "hoardd: native notifications aren't available; the app will send them while it's open"
                );
                Self {
                    sink: None,
                    complained: AtomicBool::new(false),
                }
            }
        }
    }

    pub fn with_sink(sink: Arc<dyn Sink>) -> Self {
        Self {
            sink: Some(sink),
            complained: AtomicBool::new(false),
        }
    }

    /// Looks at the event and notifies when it should. The prefs are read **fresh**
    /// for every notice: the user has just flipped the switch in Settings and the
    /// service does not restart for that. They are only read when there is a
    /// notifiable event, which is a handful a day.
    pub async fn consider(&self, event: &AgentEvent) {
        if self.sink.is_none() {
            return;
        }
        // Cheap and disk-free: it discards at a glance the events that never notify
        // (the vast majority) before touching `prefs.json`.
        if !notifiable(event) {
            return;
        }
        let prefs = load_prefs();
        let Some(notice) = notice_for(event, &prefs) else {
            return;
        };
        let name = notice
            .name
            .clone()
            .or_else(|| name_from_state(&notice.save_id))
            .unwrap_or_else(|| short_id(&notice.save_id));
        let note = text::render(
            &notice.kind,
            &name,
            Lang::for_user(prefs.language.as_deref()),
        );
        self.send(note).await;
    }

    /// Notifies about something that does not come from an engine event. Today only
    /// the updater ([`Kind::UpdateReady`]): there is no `save_id` to get a name from
    /// and no per-save preference to consult, so it does not go through
    /// [`Self::consider`].
    ///
    /// The prefs gate is still honoured, with the same one that governs the problem
    /// notices: whoever turned failure notifications off does not want us talking to
    /// them about anything that needs their intervention.
    pub async fn announce(&self, kind: Kind) {
        if self.sink.is_none() {
            return;
        }
        let prefs = load_prefs();
        if !prefs.notify_on_failure {
            return;
        }
        let note = text::render(&kind, "", Lang::for_user(prefs.language.as_deref()));
        self.send(note).await;
    }

    /// Entrega un aviso ya escrito. Separado de [`Self::consider`] para que el
    /// transporte se pueda probar sin depender de las prefs ni del `state.json`
    /// de quien ejecuta los tests.
    async fn send(&self, note: Note) {
        let Some(sink) = self.sink.clone() else {
            return;
        };
        // `deliver` talks to the bus and blocks, so it goes off the reactor. And with
        // a ceiling, because our caller is the event pump: a notification server that
        // does not answer must not jam the journal, with state persistence and the
        // push to the clients queued behind it.
        let delivery = tokio::task::spawn_blocking(move || sink.deliver(&note));
        match tokio::time::timeout(DELIVERY_TIMEOUT, delivery).await {
            Ok(Ok(Ok(()))) => {}
            Ok(Ok(Err(err))) => self.complain(&format!("{err:#}")),
            Ok(Err(err)) => self.complain(&format!("{err}")),
            Err(_) => self.complain(&format!(
                "the notification server didn't answer in {}s",
                DELIVERY_TIMEOUT.as_secs()
            )),
        }
    }

    fn complain(&self, error: &str) {
        if self.complained.swap(true, Ordering::Relaxed) {
            tracing::debug!(error = %error, "hoardd: couldn't deliver a native notification");
        } else {
            tracing::warn!(
                error = %error,
                "hoardd: couldn't deliver a native notification (further failures log at debug)"
            );
        }
    }
}

/// Can this event end up as a notice? It only looks at the variant, so it does not
/// cost even one `read`. The real gate is [`notice_for`], which needs the prefs.
fn notifiable(event: &AgentEvent) -> bool {
    matches!(
        event,
        AgentEvent::BackupSuccess { .. }
            | AgentEvent::BackupFailed { .. }
            | AgentEvent::BackupTooLarge { .. }
            | AgentEvent::SaveAutoRestoreStuck { .. }
    )
}

fn load_prefs() -> Prefs {
    match Prefs::load_default() {
        Ok((prefs, _)) => prefs,
        Err(err) => {
            // Sin prefs legibles mandan los defaults, que es lo que el motor ya
            // hace con el resto de la config (`engine_config`).
            tracing::debug!(error = %err, "hoardd: couldn't read prefs for a notification");
            Prefs::default()
        }
    }
}

/// El nombre del juego para un `save_id`. `BackupSuccess` no lo trae, y un aviso
/// que diga "a1b2c3d4" no le sirve a nadie.
fn name_from_state(save_id: &str) -> Option<String> {
    let (state, _path) = CliState::load_default().ok()?;
    let entry = state.saves.get(save_id)?;
    Some(pick_name(&entry.label, &entry.game_slug))
}

/// Last resort: the save exists but is not in `state.json` (a cloud save backed up
/// before it was adopted).
fn short_id(save_id: &str) -> String {
    save_id.chars().take(8).collect()
}

// ---- Linux: the session bus (org.freedesktop.Notifications)

#[cfg(target_os = "linux")]
mod platform {
    use std::sync::Arc;

    use super::{Note, Sink};

    pub const SUPPORTED: bool = true;
    pub const TRANSPORT: &str = "D-Bus (org.freedesktop.Notifications)";

    /// The icon the notification server looks for in the theme: the one the
    /// `.deb`/`.rpm` install (`/usr/share/icons/hicolor/*/apps/`). When it is missing
    /// (running from `target/`), the server paints the generic one.
    const ICON: &str = "hoard-desktop";

    /// The name that shows in the notice. The product's, not the binary's: the user
    /// is being told by Hoard, not by a service they have never heard of.
    const APP_NAME: &str = "Hoard";

    pub fn sink() -> Result<Arc<dyn Sink>, String> {
        // No check that a notification server exists: in a desktop session it is
        // dbus-activated on demand, so asking now would only give a "no" that stops
        // being true a second later. If there really is none, delivery fails and it is
        // said (once).
        Ok(Arc::new(Dbus))
    }

    struct Dbus;

    impl Sink for Dbus {
        fn deliver(&self, note: &Note) -> anyhow::Result<()> {
            notify_rust::Notification::new()
                .appname(APP_NAME)
                .icon(ICON)
                .summary(&note.title)
                .body(&note.body)
                .show()?;
            Ok(())
        }
    }
}

// ---- Windows and macOS: pending, behind the same interface

#[cfg(not(target_os = "linux"))]
mod platform {
    use std::sync::Arc;

    use super::Sink;

    pub const SUPPORTED: bool = false;
    pub const TRANSPORT: &str = "none";

    /// There is no backend here yet, and the daemon says so instead of swallowing
    /// it: while `SUPPORTED` is `false`, `DaemonStatus::notifications` travels as
    /// `false` and **the frontend keeps sending the notice itself** (a Windows toast,
    /// macOS's notification centre). When the backend lands, returning a `Sink` here
    /// is enough: the app goes quiet on its own because it reads the flag, not a list
    /// of platforms.
    pub fn sink() -> Result<Arc<dyn Sink>, String> {
        Err(format!(
            "the Hoard service doesn't send native notifications on {} yet (ADR 0021 D.19)",
            std::env::consts::OS
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hoard_core::ipc::events::TooLargeKind;
    use std::sync::Mutex;

    fn prefs(success: bool, failure: bool) -> Prefs {
        Prefs {
            notify_on_success: success,
            notify_on_failure: failure,
            ..Prefs::default()
        }
    }

    fn success(already_landed: bool) -> AgentEvent {
        success_with(already_landed, false)
    }

    fn success_with(already_landed: bool, deliberate: bool) -> AgentEvent {
        AgentEvent::BackupSuccess {
            save_id: "abcdef0123456789".into(),
            version_num: 12,
            total_bytes: 2048,
            set_hash: None,
            already_landed,
            deliberate,
        }
    }

    fn failure() -> AgentEvent {
        AgentEvent::BackupFailed {
            save_id: "s1".into(),
            game_slug: "factorio".into(),
            error: "the server said no".into(),
            will_retry: true,
        }
    }

    #[test]
    fn success_is_gated_by_notify_on_success() {
        assert!(notice_for(&success(false), &prefs(false, true)).is_none());
        let notice = notice_for(&success(false), &prefs(true, false)).expect("should notify");
        assert_eq!(
            notice.kind,
            Kind::BackupSaved {
                version: 12,
                bytes: 2048
            }
        );
    }

    #[test]
    fn failures_are_gated_by_notify_on_failure() {
        assert!(notice_for(&failure(), &prefs(true, false)).is_none());
        let notice = notice_for(&failure(), &prefs(false, true)).expect("should notify");
        assert_eq!(notice.name.as_deref(), Some("factorio"));
    }

    /// A backup that uploaded nothing is not announced: the content was already up
    /// there (ADR 0021 D.18). The state does advance, the notice does not go out.
    #[test]
    fn a_backup_that_already_landed_is_not_announced() {
        assert!(notice_for(&success(true), &prefs(true, true)).is_none());
    }

    /// Los eventos de reposo/ritmo no son noticia: el throttle se reintenta solo
    /// y el juego que arranca ya se ve en la app.
    #[test]
    fn routine_events_never_notify() {
        let quiet = [
            AgentEvent::GameStarted {
                save_id: "s1".into(),
                game_slug: "factorio".into(),
            },
            AgentEvent::BackupStarted {
                save_id: "s1".into(),
                game_slug: "factorio".into(),
                label: "Partida".into(),
            },
            AgentEvent::BackupThrottled {
                save_id: "s1".into(),
                game_slug: "factorio".into(),
                label: "Partida".into(),
                retry_after_secs: 30,
            },
            AgentEvent::SaveAutoRestoreFailed {
                save_id: "s1".into(),
                game_slug: "factorio".into(),
                error: "network".into(),
            },
        ];
        for event in quiet {
            assert!(!notifiable(&event), "{event:?} shouldn't be notifiable");
            assert!(notice_for(&event, &prefs(true, true)).is_none());
        }
    }

    /// With `notify_on_success` off (the default), an automatic backup stays quiet
    /// and one the user asked for still notifies. The preference exists so the engine
    /// does not narrate every autosave, not to leave whoever pressed a button without
    /// an answer.
    #[test]
    fn a_copy_the_user_asked_for_confirms_even_with_success_notices_off() {
        let prefs = Prefs {
            notify_on_success: false,
            notify_on_failure: true,
            ..Prefs::default()
        };
        assert!(notice_for(&success_with(false, false), &prefs).is_none());
        assert!(notice_for(&success_with(false, true), &prefs).is_some());
        // Unless nothing happened: the content was already the server's head, so
        // there is no backup to notify about.
        assert!(notice_for(&success_with(true, true), &prefs).is_none());
    }

    /// `notifiable` is the shortcut that avoids reading `prefs.json` on every tick,
    /// so it has to cover **everything** `notice_for` knows how to notify about: if
    /// they drift apart, the notice disappears with nobody noticing.
    #[test]
    fn the_cheap_filter_matches_the_real_gate() {
        let notifying = [
            success(false),
            failure(),
            AgentEvent::BackupTooLarge {
                save_id: "s1".into(),
                game_slug: "factorio".into(),
                label: String::new(),
                kind: TooLargeKind::PlanCap,
                plan: "free".into(),
                limit_bytes: 100,
                actual_bytes: 200,
                received_bytes: 0,
            },
            AgentEvent::SaveAutoRestoreStuck {
                save_id: "s1".into(),
                game_slug: "factorio".into(),
                failures: 3,
                error: "sha mismatch".into(),
            },
        ];
        for event in notifying {
            assert!(notifiable(&event), "{event:?} should be notifiable");
            assert!(notice_for(&event, &prefs(true, true)).is_some());
        }
    }

    /// The user's label beats the slug, and an empty label does not leave the notice
    /// without a name.
    #[test]
    fn the_label_wins_but_never_leaves_it_blank() {
        let with_label = AgentEvent::BackupTooLarge {
            save_id: "s1".into(),
            game_slug: "factorio".into(),
            label: "Mundo nuevo".into(),
            kind: TooLargeKind::PlanCap,
            plan: "free".into(),
            limit_bytes: 100,
            actual_bytes: 200,
            received_bytes: 0,
        };
        assert_eq!(
            notice_for(&with_label, &prefs(false, true))
                .unwrap()
                .name
                .as_deref(),
            Some("Mundo nuevo")
        );
    }

    #[test]
    fn an_unknown_save_falls_back_to_a_short_id() {
        assert_eq!(short_id("abcdef0123456789"), "abcdef01");
    }

    struct Recorder(Mutex<Vec<Note>>);

    impl Sink for Recorder {
        fn deliver(&self, note: &Note) -> anyhow::Result<()> {
            self.0.lock().unwrap().push(note.clone());
            Ok(())
        }
    }

    /// Sin backend de plataforma el daemon no avisa y **no se cae**: es el
    /// estado de Windows/macOS hasta que aterricen los suyos.
    #[tokio::test]
    async fn a_notifier_without_a_sink_is_quiet() {
        let notifier = Notifier {
            sink: None,
            complained: AtomicBool::new(false),
        };
        notifier.consider(&failure()).await;
    }

    /// **Real** smoke against the session bus. It does not run under `cargo test`:
    /// neither CI nor a session with no desktop has a notification server, so it
    /// would fail on the environment and not on the code. It is the manual check that
    /// the notice really goes out and shows up where it should:
    ///
    /// ```text
    /// cargo test -p hoardd -- --ignored --nocapture the_session_bus
    /// ```
    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "needs a session bus with a notification server"]
    fn the_session_bus_takes_a_real_notification() {
        let sink = platform::sink().expect("linux always has a sink");
        let note = text::render(
            &Kind::BackupSaved {
                version: 42,
                bytes: 3 * 1024 * 1024,
            },
            "Factorio",
            Lang::for_user(None),
        );
        sink.deliver(&note).expect("the session bus took it");
    }

    /// What is written reaches the transport verbatim. `consider` is no good for
    /// this: it would read the prefs and the `state.json` of whoever runs the tests,
    /// so its result would depend on the machine.
    #[tokio::test]
    async fn what_gets_written_is_what_gets_delivered() {
        let recorder = Arc::new(Recorder(Mutex::new(Vec::new())));
        let notifier = Notifier::with_sink(recorder.clone());
        let note = text::render(
            &Kind::RestoreStuck { failures: 3 },
            "Factorio",
            text::Lang::Es,
        );
        notifier.send(note.clone()).await;
        assert_eq!(recorder.0.lock().unwrap().as_slice(), &[note]);
    }

    /// An event the prefs do not let through never reaches the transport. It is
    /// tested through the pure gate ([`notice_for`]) because that is what decides;
    /// `consider` only adds the disk to it.
    #[tokio::test]
    async fn a_silenced_event_never_reaches_the_sink() {
        let recorder = Arc::new(Recorder(Mutex::new(Vec::new())));
        let notifier = Notifier::with_sink(recorder.clone());
        assert!(notice_for(&failure(), &prefs(true, false)).is_none());
        notifier
            .consider(&AgentEvent::GameStopped {
                save_id: "s1".into(),
                game_slug: "factorio".into(),
            })
            .await;
        assert!(recorder.0.lock().unwrap().is_empty());
    }
}
