use anyhow::Result;
use serde::Serialize;

use hoard_agent::api::ApiClient;
use hoard_agent::config::CliConfig;

use crate::output;

#[derive(Serialize)]
pub struct StatusOut {
    pub server: String,
    pub status: String,
    pub version: String,
    pub uptime_secs: u64,
}

pub async fn run() -> Result<()> {
    let (cfg, _) = CliConfig::load_default()?;
    // Use whatever token we have, or none: /v1/health is unauthenticated.
    let token = cfg.auth.token.clone().unwrap_or_default();
    let client = ApiClient::new(cfg.server.url.clone(), token)?;
    let h = client.health().await?;
    let out = StatusOut {
        server: cfg.server.url.clone(),
        status: h.status,
        version: h.version,
        uptime_secs: h.uptime_secs as u64,
    };
    output::emit(&out, |o| {
        println!(
            "server:  {}\nstatus:  {}\nversion: {}\nuptime:  {}s",
            o.server, o.status, o.version, o.uptime_secs
        );
    })
}
