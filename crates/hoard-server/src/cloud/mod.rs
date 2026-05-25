//! Cloud mode: Postgres + Supabase JWT + Cloudflare R2 + Lemon Squeezy.
//!
//! Entirely behind `--features cloud`. With the feature off, `hoard-server`
//! never sees this module and the self-hosted SQLite + bearer-token flow is
//! the only thing compiled.
//!
//! Layout:
//! - `state` — shared Axum state (Postgres pool, R2 client, JWKS cache,
//!   config snapshot).
//! - `db` — connect + run migrations against Supabase Postgres.
//! - `auth` — JWT validation middleware against Supabase JWKS.
//! - `r2` — small S3-compatible client wrapper for Cloudflare R2.
//! - `quota` — plan limits + middleware that enforces them.
//! - `webhooks` — Lemon Squeezy HMAC verify + subscription state machine.
//! - `routes` — `/v1/cloud/...`, `/v1/me`, `/v1/webhooks/lemonsqueezy`.
//! - `run` — top-level `cloud::run(cfg)` invoked by `main` when
//!   `database.backend = "postgres"`.

pub mod auth;
pub mod bandwidth;
pub mod db;
pub mod errors;
pub mod plans;
pub mod quota;
pub mod r2;
pub mod routes;
pub mod run;
pub mod state;
pub mod webhooks;

pub use run::run;
