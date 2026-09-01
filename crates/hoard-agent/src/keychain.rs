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

use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

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
}

impl std::fmt::Display for KeyringTimeout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

impl std::fmt::Display for KeyringUnreadable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the system keyring refused to hand over the saved session while {}. \
             Signing in again rewrites it under the service, which fixes it for good",
            self.doing
        )
    }
}

impl std::error::Error for KeyringUnreadable {}

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
    if err.is::<KeyringTimeout>() {
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
    // There, and mute. Same shape as our own cap running out.
    if lower.contains("did not receive a reply")
        || lower.contains("timed out")
        || lower.contains("timeout")
        || lower.contains("is locked")
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

/// Runs `op` on the keyring thread and stops waiting for it after `wait`. Returns
/// whatever `op` returns, or [`KeyringTimeout`] when it did not answer in time.
pub(crate) fn keyring_op<T: Send + 'static>(
    doing: &'static str,
    wait: Duration,
    op: impl FnOnce() -> Result<T> + Send + 'static,
) -> Result<T> {
    let Some(queue) = keyring_queue() else {
        bail!("no keyring thread available");
    };
    let (tx, rx) = mpsc::channel();
    let job: KeyringJob = Box::new(move || {
        // If whoever asked has already given up, the send fails and the result is
        // dropped: nobody is left waiting on anybody.
        let _ = tx.send(op());
    });
    // As in the journal and the engine slot: somebody else's panic must not leave
    // the keyring unreachable forever.
    let sender = queue
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if sender.send(job).is_err() {
        bail!("the keyring thread is gone");
    }
    drop(sender);
    match rx.recv_timeout(wait) {
        Ok(result) => result,
        Err(RecvTimeoutError::Timeout) => {
            Err(anyhow::Error::new(KeyringTimeout { doing, after: wait }))
        }
        // The thread left with the operation half done (a panic inside `keyring`).
        Err(RecvTimeoutError::Disconnected) => bail!("the keyring call died while {doing}"),
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
        });
        assert_eq!(fault(&capped), Some(KeyringFault::Locked));
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
