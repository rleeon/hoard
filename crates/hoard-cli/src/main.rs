mod commands;
mod output;

use anyhow::Result;
use clap::{Parser, Subcommand};
use hoard_agent::api;
use std::path::PathBuf;
use tracing_appender::non_blocking::WorkerGuard;

#[derive(Parser)]
#[command(name = "hoard", version, about = "Hoard save-sync client")]
struct Cli {
    // No subcommand: paint the panel (banner::show). With a subcommand: dispatch
    // below.
    #[command(subcommand)]
    command: Option<Commands>,
    /// Machine-readable output: one JSON envelope on stdout, human logs on
    /// stderr. What agents and scripts read; see `hoard agents`.
    #[arg(long, global = true)]
    json: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Install (or repair) every Hoard component this machine wants
    Install {
        /// Core only: never the desktop app, even where one would fit
        #[arg(long, conflicts_with = "with_desktop")]
        headless: bool,
        /// Install the desktop app too, even if this machine looks headless
        #[arg(long)]
        with_desktop: bool,
        /// Pin a specific version instead of this binary's (e.g. `1.0.4`)
        #[arg(long, conflicts_with = "from")]
        version: Option<String>,
        /// Install from a folder of Hoard binaries instead of downloading: the
        /// core, and the app and Hoard Setup when they are there too
        #[arg(long, value_name = "DIR")]
        from: Option<std::path::PathBuf>,
    },
    /// Upgrade every installed component to the latest release, together
    Upgrade {
        /// Pin a specific version instead of the latest (e.g. `1.0.4`)
        #[arg(long)]
        version: Option<String>,
    },
    /// Open the desktop app (forwards to `hoard-desktop`)
    Desktop {
        /// Arguments passed through as-is to the app
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Start the self-host server (forwards to `hoard-server`)
    Server {
        /// Arguments passed through as-is to the server
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Manage the background sync service: the resident automatic sync (the app
    /// without a window) run under your OS service manager (systemd --user,
    /// launchd, Task Scheduler). `hoard sync start|stop|status`.
    Sync {
        #[command(subcommand)]
        action: Option<commands::service::SyncCommand>,
    },
    /// (deprecated) shows the status panel; the daemon is now `hoard sync`
    #[command(hide = true)]
    Daemon,
    /// Detect a game, create its save and remember the path (what `daemon`/`sync`
    /// then watch)
    Track {
        /// Game name or slug (optional if --slug is given)
        query: Option<String>,
        /// Exact slug: skips the fuzzy search
        #[arg(long)]
        slug: Option<String>,
        /// Explicit save folder (wins over detection)
        #[arg(long)]
        path: Option<PathBuf>,
        /// Save label (default "main")
        #[arg(long)]
        label: Option<String>,
        /// Deep scan (slower, wider coverage)
        #[arg(long)]
        deep: bool,
    },
    /// List the saves this machine tracks (local, no network)
    Saves,
    /// Show server status (uses /v1/health)
    Status,
    /// List the machines on this account: which are on and what they're playing
    Devices,
    /// Check this machine's tracked saves for the mistakes that break syncing,
    /// such as folders that vanished, backup mirrors tracked instead of the real
    /// save, or rows named after an installer, and print the command that fixes
    /// each. Local and offline; changes nothing.
    Doctor,
    /// See and change which notifications this machine gets. The same switches
    /// as the app's Settings and the Decky plugin: the service sends them, not
    /// this terminal.
    Notifications {
        #[command(subcommand)]
        command: Option<commands::notifications::NotificationsCommand>,
    },
    /// Configuration file management
    Config {
        #[command(subcommand)]
        action: commands::config::ConfigCommand,
    },
    /// Sign in. With no flags it asks whether you want Hoard One or a
    /// self-hosted server. `--token` goes straight to self-host; `--email`
    /// forces the Cloud email/code path.
    Login {
        /// Self-host bearer token (`hoard_v1_<hex>`, from `hoard-admin token
        /// create`). Skips the interactive menu and signs in to self-host.
        #[arg(long)]
        token: Option<String>,
        /// Self-host server URL (used with `--token`; otherwise taken from your
        /// config). E.g. `http://ubserver:12421`.
        #[arg(long)]
        server: Option<String>,
        /// Force the Cloud email + password / emailed-code path instead of phone
        /// pairing, so you pick which account to sign in as.
        #[arg(long)]
        email: bool,
    },
    /// Sign out (Cloud and self-host)
    Logout,
    /// Show the current session (Cloud or self-host)
    Whoami,
    /// Set up an AI assistant to drive Hoard: prints the skill file that
    /// teaches it the commands, the safety rules and how to keep itself
    /// current. The skill ships inside this binary, so it updates with Hoard.
    Agents {
        /// Print the skill file itself, for the assistant to save
        /// (`hoard agents --skill > ~/.claude/skills/hoard/SKILL.md`)
        #[arg(long)]
        skill: bool,
    },
    /// Browse the game catalog
    Games {
        #[command(subcommand)]
        action: commands::games::GameCommand,
    },
    /// Hoard One account: export, storage, entitlements, playtime
    Cloud {
        #[command(subcommand)]
        action: commands::cloud::CloudCommand,
    },
    /// Benchmark the local game-detection scan (the heavy half of what Automatic
    /// Mode runs each tick). No server needed; writes nothing.
    Scan {
        /// List every detected game, not just the summary counts. Implied by
        /// --json, which always carries the list.
        #[arg(long)]
        verbose: bool,
        /// Run the exhaustive deep scan: arbitrary Wine prefixes
        /// (Heroic/CrossOver/Flatpak/mounted media), Flatpak/Snap/EmuDeck
        /// roots, deeper walks. Slower; mirrors the Library deep-scan tile.
        #[arg(long)]
        deep: bool,
        /// Stop offering this folder (and anything under it) in future scans.
        /// Repeatable. Excluding by folder is what sticks when a phase-4
        /// find keeps coming back under a different name.
        #[arg(long = "exclude", value_name = "PATH")]
        exclude: Vec<String>,
        /// Undo `--exclude` for this exact folder. Repeatable.
        #[arg(long = "unexclude", value_name = "PATH")]
        unexclude: Vec<String>,
        /// Print the excluded folders and exit.
        #[arg(long)]
        list_excluded: bool,
    },
    /// Manage save namespaces
    Save {
        #[command(subcommand)]
        action: commands::saves::SaveCommand,
    },
    /// Manage snapshots (list / delete / undelete)
    Snapshots {
        #[command(subcommand)]
        action: SnapshotCommand,
    },
    /// Upload a directory as a new snapshot
    Backup {
        /// Save id (UUID), see `hoard save list`
        save_id: String,
        /// Source directory to back up. Required unless previously remembered.
        #[arg(long)]
        from: Option<PathBuf>,
        /// Save the (save_id, local_path) mapping in local state for future runs
        #[arg(long)]
        remember: bool,
    },
    /// Restore a snapshot to disk
    Restore {
        /// Save id (UUID)
        save_id: String,
        /// Snapshot version number; defaults to latest
        #[arg(long)]
        version: Option<i64>,
        /// Destination directory (or use the remembered local_path if omitted)
        #[arg(long)]
        to: Option<PathBuf>,
        /// Skip SHA256 verification
        #[arg(long)]
        no_verify: bool,
        /// Allow extracting into a non-empty directory
        #[arg(long)]
        force: bool,
        /// Show what would change in the folder and stop, without restoring
        #[arg(long)]
        dry_run: bool,
        /// Also write the snapshot's config files (.ini, .cfg, .toml, settings)
        /// over this machine's. Off by default: those files carry the other
        /// machine's resolution, GPU and paths, and games crash on them.
        #[arg(long, alias = "allow-config")]
        allow_ini: bool,
        /// Keep --to as this save's folder on this machine: later restores and
        /// backups use it, and the sync service starts watching it
        #[arg(long, requires = "to")]
        remember: bool,
    },
}

#[derive(Subcommand)]
enum SnapshotCommand {
    /// List snapshots for a save
    List {
        save_id: String,
        /// Include soft-deleted snapshots (in trash)
        #[arg(long)]
        all: bool,
    },
    /// Soft-delete a snapshot (moves it to trash; recover with `undelete`).
    /// Hoard One has no trash: there the version is deleted for good, and the
    /// whole save with it if it was the last one.
    Delete {
        save_id: String,
        version: i64,
        #[arg(long)]
        yes: bool,
    },
    /// Restore a soft-deleted snapshot back to active state (self-hosted only)
    Undelete { save_id: String, version: i64 },
    /// Show or set your cap on stored versions per save. No value shows it, a
    /// number sets it, `off` means unlimited. The server prunes immediately.
    ///
    /// The copies you ask for yourself (`hoard backup`, and the safety copy taken
    /// before a restore) have their own budget, unlimited by default: `--manual`.
    /// That way a game autosaving every minute cannot fill the history and take
    /// out the copy you made before the boss.
    MaxVersions {
        /// New cap (1 to 10000), or `off` to remove the cap
        value: Option<String>,
        /// Act on the budget for the copies you asked for, not the automatic ones
        #[arg(long)]
        manual: bool,
        /// Skip the confirmation when the new cap would delete versions
        #[arg(long)]
        yes: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    // Before anything can print: every emitter reads this.
    output::set_json(cli.json);

    // Hold the file writer's flush guard for the whole process so the sync
    // service's log file flushes on exit.
    let _file_guard = init_tracing(&cli);

    if let Err(e) = dispatch(cli).await {
        let exit = output::emit_error(&e);
        std::process::exit(exit);
    }
    Ok(())
}

/// Initialize the global tracing subscriber: stderr always, plus a file layer
/// mirroring `tracing` events to `<state_dir>/logs/sync.log` when running as
/// `hoard sync run`. The Task Scheduler service (Windows) drops stdout/stderr,
/// so without the file layer `hoard sync logs` would have nothing to show;
/// macOS launchd and Linux systemd already capture stdout/stderr via the plist
/// / journald, so the extra file is harmless there. Returns the `WorkerGuard`
/// that must outlive the process to flush the file on exit (None unless the
/// file layer is active).
fn init_tracing(cli: &Cli) -> Option<WorkerGuard> {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"));

    // Only `hoard sync run` (the service's ExecStart) writes the file log; a
    // one-shot CLI command doesn't need a persistent log file.
    let (file_layer, guard) = match &cli.command {
        Some(Commands::Sync {
            action: Some(commands::service::SyncCommand::Run { .. }),
        }) => match commands::daemon::sync_log_writer() {
            Some((writer, guard)) => (
                Some(tracing_subscriber::fmt::layer().with_writer(writer)),
                Some(guard),
            ),
            None => (None, None),
        },
        _ => (None, None),
    };

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .with(file_layer)
        // Best-effort log shipping to the connected server. Short-lived
        // CLI invocations may exit before a batch flushes; that's fine.
        .with(hoard_agent::logship::start())
        .init();
    guard
}

/// Which commands actually honour `--json`.
///
/// The flag promises one envelope on stdout. A command that has not been converted
/// would print its human table instead *and exit 0*, which is worse than refusing:
/// the caller parses that as JSON, fails, and has no way to tell a malformed
/// answer from a command that was never going to answer. So the ones that cannot,
/// say so.
fn supports_json(cmd: &Commands) -> bool {
    match cmd {
        Commands::Saves
        | Commands::Status
        | Commands::Devices
        | Commands::Doctor
        | Commands::Notifications { .. }
        | Commands::Whoami
        | Commands::Scan { .. }
        | Commands::Restore { .. } => true,
        Commands::Save { action } => matches!(
            action,
            commands::saves::SaveCommand::List { .. }
                | commands::saves::SaveCommand::Show { .. }
                | commands::saves::SaveCommand::Untrack { .. }
        ),
        Commands::Snapshots { action } => matches!(action, SnapshotCommand::List { .. }),
        _ => false,
    }
}

async fn dispatch(cli: Cli) -> Result<()> {
    let Some(command) = cli.command else {
        return commands::banner::show(true).await;
    };

    if output::json() && !supports_json(&command) {
        return Err(output::err(
            "json_unsupported",
            "this command has no --json output yet. The ones that do: saves, \
             doctor, status, devices, notifications, whoami, scan, restore, \
             save list, save show, save untrack, snapshots list.",
        ));
    }

    match command {
        Commands::Install {
            headless,
            with_desktop,
            version,
            from,
        } => {
            let want = match (headless, with_desktop) {
                (true, _) => commands::install::Want::Headless,
                (_, true) => commands::install::Want::Desktop,
                _ => commands::install::Want::Keep,
            };
            commands::install::run(want, version, from).await
        }
        Commands::Upgrade { version } => commands::upgrade::run(version).await,
        Commands::Desktop { args } => commands::launch::run("hoard-desktop", &args),
        Commands::Server { args } => commands::launch::run("hoard-server", &args),
        Commands::Sync { action } => commands::service::run(action).await,
        Commands::Daemon => commands::banner::show(true).await,
        Commands::Track {
            query,
            slug,
            path,
            label,
            deep,
        } => {
            commands::track::run(commands::track::Args {
                query,
                slug,
                path,
                label,
                deep,
            })
            .await
        }
        Commands::Saves => commands::tracked::run().await,
        Commands::Status => commands::status::run().await,
        Commands::Devices => commands::devices::run().await,
        Commands::Doctor => commands::doctor::run().await,
        Commands::Notifications { command } => commands::notifications::run(command).await,
        Commands::Config { action } => commands::config::run(action).await,
        Commands::Login {
            token,
            server,
            email,
        } => commands::auth::login(token, server, email).await,
        Commands::Logout => commands::auth::logout().await,
        Commands::Whoami => commands::auth::whoami().await,
        Commands::Agents { skill } => commands::agents::run(skill),
        Commands::Games { action } => commands::games::run(action).await,
        Commands::Cloud { action } => commands::cloud::run(action).await,
        Commands::Scan {
            verbose,
            deep,
            exclude,
            unexclude,
            list_excluded,
        } => commands::scan::run(verbose, deep, exclude, unexclude, list_excluded).await,
        Commands::Save { action } => commands::saves::run(action).await,
        Commands::Snapshots { action } => snapshots_dispatch(action).await,
        Commands::Backup {
            save_id,
            from,
            remember,
        } => commands::backup::run(save_id, from, remember).await,
        Commands::Restore {
            save_id,
            version,
            to,
            no_verify,
            force,
            dry_run,
            allow_ini,
            remember,
        } => {
            commands::restore::apply(
                save_id, version, to, no_verify, force, dry_run, allow_ini, remember,
            )
            .await
        }
    }
}

async fn snapshots_dispatch(cmd: SnapshotCommand) -> Result<()> {
    let active = commands::link::resolve_session().await?;
    let is_cloud = active.is_cloud;
    let client = active.client;
    match cmd {
        SnapshotCommand::List { save_id, all } => list_snapshots(&client, save_id, all).await,
        SnapshotCommand::Delete {
            save_id,
            version,
            yes,
        } => {
            // Self-hosted keeps a trash and Cloud does not, so the question has
            // to say which of the two this is.
            let (question, done) = if is_cloud {
                (
                    format!(
                        "permanently delete v{version} of save {save_id}? Hoard One has no \
                         trash, and if it is the last version the save goes with it."
                    ),
                    format!("deleted v{version} of save {save_id} (permanent)"),
                )
            } else {
                (
                    format!("soft-delete v{version} of save {save_id}?"),
                    format!("soft-deleted v{version} of save {save_id}"),
                )
            };
            if !yes && !output::interactive() {
                return Err(output::err(
                    "needs_confirmation",
                    format!(
                        "{question} That needs a confirmation and there is no terminal \
                         to ask. Pass --yes if you mean it."
                    ),
                ));
            }
            if !yes {
                use std::io::Write;
                print!("{question} [y/N] ");
                std::io::stdout().flush()?;
                let mut buf = String::new();
                std::io::stdin().read_line(&mut buf)?;
                if !matches!(buf.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
                    println!("aborted");
                    return Ok(());
                }
            }
            client.snapshot_delete(&save_id, version).await?;
            println!("{done}");
            Ok(())
        }
        SnapshotCommand::Undelete { save_id, version } => {
            client.snapshot_restore(&save_id, version).await?;
            println!("undeleted v{} of save {}", version, save_id);
            Ok(())
        }
        SnapshotCommand::MaxVersions { value, manual, yes } => {
            let kind = if manual { "manual" } else { "automatic" };
            match value.as_deref() {
                None => match client.get_max_versions(manual).await? {
                    Some(n) => println!("max {kind} versions per save: {n}"),
                    None => println!("max {kind} versions per save: unlimited"),
                },
                Some("off") => {
                    client.set_max_versions(None, manual).await?;
                    println!("max {kind} versions per save: unlimited");
                }
                Some(raw) => {
                    let n: i64 = raw
                        .parse()
                        .map_err(|_| anyhow::anyhow!("expected a number or `off`, got {raw:?}"))?;
                    // Dry-run first: if the cap would delete stored versions,
                    // confirm before the server prunes them.
                    let would_prune = client.preview_max_versions(n, manual).await?;
                    if would_prune > 0 && !yes {
                        use std::io::Write;
                        print!(
                            "a cap of {n} will delete the {would_prune} oldest stored version(s); continue? [y/N] "
                        );
                        std::io::stdout().flush()?;
                        let mut buf = String::new();
                        std::io::stdin().read_line(&mut buf)?;
                        if !matches!(buf.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
                            println!("aborted");
                            return Ok(());
                        }
                    }
                    client.set_max_versions(Some(n), manual).await?;
                    if would_prune > 0 {
                        println!(
                            "max {kind} versions per save: {n} ({would_prune} old versions pruned)"
                        );
                    } else {
                        println!("max {kind} versions per save: {n}");
                    }
                }
            }
            Ok(())
        }
    }
}

/// One stored version of a save.
#[derive(serde::Serialize)]
struct SnapshotRow {
    version_num: i64,
    file_count: i64,
    total_size_bytes: i64,
    created_at: String,
    /// Which machine made this copy. `None` on versions stored before the server
    /// kept it, which the table shows as a dash.
    device_name: Option<String>,
    /// `active`, `pinned` or `trash`. Tagged rather than left implicit: a trashed
    /// version still lists, and restoring one by accident is exactly the mistake
    /// worth making impossible to stumble into.
    state: &'static str,
    /// The save this version is about, derived by the server from the manifest.
    /// `None` on versions stored before it did that, and on servers that don't.
    #[serde(skip_serializing_if = "Option::is_none")]
    save_name: Option<String>,
    /// Files added or rewritten since the previous version. `None` when there is
    /// no insight to say, and never `0`, which would claim nothing changed.
    #[serde(skip_serializing_if = "Option::is_none")]
    changed_files: Option<u32>,
}

async fn list_snapshots(
    client: &api::ApiClient,
    save_id: String,
    include_deleted: bool,
) -> Result<()> {
    let snaps = client.list_snapshots(&save_id, include_deleted).await?;
    let rows: Vec<SnapshotRow> = snaps
        .into_iter()
        .map(|s| SnapshotRow {
            version_num: s.version_num,
            file_count: s.file_count,
            total_size_bytes: s.total_size_bytes,
            created_at: s.created_at.to_string(),
            device_name: s.device_name,
            state: if s.deleted_at.is_some() {
                "trash"
            } else if s.is_pinned {
                "pinned"
            } else {
                "active"
            },
            save_name: s.insight.as_ref().and_then(|i| i.title.clone()),
            changed_files: s.insight.as_ref().map(|i| i.changed_files),
        })
        .collect();

    output::emit(&rows, |rows| {
        if rows.is_empty() {
            println!("(no snapshots)");
            return;
        }
        // The machine goes in the table for the same reason it goes in the
        // window: with one save synced on two machines, the date does not say
        // which of the two copies this is. Versions from before the server stored
        // it come out as a dash. The SAVE column is the one that answers "which of
        // my saves is this?" when several share a folder, and it is blank where
        // the server did not derive it.
        println!(
            "{:>5}  {:<20}  {:>5}  {:>10}  {:<25}  {:<16}  STATE",
            "VER", "SAVE", "FILES", "SIZE", "CREATED", "DEVICE"
        );
        for s in rows {
            println!(
                "{:>5}  {:<20}  {:>5}  {:>10}  {:<25}  {:<16}  {}",
                s.version_num,
                s.save_name.as_deref().unwrap_or("—"),
                s.file_count,
                fmt_bytes(s.total_size_bytes as u64),
                s.created_at,
                s.device_name.as_deref().unwrap_or("—"),
                s.state.to_uppercase()
            );
        }
    })
}

fn fmt_bytes(b: u64) -> String {
    let b = b as f64;
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    if b >= GB {
        format!("{:.2}G", b / GB)
    } else if b >= MB {
        format!("{:.2}M", b / MB)
    } else if b >= KB {
        format!("{:.2}K", b / KB)
    } else {
        format!("{}B", b as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::error::ErrorKind;
    use clap::CommandFactory;

    fn parse(argv: &[&str]) -> Cli {
        let full = std::iter::once("hoard").chain(argv.iter().copied());
        match Cli::try_parse_from(full) {
            Ok(cli) => cli,
            Err(err) => panic!("`hoard {}` should parse: {err}", argv.join(" ")),
        }
    }

    fn refusal(argv: &[&str]) -> ErrorKind {
        let full = std::iter::once("hoard").chain(argv.iter().copied());
        match Cli::try_parse_from(full) {
            Ok(_) => panic!("`hoard {}` should be refused", argv.join(" ")),
            Err(err) => err.kind(),
        }
    }

    /// `conflicts_with` and `requires` name their partners by string, so a
    /// renamed field leaves them pointing at nothing, and clap only notices when
    /// somebody happens to type that exact combination. This walks the whole tree
    /// now.
    #[test]
    fn the_command_tree_is_well_formed() {
        Cli::command().debug_assert();
    }

    /// Bare `hoard` paints the status panel, so it must not be a usage error.
    #[test]
    fn bare_hoard_is_a_command_of_its_own() {
        assert!(parse(&[]).command.is_none());
    }

    /// Agents tack `--json` on wherever it lands; both places have to mean it.
    #[test]
    fn json_is_honoured_before_and_after_the_subcommand() {
        assert!(parse(&["--json", "saves"]).json);
        assert!(parse(&["saves", "--json"]).json);
        assert!(parse(&["save", "list", "--json"]).json);
        assert!(!parse(&["saves"]).json);
    }

    #[test]
    fn install_refuses_contradictory_requests() {
        assert_eq!(
            refusal(&["install", "--headless", "--with-desktop"]),
            ErrorKind::ArgumentConflict
        );
        // A folder of binaries has the version it has; pinning another is a lie.
        assert_eq!(
            refusal(&["install", "--version", "1.0.4", "--from", "/tmp/hoard"]),
            ErrorKind::ArgumentConflict
        );
    }

    /// `--remember` keeps the destination as the save's folder, so with no
    /// `--to` there is nothing to keep.
    #[test]
    fn restore_remember_needs_a_destination() {
        assert_eq!(
            refusal(&["restore", "some-save", "--remember"]),
            ErrorKind::MissingRequiredArgument
        );
        match parse(&["restore", "some-save", "--to", "/tmp/x", "--remember"]).command {
            Some(Commands::Restore { remember, to, .. }) => {
                assert!(remember);
                assert_eq!(to, Some(PathBuf::from("/tmp/x")));
            }
            _ => panic!("not a restore"),
        }
    }

    /// The flag was renamed, and scripts written against the old spelling must
    /// keep working.
    #[test]
    fn restore_still_accepts_allow_config() {
        match parse(&["restore", "some-save", "--allow-config"]).command {
            Some(Commands::Restore { allow_ini, .. }) => assert!(allow_ini),
            _ => panic!("not a restore"),
        }
    }

    /// `hoard desktop` and `hoard server` forward the rest of the line to another
    /// binary, so flags this CLI has never heard of must reach it untouched.
    #[test]
    fn forwarding_commands_pass_unknown_flags_through() {
        match parse(&["server", "--config", "/etc/hoard.toml", "-v"]).command {
            Some(Commands::Server { args }) => {
                assert_eq!(args, ["--config", "/etc/hoard.toml", "-v"]);
            }
            _ => panic!("not server"),
        }
        match parse(&["desktop", "--minimized"]).command {
            Some(Commands::Desktop { args }) => assert_eq!(args, ["--minimized"]),
            _ => panic!("not desktop"),
        }
    }

    #[test]
    fn sync_autostart_takes_only_on_or_off() {
        assert_eq!(
            refusal(&["sync", "autostart", "maybe"]),
            ErrorKind::InvalidValue
        );
        for state in ["on", "off"] {
            match parse(&["sync", "autostart", state]).command {
                Some(Commands::Sync {
                    action: Some(commands::service::SyncCommand::Autostart { state: Some(got) }),
                }) => assert_eq!(got, state),
                _ => panic!("`sync autostart {state}` parsed as something else"),
            }
        }
    }

    /// `hoard notifications on` with nothing after it would ask the service to
    /// change nothing and print the switches as if it had.
    #[test]
    fn notifications_on_and_off_need_a_switch() {
        assert_eq!(
            refusal(&["notifications", "on"]),
            ErrorKind::MissingRequiredArgument
        );
        assert_eq!(
            refusal(&["notifications", "off", "everything"]),
            ErrorKind::InvalidValue
        );
        parse(&["notifications", "off", "updates", "game-activity"]);
    }

    /// The `--json` promise per command: the ones listed answer with an
    /// envelope, every other one refuses up front instead of printing a human
    /// table that the caller then fails to parse.
    #[test]
    fn json_support_is_declared_per_command() {
        let answers = [
            vec!["saves"],
            vec!["status"],
            vec!["devices"],
            vec!["doctor"],
            vec!["whoami"],
            vec!["scan"],
            vec!["notifications"],
            vec!["restore", "some-save"],
            vec!["save", "list"],
            vec!["save", "show", "some-save"],
            vec!["save", "untrack", "some-save"],
            vec!["snapshots", "list", "some-save"],
        ];
        for argv in answers {
            let cmd = parse(&argv).command.unwrap();
            assert!(supports_json(&cmd), "`hoard {}`", argv.join(" "));
        }

        let refuses = [
            vec!["install"],
            vec!["upgrade"],
            vec!["login"],
            vec!["logout"],
            vec!["agents"],
            vec!["sync"],
            vec!["backup", "some-save"],
            vec!["save", "delete", "some-save"],
            vec!["save", "pause", "some-save"],
            vec!["snapshots", "delete", "some-save", "3"],
            vec!["snapshots", "max-versions"],
        ];
        for argv in refuses {
            let cmd = parse(&argv).command.unwrap();
            assert!(!supports_json(&cmd), "`hoard {}`", argv.join(" "));
        }
    }

    /// `snapshots list --json` is the contract. The insight fields come and go
    /// with the server, but a missing device is still a field, so a caller can
    /// tell "unknown machine" from "an older Hoard that never said".
    #[test]
    fn a_snapshot_row_without_insight_keeps_its_device_field() {
        let row = SnapshotRow {
            version_num: 4,
            file_count: 2,
            total_size_bytes: 2048,
            created_at: "2026-11-06T10:00:00Z".into(),
            device_name: None,
            state: "trash",
            save_name: None,
            changed_files: None,
        };
        let json = serde_json::to_value(&row).unwrap();
        let obj = json.as_object().unwrap();
        assert_eq!(obj["device_name"], serde_json::Value::Null);
        assert_eq!(obj["state"], "trash");
        assert!(!obj.contains_key("save_name"));
        assert!(!obj.contains_key("changed_files"));

        let with_insight = SnapshotRow {
            save_name: Some("Slot 1".into()),
            changed_files: Some(3),
            ..row
        };
        let json = serde_json::to_value(&with_insight).unwrap();
        assert_eq!(json["save_name"], "Slot 1");
        assert_eq!(json["changed_files"], 3);
    }

    #[test]
    fn sizes_switch_unit_at_each_power_of_1024() {
        assert_eq!(fmt_bytes(0), "0B");
        assert_eq!(fmt_bytes(1023), "1023B");
        assert_eq!(fmt_bytes(1024), "1.00K");
        assert_eq!(fmt_bytes(1536), "1.50K");
        assert_eq!(fmt_bytes(1024 * 1024), "1.00M");
        assert_eq!(fmt_bytes(1024 * 1024 * 1024), "1.00G");
        // There is no unit past G: a terabyte is still counted in G.
        assert_eq!(fmt_bytes(1024u64.pow(4)), "1024.00G");
    }
}
