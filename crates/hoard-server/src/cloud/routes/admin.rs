//! The admin panel's three metrics functions, over the API.
//!
//! `tools/admin-dashboard.html` used to call them through Supabase's REST layer
//! (`/rest/v1/rpc/...`), which only reaches a database Supabase hosts. This does
//! what that layer did: open a transaction, put the caller's verified JWT
//! claims in the session, call the function. Who may read stays decided inside
//! each function, `auth.uid()` against the admin's id and 42501 otherwise, so
//! nothing here knows who the admin is and no id ends up in public code.

use crate::cloud::{auth::CloudUser, errors::CloudError, state::CloudState};
use axum::{
    extract::{Extension, Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use sqlx::PgPool;
use uuid::Uuid;

const FUNCTIONS: [&str; 3] = [
    "admin_metrics",
    "admin_metrics_extra",
    "admin_metrics_screen",
];

/// `POST /v1/admin/rpc/:name`. Errors keep the SQLSTATE in `code` the way the
/// REST layer did, because the panel tells a timeout (57014, worth a retry)
/// from a refusal by reading it.
pub async fn rpc(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
    Path(name): Path<String>,
) -> Result<Response, CloudError> {
    let Some(func) = FUNCTIONS.iter().find(|f| **f == name) else {
        return Err(CloudError::NotFound("no such function"));
    };
    match call_as(&state.pool, user.user_id, func).await {
        Ok(json) => Ok(([(header::CONTENT_TYPE, "application/json")], json).into_response()),
        Err(e) => match e.as_database_error().and_then(|d| d.code()).as_deref() {
            Some("42501") => Err(CloudError::Forbidden("not authorized")),
            Some("57014") => Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "statement timeout", "code": "57014" })),
            )
                .into_response()),
            _ => Err(e.into()),
        },
    }
}

/// Call `public.<func>()` as `user_id`. `func` is never taken from a request:
/// the route checks it against [`FUNCTIONS`] first.
#[doc(hidden)]
pub async fn call_as(pool: &PgPool, user_id: Uuid, func: &str) -> Result<String, sqlx::Error> {
    let mut tx = pool.begin().await?;
    // `true` makes the setting local to this transaction. The connection goes
    // back to a pool the rest of the server shares, and no other query should
    // ever run as somebody.
    let claims = serde_json::json!({ "sub": user_id, "role": "authenticated" }).to_string();
    sqlx::query("SELECT set_config('request.jwt.claims', $1, true)")
        .bind(&claims)
        .execute(&mut *tx)
        .await?;
    let json: String = sqlx::query_scalar(&format!("SELECT public.{func}()::text"))
        .fetch_one(&mut *tx)
        .await?;
    tx.rollback().await?;
    Ok(json)
}
