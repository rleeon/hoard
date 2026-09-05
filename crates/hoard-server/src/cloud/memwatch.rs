//! Bounce the process before the kernel does it for us.
//!
//! The cloud machine can never idle-stop: every installed client beats
//! `/v1/presence/heartbeat` twice a minute and Fly probes `/v1/health` every
//! 15 s, so the VM is billed around the clock and its RAM is sized for the
//! peak, not the average. Resident set measured 85 MB after one deploy and
//! 128 MB four days later, which is the shape of something that only ever
//! grows, and the ceiling is what we pay for.
//!
//! So: watch the resident set, and when it approaches the cgroup's limit,
//! drain and exit non-zero so Fly's `on-failure` policy starts a fresh one.
//! A restart is not a fix for whatever is growing, it is a way to buy the
//! smaller machine without waiting for an OOM kill to arrive mid-upload.
//!
//! Two things this deliberately does not do. It does not fire during the
//! first [`MIN_UPTIME`] of a process: a server that is already over the line
//! moments after boot has a problem a restart cannot solve, and bouncing it
//! would just spend Fly's ten retries and leave the app stopped. And it does
//! not guess a limit: no readable cgroup means no watchdog, because a wrong
//! ceiling here restarts a healthy server forever.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Fraction of the cgroup limit that trips the bounce. On the 256 MB machine
/// this is the 240 MB it is expected to turn over at; on a 512 MB one it
/// scales with it instead of firing early.
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

/// Resident set in bytes, from `/proc/self/statm` (field 2, in pages).
fn resident_bytes() -> Option<u64> {
    let statm = std::fs::read_to_string("/proc/self/statm").ok()?;
    let pages: u64 = statm.split_whitespace().nth(1)?.parse().ok()?;
    // `sysconf(_SC_PAGESIZE)` without pulling in libc: every platform this
    // runs on is 4 KiB, and being wrong here only shifts the trip point.
    Some(pages * 4096)
}

/// The container's memory ceiling. cgroup v2 first, since that is what Fly
/// runs; v1 as a fallback for a self-hoster on an older kernel. `max` means
/// unlimited, which is not a ceiling we can watch.
fn cgroup_limit_bytes() -> Option<u64> {
    let v2 = std::fs::read_to_string("/sys/fs/cgroup/memory.max").ok();
    let v1 = || std::fs::read_to_string("/sys/fs/cgroup/memory/memory.limit_in_bytes").ok();
    parse_limit(&v2.or_else(v1)?)
}

/// Split out from the read so the sentinels are testable. `max` is how cgroup
/// v2 says unlimited; v1 says it with a number near `u64::MAX` instead, and a
/// ceiling above any real machine is the same as no ceiling at all.
fn parse_limit(raw: &str) -> Option<u64> {
    let raw = raw.trim();
    if raw == "max" {
        return None;
    }
    let n: u64 = raw.parse().ok()?;
    (n < (1 << 40)).then_some(n)
}

/// Start the watchdog. `shutdown` is fired instead of exiting outright so the
/// listener drains first; the caller checks [`bouncing`] afterwards to pick
/// the exit code.
pub fn spawn(shutdown: tokio::sync::watch::Sender<bool>) {
    let Some(limit) = cgroup_limit_bytes() else {
        tracing::warn!("memory watchdog off: no readable cgroup limit");
        return;
    };
    let trip = (limit as f64 * TRIP_FRACTION) as u64;
    tracing::info!(
        limit_mb = limit / 1024 / 1024,
        trip_mb = trip / 1024 / 1024,
        "memory watchdog armed"
    );

    tokio::spawn(async move {
        let started = tokio::time::Instant::now();
        let mut tick = tokio::time::interval(INTERVAL);
        loop {
            tick.tick().await;
            let Some(rss) = resident_bytes() else { continue };
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
            let _ = shutdown.send(true);
            // The drain has a bounded wait: a request wedged on a slow R2 read
            // must not keep a machine alive that is already out of room.
            tokio::time::sleep(DRAIN_GRACE).await;
            tracing::error!("drain grace elapsed, exiting for restart");
            std::process::exit(1);
        }
    });
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
    fn an_unlimited_cgroup_reads_as_no_ceiling() {
        // Both spellings of "no limit" have to come back None, or the watchdog
        // trips at 94% of a number that means the opposite.
        assert_eq!(parse_limit("max\n"), None);
        assert_eq!(parse_limit("9223372036854771712"), None); // v1 sentinel
        assert_eq!(parse_limit("nonsense"), None);
    }

    #[test]
    fn a_real_ceiling_survives_the_parse() {
        assert_eq!(parse_limit("268435456\n"), Some(256 * 1024 * 1024));
        // 94% of the 256 MB machine is the 240 MB it should turn over at.
        let trip = (parse_limit("268435456").unwrap() as f64 * TRIP_FRACTION) as u64;
        assert_eq!(trip / 1024 / 1024, 240);
    }
}
