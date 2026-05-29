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
    extract::{Extension, State},
    http::StatusCode,
    response::Json,
};
use serde::Serialize;
use std::sync::Arc;

use crate::auth::AuthUser;
use crate::routes::health::ServerState;

/// Filename of the upgrade marker, dropped in the server's writable
/// `data_dir`. A root systemd path-unit (`hoard-upgrade.path`) watches for
/// it and runs the privileged upgrade. Keep in sync with
/// `deploy/systemd/hoard-upgrade.path`.
pub const UPGRADE_MARKER: &str = ".upgrade-requested";

#[derive(Serialize)]
pub struct UpgradeAck {
    pub status: &'static str,
}

/// `POST /v1/admin/upgrade` — request a self-upgrade of this server.
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
    // Content is informational only — the oneshot reads nothing from it.
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

    Ok((StatusCode::ACCEPTED, Json(UpgradeAck { status: "scheduled" })))
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
