use anyhow::{bail, Result};
use clap::Subcommand;

use hoard_agent::config::CliConfig;
use hoard_agent::prefs::{ModeChange, Prefs, ServiceLink, SyncMode};
use hoard_core::ipc::Request;

#[derive(Subcommand)]
pub enum ConfigCommand {
    /// Create a default config file at the standard location
    Init {
        /// Server URL to write into the new config
        #[arg(long, default_value = "http://localhost:12421")]
        server: String,
        /// Overwrite an existing config without asking
        #[arg(long)]
        force: bool,
    },
    /// Print the resolved config
    Show,
    /// Set a config field. Supported: server.url; and the app's Settings
    /// switches: sync.mode (`manual`, `backup` or `sync`), detection.auto (`on`
    /// or `off`) and backup.when (`saving` or `closing`)
    Set { key: String, value: String },
    /// Print the path of the config file (whether it exists or not)
    Path,
}

pub async fn run(cmd: ConfigCommand) -> Result<()> {
    match cmd {
        ConfigCommand::Init { server, force } => {
            let path = CliConfig::default_path()?;
            if path.exists() && !force {
                bail!(
                    "config already exists at {} (pass --force to overwrite)",
                    path.display()
                );
            }
            let cfg = CliConfig {
                server: hoard_agent::config::ServerSection {
                    url: hoard_agent::serverclass::normalize_server_url(&server),
                },
                auth: Default::default(),
            };
            cfg.save(&path)?;
            println!("wrote {}", path.display());
        }
        ConfigCommand::Show => {
            let (cfg, path) = CliConfig::load_default()?;
            println!("# {}", path.display());
            print!("{}", toml::to_string_pretty(&cfg)?);
            let (prefs, path) = Prefs::load_for_cli_edit()?;
            println!();
            println!("# {} (the app's Settings)", path.display());
            println!("sync.mode = \"{}\"", mode_name(prefs.sync_mode()));
            println!(
                "detection.auto = \"{}\"",
                if prefs.automatic_mode { "on" } else { "off" }
            );
            println!(
                "backup.when = \"{}\"",
                if prefs.backup_on_close { "closing" } else { "saving" }
            );
        }
        ConfigCommand::Set { key, value } if key == "backup.when" => {
            set_backup_when(&value).await?;
        }
        ConfigCommand::Set { key, value } if key == "sync.mode" => {
            set_sync_mode(&value).await?;
        }
        ConfigCommand::Set { key, value } if key == "detection.auto" => {
            set_detection_auto(&value).await?;
        }
        ConfigCommand::Set { key, value } => {
            let path = CliConfig::default_path()?;
            let mut cfg = CliConfig::load(&path)?;
            match key.as_str() {
                // A `user@` here would end up as an HTTP Basic header that
                // shadows the access key on every request: a 401 that blames the
                // token. Clean it on the way in, so what lands in config.toml is
                // what the client will actually talk to.
                "server.url" => {
                    cfg.server.url = hoard_agent::serverclass::normalize_server_url(&value)
                }
                other => bail!(
                    "unknown key: {other} (supported: server.url, sync.mode, detection.auto, backup.when)"
                ),
            }
            cfg.save(&path)?;
            println!("updated {}", path.display());
        }
        ConfigCommand::Path => {
            println!("{}", CliConfig::default_path()?.display());
        }
    }
    Ok(())
}

fn mode_name(mode: SyncMode) -> &'static str {
    match mode {
        SyncMode::Manual => "manual",
        SyncMode::BackupOnly => "backup",
        SyncMode::FullSync => "sync",
    }
}

/// The service as the CLI reaches it. None running is fine: it reads the file
/// when it starts.
struct CliLink(Option<hoardd::client::Client>);

impl ServiceLink for CliLink {
    async fn send(&mut self, request: Request) -> Result<bool> {
        let Some(client) = self.0.as_mut() else {
            return Ok(false);
        };
        crate::commands::link::ask(client, request).await.map(|_| true)
    }
}

fn applied(live: bool) -> &'static str {
    if live {
        "the sync service picked it up"
    } else {
        "it applies when the sync service starts"
    }
}

/// `sync.mode`: the app's Settings mode picker, through the same function.
async fn set_sync_mode(value: &str) -> Result<()> {
    let mode = match value {
        "manual" => SyncMode::Manual,
        "backup" => SyncMode::BackupOnly,
        "sync" => SyncMode::FullSync,
        other => bail!("sync.mode is `manual`, `backup` or `sync`, not `{other}`"),
    };
    let (current, path) = Prefs::load_for_cli_edit()?;
    let mut link = CliLink(crate::commands::link::attached("config").await);
    match hoard_agent::prefs::change_sync_mode(&path, current, mode, &mut link).await? {
        ModeChange::Applied { live, .. } => {
            let said = match mode {
                SyncMode::Manual => "nothing goes up or comes down unless you ask",
                SyncMode::BackupOnly => "saves go up on their own; restoring is up to you",
                SyncMode::FullSync => "saves go up and come down on their own",
            };
            println!("sync mode: {} ({said}; {})", mode_name(mode), applied(live));
            Ok(())
        }
        ModeChange::ServiceTooOld => bail!(
            "the Hoard service on this machine is too old for the manual mode; nothing changed. Update Hoard (`hoard upgrade`) and try again"
        ),
    }
}

/// `detection.auto`: the automatic mode, the periodic scan that adds new games.
/// Not part of the sync mode: it runs in the manual one too.
async fn set_detection_auto(value: &str) -> Result<()> {
    let on = match value {
        "on" => true,
        "off" => false,
        other => bail!("detection.auto is `on` or `off`, not `{other}`"),
    };
    let (current, path) = Prefs::load_for_cli_edit()?;
    let mut link = CliLink(crate::commands::link::attached("config").await);
    let (_, live) = hoard_agent::prefs::change_automatic_mode(&path, current, on, &mut link).await?;
    let said = if on { "on" } else { "off" };
    println!("automatic detection: {said} ({})", applied(live));
    Ok(())
}

/// `backup.when` lives in the prefs file the app reads too, not in
/// `config.toml`: it is the switch in Settings, and the engine takes it from
/// there when it starts.
async fn set_backup_when(value: &str) -> Result<()> {
    let enabled = match value {
        "closing" => true,
        "saving" => false,
        other => bail!("backup.when is `saving` or `closing`, not `{other}`"),
    };
    let (mut prefs, path) = Prefs::load_for_cli_edit()?;
    prefs.backup_on_close = enabled;
    prefs.save(&path)?;
    let applied = crate::commands::link::notify_backup_on_close(enabled).await;
    let said = if enabled { "closes" } else { "saves" };
    println!("saves back up when the game {said} ({applied})");
    Ok(())
}
