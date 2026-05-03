# 0003 — CLI local state stored as JSON, not SQLite

Date: 2026-05-03
Status: accepted

## Context

The build guide originally suggested storing CLI-side state (path mappings,
last backup timestamps, last version) in a small SQLite database at
`~/.local/share/hoard/state.db`, alongside `~/.config/hoard/config.toml`.

The state we actually need is tiny:

- a map from `save_id` → `{ local_path, game_slug, label, last_backup_at, last_version_num }`
- nothing else

There are no joins, no history, no concurrent writers. A new `hoard` invocation
loads the file, mutates the map, and writes it back.

## Decision

Store CLI state in `state.json` next to the config dir, using `serde_json`.

## Consequences

- **Pro**: drops `sqlx`/sqlite from `hoard-cli` deps. The CLI now compiles
  without `libsqlite3` linkage, which matters for future Windows
  cross-compilation (`x86_64-pc-windows-gnu`) where bundling sqlite is fiddly.
- **Pro**: human-readable state file — users can inspect/edit it manually if
  needed (e.g. to retarget a save to a new local path after moving directories).
- **Con**: not safe under concurrent CLI invocations on the same user account.
  We accept this — a single user running `hoard` in parallel against itself is
  not a real workflow.
- **Future**: if state grows (e.g. caching server manifests, holding chunked
  upload resume markers), revisit and migrate to SQLite at that point.
