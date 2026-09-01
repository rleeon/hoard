//! `hoard login` / `logout` / `whoami`.
//!
//! Two session kinds, never both effectively at once (Cloud wins over self-host
//! when resolving, see `session::resolve`):
//! - Cloud (default, bare `hoard login`): Supabase without a browser, so email
//!   plus password, or an email OTP code if you leave the password blank.
//! - self-host (`hoard login --token <token>`): the server's bearer token.

use std::io::{self, IsTerminal, Write};
use std::time::{Duration, Instant};

use anyhow::{bail, Result};

use hoard_agent::api::ApiClient;
use hoard_agent::cloud_auth;
use hoard_agent::config::CliConfig;
use hoard_agent::state;

use crate::commands::link;

pub async fn login(token: Option<String>, server: Option<String>, force_email: bool) -> Result<()> {
    // Explicit flags skip the menu and stay scriptable.
    if let Some(token) = token {
        return login_selfhost(token, server).await;
    }
    if force_email {
        return login_cloud_email(&cloud_auth::cloud_base_url()).await;
    }

    // No flags: ask what to sign in to, but only if there's a terminal to ask
    // on. Piped, CI and service contexts can't answer a prompt, so default to
    // Cloud (its device/email flow doesn't depend on an interactive menu).
    if !io::stdin().is_terminal() {
        return login_cloud(false).await;
    }
    match choose_kind()? {
        Kind::Cloud => login_cloud(false).await,
        Kind::SelfHost => login_selfhost_interactive().await,
    }
}

enum Kind {
    Cloud,
    SelfHost,
}

/// Interactive top-level choice: Hoard Cloud or a self-hosted server.
fn choose_kind() -> Result<Kind> {
    println!("Where do you want to sign in?");
    println!("   1) Hoard Cloud          (managed, no browser)");
    println!("   2) Self-hosted server   (your own Hoard server + access token)");
    println!();
    loop {
        match prompt("Pick [1-2, default 1]: ")?.as_str() {
            "" | "1" => return Ok(Kind::Cloud),
            "2" => return Ok(Kind::SelfHost),
            _ => println!("Type 1 or 2."),
        }
    }
}

/// self-host: validates the bearer against the server and saves it in the config.
/// `server` overrides the configured URL (and is persisted) when given.
async fn login_selfhost(token: String, server: Option<String>) -> Result<()> {
    let path = CliConfig::default_path()?;
    let mut cfg = CliConfig::load(&path)?;
    if let Some(url) = server {
        cfg.server.url = url.trim().to_string();
    }

    let client = ApiClient::new(cfg.server.url.clone(), token.clone())?;
    let me = client.whoami().await?;

    // To the service, which owns the store the engine resolves from (D.20).
    // Without this, on a machine where the app already had a session, this login
    // would not change which server the engine syncs with.
    let handed = link::hand_over_server_session(hoard_core::ipc::ServerSession {
        server_url: cfg.server.url.clone(),
        token: token.clone(),
        user: Some(hoard_core::ipc::ServerUser {
            user_id: me.user_id.clone(),
            username: me.username.to_string(),
            is_admin: me.is_admin,
        }),
    })
    .await;

    // And to `config.toml` as well: it is the headless path and what this same
    // CLI's one-shots read with no service in front. Plain text and 0600, as
    // always; there is no keyring to poison here.
    cfg.auth.token = Some(token);
    cfg.save(&path)?;
    if !handed {
        // The service did not hear about it through the handover. On top of
        // that, whatever session the app may have stored has to go: the engine
        // prefers it, so leaving it there would make this login pointless.
        let _ = hoard_agent::credentials::forget_unlocked();
        link::notify_session_changed().await;
    }
    println!(
        "connected to self-host ({}) as {} (admin: {}) — saved to {}",
        cfg.server.url,
        me.username,
        me.is_admin,
        path.display()
    );
    Ok(())
}

/// self-host from the interactive menu: asks for the server URL (defaulting to
/// whatever's in the config) and the access token, then validates and saves.
async fn login_selfhost_interactive() -> Result<()> {
    let (cfg, _) = CliConfig::load_default()?;
    let default_url = cfg.server.url.clone();

    let url = prompt(&format!("Server URL [{default_url}]: "))?;
    let url = if url.is_empty() { default_url } else { url };

    let token = prompt("Access token (hoard_v1_…): ")?;
    if token.is_empty() {
        anyhow::bail!("no access token given");
    }
    login_selfhost(token, Some(url)).await
}

/// Cloud without a browser. The main path is mobile pairing: the CLI shows a URL
/// and a code, you approve it from your phone (already signed in on the web) and
/// the server mints the session. If the server doesn't have it configured, it
/// falls back to the email plus password or OTP-code path. The session is stored
/// where the desktop keeps it (keyring plus `cloud.toml`) to share the login.
async fn login_cloud(force_email: bool) -> Result<()> {
    let base = cloud_auth::cloud_base_url();
    println!("Sign in to Hoard Cloud ({base})");

    // `--email`: skip phone pairing (which just re-approves whatever account the
    // phone browser already has) and let the user type the address to sign in as.
    if force_email {
        return login_cloud_email(&base).await;
    }

    match cloud_auth::device_start(local_hostname().as_deref()).await? {
        // Server with the feature: mobile pairing.
        Some(start) => return login_cloud_device(&base, start).await,
        // Server without the feature (older version or Cloud without
        // service_role): fall back to email. A real network failure still
        // propagates (the `?` above).
        None => println!("(mobile pairing unavailable on the server; using email)"),
    }
    login_cloud_email(&base).await
}

/// Mobile pairing: shows a URL + code and polls until it's approved from the
/// phone.
async fn login_cloud_device(base: &str, start: cloud_auth::DeviceStart) -> Result<()> {
    let link = start
        .verification_uri_complete
        .as_deref()
        .unwrap_or(&start.verification_uri);
    println!();
    println!("  Open this on your phone (signed in to Hoard Cloud):");
    println!("    {link}");
    println!("  and confirm the code:  {}", start.user_code);
    println!();
    print!("Waiting for approval");
    io::stdout().flush().ok();

    let interval = Duration::from_secs(start.interval_secs.max(1));
    let ttl = if start.expires_in_secs == 0 {
        600
    } else {
        start.expires_in_secs
    };
    let deadline = Instant::now() + Duration::from_secs(ttl);

    let tokens = loop {
        if Instant::now() >= deadline {
            println!();
            anyhow::bail!("the code expired before it was approved. Retry with `hoard login`.");
        }
        tokio::time::sleep(interval).await;
        match cloud_auth::device_poll(&start.device_code).await? {
            cloud_auth::DeviceStatus::Approved(t) => {
                println!("\nApproved.");
                break t;
            }
            cloud_auth::DeviceStatus::Denied => {
                println!();
                anyhow::bail!("pairing rejected from the phone.");
            }
            cloud_auth::DeviceStatus::Expired => {
                println!();
                anyhow::bail!("the code expired. Retry with `hoard login`.");
            }
            cloud_auth::DeviceStatus::Pending => {
                print!(".");
                io::stdout().flush().ok();
            }
        }
    };

    finish_cloud_login(base, &tokens).await
}

/// Browserless fallback: email + password, or an email OTP code if you leave the
/// password blank.
async fn login_cloud_email(base: &str) -> Result<()> {
    let email = prompt("Email: ")?;
    if email.is_empty() {
        anyhow::bail!("empty email");
    }
    let password = prompt_hidden("Password (blank = I'll email you a code): ")?;

    let tokens = if password.trim().is_empty() {
        cloud_auth::otp_start(&email).await?;
        println!(
            "Sent a code to {email}. Open it wherever you read your mail (phone, etc.) \
             and type it here."
        );
        let code = prompt("Code: ")?;
        cloud_auth::otp_verify(&email, &code).await?
    } else {
        cloud_auth::login_password(&email, &password).await?
    };

    finish_cloud_login(base, &tokens).await
}

/// Persists the session and sets the Cloud context. Shared by both paths.
///
/// Minting a session is not rotating one, so logging in is the client's job, but
/// *storing* it is the service's (D.20): the keyring item only authorises the
/// binary that creates it, and what reads it on every engine start is `hoardd`.
/// It gets handed over by IPC, and that alone tells the service the session
/// changed, so no separate notice is needed.
async fn finish_cloud_login(base: &str, tokens: &cloud_auth::Tokens) -> Result<()> {
    // The handover includes forgetting the previous one, so signing in with
    // another account does not leave its `user` or `server_url` on disk.
    let handed = link::hand_over_session(hoard_core::ipc::AdoptedSession {
        server_url: base.to_string(),
        access_token: tokens.access.clone(),
        refresh_token: tokens.refresh.clone(),
    })
    .await;
    if !handed {
        // With no service: to the 0600 file, and NOT to the keyring. The service
        // picks it up from there on start and puts it in the keyring itself, as
        // its owner. Writing it here is what left the engine asking permission.
        cloud_auth::forget_tokens_unlocked()?;
        cloud_auth::store_tokens_unlocked(tokens, base)?;
    }
    let me = cloud_auth::fetch_me(base, &tokens.access).await?;
    state::set_active_context(Some(state::cloud_context(&me.user_id)));
    if !handed {
        // The service did not hear about the login through the handover, so tell
        // it in case it started in between.
        link::notify_session_changed().await;
    }
    println!(
        "connected to Hoard Cloud as {} · plan {}",
        me.email, me.plan
    );
    Ok(())
}

/// Machine name, best-effort, so the phone shows what it's authorizing.
fn local_hostname() -> Option<String> {
    for key in ["HOSTNAME", "COMPUTERNAME", "HOST"] {
        if let Ok(v) = std::env::var(key) {
            let v = v.trim().to_string();
            if !v.is_empty() {
                return Some(v);
            }
        }
    }
    std::fs::read_to_string("/etc/hostname")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub async fn logout() -> Result<()> {
    let had_cloud = cloud_auth::load_session()?.is_some();
    // The pair is deleted by its owner, the service. With no service, removing
    // the session file is enough: without it there is no session to resolve.
    let cloud_forgotten_by_service = had_cloud && link::hand_over_logout().await;
    if had_cloud && !cloud_forgotten_by_service {
        cloud_auth::forget_tokens_unlocked()?;
    }

    let path = CliConfig::default_path()?;
    let mut cfg = CliConfig::load(&path)?;
    // The two self-hosted sources: this CLI's `config.toml` and the session store
    // the service keeps, where the app's lives. A logout has to close both or the
    // machine would carry on syncing.
    let stored_selfhost = hoard_agent::credentials::load_public()?.is_some();
    let had_selfhost = cfg.auth.token.is_some() || stored_selfhost;
    if cfg.auth.token.is_some() {
        cfg.auth.token = None;
        cfg.save(&path)?;
    }
    let selfhost_forgotten_by_service = stored_selfhost && link::hand_over_server_logout().await;
    if stored_selfhost && !selfhost_forgotten_by_service {
        hoard_agent::credentials::forget_unlocked()?;
    }

    // The credentials are gone, so have the service drop the engine and resolve
    // afresh rather than carry on with a token we just deleted. `ForgetSession`
    // already carries that, so it only needs telling when it did not go that
    // way.
    if (had_selfhost && !selfhost_forgotten_by_service)
        || (had_cloud && !cloud_forgotten_by_service)
    {
        link::notify_session_changed().await;
    }

    match (had_cloud, had_selfhost) {
        (false, false) => println!("no active session"),
        (true, false) => println!("Cloud session closed"),
        (false, true) => println!("self-host session closed"),
        (true, true) => println!("Cloud and self-host sessions closed"),
    }
    Ok(())
}

/// The signed-in session. Two shapes, tagged: an agent branches on `kind`
/// instead of guessing from which fields happen to be present.
#[derive(serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WhoamiOut {
    Cloud {
        email: String,
        user_id: String,
        plan: String,
        /// Raw bytes, not the human "1,2 GB": agents compare numbers.
        storage_used_bytes: i64,
        storage_limit_bytes: i64,
    },
    SelfHost {
        server: String,
        username: String,
        user_id: String,
        is_admin: bool,
    },
}

pub async fn whoami() -> Result<()> {
    let out = if cloud_auth::load_session()?.is_some() {
        // A token lent by the service, not refreshed here: two processes rotating
        // the same refresh token is the reuse detection that revokes the whole
        // session (ADR 0021, part A).
        let active = link::resolve_session().await?;
        let Some(sess) = active.cloud else {
            bail!("the stored Cloud session is unreadable — run `hoard login`");
        };
        let me = cloud_auth::fetch_me(&sess.server_url, &sess.access).await?;
        WhoamiOut::Cloud {
            email: me.email,
            user_id: me.user_id.to_string(),
            plan: me.plan,
            storage_used_bytes: me.storage_used_bytes,
            storage_limit_bytes: me.storage_limit_bytes,
        }
    } else {
        let (cfg, _) = CliConfig::load_default()?;
        let token = crate::output::require_token(&cfg)?;
        let client = ApiClient::new(cfg.server.url.clone(), token)?;
        let me = client.whoami().await?;
        WhoamiOut::SelfHost {
            server: cfg.server.url.clone(),
            username: me.username.to_string(),
            user_id: me.user_id.to_string(),
            is_admin: me.is_admin,
        }
    };

    crate::output::emit(&out, |out| match out {
        WhoamiOut::Cloud {
            email,
            user_id,
            plan,
            storage_used_bytes,
            storage_limit_bytes,
        } => println!(
            "Hoard Cloud\n  email:   {}\n  user_id: {}\n  plan:    {}\n  usage:   {} / {}",
            email,
            user_id,
            plan,
            fmt_bytes(*storage_used_bytes),
            fmt_bytes(*storage_limit_bytes),
        ),
        WhoamiOut::SelfHost {
            server,
            username,
            user_id,
            is_admin,
        } => println!(
            "self-host ({})\n  username: {}\n  user_id:  {}\n  admin:    {}",
            server, username, user_id, is_admin
        ),
    })
}

fn prompt(label: &str) -> Result<String> {
    print!("{label}");
    io::stdout().flush()?;
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    Ok(buf.trim().to_string())
}

fn prompt_hidden(label: &str) -> Result<String> {
    // Without a tty (pipe, CI) rpassword reads from stdin without hiding; fine.
    Ok(rpassword::prompt_password(label)?)
}

fn fmt_bytes(b: i64) -> String {
    if b <= 0 {
        return "0".to_string();
    }
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
