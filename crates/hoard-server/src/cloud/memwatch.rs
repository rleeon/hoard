//! Bounce the process before the kernel does it for us.
//!
//! The cloud machine can never idle-stop: every installed client beats
//! `/v1/presence/heartbeat` twice a minute and Fly probes `/v1/health` every
//! 15 s, so the VM is billed around the clock and its RAM is sized for the
//! peak, not the average. Resident set measured 85 MB after one deploy and
//! 128 MB four days later, which is the shape of something that only ever
//! grows, and the ceiling is what we pay for.
//!
//! So: watch the resident set, and when it approaches the machine's limit,
//! drain and exit non-zero so Fly's `on-failure` policy starts a fresh one.
//! A restart is not a fix for whatever is growing, it is a way to buy the
//! smaller machine without waiting for an OOM kill to arrive mid-upload.
//!
//! Two things this deliberately does not do. It does not fire during the
//! first [`MIN_UPTIME`] of a process: a server already over the line moments
//! after boot has a problem a restart cannot solve, and bouncing it would
//! spend Fly's ten retries and leave the app stopped. And it does not guess a
//! ceiling: an unreadable one means the watchdog reports and never fires,
//! because a wrong ceiling here restarts a healthy server forever.
//!
//! The first cut of this shipped [`spawn`] returning early when it found no
//! ceiling, which dropped the sender; a dropped `watch::Sender` wakes every
//! receiver at once, the shutdown future read that as "drain now", and the
//! server exited 0 seconds after boot, which is the one exit code Fly does not
//! restart. Hence [`Watch`]: the task owns the sender for the life of the
//! process whether or not it is armed, and the receiver side treats a dropped
//! sender as "never".

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Fraction of the machine's memory that trips the bounce. Scales with the
/// machine instead of hard-coding a number that goes stale on the next resize.
const TRIP_FRACTION: f64 = 0.94;

/// How long a process must have been up before the watchdog may fire.
const MIN_UPTIME: Duration = Duration::from_secs(15 * 60);

/// Gap between readings. Growth is measured in MB per day, so this is about
/// noticing before the kernel does, not about reacting fast.
const INTERVAL: Duration = Duration::from_secs(30);

/// How long to let in-flight work finish once the bounce is decided.
const DRAIN_GRACE: Duration = Duration::from_secs(20);

/// Set when the watchdog has asked for a bounce, so the exit code after the
/// graceful shutdown says "restart me" rather than "I am done".
static BOUNCING: AtomicBool = AtomicBool::new(false);

/// Whether the shutdown now under way was asked for by the watchdog.
pub fn bouncing() -> bool {
    BOUNCING.load(Ordering::SeqCst)
}

/// Where the ceiling came from, so the startup line says which number is in
/// play and a wrong one is obvious in the log rather than at 3am.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ceiling {
    /// A container memory limit. This is the real bound under Docker.
    Cgroup(u64),
    /// The VM's own RAM. On Fly there is no container limit, the microVM's
    /// total *is* the bound, and it reads a little under the advertised size
    /// because the kernel reserves some.
    MemTotal(u64),
    /// Nothing trustworthy. The watchdog reports and never fires.
    Unknown,
}

impl Ceiling {
    fn bytes(self) -> Option<u64> {
        match self {
            Ceiling::Cgroup(n) | Ceiling::MemTotal(n) => Some(n),
            Ceiling::Unknown => None,
        }
    }

    fn source(self) -> &'static str {
        match self {
            Ceiling::Cgroup(_) => "cgroup",
            Ceiling::MemTotal(_) => "meminfo",
            Ceiling::Unknown => "none",
        }
    }
}

/// Resident set in bytes, from `/proc/self/statm` (field 2, in pages).
fn resident_bytes() -> Option<u64> {
    let statm = std::fs::read_to_string("/proc/self/statm").ok()?;
    let pages: u64 = statm.split_whitespace().nth(1)?.parse().ok()?;
    // `sysconf(_SC_PAGESIZE)` without pulling in libc: every platform this
    // runs on is 4 KiB, and being wrong here only shifts the trip point.
    Some(pages * 4096)
}

/// The memory ceiling, preferring a container limit over the machine's RAM:
/// under Docker the cgroup is the real bound and `MemTotal` is the whole host,
/// which would be far too high to ever trip.
fn ceiling() -> Ceiling {
    let v2 = std::fs::read_to_string("/sys/fs/cgroup/memory.max").ok();
    let v1 = || std::fs::read_to_string("/sys/fs/cgroup/memory/memory.limit_in_bytes").ok();
    if let Some(n) = v2.or_else(v1).as_deref().and_then(parse_cgroup_limit) {
        return Ceiling::Cgroup(n);
    }
    match std::fs::read_to_string("/proc/meminfo").ok().as_deref().and_then(parse_mem_total) {
        Some(n) => Ceiling::MemTotal(n),
        None => Ceiling::Unknown,
    }
}

/// `max` is how cgroup v2 says unlimited; v1 says it with a number near
/// `u64::MAX` instead, and a ceiling above any real machine is the same as
/// none, in which case the caller falls through to `MemTotal`.
fn parse_cgroup_limit(raw: &str) -> Option<u64> {
    let raw = raw.trim();
    if raw == "max" {
        return None;
    }
    let n: u64 = raw.parse().ok()?;
    (n < (1 << 40)).then_some(n)
}

/// `MemTotal:  1932032 kB` off the top of `/proc/meminfo`.
fn parse_mem_total(raw: &str) -> Option<u64> {
    let line = raw.lines().find(|l| l.starts_with("MemTotal:"))?;
    let kb: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kb * 1024)
}

/// Live handle on the bounce signal. The task owns this for the life of the
/// process, armed or not: dropping the sender would wake the shutdown future
/// and take the server down.
struct Watch(tokio::sync::watch::Sender<bool>);

/// Start the watchdog. `shutdown` is fired rather than exiting outright so the
/// listener drains first; the caller checks [`bouncing`] afterwards to pick the
/// exit code.
pub fn spawn(shutdown: tokio::sync::watch::Sender<bool>) {
    let found = ceiling();
    let trip = found.bytes().map(|n| (n as f64 * TRIP_FRACTION) as u64);
    match trip {
        Some(t) => tracing::info!(
            source = found.source(),
            ceiling_mb = found.bytes().unwrap_or(0) / 1024 / 1024,
            trip_mb = t / 1024 / 1024,
            rss_mb = resident_bytes().unwrap_or(0) / 1024 / 1024,
            "memory watchdog armed"
        ),
        None => tracing::warn!(
            rss_mb = resident_bytes().unwrap_or(0) / 1024 / 1024,
            "memory watchdog idle: no readable memory ceiling, it will never bounce"
        ),
    }

    tokio::spawn(async move {
        // Held, never dropped: see the module note. Even the unarmed path
        // parks here forever rather than letting the sender fall out of scope.
        let watch = Watch(shutdown);
        let started = tokio::time::Instant::now();
        let mut tick = tokio::time::interval(INTERVAL);
        loop {
            tick.tick().await;
            let Some(rss) = resident_bytes() else { continue };
            let Some(trip) = trip else { continue };
            if rss < trip {
                continue;
            }
            if started.elapsed() < MIN_UPTIME {
                tracing::warn!(
                    rss_mb = rss / 1024 / 1024,
                    trip_mb = trip / 1024 / 1024,
                    "over the memory trip point this early; a restart would not fix it, holding"
                );
                continue;
            }
            tracing::error!(
                rss_mb = rss / 1024 / 1024,
                trip_mb = trip / 1024 / 1024,
                uptime_secs = started.elapsed().as_secs(),
                "memory trip point reached, draining and bouncing"
            );
            BOUNCING.store(true, Ordering::SeqCst);
            let _ = watch.0.send(true);
            // Bounded: a request wedged on a slow R2 read must not keep alive a
            // machine that is already out of room.
            tokio::time::sleep(DRAIN_GRACE).await;
            tracing::error!("drain grace elapsed, exiting for restart");
            std::process::exit(1);
        }
    });
}

/// Resolve once the watchdog asks for a bounce, and never otherwise. A sender
/// that goes away means no watchdog, which must park rather than fire: reading
/// a dropped sender as a shutdown request is what took the API down once.
pub async fn bounce_requested(mut rx: tokio::sync::watch::Receiver<bool>) {
    loop {
        if rx.changed().await.is_err() {
            std::future::pending::<()>().await;
        }
        if *rx.borrow() {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resident_set_is_readable_and_sane() {
        // Only meaningful on Linux, which is every host this ships to.
        if !std::path::Path::new("/proc/self/statm").exists() {
            return;
        }
        let rss = resident_bytes().expect("statm parses");
        assert!(rss > 1024 * 1024, "a running test process holds more than 1 MB");
    }

    #[test]
    fn an_unlimited_cgroup_falls_through_to_meminfo() {
        // Both spellings of "no limit" have to come back None so the caller
        // moves on, instead of tripping at 94% of a number meaning the opposite.
        assert_eq!(parse_cgroup_limit("max\n"), None);
        assert_eq!(parse_cgroup_limit("9223372036854771712"), None);
        assert_eq!(parse_cgroup_limit("nonsense"), None);
    }

    #[test]
    fn a_real_ceiling_survives_the_parse() {
        assert_eq!(parse_cgroup_limit("268435456\n"), Some(256 * 1024 * 1024));
        let trip = (parse_cgroup_limit("268435456").unwrap() as f64 * TRIP_FRACTION) as u64;
        assert_eq!(trip / 1024 / 1024, 240, "94% of the 256 MB machine is 240 MB");
    }

    #[test]
    fn meminfo_yields_the_machine_ram() {
        let sample = "MemTotal:         246336 kB\nMemFree:          123456 kB\n";
        assert_eq!(parse_mem_total(sample), Some(246336 * 1024));
        assert_eq!(parse_mem_total("nothing useful here"), None);
    }

    // The regression that matters: the shutdown future must not resolve just
    // because nobody is holding the sender any more.
    #[tokio::test]
    async fn a_dropped_sender_never_asks_for_a_bounce() {
        let (tx, rx) = tokio::sync::watch::channel(false);
        drop(tx);
        let fired = tokio::time::timeout(Duration::from_millis(50), bounce_requested(rx)).await;
        assert!(fired.is_err(), "a dropped sender must park, not fire");
    }

    #[tokio::test]
    async fn a_real_request_does_ask_for_a_bounce() {
        let (tx, rx) = tokio::sync::watch::channel(false);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(5)).await;
            let _ = tx.send(true);
            // Held past the send, as the watchdog task holds it.
            tokio::time::sleep(Duration::from_secs(60)).await;
        });
        let fired = tokio::time::timeout(Duration::from_millis(500), bounce_requested(rx)).await;
        assert!(fired.is_ok(), "a real bounce request must resolve");
    }

    #[tokio::test]
    async fn an_unrelated_send_does_not_ask_for_a_bounce() {
        let (tx, rx) = tokio::sync::watch::channel(false);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(5)).await;
            let _ = tx.send(false); // a wake that is not a request
            tokio::time::sleep(Duration::from_secs(60)).await;
        });
        let fired = tokio::time::timeout(Duration::from_millis(50), bounce_requested(rx)).await;
        assert!(fired.is_err(), "only `true` is a bounce request");
    }
}
