use anyhow::{anyhow, Context, Result};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;
use time::OffsetDateTime;

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
    token: String,
    http: Client,
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
            token: token.into(),
            http,
        })
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.token)
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
        let body = serde_json::json!({ "game_slug": game_slug, "label": label });
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
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Health {
    pub status: String,
    pub version: String,
    pub uptime_secs: u64,
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
