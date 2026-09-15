//! Discord status channel, driven by the server itself.
//!
//! Keeps one embed in a Discord channel up to date with this instance's
//! health and renames the channel `🟢-status` / `🟡-status` to match. It
//! replaces an external Python bot that polled `/v1/health` over the public
//! internet from a separate host.
//!
//! Two consequences of living inside the process it reports on, both
//! deliberate:
//!
//!   * There is no red. A process that has stopped cannot post "I stopped",
//!     so the states are `ok` and `degraded` (up, but Postgres is failing).
//!     A hard outage shows as an embed that stops refreshing; the timestamp
//!     in the footer is what gives it away, which is why every tick rewrites
//!     it even when nothing else changed.
//!   * The health probe is a local `SELECT 1`, not an HTTP round trip to
//!     ourselves. Nothing traverses the proxy, so this task never counts as
//!     traffic and never keeps a machine awake on its own.
//!
//! Below the status, two failure counts for the last 24 hours: what the server
//! counted itself (`incidents`: 5xx by upload/download/delete, and background
//! jobs that delete user data failing), and what the apps reported through
//! `client_logs`, counted in affected users rather than lines.
//!
//! Off unless `[cloud.discord]` carries both a bot token and a channel id
//! (`HOARD__CLOUD__DISCORD__*`), so a fresh deploy and every self-hosted
//! instance simply skip it.

use crate::cloud::incidents::{self, Kind};
use crate::cloud::state::CloudState;
use crate::config::DiscordConfig;
use anyhow::{Context, Result};
use reqwest::{Method, RequestBuilder, StatusCode};
use std::time::{Duration, Instant};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

const API: &str = "https://discord.com/api/v10";

/// Below this the channel-rename budget (2 per 10 minutes) is the binding
/// constraint, not the poll. Guards against a config of `0`, which would
/// panic `tokio::time::interval`.
const MIN_POLL_SECS: u64 = 15;

/// How often `client_logs` is read. It scans a day of rows, and a day-long
/// window doesn't go stale in five minutes.
const APPS_REFRESH: Duration = Duration::from_secs(5 * 60);

/// The error window. The server half is in memory, so a younger process
/// reports "since boot" instead of pretending to a full day.
const ERROR_WINDOW: Duration = Duration::from_secs(24 * 60 * 60);

/// True when the config has enough to talk to Discord at all.
pub fn is_configured(cfg: &DiscordConfig) -> bool {
    !cfg.bot_token.is_empty() && cfg.channel_id != 0
}

/// Start the status task. A no-op when unconfigured.
pub fn spawn(state: CloudState) {
    let Some(cfg) = state.config.cloud.as_ref().map(|c| c.discord.clone()) else {
        return;
    };
    if !is_configured(&cfg) {
        return;
    }
    tokio::spawn(async move { run(state, cfg).await });
}

// ---------------------------------------------------------------------------
// Health
// ---------------------------------------------------------------------------

/// Same probe and the same 2s budget as the `/v1/health` handler, so the
/// channel and the endpoint can never disagree about what "degraded" means.
async fn probe(state: &CloudState) -> &'static str {
    let db_ok = matches!(
        tokio::time::timeout(
            Duration::from_secs(2),
            sqlx::query("SELECT 1").execute(&state.pool),
        )
        .await,
        Ok(Ok(_))
    );
    if db_ok {
        "ok"
    } else {
        "degraded"
    }
}

fn embed(status: &str, errors: &Errors) -> serde_json::Value {
    let (emoji, label, colour, description) = match status {
        "ok" => ("🟢", "ONLINE", 0x00FF00, "All systems operational."),
        _ => (
            "🟡",
            "DEGRADED",
            0xFFFF00,
            "The server is up, but the database is failing.",
        ),
    };
    let now = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_default();

    serde_json::json!({
        "title": format!("{emoji} Hoard Server — {label}"),
        "description": description,
        "color": colour,
        "timestamp": now,
        "fields": [
            { "name": "Status", "value": format!("`{status}`"), "inline": true },
            { "name": "Version", "value": format!("`{}`", env!("CARGO_PKG_VERSION")), "inline": true },
            { "name": "Errors (24h) · server", "value": server_errors_text(&errors.server, errors.since_boot), "inline": false },
            { "name": "Errors (24h) · apps", "value": app_errors_text(errors.apps.as_ref()), "inline": false },
        ],
        "footer": { "text": "Hoard Cloud Status" },
    })
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// The failure half of the embed.
#[derive(Debug, Clone, Default)]
struct Errors {
    server: incidents::Snapshot,
    /// `None` until the first successful read of `client_logs`.
    apps: Option<AppFailures>,
    /// Set while the process is younger than the window. The server count
    /// lives in memory and a restart zeroes it, so a clean sheet might only
    /// mean a fresh process, and the reader should be told.
    since_boot: Option<Duration>,
}

/// One kind of app-side failure over the window: log lines, and the distinct
/// accounts they came from. Users is the number that matters: one machine
/// whose ISP can't reach the bucket retries hundreds of times a day and would
/// drown every other signal if lines were the unit.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct AppCount {
    events: i64,
    users: i64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct AppFailures {
    upload: AppCount,
    download: AppCount,
    crash: AppCount,
}

/// App failures that mean a save didn't go up, didn't come down, or the app
/// itself fell over. Matched on the exact messages the apps log, so a reworded
/// log line in `hoard-agent` (`agent.rs`, `backup.rs`, `restore.rs`) or in
/// `hoard-desktop` (`commands/cloud_pull.rs`) silently drops out of here.
///
/// Deliberately left out, because they are the app doing its job or the
/// user's own machine: storage full and per-save cap (plan limits), a file
/// that changed mid-upload or can't be read (retried, local), the keyring
/// fallback, the fs watcher, WebView2, and "couldn't observe the cloud head",
/// which fires on every client during every deploy.
const APP_FAILURES_SQL: &str = "
SELECT count(*) FILTER (WHERE up),
       count(DISTINCT user_id) FILTER (WHERE up),
       count(*) FILTER (WHERE down),
       count(DISTINCT user_id) FILTER (WHERE down),
       count(*) FILTER (WHERE crash),
       count(DISTINCT user_id) FILTER (WHERE crash)
  FROM (SELECT user_id,
               (message = 'agent: backup attempt failed'
                  AND coalesce(fields->>'error', '') NOT LIKE '%changed while it was being%'
                  AND coalesce(fields->>'error', '') NOT LIKE 'hashing %')
               OR message IN ('agent: backup conflict; reconcile failed, surfacing',
                              'agent: backup parked, the storage endpoint can''t be reached from this machine')
                 AS up,
               message IN ('agent: auto-restore failed',
                           'cloud-pull: couldn''t read response body',
                           'cloud-pull: couldn''t parse manifest',
                           'cloud-pull: non-2xx response')
                 AS down,
               (message LIKE 'PANIC%' OR message LIKE 'supervisor: loop panicked%') AS crash
          FROM client_logs
         WHERE received_at > now() - interval '24 hours'
           AND level IN ('error', 'warn', 'ERROR', 'WARN')) t";

async fn app_failures(state: &CloudState) -> Result<AppFailures, sqlx::Error> {
    let r: (i64, i64, i64, i64, i64, i64) = sqlx::query_as(APP_FAILURES_SQL)
        .fetch_one(&state.pool)
        .await?;
    Ok(AppFailures {
        upload: AppCount {
            events: r.0,
            users: r.1,
        },
        download: AppCount {
            events: r.2,
            users: r.3,
        },
        crash: AppCount {
            events: r.4,
            users: r.5,
        },
    })
}

fn server_errors_text(s: &incidents::Snapshot, since_boot: Option<Duration>) -> String {
    let mut out = format!(
        "Uploads **{}** · Downloads **{}** · Deletes **{}** · Other **{}**",
        s.count(Kind::Upload),
        s.count(Kind::Download),
        s.count(Kind::Delete),
        s.count(Kind::Other),
    );
    if let Some((at, what)) = s.latest() {
        out.push_str(&format!("\nLast: `{what}` at {} UTC", hhmm(*at)));
    }
    if let Some(up) = since_boot {
        let mins = up.as_secs() / 60;
        out.push_str(&format!(
            "\n_Counting since boot, {}h {:02}m ago._",
            mins / 60,
            mins % 60
        ));
    }
    out
}

fn app_errors_text(apps: Option<&AppFailures>) -> String {
    let Some(a) = apps else {
        return "_No data yet._".to_owned();
    };
    let line = |label: &str, c: AppCount| {
        if c.users == 0 {
            return format!("{label}: **0**");
        }
        let who = if c.users == 1 { "user" } else { "users" };
        let what = if c.events == 1 { "failure" } else { "failures" };
        format!("{label}: **{}** {who} ({} {what})", c.users, c.events)
    };
    format!(
        "{}\n{}\n{}",
        line("Uploads", a.upload),
        line("Downloads", a.download),
        line("Crashes", a.crash)
    )
}

fn hhmm(unix: u64) -> String {
    OffsetDateTime::from_unix_timestamp(unix as i64)
        .ok()
        .and_then(|t| {
            t.format(time::macros::format_description!("[hour]:[minute]"))
                .ok()
        })
        .unwrap_or_else(|| "?".to_owned())
}

/// Discord slugifies channel names: ask for `🟢 status` and it stores
/// `🟢-status`. Generating the hyphen here is what stops the comparison in
/// the loop from failing every single tick and burning the rename budget.
fn channel_name_for(status: &str) -> String {
    let emoji = if status == "ok" { "🟢" } else { "🟡" };
    format!("{emoji}-status")
}

// ---------------------------------------------------------------------------
// Discord REST
// ---------------------------------------------------------------------------

struct Discord {
    http: reqwest::Client,
    token: String,
    channel: u64,
}

impl Discord {
    fn new(cfg: &DiscordConfig) -> Self {
        // Built once and kept, rather than per tick: the machine has 256 MB
        // and memwatch bounces it at 94%, so a fresh TLS config and root
        // store every minute is allocator churn with nothing to show for it.
        // The timeout matters more than it looks: without one, a Discord
        // call that never answers parks this task forever, silently.
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .pool_max_idle_per_host(1)
            .build()
            .unwrap_or_default();
        Self {
            http,
            token: cfg.bot_token.clone(),
            channel: cfg.channel_id,
        }
    }

    fn req(&self, method: Method, url: String) -> RequestBuilder {
        self.http
            .request(method, url)
            .header("Authorization", format!("Bot {}", self.token))
    }

    /// Execute, and on a 429 wait out `retry_after` and try once more.
    /// Discord's rename bucket is 2 per 10 minutes, so the wait can be
    /// minutes long; sleeping the task is correct, there is nothing else for
    /// it to do and the next tick would hit the same wall.
    async fn send(&self, rb: RequestBuilder) -> Result<reqwest::Response> {
        let retry = rb.try_clone();
        let resp = rb.send().await.context("discord request")?;
        if resp.status() != StatusCode::TOO_MANY_REQUESTS {
            return Ok(resp);
        }
        let body: serde_json::Value = resp.json().await.unwrap_or_default();
        let wait = body
            .get("retry_after")
            .and_then(|v| v.as_f64())
            .filter(|w| w.is_finite())
            .unwrap_or(5.0)
            .clamp(0.0, 900.0);
        tracing::debug!(wait, "discord status: rate limited, backing off");
        tokio::time::sleep(Duration::from_secs_f64(wait)).await;
        match retry {
            Some(rb) => rb.send().await.context("discord request (retry)"),
            None => anyhow::bail!("rate limited and the request could not be retried"),
        }
    }

    async fn json(&self, rb: RequestBuilder) -> Result<serde_json::Value> {
        let resp = self.send(rb).await?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            anyhow::bail!("discord returned {status}: {body}");
        }
        Ok(serde_json::from_str(&body).unwrap_or_default())
    }

    /// Our own user id, needed to tell our messages from everyone else's.
    async fn me(&self) -> Result<String> {
        let v = self
            .json(self.req(Method::GET, format!("{API}/users/@me")))
            .await?;
        v.get("id")
            .and_then(|v| v.as_str())
            .map(str::to_owned)
            .context("users/@me has no id")
    }

    async fn channel_name(&self) -> Result<String> {
        let v = self
            .json(self.req(Method::GET, format!("{API}/channels/{}", self.channel)))
            .await?;
        Ok(v.get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_owned())
    }

    async fn rename(&self, name: &str) -> Result<()> {
        self.json(
            self.req(Method::PATCH, format!("{API}/channels/{}", self.channel))
                .json(&serde_json::json!({ "name": name })),
        )
        .await
        .map(|_| ())
    }

    /// The message to keep editing: our most recent one in the channel. Any
    /// older ones are deleted, which is a one-time cleanup of what the
    /// previous delete-and-repost bot left behind.
    async fn adopt(&self, me: &str) -> Result<Option<String>> {
        let v = self
            .json(self.req(
                Method::GET,
                format!("{API}/channels/{}/messages?limit=25", self.channel),
            ))
            .await?;
        // Newest first, so the head is the one worth keeping.
        let mine: Vec<String> = v
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .filter(|m| m.pointer("/author/id").and_then(|v| v.as_str()) == Some(me))
            .filter_map(|m| m.get("id").and_then(|v| v.as_str()).map(str::to_owned))
            .collect();

        let Some((keep, stale)) = mine.split_first() else {
            return Ok(None);
        };
        for id in stale {
            let _ = self
                .send(self.req(
                    Method::DELETE,
                    format!("{API}/channels/{}/messages/{id}", self.channel),
                ))
                .await;
        }
        if !stale.is_empty() {
            tracing::info!(n = stale.len(), "discord status: removed duplicate posts");
        }
        Ok(Some(keep.clone()))
    }

    async fn post(&self, embed: &serde_json::Value) -> Result<String> {
        let v = self
            .json(
                self.req(
                    Method::POST,
                    format!("{API}/channels/{}/messages", self.channel),
                )
                .json(&serde_json::json!({ "embeds": [embed] })),
            )
            .await?;
        v.get("id")
            .and_then(|v| v.as_str())
            .map(str::to_owned)
            .context("posted message has no id")
    }

    /// `Ok(false)` means the message is gone (someone deleted it by hand) and
    /// the caller should post a fresh one.
    async fn edit(&self, id: &str, embed: &serde_json::Value) -> Result<bool> {
        let resp = self
            .send(
                self.req(
                    Method::PATCH,
                    format!("{API}/channels/{}/messages/{id}", self.channel),
                )
                .json(&serde_json::json!({ "embeds": [embed] })),
            )
            .await?;
        if resp.status() == StatusCode::NOT_FOUND {
            return Ok(false);
        }
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("discord returned {status}: {body}");
        }
        Ok(true)
    }
}

// ---------------------------------------------------------------------------
// The loop
// ---------------------------------------------------------------------------

async fn run(state: CloudState, cfg: DiscordConfig) {
    let d = Discord::new(&cfg);
    let poll = Duration::from_secs(cfg.poll_interval_secs.max(MIN_POLL_SECS));

    // Our user id is the one thing the task cannot start without, and the
    // usual reason it fails is a bad token, which no amount of retrying
    // fixes. Try a few times to ride out a network blip at boot, then give up
    // quietly rather than hammer Discord forever.
    let mut me = None;
    for attempt in 1..=5 {
        match d.me().await {
            Ok(id) => {
                me = Some(id);
                break;
            }
            Err(e) => {
                tracing::warn!(error = %e, attempt, "discord status: identify failed");
                tokio::time::sleep(Duration::from_secs(10 * attempt)).await;
            }
        }
    }
    let Some(me) = me else {
        tracing::error!("discord status: could not identify, task disabled");
        return;
    };

    let mut message = match d.adopt(&me).await {
        Ok(id) => id,
        Err(e) => {
            tracing::warn!(error = %e, "discord status: could not read history, will post fresh");
            None
        }
    };
    // Tracked locally so the loop doesn't GET the channel every tick; kept in
    // step by only ever changing it after a rename Discord accepted.
    let mut name = d.channel_name().await.unwrap_or_default();
    let mut last_status = "";

    tracing::info!(
        poll_secs = poll.as_secs(),
        adopted = message.is_some(),
        "discord status: started"
    );

    let mut apps: Option<AppFailures> = None;
    let mut apps_read_at: Option<Instant> = None;

    let mut tick = tokio::time::interval(poll);
    loop {
        tick.tick().await; // fires immediately the first time round
        let status = probe(&state).await;

        // A failed read keeps the last good numbers rather than blanking the
        // field: over a 24-hour window, five minutes of staleness is nothing.
        if apps_read_at.is_none_or(|t| t.elapsed() >= APPS_REFRESH) {
            match app_failures(&state).await {
                Ok(a) => apps = Some(a),
                Err(e) => tracing::warn!(error = %e, "discord status: client_logs read failed"),
            }
            apps_read_at = Some(Instant::now());
        }
        let up = state.start_time.elapsed();
        let errors = Errors {
            server: incidents::snapshot(),
            apps,
            since_boot: (up < ERROR_WINDOW).then_some(up),
        };
        let embed = embed(status, &errors);

        let wanted = channel_name_for(status);
        if name != wanted {
            match d.rename(&wanted).await {
                Ok(()) => {
                    tracing::info!(name = %wanted, "discord status: channel renamed");
                    name = wanted;
                }
                Err(e) => tracing::warn!(error = %e, "discord status: rename failed"),
            }
        }

        // `Ok(false)` covers both "never had one" and "someone deleted it by
        // hand"; either way the fix is a fresh post.
        let edited = match message.as_deref() {
            Some(id) => d.edit(id, &embed).await,
            None => Ok(false),
        };
        match edited {
            Ok(true) => {}
            Ok(false) => match d.post(&embed).await {
                Ok(id) => {
                    tracing::info!("discord status: posted a new status message");
                    message = Some(id);
                }
                Err(e) => tracing::warn!(error = %e, "discord status: post failed"),
            },
            Err(e) => tracing::warn!(error = %e, "discord status: update failed"),
        }

        // One line per change, not one per tick: at 60s the latter would be
        // 1440 identical entries a day in the log shipper.
        if status != last_status {
            tracing::info!(from = last_status, to = status, "discord status: changed");
            last_status = status;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unconfigured_by_default() {
        assert!(!is_configured(&DiscordConfig::default()));
    }

    #[test]
    fn both_halves_are_required() {
        let token_only = DiscordConfig {
            bot_token: "x".into(),
            ..Default::default()
        };
        let channel_only = DiscordConfig {
            channel_id: 1,
            ..Default::default()
        };
        assert!(!is_configured(&token_only));
        assert!(!is_configured(&channel_only));
        assert!(is_configured(&DiscordConfig {
            bot_token: "x".into(),
            channel_id: 1,
            ..Default::default()
        }));
    }

    /// The bug that made the old bot rename the channel on every poll:
    /// Discord stores `🟢 status` as `🟢-status`, so a name with a space
    /// never matched what came back and the 2-per-10-minutes budget went up
    /// in smoke.
    #[test]
    fn channel_name_is_pre_slugified() {
        assert_eq!(channel_name_for("ok"), "🟢-status");
        assert_eq!(channel_name_for("degraded"), "🟡-status");
        assert!(!channel_name_for("ok").contains(' '));
    }

    #[test]
    fn embed_carries_status_and_version() {
        let e = embed("ok", &Errors::default());
        assert_eq!(e["color"], 0x00FF00);
        assert!(e["title"].as_str().unwrap().contains("ONLINE"));
        let fields = e["fields"].as_array().unwrap();
        assert_eq!(fields[0]["value"], "`ok`");
        assert_eq!(
            fields[1]["value"],
            format!("`{}`", env!("CARGO_PKG_VERSION"))
        );
        // Rewritten every tick: it is the only thing that tells a reader the
        // embed is still live rather than frozen by an outage.
        assert!(e["timestamp"].as_str().unwrap().contains('T'));
    }

    /// The shape production actually uses: every setting arrives as an
    /// environment variable through figment, and figment hands an all-digit
    /// value over as an integer. The second half is the failure a `String`
    /// field would have been: not a skipped feature but a config that does
    /// not parse, which is a server that refuses to boot.
    #[test]
    fn channel_id_parses_from_an_all_digit_env_value() {
        use figment::{providers::Env, Figment};

        // A prefix no real setting or other test uses: the environment is
        // process-wide and tests run in parallel.
        std::env::set_var(
            "HOARDTESTDISCORD__CLOUD__DISCORD__CHANNEL_ID",
            "1234567890123456789",
        );
        std::env::set_var(
            "HOARDTESTDISCORD__CLOUD__DISCORD__BOT_TOKEN",
            "MTIz.abc-def_ghi",
        );
        let env = || Env::prefixed("HOARDTESTDISCORD__").split("__");

        #[derive(serde::Deserialize)]
        struct Root {
            cloud: crate::config::CloudConfig,
        }
        let root: Root = Figment::new()
            .merge(env())
            .extract()
            .expect("config parses");
        let d = &root.cloud.discord;
        assert_eq!(d.channel_id, 1234567890123456789);
        assert_eq!(d.bot_token, "MTIz.abc-def_ghi");
        assert_eq!(d.poll_interval_secs, 60);
        assert!(is_configured(d));

        #[derive(serde::Deserialize)]
        struct StringRoot {
            #[allow(dead_code)]
            cloud: StringCloud,
        }
        #[derive(serde::Deserialize)]
        struct StringCloud {
            #[allow(dead_code)]
            discord: StringDiscord,
        }
        #[derive(serde::Deserialize)]
        struct StringDiscord {
            #[allow(dead_code)]
            channel_id: String,
        }
        assert!(Figment::new().merge(env()).extract::<StringRoot>().is_err());
    }

    #[test]
    fn degraded_is_amber_not_red() {
        let e = embed("degraded", &Errors::default());
        assert_eq!(e["color"], 0xFFFF00);
        assert!(e["title"].as_str().unwrap().contains("DEGRADED"));
    }

    #[test]
    fn embed_has_both_error_fields() {
        let e = embed("ok", &Errors::default());
        let fields = e["fields"].as_array().unwrap();
        assert_eq!(fields.len(), 4);
        assert_eq!(fields[2]["name"], "Errors (24h) · server");
        assert_eq!(fields[3]["name"], "Errors (24h) · apps");
        assert_eq!(fields[3]["value"], "_No data yet._");
    }

    #[test]
    fn server_errors_show_counts_the_latest_and_the_boot_caveat() {
        let mut last: [incidents::Last; incidents::KINDS] = Default::default();
        last[Kind::Delete as usize] = Some((
            1_788_000_000,
            "DELETE /v1/cloud/saves/:save_id → 500".into(),
        ));
        let s = incidents::Snapshot {
            counts: [2, 0, 1, 0],
            last,
        };
        let text = server_errors_text(&s, Some(Duration::from_secs(3 * 3600 + 5 * 60)));
        assert!(text.starts_with("Uploads **2** · Downloads **0** · Deletes **1** · Other **0**"));
        assert!(text.contains("Last: `DELETE /v1/cloud/saves/:save_id → 500` at "));
        assert!(text.contains("since boot, 3h 05m ago"));

        // A full day up: no caveat, and a clean sheet says nothing more.
        let clean = server_errors_text(&incidents::Snapshot::default(), None);
        assert_eq!(
            clean,
            "Uploads **0** · Downloads **0** · Deletes **0** · Other **0**"
        );
    }

    #[test]
    fn app_errors_count_users_first() {
        let a = AppFailures {
            upload: AppCount {
                events: 41,
                users: 3,
            },
            download: AppCount {
                events: 1,
                users: 1,
            },
            crash: AppCount::default(),
        };
        assert_eq!(
            app_errors_text(Some(&a)),
            "Uploads: **3** users (41 failures)\nDownloads: **1** user (1 failure)\nCrashes: **0**"
        );
    }
}
