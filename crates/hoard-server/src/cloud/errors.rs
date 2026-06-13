use axum::{
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde::Serialize;

/// Cloud-wide error type. Maps to JSON `{error: "...", code: "..."}` and a
/// matching HTTP status. Domain-specific responses (e.g. 402 quota) build
/// their own structured payloads — this is the catch-all for everything
/// else.
#[derive(Debug)]
pub enum CloudError {
    Unauthorized(&'static str),
    Forbidden(&'static str),
    NotFound(&'static str),
    BadRequest(String),
    Conflict(&'static str),
    /// A Pro feature is locked (not on Pro, no active trial) → HTTP 402.
    PaymentRequired { feature: &'static str },
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
            CloudError::Forbidden(m) => (StatusCode::FORBIDDEN, "forbidden", (*m).to_string()),
            CloudError::NotFound(m) => (StatusCode::NOT_FOUND, "not_found", (*m).to_string()),
            CloudError::BadRequest(m) => (StatusCode::BAD_REQUEST, "bad_request", m.clone()),
            CloudError::Conflict(m) => (StatusCode::CONFLICT, "conflict", (*m).to_string()),
            CloudError::PaymentRequired { feature } => (
                StatusCode::PAYMENT_REQUIRED,
                "pro_required",
                format!("Pro feature '{feature}' is locked"),
            ),
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
