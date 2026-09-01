use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Json, Response},
};
use serde::Serialize;
use sqlx::SqlitePool;
use tracing::warn;
use uuid::Uuid;

/// Authenticated user identity, inserted into request extensions by the middleware.
#[derive(Clone, Debug)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub username: String,
    pub is_admin: bool,
}

#[derive(Serialize)]
struct ErrorBody {
    error: &'static str,
}

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(ErrorBody {
            error: "invalid or missing token",
        }),
    )
        .into_response()
}

/// Cookie the panel's browser session travels in.
///
/// Its value is a plain `api_tokens` row, so everything that already governs a
/// token governs a browser session too: expiry, `revoked_at`, and
/// `hoard-admin token list/revoke`. That is why the middleware below needs no
/// second code path: it looks the cookie up exactly like a Bearer header.
///
/// The cookie is minted `SameSite=Strict`, which is what stands in for a CSRF
/// token: a POST from another origin arrives without it and lands on the
/// `unauthorized()` branch. Anything mounted here that mutates state depends on
/// that flag, so don't relax it to `Lax` for a nicer cross-site link.
pub const SESSION_COOKIE: &str = "hoard_session";

/// Axum middleware: extract Bearer token, validate against DB, inject AuthUser.
pub async fn require_auth(
    State(pool): State<SqlitePool>,
    mut req: Request,
    next: Next,
) -> Response {
    let token = match extract_token(&req) {
        Some(t) => t,
        None => return unauthorized(),
    };

    let token_hash = hoard_core::hashing::hash_token(&token);

    let row = sqlx::query!(
        r#"
        SELECT u.id as user_id, u.username, u.is_admin,
               t.id as token_id, t.expires_at, t.revoked_at
        FROM api_tokens t
        JOIN users u ON u.id = t.user_id
        WHERE t.token_hash = ?
        "#,
        token_hash
    )
    .fetch_optional(&pool)
    .await;

    let row = match row {
        Ok(Some(r)) => r,
        Ok(None) => return unauthorized(),
        Err(e) => {
            warn!(error = %e, "DB error during auth");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    // Check revoked
    if row.revoked_at.is_some() {
        return unauthorized();
    }

    // Check expiry
    if let Some(exp) = &row.expires_at {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default();
        if exp.as_str() < now.as_str() {
            return unauthorized();
        }
    }

    // Update last_used_at asynchronously (best-effort, don't fail the request)
    let token_id = row.token_id.clone();
    let pool2 = pool.clone();
    tokio::spawn(async move {
        let _ = sqlx::query!(
            "UPDATE api_tokens SET last_used_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE id = ?",
            token_id
        )
        .execute(&pool2)
        .await;
    });

    let user_id = Uuid::parse_str(&row.user_id).unwrap_or_else(|_| Uuid::nil());
    req.extensions_mut().insert(AuthUser {
        user_id,
        username: row.username,
        is_admin: row.is_admin != 0,
    });

    next.run(req).await
}

/// A request authenticates with a Bearer header (every client) or with the
/// panel's session cookie (browsers only). Header first: a client that sends
/// both is doing so deliberately.
fn extract_token(req: &Request) -> Option<String> {
    extract_bearer(req).or_else(|| extract_cookie(req, SESSION_COOKIE))
}

fn extract_bearer(req: &Request) -> Option<String> {
    let val = req.headers().get(header::AUTHORIZATION)?.to_str().ok()?;
    val.strip_prefix("Bearer ").map(|s| s.trim().to_string())
}

/// Pull one cookie out of the `Cookie` header. Hand-rolled instead of pulling
/// in a cookie crate because we need exactly one name out of a
/// `; `-separated list, and the RFC 6265 machinery worth a dependency (domains,
/// expiry, the secure flag) all lives on the browser's side of the wire.
fn extract_cookie(req: &Request, name: &str) -> Option<String> {
    let raw = req.headers().get(header::COOKIE)?.to_str().ok()?;
    raw.split(';')
        .filter_map(|pair| pair.split_once('='))
        .find(|(k, _)| k.trim() == name)
        .map(|(_, v)| v.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request as HttpRequest;

    fn with_cookie(raw: &str) -> Request {
        HttpRequest::builder()
            .header(header::COOKIE, raw)
            .body(axum::body::Body::empty())
            .unwrap()
    }

    #[test]
    fn session_cookie_is_found_among_others() {
        let req = with_cookie("theme=dark; hoard_session=hoard_v1_abc; lang=es");
        assert_eq!(
            extract_cookie(&req, SESSION_COOKIE).as_deref(),
            Some("hoard_v1_abc")
        );
    }

    /// A name that merely *ends* with ours must not match, or a cookie set by
    /// something else sharing the host could stand in for a session.
    #[test]
    fn cookie_names_match_whole() {
        let req = with_cookie("not_hoard_session=evil");
        assert!(extract_cookie(&req, SESSION_COOKIE).is_none());
    }

    #[test]
    fn bearer_wins_over_cookie() {
        let req = HttpRequest::builder()
            .header(header::AUTHORIZATION, "Bearer hoard_v1_header")
            .header(header::COOKIE, "hoard_session=hoard_v1_cookie")
            .body(axum::body::Body::empty())
            .unwrap();
        assert_eq!(extract_token(&req).as_deref(), Some("hoard_v1_header"));
    }
}
