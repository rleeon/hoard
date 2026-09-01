//! The Cloud account: portable REST calls (export, the storage black box,
//! archive and reactivate, deleting and reactivating an account, entitlements,
//! features, playtime). There is no Tauri and no keyring here.
//!
//! Each function takes `(base, token)` already resolved and returns data
//! (`Result<_, CloudError>`). The desktop resolves credentials through its
//! Supabase session and keyring and maps the error to i18n; the CLI resolves them
//! through [`crate::cloud_auth`] and prints it. The retry-after-401 stays in each
//! frontend, because each refreshes the JWT differently.

use serde::{Deserialize, Serialize};

use crate::playtime::{PlaytimeRow, PlaytimeSummary};

// ---- error ------------------------------------------------------------

/// An error from a Cloud call. It keeps `status` and `body` in the HTTP case so
/// the desktop can reproduce the exact message it already showed, its `i18n:<key>`
/// mapping included, while the CLI settles for [`CloudError::message`].
#[derive(Debug)]
pub enum CloudError {
    /// 401: the JWT expired. The caller can refresh and retry.
    Unauthorized,
    /// Any other non-2xx, 402 payment-required included. `status` is the raw HTTP
    /// code.
    Http { status: u16, body: String },
    /// Error de red / transporte.
    Network(String),
    /// An unreadable response (JSON that does not parse).
    Parse(String),
}

impl CloudError {
    /// Neutral human text, with no i18n. The CLI uses it, and it is the desktop's
    /// fallback for the cases it does not intercept by code.
    pub fn message(&self) -> String {
        match self {
            CloudError::Unauthorized => "la sesión Cloud caducó, vuelve a iniciar sesión".into(),
            CloudError::Http { status, body } => {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(body) {
                    if let Some(msg) = v.get("error").and_then(|x| x.as_str()) {
                        return format!("Hoard Cloud: {msg} ({status})");
                    }
                }
                format!("Hoard Cloud devolvió {status}: {body}")
            }
            CloudError::Network(m) | CloudError::Parse(m) => m.clone(),
        }
    }
}

impl std::fmt::Display for CloudError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message())
    }
}

impl std::error::Error for CloudError {}

// ---- helpers HTTP -----------------------------------------------------

fn http_client() -> Result<reqwest::Client, CloudError> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent(concat!("hoard-agent/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| CloudError::Network(e.to_string()))
}

/// Turns an unsuccessful response into a [`CloudError`], singling out the 401.
async fn into_error(resp: reqwest::Response) -> CloudError {
    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED {
        return CloudError::Unauthorized;
    }
    let body = resp.text().await.unwrap_or_default();
    CloudError::Http {
        status: status.as_u16(),
        body,
    }
}

fn parse_json<T: serde::de::DeserializeOwned>(body: &str) -> Result<T, CloudError> {
    serde_json::from_str::<T>(body)
        .map_err(|e| CloudError::Parse(format!("parseando respuesta Cloud: {e}: {body}")))
}

// ---- export -----------------------------------------------------------

/// A server-side export job. The worker builds the ZIP and the client polls
/// [`export_status`] until the download link appears.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportJob {
    pub job_id: String,
    pub status: String,
}

/// The last export job's state, with a presigned `download_url` once the ZIP is
/// ready. Every field is `None` when the user never exported.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExportStatus {
    pub job_id: Option<String>,
    pub status: Option<String>,
    pub requested_at: Option<String>,
    pub size_bytes: Option<i64>,
    pub expires_at: Option<String>,
    pub download_url: Option<String>,
    pub error: Option<String>,
}

/// `POST {base}/v1/me/export`: starts the export job.
pub async fn export_all(base: &str, token: &str) -> Result<ExportJob, CloudError> {
    let url = format!("{base}/v1/me/export");
    let resp = http_client()?
        .post(&url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| CloudError::Network(format!("Network error: {e}")))?;
    if !resp.status().is_success() {
        return Err(into_error(resp).await);
    }
    let body = resp.text().await.unwrap_or_default();
    parse_json(&body)
}

/// `GET {base}/v1/me/export`: the last job's state.
pub async fn export_status(base: &str, token: &str) -> Result<ExportStatus, CloudError> {
    let url = format!("{base}/v1/me/export");
    let resp = http_client()?
        .get(&url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| CloudError::Network(format!("Network error: {e}")))?;
    if !resp.status().is_success() {
        return Err(into_error(resp).await);
    }
    let body = resp.text().await.unwrap_or_default();
    parse_json(&body)
}

// ---- caja negra: storage / archived games -----------------------------

/// A save's freeable footprint. Mirrors the server's `GameFootprint`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageGame {
    pub save_id: String,
    pub game_slug: String,
    pub label: String,
    /// Bytes archiving would give back to the quota (deduplicated exclusive blobs).
    pub freeable_bytes: i64,
    #[serde(default)]
    pub archived: bool,
    /// RFC3339 instant of the final purge, present only while it is archived.
    #[serde(default)]
    pub purge_after: Option<String>,
}

/// `GET {base}/v1/cloud/storage/games`: per-save footprint plus quota figures.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageGames {
    pub plan: String,
    pub used_bytes: u64,
    pub limit_bytes: u64,
    /// Bytes over the limit (0 when inside it).
    pub over_bytes: u64,
    pub games: Vec<StorageGame>,
    /// Blobs two or more live saves share, grouped by the exact set sharing them.
    /// Those bytes are exclusive to none of them, so they appear in no
    /// `freeable_bytes`: they only come back if every save in the group is
    /// archived. The typical case is the same folder tracked twice.
    #[serde(default)]
    pub shared_groups: Vec<SharedGroup>,
}

/// A group of shared blobs and what they weigh. Mirrors the server's `SharedGroup`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedGroup {
    pub save_ids: Vec<String>,
    pub bytes: i64,
}

/// The result of archiving. Mirrors the server's `ArchiveOut`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveResult {
    pub save_id: String,
    pub archived: bool,
    /// RFC3339: when the frozen copy is purged (the instant plus 7 days).
    pub purge_after: String,
    pub freed_bytes: i64,
}

/// `GET {base}/v1/cloud/storage/games`.
pub async fn storage_games(base: &str, token: &str) -> Result<StorageGames, CloudError> {
    let url = format!("{base}/v1/cloud/storage/games");
    let resp = http_client()?
        .get(&url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| CloudError::Network(format!("Network error: {e}")))?;
    if !resp.status().is_success() {
        return Err(into_error(resp).await);
    }
    let body = resp.text().await.unwrap_or_default();
    parse_json(&body)
}

/// `POST {base}/v1/cloud/saves/:id/archive`: parks a save in the black box,
/// freeing quota now, leaving it downloadable for 7 days, then a cron purges it.
pub async fn archive_save(
    base: &str,
    token: &str,
    save_id: &str,
) -> Result<ArchiveResult, CloudError> {
    let url = format!("{base}/v1/cloud/saves/{save_id}/archive");
    let resp = http_client()?
        .post(&url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| CloudError::Network(format!("Network error: {e}")))?;
    if !resp.status().is_success() {
        return Err(into_error(resp).await);
    }
    let body = resp.text().await.unwrap_or_default();
    parse_json(&body)
}

/// `POST {base}/v1/cloud/saves/:id/reactivate`: recovers an archived save.
pub async fn reactivate_save(base: &str, token: &str, save_id: &str) -> Result<(), CloudError> {
    let url = format!("{base}/v1/cloud/saves/{save_id}/reactivate");
    let resp = http_client()?
        .post(&url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| CloudError::Network(format!("Network error: {e}")))?;
    if !resp.status().is_success() {
        return Err(into_error(resp).await);
    }
    Ok(())
}

/// `POST {base}/v1/notifications/:id/dismiss`: records that a broadcast was
/// dismissed so the server never delivers it to that user again, on any device or
/// after a reinstall. Idempotent server-side; a single attempt, with the
/// retry-after-401 left to the caller, as in `entitlements`.
pub async fn dismiss_notification(base: &str, token: &str, id: &str) -> Result<(), CloudError> {
    let url = format!("{base}/v1/notifications/{id}/dismiss");
    let resp = http_client()?
        .post(&url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| CloudError::Network(format!("Network error: {e}")))?;
    if !resp.status().is_success() {
        return Err(into_error(resp).await);
    }
    Ok(())
}

// ---- términos ---------------------------------------------------------

/// What the server knows about this account's acceptance.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TermsStatus {
    pub accepted_version: Option<String>,
    pub accepted_at: Option<String>,
    pub current_version: String,
    /// `true` when the checkbox has to be shown again.
    pub needs_acceptance: bool,
}

/// `POST {base}/v1/me/terms`: records the acceptance.
///
/// The caller does not choose the version: it is the one this binary has compiled
/// in ([`hoard_core::wire::TERMS_VERSION`]), which is the only one the user can
/// have seen from here. The server rejects any other, so an older client gets a
/// 400 rather than leaving an acceptance on record for a text that no longer
/// exists.
pub async fn accept_terms(
    base: &str,
    token: &str,
    source: &str,
    app_version: Option<&str>,
) -> Result<TermsStatus, CloudError> {
    let url = format!("{base}/v1/me/terms");
    let resp = http_client()?
        .post(&url)
        .bearer_auth(token)
        .json(&serde_json::json!({
            "version": hoard_core::wire::TERMS_VERSION,
            "source": source,
            "app_version": app_version,
        }))
        .send()
        .await
        .map_err(|e| CloudError::Network(format!("Network error: {e}")))?;
    if !resp.status().is_success() {
        return Err(into_error(resp).await);
    }
    let body = resp.text().await.unwrap_or_default();
    parse_json(&body)
}

/// `GET {base}/v1/me/terms`: what this account accepted and whether to ask again.
pub async fn terms_status(base: &str, token: &str) -> Result<TermsStatus, CloudError> {
    let url = format!("{base}/v1/me/terms");
    let resp = http_client()?
        .get(&url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| CloudError::Network(format!("Network error: {e}")))?;
    if !resp.status().is_success() {
        return Err(into_error(resp).await);
    }
    let body = resp.text().await.unwrap_or_default();
    parse_json(&body)
}

// ---- cuenta -----------------------------------------------------------

/// `DELETE {base}/v1/me`: soft-deletes and freezes the account (30 days' grace).
/// Clearing the local session is each frontend's glue.
pub async fn delete_account(base: &str, token: &str) -> Result<(), CloudError> {
    let url = format!("{base}/v1/me");
    let resp = http_client()?
        .delete(&url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| CloudError::Network(format!("Network error: {e}")))?;
    if !resp.status().is_success() {
        return Err(into_error(resp).await);
    }
    Ok(())
}

/// `POST {base}/v1/me/reactivate`: cancels a pending soft-delete. The frontend
/// re-reads `/v1/me` afterwards to refresh its account snapshot.
pub async fn reactivate_account(base: &str, token: &str) -> Result<(), CloudError> {
    let url = format!("{base}/v1/me/reactivate");
    let resp = http_client()?
        .post(&url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| CloudError::Network(format!("Network error: {e}")))?;
    if !resp.status().is_success() {
        return Err(into_error(resp).await);
    }
    Ok(())
}

// ---- entitlements / features ------------------------------------------

/// Per-feature Pro access, mirroring `GET /v1/cloud/entitlements`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudEntitlements {
    pub plan: String,
    pub features: CloudFeatures,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudFeatures {
    pub screen: FeatureState,
    pub wrapple: FeatureState,
}

/// A feature's access state. `tag = "state"` to match the server's enum
/// (`entitled`, `trial_available`, `trial`, `trial_expired`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum FeatureState {
    Entitled,
    TrialAvailable { days: i64 },
    Trial { expires_at: String },
    TrialExpired,
}

/// `GET {base}/v1/cloud/entitlements`: a read-only snapshot that starts no trial.
/// A single attempt; the retry-after-401 is the caller's.
pub async fn entitlements(base: &str, token: &str) -> Result<CloudEntitlements, CloudError> {
    let url = format!("{base}/v1/cloud/entitlements");
    let resp = http_client()?
        .get(&url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| CloudError::Network(format!("Network error: {e}")))?;
    if !resp.status().is_success() {
        return Err(into_error(resp).await);
    }
    let body = resp.text().await.unwrap_or_default();
    parse_json(&body)
}

/// `POST {base}/v1/cloud/features/:feature/activate`: opens a Pro feature,
/// starting the one-month trial on first use (the server is idempotent). A `402`
/// (locked: no Pro, trial spent) is translated to `TrialExpired` so the UI keeps
/// the padlock rather than showing an error.
pub async fn activate_feature(
    base: &str,
    token: &str,
    feature: &str,
) -> Result<FeatureState, CloudError> {
    let url = format!("{base}/v1/cloud/features/{feature}/activate");
    let resp = http_client()?
        .post(&url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| CloudError::Network(format!("Network error: {e}")))?;
    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err(CloudError::Unauthorized);
    }
    if status == reqwest::StatusCode::PAYMENT_REQUIRED {
        return Ok(FeatureState::TrialExpired);
    }
    if !status.is_success() {
        return Err(into_error(resp).await);
    }
    let body = resp.text().await.unwrap_or_default();
    parse_json(&body)
}

// ---- playtime ---------------------------------------------------------

/// The upload body: this machine's `(day, game, secs)` breakdown plus its
/// fingerprint, so the server can keep the rows apart per machine.
#[derive(Debug, Serialize)]
pub struct PlaytimeUploadBody {
    pub device_fp: String,
    /// This machine claims to know all of its own past, because its store came
    /// off a file that existed
    /// ([`crate::playtime::PlaytimeStore::is_authoritative`]).
    ///
    /// With `false` the server only touches the days carried in `rows`: a client
    /// that lost its file claims nothing about the older days, so it cannot delete
    /// them. With `true` it replaces the whole device, which is what it takes for
    /// dropping a game from the count to really drop it.
    pub authoritative: bool,
    pub rows: Vec<PlaytimeRow>,
}

/// `POST {base}{path}`: uploads this machine's playtime breakdown. `path` is
/// `/v1/cloud/playtime` (Cloud) or `/v1/playtime` (self-hosted).
pub async fn push_playtime(
    base: &str,
    path: &str,
    token: &str,
    body: &PlaytimeUploadBody,
) -> Result<(), CloudError> {
    let url = format!("{}{path}", base.trim_end_matches('/'));
    let resp = http_client()?
        .post(&url)
        .bearer_auth(token)
        .json(body)
        .send()
        .await
        .map_err(|e| CloudError::Network(format!("Network error: {e}")))?;
    if !resp.status().is_success() {
        return Err(into_error(resp).await);
    }
    Ok(())
}

/// `GET {base}{path}`: reads the playtime aggregate merged across devices.
pub async fn fetch_playtime(
    base: &str,
    path: &str,
    token: &str,
) -> Result<PlaytimeSummary, CloudError> {
    let url = format!("{}{path}", base.trim_end_matches('/'));
    let resp = http_client()?
        .get(&url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| CloudError::Network(format!("Network error: {e}")))?;
    if !resp.status().is_success() {
        return Err(into_error(resp).await);
    }
    let body = resp.text().await.unwrap_or_default();
    parse_json(&body)
}
