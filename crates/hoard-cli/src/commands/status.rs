use anyhow::Result;

use crate::api::ApiClient;
use crate::config::CliConfig;

pub async fn run() -> Result<()> {
    let (cfg, _) = CliConfig::load_default()?;
    // Use whatever token we have (or empty) — /v1/health is unauthenticated.
    let token = cfg.auth.token.clone().unwrap_or_default();
    let client = ApiClient::new(cfg.server.url.clone(), token)?;
    let h = client.health().await?;
    println!(
        "server:  {}\nstatus:  {}\nversion: {}\nuptime:  {}s",
        cfg.server.url, h.status, h.version, h.uptime_secs
    );
    Ok(())
}
