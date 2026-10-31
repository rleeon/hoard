//! Every notification Hoard shows, decided and sent **by the service** (ADR 0021
//! D.14.1).
//!
//! The desktop used to send them from its Svelte store, so they only existed with
//! the app open, which is exactly when they are least needed, since the window is
//! already saying the same thing. The engine lives here now and outlives the app
//! being closed, so the notification has to come from here too: it is the only way
//! to learn that a backup failed while you were playing full-screen, or that the
//! machine has not synced for an hour.
//!
//! The Decky plugin and the app each used to keep a list of their own on top of
//! that, three lists that had drifted into telling different stories about the same
//! backup. No client decides anything now, and the switches live here too, so a
//! change from the app, the CLI or the plugin is the same change.
//!
//! ## The shape
//!
//! - [`Notice`]: *what* has to be said, derived from the event and the prefs by
//!   [`notice_for`]. A pure function, so the gate's tests (what gets notified and
//!   what does not) touch neither the bus nor the disk.
//! - [`text`]: how it is said, in the language the user picked in the app.
//! - [`way_out`]: which one road each notice takes. Never two: that is what stops
//!   the same notice showing up twice.
//! - [`Sink`]: the OS's road. `platform::sink()` returns this platform's, or the
//!   reason there is none.
//!
//! On Linux it goes out over the session bus (`org.freedesktop.Notifications`,
//! through `notify-rust`), on Windows as a toast. gamescope shows neither, so in
//! Game Mode the Decky plugin claims the notices ([`Notifier::claim`]) and Steam
//! shows them. macOS has no road of its own yet: a bare binary has no bundle to
//! notify as, so there the notice goes to the app, while it is open.

pub mod text;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Context;
use hoard_agent::agent::AgentEvent;
use hoard_agent::prefs::Prefs;
use hoard_agent::state::CliState;
use hoard_core::ipc::events::TooLargeKind;
use hoard_core::ipc::{NoticeLevel, NoticeNote, NotificationSwitch, NotificationSwitches};
use tokio::sync::broadcast;

use crate::notify::text::{Lang, Note};

/// How long the notification server gets to accept the notice. It is a call to a
/// local bus: taking longer than this means it is hung, and the event pump has
/// better things to do than wait for it.
const DELIVERY_TIMEOUT: Duration = Duration::from_secs(5);

/// How long the same problem on the same save stays quiet after being said. A
/// parked upload comes round again every hour, an unreadable file on every copy,
/// and an empty folder on every sweep: the first notice is news, the rest of the
/// afternoon is the same notice.
const PROBLEM_QUIET: Duration = Duration::from_secs(6 * 60 * 60);

/// Notices waiting for a connection to pick them up. There are a handful a day.
const NOTICE_CHANNEL: usize = 16;

/// The label a save has until somebody renames it. It says nothing about the game.
const DEFAULT_LABEL: &str = "main";

/// Can this build reach the OS's notifications? It does not say *whether* to notify,
/// which is what the switches are for, only by which road.
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

/// The notices the service sends. Each one answers to one switch in Settings (see
/// [`notice_for`]).
#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    /// Backup uploaded. `bytes` is what travelled.
    BackupSaved {
        version: i64,
        bytes: u64,
    },
    /// A cloud version landed on this machine. `bytes` is what was written.
    RestoreDone {
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
    /// The account is out of space. Nothing about this save is wrong.
    QuotaFull,
    /// The copy went up without its oldest files: the plan's per-save cap.
    BackupTrimmed {
        omitted: u64,
    },
    /// Files whose bytes would not be read. With `uploaded` false, not one could,
    /// and there is no copy at all.
    FilesUnreadable {
        count: u64,
        uploaded: bool,
    },
    /// It has stopped retrying and needs a person.
    NeedsAttention {
        conflicts: u32,
    },
    /// Never backed up and empty: a mis-tracked folder, not a game between saves.
    WrongFolder,
    /// The restore has failed N times in a row on the same version: this one does
    /// not fix itself.
    RestoreStuck {
        failures: u32,
    },
    /// A restore moved local files aside before writing over them.
    ConflictsKept {
        count: u64,
    },
    GameStarted,
    GameStopped,
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

impl Kind {
    fn level(&self) -> NoticeLevel {
        match self {
            Kind::BackupSaved { .. } | Kind::RestoreDone { .. } => NoticeLevel::Ok,
            Kind::GameStarted | Kind::GameStopped | Kind::UpdateReady { .. } => NoticeLevel::Info,
            Kind::BackupTrimmed { .. }
            | Kind::FilesUnreadable { uploaded: true, .. }
            | Kind::WrongFolder
            | Kind::ConflictsKept { .. } => NoticeLevel::Warn,
            Kind::BackupFailed { .. }
            | Kind::BackupTooLarge { .. }
            | Kind::QuotaFull
            | Kind::FilesUnreadable { .. }
            | Kind::NeedsAttention { .. }
            | Kind::RestoreStuck { .. } => NoticeLevel::Error,
        }
    }
}

/// What to notify about for this event, or nothing.
///
/// **Pure, and the only place this is decided.** One switch per kind of news:
/// `notify_on_success` for the saved backup, `notify_on_failure` for everything
/// wrong with a backup, `notify_on_restore_success` / `notify_on_restore_failure`
/// for the downloads and `notify_on_game_activity` for games opening and closing. A
/// lone `SaveAutoRestoreFailed` never notifies: the sweep retries it, and only the
/// stuck one (several in a row on the same version) is news.
pub fn notice_for(event: &AgentEvent, prefs: &Prefs) -> Option<Notice> {
    let (allowed, name, save_id, kind) = match event {
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
            (
                (prefs.notify_on_success || *deliberate) && !*already_landed,
                None,
                save_id,
                Kind::BackupSaved {
                    version: *version_num,
                    bytes: *total_bytes,
                },
            )
        }
        AgentEvent::BackupFailed {
            save_id,
            game_slug,
            error,
            will_retry,
        } => (
            prefs.notify_on_failure,
            Some(game_name(game_slug, "")),
            save_id,
            Kind::BackupFailed {
                error: error.clone(),
                retrying: *will_retry,
            },
        ),
        AgentEvent::BackupTooLarge {
            save_id,
            game_slug,
            label,
            kind,
            limit_bytes,
            actual_bytes,
            ..
        } => (
            prefs.notify_on_failure,
            Some(game_name(game_slug, label)),
            save_id,
            Kind::BackupTooLarge {
                kind: *kind,
                limit_bytes: *limit_bytes,
                actual_bytes: *actual_bytes,
            },
        ),
        AgentEvent::BackupQuotaFull {
            save_id,
            game_slug,
            label,
            ..
        } => (
            prefs.notify_on_failure,
            Some(game_name(game_slug, label)),
            save_id,
            Kind::QuotaFull,
        ),
        AgentEvent::BackupTrimmed {
            save_id,
            game_slug,
            label,
            omitted_files,
            ..
        } => (
            prefs.notify_on_failure,
            Some(game_name(game_slug, label)),
            save_id,
            Kind::BackupTrimmed {
                omitted: *omitted_files,
            },
        ),
        AgentEvent::BackupFilesUnreadable {
            save_id,
            game_slug,
            label,
            count,
            uploaded,
            ..
        } => (
            prefs.notify_on_failure,
            Some(game_name(game_slug, label)),
            save_id,
            Kind::FilesUnreadable {
                count: *count,
                uploaded: *uploaded,
            },
        ),
        AgentEvent::BackupNeedsAttention {
            save_id,
            game_slug,
            label,
            conflicts,
            ..
        } => (
            prefs.notify_on_failure,
            Some(game_name(game_slug, label)),
            save_id,
            Kind::NeedsAttention {
                conflicts: *conflicts,
            },
        ),
        AgentEvent::BackupSkippedEmpty {
            save_id,
            game_slug,
            likely_wrong_path,
        } => (
            prefs.notify_on_failure && *likely_wrong_path,
            Some(game_name(game_slug, "")),
            save_id,
            Kind::WrongFolder,
        ),
        AgentEvent::SaveAutoRestored {
            save_id,
            game_slug,
            version_num,
            bytes_extracted,
            ..
        } => (
            prefs.notify_on_restore_success,
            Some(game_name(game_slug, "")),
            save_id,
            Kind::RestoreDone {
                version: *version_num,
                bytes: *bytes_extracted,
            },
        ),
        AgentEvent::SaveAutoRestoreStuck {
            save_id,
            game_slug,
            failures,
            ..
        } => (
            prefs.notify_on_restore_failure,
            Some(game_name(game_slug, "")),
            save_id,
            Kind::RestoreStuck {
                failures: *failures,
            },
        ),
        AgentEvent::SaveConflictsBackedUp {
            save_id,
            game_slug,
            count,
            ..
        } => (
            prefs.notify_on_restore_failure,
            Some(game_name(game_slug, "")),
            save_id,
            Kind::ConflictsKept { count: *count },
        ),
        AgentEvent::GameStarted { save_id, game_slug } => (
            prefs.notify_on_game_activity,
            Some(game_name(game_slug, "")),
            save_id,
            Kind::GameStarted,
        ),
        AgentEvent::GameStopped { save_id, game_slug } => (
            prefs.notify_on_game_activity,
            Some(game_name(game_slug, "")),
            save_id,
            Kind::GameStopped,
        ),
        _ => return None,
    };
    allowed.then(|| Notice {
        name,
        save_id: save_id.clone(),
        kind,
    })
}

/// What to call a save in a notice: the game, and the label only when somebody
/// chose one. Every save is `main` until renamed, so a label on its own read as
/// "main · v12" and told the user nothing.
fn game_name(game_slug: &str, label: &str) -> String {
    // Slugs from the catalogue are `hollow-knight`. A custom game can already be
    // spelt the way its owner wants it.
    let game = if game_slug == game_slug.to_lowercase() && !game_slug.contains(' ') {
        game_slug
            .split('-')
            .filter(|word| !word.is_empty())
            .map(|word| {
                let mut letters = word.chars();
                match letters.next() {
                    Some(first) => first.to_uppercase().chain(letters).collect::<String>(),
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        game_slug.to_string()
    };
    let label = label.trim();
    if label.is_empty() || label == DEFAULT_LABEL || label.eq_ignore_ascii_case(&game) {
        game
    } else {
        format!("{game} ({label})")
    }
}

/// What makes two notices the same problem, or `None` for one that is news every
/// time. Keyed on the save, so a second game failing is said; the full account is
/// the exception, because it is one fact that twenty games discover one by one.
fn quiet_key(notice: &Notice) -> Option<String> {
    let problem = match notice.kind {
        Kind::BackupFailed { .. } => "failed",
        Kind::BackupTooLarge { .. } => "too_large",
        Kind::BackupTrimmed { .. } => "trimmed",
        Kind::FilesUnreadable { .. } => "unreadable",
        Kind::WrongFolder => "wrong_folder",
        Kind::QuotaFull => return Some("quota_full".to_string()),
        _ => return None,
    };
    Some(format!("{problem}:{}", notice.save_id))
}

/// Which problems have been said recently.
#[derive(Default)]
struct Quiet(Mutex<HashMap<String, Instant>>);

impl Quiet {
    /// True when this is news. A repeat does not extend the window, or something
    /// repeating forever would never be said again.
    fn admit(&self, key: String, now: Instant) -> bool {
        let mut said = self.0.lock().unwrap_or_else(|e| e.into_inner());
        said.retain(|_, at| now.duration_since(*at) < PROBLEM_QUIET);
        if said.contains_key(&key) {
            return false;
        }
        said.insert(key, now);
        true
    }

    /// A backup landed, so what stopped it is over and the next failure is new.
    /// Not the trimmed or unreadable ones: those arrive right after a success, and
    /// say that the copy is still incomplete.
    fn backup_landed(&self, save_id: &str) {
        let mut said = self.0.lock().unwrap_or_else(|e| e.into_inner());
        said.remove(&format!("failed:{save_id}"));
        said.remove(&format!("too_large:{save_id}"));
        said.remove("quota_full");
    }
}

/// Where a notice goes out. One implementation per platform; in the tests, one
/// that only writes down what it is handed.
pub trait Sink: Send + Sync + 'static {
    fn deliver(&self, note: &Note) -> anyhow::Result<()>;
}

/// The one road a notice takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Way {
    /// A client that said it is the screen right now. It goes first because it only
    /// claims where the OS's road ends nowhere.
    Claimant,
    System,
    /// No road of our own on this platform, so the app paints it, if it is open.
    App,
}

pub fn way_out(claimed: bool, has_sink: bool) -> Way {
    match (claimed, has_sink) {
        (true, _) => Way::Claimant,
        (false, true) => Way::System,
        (false, false) => Way::App,
    }
}

/// A notice on its way to a connection. All of them hear it; only the one it is
/// for forwards it.
#[derive(Debug, Clone)]
pub struct Delivery {
    pub to: Way,
    pub note: NoticeNote,
}

/// A client holding the notices. Dropping it, which is what a closed connection
/// does, gives them back to the OS: a plugin that dies must not swallow them.
pub struct Claim(Arc<AtomicUsize>);

impl Drop for Claim {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}

/// El que avisa. Vive en el daemon y lo alimenta la bomba de eventos.
pub struct Notifier {
    sink: Option<Arc<dyn Sink>>,
    /// We have complained once already that delivery fails. On a machine with no
    /// notification server (a NAS, a session with no desktop) **all** of them fail,
    /// and one WARN line per backup would be a log full of the same thing. The first
    /// goes out in full; the rest go to `debug`.
    complained: AtomicBool,
    quiet: Quiet,
    claims: Arc<AtomicUsize>,
    notices: broadcast::Sender<Delivery>,
    switches: broadcast::Sender<NotificationSwitches>,
    /// Two clients flipping switches at once are two read-then-write cycles over
    /// one file.
    writing: tokio::sync::Mutex<()>,
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
                    "hoardd: native notifications aren't available; the app will show them while it's open"
                );
                Self::built(None)
            }
        }
    }

    pub fn with_sink(sink: Arc<dyn Sink>) -> Self {
        Self::built(Some(sink))
    }

    fn built(sink: Option<Arc<dyn Sink>>) -> Self {
        Self {
            sink,
            complained: AtomicBool::new(false),
            quiet: Quiet::default(),
            claims: Arc::new(AtomicUsize::new(0)),
            notices: broadcast::channel(NOTICE_CHANNEL).0,
            switches: broadcast::channel(NOTICE_CHANNEL).0,
            writing: tokio::sync::Mutex::new(()),
        }
    }

    /// Looks at the event and notifies when it should. The prefs are read **fresh**
    /// for every notice: the user has just flipped the switch in Settings and the
    /// service does not restart for that. They are only read when there is a
    /// notifiable event, which is a handful a day.
    pub async fn consider(&self, event: &AgentEvent) {
        if let AgentEvent::BackupSuccess {
            save_id,
            already_landed: false,
            ..
        } = event
        {
            self.quiet.backup_landed(save_id);
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
        if let Some(key) = quiet_key(&notice) {
            if !self.quiet.admit(key, Instant::now()) {
                return;
            }
        }
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
        self.send(note, notice.kind.level()).await;
    }

    /// Notifies about something that does not come from an engine event. Today only
    /// the updater ([`Kind::UpdateReady`]): there is no `save_id` to get a name from,
    /// so it does not go through [`Self::consider`].
    pub async fn announce(&self, kind: Kind) {
        let prefs = load_prefs();
        if !prefs.notify_on_update {
            return;
        }
        let note = text::render(&kind, "", Lang::for_user(prefs.language.as_deref()));
        self.send(note, kind.level()).await;
    }

    /// Hands the notices to a client until the returned [`Claim`] drops.
    pub fn claim(&self) -> Claim {
        self.claims.fetch_add(1, Ordering::Relaxed);
        Claim(self.claims.clone())
    }

    pub fn subscribe_notices(&self) -> broadcast::Receiver<Delivery> {
        self.notices.subscribe()
    }

    pub fn subscribe_switches(&self) -> broadcast::Receiver<NotificationSwitches> {
        self.switches.subscribe()
    }

    pub fn switches(&self) -> NotificationSwitches {
        load_prefs().notification_switches()
    }

    /// Flips one switch and tells every client. The only write of these fields in
    /// the whole system: the window's own save keeps them as they are on disk.
    pub async fn set_switch(
        &self,
        switch: NotificationSwitch,
        enabled: bool,
    ) -> anyhow::Result<NotificationSwitches> {
        let switches = {
            let _writing = self.writing.lock().await;
            write_switch(switch, enabled)?
        };
        tracing::info!(?switch, enabled, "hoardd: notification switch changed");
        let _ = self.switches.send(switches);
        Ok(switches)
    }

    /// Entrega un aviso ya escrito. Separado de [`Self::consider`] para que el
    /// transporte se pueda probar sin depender de las prefs ni del `state.json`
    /// de quien ejecuta los tests.
    async fn send(&self, note: Note, level: NoticeLevel) {
        let way = way_out(self.claims.load(Ordering::Relaxed) > 0, self.sink.is_some());
        let Way::System = way else {
            let note = NoticeNote {
                title: note.title,
                body: note.body,
                level,
            };
            // No receiver is not an error: with no app open there is nobody to show
            // it to, and the journal still has the event.
            let _ = self.notices.send(Delivery { to: way, note });
            return;
        };
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

fn write_switch(switch: NotificationSwitch, enabled: bool) -> anyhow::Result<NotificationSwitches> {
    let (mut prefs, path) = Prefs::load_default().context("reading the preferences")?;
    prefs.set_notification_switch(switch, enabled);
    prefs.save(&path).context("saving the preferences")?;
    Ok(prefs.notification_switches())
}

/// Can this event end up as a notice? It only looks at the variant, so it does not
/// cost even one `read`. The real gate is [`notice_for`], which needs the prefs.
fn notifiable(event: &AgentEvent) -> bool {
    matches!(
        event,
        AgentEvent::BackupSuccess { .. }
            | AgentEvent::BackupFailed { .. }
            | AgentEvent::BackupTooLarge { .. }
            | AgentEvent::BackupQuotaFull { .. }
            | AgentEvent::BackupTrimmed { .. }
            | AgentEvent::BackupFilesUnreadable { .. }
            | AgentEvent::BackupNeedsAttention { .. }
            | AgentEvent::BackupSkippedEmpty {
                likely_wrong_path: true,
                ..
            }
            | AgentEvent::SaveAutoRestored { .. }
            | AgentEvent::SaveAutoRestoreStuck { .. }
            | AgentEvent::SaveConflictsBackedUp { .. }
            | AgentEvent::GameStarted { .. }
            | AgentEvent::GameStopped { .. }
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
    Some(game_name(&entry.game_slug, &entry.label))
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

// ---- Windows: a toast

#[cfg(windows)]
mod platform {
    use std::sync::{Arc, Once};

    use super::{Note, Sink};

    pub const SUPPORTED: bool = true;
    pub const TRANSPORT: &str = "Windows toast";

    /// Windows files a toast under an application id. It is the desktop's identifier,
    /// so the app and the service are one sender in the notification settings.
    const APP_ID: &str = "dev.hoard.desktop";
    const ICON: &[u8] = include_bytes!("../../../hoard-desktop/icons/128x128.png");

    pub fn sink() -> Result<Arc<dyn Sink>, String> {
        Ok(Arc::new(Toast))
    }

    struct Toast;

    impl Sink for Toast {
        fn deliver(&self, note: &Note) -> anyhow::Result<()> {
            static REGISTER: Once = Once::new();
            REGISTER.call_once(|| {
                if let Err(err) = register() {
                    tracing::warn!(error = %format!("{err:#}"), "hoardd: couldn't register the toast sender");
                }
            });
            notify_rust::Notification::new()
                .app_id(APP_ID)
                .summary(&note.title)
                .body(&note.body)
                .show()?;
            Ok(())
        }
    }

    /// A toast only shows a name and an icon for an id Windows knows about, and the
    /// shortcut the setup creates does not carry one. This key is the documented way
    /// for an unpackaged app, needs no rights beyond the user's own and can be
    /// rewritten on every start of the service.
    fn register() -> anyhow::Result<()> {
        use anyhow::Context;
        use winreg::enums::HKEY_CURRENT_USER;

        let dir = hoard_agent::prefs::Prefs::default_path()?
            .parent()
            .context("the prefs path has no parent")?
            .to_path_buf();
        let icon = dir.join("notification-icon.png");
        std::fs::write(&icon, ICON).context("writing the notification icon")?;
        let (key, _) = winreg::RegKey::predef(HKEY_CURRENT_USER)
            .create_subkey(format!(r"Software\Classes\AppUserModelId\{APP_ID}"))
            .context("creating the sender's key")?;
        key.set_value("DisplayName", &"Hoard")?;
        key.set_value("IconUri", &icon.display().to_string())?;
        Ok(())
    }
}

// ---- macOS and mobile: no road of their own, the app paints it

#[cfg(not(any(target_os = "linux", windows)))]
mod platform {
    use std::sync::Arc;

    use super::Sink;

    pub const SUPPORTED: bool = false;
    pub const TRANSPORT: &str = "none";

    /// Said out loud rather than swallowed. Returning a `Sink` here is all a backend
    /// needs: every notice would start taking it, and the app would stop being asked.
    pub fn sink() -> Result<Arc<dyn Sink>, String> {
        Err(format!(
            "the Hoard service doesn't reach the OS's notifications on {} yet (ADR 0021 D.19)",
            std::env::consts::OS
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hoard_core::ipc::events::TooLargeKind;
    use std::sync::Mutex;

    /// The backup and download switches move together here; the test that keeps
    /// them apart builds its own.
    fn prefs(success: bool, failure: bool) -> Prefs {
        Prefs {
            notify_on_success: success,
            notify_on_failure: failure,
            notify_on_restore_success: success,
            notify_on_restore_failure: failure,
            notify_on_game_activity: success,
            notify_on_update: failure,
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

    fn restored() -> AgentEvent {
        AgentEvent::SaveAutoRestored {
            save_id: "s1".into(),
            game_slug: "factorio".into(),
            version_num: 7,
            files_extracted: 3,
            bytes_extracted: 4096,
        }
    }

    fn quota_full() -> AgentEvent {
        AgentEvent::BackupQuotaFull {
            save_id: "s1".into(),
            game_slug: "factorio".into(),
            label: "main".into(),
            plan: "free".into(),
            used_bytes: 100,
            limit_bytes: 100,
        }
    }

    fn stuck() -> AgentEvent {
        AgentEvent::SaveAutoRestoreStuck {
            save_id: "s1".into(),
            game_slug: "factorio".into(),
            failures: 3,
            error: "sha mismatch".into(),
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
        assert_eq!(notice.name.as_deref(), Some("Factorio"));
    }

    /// Each download notice answers to its own switch, not to the backup's: turning
    /// backup failures off must not silence a restore that keeps failing, and the
    /// other way round.
    #[test]
    fn downloads_have_their_own_switches() {
        let only = |restore_success: bool, restore_failure: bool| Prefs {
            notify_on_success: !restore_success,
            notify_on_failure: !restore_failure,
            notify_on_restore_success: restore_success,
            notify_on_restore_failure: restore_failure,
            ..Prefs::default()
        };
        assert!(notice_for(&restored(), &only(false, true)).is_none());
        assert_eq!(
            notice_for(&restored(), &only(true, false)).unwrap().kind,
            Kind::RestoreDone {
                version: 7,
                bytes: 4096
            }
        );
        assert!(notice_for(&stuck(), &only(true, false)).is_none());
        assert_eq!(
            notice_for(&stuck(), &only(false, true)).unwrap().kind,
            Kind::RestoreStuck { failures: 3 }
        );
        // And the backup notices ignore the download switches.
        assert!(notice_for(&failure(), &only(true, true)).is_none());
        assert!(notice_for(&success(false), &only(true, true)).is_none());
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
            AgentEvent::BackupSkippedEmpty {
                save_id: "s1".into(),
                game_slug: "factorio".into(),
                likely_wrong_path: false,
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
            restored(),
            stuck(),
            quota_full(),
            AgentEvent::BackupTrimmed {
                save_id: "s1".into(),
                game_slug: "factorio".into(),
                label: "main".into(),
                kept_files: 3,
                omitted_files: 2,
                omitted_bytes: 1024,
                plan: "free".into(),
                limit_bytes: 100,
            },
            AgentEvent::BackupFilesUnreadable {
                save_id: "s1".into(),
                game_slug: "factorio".into(),
                label: "main".into(),
                count: 2,
                kept_files: 1,
                sample_path: "a.sav".into(),
                sample_error: "denied".into(),
                uploaded: true,
            },
            AgentEvent::BackupNeedsAttention {
                save_id: "s1".into(),
                game_slug: "factorio".into(),
                label: "main".into(),
                conflicts: 5,
                error: "409".into(),
            },
            AgentEvent::BackupSkippedEmpty {
                save_id: "s1".into(),
                game_slug: "factorio".into(),
                likely_wrong_path: true,
            },
            AgentEvent::SaveConflictsBackedUp {
                save_id: "s1".into(),
                game_slug: "factorio".into(),
                count: 2,
                conflict_dir: "/tmp/x".into(),
            },
            AgentEvent::GameStarted {
                save_id: "s1".into(),
                game_slug: "factorio".into(),
            },
            AgentEvent::GameStopped {
                save_id: "s1".into(),
                game_slug: "factorio".into(),
            },
        ];
        for event in notifying {
            assert!(notifiable(&event), "{event:?} should be notifiable");
            assert!(notice_for(&event, &prefs(true, true)).is_some());
        }
    }

    /// Every problem answers to the failure switch. These are the notices that used
    /// to depend on which client you had: a full account said nothing on any desktop.
    #[test]
    fn every_backup_problem_answers_to_one_switch() {
        for event in [quota_full(), failure()] {
            assert!(notice_for(&event, &prefs(true, false)).is_none());
            assert!(notice_for(&event, &prefs(false, true)).is_some());
        }
    }

    #[test]
    fn games_opening_and_closing_have_their_own_switch() {
        let event = AgentEvent::GameStarted {
            save_id: "s1".into(),
            game_slug: "factorio".into(),
        };
        let quiet = Prefs::default();
        assert!(!quiet.notify_on_game_activity);
        assert!(notice_for(&event, &quiet).is_none());
        let loud = Prefs {
            notify_on_game_activity: true,
            ..Prefs::default()
        };
        assert_eq!(notice_for(&event, &loud).unwrap().kind, Kind::GameStarted);
    }

    #[test]
    fn a_repeated_problem_is_said_once_until_the_window_passes() {
        let quiet = Quiet::default();
        let start = Instant::now();
        assert!(quiet.admit("failed:s1".into(), start));
        assert!(!quiet.admit("failed:s1".into(), start + Duration::from_secs(60)));
        // Another save is another problem.
        assert!(quiet.admit("failed:s2".into(), start + Duration::from_secs(60)));
        assert!(quiet.admit("failed:s1".into(), start + PROBLEM_QUIET));
    }

    /// A failure right after a good backup is new, however soon it comes.
    #[test]
    fn a_landed_backup_ends_the_silence() {
        let quiet = Quiet::default();
        let now = Instant::now();
        assert!(quiet.admit("failed:s1".into(), now));
        assert!(quiet.admit("quota_full".into(), now));
        assert!(quiet.admit("trimmed:s1".into(), now));
        quiet.backup_landed("s1");
        assert!(quiet.admit("failed:s1".into(), now));
        assert!(quiet.admit("quota_full".into(), now));
        assert!(!quiet.admit("trimmed:s1".into(), now));
    }

    /// Twenty games finding the same full account are one notice.
    #[test]
    fn a_full_account_is_one_problem_across_saves() {
        let one = notice_for(&quota_full(), &prefs(false, true)).unwrap();
        let mut other = one.clone();
        other.save_id = "another".into();
        assert_eq!(quiet_key(&one), quiet_key(&other));
    }

    /// The whole point: one notice, one road, whichever the machine is.
    #[test]
    fn a_notice_takes_exactly_one_way_out() {
        assert_eq!(way_out(true, true), Way::Claimant);
        assert_eq!(way_out(true, false), Way::Claimant);
        assert_eq!(way_out(false, true), Way::System);
        assert_eq!(way_out(false, false), Way::App);
    }

    /// In Game Mode the OS's road ends nowhere, so a claim has to take the notice
    /// off it, and dropping the claim has to put it back.
    #[tokio::test]
    async fn a_claim_takes_the_notices_off_the_os_until_it_drops() {
        let recorder = Arc::new(Recorder(Mutex::new(Vec::new())));
        let notifier = Notifier::with_sink(recorder.clone());
        let mut heard = notifier.subscribe_notices();
        let note = text::render(&Kind::GameStarted, "Factorio", text::Lang::En);

        let claim = notifier.claim();
        notifier.send(note.clone(), NoticeLevel::Info).await;
        assert!(recorder.0.lock().unwrap().is_empty());
        let delivery = heard.recv().await.unwrap();
        assert_eq!(delivery.to, Way::Claimant);
        assert_eq!(delivery.note.title, note.title);

        drop(claim);
        notifier.send(note.clone(), NoticeLevel::Info).await;
        assert_eq!(recorder.0.lock().unwrap().as_slice(), &[note]);
        assert!(heard.try_recv().is_err());
    }

    /// Without a road of its own the notice goes to the app, not nowhere.
    #[tokio::test]
    async fn with_no_sink_the_app_is_asked() {
        let notifier = Notifier::built(None);
        let mut heard = notifier.subscribe_notices();
        let note = text::render(&Kind::GameStopped, "Factorio", text::Lang::En);
        notifier.send(note, NoticeLevel::Info).await;
        assert_eq!(heard.recv().await.unwrap().to, Way::App);
    }

    #[test]
    fn a_notice_names_the_game_and_only_a_chosen_label() {
        assert_eq!(game_name("hollow-knight", "main"), "Hollow Knight");
        assert_eq!(game_name("hollow-knight", ""), "Hollow Knight");
        assert_eq!(
            game_name("factorio", "Mundo nuevo"),
            "Factorio (Mundo nuevo)"
        );
        assert_eq!(game_name("factorio", "factorio"), "Factorio");
        // A custom game keeps the spelling its owner gave it.
        assert_eq!(game_name("OpenMW Saves", "main"), "OpenMW Saves");
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
            Some("Factorio (Mundo nuevo)")
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

    /// Sin backend de plataforma el daemon **no se cae**, y sin nadie escuchando
    /// tampoco: es macOS con la app cerrada.
    #[tokio::test]
    async fn a_notifier_with_nobody_listening_does_not_fall_over() {
        Notifier::built(None).consider(&failure()).await;
    }

    /// **Real** smoke against the OS. It does not run under `cargo test`: neither CI
    /// nor a session with no desktop has anything to show it, so it would fail on
    /// the environment and not on the code. It is the manual check that the notice
    /// really goes out and shows up where it should:
    ///
    /// ```text
    /// cargo test -p hoardd -- --ignored --nocapture the_os_takes
    /// ```
    #[cfg(any(target_os = "linux", windows))]
    #[test]
    #[ignore = "needs a desktop session that shows notifications"]
    fn the_os_takes_a_real_notification() {
        let sink = platform::sink().expect("this platform has a sink");
        let note = text::render(
            &Kind::BackupSaved {
                version: 42,
                bytes: 3 * 1024 * 1024,
            },
            "Factorio",
            Lang::for_user(None),
        );
        sink.deliver(&note).expect("the OS took it");
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
        notifier.send(note.clone(), NoticeLevel::Error).await;
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
