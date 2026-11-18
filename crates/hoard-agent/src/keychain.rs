//! The system keyring, always bounded.
//!
//! Every `keyring` call the client makes comes through here: the Cloud session
//! ([`crate::cloud_auth`]) and the self-hosted token ([`crate::credentials`]).
//! The reason is the D.19 failure (ADR 0021): a locked keyring does not fail, it
//! waits, and a synchronous call that never returns hangs whoever made it.
//!
//! Both sessions share one thread and one queue on purpose. There are not two
//! keyrings: if `org.freedesktop.secrets` is locked it is locked for both, so a
//! thread per module would only give two hung threads instead of one. With a
//! healthy keyring the operations take milliseconds and the queue is invisible.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

use anyhow::{bail, Result};
pub use hoard_core::ipc::KeyringFault;

/// The wait cap on any keyring operation at all.
///
/// A locked keyring does not fail: `org.freedesktop.secrets` waits for somebody
/// to answer the unlock prompt, and in a session with no desktop (SSH, a NAS, the
/// D.19 dogfooding) nobody ever will. That unbounded wait left the engine in
/// `starting` forever without a single log line (`last_error` at `None`,
/// indistinguishable from "starting up") and, worse, made the daemon
/// unstoppable: `abort()` does not evict a synchronous call, so
/// `systemctl --user stop` sat in `deactivating` until the SIGKILL.
///
/// A healthy keyring answers in milliseconds, so five seconds gives no false
/// positives; and if the user is slow typing their password, the keeper's retry
/// picks it up as soon as it is unlocked.
pub const KEYRING_TIMEOUT: Duration = Duration::from_secs(5);

/// The keyring did not answer inside the cap. Its own type so "it is locked" is
/// never confused with "there is no session": confusing them is exactly what made
/// the failure invisible.
#[derive(Debug)]
pub struct KeyringTimeout {
    /// What was being done, in one phrase, for the log and `last_error`.
    pub doing: &'static str,
    pub after: Duration,
    /// An earlier call that gave up is still waiting on the keyring, so this one
    /// was never sent (see [`keyring_op`]).
    pub still_busy: bool,
}

impl std::fmt::Display for KeyringTimeout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.still_busy {
            return write!(
                f,
                "the system keyring still hasn't answered an earlier request after {}s, \
                 so Hoard didn't queue another one while {}. It is most likely locked, \
                 waiting for an unlock nobody can answer; unlock the login keyring \
                 (or sign in again with `hoard login`)",
                self.after.as_secs(),
                self.doing
            );
        }
        write!(
            f,
            "the system keyring didn't answer in {}s while {}. It is most likely \
             locked, waiting for an unlock nobody can answer; unlock the login \
             keyring (or sign in again with `hoard login`)",
            self.after.as_secs(),
            self.doing
        )
    }
}

impl std::error::Error for KeyringTimeout {}

/// The keyring answered, and said no. No D-Bus in a session with no desktop, a
/// corrupt entry, or, the case that matters here, a macOS ACL that authorises only
/// the binary that created the item, which is not the one reading it.
///
/// Its own type for the same reason as [`KeyringTimeout`]: "I won't give it to
/// you" and "there is no session" lead to opposite advice. In the first the user
/// did sign in, and signing in again fixes it at the root (it rewrites the item in
/// the name of whoever reads it); in the second they never signed in. Confusing
/// them sends the user looking for a problem they do not have.
#[derive(Debug)]
pub struct KeyringUnreadable {
    /// What was being done, for the log and `last_error`.
    pub doing: &'static str,
}

// No advice in the text. It used to promise that signing in again "fixes it for
// good", which is false when the keyring daemon is simply not running yet (a
// lingering service at boot): the Bazzite reporter signed in twice and got the
// same error on the next start. What to do depends on the fault, so the window
// picks the sentence from [`fault`].
impl std::fmt::Display for KeyringUnreadable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the system keyring didn't hand over the saved session while {}",
            self.doing
        )
    }
}

impl std::error::Error for KeyringUnreadable {}

/// The keyring is still locked and the last operation couldn't unlock it, so this
/// one was not sent: it would only raise the same failing prompt again.
///
/// Linux only in practice (see `prompts`), but the type stays on every platform so
/// [`fault`] can name it without a `cfg`.
#[derive(Debug)]
pub struct KeyringStillLocked;

impl std::fmt::Display for KeyringStillLocked {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            "the system keyring is still locked and couldn't be unlocked a moment ago; \
             Hoard won't raise another unlock prompt until it is unlocked",
        )
    }
}

impl std::error::Error for KeyringStillLocked {}

/// The user's services start at boot (`loginctl enable-linger`), before anyone
/// logs in, and a desktop keyring only exists inside a session: the service can't
/// rely on the keyring alone, so the session also stays in its 0600 file.
///
/// The Bazzite report of 22-10-2026: a sync that never ran between boot and
/// login. It is a plain file, like `config.toml` has always been: encrypting it
/// with a key kept on the same disk would protect it from nothing that can read it.
pub fn lingering() -> bool {
    if !cfg!(target_os = "linux") {
        return false;
    }
    ["USER", "LOGNAME"]
        .iter()
        .find_map(|k| {
            std::env::var(k)
                .ok()
                .filter(|v| !v.is_empty() && !v.contains('/'))
        })
        .is_some_and(|user| {
            std::path::Path::new("/var/lib/systemd/linger")
                .join(user)
                .exists()
        })
}

/// Which way the keyring failed, for the sentence the window shows.
///
/// [`KeyringUnreadable`] and [`KeyringTimeout`] answer "can Hoard get the saved
/// session?": no, and no. This answers "why not", and the four production
/// errors want four different next steps: `DBus error: The name is not
/// activatable` is a machine with no secret-service daemon, so telling that user
/// to unlock their login keyring sends them looking for something that isn't
/// installed; `Did not receive a reply` and our own cap are one that's there and
/// mute; `Platform secure storage failure: Crypto error: Unpad Error` is one
/// that's there, answering, and holding something it can't decrypt.
///
/// **This is the one place a message gets read to classify it, and it's allowed
/// here because the message isn't ours.** Everywhere else in this codebase the
/// rule is downcast-never-match, precisely because our own wording gets rewritten
/// without a thought. `keyring::Error::PlatformFailure` boxes the platform's
/// error and covers all three D-Bus shapes, so the variant alone can't tell them
/// apart and the only signal left is the text the platform produced. Read it as
/// data from outside, and keep the fallback honest: anything unrecognised is
/// [`Refused`](KeyringFault::Refused), never a guess.
pub fn fault(err: &anyhow::Error) -> Option<KeyringFault> {
    // Our own cap first: it never reaches `keyring::Error`, because the point of
    // it is that the call never came back.
    if err.is::<KeyringTimeout>() || err.is::<KeyringStillLocked>() {
        return Some(KeyringFault::Locked);
    }
    let native = err.downcast_ref::<keyring::Error>()?;
    Some(match native {
        keyring::Error::NoEntry => return None,
        keyring::Error::BadEncoding(_) | keyring::Error::Ambiguous(_) => KeyringFault::Damaged,
        keyring::Error::PlatformFailure(inner) | keyring::Error::NoStorageAccess(inner) => {
            classify_platform(&inner.to_string())
        }
        _ => KeyringFault::Refused,
    })
}

/// The platform's own words, for the three shapes production actually produced.
fn classify_platform(text: &str) -> KeyringFault {
    let lower = text.to_ascii_lowercase();
    // Nothing to talk to: the bus has no such name and can't start one. On a
    // headless box or a desktop without a secret-service daemon this is the
    // permanent answer, not a bad moment.
    if lower.contains("not activatable")
        || lower.contains("serviceunknown")
        || lower.contains("no such interface")
        || lower.contains("no session bus")
        || lower.contains("not provided by any .service files")
    {
        return KeyringFault::Missing;
    }
    // There, and mute. Same shape as our own cap running out. A locked GNOME
    // keyring with nobody logged in has no prompter, so the unlock prompt it
    // raises is dismissed at once: a lingering box that booted at 09:40 on
    // 25-10-2026 read that as a refusal and retried every 5 minutes for 9 hours,
    // with the login that would have unlocked it never waking the engine.
    if lower.contains("did not receive a reply")
        || lower.contains("timed out")
        || lower.contains("timeout")
        || lower.contains("is locked")
        || lower.contains("prompt was dismissed")
    {
        return KeyringFault::Locked;
    }
    // There, answering, and what it holds won't come back out. `Unpad Error` is
    // the encrypted-session negotiation failing.
    if lower.contains("crypto error")
        || lower.contains("unpad")
        || lower.contains("decrypt")
        || lower.contains("corrupt")
    {
        return KeyringFault::Damaged;
    }
    KeyringFault::Refused
}

type KeyringJob = Box<dyn FnOnce() + Send>;

/// The queue for the single thread that talks to the keyring.
///
/// One thread per call would be enough not to over-wait, but a hung call cannot be
/// cancelled: with the keyring locked and the keeper retrying every few minutes,
/// each attempt would leave one more thread hung forever. Serialising means what
/// piles up is the queue (one `Box` per attempt) rather than the threads.
///
/// And it is a loose thread rather than one from the `spawn_blocking` pool: when
/// the runtime is dropped, tokio waits for its blocking threads to finish, so one
/// hung there would again stop the process dying, which is half the bug. Nobody
/// waits on a thread of its own that is never joined.
fn keyring_queue() -> Option<&'static Mutex<mpsc::Sender<KeyringJob>>> {
    static QUEUE: OnceLock<Option<Mutex<mpsc::Sender<KeyringJob>>>> = OnceLock::new();
    QUEUE
        .get_or_init(|| {
            let (tx, rx) = mpsc::channel::<KeyringJob>();
            std::thread::Builder::new()
                .name("hoard-keyring".to_string())
                .spawn(move || {
                    while let Ok(job) = rx.recv() {
                        job();
                    }
                })
                .map_err(|err| {
                    tracing::error!(error = %err, "keyring: couldn't start the keyring thread")
                })
                .ok()?;
            Some(Mutex::new(tx))
        })
        .as_ref()
}

// As in the journal and the engine slot: somebody else's panic must not leave the
// keyring unreachable forever.
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[derive(Default)]
struct Finished {
    done: Mutex<bool>,
    cv: Condvar,
}

impl Finished {
    fn set(&self) {
        *lock(&self.done) = true;
        self.cv.notify_all();
    }

    fn wait(&self, up_to: Duration) -> bool {
        let guard = lock(&self.done);
        let (guard, _) = self
            .cv
            .wait_timeout_while(guard, up_to, |done| !*done)
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *guard
    }
}

/// Position in the queue, so the latest abandoned job can be told from an older one.
static NEXT_SEQ: AtomicU64 = AtomicU64::new(0);

/// The newest job whose caller gave up while it was still running.
///
/// The queue alone did not stop a pile-up. On 07-11-2026 a laptop with nobody
/// logged in had its keyring thread stuck on a prompt that would never complete
/// (`dbus-secret-service` waits up to a year for it), and every minute another
/// read went into the queue behind it, each one waiting its 5 s first. Now a call
/// waits for that job instead, within its own cap, and is not queued at all if the
/// job is still stuck. The thread runs jobs in order, so the newest abandoned one
/// being done means every older one is too.
static ABANDONED: Mutex<Option<(u64, Arc<Finished>)>> = Mutex::new(None);

fn earlier_call_finished(up_to: Duration) -> bool {
    let pending = lock(&ABANDONED).clone();
    let Some((seq, finished)) = pending else {
        return true;
    };
    if !finished.wait(up_to) {
        return false;
    }
    let mut slot = lock(&ABANDONED);
    if slot.as_ref().is_some_and(|(s, _)| *s == seq) {
        *slot = None;
    }
    true
}

fn remember_abandoned(seq: u64, finished: Arc<Finished>) {
    let mut slot = lock(&ABANDONED);
    if slot.as_ref().is_none_or(|(s, _)| seq > *s) {
        *slot = Some((seq, finished));
    }
}

#[cfg(test)]
thread_local! {
    static CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// How many keyring operations this thread has asked for, so a test can prove a
/// path never touches the keyring without reading the real one.
#[cfg(test)]
pub(crate) fn calls_on_this_thread() -> usize {
    CALLS.with(|c| c.get())
}

/// Runs `op` on the keyring thread and stops waiting for it after `wait`. Returns
/// whatever `op` returns, or [`KeyringTimeout`] when it did not answer in time.
pub(crate) fn keyring_op<T: Send + 'static>(
    doing: &'static str,
    wait: Duration,
    op: impl FnOnce() -> Result<T> + Send + 'static,
) -> Result<T> {
    #[cfg(test)]
    CALLS.with(|c| c.set(c.get() + 1));

    let deadline = Instant::now() + wait;
    let Some(queue) = keyring_queue() else {
        bail!("no keyring thread available");
    };
    if !earlier_call_finished(wait) {
        return Err(anyhow::Error::new(KeyringTimeout {
            doing,
            after: wait,
            still_busy: true,
        }));
    }
    let finished = Arc::new(Finished::default());
    let (tx, rx) = mpsc::channel();
    let job_finished = Arc::clone(&finished);
    let job: KeyringJob = Box::new(move || {
        let result = guarded(op);
        job_finished.set();
        // If whoever asked has already given up, the send fails and the result is
        // dropped: nobody is left waiting on anybody.
        let _ = tx.send(result);
    });
    let sender = lock(queue);
    // Taken under the sender's lock, so sequence order is queue order.
    let seq = NEXT_SEQ.fetch_add(1, Ordering::Relaxed);
    if sender.send(job).is_err() {
        bail!("the keyring thread is gone");
    }
    drop(sender);
    match rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
        Ok(result) => result,
        Err(RecvTimeoutError::Timeout) => {
            remember_abandoned(seq, finished);
            Err(anyhow::Error::new(KeyringTimeout {
                doing,
                after: wait,
                still_busy: false,
            }))
        }
        // The thread left with the operation half done (a panic inside `keyring`).
        Err(RecvTimeoutError::Disconnected) => bail!("the keyring call died while {doing}"),
    }
}

/// The operation as the keyring thread runs it: skipped while a dismissed prompt
/// would only be raised again.
#[cfg(target_os = "linux")]
fn guarded<T>(op: impl FnOnce() -> Result<T>) -> Result<T> {
    if prompts::skip() {
        return Err(anyhow::Error::new(KeyringStillLocked));
    }
    let result = op();
    if let Err(err) = &result {
        prompts::observe(err);
    }
    result
}

// Keychain and Credential Manager raise no unlock prompt a session can't show.
#[cfg(not(target_os = "linux"))]
fn guarded<T>(op: impl FnOnce() -> Result<T>) -> Result<T> {
    op()
}

/// Any operation on a locked Secret Service collection raises an unlock prompt
/// (`keyring` unlocks before it reads or writes). With no display the prompt fails
/// at once, and GNOME Keyring keeps one descriptor for every prompt that failed: on
/// 07-11-2026 a laptop with nobody logged in took 1.012 prompts in 7 hours, one per
/// minute from the desktop's cloud poll, hit its 1.024 limit and spun at 100 %
/// writing the same error to the journal.
///
/// So after an operation that found the keyring locked, ask before each next one
/// whether the collection still is. That is a property read: no prompt, no
/// descriptor. While it is locked the operation is skipped; once somebody unlocks it
/// (a login unlocks the login keyring) the next one goes through, with no timer to
/// wait out. A prompt the user cancelled on purpose counts too, and that is wanted:
/// it stops Hoard from asking again every minute.
#[cfg(target_os = "linux")]
mod prompts {
    use std::sync::atomic::{AtomicBool, Ordering};

    use anyhow::Result;

    static ARMED: AtomicBool = AtomicBool::new(false);

    #[derive(Debug, PartialEq)]
    pub(super) enum Before {
        Run,
        /// Unlocked since the operation that armed the check.
        Disarm,
        Skip,
    }

    pub(super) fn before(armed: bool, still_locked: impl FnOnce() -> Result<bool>) -> Before {
        if !armed {
            return Before::Run;
        }
        match still_locked() {
            Ok(true) => Before::Skip,
            Ok(false) => Before::Disarm,
            // Can't tell (no default collection, the service went away): let the
            // operation report what is really wrong.
            Err(_) => Before::Run,
        }
    }

    pub(super) fn skip() -> bool {
        match before(ARMED.load(Ordering::Relaxed), default_collection_locked) {
            Before::Run => false,
            Before::Disarm => {
                ARMED.store(false, Ordering::Relaxed);
                false
            }
            Before::Skip => true,
        }
    }

    pub(super) fn observe(err: &anyhow::Error) {
        if left_locked(err) {
            ARMED.store(true, Ordering::Relaxed);
        }
    }

    /// The keyring was locked and stayed locked. Told by type and by the error name
    /// the spec defines, never by the text: `keyring` boxes the platform's own error.
    ///
    /// Two shapes, both from the same laptop with GNOME Keyring: updating an item
    /// that exists comes back as a dismissed prompt, and creating one comes back as
    /// `IsLocked` even though the prompt was raised (and failed) just the same.
    pub(super) fn left_locked(err: &anyhow::Error) -> bool {
        let inner = match err.downcast_ref::<keyring::Error>() {
            Some(keyring::Error::NoStorageAccess(inner))
            | Some(keyring::Error::PlatformFailure(inner)) => inner,
            _ => return false,
        };
        match inner.downcast_ref::<dbus_secret_service::Error>() {
            Some(dbus_secret_service::Error::Prompt | dbus_secret_service::Error::Locked) => true,
            Some(dbus_secret_service::Error::Dbus(e)) => {
                e.name() == Some("org.freedesktop.Secret.Error.IsLocked")
            }
            _ => false,
        }
    }

    // `keyring` keeps its items in the `default` collection.
    fn default_collection_locked() -> Result<bool> {
        use dbus_secret_service::{EncryptionType, SecretService};
        let ss = SecretService::connect(EncryptionType::Plain)?;
        let locked = ss.get_default_collection()?.is_locked()?;
        Ok(locked)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    /// What a locked keyring does: wait for an unlock nobody is going to answer.
    /// Simulated with an operation that takes far longer than the cap, rather than
    /// an infinite one, so the keyring thread is not left busy for the rest of the
    /// suite.
    fn a_locked_keyring() -> impl FnOnce() -> Result<Option<String>> + Send + 'static {
        || {
            std::thread::sleep(Duration::from_millis(300));
            // By now nobody is listening: the result is dropped and the keyring
            // thread is free for the next test.
            Ok(None)
        }
    }

    /// The D.19 failure: the keyring call never came back. Now the wait stops,
    /// and with a typed reason, the one that lands in `last_error` and in the
    /// service's log.
    #[test]
    fn a_keyring_that_never_answers_gives_up_with_a_reason() {
        let started = Instant::now();
        let err = keyring_op(
            "reading the Cloud session",
            Duration::from_millis(20),
            a_locked_keyring(),
        )
        .expect_err("tiene que rendirse, no esperar");
        // What matters is not the number but that the wait is bounded: the caller
        // gets control back long before the operation finishes.
        assert!(
            started.elapsed() < Duration::from_millis(250),
            "waited too long: {:?}",
            started.elapsed()
        );

        let timeout = err
            .downcast_ref::<KeyringTimeout>()
            .expect("the reason is typed, not only in the text");
        assert_eq!(timeout.doing, "reading the Cloud session");
        let text = err.to_string();
        assert!(
            text.contains("keyring") && text.contains("locked"),
            "{text}"
        );
    }

    /// A keyring that does answer passes through untouched, and its own failure
    /// (no D-Bus, a corrupt entry) is not disguised as a timeout: the reason has
    /// to be the real one.
    #[test]
    fn a_keyring_that_answers_is_passed_through_verbatim() {
        let got = keyring_op("reading the Cloud session", KEYRING_TIMEOUT, || {
            Ok(Some("jwt".to_string()))
        })
        .expect("contesta");
        assert_eq!(got.as_deref(), Some("jwt"));

        let err = keyring_op::<()>("reading the Cloud session", KEYRING_TIMEOUT, || {
            bail!("no D-Bus session bus")
        })
        .expect_err("el fallo del llavero se propaga");
        assert!(err.downcast_ref::<KeyringTimeout>().is_none());
        assert_eq!(err.to_string(), "no D-Bus session bus");
    }

    /// The four shapes production actually produced, each landing on the advice
    /// that helps. Telling someone with no secret-service daemon to unlock their
    /// login keyring sends them looking for something that isn't installed, and
    /// that was the state of the art for seven users.
    #[test]
    fn the_four_production_errors_classify_apart() {
        fn platform(text: &'static str) -> anyhow::Error {
            anyhow::Error::new(keyring::Error::PlatformFailure(text.into()))
        }

        assert_eq!(
            fault(&platform("DBus error: The name is not activatable")),
            Some(KeyringFault::Missing)
        );
        assert_eq!(
            fault(&platform("Did not receive a reply")),
            Some(KeyringFault::Locked)
        );
        assert_eq!(
            fault(&platform("Crypto error: Unpad Error")),
            Some(KeyringFault::Damaged)
        );
        // Our own cap never reaches a `keyring::Error`, the whole point being
        // that the call didn't come back, so it has to be recognised on its own.
        let capped = anyhow::Error::new(KeyringTimeout {
            doing: "reading the Cloud session",
            after: KEYRING_TIMEOUT,
            still_busy: false,
        });
        assert_eq!(fault(&capped), Some(KeyringFault::Locked));
        assert_eq!(
            fault(&anyhow::Error::new(KeyringStillLocked)),
            Some(KeyringFault::Locked)
        );
    }

    /// The 07-11-2026 pile-up: with the keyring thread stuck, a new call must not
    /// join the queue behind it, where it would run (and prompt) whenever the
    /// thread frees up, long after its caller left.
    #[test]
    fn a_call_behind_one_that_never_answered_is_not_queued() {
        keyring_op(
            "reading the Cloud session",
            Duration::from_millis(20),
            a_locked_keyring(),
        )
        .expect_err("tiene que rendirse, no esperar");

        let ran = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let ran_in_op = Arc::clone(&ran);
        let started = Instant::now();
        let err = keyring_op(
            "saving the Cloud session",
            Duration::from_millis(20),
            move || {
                ran_in_op.store(true, Ordering::SeqCst);
                Ok(())
            },
        )
        .expect_err("the keyring is still busy");
        assert!(
            started.elapsed() < Duration::from_millis(250),
            "waited too long: {:?}",
            started.elapsed()
        );
        let timeout = err.downcast_ref::<KeyringTimeout>().expect("motivo tipado");
        assert!(timeout.still_busy, "{err:#}");
        assert_eq!(timeout.doing, "saving the Cloud session");

        // Not queued, so not even a free thread runs it later.
        std::thread::sleep(Duration::from_millis(400));
        assert!(!ran.load(Ordering::SeqCst));

        // And once the stuck call returns, the keyring is reachable again.
        keyring_op("reading the Cloud session", KEYRING_TIMEOUT, || Ok(())).expect("answers again");
    }

    /// Both shapes a locked GNOME Keyring gave on 07-11-2026 arm the check; the
    /// Windows size limit seen through the same wrapper, and the bare text, don't.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_keyring_left_locked_is_told_apart_by_type() {
        fn native(e: dbus_secret_service::Error) -> anyhow::Error {
            anyhow::Error::new(keyring::Error::NoStorageAccess(Box::new(e)))
        }
        let dismissed =
            native(dbus_secret_service::Error::Prompt).context("saving the Cloud session");
        assert!(prompts::left_locked(&dismissed));
        let is_locked = anyhow::Error::new(keyring::Error::PlatformFailure(Box::new(
            dbus_secret_service::Error::Dbus(dbus::Error::new_custom(
                "org.freedesktop.Secret.Error.IsLocked",
                "Cannot create an item in a locked collection",
            )),
        )));
        assert!(prompts::left_locked(&is_locked));
        assert!(prompts::left_locked(&native(
            dbus_secret_service::Error::Locked
        )));

        let other_dbus = anyhow::Error::new(keyring::Error::PlatformFailure(Box::new(
            dbus_secret_service::Error::Dbus(dbus::Error::new_custom(
                "org.freedesktop.DBus.Error.ServiceUnknown",
                "The name is not activatable",
            )),
        )));
        assert!(!prompts::left_locked(&other_dbus));
        let too_long = anyhow::Error::new(keyring::Error::PlatformFailure(
            "Attribute 'password encoded as UTF-16' is longer than platform limit of 2560 chars"
                .into(),
        ));
        assert!(!prompts::left_locked(&too_long));
        let text_only = anyhow::Error::new(keyring::Error::PlatformFailure(
            "Secret Service: unlock prompt was dismissed".into(),
        ));
        assert!(!prompts::left_locked(&text_only));
    }

    /// Unarmed, nothing is asked; armed, only a keyring that is still locked is left
    /// alone, and not being able to tell runs the call.
    #[cfg(target_os = "linux")]
    #[test]
    fn after_a_dismissed_prompt_only_a_still_locked_keyring_is_skipped() {
        use prompts::{before, Before};
        assert_eq!(
            before(false, || panic!("nothing to check while unarmed")),
            Before::Run
        );
        assert_eq!(before(true, || Ok(true)), Before::Skip);
        assert_eq!(before(true, || Ok(false)), Before::Disarm);
        assert_eq!(before(true, || bail!("no default collection")), Before::Run);
    }

    /// The text a locked GNOME keyring gives a lingering service before anyone
    /// logs in, word for word from the journal. `Locked` is what lets the login
    /// wake the engine at once instead of on its next 5-minute retry.
    #[test]
    fn a_dismissed_unlock_prompt_is_a_locked_keyring() {
        let err = anyhow::Error::new(keyring::Error::PlatformFailure(
            "Secret Service: unlock prompt was dismissed".into(),
        ));
        assert_eq!(fault(&err), Some(KeyringFault::Locked));
    }

    /// The reason has to survive the wrapping every caller does on the way up,
    /// and "no entry" is not a fault at all: a machine that was never signed in
    /// would otherwise be told its keyring is broken.
    #[test]
    fn the_fault_survives_context_and_ignores_an_empty_keyring() {
        let wrapped = anyhow::Error::new(keyring::Error::PlatformFailure(
            "Crypto error: Unpad Error".into(),
        ))
        .context(KeyringUnreadable {
            doing: "reading the Cloud session",
        })
        .context("starting the engine");
        assert_eq!(fault(&wrapped), Some(KeyringFault::Damaged));

        assert_eq!(fault(&anyhow::Error::new(keyring::Error::NoEntry)), None);
        assert_eq!(fault(&anyhow::anyhow!("no network")), None);
    }

    /// Anything the platform says that we don't recognise is `Refused`, not a
    /// guess at the friendliest-sounding cause. `Missing` in particular tells the
    /// user this machine has no keyring at all, and being wrong about that sends
    /// them off to install something they already have.
    #[test]
    fn an_unrecognised_platform_error_is_not_guessed_at() {
        let odd = anyhow::Error::new(keyring::Error::NoStorageAccess(
            "the vault is on fire".into(),
        ));
        assert_eq!(fault(&odd), Some(KeyringFault::Refused));
    }

    /// The cap covers both sessions, which is why there is only one thread: the
    /// self-hosted token's gives up just like the Cloud one, and with its own
    /// reason.
    #[test]
    fn the_self_hosted_session_is_bounded_by_the_same_thread() {
        let err = keyring_op(
            "reading the self-hosted session",
            Duration::from_millis(20),
            a_locked_keyring(),
        )
        .expect_err("tiene que rendirse, no esperar");
        let timeout = err.downcast_ref::<KeyringTimeout>().expect("motivo tipado");
        assert_eq!(timeout.doing, "reading the self-hosted session");
    }

    /// The other half of D.19: the wait cannot live on the thread of the task the
    /// shutdown aborts. With the read on the blocking pool, the task awaiting it is
    /// cancelled at once, so the runtime is free and the daemon stoppable even
    /// while the keyring still refuses to answer.
    #[tokio::test(flavor = "current_thread")]
    async fn a_task_waiting_on_the_keyring_can_be_aborted_at_once() {
        let task = tokio::spawn(async {
            match tokio::task::spawn_blocking(|| {
                keyring_op(
                    "reading the Cloud session",
                    Duration::from_secs(30),
                    a_locked_keyring(),
                )
            })
            .await
            {
                Ok(result) => result,
                Err(join) => Err(anyhow::Error::new(join)),
            }
        });
        // A single-threaded runtime: if the wait lived on it, this `yield` would
        // not come back and the `abort` would never run.
        tokio::task::yield_now().await;
        let started = Instant::now();
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert!(
            started.elapsed() < Duration::from_millis(250),
            "shutdown waited on the keyring: {:?}",
            started.elapsed()
        );
    }
}
