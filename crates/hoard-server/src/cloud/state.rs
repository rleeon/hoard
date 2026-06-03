//! Shared Axum state for cloud mode.

use crate::cloud::{auth::JwksCache, r2::R2Store};
use crate::config::Config;
use sqlx::PgPool;
use std::{sync::Arc, time::Instant};

#[derive(Clone)]
pub struct CloudState {
    pub pool: PgPool,
    pub config: Config,
    pub jwks: Arc<JwksCache>,
    pub r2: Arc<R2Store>,
    pub start_time: Instant,
}
