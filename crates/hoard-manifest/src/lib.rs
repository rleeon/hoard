//! `hoard-manifest` — Ludusavi save-path catalog.
//!
//! Single source of truth for save-path templates: the [Ludusavi
//! manifest][1] (~20k games), bulk-imported from PCGamingWiki. The JSON is
//! embedded at compile time so the desktop app needs no network access to
//! know where saves live; the [`ludusavi`] module also exposes a runtime
//! override path so users can refresh the catalog without re-installing.
//!
//! The hand-curated TOML catalog that used to coexist here was removed in
//! 1.5.0 (P-DET-4). See ADR
//! [`0009-path-detection-overhaul`](../../../docs/decisions/0009-path-detection-overhaul.md)
//! for the rationale.
//!
//! ## Licensing
//!
//! Ludusavi data is sourced from [PCGamingWiki][2] and is licensed
//! CC-BY-NC-SA-3.0. Distributors who want to ship Hoard commercially
//! should remove `data/ludusavi-catalog.json` before bundling.
//!
//! [1]: https://github.com/mtkennerly/ludusavi-manifest
//! [2]: https://www.pcgamingwiki.com/

pub mod ludusavi;
