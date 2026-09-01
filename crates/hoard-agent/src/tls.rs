//! The one line of setup rustls needs before a WebSocket handshake.
//!
//! `tokio_tungstenite::connect_async` builds its `ClientConfig` with
//! `ClientConfig::builder()`, which asks rustls for the *process-level* crypto
//! provider. rustls picks one from its own crate features, and only when
//! exactly one is enabled. Our tree enables two: reqwest pulls `ring`, the AWS
//! SDK (S3, used by the server) pulls `aws-lc-rs`, and cargo unifies features
//! across a workspace build, so both land in the same `rustls`. With two
//! candidates rustls refuses to guess and `builder()` panics, not at startup
//! but the first time Realtime tries to connect.
//!
//! So we name one. `ring` matches what reqwest already uses for every HTTPS
//! call the app makes, which keeps a single crypto implementation in the
//! binary's hot paths. Installing is idempotent by design: the second call
//! returns `Err` because a provider is already there, which is exactly what we
//! want, so the result is dropped.

use std::sync::Once;

static INSTALL: Once = Once::new();

/// Ensures a rustls crypto provider is installed for this process.
///
/// Call it before anything that reaches `rustls::ClientConfig::builder()`.
/// Cheap and safe to call from every connection attempt: the work happens once.
pub fn ensure_crypto_provider() {
    INSTALL.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}
