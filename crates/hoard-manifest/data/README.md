# Hoard manifest catalogue

Hand-authored save-path definitions for popular games, used by `hoard-manifest`.

## Licensing

Everything in this directory is **original work by the Hoard project**,
licensed under **AGPL-3.0** (same as the rest of the codebase). It is not
imported from PCGamingWiki or any other CC-BY-NC-SA-licensed source.

When v0.4+ adds bulk PCGamingWiki import, that data will live in a separate
runtime-loaded `data/manifest/` directory shipped alongside the binary, with
its own `LICENSE` file. Mixing licenses inside the binary is not allowed.

## Adding a game

1. Create `games/<slug>.toml` (slug = lowercase-kebab).
2. Fill out fields per the schema (`crates/hoard-manifest/src/schema.rs`).
3. Use placeholder tokens for paths — never hardcode `C:\Users\…`.
   Available tokens: `APPDATA`, `LOCALAPPDATA`, `LOCALAPPDATALOW`,
   `USERPROFILE`, `DOCUMENTS`, `SAVEDGAMES`, `PUBLIC`, `PROGRAMFILES`,
   `PROGRAMFILESX86`.
4. Test locally: `cargo test -p hoard-manifest`.

Keep `source = "hand-curated"`. Anything sourced from PCGamingWiki belongs
in the runtime-loaded directory, not here.
