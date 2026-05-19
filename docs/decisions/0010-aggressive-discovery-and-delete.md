# 0010 — Aggressive discovery + complete-delete

## Status

Accepted, 2026-05-18. Lands in 1.5.1. Extends ADR
[0009](0009-path-detection-overhaul.md) — the post-overhaul pipeline is
unchanged in shape; this ADR slots in two new stages and an explicit
recovery path. Does not supersede 0009.

## Context

1.5.0 closed the six structural cracks documented in
[`detection.md`](../plans/detection.md) and ADR 0009. The pipeline is
sound for every slug the Ludusavi catalog covers with a template that
matches the on-disk layout. After two weeks of daily use against the
shipped 1.5.0, three failure modes remain visible to the user and they
all share the same root cause: the pipeline is *passive* about anything
the catalog cannot describe, and *irreversible* once it has latched
onto a bad path.

1. **Steam-detected games with no save path.** The Steam library scan
   surfaces the game (we know it's installed), but the filesystem
   heuristic at
   [`crates/hoard-agent/src/detection.rs`](../../crates/hoard-agent/src/detection.rs)
   only walks paths that come from `entry.paths.<os>`. When Ludusavi
   has no entry for the slug, or has an entry whose templates point at
   a directory the game does not actually use (indies that save inside
   their own install dir, mid-AAA titles that use a custom subdir of
   `AppData/LocalLow` not yet in the catalog), the result is an amber
   "No save folder yet" card with no actionable information. The user
   is being asked to fix something the engine never tried to fix.

2. **Games whose saves live in places the catalog does not list.**
   Recent indies, modded titles, GOG-only games, anything bought outside
   Steam. The user has to remember each path and type it in by hand
   through the override picker. The catalog is a closed-world
   assumption that produces an open-world UX problem.

3. **No way to recover from a bad track.** The papelera button in the
   Library tracked-saves strip ("Desmonitorizar") only removes the row
   from local `CliState`. Snapshots stay on the server by design
   (ADR 0009 consequences: drop the local row, never the user's
   backups). The side effect that ships in 1.5.0 is that *a save
   tracked against the wrong path cannot be re-detected*: on the next
   scan the desktop calls `add_game_to_tracking`, the server returns
   HTTP 409 because the `(user_id, game_slug, label)` row still exists,
   the desktop swallows the conflict, and the bad save_id silently
   re-binds (see the comment on `add_game_to_tracking` 409 handling
   referenced in `CLAUDE.md`). The only way out is `hoard-admin` against
   the server DB, which is not a UX. The server already exposes the
   right primitive —
   `DELETE /v1/saves/{id}` at
   [`crates/hoard-server/src/routes/saves.rs:377`](../../crates/hoard-server/src/routes/saves.rs#L377) —
   but no desktop affordance reaches it.

There is a bonus, fourth class of failure that compounds (3): **saves
orphaned on the server**. If the user reinstalls, switches machines, or
restores from a backup of `state.json` that predates a track, the save
exists server-side but `CliState.saves` does not know about it. The
current `list_tracked_saves` at
[`crates/hoard-desktop/src/commands/library.rs:340`](../../crates/hoard-desktop/src/commands/library.rs#L340)
filters every server save against `cli_state.saves.get(&s.id)` and
`continue`s on miss. Those saves are invisible in the UI and again the
only cleanup path is `hoard-admin`. The filter was added in 1.5.0 for
sound reasons — it suppressed ghost cards that bounced back after
untrack and silenced the amber "no save folder" alert — but the cure
hides legitimate state from the user.

## Decision

Adopt a 1.5.1 pipeline that adds two opt-in stages and one explicit
recovery path, all gated to keep the 1.5.0 hot path unaffected.

1. **Aggressive walker** in
   `crates/hoard-agent/src/detection.rs`, invoked *only* for slugs whose
   `found_paths` are still empty after `refine_save_dir` and before
   `apply_manual_overrides`. The walker takes the Steam install dir
   and/or the Proton prefix root, walks with `depth ≤ 4`, skips a
   denylist of dirs that never hold saves (`bin`, `lib`, `locales`,
   `audio`, `video`, `movies`, `music`, `fonts`, `shaders`, `content`,
   `_CommonRedist`, `vcredist`, `dotnet`, `node_modules`, `.git`,
   `.vs`), and matches dir names against the existing `SAVE_PATTERNS`
   plus a `^(slot|profile|user)[\W_]?\d+$` regex. A hit is `Low`
   confidence on name alone; if the dir contains a file with a
   save-like extension (`.sav .save .profile .json .dat .xml`) modified
   in the last 90 days it promotes to `Medium`. Each root has a 1.5s
   timeout and a cap of 5 candidates. Results merge via the existing
   `merge_fs_hit` path so the rest of the pipeline does not change.

2. **Fuzzy-match fallback for the Steam → catalog cross-reference.**
   When `find_by_steam_app_id` and `find_by_slug(slugify(name))` both
   return None, try a Levenshtein-normalized match across the catalog.
   The threshold is `distance / max(len_a, len_b) < 0.15` — about one
   edit per seven characters, which catches "Definitive" / "Edition" /
   "Remastered" suffixes and minor localization drift without bridging
   genuinely distinct titles. Empirically `Civilization V` vs
   `Civilization VI` normalizes to ≈ 0.07, which still slots under the
   threshold, so the exact-slug match always wins first and the fuzzy
   pass is fallback-only. Ties prefer the entry with `steam_app_id.is_some()`.
   Matches surface with `Confidence::Low` and a diagnostic reason so
   the panel can show why a slug landed on a particular catalog row.

3. **Tauri command `delete_save_completely(save_id)`** in
   `crates/hoard-desktop/src/commands/library.rs`. Calls
   `client.delete_save(&save_id)` (already in
   `crates/hoard-agent/src/api.rs`), then removes
   `cli_state.saves[&save_id]` and any associated
   `cli_state.manual_paths[&game_slug]`, detaches the watcher if running,
   and persists `CliState`. Wired into `invoke_handler!` alongside
   `untrack_save`. The shape is intentionally identical to `untrack_save`
   *plus* the server DELETE call — same persistence path, same watcher
   teardown — so the new command is testable as a thin wrapper without
   duplicating state-management code.

4. **UI: two differentiated buttons in the tracked-saves strip.**
   The existing papelera (`Trash2`, opened-lid) keeps its current
   semantics (drop local row, server untouched). A new `Trash` icon
   (closed-lid, solid) in `text-rose-500` lives next to it and opens a
   confirmation modal with copy that names exactly what disappears
   ("snapshots on the server, no local files, not reversible"). New
   i18n keys land in all eight locales; `es.json` is authoritative for
   user-visible Spanish per `CLAUDE.md`.

5. **Orphan saves stay visible.** `list_tracked_saves` stops dropping
   server saves that have no `CliState` row. The serialized
   `TrackedSave` grows an `orphan: bool` field; when set, the front
   renders a `library.orphan_badge` tag, the papelera (untrack-local)
   is disabled because there is nothing local to untrack, and the new
   red Trash button is the only enabled action. The 1.5.0 reason for
   dropping the row in the first place (ghost cards after untrack
   suppressing the amber "no save folder" alert) is moot once the new
   button exists: the user can now resolve a ghost in one click rather
   than living with it.

## Consequences

- **Coverage rises for uncatalogued slugs and template misses.**
  Indies with `<install>/save/`, GOG re-releases the catalog has not
  picked up yet, modded titles, anything where the slug or template is
  off by a small edit. These were the long tail of amber "No save
  folder yet" cards in 1.5.0 and they are now actionable.

- **The walker can produce false positives.** A dir literally named
  `save` inside a tools subfolder, or a `profile1` dir that is editor
  state and not save state, will surface. Two mitigations: confidence
  is `Low` by default and only promotes to `Medium` with recent
  save-like file contents; the existing amber "Detección heurística —
  revisa antes de añadir" badge from 1.5.0 covers Low-confidence hits.
  The Diagnostics panel (P-DET-6) gains the walker's reasons so a
  developer can see *why* a path landed.

- **Walker performance is bounded.** Depth 4 with a denylist on a
  modern SSD walks a typical install dir in ≤ 200ms. The 1.5s
  per-root timeout is a worst-case ceiling for spinning rust or
  unusually deep mod trees; partial results return on timeout so the
  scan never hangs.

- **Fuzzy match is O(N×M) per scan.** N is the Steam app count
  (typically < 200, hard cap on uncatalogued Steam scan results) and
  M is the catalog size (~20k entries). Worst case is a few million
  short-string Levenshtein computations per scan, all in-memory, all
  on slugified inputs — measured at well under 1s on a 2024-class CPU.
  Run time is paid once per Library scan, not per slug per stage.

- **`delete_save_completely` is destructive and irrevocable.** Mitigation
  is the confirmation modal: red button at `rose-500`/`rose-600` to
  visually distinguish from the lighter zinc papelera, copy that names
  the data lost ("todos los snapshots del servidor"), and an explicit
  "Eliminar definitivamente" action label that the user has to read
  before clicking. The server endpoint stays the same — we are not
  introducing a new destructive primitive, just exposing the existing
  one with the right framing.

- **Orphan visibility ends the "invisible save" class of bug.** Users
  who reinstall or switch machines now see the server's state without
  needing to re-track. The papelera being `disabled` on orphan rows
  makes the right action obvious (the new red button) without
  introducing a third button or a context menu.

- **No catalog format changes.** ADR 0009 left Ludusavi as the single
  source of truth for templates and that is unchanged. The walker is a
  *complement* gated on `found_paths.is_empty()`, not a parallel
  catalog. The fuzzy pass is a lookup strategy inside the same catalog.

- **No new persistence.** `CliState`, `state.json`, `detection.json`,
  and the cached catalog are unchanged. The walker is stateless per
  scan; orphan flag is computed at read time from
  `cli_state.saves.contains_key(&s.id)`.

## Alternatives considered and why not

- **Hand-curated TOML catalog of "what Ludusavi missed."** Rejected.
  ADR 0009 closed the multi-catalog model deliberately and the same
  reasons apply: permanent maintenance overhead, drift between the
  hand-curated side and upstream Ludusavi, no clear ownership for
  who keeps it current. The fuzzy match and the walker together cover
  the gap without forking the catalog story.

- **Run the aggressive walker for every slug, not just empty ones.**
  Rejected. Two costs and no upside. Cost one is performance: every
  scan would walk every install dir and every prefix even when the
  catalog already resolved a path correctly, multiplying scan time by
  install count. Cost two is noise: the walker would surface
  Low-confidence guesses for slugs that already have High-confidence
  catalog hits, forcing the merge logic to dedupe and the UI to
  choose. Gating on `found_paths.is_empty()` keeps the walker in its
  lane — last-chance, only when nothing else fired.

- **Soft-delete the save row instead of hard-deleting.** Rejected. The
  use case is "this track is wrong and I want to re-scan clean." A
  soft-delete leaves the `(user_id, game_slug, label)` UNIQUE row in
  the table and the next `add_game_to_tracking` still 409s and re-binds
  to the now-tombstoned row. To fix that we would either need to
  rewrite the conflict resolution in `add_game_to_tracking` or add a
  separate `resurrect` flow. Untrack already covers the "I want my
  snapshots preserved" case; "Eliminar juego" is the explicit
  destructive twin and a hard DELETE is the right primitive for it.

- **A single "smart" button that picks untrack vs delete based on
  orphan status.** Rejected. The semantics differ enough (one keeps
  server data, one removes it) that conflating them at the affordance
  level invites user error. Two visually distinct buttons with
  different colors and different confirmation copy is the explicit
  choice.

- **Threshold below 0.15 for the fuzzy match.** Tested mentally
  against the failure modes 1.5.0 surfaced — "Stardew Vally" typo,
  "Game Title — Definitive Edition" vs "Game Title". 0.10 drops the
  definitive-edition family; 0.20 starts pulling in sibling titles in
  long-running franchises. 0.15 is the empirical fit and stays the
  conservative side of the trade.
