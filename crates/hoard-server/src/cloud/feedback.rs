//! Hoard-help: reports and ideas sent from the app, and what happens to them
//! after they land. The routes are in `routes/feedback.rs`.
//!
//! Files live in R2 under `feedback/<report>/`, a prefix no other sweeper
//! lists (`reconcile` and the orphan GC only walk `blobs/`). That makes this
//! module the only thing that ever deletes them, so the sweep below is not
//! optional housekeeping: without it the bucket keeps every video forever.

use super::incidents::{self, Kind};
use crate::cloud::state::CloudState;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;

/// What the app promises in the line above the form.
pub const RETENTION_DAYS: i32 = 90;

/// A report still pending after this is an upload that died with the app.
const PENDING_TTL_HOURS: i32 = 24;

pub fn spawn(state: CloudState) {
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(11 * 60)).await;
        let mut tick = tokio::time::interval(Duration::from_secs(60 * 60));
        loop {
            tick.tick().await;
            match sweep(&state).await {
                Ok(0) => {}
                Ok(n) => tracing::info!(reports = n, "feedback: swept expired reports"),
                Err(e) => {
                    tracing::warn!(error = %e, "feedback: sweep failed");
                    incidents::record(Kind::Delete, "feedback: sweep failed");
                }
            }
        }
    });
}

/// Deletes expired reports (objects first, then rows) and blanks the address
/// hashes the throttle no longer needs. Returns how many reports went.
pub async fn sweep(state: &CloudState) -> anyhow::Result<u64> {
    sqlx::query(
        "UPDATE feedback_reports SET ip_hash = NULL
          WHERE ip_hash IS NOT NULL AND created_at < now() - interval '1 day'",
    )
    .execute(&state.pool)
    .await?;

    let expired: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM feedback_reports
          WHERE (completed_at IS NULL AND created_at < now() - make_interval(hours => $1))
             OR created_at < now() - make_interval(days => $2)
          LIMIT 500",
    )
    .bind(PENDING_TTL_HOURS)
    .bind(RETENTION_DAYS)
    .fetch_all(&state.pool)
    .await?;

    let mut gone = 0u64;
    'reports: for (id,) in expired {
        let keys: Vec<(String,)> =
            sqlx::query_as("SELECT r2_key FROM feedback_files WHERE report_id = $1")
                .bind(id)
                .fetch_all(&state.pool)
                .await?;
        // A file that never arrived has no object, and S3 answers a DELETE of
        // a missing key with success, so there is nothing to tell apart here.
        // One that fails keeps its row, so the next sweep tries the whole
        // report again instead of losing track of the object.
        for (key,) in keys {
            if let Err(e) = state.r2.delete_object(&key).await {
                tracing::warn!(error = %e, %key, "feedback: could not delete a file, retrying next sweep");
                continue 'reports;
            }
        }
        sqlx::query("DELETE FROM feedback_reports WHERE id = $1")
            .bind(id)
            .execute(&state.pool)
            .await?;
        gone += 1;
    }
    Ok(gone)
}

/// `feedback/<report>/<idx>-<name>`. The name rides along so a download from the
/// panel saves as `2-hoard-logs.txt` and not as a bare number.
pub fn object_key(report: Uuid, idx: i16, name: &str) -> String {
    format!("feedback/{report}/{idx}-{}", safe_name(name))
}

/// The file name as the sender had it, reduced to what is safe inside a key and
/// a `Content-Disposition`.
pub fn safe_name(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let mut out: String = base
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    // Keep the extension when cutting: `…_.mp4` still opens, `…_.m` does not.
    if out.len() > 80 {
        let ext = out
            .rfind('.')
            .filter(|&i| out.len() - i <= 10)
            .map(|i| out[i..].to_string())
            .unwrap_or_default();
        out.truncate(80 - ext.len());
        out.push_str(&ext);
    }
    let out = out.trim_start_matches('.').to_string();
    if out.is_empty() {
        "file".into()
    } else {
        out
    }
}

pub fn sha256_hex(s: &str) -> String {
    hex::encode(Sha256::digest(s.as_bytes()))
}

/// Recent reports from one address and in total, for the throttle.
pub struct Recent {
    pub ip_hour: i64,
    pub ip_day: i64,
    pub all_hour: i64,
    pub bytes_day: i64,
}

pub async fn recent(pool: &PgPool, ip_hash: &str) -> Result<Recent, sqlx::Error> {
    let (ip_hour, ip_day, all_hour): (i64, i64, i64) = sqlx::query_as(
        "SELECT
            count(*) FILTER (WHERE ip_hash = $1 AND created_at > now() - interval '1 hour'),
            count(*) FILTER (WHERE ip_hash = $1),
            count(*) FILTER (WHERE created_at > now() - interval '1 hour')
           FROM feedback_reports
          WHERE created_at > now() - interval '1 day'",
    )
    .bind(ip_hash)
    .fetch_one(pool)
    .await?;
    let bytes_day: i64 = sqlx::query_scalar(
        "SELECT coalesce(sum(f.size_bytes), 0)::bigint
           FROM feedback_files f
           JOIN feedback_reports r ON r.id = f.report_id
          WHERE r.created_at > now() - interval '1 day'",
    )
    .fetch_one(pool)
    .await?;
    Ok(Recent {
        ip_hour,
        ip_day,
        all_hour,
        bytes_day,
    })
}

#[derive(sqlx::FromRow)]
struct Arrival {
    kind: String,
    app_version: Option<String>,
    os: Option<String>,
    mode: Option<String>,
    account: bool,
    files: i64,
    bytes: i64,
}

/// Posts the arrival to the Discord feedback channel, when there is one.
///
/// Says what arrived and nothing the person wrote: the app tells them only we
/// read it, and a Discord channel is somebody else's server. The text is in the
/// admin panel.
pub async fn notify(state: &CloudState, id: Uuid) {
    let Some(cfg) = state.config.cloud.as_ref().map(|c| c.discord.clone()) else {
        return;
    };
    if cfg.bot_token.is_empty() || cfg.feedback_channel_id == 0 {
        return;
    }
    let row: Result<Arrival, _> = sqlx::query_as(
        "SELECT r.kind, r.app_version, r.os, r.mode, r.user_id IS NOT NULL AS account,
                count(f.idx) AS files, coalesce(sum(f.size_bytes), 0)::bigint AS bytes
           FROM feedback_reports r
           LEFT JOIN feedback_files f ON f.report_id = r.id
          WHERE r.id = $1
          GROUP BY r.id",
    )
    .bind(id)
    .fetch_one(&state.pool)
    .await;
    let Ok(arrival) = row else {
        return;
    };
    if let Err(e) =
        super::discord::post_to(&cfg, cfg.feedback_channel_id, &embed(arrival, id)).await
    {
        tracing::warn!(error = %e, "feedback: discord notice failed");
    }
}

fn embed(a: Arrival, id: Uuid) -> serde_json::Value {
    let (title, color) = match a.kind.as_str() {
        "bug" => ("New bug report", 0xd9_59_26),
        _ => ("New idea", 0x39_87_e5),
    };
    let mode = match a.mode.as_deref() {
        Some("cloud") if a.account => "Hoard Cloud",
        Some("cloud") => "Hoard Cloud (signed out)",
        Some("selfhosted") => "self-hosted",
        _ => "no server",
    };
    let files = if a.files == 0 {
        "none".to_string()
    } else {
        format!("{} ({:.1} MB)", a.files, a.bytes as f64 / 1_000_000.0)
    };
    serde_json::json!({
        "title": title,
        "description": "Read it in the admin panel, Hoard-help tab.",
        "color": color,
        "fields": [
            { "name": "Version", "value": a.app_version.unwrap_or_else(|| "?".into()), "inline": true },
            { "name": "OS", "value": a.os.unwrap_or_else(|| "?".into()), "inline": true },
            { "name": "Install", "value": mode, "inline": true },
            { "name": "Files", "value": files, "inline": true },
        ],
        "footer": { "text": id.to_string() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_lose_paths_and_odd_characters() {
        assert_eq!(
            safe_name("C:\\Users\\angel\\Videos\\bug 1.mp4"),
            "bug_1.mp4"
        );
        assert_eq!(safe_name("../../etc/passwd"), "passwd");
        assert_eq!(safe_name("..."), "file");
        assert_eq!(safe_name("captura ñ.png"), "captura__.png");
    }

    #[test]
    fn long_names_keep_their_extension() {
        let long = format!("{}.mp4", "a".repeat(200));
        let out = safe_name(&long);
        assert_eq!(out.len(), 80);
        assert!(out.ends_with(".mp4"));
    }

    #[test]
    fn the_embed_carries_no_message_text() {
        let a = Arrival {
            kind: "bug".into(),
            app_version: Some("1.2.0".into()),
            os: None,
            mode: Some("selfhosted".into()),
            account: false,
            files: 2,
            bytes: 3_000_000,
        };
        let e = embed(a, Uuid::nil());
        assert_eq!(e["title"], "New bug report");
        assert_eq!(e["fields"][2]["value"], "self-hosted");
        assert_eq!(e["fields"][3]["value"], "2 (3.0 MB)");
    }
}
