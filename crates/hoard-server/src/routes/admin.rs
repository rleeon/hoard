//! Admin-only operational endpoints for self-hosted instances.
//!
//! These routes are mounted **only** by `run_self_hosted` in `main.rs`,
//! never by the cloud router (`cloud/run.rs`), so they don't exist on the
//! managed Fly.io instance. They sit behind the same `require_auth`
//! middleware as everything else and additionally require
//! `AuthUser.is_admin`.
//!
//! See ADR 0017 for the full design of remote-triggered server upgrades.

use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::auth::AuthUser;
use crate::routes::health::ServerState;
use crate::routes::session::SESSION_DEVICE_NAME;

/// Filename of the upgrade marker, dropped in the server's writable
/// `data_dir`. A root systemd path-unit (`hoard-upgrade.path`) watches for
/// it and runs the privileged upgrade. Keep in sync with
/// `deploy/systemd/hoard-upgrade.path`.
pub const UPGRADE_MARKER: &str = ".upgrade-requested";

#[derive(Serialize)]
pub struct UpgradeAck {
    pub status: &'static str,
}

/// `POST /v1/admin/upgrade`: request a self-upgrade of this server.
///
/// The web process is sandboxed (see `deploy/systemd/hoard-server.service`)
/// and deliberately cannot touch its own binary. All it does here is drop a
/// marker file in `data_dir`; the root oneshot does the download + signature
/// check + binary swap + restart. We **ignore any request body**: the
/// privileged side always installs the latest *signed* canonical release, so
/// even a forged request can't choose what gets installed.
pub async fn upgrade(
    Extension(user): Extension<AuthUser>,
    State(state): State<Arc<ServerState>>,
) -> Result<(StatusCode, Json<UpgradeAck>), (StatusCode, String)> {
    if !user.is_admin {
        return Err((
            StatusCode::FORBIDDEN,
            "remote upgrade requires an admin token".to_string(),
        ));
    }
    if !state.config.server.allow_remote_upgrade {
        return Err((
            StatusCode::FORBIDDEN,
            "remote upgrade is disabled on this server (server.allow_remote_upgrade = false)"
                .to_string(),
        ));
    }

    let marker = state.config.storage.data_dir.join(UPGRADE_MARKER);
    // Content is informational only; the oneshot reads nothing from it.
    let body = format!(
        "requested_by={}\nrequested_at={}\n",
        user.username,
        now_unix(),
    );
    tokio::fs::write(&marker, body).await.map_err(|e| {
        tracing::error!(error = %e, path = %marker.display(), "failed to write upgrade marker");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "could not schedule the upgrade".to_string(),
        )
    })?;

    tracing::warn!(
        requested_by = %user.username,
        marker = %marker.display(),
        "remote upgrade scheduled; root oneshot will pick it up"
    );

    Ok((
        StatusCode::ACCEPTED,
        Json(UpgradeAck {
            status: "scheduled",
        }),
    ))
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Operator views behind the panel's "server" section.
//
// Everything below is `is_admin`-only and self-hosted-only. The gate lives in
// each handler rather than in a layer because these hang off the same authed
// router as the rest, and a second middleware stack that could drift out of
// sync with this one is a worse trade than five explicit checks.
//
// What is deliberately NOT here: deleting a user, migrating storage backends,
// verifying every object. Those are `hoard-admin` subcommands and they stay
// there: each one is long-running or irreversible, and both properties are
// better served by a terminal that can print progress and refuse to be closed
// than by a browser tab.
// ---------------------------------------------------------------------------

type ApiError = (StatusCode, Json<serde_json::Value>);

/// `(id, user_id, username, device_name, created_at, last_used_at, expires_at,
/// revoked_at)` straight from the join.
type TokenRecord = (
    String,
    String,
    String,
    Option<String>,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
);

/// `(at, username, level, target, message, device_name, device_os,
/// app_version, fields)`.
type LogRecord = (
    String,
    String,
    String,
    Option<String>,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

fn err(status: StatusCode, code: &str) -> ApiError {
    (status, Json(serde_json::json!({ "error": code })))
}

fn db_error(e: sqlx::Error, what: &str) -> ApiError {
    tracing::error!(error = %e, "{what} failed");
    err(StatusCode::INTERNAL_SERVER_ERROR, "internal")
}

fn require_admin(user: &AuthUser) -> Result<(), ApiError> {
    if user.is_admin {
        Ok(())
    } else {
        Err(err(StatusCode::FORBIDDEN, "admin_only"))
    }
}

#[derive(Serialize)]
pub struct AdminOverview {
    pub totals: Totals,
    pub users: Vec<UserRow>,
}

#[derive(Serialize)]
pub struct Totals {
    pub users: i64,
    pub admins: i64,
    pub saves: i64,
    pub versions: i64,
    pub trashed_versions: i64,
    pub logical_bytes: i64,
    pub stored_bytes: i64,
    pub trash_bytes: i64,
    /// Blobs and chunks nothing references any more. They are already spent
    /// disk; the hourly cleanup collects them. A number that only grows means
    /// the sweep is failing, which is the kind of thing an operator can only
    /// notice if someone shows it to them.
    pub orphan_objects: i64,
    pub orphan_bytes: i64,
    pub objects: i64,
    /// Size of the SQLite file plus its write-ahead log, or `null` when the
    /// database is not a local file we can stat (a Postgres cloud deployment
    /// never mounts these routes, so in practice: an unusual URL).
    pub db_bytes: Option<i64>,
    pub client_logs: i64,
    pub oldest_snapshot_at: Option<String>,
}

#[derive(Serialize)]
pub struct UserRow {
    pub id: String,
    pub username: String,
    pub is_admin: bool,
    pub used_bytes: i64,
    pub quota_bytes: i64,
    pub stored_bytes: i64,
    pub saves: i64,
    pub versions: i64,
    pub devices: i64,
    pub last_seen_at: Option<String>,
    pub created_at: String,
}

/// `GET /v1/admin/overview`
pub async fn overview(
    Extension(user): Extension<AuthUser>,
    State(state): State<Arc<ServerState>>,
) -> Result<Json<AdminOverview>, ApiError> {
    require_admin(&user)?;
    let pool = &state.pool;

    let (users, admins): (i64, i64) =
        sqlx::query_as("SELECT COUNT(*), COALESCE(SUM(is_admin),0) FROM users")
            .fetch_one(pool)
            .await
            .map_err(|e| db_error(e, "admin user counts"))?;

    let (saves,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM saves")
        .fetch_one(pool)
        .await
        .map_err(|e| db_error(e, "admin save count"))?;

    let (versions, logical_bytes): (i64, i64) = sqlx::query_as(
        "SELECT COUNT(*), COALESCE(SUM(total_size_bytes),0) FROM snapshots \
         WHERE deleted_at IS NULL",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| db_error(e, "admin snapshot totals"))?;

    let (trashed_versions, trash_bytes): (i64, i64) = sqlx::query_as(
        "SELECT COUNT(*), COALESCE(SUM(total_size_bytes),0) FROM snapshots \
         WHERE deleted_at IS NOT NULL",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| db_error(e, "admin trash totals"))?;

    let (objects, stored_bytes, orphan_objects, orphan_bytes): (i64, i64, i64, i64) =
        sqlx::query_as(
            "SELECT (SELECT COUNT(*) FROM blobs) + (SELECT COUNT(*) FROM chunks), \
                    (SELECT COALESCE(SUM(size_bytes),0) FROM blobs) \
                  + (SELECT COALESCE(SUM(size_bytes),0) FROM chunks), \
                    (SELECT COUNT(*) FROM blobs WHERE refcount <= 0) \
                  + (SELECT COUNT(*) FROM chunks WHERE refcount <= 0), \
                    (SELECT COALESCE(SUM(size_bytes),0) FROM blobs WHERE refcount <= 0) \
                  + (SELECT COALESCE(SUM(size_bytes),0) FROM chunks WHERE refcount <= 0)",
        )
        .fetch_one(pool)
        .await
        .map_err(|e| db_error(e, "admin object totals"))?;

    let (client_logs,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM client_logs")
        .fetch_one(pool)
        .await
        .map_err(|e| db_error(e, "admin log count"))?;

    let (oldest_snapshot_at,): (Option<String>,) =
        sqlx::query_as("SELECT MIN(created_at) FROM snapshots")
            .fetch_one(pool)
            .await
            .map_err(|e| db_error(e, "admin oldest snapshot"))?;

    let rows: Vec<(String, String, i64, i64, i64, String)> = sqlx::query_as(
        "SELECT id, username, is_admin, storage_used_bytes, storage_quota_bytes, created_at \
         FROM users ORDER BY username COLLATE NOCASE",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| db_error(e, "admin user list"))?;

    let mut users_out = Vec::with_capacity(rows.len());
    for (id, username, admin_flag, used, quota, created_at) in rows {
        // Per-user rollups one query at a time. A self-hosted instance has a
        // handful of users, and the readable version wins over a five-way join
        // that has to fake outer-join semantics for users with no saves yet.
        let (saves, versions): (i64, i64) = sqlx::query_as(
            "SELECT COUNT(DISTINCT s.id), \
                    COUNT(CASE WHEN sn.deleted_at IS NULL THEN sn.id END) \
             FROM saves s LEFT JOIN snapshots sn ON sn.save_id = s.id \
             WHERE s.user_id = ?",
        )
        .bind(&id)
        .fetch_one(pool)
        .await
        .map_err(|e| db_error(e, "admin per-user saves"))?;

        let (stored_bytes,): (i64,) = sqlx::query_as(
            "SELECT (SELECT COALESCE(SUM(size_bytes),0) FROM blobs WHERE user_id = ?1) \
                  + (SELECT COALESCE(SUM(size_bytes),0) FROM chunks WHERE user_id = ?1)",
        )
        .bind(&id)
        .fetch_one(pool)
        .await
        .map_err(|e| db_error(e, "admin per-user bytes"))?;

        let (devices, last_seen_at): (i64, Option<String>) =
            sqlx::query_as("SELECT COUNT(*), MAX(last_seen_at) FROM devices WHERE user_id = ?")
                .bind(&id)
                .fetch_one(pool)
                .await
                .map_err(|e| db_error(e, "admin per-user devices"))?;

        users_out.push(UserRow {
            id,
            username,
            is_admin: admin_flag != 0,
            used_bytes: used,
            quota_bytes: quota,
            stored_bytes,
            saves,
            versions,
            devices,
            last_seen_at,
            created_at,
        });
    }

    Ok(Json(AdminOverview {
        totals: Totals {
            users,
            admins,
            saves,
            versions,
            trashed_versions,
            logical_bytes,
            stored_bytes,
            trash_bytes,
            orphan_objects,
            orphan_bytes,
            objects,
            db_bytes: db_file_bytes(&state.config.database.url),
            client_logs,
            oldest_snapshot_at,
        },
        users: users_out,
    }))
}

/// Shortest password the panel and `hoard-admin user create` both accept.
/// Kept in one place because the two used to disagree.
pub const MIN_PASSWORD_LEN: usize = 8;

/// Longest username. Not a storage limit (the column is TEXT) but a name
/// that does not fit any table cell it appears in is a support ticket.
const MAX_USERNAME_LEN: usize = 64;

/// A username has to survive being typed into `hoard-admin token create <name>`
/// and into a URL, so it is deliberately narrow: letters, digits, and the three
/// separators that do not need quoting in a shell.
fn valid_username(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_USERNAME_LEN
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// True when the error is SQLite's UNIQUE violation, which for every statement
/// here means the username is taken.
fn is_unique_violation(e: &sqlx::Error) -> bool {
    matches!(e, sqlx::Error::Database(d) if d.code().as_deref() == Some("2067") || d.is_unique_violation())
}

#[derive(Deserialize)]
pub struct UserPatch {
    pub is_admin: Option<bool>,
    pub storage_quota_bytes: Option<i64>,
    /// New username. Rename is a pure metadata change: everything on disk and
    /// every foreign key hangs off the user's id, so not a byte moves.
    pub username: Option<String>,
    /// New password, in the clear over the same TLS the login already uses.
    /// Hashed here with the same function `hoard-admin user passwd` calls.
    pub password: Option<String>,
}

/// `PATCH /v1/admin/users/:id`: flip the admin bit, move a quota, rename, or
/// set a password.
///
/// The first three are reversible from this same screen. The one irreversible
/// move it refuses is removing the last admin: the flag guards its own route,
/// so a server with zero admins cannot promote anyone back without
/// `hoard-admin` and a shell on the box.
pub async fn patch_user(
    Extension(user): Extension<AuthUser>,
    State(state): State<Arc<ServerState>>,
    Path(target_id): Path<String>,
    Json(body): Json<UserPatch>,
) -> Result<StatusCode, ApiError> {
    require_admin(&user)?;

    let existing: Option<(String, i64)> =
        sqlx::query_as("SELECT username, is_admin FROM users WHERE id = ?")
            .bind(&target_id)
            .fetch_optional(&state.pool)
            .await
            .map_err(|e| db_error(e, "admin patch lookup"))?;
    let Some((target_name, was_admin)) = existing else {
        return Err(err(StatusCode::NOT_FOUND, "no_such_user"));
    };

    if let Some(quota) = body.storage_quota_bytes {
        if quota < 0 {
            return Err(err(StatusCode::BAD_REQUEST, "bad_quota"));
        }
        sqlx::query("UPDATE users SET storage_quota_bytes = ? WHERE id = ?")
            .bind(quota)
            .bind(&target_id)
            .execute(&state.pool)
            .await
            .map_err(|e| db_error(e, "admin quota update"))?;
        tracing::info!(actor = %user.username, target = %target_name, quota, "admin: quota set");
    }

    if let Some(make_admin) = body.is_admin {
        if !make_admin && was_admin != 0 {
            let (admins,): (i64,) =
                sqlx::query_as("SELECT COUNT(*) FROM users WHERE is_admin <> 0")
                    .fetch_one(&state.pool)
                    .await
                    .map_err(|e| db_error(e, "admin count"))?;
            if admins <= 1 {
                return Err(err(StatusCode::CONFLICT, "last_admin"));
            }
        }
        sqlx::query("UPDATE users SET is_admin = ? WHERE id = ?")
            .bind(make_admin as i64)
            .bind(&target_id)
            .execute(&state.pool)
            .await
            .map_err(|e| db_error(e, "admin flag update"))?;
        tracing::info!(actor = %user.username, target = %target_name, make_admin, "admin: role set");
    }

    if let Some(new_name) = body.username.as_deref().map(str::trim) {
        if !valid_username(new_name) {
            return Err(err(StatusCode::BAD_REQUEST, "bad_username"));
        }
        if new_name != target_name {
            sqlx::query("UPDATE users SET username = ? WHERE id = ?")
                .bind(new_name)
                .bind(&target_id)
                .execute(&state.pool)
                .await
                .map_err(|e| {
                    if is_unique_violation(&e) {
                        err(StatusCode::CONFLICT, "username_taken")
                    } else {
                        db_error(e, "admin rename")
                    }
                })?;
            tracing::info!(actor = %user.username, from = %target_name, to = %new_name, "admin: user renamed");
        }
    }

    if let Some(password) = body.password.as_deref() {
        if password.len() < MIN_PASSWORD_LEN {
            return Err(err(StatusCode::BAD_REQUEST, "password_too_short"));
        }
        let hash = hoard_core::hashing::hash_password(password).map_err(|e| {
            tracing::error!(error = %e, "admin password hash failed");
            err(StatusCode::INTERNAL_SERVER_ERROR, "internal")
        })?;
        sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
            .bind(&hash)
            .bind(&target_id)
            .execute(&state.pool)
            .await
            .map_err(|e| db_error(e, "admin password update"))?;

        // Browser sessions die with the old password; device tokens do not.
        // A token is how a PC syncs, and nobody asking for a new password is
        // asking for their machines to stop backing up until they re-pair each
        // one. `SESSION_DEVICE_NAME` is what tells the two apart.
        let killed = sqlx::query(
            "UPDATE api_tokens SET revoked_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') \
             WHERE user_id = ? AND device_name = ? AND revoked_at IS NULL",
        )
        .bind(&target_id)
        .bind(SESSION_DEVICE_NAME)
        .execute(&state.pool)
        .await
        .map_err(|e| db_error(e, "admin session revoke"))?
        .rows_affected();
        tracing::info!(actor = %user.username, target = %target_name, sessions_revoked = killed, "admin: password set");
    }

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct NewUser {
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub is_admin: bool,
    /// Optional per-user quota in bytes. Absent leaves the column at its
    /// default, which is what `hoard-admin user create` does.
    pub storage_quota_bytes: Option<i64>,
}

#[derive(Serialize)]
pub struct CreatedUser {
    pub id: String,
    pub username: String,
}

/// `POST /v1/admin/users`: create an account.
///
/// Same three columns `hoard-admin user create` writes, through the same
/// `hash_password`. It exists so that adding the second person to a NAS does
/// not need a shell on the box: the container's `HOARD_ADMIN_*` variables only
/// ever create the first one.
pub async fn create_user(
    Extension(user): Extension<AuthUser>,
    State(state): State<Arc<ServerState>>,
    Json(body): Json<NewUser>,
) -> Result<(StatusCode, Json<CreatedUser>), ApiError> {
    require_admin(&user)?;

    let username = body.username.trim().to_string();
    if !valid_username(&username) {
        return Err(err(StatusCode::BAD_REQUEST, "bad_username"));
    }
    if body.password.len() < MIN_PASSWORD_LEN {
        return Err(err(StatusCode::BAD_REQUEST, "password_too_short"));
    }
    if body.storage_quota_bytes.is_some_and(|q| q < 0) {
        return Err(err(StatusCode::BAD_REQUEST, "bad_quota"));
    }

    let hash = hoard_core::hashing::hash_password(&body.password).map_err(|e| {
        tracing::error!(error = %e, "admin password hash failed");
        err(StatusCode::INTERNAL_SERVER_ERROR, "internal")
    })?;
    let id = uuid::Uuid::new_v4().to_string();

    sqlx::query("INSERT INTO users (id, username, password_hash, is_admin) VALUES (?,?,?,?)")
        .bind(&id)
        .bind(&username)
        .bind(&hash)
        .bind(body.is_admin as i64)
        .execute(&state.pool)
        .await
        .map_err(|e| {
            if is_unique_violation(&e) {
                err(StatusCode::CONFLICT, "username_taken")
            } else {
                db_error(e, "admin user create")
            }
        })?;

    if let Some(quota) = body.storage_quota_bytes {
        sqlx::query("UPDATE users SET storage_quota_bytes = ? WHERE id = ?")
            .bind(quota)
            .bind(&id)
            .execute(&state.pool)
            .await
            .map_err(|e| db_error(e, "admin quota on create"))?;
    }

    tracing::info!(actor = %user.username, target = %username, is_admin = body.is_admin, "admin: user created");
    Ok((StatusCode::CREATED, Json(CreatedUser { id, username })))
}

#[derive(Serialize)]
pub struct DeletedUser {
    pub username: String,
    pub objects_removed: u64,
    pub bytes_removed: i64,
}

/// `DELETE /v1/admin/users/:id`: remove an account and everything it stored.
///
/// Irreversible, and the only route here that destroys data. It refuses one
/// case: **deleting yourself**. The confirmation sits a click away from the
/// account you are signed in as, and a panel that logs you out mid-request
/// cannot tell you what happened.
///
/// That single check is also what keeps the server from reaching zero admins,
/// which would need a shell to undo, since the admin flag guards its own route. No
/// separate last-admin count is needed here, unlike when demoting: the caller
/// is an admin, the target is somebody else, so there are at least two. The
/// CLI has no "yourself" to refuse and does count, in `hoard-admin user
/// delete`.
///
/// Stored objects go **before** the row, because the `ON DELETE CASCADE` on
/// `blobs`/`chunks` takes the only record of which keys were theirs. See
/// [`store::purge_user_objects`](crate::store::purge_user_objects).
pub async fn delete_user(
    Extension(user): Extension<AuthUser>,
    State(state): State<Arc<ServerState>>,
    Path(target_id): Path<String>,
) -> Result<Json<DeletedUser>, ApiError> {
    require_admin(&user)?;

    if target_id == user.user_id.to_string() {
        return Err(err(StatusCode::CONFLICT, "cannot_delete_self"));
    }

    let existing: Option<(String,)> = sqlx::query_as("SELECT username FROM users WHERE id = ?")
        .bind(&target_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| db_error(e, "admin delete lookup"))?;
    let Some((target_name,)) = existing else {
        return Err(err(StatusCode::NOT_FOUND, "no_such_user"));
    };

    let (objects_removed, bytes_removed) =
        crate::store::purge_user_objects(&state.pool, &state.store, &target_id)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "admin user purge failed");
                err(StatusCode::INTERNAL_SERVER_ERROR, "internal")
            })?;

    sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(&target_id)
        .execute(&state.pool)
        .await
        .map_err(|e| db_error(e, "admin user delete"))?;

    tracing::warn!(
        actor = %user.username,
        target = %target_name,
        objects_removed,
        bytes_removed,
        "admin: user deleted"
    );
    Ok(Json(DeletedUser {
        username: target_name,
        objects_removed,
        bytes_removed,
    }))
}

#[derive(Serialize)]
pub struct TokenRow {
    pub id: String,
    pub user_id: String,
    pub username: String,
    pub device_name: Option<String>,
    /// True when this row is a browser session rather than a device's token.
    pub is_session: bool,
    pub created_at: String,
    pub last_used_at: Option<String>,
    pub expires_at: Option<String>,
    pub revoked_at: Option<String>,
}

#[derive(Deserialize)]
pub struct TokenQuery {
    pub user_id: Option<String>,
    /// Revoked tokens are hidden by default; they are audit trail, not state.
    #[serde(default)]
    pub include_revoked: bool,
}

/// `GET /v1/admin/tokens`
pub async fn tokens(
    Extension(user): Extension<AuthUser>,
    State(state): State<Arc<ServerState>>,
    Query(q): Query<TokenQuery>,
) -> Result<Json<Vec<TokenRow>>, ApiError> {
    require_admin(&user)?;

    let rows: Vec<TokenRecord> = sqlx::query_as(
        "SELECT t.id, t.user_id, u.username, t.device_name, t.created_at, \
                t.last_used_at, t.expires_at, t.revoked_at \
         FROM api_tokens t JOIN users u ON u.id = t.user_id \
         WHERE (?1 IS NULL OR t.user_id = ?1) \
           AND (?2 = 1 OR t.revoked_at IS NULL) \
         ORDER BY t.created_at DESC",
    )
    .bind(&q.user_id)
    .bind(q.include_revoked as i64)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| db_error(e, "admin token list"))?;

    Ok(Json(
        rows.into_iter()
            .map(
                |(id, user_id, username, device_name, created_at, last, exp, rev)| TokenRow {
                    is_session: device_name.as_deref() == Some(SESSION_DEVICE_NAME),
                    id,
                    user_id,
                    username,
                    device_name,
                    created_at,
                    last_used_at: last,
                    expires_at: exp,
                    revoked_at: rev,
                },
            )
            .collect(),
    ))
}

#[derive(Deserialize)]
pub struct NewToken {
    pub user_id: String,
    /// Label shown in the token list and in the desktop app's device list.
    pub device_name: Option<String>,
}

#[derive(Serialize)]
pub struct MintedToken {
    pub id: String,
    /// The token in the clear. The only time it is ever readable: only its
    /// SHA-256 is stored, so a caller that loses this has to mint another.
    pub token: String,
    pub username: String,
    pub device_name: Option<String>,
    pub expires_at: Option<String>,
}

/// `POST /v1/admin/tokens`: mint a device token.
///
/// The gap this closes: the container prints a token for the first PC on first
/// boot, and every PC after that needed `hoard-admin token create` from a shell
/// on the server. On a NAS appliance that means the Docker console, which is
/// exactly the audience least likely to open one.
pub async fn create_token(
    Extension(user): Extension<AuthUser>,
    State(state): State<Arc<ServerState>>,
    Json(body): Json<NewToken>,
) -> Result<(StatusCode, Json<MintedToken>), ApiError> {
    require_admin(&user)?;

    let username: Option<(String,)> = sqlx::query_as("SELECT username FROM users WHERE id = ?")
        .bind(&body.user_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| db_error(e, "admin token user lookup"))?;
    let Some((username,)) = username else {
        return Err(err(StatusCode::NOT_FOUND, "no_such_user"));
    };

    // A device called "web panel" would show up as a browser session in the
    // list above and could not be told apart from one afterwards.
    let device_name = body
        .device_name
        .as_deref()
        .map(str::trim)
        .filter(|d| !d.is_empty())
        .map(str::to_string);
    if device_name.as_deref() == Some(SESSION_DEVICE_NAME) {
        return Err(err(StatusCode::BAD_REQUEST, "reserved_device_name"));
    }

    let token = hoard_core::hashing::generate_token();
    let token_hash = hoard_core::hashing::hash_token(&token);
    let id = uuid::Uuid::new_v4().to_string();

    let expires_at = if state.config.auth.token_lifetime_days > 0 {
        let exp = time::OffsetDateTime::now_utc()
            + time::Duration::days(state.config.auth.token_lifetime_days as i64);
        Some(
            exp.format(&time::format_description::well_known::Rfc3339)
                .map_err(|e| {
                    tracing::error!(error = %e, "token expiry format failed");
                    err(StatusCode::INTERNAL_SERVER_ERROR, "internal")
                })?,
        )
    } else {
        None
    };

    sqlx::query(
        "INSERT INTO api_tokens (id, user_id, token_hash, device_name, expires_at) \
         VALUES (?,?,?,?,?)",
    )
    .bind(&id)
    .bind(&body.user_id)
    .bind(&token_hash)
    .bind(&device_name)
    .bind(&expires_at)
    .execute(&state.pool)
    .await
    .map_err(|e| db_error(e, "admin token create"))?;

    tracing::info!(actor = %user.username, target = %username, device = ?device_name, "admin: token minted");
    Ok((
        StatusCode::CREATED,
        Json(MintedToken {
            id,
            token,
            username,
            device_name,
            expires_at,
        }),
    ))
}

/// `POST /v1/admin/tokens/:id/revoke`
///
/// The token itself is never readable here (only its SHA-256 is stored) so
/// revocation goes by row id. Revoking your own session is allowed and logs you
/// out on the next request, which is the correct behaviour for "I clicked the
/// wrong row".
pub async fn revoke_token(
    Extension(user): Extension<AuthUser>,
    State(state): State<Arc<ServerState>>,
    Path(token_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    require_admin(&user)?;

    let affected = sqlx::query(
        "UPDATE api_tokens SET revoked_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') \
         WHERE id = ? AND revoked_at IS NULL",
    )
    .bind(&token_id)
    .execute(&state.pool)
    .await
    .map_err(|e| db_error(e, "admin token revoke"))?
    .rows_affected();

    if affected == 0 {
        return Err(err(StatusCode::NOT_FOUND, "no_such_token"));
    }
    tracing::info!(actor = %user.username, token_id, "admin: token revoked");
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize)]
pub struct LogRow {
    pub at: String,
    pub username: String,
    pub level: String,
    pub target: Option<String>,
    pub message: String,
    pub device_name: Option<String>,
    pub device_os: Option<String>,
    pub app_version: Option<String>,
    pub fields: Option<String>,
}

#[derive(Deserialize)]
pub struct LogQuery {
    pub user_id: Option<String>,
    pub level: Option<String>,
    pub q: Option<String>,
    pub limit: Option<i64>,
}

/// `GET /v1/admin/logs`
///
/// First reader `client_logs` has ever had. Clients have been shipping their
/// diagnostics to the server since migration 0012 and the only code that
/// touched the table was the retention sweep deleting them, and an operator
/// debugging a device had to open SQLite by hand.
pub async fn logs(
    Extension(user): Extension<AuthUser>,
    State(state): State<Arc<ServerState>>,
    Query(q): Query<LogQuery>,
) -> Result<Json<Vec<LogRow>>, ApiError> {
    require_admin(&user)?;
    let limit = q.limit.unwrap_or(200).clamp(1, 1000);
    // Substring search, escaped so a user's `%` or `_` matches itself instead
    // of turning into a wildcard.
    let needle = q.q.as_ref().map(|raw| {
        format!(
            "%{}%",
            raw.replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_")
        )
    });

    let rows: Vec<LogRecord> = sqlx::query_as(
        "SELECT COALESCE(l.client_ts, l.received_at), u.username, l.level, l.target, \
                l.message, l.device_name, l.device_os, l.app_version, l.fields \
         FROM client_logs l JOIN users u ON u.id = l.user_id \
         WHERE (?1 IS NULL OR l.user_id = ?1) \
           AND (?2 IS NULL OR l.level = ?2) \
           AND (?3 IS NULL OR l.message LIKE ?3 ESCAPE '\\' OR l.target LIKE ?3 ESCAPE '\\') \
         ORDER BY l.received_at DESC LIMIT ?4",
    )
    .bind(&q.user_id)
    .bind(&q.level)
    .bind(&needle)
    .bind(limit)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| db_error(e, "admin logs"))?;

    Ok(Json(
        rows.into_iter()
            .map(
                |(at, username, level, target, message, dev, os, ver, fields)| LogRow {
                    at,
                    username,
                    level,
                    target,
                    message,
                    device_name: dev,
                    device_os: os,
                    app_version: ver,
                    fields,
                },
            )
            .collect(),
    ))
}

/// Size of the SQLite file and its `-wal` sidecar. The WAL is included because
/// on a busy instance it is routinely the larger of the two, and an operator
/// looking at "database" wants the number that explains their disk.
fn db_file_bytes(url: &str) -> Option<i64> {
    let path = url
        .strip_prefix("sqlite://")
        .or_else(|| url.strip_prefix("sqlite:"))?
        .split('?')
        .next()?;
    if path.is_empty() || path == ":memory:" {
        return None;
    }
    let main = std::fs::metadata(path).ok()?.len() as i64;
    let wal = std::fs::metadata(format!("{path}-wal"))
        .map(|m| m.len() as i64)
        .unwrap_or(0);
    Some(main + wal)
}

#[cfg(test)]
mod db_path_tests {
    use super::db_file_bytes;

    #[test]
    fn unusual_urls_report_nothing_rather_than_a_wrong_number() {
        assert!(db_file_bytes("sqlite::memory:").is_none());
        assert!(db_file_bytes("postgres://localhost/hoard").is_none());
        assert!(db_file_bytes("sqlite:///nonexistent/hoard.db").is_none());
    }
}
