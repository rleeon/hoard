pub mod auth;
pub mod blobs;
pub mod chunking;
pub mod cleanup;
pub mod clientip;
pub mod compression;
pub mod config;
pub mod db;
pub mod insight;
pub mod patch;
pub mod ratelimit;
pub mod retention;
pub mod routes;
pub mod store;
pub mod upgrade;

/// Shared S3-compatible client. Compiled whenever `s3-backend` is on (which
/// `cloud` implies): the self-hosted S3 blob backend and the cloud R2 client
/// both build on it.
#[cfg(feature = "s3-backend")]
pub mod s3;

#[cfg(feature = "cloud")]
pub mod cloud;
