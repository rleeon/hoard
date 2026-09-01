//! `hoard-manifest`: the Ludusavi save-path catalog.
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
//! **CC BY-NC-SA 3.0**: attribution, share-alike and NonCommercial. Build
//! with `--no-default-features` (feature `bundled-catalog` off) for a binary
//! that carries no catalogue and downloads one at first run; see the module
//! docs of [`ludusavi`] and the root `NOTICE` for why that lever exists and
//! when it has to be pulled.
//!
//! [1]: https://github.com/mtkennerly/ludusavi-manifest
//! [2]: https://www.pcgamingwiki.com/

pub mod ludusavi;
