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

/// Axum middleware: extract Bearer token, validate against DB, inject AuthUser.
pub async fn require_auth(
    State(pool): State<SqlitePool>,
    mut req: Request,
    next: Next,
) -> Response {
    let token = match extract_bearer(&req) {
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

fn extract_bearer(req: &Request) -> Option<String> {
    let val = req.headers().get(header::AUTHORIZATION)?.to_str().ok()?;
    val.strip_prefix("Bearer ").map(|s| s.trim().to_string())
}
