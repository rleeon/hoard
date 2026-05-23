//! Supabase JWT validation.
//!
//! Supabase Auth issues RS256-signed JWTs. The public keys live at the
//! project's JWKS endpoint
//! (`https://<project>.supabase.co/auth/v1/.well-known/jwks.json`). We
//! fetch them at boot and refresh hourly. The cache survives transient
//! network failures: if a refresh fails, we keep serving the previously
//! loaded keys until the next interval.

use crate::cloud::errors::CloudError;
use crate::cloud::state::CloudState;
use anyhow::{anyhow, Context, Result};
use axum::{
    extract::{Request, State},
    http::header,
    middleware::Next,
    response::{IntoResponse, Response},
};
use jsonwebtoken::{decode, decode_header, jwk::JwkSet, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use std::{sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tracing::{debug, warn};
use uuid::Uuid;

/// Identity extracted from a verified JWT and inserted into request extensions.
#[derive(Clone, Debug)]
pub struct CloudUser {
    pub user_id: Uuid,
    pub email: String,
    pub role: String,
}

/// In-memory JWKS cache. Concurrent reads via `RwLock` (overwhelmingly the
/// hot path — every authenticated request hits this).
pub struct JwksCache {
    inner: RwLock<JwkSet>,
    audience: String,
    issuer: Option<String>,
    jwks_url: String,
    http: reqwest::Client,
}

impl JwksCache {
    /// Build a cache, fetching once up-front. Returns an Arc so it can be
    /// shared by both the middleware and the refresh task.
    pub async fn new(
        jwks_url: String,
        audience: String,
        issuer: Option<String>,
    ) -> Result<Arc<Self>> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;
        let initial = fetch_jwks(&http, &jwks_url).await?;
        Ok(Arc::new(Self {
            inner: RwLock::new(initial),
            audience,
            issuer,
            jwks_url,
            http,
        }))
    }

    /// Spawn the background refresh task. The cache lifetime equals the
    /// server lifetime; if the refresh fails, we log and keep the previous
    /// JWKS rather than panicking — a stale-but-valid keyset is far better
    /// than dropping every authenticated request.
    pub fn spawn_refresh(self: Arc<Self>, interval: Duration) {
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(interval);
            tick.tick().await; // skip the immediate first tick
            loop {
                tick.tick().await;
                match fetch_jwks(&self.http, &self.jwks_url).await {
                    Ok(fresh) => {
                        *self.inner.write().await = fresh;
                        debug!("jwks refreshed");
                    }
                    Err(e) => warn!(error = %e, "jwks refresh failed; keeping previous"),
                }
            }
        });
    }

    /// Validate a JWT against the cached keys. Returns the verified claims.
    pub async fn verify(&self, token: &str) -> Result<Claims> {
        let header = decode_header(token).context("invalid jwt header")?;
        let kid = header
            .kid
            .ok_or_else(|| anyhow!("jwt missing 'kid' header"))?;

        let key = {
            let guard = self.inner.read().await;
            let jwk = guard
                .find(&kid)
                .ok_or_else(|| anyhow!("kid '{}' not in jwks", kid))?;
            DecodingKey::from_jwk(jwk).context("could not build decoding key from jwk")?
        };

        let alg = header.alg;
        let allowed: &[Algorithm] = match alg {
            Algorithm::RS256
            | Algorithm::RS384
            | Algorithm::RS512
            | Algorithm::ES256
            | Algorithm::ES384 => match alg {
                Algorithm::RS256 => &[Algorithm::RS256],
                Algorithm::RS384 => &[Algorithm::RS384],
                Algorithm::RS512 => &[Algorithm::RS512],
                Algorithm::ES256 => &[Algorithm::ES256],
                Algorithm::ES384 => &[Algorithm::ES384],
                _ => unreachable!(),
            },
            _ => return Err(anyhow!("unsupported jwt alg: {:?}", alg)),
        };
        let mut validation = Validation::new(allowed[0]);
        validation.algorithms = allowed.to_vec();
        validation.set_audience(&[&self.audience]);
        if let Some(iss) = &self.issuer {
            validation.set_issuer(&[iss]);
        }

        let data = decode::<Claims>(token, &key, &validation).context("jwt verification failed")?;
        Ok(data.claims)
    }
}

async fn fetch_jwks(http: &reqwest::Client, url: &str) -> Result<JwkSet> {
    let resp = http
        .get(url)
        .send()
        .await
        .with_context(|| format!("fetching jwks from {}", url))?
        .error_for_status()
        .context("jwks endpoint returned non-2xx")?;
    let set: JwkSet = resp.json().await.context("parsing jwks json")?;
    Ok(set)
}

/// JWT claims we care about. Supabase issues extra fields we ignore.
#[derive(Debug, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub email: Option<String>,
    #[serde(default)]
    pub role: String,
    pub exp: usize,
    pub aud: serde_json::Value,
    pub iss: Option<String>,
}

/// Axum middleware: extract Bearer JWT, validate, inject `CloudUser`.
pub async fn require_cloud_auth(
    State(state): State<CloudState>,
    mut req: Request,
    next: Next,
) -> Response {
    let token = match extract_bearer(&req) {
        Some(t) => t,
        None => return CloudError::Unauthorized("missing bearer token").into_response(),
    };
    let claims = match state.jwks.verify(&token).await {
        Ok(c) => c,
        Err(e) => {
            debug!(error = %e, "jwt verification failed");
            return CloudError::Unauthorized("invalid token").into_response();
        }
    };
    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(u) => u,
        Err(_) => {
            return CloudError::Unauthorized("sub is not a uuid").into_response();
        }
    };
    let email = claims.email.unwrap_or_default();
    req.extensions_mut().insert(CloudUser {
        user_id,
        email,
        role: claims.role,
    });
    next.run(req).await
}

fn extract_bearer(req: &Request) -> Option<String> {
    let val = req.headers().get(header::AUTHORIZATION)?.to_str().ok()?;
    val.strip_prefix("Bearer ").map(|s| s.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // We can't test against a real Supabase JWKS without network. Instead
    // we test the bits we own: header extraction and basic claim shape.

    #[test]
    fn extract_bearer_strips_prefix() {
        use axum::http::Request as HttpReq;
        let req = HttpReq::builder()
            .header("Authorization", "Bearer abc.def.ghi")
            .body(axum::body::Body::empty())
            .unwrap();
        assert_eq!(extract_bearer(&req), Some("abc.def.ghi".to_string()));
    }

    #[test]
    fn extract_bearer_missing_header() {
        use axum::http::Request as HttpReq;
        let req = HttpReq::builder()
            .body(axum::body::Body::empty())
            .unwrap();
        assert!(extract_bearer(&req).is_none());
    }

    #[test]
    fn extract_bearer_wrong_scheme() {
        use axum::http::Request as HttpReq;
        let req = HttpReq::builder()
            .header("Authorization", "Basic Zm9vOmJhcg==")
            .body(axum::body::Body::empty())
            .unwrap();
        assert!(extract_bearer(&req).is_none());
    }
}
