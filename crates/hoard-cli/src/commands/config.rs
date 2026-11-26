use anyhow::{bail, Result};
use clap::Subcommand;

use hoard_agent::config::CliConfig;
use hoard_agent::prefs::{fmt_interval, AutomaticTimer, ModeChange, Prefs, ServiceLink, SyncMode};
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
    /// or `off`), backup.when (`saving` or `closing`), and automatic mode's
    /// timers: detection.scan_interval (how often it looks for new games, 1m to
    /// 1h) and detection.backup_interval (how often it re-checks every save for
    /// changes, 5m to 24h), as `90s`, `10m`, `2h` or seconds
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
            let (mut cfg, path) = CliConfig::load_default()?;
            // The self-hosted token is a password: this output lands in
            // scrollback, screenshots and whatever an assistant reads.
            if cfg.auth.token.as_deref().is_some_and(|t| !t.is_empty()) {
                cfg.auth.token = Some("<hidden>".to_string());
            }
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
            println!(
                "detection.scan_interval = \"{}\"",
                fmt_interval(prefs.automatic_scan_interval_secs)
            );
            println!(
                "detection.backup_interval = \"{}\"",
                fmt_interval(prefs.automatic_backup_interval_secs)
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
        ConfigCommand::Set { key, value } if key == "detection.scan_interval" => {
            set_interval(AutomaticTimer::Scan, &value).await?;
        }
        ConfigCommand::Set { key, value } if key == "detection.backup_interval" => {
            set_interval(AutomaticTimer::Backup, &value).await?;
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
                    "unknown key: {other} (supported: server.url, sync.mode, detection.auto, \
                     backup.when, detection.scan_interval, detection.backup_interval)"
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

/// `detection.scan_interval` / `detection.backup_interval`: the sliders under
/// automatic mode in the app's Settings, through the same rule.
async fn set_interval(timer: AutomaticTimer, value: &str) -> Result<()> {
    let Some(secs) = parse_interval(value) else {
        bail!("`{value}` isn't a duration; write it as `90s`, `10m`, `2h` or a number of seconds");
    };
    let (mut prefs, path) = Prefs::load_for_cli_edit()?;
    prefs.set_automatic_interval(timer, secs)?;
    prefs.save(&path)?;
    let what = match timer {
        AutomaticTimer::Scan => "automatic mode looks for new games",
        AutomaticTimer::Backup => "automatic mode re-checks every save",
    };
    if prefs.automatic_mode {
        let applied = crate::commands::link::notify_automatic_changed().await;
        println!("{what} every {} ({applied})", fmt_interval(secs));
    } else {
        println!(
            "{what} every {} once it's on (`hoard config set detection.auto on`)",
            fmt_interval(secs)
        );
    }
    Ok(())
}

/// `90s`, `10m`, `2h`, `1d` or plain seconds. `1h30m` too.
fn parse_interval(value: &str) -> Option<u64> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    if let Ok(secs) = value.parse::<u64>() {
        return Some(secs);
    }
    let mut total = 0u64;
    let mut digits = String::new();
    for c in value.chars() {
        if c.is_ascii_digit() {
            digits.push(c);
            continue;
        }
        let n: u64 = digits.parse().ok()?;
        digits.clear();
        let unit = match c {
            's' => 1,
            'm' => 60,
            'h' => 3600,
            'd' => 86400,
            _ => return None,
        };
        total = total.checked_add(n.checked_mul(unit)?)?;
    }
    // Digits left over with no unit after them (`1h30`) are a typo, not seconds.
    digits.is_empty().then_some(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intervals_read_the_way_people_write_them() {
        assert_eq!(parse_interval("600"), Some(600));
        assert_eq!(parse_interval("90s"), Some(90));
        assert_eq!(parse_interval("10m"), Some(600));
        assert_eq!(parse_interval("2h"), Some(7200));
        assert_eq!(parse_interval("1d"), Some(86400));
        assert_eq!(parse_interval("1h30m"), Some(5400));
        assert_eq!(parse_interval(" 5m "), Some(300));
        assert_eq!(parse_interval("1h30"), None);
        assert_eq!(parse_interval("ten minutes"), None);
        assert_eq!(parse_interval(""), None);
    }
}
