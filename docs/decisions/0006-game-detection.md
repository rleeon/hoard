# 0006 — Game catalog from the Ludusavi manifest

## Status

Accepted, 2026-05.

## Context

Hoard needs to know, for any given game, where its save files live on each
operating system. Maintaining that mapping by hand for ~10k titles is not
viable. Several open data sources exist; the candidates were:

- **PCGamingWiki** — authoritative but only available as a wiki dump; we'd
  need to scrape and keep parsing logic in sync with template changes.
- **Ludusavi manifest** (mtkennerly/ludusavi-manifest) — a community-curated
  YAML on GitHub, MIT-licensed, derived from PCGamingWiki with attribution.
  Already structured around save-path templates and per-OS / per-store
  constraints. Used in production by Ludusavi itself.
- **Roll our own** — start tiny, accept that detection only works for the
  handful of titles we curate.

## Decision

Use the Ludusavi manifest as the upstream catalog. Ship a
`hoard-admin manifest import` command that:

1. Downloads (or reads from disk) the YAML.
2. Transforms each entry into our normalised `save_paths_json` structure
   bucketed by `windows`/`linux`/`mac`.
3. Upserts into the `games` table, **never overwriting** rows that an admin
   added manually (`imported_from = 'manual'`).
4. Optionally prunes stale rows that came from a previous import and have no
   user data attached.

Server endpoints:

- `GET /v1/games/:slug/known-paths` — structured save-path manifest.
- `GET /v1/manifest/version` — last-import metadata, so the client can show
  "catalog as of <date>" and prompt the admin if it gets too old.

The desktop client expands placeholders (`<winAppData>`, `<xdgData>`, …)
locally via `hoard-agent::pathexpand`. Path expansion lives on the client
because the server has no idea what `$HOME` or `%APPDATA%` map to on the
user's machine.

## Consequences

- Catalog freshness depends on a third-party repo. Mitigated by keeping
  imports admin-driven (no auto-pull) and preserving manual overrides.
- We inherit Ludusavi's quirks: some entries are aliases of other entries
  (skipped on import); a small number of paths use placeholders we don't
  understand yet (dropped, not crashed). Both are logged as "skipped".
- Steam Cloud / GOG Cloud signals are derived from the *presence* of a
  store reference, not from the actual cloud setting on the user's account.
  That's a useful "this game probably has its own cloud" hint but not a
  guarantee — the GUI must phrase it as such.
- Slug collisions are theoretically possible (two display names slugifying
  to the same value). Today we resolve by "first wins" inside one import
  pass; if it bites in practice we'll add disambiguation.

## Alternatives considered and why not

- **Bundling a snapshot inside the binary**: would tie catalog updates to
  binary releases. Self-hosters who can't update frequently would fall
  behind, and the binary would grow by ~6 MB.
- **Live API call to a Hoard-hosted catalog service**: introduces a
  central point that contradicts the self-hosted-first ethos.
- **Per-game opt-in hand-curation**: doesn't scale. v0.1 already tried this
  with 10 seeded games and it was not enough for any real test user.
