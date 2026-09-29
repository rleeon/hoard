use anyhow::Result;
use serde::Serialize;

use hoard_agent::api::ApiClient;
use hoard_agent::cloud_auth;
use hoard_agent::config::CliConfig;

use crate::commands::link;
use crate::output;

#[derive(Serialize)]
pub struct StatusOut {
    pub server: String,
    pub status: String,
    pub version: String,
    pub uptime_secs: u64,
}

pub async fn run() -> Result<()> {
    // The server in use, which is not always the one in `config.toml`: with a
    // Cloud session it is Cloud, and a self-hosted sign-in from the app lives
    // with the service. The config's URL defaults to localhost:12421, where
    // nothing listens on a Cloud-only machine, so asking it read "connection
    // refused" while sync was fine.
    let server = match cloud_auth::load_session()? {
        Some(sess) => sess.server_url,
        None => match link::borrow_server_session().await {
            Some(s) => s.server_url,
            None => CliConfig::load_default()?.0.server.url,
        },
    };
    // /v1/health is unauthenticated.
    let client = ApiClient::new(server.clone(), "")?;
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
