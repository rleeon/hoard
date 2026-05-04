use anyhow::{bail, Result};
use clap::Subcommand;

use hoard_agent::config::CliConfig;

#[derive(Subcommand)]
pub enum ConfigCommand {
    /// Create a default config file at the standard location
    Init {
        /// Server URL to write into the new config
        #[arg(long, default_value = "http://localhost:8080")]
        server: String,
        /// Overwrite an existing config without asking
        #[arg(long)]
        force: bool,
    },
    /// Print the resolved config
    Show,
    /// Set a config field. Currently supported: server.url
    Set { key: String, value: String },
    /// Print the path of the config file (whether it exists or not)
    Path,
}

pub fn run(cmd: ConfigCommand) -> Result<()> {
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
                server: hoard_agent::config::ServerSection { url: server },
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
        ConfigCommand::Set { key, value } => {
            let path = CliConfig::default_path()?;
            let mut cfg = CliConfig::load(&path)?;
            match key.as_str() {
                "server.url" => cfg.server.url = value,
                other => bail!("unknown key: {other} (supported: server.url)"),
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
