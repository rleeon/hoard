//! `hoardd`, Hoard's local sync service.
//!
//! Resident and per user: it stays alive with no clients around (this is
//! background sync) and it outlives the app being closed. It starts two ways, as a
//! user service at boot, or launched by a client that found none ("spawn if
//! absent", see [`hoardd::client::Client::ensure_running`]).

// Windows: no console in release. Of the two ways in, the client's already
// launches with `CREATE_NO_WINDOW` (see `client::detach`), but the service's
// cannot: the Task Scheduler runs the `.exe` with `InteractiveToken`, that is,
// inside the user's session, and Windows hands a "console" subsystem binary a
// console. The result is a black window with the sync's log every time you sign in.
// The "windows" subsystem removes it at the root; no diagnostics are lost because
// `init_tracing` also writes to a file (which is what `hoard sync logs` reads). In
// debug the console is kept, since that is where you do want to watch the daemon by
// hand, the same call `hoard-desktop` makes.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use anyhow::Result;
use clap::Parser;
use hoardd::endpoint::Endpoint;

/// The env var that turns the engine off without passing the flag. It exists
/// because whoever launches the daemon is almost always a client, not a person: an
/// integration test cannot let the daemon it launches start syncing the saves of
/// whoever runs the tests.
const NO_ENGINE_ENV: &str = "HOARDD_NO_ENGINE";

#[derive(Parser, Debug)]
#[command(
    name = "hoardd",
    version,
    about = "Hoard local sync service: owns the sync engine and serves thin clients over IPC"
)]
struct Cli {
    /// Socket (unix) o nombre del named pipe (Windows) donde escuchar. Por
    /// defecto, el del usuario actual.
    #[arg(long, value_name = "PATH")]
    socket: Option<String>,

    /// Serves the IPC without starting the engine. Diagnostics and tests.
    #[arg(long)]
    no_engine: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let _guard = init_tracing();
    hoardd::install_panic_logger();

    let no_engine = cli.no_engine
        || std::env::var_os(NO_ENGINE_ENV)
            .is_some_and(|v| !v.is_empty() && v != "0" && v != "false");

    let options = hoardd::Options {
        endpoint: cli.socket.map(Endpoint::new),
        with_engine: !no_engine,
    };

    // An explicit multi-threaded runtime (not `#[tokio::main]`) so the log and the
    // panic hook can be set up before a runtime exists: a panic during startup has to
    // land in the file too.
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let outcome = runtime.block_on(hoardd::run(options))?;
    match outcome {
        hoardd::Outcome::Served => {}
        hoardd::Outcome::AlreadyRunning => {
            // Exit 0 on purpose: "there was already a service" is the right outcome
            // of an idempotent start, not a failure that should stain the service
            // manager's log or scare the client that launched us.
            eprintln!("hoardd: another instance already owns the socket; nothing to do");
        }
        hoardd::Outcome::Relaunching { version } => {
            runtime.block_on(relaunch(&version));
        }
    }
    Ok(())
}

/// The exit code that asks to be relieved. Any non-zero value would do: all that
/// is needed is for systemd to see a failure and apply its `Restart=on-failure`. An
/// unlikely one is picked so a log reads "update", not "it died".
const EXIT_RELAUNCH: i32 = 75;

/// Our own binary on disk has just been replaced. Somebody has to start the new
/// one, and **who** depends on where we live:
///
/// - **Under a service manager on Unix** we exit with [`EXIT_RELAUNCH`] and it
///   brings us back (systemd through `Restart=on-failure`, launchd through
///   `KeepAlive`). Spawning a child here would not work: systemd kills the unit's
///   whole cgroup when it stops it, so the child would die with us however much
///   `setsid` it carried.
/// - **In every other case** (Windows, or a daemon a client brought up with no
///   service installed) there is nobody to start us again, so we start the new copy
///   ourselves and exit. With no cgroup in the way, the child survives.
async fn relaunch(version: &str) {
    let managed = cfg!(unix) && hoardd::autostart::installed().await;
    if managed {
        tracing::info!(
            version,
            "hoardd: exiting so the service manager starts the new binary"
        );
        // The log guard goes with the process, so it is given a moment for the last
        // line to reach the file.
        std::thread::sleep(std::time::Duration::from_millis(200));
        std::process::exit(EXIT_RELAUNCH);
    }
    match hoardd::client::respawn_service() {
        Ok(pid) => tracing::info!(version, pid, "hoardd: started the new binary"),
        Err(err) => tracing::error!(
            version,
            error = %format!("{err:#}"),
            "hoardd: the update is installed but nothing could start the new binary; \
             it will come up the next time a client needs it"
        ),
    }
}

/// Logs to stderr (which the service manager captures on Linux/macOS) **and** to
/// `<cache_dir>/logs/hoardd.log`, because on Windows the Task Scheduler throws
/// stdout/stderr away and with no file there would be no way to read anything.
/// Returns the `WorkerGuard`, which must live as long as the process so the file is
/// flushed on the way out.
fn init_tracing() -> Option<tracing_appender::non_blocking::WorkerGuard> {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    let (file_layer, guard) = match log_writer() {
        Some((writer, guard)) => (
            Some(tracing_subscriber::fmt::layer().with_writer(writer)),
            Some(guard),
        ),
        None => (None, None),
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .with(file_layer)
        // Log shipping to the connected server, as in the CLI and the desktop. It is
        // governed by the telemetry pref, which is re-read on every cycle.
        .with(hoard_agent::logship::start())
        .init();
    guard
}

fn log_writer() -> Option<(
    tracing_appender::non_blocking::NonBlocking,
    tracing_appender::non_blocking::WorkerGuard,
)> {
    let dir = hoard_agent::config::CliConfig::logs_dir().ok()?;
    std::fs::create_dir_all(&dir).ok()?;
    Some(tracing_appender::non_blocking(
        tracing_appender::rolling::never(dir, "hoardd.log"),
    ))
}
