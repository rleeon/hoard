# Hoard manifest catalogue

Save-path definitions used by `hoard-manifest`.

## `ludusavi-catalog.json` — bulk-imported

Compact JSON derived from the [Ludusavi manifest][ludusavi]
(`mtkennerly/ludusavi-manifest`, MIT-licensed manifest tooling). The
underlying data is sourced from [PCGamingWiki][pcgw] and is licensed
**CC-BY-NC-SA-3.0**. The desktop binary statically embeds this JSON so
detection works fully offline on Windows where the user just installed
the app.

Path templates use Ludusavi's bracket syntax (`<winAppData>`, `<xdgData>`,
`<home>`, `<storeUserId>`, …) — these are expanded by
`hoard-agent::pathexpand`.

The hand-curated TOML catalog that lived alongside this JSON was removed
in 1.5.0 (see ADR `0009-path-detection-overhaul`). The Ludusavi catalog
is now the single source of truth for save-path templates.

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
