# 0009 — Path detection overhaul

## Status

Accepted, 2026-05. Lands in 1.5.0. Supersedes the v0.3-era detection paths
in `hoard-detect` and `hoard-agent::autodetect`.

## Context

The detection pipeline in `hoard-agent` produces the Library list, but it
has six structural cracks that the user hits daily:

1. **Proton/Wine is invisible on Linux.** `detect_all()` expands only
   `entry.paths.linux`. Windows-only Steam titles run under Proton store
   their saves in `~/.steam/steam/steamapps/compatdata/<appid>/pfx/drive_c/…`
   and never surface unless the catalog also lists a native Linux path.
2. **The "game root → save subdir" refinement is hardcoded to a single
   slug.** [`crates/hoard-agent/src/detection.rs:118`](../../crates/hoard-agent/src/detection.rs#L118)
   only lists `stellaris`. The full Paradox family (CK3, EU4, HoI4,
   Imperator, Victoria 3) and every other title whose Ludusavi template
   points at the game root ends up tracing the whole root — mods, config,
   telemetry — and triggers spurious backups on every game write.
3. **Steam ↔ catalog cross-reference is keyed only on `steam_app_id`.**
   A Ludusavi entry without an appid surfaces only if the filesystem
   heuristic already finds a save on disk. Steam-installed games with no
   save written yet disappear from the list even when Steam knows them.
4. **`expand_path` mangles absolute literal templates.**
   [`crates/hoard-agent/src/pathexpand.rs:33`](../../crates/hoard-agent/src/pathexpand.rs#L33)
   does a `trim_start_matches('/')` and hands back a relative `PathBuf`.
   The `literal_path_passes_through` test enshrines the bug as the
   expected behaviour.
5. **Two placeholder systems coexist.** Hand-curated TOML in
   `crates/hoard-manifest/data/games/*.toml` uses `{APPDATA}`; the
   Ludusavi catalog uses `<winAppData>`. The hot path in `detection.rs`
   ignores the TOML side entirely. `crates/hoard-agent/src/autodetect.rs`
   and the `crates/hoard-detect/` crate are v0.3 dead code that still
   compiles and misleads anyone tracing the data flow.
6. **No persistent user overrides.** When detection guesses wrong and
   the user corrects it with the folder picker, the correction is not
   stored as an override. Every re-scan suggests the wrong path again.

There is also a seventh, operational gap: when detection misbehaves there
is no way to inspect *why*. No `tracing` events, no diagnostic panel.

## Decision

Adopt the target architecture from `docs/plans/detection.md` §4. The new
pipeline:

1. Load the Ludusavi catalog and the Steam scan results.
2. Cross-reference Steam ↔ catalog by appid, then by slugified name as a
   `Confidence::Low` fallback.
3. Run the native filesystem heuristic against `entry.paths.<os>`, and on
   Linux additionally expand `entry.paths.windows` against every detected
   Proton prefix under `steamapps/compatdata/<appid>/pfx`.
4. Apply a general save-dir refinement to every hit: if the segment name
   already contains a `save`/`saves`/`save games`/`savegame` token, keep
   it; otherwise look for a single matching subdir; otherwise drop the
   hit and let the UI render the amber "pick a folder" affordance.
5. Apply `state.manual_paths` overrides last — the user's pick always
   wins and is emitted as `DetectionSource::ManualOverride`.

Each stage emits structured `tracing` events keyed by slug, and a new
`detection_diagnostics(slug)` Tauri command replays the pipeline into a
`DetectionTrace` for a hidden Settings panel.

Explicit eliminations (`docs/plans/detection.md` §4.3):

- **Delete the `crates/hoard-detect/` crate** from the workspace.
  `process.rs` moves to `hoard-agent/src/process.rs` if any live caller
  still uses it; otherwise it goes with the crate.
- **Delete `crates/hoard-agent/src/autodetect.rs`** and its tests.
  `register_one` / `run_autodetect` are unreferenced from the hot path.
- **Delete the hand-curated TOML catalog** under
  `crates/hoard-manifest/data/games/` along with the `{APPDATA}`-style
  placeholders in `crates/hoard-manifest/src/placeholders.rs` and the
  `catalogue` / `lookup` / `all_games` entry points. Any future
  "manual override" entries are injected into the loaded Ludusavi
  catalog at startup, not maintained as a parallel system.
- **Keep only the Ludusavi catalog** as the source of truth for
  save-path templates. The placeholder vocabulary is the `<winAppData>` /
  `<xdgData>` family from ADR 0006.

Persistence delta: `state.json` gains
`manual_paths: HashMap<String, PathBuf>` (slug → path), written through
the existing atomic save path. `detection.json` and the cached Ludusavi
catalog on disk are unchanged.

## Consequences

- Linux/Proton users — the bulk of the current install base, including
  Steam Deck — get a usable Library on first scan instead of an empty
  one. This is the headline win that justifies pulling the work out of
  the 1.8.0 bucket where it was originally parked.
- Deleting the hand-curated TOML catalog breaks nothing on the hot path,
  because nothing on the hot path consulted it. The risk is a forgotten
  consumer elsewhere in the tree; P-DET-4 mandates an exhaustive `grep`
  before deletion and a stop-and-ask if anything turns up.
- `manual_paths` introduces an explicit divergence point between the
  catalog's suggestion and the user's pick. The contract is unambiguous:
  the override always wins, persists across re-scans, and is only undone
  by an explicit "use auto-detection" action.
- Telemetry adds INFO/DEBUG events per slug per stage. With ~20k catalog
  entries and a Semaphore-bounded scan, the volume is bearable, but the
  default log level may need to drop from `info` to `warn` for the
  detection module if it surfaces as noise in production logs.
- The general save-dir refinement may produce false negatives for games
  whose save folder doesn't match any of the token patterns. Those slugs
  appear in amber and the user picks a folder once; the pick persists
  via `manual_paths`. A `SAVE_DIR_OVERRIDES` table remains for atypical
  cases that warrant a hard-coded answer.
- The diagnostics command and panel add a maintenance surface but are
  the only way to keep the pipeline debuggable as the catalog evolves.
  Cost of *not* having it is paid every time a user reports "X doesn't
  show up" and a developer has to read code instead of a trace.

## Alternatives considered and why not

- **Keep both catalogs (TOML hand-curated + Ludusavi).** Rejected.
  Permanent maintenance overhead with no upside: the TOML side has no
  consumer on the hot path, and merging hand-curated overrides into the
  loaded Ludusavi catalog gives us the same flexibility without a
  parallel system to keep in sync.
- **Detect Proton via a `wineserver` process-name heuristic.** Rejected.
  False positives on Lutris and Bottles, and a process-name signal
  cannot tell us *which* Steam appid a prefix belongs to. The
  `compatdata/<appid>` directory layout is a stronger and more direct
  signal that maps one-to-one to catalog entries.
- **Prompt the user to pick every save path manually on first run.**
  Rejected. Violates the "Hoard works cold" promise in the README and
  produces an onboarding cliff that drops users before they see the
  product's value. The pipeline must do its best work before asking
  anything; the picker is the fallback, not the default.
