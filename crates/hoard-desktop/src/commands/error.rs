//! Structured error envelope shared across Tauri commands.
//!
//! Pre-1.5.3 each command surfaced `Result<_, String>` and the UI piped the
//! raw string into a toast. That worked for one-line failures but broke down
//! for the updater, which can emit a 250-char message listing every release
//! asset when no installer matches the platform. We want a single visual
//! block — title + body + collapsible technical detail — and that needs a
//! structured envelope on the wire.
//!
//! `AppError` is intentionally tiny: two i18n keys plus an optional raw
//! detail string. Commands return `Result<T, AppError>` and the frontend
//! resolves the keys through `svelte-i18n` at render time, so a single
//! Rust-side commit ships English / Spanish / German / etc. fluent text
//! without bouncing strings through the bridge.
//!
//! The `From<String>` / `From<&str>` impls keep retrocompat for callers
//! that still use `?` on a `String`-error chain. Those land as
//! `common.error_title` + the raw string as the body — readable, not
//! pretty, but never a regression.

#[derive(Debug, Clone, serde::Serialize)]
pub struct AppError {
    /// i18n key resolved by svelte-i18n on the UI side. Defaults to
    /// `common.error_title` for ad-hoc errors.
    pub title: String,
    /// i18n key resolved by svelte-i18n on the UI side. For raw `From<String>`
    /// conversions this holds the raw message — the dialog renders it
    /// verbatim because the key won't resolve.
    pub body: String,
    /// Optional technical detail surfaced under a "Show details" toggle. Used
    /// to carry the raw `reqwest::Error::to_string()` etc. without polluting
    /// the human-readable body.
    pub detail: Option<String>,
}

impl AppError {
    pub fn new(title: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
            detail: None,
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Wrap a raw error string in an `AppError` with the generic title.
    /// Used by the `From<String>` impl to keep `?` ergonomics working
    /// without forcing every helper to migrate at once.
    pub fn plain(msg: impl Into<String>) -> Self {
        let msg = msg.into();
        Self {
            title: "common.error_title".into(),
            body: msg,
            detail: None,
        }
    }
}

impl From<String> for AppError {
    fn from(s: String) -> Self {
        Self::plain(s)
    }
}

impl From<&str> for AppError {
    fn from(s: &str) -> Self {
        Self::plain(s.to_string())
    }
}
