//! Password login for the browser panel.
//!
//! Until now `users.password_hash` was written by `hoard-admin user create`
//! and read by nobody: the only way in was a `hoard_v1_…` token, which is the
//! right credential for a client that stores it in a keyring and the wrong one
//! for a human at a browser. This module makes that column finally do its job.
//!
//! A session is not a new kind of credential. Login mints an ordinary
//! `api_tokens` row (short-lived, `device_name = 'web panel'`) and hands it to
//! the browser in an httpOnly cookie, so it expires, revokes and lists through
//! the machinery that already exists: `hoard-admin token list <user>` shows a
//! browser session next to the desktop's token, and revoking it logs the
//! browser out. See [`crate::auth::SESSION_COOKIE`] for why `SameSite=Strict`
//! is load-bearing.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use axum::{
    extract::{ConnectInfo, Extension, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::auth::{AuthUser, SESSION_COOKIE};
use crate::routes::health::ServerState;

/// Same floor the CLI enforces in `hoard-admin user create`. Kept in sync by
/// hand; if one moves, move both, or a password the panel accepts becomes one
/// the CLI would have refused.
const MIN_PASSWORD_LEN: usize = 8;

/// Wrong passwords tolerated from one origin for one account before the door
/// shuts for `panel.login_throttle_secs`.
const MAX_FAILURES: u32 = 5;
/// The same, counted per origin across every account it tries. Without it the
/// per-account limit above is bypassed by rotating the username, and each
/// attempt still costs a full argon2id verify, 19 MiB and tens of milliseconds,
/// which turns the password hash into a lever against the box
/// instead of a wall in front of it. Higher than the per-account number so a
/// household behind one NAT address doesn't lock itself out by fumbling two
/// different passwords.
const MAX_FAILURES_PER_ORIGIN: u32 = 20;
#[derive(Deserialize)]
pub struct LoginBody {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginOk {
    pub username: String,
    pub is_admin: bool,
    /// Seconds until the cookie expires. The panel uses it to warn before a
    /// long-idle tab starts 401-ing mid-click.
    pub expires_in_secs: i64,
}

#[derive(Deserialize)]
pub struct PasswordBody {
    pub current_password: String,
    pub new_password: String,
}

/// Machine-readable failure. The panel maps the code to a translated string,
/// the server has no business guessing the reader's language, and these codes
/// are also what a script would branch on.
fn fail(status: StatusCode, code: &str) -> (StatusCode, Json<serde_json::Value>) {
    (status, Json(serde_json::json!({ "error": code })))
}

type ApiError = (StatusCode, Json<serde_json::Value>);

/// `POST /v1/auth/login`: exchange username and password for a session cookie.
pub async fn login(
    State(state): State<Arc<ServerState>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<LoginBody>,
) -> Result<Response, ApiError> {
    let username = body.username.trim().to_string();
    let origin = crate::clientip::client_ip(&headers, peer, &state.trusted_proxies);
    // Bucketed, not the bare address: an IPv6 client owns a /64 and would
    // otherwise get a fresh counter per attempt. See `clientip::throttle_bucket`.
    let bucket = crate::clientip::throttle_bucket(origin);
    let throttle_key = format!("{bucket}|{}", username.to_lowercase());
    let origin_key = format!("{bucket}|*");

    // Through the accessor, never the raw field: it is the one place the floor
    // is applied, and reading the number directly is how `login_throttle_secs =
    // 0` turned the throttle off entirely while the boot log claimed otherwise.
    let window = state.config.panel.login_throttle();
    let wait = throttled_for(&throttle_key, MAX_FAILURES, window)
        .or_else(|| throttled_for(&origin_key, MAX_FAILURES_PER_ORIGIN, window));
    if let Some(wait) = wait {
        tracing::warn!(%username, %origin, "panel: login throttled");
        // Returned through the Ok branch so it can carry `Retry-After`: the
        // error type here is a bare (status, json) pair with nowhere to put a
        // header, and a script hitting this deserves the standard one rather
        // than having to parse the body. It is still a 429 on the wire.
        let secs = wait.as_secs().max(1);
        return Ok((
            StatusCode::TOO_MANY_REQUESTS,
            [(header::RETRY_AFTER, secs.to_string())],
            Json(serde_json::json!({
                "error": "too_many_attempts",
                "retry_after_secs": secs,
            })),
        )
            .into_response());
    }

    let user = lookup_user(&state.pool, &username)
        .await
        .map_err(|e| internal(e, "panel login lookup"))?;

    // An unknown username and a wrong password have to cost the same and say
    // the same thing, or the login form doubles as a "does this account exist"
    // oracle. The dummy hash below is a real argon2 verify against a hash of
    // random bytes, so the timing matches too.
    let (user_id, real_username, is_admin, hash) = match user {
        Some(u) => u,
        None => {
            let _ = hoard_core::hashing::verify_password(&body.password, dummy_hash());
            record_failure(&throttle_key, MAX_FAILURES, window);
            record_failure(&origin_key, MAX_FAILURES_PER_ORIGIN, window);
            return Err(fail(StatusCode::UNAUTHORIZED, "invalid_credentials"));
        }
    };

    let ok = hoard_core::hashing::verify_password(&body.password, &hash).unwrap_or_else(|e| {
        // A hash the verifier can't parse is a row written by something that
        // isn't `hash_password`: hand-edited, or restored from a backup of a
        // different scheme. Treat it as a failed login, not a 500: the account
        // needs `hoard-admin user passwd`, and saying so in the log is the only
        // way the operator finds out.
        tracing::error!(username = %real_username, error = %e, "panel: unparseable password hash");
        false
    });
    if !ok {
        record_failure(&throttle_key, MAX_FAILURES, window);
        record_failure(&origin_key, MAX_FAILURES_PER_ORIGIN, window);
        return Err(fail(StatusCode::UNAUTHORIZED, "invalid_credentials"));
    }
    // Only the account counter is forgiven. Clearing the origin's on a correct
    // password would let an attacker who holds one valid account reset the
    // budget between guesses at everyone else's.
    clear_failures(&throttle_key);

    let ttl_secs = (state.config.panel.session_days.max(1) * 86_400) as i64;
    let token = mint_session(&state.pool, &user_id, ttl_secs)
        .await
        .map_err(|e| internal(e, "panel session mint"))?;

    tracing::info!(username = %real_username, %origin, "panel: login");

    let cookie = session_cookie(&token, ttl_secs, is_secure(&headers));
    Ok((
        StatusCode::OK,
        [(header::SET_COOKIE, cookie)],
        Json(LoginOk {
            username: real_username,
            is_admin,
            expires_in_secs: ttl_secs,
        }),
    )
        .into_response())
}

/// `POST /v1/auth/session`: trade a working `hoard_v1_…` token for a session.
///
/// The second way into the panel, and the one that saves an account whose
/// password nobody remembers: paste the token the CLI already uses. It runs
/// through the normal auth middleware, so by the time we get here the token has
/// already proved itself.
///
/// It mints a *new* short-lived session rather than putting the pasted token in
/// the cookie. Two reasons: logging out of the browser must not revoke the
/// device that token belongs to, and a token with `token_lifetime_days = 0` (no
/// expiry, a legitimate self-hosted choice) would otherwise become a
/// never-expiring browser credential.
///
/// It also means the panel never has to hold a token in `localStorage`, where
/// any script on the page could read it: the paste goes straight out in one
/// request and what comes back is httpOnly.
pub async fn exchange_token(
    State(state): State<Arc<ServerState>>,
    Extension(user): Extension<AuthUser>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let ttl_secs = (state.config.panel.session_days.max(1) * 86_400) as i64;
    let token = mint_session(&state.pool, &user.user_id.to_string(), ttl_secs)
        .await
        .map_err(|e| internal(e, "panel token exchange"))?;

    tracing::info!(username = %user.username, "panel: session from token");

    let cookie = session_cookie(&token, ttl_secs, is_secure(&headers));
    Ok((
        StatusCode::OK,
        [(header::SET_COOKIE, cookie)],
        Json(LoginOk {
            username: user.username.clone(),
            is_admin: user.is_admin,
            expires_in_secs: ttl_secs,
        }),
    )
        .into_response())
}

/// `POST /v1/auth/logout`: revoke the session this request arrived with.
///
/// Deliberately narrow: it revokes the one token in the cookie, never the
/// user's other tokens. Logging out of a browser must not stop the desktop
/// app from syncing.
pub async fn logout(
    State(state): State<Arc<ServerState>>,
    Extension(user): Extension<AuthUser>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    if let Some(tok) = cookie_value(&headers, SESSION_COOKIE) {
        let hash = hoard_core::hashing::hash_token(&tok);
        sqlx::query(
            "UPDATE api_tokens \
             SET revoked_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') \
             WHERE token_hash = ? AND revoked_at IS NULL",
        )
        .bind(&hash)
        .execute(&state.pool)
        .await
        .map_err(|e| internal(e.into(), "panel logout revoke"))?;
        tracing::info!(username = %user.username, "panel: logout");
    }

    // Clear the cookie even when there wasn't one (a Bearer client calling
    // logout): the browser's copy is what we're trying to get rid of.
    let cleared = format!("{SESSION_COOKIE}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0");
    Ok((StatusCode::NO_CONTENT, [(header::SET_COOKIE, cleared)]).into_response())
}

/// `POST /v1/auth/password`: change your own password.
///
/// Every *other* browser session is revoked on success, which is the point of
/// changing a password you think leaked. API tokens survive on purpose: they
/// belong to devices the user set up deliberately, and silently unsyncing
/// their machines would be a worse surprise than the one they're fixing.
/// `hoard-admin token revoke` is the tool for those.
pub async fn change_password(
    State(state): State<Arc<ServerState>>,
    Extension(user): Extension<AuthUser>,
    headers: HeaderMap,
    Json(body): Json<PasswordBody>,
) -> Result<StatusCode, ApiError> {
    if body.new_password.chars().count() < MIN_PASSWORD_LEN {
        return Err(fail(StatusCode::BAD_REQUEST, "password_too_short"));
    }

    let user_id = user.user_id.to_string();
    let hash: (String,) = sqlx::query_as("SELECT password_hash FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| internal(e.into(), "password change lookup"))?;

    if !hoard_core::hashing::verify_password(&body.current_password, &hash.0).unwrap_or(false) {
        return Err(fail(StatusCode::UNAUTHORIZED, "invalid_credentials"));
    }

    let new_hash = hoard_core::hashing::hash_password(&body.new_password)
        .map_err(|e| internal(e, "password hashing"))?;
    sqlx::query(
        "UPDATE users SET password_hash = ?, \
         updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE id = ?",
    )
    .bind(&new_hash)
    .bind(&user_id)
    .execute(&state.pool)
    .await
    .map_err(|e| internal(e.into(), "password update"))?;

    let keep = cookie_value(&headers, SESSION_COOKIE)
        .map(|t| hoard_core::hashing::hash_token(&t))
        .unwrap_or_default();
    let revoked = sqlx::query(
        "UPDATE api_tokens \
         SET revoked_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') \
         WHERE user_id = ? AND device_name = ? AND revoked_at IS NULL AND token_hash <> ?",
    )
    .bind(&user_id)
    .bind(SESSION_DEVICE_NAME)
    .bind(&keep)
    .execute(&state.pool)
    .await
    .map_err(|e| internal(e.into(), "session sweep"))?
    .rows_affected();

    tracing::info!(username = %user.username, revoked, "panel: password changed");
    Ok(StatusCode::NO_CONTENT)
}

/// What a browser session is called in `api_tokens.device_name`. The panel and
/// the admin views filter on this exact string to tell a browser apart from a
/// device, so it is a value, not a label: changing it orphans existing rows.
pub const SESSION_DEVICE_NAME: &str = "web panel";

async fn mint_session(
    pool: &sqlx::SqlitePool,
    user_id: &str,
    ttl_secs: i64,
) -> anyhow::Result<String> {
    let token = hoard_core::hashing::generate_token();
    let token_hash = hoard_core::hashing::hash_token(&token);
    let id = uuid::Uuid::new_v4().to_string();
    let expires_at = (OffsetDateTime::now_utc() + time::Duration::seconds(ttl_secs))
        .format(&time::format_description::well_known::Rfc3339)?;

    sqlx::query(
        "INSERT INTO api_tokens (id, user_id, token_hash, device_name, expires_at) \
         VALUES (?,?,?,?,?)",
    )
    .bind(&id)
    .bind(user_id)
    .bind(&token_hash)
    .bind(SESSION_DEVICE_NAME)
    .bind(&expires_at)
    .execute(pool)
    .await?;

    Ok(token)
}

/// Exact match first; a case-insensitive match only counts when it is the only
/// one. `users.username` is UNIQUE but case-sensitively so, meaning `Ana` and
/// `ana` can both exist. Rare, but if they do, "whichever `COLLATE NOCASE`
/// returns first" would be a coin flip deciding whose account you enter.
async fn lookup_user(
    pool: &sqlx::SqlitePool,
    username: &str,
) -> anyhow::Result<Option<(String, String, bool, String)>> {
    let exact: Option<(String, String, i64, String)> = sqlx::query_as(
        "SELECT id, username, is_admin, password_hash FROM users WHERE username = ?",
    )
    .bind(username)
    .fetch_optional(pool)
    .await?;
    if let Some(r) = exact {
        return Ok(Some((r.0, r.1, r.2 != 0, r.3)));
    }

    let loose: Vec<(String, String, i64, String)> = sqlx::query_as(
        "SELECT id, username, is_admin, password_hash FROM users \
         WHERE username = ? COLLATE NOCASE LIMIT 2",
    )
    .bind(username)
    .fetch_all(pool)
    .await?;
    if loose.len() == 1 {
        let r = &loose[0];
        return Ok(Some((r.0.clone(), r.1.clone(), r.2 != 0, r.3.clone())));
    }
    Ok(None)
}

fn session_cookie(token: &str, ttl_secs: i64, secure: bool) -> String {
    let mut c =
        format!("{SESSION_COOKIE}={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age={ttl_secs}");
    if secure {
        c.push_str("; Secure");
    }
    c
}

/// `Secure` is set only when the request actually arrived over TLS. A LAN
/// instance on plain `http://192.168.1.x:12421` is the common self-hosted
/// shape, and a `Secure` cookie there is one the browser accepts and never
/// sends back: a login that silently does nothing.
fn is_secure(headers: &HeaderMap) -> bool {
    headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .map(|v| {
            v.split(',')
                .next()
                .unwrap_or("")
                .trim()
                .eq_ignore_ascii_case("https")
        })
        .unwrap_or(false)
}

fn cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    raw.split(';')
        .filter_map(|p| p.split_once('='))
        .find(|(k, _)| k.trim() == name)
        .map(|(_, v)| v.trim().to_string())
}

fn internal(e: anyhow::Error, what: &str) -> ApiError {
    tracing::error!(error = %e, "{what} failed");
    fail(StatusCode::INTERNAL_SERVER_ERROR, "internal")
}

/// A hash of random bytes, computed once, to spend the same argon2 time on an
/// unknown username as on a known one.
fn dummy_hash() -> &'static str {
    static H: OnceLock<String> = OnceLock::new();
    H.get_or_init(|| {
        let filler = hoard_core::hashing::generate_token();
        hoard_core::hashing::hash_password(&filler)
            .unwrap_or_else(|_| "$argon2id$v=19$m=19456,t=2,p=1$c2FsdA$aGFzaA".to_string())
    })
}

struct Failures {
    count: u32,
    first: Instant,
    /// The limit this key is counted against, so [`evict_if_full`] can keep a
    /// counter that is actively refusing someone over one that isn't.
    limit: u32,
}

/// How many (origin, account) counters the table holds before it starts
/// evicting. Each origin can only put `MAX_FAILURES_PER_ORIGIN + 1` keys in it
/// before its own budget runs out, so this is roughly "fifty attackers at
/// once", small enough to stay a rounding error in memory.
const MAX_TRACKED_KEYS: usize = 1024;

/// Keyed on (origin bucket, username) rather than username alone: keying on the
/// account would hand anyone a way to lock a user out by guessing badly on
/// purpose. It is not meant to stop a distributed attacker; argon2id at 19 MiB
/// is the wall there. What it stops is that same wall being used as a CPU lever
/// against the server, which is the cheaper attack.
///
/// The bucket is only as good as the address behind it, which is why
/// [`crate::clientip`] refuses to read `X-Forwarded-For` from an untrusted
/// peer: believing it made both counters below decorative, since a direct
/// caller could pick a new key on every attempt.
fn throttle() -> &'static Mutex<HashMap<String, Failures>> {
    static T: OnceLock<Mutex<HashMap<String, Failures>>> = OnceLock::new();
    T.get_or_init(|| Mutex::new(HashMap::new()))
}

/// `None` when this key still has budget; otherwise how long the door stays
/// shut. There is no "window of zero" case to handle, because [`PanelConfig::login_throttle`]
/// clamps to [`crate::config::MIN_LOGIN_THROTTLE_SECS`], and the callers go
/// through it.
///
/// [`PanelConfig::login_throttle`]: crate::config::PanelConfig::login_throttle
fn throttled_for(key: &str, limit: u32, window: Duration) -> Option<Duration> {
    let mut map = throttle().lock().ok()?;
    let entry = map.get(key)?;
    let elapsed = entry.first.elapsed();
    if entry.count >= limit && elapsed < window {
        return Some(window - elapsed);
    }
    if elapsed >= window {
        map.remove(key);
    }
    None
}

/// Count one wrong password against `key`. `limit` is stored with the counter
/// so eviction can tell a door that is shut from one that is merely ajar. The
/// two callers use different limits, and only the caller knows which.
fn record_failure(key: &str, limit: u32, window: Duration) {
    let Ok(mut map) = throttle().lock() else {
        return;
    };
    evict_if_full(&mut map, window);
    let entry = map.entry(key.to_string()).or_insert(Failures {
        count: 0,
        first: Instant::now(),
        limit,
    });
    if entry.first.elapsed() >= window {
        entry.count = 0;
        entry.first = Instant::now();
    }
    entry.limit = limit;
    entry.count += 1;
}

/// Keep the table bounded without handing an attacker a way to empty it.
///
/// It used to `clear()` when pruning wasn't enough, which is a reset button:
/// spend a few origins' budgets, overflow the table, and every counter in it,
/// the ones holding a door shut included, went away. Now expired entries go
/// first, then the ones still under their limit (oldest first, since they are
/// the closest to expiring anyway), and a counter that is actively refusing
/// someone is the last thing dropped.
fn evict_if_full(map: &mut HashMap<String, Failures>, window: Duration) {
    if map.len() <= MAX_TRACKED_KEYS {
        return;
    }
    map.retain(|_, v| v.first.elapsed() < window);
    if map.len() <= MAX_TRACKED_KEYS {
        return;
    }
    let mut order: Vec<(bool, Instant, String)> = map
        .iter()
        .map(|(k, v)| (v.count >= v.limit, v.first, k.clone()))
        .collect();
    // `false` sorts before `true`, so the unblocked go first, oldest of each.
    order.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    for (_, _, k) in order.into_iter().take(map.len() - MAX_TRACKED_KEYS) {
        map.remove(&k);
    }
}

fn clear_failures(key: &str) {
    if let Ok(mut map) = throttle().lock() {
        map.remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cookie_carries_secure_only_behind_tls() {
        let plain = session_cookie("hoard_v1_x", 60, false);
        assert!(plain.contains("HttpOnly"));
        assert!(plain.contains("SameSite=Strict"));
        assert!(!plain.contains("Secure"));
        assert!(session_cookie("hoard_v1_x", 60, true).contains("; Secure"));
    }

    #[test]
    fn forwarded_proto_list_is_read_from_the_left() {
        let mut h = HeaderMap::new();
        h.insert("x-forwarded-proto", "https, http".parse().unwrap());
        assert!(is_secure(&h));
        h.insert("x-forwarded-proto", "http".parse().unwrap());
        assert!(!is_secure(&h));
        assert!(!is_secure(&HeaderMap::new()));
    }

    const W: Duration = Duration::from_secs(10);

    #[test]
    fn failures_lock_the_door_and_success_unlocks_it() {
        let key = "test|door";
        clear_failures(key);
        for _ in 0..MAX_FAILURES {
            assert!(throttled_for(key, MAX_FAILURES, W).is_none());
            record_failure(key, MAX_FAILURES, W);
        }
        assert!(throttled_for(key, MAX_FAILURES, W).is_some());
        clear_failures(key);
        assert!(throttled_for(key, MAX_FAILURES, W).is_none());
    }

    /// The floor is what the request path actually uses. Asking for no throttle
    /// at all used to work (the handler read the raw field) and the boot log
    /// said the minimum was in force while it wasn't.
    #[test]
    fn the_configured_window_never_goes_below_the_floor() {
        use crate::config::{PanelConfig, MIN_LOGIN_THROTTLE_SECS};
        let off = PanelConfig {
            login_throttle_secs: 0,
            ..PanelConfig::default()
        };
        assert_eq!(
            off.login_throttle(),
            Duration::from_secs(MIN_LOGIN_THROTTLE_SECS)
        );
        assert!(off.login_throttle_was_raised());

        let asked = PanelConfig {
            login_throttle_secs: 45,
            ..PanelConfig::default()
        };
        assert_eq!(asked.login_throttle(), Duration::from_secs(45));
        assert!(!asked.login_throttle_was_raised());
    }

    /// Filling the table must not reset the doors already shut. Clearing it
    /// wholesale was a way to buy an unlimited number of guesses: spend the
    /// budget from enough addresses, flush, start over.
    #[test]
    fn overflowing_the_table_keeps_the_newest_counters() {
        let held = "test-evict|held";
        clear_failures(held);
        for _ in 0..MAX_FAILURES {
            record_failure(held, MAX_FAILURES, W);
        }
        assert!(throttled_for(held, MAX_FAILURES, W).is_some());

        // The victim is the oldest, and `held` was just written, so it stays.
        for i in 0..MAX_TRACKED_KEYS + 50 {
            record_failure(&format!("test-evict|filler{i}"), MAX_FAILURES, W);
        }
        assert!(
            throttled_for(held, MAX_FAILURES, W).is_some(),
            "the flood reopened a door that was shut"
        );
        clear_failures(held);
    }

    /// Guessing against a different username each time must still run out of
    /// budget: the per-account counter never fills, so the origin's is the one
    /// doing the work.
    #[test]
    fn rotating_the_username_does_not_dodge_the_limit() {
        let origin = "test-rotate|*";
        clear_failures(origin);
        for i in 0..MAX_FAILURES_PER_ORIGIN {
            let account = format!("test-rotate|user{i}");
            assert!(throttled_for(&account, MAX_FAILURES, W).is_none());
            assert!(
                throttled_for(origin, MAX_FAILURES_PER_ORIGIN, W).is_none(),
                "locked out after {i} attempts, before the budget was spent"
            );
            record_failure(&account, MAX_FAILURES, W);
            record_failure(origin, MAX_FAILURES_PER_ORIGIN, W);
        }
        assert!(throttled_for(origin, MAX_FAILURES_PER_ORIGIN, W).is_some());
        clear_failures(origin);
    }
}
