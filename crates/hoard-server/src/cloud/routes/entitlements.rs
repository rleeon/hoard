//! `GET /v1/cloud/entitlements` — read-only Pro entitlement snapshot for the
//! desktop UI (badge / lock / days-left). This does NOT start any trial; trials
//! begin on first use of a Pro *content* endpoint. The authoritative check is
//! always per-endpoint `entitlements::require_feature`, never this response.
//!
//! `POST /v1/cloud/features/:feature/activate` — the mutating counterpart used
//! when the user actually opens a Pro feature. It starts the one-month trial on
//! first use (idempotent) and returns the resulting [`FeatureState`], or `402`
//! once the window has elapsed. This is the endpoint that turns a `Free`
//! account's first click into an active `trial`.

use crate::cloud::auth::CloudUser;
use crate::cloud::entitlements::{self, Feature, FeatureState};
use crate::cloud::errors::CloudError;
use crate::cloud::state::CloudState;
use axum::{
    extract::{Path, State},
    response::Json,
    Extension,
};
use serde::Serialize;

#[derive(Serialize)]
pub struct Entitlements {
    pub plan: &'static str,
    pub features: FeaturesOut,
}

#[derive(Serialize)]
pub struct FeaturesOut {
    pub screen: FeatureState,
    pub wrapple: FeatureState,
}

pub async fn get_entitlements(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
) -> Result<Json<Entitlements>, CloudError> {
    let plan = entitlements::current_plan(&state.pool, user.user_id).await?;
    let screen =
        entitlements::feature_state(&state.pool, user.user_id, plan, Feature::Screen).await?;
    let wrapple =
        entitlements::feature_state(&state.pool, user.user_id, plan, Feature::Wrapple).await?;
    Ok(Json(Entitlements {
        plan: plan.as_str(),
        features: FeaturesOut { screen, wrapple },
    }))
}

/// Open (and, on first use, start the trial for) a Pro feature. Returns the
/// resulting state so the client can unlock for the trial window, or `402` once
/// the trial has elapsed and the account isn't on paid Pro.
pub async fn activate_feature(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
    Path(feature): Path<String>,
) -> Result<Json<FeatureState>, CloudError> {
    let feature = Feature::from_str(&feature).ok_or(CloudError::NotFound("unknown feature"))?;
    let plan = entitlements::current_plan(&state.pool, user.user_id).await?;
    entitlements::require_feature(&state.pool, user.user_id, plan, feature).await?;
    let st = entitlements::feature_state(&state.pool, user.user_id, plan, feature).await?;
    Ok(Json(st))
}
