//! Device census and live presence, self-hosted.
//!
//! # What this answers
//!
//! Three questions a self-hoster with more than one machine asks, and which until
//! 1.1.2 only Hoard Cloud could answer:
//!
//! - Which machine this version came from. That one was already answered by the
//!   `snapshots.device_name` column, and the history has drawn it since. Nothing
//!   here is needed for it.
//! - Which machines exist on this account. The census: `GET /v1/devices`.
//! - Which are switched on right now and playing what. The presence:
//!   `POST /v1/presence/heartbeat` every 30 seconds or so from each machine.
//!
//! # Nothing leaves your server
//!
//! This is the opposite of an external service: the census lives in *your* SQLite,
//! your own machines write it against your own server, and there is no route that
//! sends it anywhere. That is why this piece fits self-hosted while operator
//! broadcasts do not: those are Hoard talking to your clients, this is your
//! clients talking to each other through your server.
//!
//! # Identity
//!
//! A machine identifies itself with the `x-hoard-device-fp` header, a stable
//! fingerprint the client computes (`hoard_agent::logship::device_identity`).
//! Genuinely stable: reinstalling the app does not create a new device. A client
//! that does not send it (older builds) registers nothing, and that is not an
//! error, it simply does not appear in the list.
//!
//! # `online` is computed on read
//!
//! There is no column to say it. A device is on while its last heartbeat is
//! younger than [`ONLINE_WINDOW_SECS`] and it has not sent the closing heartbeat.
//! Storing it would require somebody to switch it off, and a machine that goes
//! away in a power cut switches nothing off: it would stay lit forever.

use axum::{
    extract::{Extension, Path, State},
    http::{HeaderMap, StatusCode},
    response::Json,
};
use hoard_core::wire::{DeviceListOut, DeviceOut, DevicePlaying, Heartbeat};
use sqlx::Row;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::routes::health::ServerState;
use crate::routes::snapshots::internal_logged;

type ApiError = (StatusCode, Json<serde_json::Value>);

/// A device counts as on while its last heartbeat is younger than this. The agent
/// beats every 30 seconds, so 90 seconds is three missed beats: short enough to be
/// noticeable live, long enough to survive a network stumble. If you touch this,
/// touch `KEEPALIVE_SECS` in `hoard_agent::presence` too.
pub const ONLINE_WINDOW_SECS: i64 = 90;

/// Caps on what a heartbeat may declare. Presence is cosmetic and only the
/// account's owner sees it, so this defends against nothing serious: it stops a
/// broken client putting megabytes into a row.
const MAX_PLAYING_GAMES: usize = 8;
const MAX_SLUG_CHARS: usize = 128;

/// A timestamp in the same format the migrations write
/// (`strftime('%Y-%m-%dT%H:%M:%SZ')`), so SQLite's text comparisons sort right.
fn stamp(at: OffsetDateTime) -> String {
    let d = at.date();
    let t = at.time();
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        d.year(),
        u8::from(d.month()),
        d.day(),
        t.hour(),
        t.minute(),
        t.second()
    )
}

fn header<'a>(headers: &'a HeaderMap, key: &str) -> Option<&'a str> {
    headers
        .get(key)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

/// Registers or refreshes the asking machine. With no fingerprint nothing is
/// registered and that is *not* an error: an older client must keep syncing just
/// the same, only without appearing in the list.
pub async fn register(
    pool: &sqlx::SqlitePool,
    user_id: &str,
    headers: &HeaderMap,
) -> Result<(), sqlx::Error> {
    let Some(fingerprint) = header(headers, "x-hoard-device-fp") else {
        return Ok(());
    };
    let name = header(headers, "x-hoard-device-name").unwrap_or("Unknown device");
    let os = header(headers, "x-hoard-device-os");
    let app_version = header(headers, "x-hoard-app-version");
    let now = stamp(OffsetDateTime::now_utc());

    // `closed_at = NULL` on reappearing: a machine that said goodbye and came
    // back is on again. `device_kind` stays 'desktop', since today the only client
    // that beats is the desktop app (or the daemon on the same machine); the day
    // there is another, this is what changes.
    sqlx::query(
        "INSERT INTO devices (id, user_id, device_name, device_kind, os, app_version,
                              fingerprint, last_seen_at, closed_at)
         VALUES (?,?,?,'desktop',?,?,?,?,NULL)
         ON CONFLICT(user_id, fingerprint) DO UPDATE SET
             last_seen_at = excluded.last_seen_at,
             closed_at    = NULL,
             device_name  = excluded.device_name,
             os           = COALESCE(excluded.os, devices.os),
             app_version  = COALESCE(excluded.app_version, devices.app_version)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(user_id)
    .bind(name)
    .bind(os)
    .bind(app_version)
    .bind(fingerprint)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(())
}

// ---- GET /v1/devices

pub async fn list(
    State(state): State<Arc<ServerState>>,
    Extension(user): Extension<AuthUser>,
    headers: HeaderMap,
) -> Result<Json<DeviceListOut>, ApiError> {
    let user_id = user.user_id.to_string();
    // Asking counts as being alive too: otherwise the machine with the panel open
    // would be the only one showing as off.
    if let Err(e) = register(&state.pool, &user_id, &headers).await {
        tracing::warn!(error = %e, "devices: register on list failed");
    }
    let caller_fp = header(&headers, "x-hoard-device-fp").unwrap_or("");
    let cutoff = stamp(OffsetDateTime::now_utc() - time::Duration::seconds(ONLINE_WINDOW_SECS));

    let rows = sqlx::query(
        "SELECT id, device_name, device_kind, os, fingerprint, playing,
                last_seen_at, created_at, closed_at
           FROM devices WHERE user_id = ?
          ORDER BY last_seen_at DESC",
    )
    .bind(&user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| internal_logged("listing devices", e))?;

    let devices = rows
        .into_iter()
        .map(|r| {
            let last_seen: String = r.get("last_seen_at");
            let closed: Option<String> = r.get("closed_at");
            let online = closed.is_none() && last_seen.as_str() > cutoff.as_str();
            let fp: String = r.get("fingerprint");
            // The game list is only served when the machine is on: "so-and-so
            // playing X" with the machine off since yesterday is worse than
            // saying nothing.
            let playing: Vec<DevicePlaying> = if online {
                r.get::<Option<String>, _>("playing")
                    .as_deref()
                    .and_then(|j| serde_json::from_str(j).ok())
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            DeviceOut {
                id: r.get("id"),
                device_name: r.get("device_name"),
                device_kind: r.get("device_kind"),
                os: r.get("os"),
                last_seen_at: Some(last_seen),
                created_at: r.get("created_at"),
                online,
                playing,
                this_device: !caller_fp.is_empty() && fp == caller_fp,
            }
        })
        .collect();

    Ok(Json(DeviceListOut { devices }))
}

// ---- DELETE /v1/devices/:id

/// Forget a machine. It only deletes the census: the versions it uploaded stay
/// where they are and its `device_name` in the history is untouched, because that
/// is a fact about what happened, not about the device.
pub async fn delete(
    State(state): State<Arc<ServerState>>,
    Extension(user): Extension<AuthUser>,
    Path(device_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<DeviceListOut>, ApiError> {
    let user_id = user.user_id.to_string();
    sqlx::query("DELETE FROM devices WHERE id = ? AND user_id = ?")
        .bind(&device_id)
        .bind(&user_id)
        .execute(&state.pool)
        .await
        .map_err(|e| internal_logged("deleting a device", e))?;
    list(State(state), Extension(user), headers).await
}

// ---- POST /v1/presence/heartbeat

pub async fn heartbeat(
    State(state): State<Arc<ServerState>>,
    Extension(user): Extension<AuthUser>,
    headers: HeaderMap,
    Json(body): Json<Heartbeat>,
) -> Result<StatusCode, ApiError> {
    let user_id = user.user_id.to_string();
    let Some(fp) = header(&headers, "x-hoard-device-fp") else {
        // A client with no fingerprint: there is nobody to attribute the beat
        // to. Silence, not an error; an older client must keep working.
        return Ok(StatusCode::NO_CONTENT);
    };
    let now = OffsetDateTime::now_utc();

    if body.closing {
        sqlx::query(
            "UPDATE devices SET last_seen_at = ?, closed_at = ?, playing = NULL
              WHERE user_id = ? AND fingerprint = ?",
        )
        .bind(stamp(now))
        .bind(stamp(now))
        .bind(&user_id)
        .bind(fp)
        .execute(&state.pool)
        .await
        .map_err(|e| internal_logged("recording a closing beat", e))?;
        return Ok(StatusCode::NO_CONTENT);
    }

    // First contact from this machine (or a return after a close): register it
    // with the same headers.
    register(&state.pool, &user_id, &headers)
        .await
        .map_err(|e| internal_logged("registering a device", e))?;

    let stored: Option<String> =
        sqlx::query_scalar("SELECT playing FROM devices WHERE user_id = ? AND fingerprint = ?")
            .bind(&user_id)
            .bind(fp)
            .fetch_optional(&state.pool)
            .await
            .map_err(|e| internal_logged("reading presence", e))?
            .flatten();
    let old: Vec<DevicePlaying> = stored
        .as_deref()
        .and_then(|j| serde_json::from_str(j).ok())
        .unwrap_or_default();

    let playing = merge_playing(&old, &body.playing, now);
    let encoded = serde_json::to_string(&playing).unwrap_or_else(|_| "[]".into());

    sqlx::query(
        "UPDATE devices SET last_seen_at = ?, closed_at = NULL, playing = ?
          WHERE user_id = ? AND fingerprint = ?",
    )
    .bind(stamp(now))
    .bind(&encoded)
    .bind(&user_id)
    .bind(fp)
    .execute(&state.pool)
    .await
    .map_err(|e| internal_logged("recording a heartbeat", e))?;

    Ok(StatusCode::NO_CONTENT)
}

/// Anchors each game's `since` *once*, on the heartbeat where its slug first
/// appears.
///
/// Pure so it can be tested. Without this the panel's "40 minutes in" would jump
/// on every heartbeat, because each one would carry a `for_secs` recomputed by the
/// client. The anchor is set by the server's clock: a client with a skewed clock
/// cannot claim to have been playing since the future.
fn merge_playing(
    old: &[DevicePlaying],
    beat: &[hoard_core::wire::PlayingBeat],
    now: OffsetDateTime,
) -> Vec<DevicePlaying> {
    beat.iter()
        .filter(|g| !g.slug.is_empty() && g.slug.chars().count() <= MAX_SLUG_CHARS)
        .take(MAX_PLAYING_GAMES)
        .map(|g| {
            let since = old
                .iter()
                .find(|o| o.slug == g.slug)
                .and_then(|o| o.since.clone())
                .unwrap_or_else(|| {
                    // First time it is seen: anchor it to what the client says it
                    // has been going, bounded so an absurd value cannot invent a
                    // session lasting years.
                    let secs = g.for_secs.min(60 * 60 * 24 * 7) as i64;
                    stamp(now - time::Duration::seconds(secs))
                });
            DevicePlaying {
                slug: g.slug.clone(),
                since: Some(since),
            }
        })
        .collect()
}

/// Forget machines that have gone `max_age_days` without a sign of life. Without
/// this the census accumulates every laptop somebody used one afternoon, forever.
pub async fn prune_stale(pool: &sqlx::SqlitePool, max_age_days: i64) -> Result<u64, sqlx::Error> {
    let cutoff = stamp(OffsetDateTime::now_utc() - time::Duration::days(max_age_days));
    let done = sqlx::query("DELETE FROM devices WHERE last_seen_at < ?")
        .bind(cutoff)
        .execute(pool)
        .await?;
    Ok(done.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;
    use hoard_core::wire::PlayingBeat;

    fn beat(slug: &str, for_secs: u64) -> PlayingBeat {
        PlayingBeat {
            slug: slug.into(),
            for_secs,
        }
    }

    /// A session's anchor is set once and does not move while the game stays in
    /// the list. That is what makes the panel's "40 minutes in" advance rather
    /// than wobble with every heartbeat.
    #[test]
    fn a_running_session_keeps_its_anchor_across_beats() {
        let now = OffsetDateTime::from_unix_timestamp(1_800_000_000).unwrap();
        let first = merge_playing(&[], &[beat("factorio", 600)], now);
        assert_eq!(first.len(), 1);
        let anchor = first[0].since.clone().unwrap();
        assert_eq!(anchor, stamp(now - time::Duration::seconds(600)));

        // Next heartbeat, half a minute later and with the client's counter
        // advanced: the anchor is untouched.
        let later = now + time::Duration::seconds(30);
        let second = merge_playing(&first, &[beat("factorio", 630)], later);
        assert_eq!(second[0].since.as_deref(), Some(anchor.as_str()));

        // A new game does get a fresh anchor.
        let third = merge_playing(&second, &[beat("factorio", 660), beat("stardew", 0)], later);
        assert_eq!(third.len(), 2);
        assert_eq!(third[0].since.as_deref(), Some(anchor.as_str()));
        assert_eq!(third[1].since, Some(stamp(later)));

        // And stopping playing takes it off the list.
        assert!(merge_playing(&third, &[], later).is_empty());
    }

    /// A broken client cannot inject junk or invent an eternal session.
    #[test]
    fn a_beat_cannot_claim_nonsense() {
        let now = OffsetDateTime::from_unix_timestamp(1_800_000_000).unwrap();
        let many: Vec<PlayingBeat> = (0..20).map(|i| beat(&format!("g{i}"), 0)).collect();
        assert_eq!(merge_playing(&[], &many, now).len(), MAX_PLAYING_GAMES);

        let long = "x".repeat(MAX_SLUG_CHARS + 1);
        assert!(merge_playing(&[], &[beat(&long, 0), beat("", 0)], now).is_empty());

        // Ten years of session get bounded to a week rather than anchoring the
        // session in 2016.
        let absurd = merge_playing(&[], &[beat("factorio", 60 * 60 * 24 * 3650)], now);
        assert_eq!(absurd[0].since, Some(stamp(now - time::Duration::days(7))));
    }
}
