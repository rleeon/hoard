use anyhow::Result;
use serde::Serialize;

use hoard_agent::api::ApiClient;
use hoard_agent::cloud_auth;
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
    // With a Cloud session the server is Cloud. The self-hosted URL defaults to
    // localhost:12421, where nothing listens on a Cloud-only machine: asking it
    // reads "connection refused" while sync is fine. /v1/health is
    // unauthenticated, and the self-hosted token has no business going to Cloud.
    let (server, token) = match cloud_auth::load_session()? {
        Some(sess) => (sess.server_url, String::new()),
        None => (cfg.server.url.clone(), cfg.auth.token.clone().unwrap_or_default()),
    };
    let client = ApiClient::new(server.clone(), token)?;
    let h = client.health().await?;
    let out = StatusOut {
        server,
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
