use anyhow::{bail, Result};
use clap::Subcommand;

use hoard_agent::config::CliConfig;
use hoard_agent::prefs::Prefs;

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
    /// Set a config field. Supported: server.url, and backup.when (`saving` or
    /// `closing`), the same switch as in the app's Settings
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
        }
        ConfigCommand::Set { key, value } if key == "backup.when" => {
            set_backup_when(&value).await?;
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
                other => bail!("unknown key: {other} (supported: server.url, backup.when)"),
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
