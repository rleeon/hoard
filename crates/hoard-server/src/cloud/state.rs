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

impl CloudState {
    /// A state for integration tests: real pool, real R2 client (pointed
    /// wherever the caller likes, usually nowhere), and a JWKS cache that
    /// authenticates nobody. It exists because the cloud module talks to
    /// Postgres through runtime queries that no compiler checks, so the only
    /// way to know they still work is to run them.
    #[doc(hidden)]
    pub fn for_test(pool: PgPool, config: Config, r2: Arc<R2Store>) -> Self {
        Self {
            pool,
            config,
            jwks: JwksCache::offline(String::new()),
            r2,
            start_time: Instant::now(),
        }
    }
}
