//! Keep a long-lived background loop alive, and say so when it dies.
//!
//! Vivía en `hoard-desktop/src/commands/supervisor.rs`. Subió aquí en el Slice
//! 4a because D.12's rule ("if it outlives a request, it goes under
//! `supervise`") applies to the daemon too, and a module private to
//! the desktop is no use to it. The desktop re-exports it from its old path, so
//! its callers did not change.
//!
//! **Why this exists (ADR 0021 D.12).** The cloud-pull poller stopped after two
//! ticks and *nothing in the log said so*: no `gate busy`, no second `started`,
//! no `stopped`. A `tokio::spawn` that panics is reaped by the runtime, and the
//! error is parked in a `JoinHandle` nobody joins, so a dead loop and a healthy
//! but quiet one read identically. The engine was blind for the rest of the
//! session with no autorecovery. The concrete panic (a `state::<CloudFeed>()`
//! against unmanaged state) is fixed at its source, but chasing one panic only
//! defers the next: a background task that dies in silence is a bug by itself.
//!
//! So every supervised loop gets the same contract: a panic is an incident,
//! logged at `error` and retried with backoff. Ending on purpose is a
//! *declaration*: the body has to hand back a [`Finished`], which a loop that
//! never returns can never produce. "Stopped by accident" is therefore not
//! representable, which beats catching it at runtime.
//!
//! One task, not two. The obvious shape, `tokio::spawn` the loop and join its
//! handle to catch the panic, is wrong here: the schedulers `abort()` the
//! handle they hold, and an inner task would survive that abort as an orphan.
//! Two pollers racing is precisely the class of bug this module exists to stop.
//! `catch_unwind` over the future keeps everything in the one task the caller
//! can still kill. (Its `AssertUnwindSafe` is honest for both callers: their
//! state is either RAII-released on unwind or derived data the next pass
//! rebuilds wholesale.)

use std::time::{Duration, Instant};

use futures::FutureExt;

/// Backoff between restarts: a loop that dies on its first line must not spin,
/// and one that dies of something transient must come back fast.
const RESTART_BACKOFF_MIN_SECS: u64 = 5;
const RESTART_BACKOFF_MAX_SECS: u64 = 5 * 60;

/// A run this long counts as healthy, so the next death restarts from the
/// bottom of the backoff instead of inheriting an old escalation.
const HEALTHY_RUN_SECS: u64 = 10 * 60;

/// Proof that a supervised loop ended on purpose: the realtime subscriber
/// returning because the user signed out. The supervisor ends with it; whoever
/// owns the lifecycle starts a fresh one when it's relevant again.
///
/// Being a value the body must construct is the point: a loop that isn't
/// supposed to end (the poller) simply never produces one, so it cannot stop
/// the supervisor by falling off its bottom. That's the "returned unexpectedly"
/// case handled by construction instead of by a log line.
pub struct Finished;

/// Run `body` under supervision, restarting it on panic until it hands back a
/// [`Finished`].
///
/// `name` tags the log lines; use the same string the loop logs under so a
/// reader can grep one word for the whole lifecycle.
pub async fn supervise<F, Fut>(name: &'static str, body: F)
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Finished>,
{
    let mut backoff = Duration::from_secs(RESTART_BACKOFF_MIN_SECS);
    loop {
        let started = Instant::now();
        let ran_secs = || started.elapsed().as_secs();
        match std::panic::AssertUnwindSafe(body()).catch_unwind().await {
            Ok(Finished) => {
                tracing::info!(name, ran_secs = ran_secs(), "supervisor: loop finished");
                return;
            }
            Err(payload) => tracing::error!(
                name,
                ran_secs = ran_secs(),
                panic = %panic_text(&payload),
                "supervisor: loop panicked, restarting"
            ),
        }
        if started.elapsed() >= Duration::from_secs(HEALTHY_RUN_SECS) {
            backoff = Duration::from_secs(RESTART_BACKOFF_MIN_SECS);
        }
        tracing::info!(
            name,
            restart_in_secs = backoff.as_secs(),
            "supervisor: restarting after backoff"
        );
        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(Duration::from_secs(RESTART_BACKOFF_MAX_SECS));
    }
}

/// Best-effort text of a caught panic payload, for the log line. Covers
/// `panic!("literal")` and `panic!("{fmt}")`, which is everything we throw.
fn panic_text(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "<non-string panic payload>".to_string()
    }
}
