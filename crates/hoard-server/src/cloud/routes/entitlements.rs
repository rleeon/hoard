//! `GET /v1/cloud/entitlements` — read-only Pro entitlement snapshot for the
//! desktop UI (badge / lock / days-left). This does NOT start any trial; trials
//! begin on first use of a Pro *content* endpoint. The authoritative check is
//! always per-endpoint `entitlements::require_feature`, never this response.

use crate::cloud::auth::CloudUser;
use crate::cloud::entitlements::{self, Feature, FeatureState};
use crate::cloud::errors::CloudError;
use crate::cloud::state::CloudState;
use axum::{extract::State, response::Json, Extension};
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
