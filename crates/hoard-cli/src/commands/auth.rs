use anyhow::Result;

use hoard_agent::api::ApiClient;
use hoard_agent::config::CliConfig;

pub async fn login(token: String) -> Result<()> {
    let path = CliConfig::default_path()?;
    let mut cfg = CliConfig::load(&path)?;

    // Validate the token against the server before saving.
    let client = ApiClient::new(cfg.server.url.clone(), token.clone())?;
    let me = client.whoami().await?;

    cfg.auth.token = Some(token);
    cfg.save(&path)?;
    println!(
        "logged in as {} (admin: {}) — token saved to {}",
        me.username,
        me.is_admin,
        path.display()
    );
    Ok(())
}

pub async fn logout() -> Result<()> {
    let path = CliConfig::default_path()?;
    let mut cfg = CliConfig::load(&path)?;
    cfg.auth.token = None;
    cfg.save(&path)?;
    println!("logged out (token cleared from {})", path.display());
    Ok(())
}

pub async fn whoami() -> Result<()> {
    let (cfg, _) = CliConfig::load_default()?;
    let token = cfg.require_token()?;
    let client = ApiClient::new(cfg.server.url.clone(), token)?;
    let me = client.whoami().await?;
    println!(
        "username: {}\nuser_id:  {}\nadmin:    {}",
        me.username, me.user_id, me.is_admin
    );
    Ok(())
}
