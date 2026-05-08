# Hoard manifest catalogue

Save-path definitions used by `hoard-manifest`.

## Two catalogs

### 1. `games/*.toml` — hand-authored

Original work by the Hoard project, **AGPL-3.0**. Use placeholder tokens
like `{APPDATA}` / `{DOCUMENTS}` (resolved via Windows Known Folders).

### 2. `ludusavi-catalog.json` — bulk-imported

Compact JSON derived from the [Ludusavi manifest][ludusavi]
(`mtkennerly/ludusavi-manifest`, MIT-licensed manifest tooling). The
underlying data is sourced from [PCGamingWiki][pcgw] and is licensed
**CC-BY-NC-SA-3.0**. The desktop binary statically embeds this JSON so
detection works fully offline on Windows where the user just installed
the app.

Path templates use Ludusavi's bracket syntax (`<winAppData>`, `<xdgData>`,
`<home>`, `<storeUserId>`, …) — these are expanded by
`hoard-agent::pathexpand`, **not** by `hoard-manifest::placeholders`. The
two placeholder vocabularies are intentionally disjoint:

- TOML hand-curated entries → `{TOKEN}` syntax → `placeholders.rs`
- JSON Ludusavi entries     → `<token>` syntax → `pathexpand.rs`

When both contain the same game, the hand-curated TOML wins (it's
narrower, better-tested, and AGPL-clean for embedding).

[ludusavi]: https://github.com/mtkennerly/ludusavi-manifest
[pcgw]: https://www.pcgamingwiki.com/

## Refreshing the Ludusavi catalog

```bash
curl -fsSL \
  https://raw.githubusercontent.com/mtkennerly/ludusavi-manifest/master/data/manifest.yaml \
  -o crates/hoard-manifest/data/ludusavi-manifest.yaml

python3 crates/hoard-manifest/data/convert-ludusavi.py
rm crates/hoard-manifest/data/ludusavi-manifest.yaml
```

The conversion script strips the YAML to just `(slug, display_name,
steam_app_id, paths_per_os)`. The raw YAML is **not** committed — only
the compact JSON is, to keep the repo small.

## Adding a hand-curated game

1. Create `games/<slug>.toml` (slug = lowercase-kebab).
2. Fill out fields per the schema (`crates/hoard-manifest/src/schema.rs`).
3. Use `{TOKEN}` placeholders — never hardcode `C:\Users\…`. Available
   tokens: `APPDATA`, `LOCALAPPDATA`, `LOCALAPPDATALOW`, `USERPROFILE`,
   `DOCUMENTS`, `SAVEDGAMES`, `PUBLIC`, `PROGRAMFILES`, `PROGRAMFILESX86`.
4. Test locally: `cargo test -p hoard-manifest`.

Keep `source = "hand-curated"`.
