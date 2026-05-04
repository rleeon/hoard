//! Client-side mirror of the manifest the server returns from
//! `/v1/games/:slug/known-paths`.
//!
//! The server stores save paths as JSON in `games.save_paths_json`. The
//! desktop client fetches that, expands the placeholders against the local
//! filesystem (see `pathexpand`), and decides which paths exist. This module
//! provides the typed wire shape and a thin fetch helper.

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::api::ApiClient;

/// Three OSes Ludusavi tracks. Useful for callers that want to fan out a
/// search across all of them in tests, or pin the lookup to the current OS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Os {
    Windows,
    Linux,
    Mac,
}

impl Os {
    /// The OS we're running on right now. Linux falls into `Linux` even on
    /// odd Unix targets — we don't pretend to support BSD as a first-class
    /// citizen yet.
    pub fn current() -> Self {
        if cfg!(target_os = "windows") {
            Os::Windows
        } else if cfg!(target_os = "macos") {
            Os::Mac
        } else {
            Os::Linux
        }
    }
}

/// One save-path template plus its constraints, as the server returns it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavePath {
    pub path: String,
    #[serde(default)]
    pub constraints: Vec<PathConstraint>,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// `when` clause from Ludusavi: the path applies under these conditions.
/// `store: None` is the same as "any store".
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PathConstraint {
    #[serde(default)]
    pub store: Option<String>,
}

/// Per-OS bundle of save-path templates for one game.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PathsByOs {
    #[serde(default)]
    pub windows: Vec<SavePath>,
    #[serde(default)]
    pub linux: Vec<SavePath>,
    #[serde(default)]
    pub mac: Vec<SavePath>,
}

impl PathsByOs {
    /// Borrow the slot for the requested OS. The CLI has used this to validate
    /// that a manifest entry has *something* for the host before showing it.
    pub fn for_os(&self, os: Os) -> &[SavePath] {
        match os {
            Os::Windows => &self.windows,
            Os::Linux => &self.linux,
            Os::Mac => &self.mac,
        }
    }
}

/// Wire shape of `GET /v1/games/:slug/known-paths`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameManifest {
    pub slug: String,
    pub display_name: String,
    #[serde(default)]
    pub steam_app_id: Option<i64>,
    #[serde(default)]
    pub cloud_steam: bool,
    #[serde(default)]
    pub cloud_gog: bool,
    #[serde(default)]
    pub manifest_version: Option<String>,
    /// Tagged JSON shape: `{ "windows": [...], "linux": [...], "mac": [...] }`.
    /// Some hand-added games don't have any paths yet — `Default::default()`
    /// keeps callers from having to deal with `null`.
    #[serde(default)]
    pub paths: PathsByOs,
}

/// Wire shape of `GET /v1/manifest/version`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestVersion {
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub manifest_version: Option<String>,
    #[serde(default)]
    pub imported_at: Option<String>,
    #[serde(default)]
    pub games_inserted: i64,
    #[serde(default)]
    pub games_updated: i64,
    #[serde(default)]
    pub games_pruned: i64,
}

impl ApiClient {
    /// Fetch the per-OS save-path manifest for one game.
    pub async fn known_paths(&self, slug: &str) -> Result<GameManifest> {
        let resp = self
            .http_get(&format!("/v1/games/{slug}/known-paths"))
            .await?;
        Ok(resp.json().await?)
    }

    /// Fetch the most recent manifest import metadata.
    pub async fn manifest_version(&self) -> Result<Option<ManifestVersion>> {
        match self.http_get("/v1/manifest/version").await {
            Ok(resp) => Ok(Some(resp.json().await?)),
            Err(e) => {
                if let Some(api) = e.downcast_ref::<crate::api::ApiError>() {
                    if matches!(api, crate::api::ApiError::NotFound) {
                        return Ok(None);
                    }
                }
                Err(e)
            }
        }
    }
}
