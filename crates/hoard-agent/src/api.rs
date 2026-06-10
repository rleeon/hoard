use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use thiserror::Error;
use time::OffsetDateTime;
use tokio::sync::OnceCell;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("authentication failed: token rejected by server (401)")]
    Unauthorized,
    #[error("forbidden (403)")]
    Forbidden,
    #[error("not found (404)")]
    NotFound,
    #[error("payload too large (413): server max snapshot size exceeded")]
    TooLarge,
    #[error("conflict (409): {0}")]
    Conflict(String),
    #[error("bad request (400): {0}")]
    BadRequest(String),
    #[error("server error ({status}): {body}")]
    Server { status: u16, body: String },
}

impl ApiError {
    pub async fn from_response(resp: reqwest::Response) -> Self {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        match status {
            StatusCode::UNAUTHORIZED => ApiError::Unauthorized,
            StatusCode::FORBIDDEN => ApiError::Forbidden,
            StatusCode::NOT_FOUND => ApiError::NotFound,
            StatusCode::PAYLOAD_TOO_LARGE => ApiError::TooLarge,
            StatusCode::CONFLICT => ApiError::Conflict(extract_message(&body)),
            StatusCode::BAD_REQUEST => ApiError::BadRequest(extract_message(&body)),
            _ => ApiError::Server {
                status: status.as_u16(),
                body,
            },
        }
    }
}

fn extract_message(body: &str) -> String {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(body) {
        if let Some(s) = v.get("message").and_then(|x| x.as_str()) {
            return s.to_string();
        }
        if let Some(s) = v.get("error").and_then(|x| x.as_str()) {
            return s.to_string();
        }
    }
    body.to_string()
}

#[derive(Debug, Clone)]
pub struct ApiClient {
    base_url: String,
    /// Bearer token, shared+swappable across every clone of this client.
    ///
    /// Self-hosted bearer tokens are stable for the process lifetime, but a
    /// Hoard Cloud session uses a short-lived Supabase JWT (~1h). The desktop's
    /// long-lived agent holds a single `ApiClient` for the whole session, so a
    /// frozen token would start answering 401 the moment the JWT expired —
    /// which is exactly what made the auto-restore sweep spam "no se pudo
    /// restaurar" once an hour in. Storing the token behind a shared `RwLock`
    /// lets the desktop's token-refresh path push a fresh JWT into the running
    /// agent's client via [`ApiClient::set_token`] without rebuilding it.
    token: Arc<RwLock<String>>,
    http: Client,
    /// Lazily-probed `/v1/health` `mode` (`Some("cloud")` on the SaaS
    /// deployment, `None`/absent self-hosted). Cached behind an `Arc` so the
    /// many `ApiClient` clones in flight share a single probe. Only cached on
    /// a successful probe — a transient health failure leaves the cell empty
    /// so the next call retries instead of wedging the client into the wrong
    /// protocol forever.
    mode: Arc<OnceCell<Option<String>>>,
}

impl ApiClient {
    pub fn new(base_url: impl Into<String>, token: impl Into<String>) -> Result<Self> {
        let http = Client::builder()
            .user_agent(concat!("hoard-agent/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(60))
            // Long-lived stream uploads/downloads handle their own timeouts via streaming
            .build()?;
        Ok(Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            token: Arc::new(RwLock::new(token.into())),
            http,
            mode: Arc::new(OnceCell::new()),
        })
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    /// Swap in a fresh bearer token. Every clone of this client shares the same
    /// token cell, so updating one updates them all — the mechanism the desktop
    /// uses to keep the long-lived agent client's Supabase JWT current after a
    /// refresh.
    pub fn set_token(&self, token: impl Into<String>) {
        if let Ok(mut guard) = self.token.write() {
            *guard = token.into();
        }
    }

    fn auth_header(&self) -> String {
        let token = self.token.read().map(|t| t.clone()).unwrap_or_default();
        format!("Bearer {token}")
    }

    async fn ok_or_err(resp: reqwest::Response) -> Result<reqwest::Response, ApiError> {
        if resp.status().is_success() {
            Ok(resp)
        } else {
            Err(ApiError::from_response(resp).await)
        }
    }

    /// Issue an authenticated GET to `path` (e.g. `/v1/manifest/version`) and
    /// return the response object on success. Other modules use this when
    /// their only interaction with the API is "GET this URL, decode JSON".
    pub async fn http_get(&self, path: &str) -> Result<reqwest::Response> {
        let resp = self
            .http
            .get(self.url(path))
            .header("authorization", self.auth_header())
            .send()
            .await?;
        Ok(Self::ok_or_err(resp).await?)
    }

    pub async fn whoami(&self) -> Result<Whoami> {
        let resp = self
            .http
            .get(self.url("/v1/auth/whoami"))
            .header("authorization", self.auth_header())
            .send()
            .await?;
        let resp = Self::ok_or_err(resp).await?;
        Ok(resp.json().await?)
    }

    pub async fn health(&self) -> Result<Health> {
        let resp = self.http.get(self.url("/v1/health")).send().await?;
        let resp = Self::ok_or_err(resp).await?;
        Ok(resp.json().await?)
    }

    /// Resolve (and cache) the server's deployment mode from `/v1/health`.
    /// `Some("cloud")` selects the Hoard Cloud protocol; `None` means
    /// self-hosted. A failed probe returns `None` *without* caching so the
    /// next call retries.
    pub async fn server_mode(&self) -> Option<String> {
        self.mode
            .get_or_try_init(|| async { self.health().await.map(|h| h.mode) })
            .await
            .ok()
            .cloned()
            .flatten()
    }

    /// True when the server is the SaaS (`api.hoard.services`) deployment,
    /// which speaks the `/v1/cloud/*` protocol instead of the self-hosted
    /// `/v1/saves` + multipart one.
    pub async fn is_cloud(&self) -> bool {
        self.server_mode().await.as_deref() == Some("cloud")
    }

    // ---- Cloud (SaaS) protocol -----------------------------------------

    /// `POST /v1/cloud/saves` — declare upload intent. The server validates
    /// plan + quota, mints a presigned R2 PUT URL, and returns the version
    /// number the client must `commit` against.
    pub async fn cloud_init_upload(&self, init: &CloudUploadInit) -> Result<CloudUploadInitOut> {
        let resp = self
            .http
            .post(self.url("/v1/cloud/saves"))
            .header("authorization", self.auth_header())
            .json(init)
            .send()
            .await?;
        let resp = Self::ok_or_err(resp).await.map_err(|e| anyhow!(e))?;
        Ok(resp.json().await?)
    }

    /// Upload bytes directly to a presigned R2 URL. No `Authorization`
    /// header — the presigned URL carries its own signature in the query
    /// string, and an extra auth header breaks the S3 v4 signature.
    pub async fn put_presigned(
        &self,
        presigned: &PresignedUrl,
        body: reqwest::Body,
        content_length: u64,
    ) -> Result<()> {
        let method = reqwest::Method::from_bytes(presigned.method.as_bytes())
            .unwrap_or(reqwest::Method::PUT);
        let resp = self
            .http
            .request(method, &presigned.url)
            .header(reqwest::header::CONTENT_LENGTH, content_length)
            .body(body)
            .send()
            .await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            bail!("storage upload failed ({status}): {text}");
        }
        Ok(())
    }

    /// `POST /v1/cloud/saves/:id/versions/:n/commit` — finalize an upload.
    /// The server verifies the object via R2 HEAD and records the sha256.
    pub async fn cloud_commit(
        &self,
        save_id: &str,
        version: i64,
        commit: &CloudUploadCommit,
    ) -> Result<CloudUploadCommitOut> {
        let resp = self
            .http
            .post(self.url(&format!(
                "/v1/cloud/saves/{save_id}/versions/{version}/commit"
            )))
            .header("authorization", self.auth_header())
            .json(commit)
            .send()
            .await?;
        let resp = Self::ok_or_err(resp).await.map_err(|e| anyhow!(e))?;
        Ok(resp.json().await?)
    }

    /// `GET /v1/cloud/saves/:id/versions/:n/download` — mint a presigned R2
    /// GET URL plus the version's sha256/size for verification.
    pub async fn cloud_download(&self, save_id: &str, version: i64) -> Result<CloudDownloadOut> {
        let resp = self
            .http
            .get(self.url(&format!(
                "/v1/cloud/saves/{save_id}/versions/{version}/download"
            )))
            .header("authorization", self.auth_header())
            .send()
            .await?;
        let resp = Self::ok_or_err(resp).await.map_err(|e| anyhow!(e))?;
        Ok(resp.json().await?)
    }

    /// `GET /v1/cloud/sync` — the manifest of the user's saves (latest
    /// version of each). The cloud analogue of `list_saves`; excludes
    /// `backup_only` saves.
    pub async fn cloud_sync(&self) -> Result<CloudManifest> {
        let resp = self
            .http
            .get(self.url("/v1/cloud/sync"))
            .header("authorization", self.auth_header())
            .send()
            .await?;
        let resp = Self::ok_or_err(resp).await.map_err(|e| anyhow!(e))?;
        Ok(resp.json().await?)
    }

    /// `GET /v1/cloud/saves/:save_id/versions` — the full version history of a
    /// cloud save (every committed version, newest first). The cloud analogue
    /// of `list_save_snapshots`; the sync manifest only carries the latest.
    pub async fn cloud_list_versions(
        &self,
        save_id: &str,
        include_deleted: bool,
    ) -> Result<Vec<Snapshot>> {
        let resp = self
            .http
            .get(self.url(&format!("/v1/cloud/saves/{save_id}/versions")))
            .query(&[("include_deleted", include_deleted)])
            .header("authorization", self.auth_header())
            .send()
            .await?;
        let resp = Self::ok_or_err(resp).await.map_err(|e| anyhow!(e))?;
        Ok(resp.json().await?)
    }

    /// `DELETE /v1/cloud/saves/:save_id/versions/:version` — drop a single
    /// version (blob + row) and repoint `latest_version_num` to the highest
    /// remaining version. Deletes the whole save only if none remain.
    pub async fn cloud_delete_version(&self, save_id: &str, version: i64) -> Result<()> {
        let resp = self
            .http
            .delete(self.url(&format!("/v1/cloud/saves/{save_id}/versions/{version}")))
            .header("authorization", self.auth_header())
            .send()
            .await?;
        Self::ok_or_err(resp).await.map_err(|e| anyhow!(e))?;
        Ok(())
    }

    /// `DELETE /v1/cloud/saves/:save_id` — remove a cloud save and all of its
    /// versions so the user reclaims storage. The cloud analogue of deleting
    /// a whole tracked save.
    pub async fn cloud_save_delete(&self, save_id: &str) -> Result<()> {
        let resp = self
            .http
            .delete(self.url(&format!("/v1/cloud/saves/{save_id}")))
            .header("authorization", self.auth_header())
            .send()
            .await?;
        Self::ok_or_err(resp).await.map_err(|e| anyhow!(e))?;
        Ok(())
    }

    /// GET the bytes behind a presigned download URL as a streaming response.
    /// No auth header, same rationale as [`put_presigned`].
    pub async fn get_presigned(&self, presigned: &PresignedUrl) -> Result<reqwest::Response> {
        let method = reqwest::Method::from_bytes(presigned.method.as_bytes())
            .unwrap_or(reqwest::Method::GET);
        let resp = self.http.request(method, &presigned.url).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            bail!("storage download failed ({status}): {text}");
        }
        Ok(resp)
    }

    /// `POST /v1/cloud/cas/init` — declare a content-addressed upload. Returns
    /// the new version number plus the subset of blobs the server is missing,
    /// each with a presigned PUT URL.
    pub async fn cloud_cas_init(&self, init: &CloudCasInit) -> Result<CloudCasInitOut> {
        let resp = self
            .http
            .post(self.url("/v1/cloud/cas/init"))
            .header("authorization", self.auth_header())
            .json(init)
            .send()
            .await?;
        let resp = Self::ok_or_err(resp).await.map_err(|e| anyhow!(e))?;
        Ok(resp.json().await?)
    }

    /// `POST /v1/cloud/saves/:id/versions/:n/cas/commit` — finalize a content-
    /// addressed upload once every missing blob has been PUT.
    pub async fn cloud_cas_commit(
        &self,
        save_id: &str,
        version: i64,
    ) -> Result<CloudUploadCommitOut> {
        let resp = self
            .http
            .post(self.url(&format!(
                "/v1/cloud/saves/{save_id}/versions/{version}/cas/commit"
            )))
            .header("authorization", self.auth_header())
            .send()
            .await?;
        let resp = Self::ok_or_err(resp).await.map_err(|e| anyhow!(e))?;
        Ok(resp.json().await?)
    }

    /// `GET /v1/cloud/saves/:id/versions/:n/manifest` — the per-file manifest
    /// of a content-addressed version. With `presign = true` each file carries
    /// a download URL (restore) and bandwidth is charged; with `false` it's a
    /// cheap listing (History detail). Returns `content_addressed = false` for
    /// legacy archive versions.
    pub async fn cloud_version_manifest(
        &self,
        save_id: &str,
        version: i64,
        presign: bool,
    ) -> Result<CloudVersionManifestOut> {
        let resp = self
            .http
            .get(self.url(&format!(
                "/v1/cloud/saves/{save_id}/versions/{version}/manifest"
            )))
            .query(&[("presign", presign)])
            .header("authorization", self.auth_header())
            .send()
            .await?;
        let resp = Self::ok_or_err(resp).await.map_err(|e| anyhow!(e))?;
        Ok(resp.json().await?)
    }

    pub async fn list_games(&self, query: Option<&str>) -> Result<Vec<Game>> {
        let mut req = self
            .http
            .get(self.url("/v1/games"))
            .header("authorization", self.auth_header());
        if let Some(q) = query {
            req = req.query(&[("search", q)]);
        }
        let resp = Self::ok_or_err(req.send().await?).await?;
        Ok(resp.json().await?)
    }

    /// Paginated catalog fetch. Used by detection to walk the full ~11k-entry
    /// games table without blowing the per-request size budget. The server
    /// caps `limit` at 1000.
    pub async fn list_games_paged(&self, limit: u32, offset: u32) -> Result<Vec<Game>> {
        let resp = self
            .http
            .get(self.url("/v1/games"))
            .header("authorization", self.auth_header())
            .query(&[("limit", limit.to_string()), ("offset", offset.to_string())])
            .send()
            .await?;
        let resp = Self::ok_or_err(resp).await?;
        Ok(resp.json().await?)
    }

    pub async fn get_game(&self, slug: &str) -> Result<Game> {
        let resp = self
            .http
            .get(self.url(&format!("/v1/games/{}", slug)))
            .header("authorization", self.auth_header())
            .send()
            .await?;
        let resp = Self::ok_or_err(resp).await?;
        Ok(resp.json().await?)
    }

    pub async fn list_saves(&self, game: Option<&str>) -> Result<Vec<Save>> {
        let mut req = self
            .http
            .get(self.url("/v1/saves"))
            .header("authorization", self.auth_header());
        if let Some(g) = game {
            req = req.query(&[("game_slug", g)]);
        }
        let resp = Self::ok_or_err(req.send().await?).await?;
        Ok(resp.json().await?)
    }

    pub async fn create_save(&self, game_slug: &str, label: &str) -> Result<Save> {
        self.create_save_with_meta(game_slug, label, None, None)
            .await
    }

    /// Create a Save and, optionally, hint to the server what the game's
    /// display name / Steam ID are. Used by the desktop client so that
    /// servers running an older Ludusavi catalog can self-heal a missing
    /// games row from the metadata the desktop already has at hand. Older
    /// servers ignore the extra fields (Serde tolerates unknown keys), so
    /// this is forward-compatible.
    pub async fn create_save_with_meta(
        &self,
        game_slug: &str,
        label: &str,
        display_name: Option<&str>,
        steam_app_id: Option<i64>,
    ) -> Result<Save> {
        let mut body = serde_json::json!({ "game_slug": game_slug, "label": label });
        if let Some(name) = display_name {
            body["display_name"] = serde_json::Value::String(name.to_string());
        }
        if let Some(id) = steam_app_id {
            body["steam_app_id"] = serde_json::Value::Number(id.into());
        }
        let resp = self
            .http
            .post(self.url("/v1/saves"))
            .header("authorization", self.auth_header())
            .json(&body)
            .send()
            .await?;
        let resp = Self::ok_or_err(resp).await?;
        Ok(resp.json().await?)
    }

    pub async fn get_save(&self, save_id: &str) -> Result<Save> {
        let resp = self
            .http
            .get(self.url(&format!("/v1/saves/{}", save_id)))
            .header("authorization", self.auth_header())
            .send()
            .await?;
        let resp = Self::ok_or_err(resp).await?;
        Ok(resp.json().await?)
    }

    pub async fn delete_save(&self, save_id: &str) -> Result<()> {
        let resp = self
            .http
            .delete(self.url(&format!("/v1/saves/{}", save_id)))
            .header("authorization", self.auth_header())
            .send()
            .await?;
        Self::ok_or_err(resp).await?;
        Ok(())
    }

    /// Rename the label on an existing save. Surfaces 409 via
    /// [`ApiError::Conflict`] so the UI can show a "label already exists"
    /// message instead of a generic server error.
    pub async fn rename_save_label(&self, save_id: &str, new_label: &str) -> Result<Save> {
        let body = serde_json::json!({ "label": new_label });
        let resp = self
            .http
            .patch(self.url(&format!("/v1/saves/{}", save_id)))
            .header("authorization", self.auth_header())
            .json(&body)
            .send()
            .await?;
        let resp = Self::ok_or_err(resp).await?;
        Ok(resp.json().await?)
    }

    pub async fn list_snapshots(
        &self,
        save_id: &str,
        include_deleted: bool,
    ) -> Result<Vec<Snapshot>> {
        let mut req = self
            .http
            .get(self.url(&format!("/v1/saves/{}/snapshots", save_id)))
            .header("authorization", self.auth_header());
        if include_deleted {
            req = req.query(&[("include_deleted", "true")]);
        }
        let resp = Self::ok_or_err(req.send().await?).await?;
        Ok(resp.json().await?)
    }

    pub async fn snapshot_detail(&self, save_id: &str, version: i64) -> Result<SnapshotDetail> {
        let resp = self
            .http
            .get(self.url(&format!("/v1/saves/{}/snapshots/{}", save_id, version)))
            .header("authorization", self.auth_header())
            .send()
            .await?;
        let resp = Self::ok_or_err(resp).await?;
        Ok(resp.json().await?)
    }

    pub async fn snapshot_download(
        &self,
        save_id: &str,
        version: i64,
    ) -> Result<reqwest::Response> {
        let resp = self
            .http
            .get(self.url(&format!(
                "/v1/saves/{}/snapshots/{}/download",
                save_id, version
            )))
            .header("authorization", self.auth_header())
            .send()
            .await?;
        Self::ok_or_err(resp)
            .await
            .map_err(|e| anyhow!(e))
            .context("download request failed")
    }

    pub async fn snapshot_delete(&self, save_id: &str, version: i64) -> Result<()> {
        let resp = self
            .http
            .delete(self.url(&format!("/v1/saves/{}/snapshots/{}", save_id, version)))
            .header("authorization", self.auth_header())
            .send()
            .await?;
        Self::ok_or_err(resp).await?;
        Ok(())
    }

    pub async fn snapshot_restore(&self, save_id: &str, version: i64) -> Result<()> {
        let resp = self
            .http
            .post(self.url(&format!(
                "/v1/saves/{}/snapshots/{}/restore",
                save_id, version
            )))
            .header("authorization", self.auth_header())
            .send()
            .await?;
        Self::ok_or_err(resp).await?;
        Ok(())
    }

    /// Upload a multipart snapshot. Each part is a file: name=`relative/path`, body bytes.
    /// Returns the created snapshot summary.
    pub async fn snapshot_upload(
        &self,
        save_id: &str,
        form: reqwest::multipart::Form,
    ) -> Result<Snapshot> {
        let resp = self
            .http
            .post(self.url(&format!("/v1/saves/{}/snapshots", save_id)))
            .header("authorization", self.auth_header())
            .multipart(form)
            .send()
            .await?;
        let resp = Self::ok_or_err(resp).await.map_err(|e| anyhow!(e))?;
        Ok(resp.json().await?)
    }
}

// ---- DTOs ---------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Whoami {
    pub user_id: String,
    pub username: String,
    pub is_admin: bool,
    /// Bytes the user has stored on the server right now. Server-side
    /// counter, kept in sync by the snapshot upload/delete pipeline.
    /// Older servers (pre v0.3) omit this; default to 0 so onboarding
    /// against a stale instance still works.
    #[serde(default)]
    pub storage_used_bytes: i64,
    /// Total bytes the user is allowed to store. Default 100 GiB on a
    /// fresh server. Pre-v0.3 servers omit this; default to 0 (the UI
    /// reads this as "quota unknown" and falls back to MB display).
    #[serde(default)]
    pub storage_quota_bytes: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Health {
    pub status: String,
    pub version: String,
    #[serde(default)]
    pub uptime_secs: u64,
    /// Minimum log level the server accepts for client-log ingest. Absent on
    /// pre-log-ingest servers — the client takes `None` to mean "this server
    /// can't receive logs" and disables shipping.
    #[serde(default)]
    pub log_min_level: Option<String>,
    /// `"cloud"` on the SaaS deployment, absent/`None` self-hosted. Selects
    /// the ingest endpoint (`/v1/cloud/logs` vs `/v1/logs`).
    #[serde(default)]
    pub mode: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Game {
    pub slug: String,
    pub display_name: String,
    pub engine: Option<String>,
    #[serde(default)]
    pub save_paths_json: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Save {
    pub id: String,
    #[serde(default)]
    pub user_id: Option<String>,
    pub game_slug: String,
    pub label: String,
    pub latest_version_num: Option<i64>,
    #[serde(default)]
    pub snapshot_count: Option<i64>,
    #[serde(default)]
    pub total_size_bytes: Option<i64>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Snapshot {
    pub id: String,
    #[serde(default)]
    pub save_id: Option<String>,
    pub version_num: i64,
    /// DAG parent (`None` = root). Older servers omit it → defaults to None.
    #[serde(default)]
    pub parent_version: Option<i64>,
    pub file_count: i64,
    pub total_size_bytes: i64,
    pub is_pinned: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option", default)]
    pub deleted_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SnapshotDetail {
    #[serde(flatten)]
    pub snapshot: Snapshot,
    pub files: Vec<SnapshotFile>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SnapshotFile {
    pub relative_path: String,
    pub size_bytes: i64,
    pub sha256: String,
}

// ---- Cloud (SaaS) protocol DTOs ----------------------------------------

/// Body for `POST /v1/cloud/saves`. Mirrors `hoard-server`'s `UploadInit`.
#[derive(Debug, Clone, Serialize)]
pub struct CloudUploadInit {
    pub save_id: String,
    pub game_slug: String,
    pub label: Option<String>,
    pub size_bytes: u64,
    /// Files inside the packed tar.zst. The server stores it verbatim so the
    /// History view can show "N archivos" (the blob is opaque server-side).
    pub file_count: i64,
    pub device_name: Option<String>,
    pub notes: Option<String>,
    pub backup_only: bool,
    /// Last-synced version for this save. Drives the server's fast-forward
    /// check: a mismatch means another device pushed since, → 409.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_version: Option<i64>,
}

/// A short-lived presigned R2 URL (PUT for upload, GET for download).
#[derive(Debug, Clone, Deserialize)]
pub struct PresignedUrl {
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub expires_in_secs: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CloudQuotaInfo {
    pub plan: String,
    pub used_bytes: u64,
    pub limit_bytes: u64,
    #[serde(default)]
    pub devices_used: u32,
    #[serde(default)]
    pub devices_limit: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CloudUploadInitOut {
    pub version_num: i64,
    pub r2_key: String,
    pub upload: PresignedUrl,
    pub quota: CloudQuotaInfo,
}

#[derive(Debug, Clone, Serialize)]
pub struct CloudUploadCommit {
    pub sha256: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CloudUploadCommitOut {
    pub save_id: String,
    pub version_num: i64,
    pub committed: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CloudDownloadOut {
    pub save_id: String,
    pub version_num: i64,
    pub sha256: String,
    pub size_bytes: i64,
    pub download: PresignedUrl,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CloudManifestEntry {
    pub save_id: String,
    pub game_slug: String,
    pub label: String,
    pub latest_version_num: i64,
    #[serde(default)]
    pub latest_parent_version: Option<i64>,
    #[serde(default)]
    pub latest_size_bytes: i64,
    /// Files in the latest version (0 = unknown / pre-file-count server).
    #[serde(default)]
    pub latest_file_count: i64,
    #[serde(default)]
    pub latest_sha256: String,
    #[serde(default)]
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CloudManifest {
    #[serde(default)]
    pub generated_at: String,
    pub saves: Vec<CloudManifestEntry>,
}

// ---- Cloud content-addressed (per-file dedup) DTOs ---------------------

/// One file in a content-addressed upload manifest. Mirrors the server's
/// `CasFileEntry`.
#[derive(Debug, Clone, Serialize)]
pub struct CloudCasFileEntry {
    pub relative_path: String,
    pub sha256: String,
    pub size_bytes: i64,
    /// Source file mtime (unix seconds), preserved on restore. `None` if the
    /// FS didn't report one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<i64>,
}

/// Body for `POST /v1/cloud/cas/init`. The client declares the whole-file
/// manifest; the server replies with the subset of blobs it doesn't have.
#[derive(Debug, Clone, Serialize)]
pub struct CloudCasInit {
    pub save_id: String,
    pub game_slug: String,
    pub label: Option<String>,
    pub device_name: Option<String>,
    pub notes: Option<String>,
    pub backup_only: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_version: Option<i64>,
    pub files: Vec<CloudCasFileEntry>,
}

/// A blob the server is missing — the client must PUT it to `upload`.
#[derive(Debug, Clone, Deserialize)]
pub struct CloudCasMissingBlob {
    pub sha256: String,
    pub size_bytes: i64,
    #[serde(default)]
    pub r2_key: String,
    pub upload: PresignedUrl,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CloudCasInitOut {
    /// Canonical cloud save id (servers ≥ 2.3.2). Differs from the requested
    /// id when (user, game_slug, label) already maps to another cloud save —
    /// the commit must target this id or it 404s. `None` on older servers.
    #[serde(default)]
    pub save_id: Option<String>,
    pub version_num: i64,
    pub missing: Vec<CloudCasMissingBlob>,
    #[allow(dead_code)]
    pub quota: CloudQuotaInfo,
}

/// One file in a version manifest. `download` is present only when the
/// manifest was requested with `presign=true` (the restore path).
#[derive(Debug, Clone, Deserialize)]
pub struct CloudManifestFile {
    pub relative_path: String,
    pub sha256: String,
    pub size_bytes: i64,
    #[serde(default)]
    pub modified_at: Option<i64>,
    #[serde(default)]
    pub download: Option<PresignedUrl>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CloudVersionManifestOut {
    /// False for legacy archive versions — the caller must fall back to the
    /// whole-archive `cloud_download` path.
    #[serde(default)]
    pub content_addressed: bool,
    #[serde(default)]
    pub files: Vec<CloudManifestFile>,
}
