use axum::{
    http::{header, StatusCode},
    response::{IntoResponse, Json, Response},
};
use serde::Serialize;

/// Cloud-wide error type. Maps to JSON `{error: "...", code: "..."}` and a
/// matching HTTP status. Domain-specific responses (e.g. 402 quota) build
/// their own structured payloads; this is the catch-all for everything
/// else.
#[derive(Debug)]
pub enum CloudError {
    Unauthorized(&'static str),
    /// A 401 with its own `code`, for the client that has to tell this refusal
    /// from an expired token. Older clients only look at the status.
    UnauthorizedCode {
        code: &'static str,
        message: &'static str,
    },
    Forbidden(&'static str),
    /// Like `Forbidden`, but carries a stable machine-readable `code` so the
    /// client can localize the message instead of showing the English
    /// `message` fallback (which older clients that don't know the code show).
    ForbiddenCode {
        code: &'static str,
        message: &'static str,
    },
    NotFound(&'static str),
    BadRequest(String),
    Conflict(&'static str),
    /// A Pro feature is locked (not on Pro, no active trial) → HTTP 402.
    PaymentRequired {
        feature: &'static str,
    },
    /// A per-account cap on a costly action → HTTP 429. `retry_after_secs`
    /// goes in the body as well as `Retry-After`, the order clients read it.
    TooManyRequests {
        code: &'static str,
        message: String,
        retry_after_secs: i64,
    },
    Db(sqlx::Error),
    Internal(anyhow::Error),
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
    code: &'static str,
}

impl IntoResponse for CloudError {
    fn into_response(self) -> Response {
        let (status, code, msg) = match &self {
            CloudError::Unauthorized(m) => {
                (StatusCode::UNAUTHORIZED, "unauthorized", (*m).to_string())
            }
            CloudError::UnauthorizedCode { code, message } => {
                (StatusCode::UNAUTHORIZED, *code, (*message).to_string())
            }
            CloudError::Forbidden(m) => (StatusCode::FORBIDDEN, "forbidden", (*m).to_string()),
            CloudError::ForbiddenCode { code, message } => {
                (StatusCode::FORBIDDEN, *code, (*message).to_string())
            }
            CloudError::NotFound(m) => (StatusCode::NOT_FOUND, "not_found", (*m).to_string()),
            CloudError::BadRequest(m) => (StatusCode::BAD_REQUEST, "bad_request", m.clone()),
            CloudError::Conflict(m) => (StatusCode::CONFLICT, "conflict", (*m).to_string()),
            CloudError::PaymentRequired { feature } => (
                StatusCode::PAYMENT_REQUIRED,
                "pro_required",
                format!("Pro feature '{feature}' is locked"),
            ),
            CloudError::TooManyRequests {
                code,
                message,
                retry_after_secs,
            } => {
                #[derive(Serialize)]
                struct Body<'a> {
                    error: &'a str,
                    code: &'static str,
                    retry_after_seconds: i64,
                }
                let retry = (*retry_after_secs).clamp(1, 86_400);
                return (
                    StatusCode::TOO_MANY_REQUESTS,
                    [(header::RETRY_AFTER, retry.to_string())],
                    Json(Body {
                        error: message,
                        code,
                        retry_after_seconds: retry,
                    }),
                )
                    .into_response();
            }
            CloudError::Db(e) => {
                tracing::error!(error = %e, "db error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "db_error",
                    "database error".to_string(),
                )
            }
            CloudError::Internal(e) => {
                tracing::error!(error = %e, "internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal",
                    "internal error".to_string(),
                )
            }
        };
        (status, Json(ErrorBody { error: msg, code })).into_response()
    }
}

impl From<sqlx::Error> for CloudError {
    fn from(e: sqlx::Error) -> Self {
        CloudError::Db(e)
    }
}

impl From<anyhow::Error> for CloudError {
    fn from(e: anyhow::Error) -> Self {
        CloudError::Internal(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The app tells an unlinked device from an expired token by this code, and
    /// an app older than it by the status alone: both have to hold.
    #[tokio::test]
    async fn an_unlinked_device_gets_a_401_with_its_own_code() {
        let resp = CloudError::UnauthorizedCode {
            code: "session_revoked",
            message: "this device was unlinked from the account; sign in again",
        }
        .into_response();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["code"], "session_revoked");
        assert!(json["error"]
            .as_str()
            .is_some_and(|e| e.contains("unlinked")));
    }
}
