//! Cloud mode: Postgres + Supabase JWT + Cloudflare R2 + Polar billing.
//!
//! Entirely behind `--features cloud`. With the feature off, `hoard-server`
//! never sees this module and the self-hosted SQLite + bearer-token flow is
//! the only thing compiled.
//!
//! Layout:
//! - `state`: shared Axum state (Postgres pool, R2 client, JWKS cache,
//!   config snapshot).
//! - `db`: connect and run migrations against Supabase Postgres.
//! - `auth`: JWT validation middleware against Supabase JWKS.
//! - `r2`: small S3-compatible client wrapper for Cloudflare R2.
//! - `quota`: plan limits and the middleware that enforces them.
//! - `polar`: Polar (Merchant of Record) Standard-Webhooks verify and
//!   subscription state machine.
//! - `routes`: `/v1/cloud/...`, `/v1/me`, `/v1/webhooks/polar`.
//! - `run`: top-level `cloud::run(cfg)` invoked by `main` when
//!   `database.backend = "postgres"`.

pub mod abandoned;
pub mod abuse;
pub mod account_purge;
pub mod archive;
pub mod auth;
pub mod bandwidth;
pub mod compress;
pub mod db;
pub mod email;
pub mod entitlements;
pub mod errors;
pub mod export;
pub mod loopguard;
pub mod memwatch;
pub mod plans;
pub mod polar;
pub mod pollguard;
pub mod purge;
pub mod quota;
pub mod r2;
pub mod routes;
pub mod run;
pub mod state;
pub mod supabase_admin;
pub mod verify;

pub use run::run;
