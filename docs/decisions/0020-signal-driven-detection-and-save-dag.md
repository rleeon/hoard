# 0020 — Signal-driven detection and the save DAG

## Status

Accepted, 2026-06. Two decisions land together because they share one
goal — make Hoard behave like git for game saves: discover the data
without a catalog, and version it across devices without losing a
divergent branch.

- **Part A — Save DAG (parent pointer + non-fast-forward gate):**
  implemented and shipped behind the existing snapshot/upload paths.
  This ADR documents what already landed.
- **Part B — Signal-driven detection:** a forward-looking direction
  that inverts the post-overhaul pipeline of ADR
  [0009](0009-path-detection-overhaul.md) (catalog-first) into
  catalog-as-safety-net. Phased; only the recall-tuning slice of
  Phase 1 has landed so far. The rest is a plan, not yet code.

Supersedes nothing. Extends ADR 0009 (detection) and ADRs
[0018](0018-storage-efficiency-dedup-retention.md) /
[0019](0019-adaptive-ingestion-and-steam-cloud-detection.md) (storage,
which already gave us git's object store: content-addressed blobs +
content-defined chunking).

---

## Part A — The save DAG

### Context

The repo already had ~80% of git's plumbing: content-addressed
storage (blobs by whole-file SHA + server-side CDC, ADR 0018/0019),
a linear `version_num` per save, and push/pull. The missing piece for
real multi-device sync was the one git solves with parent pointers:
**telling a fast-forward push apart from a divergent one.**

Before this change, `version_num` was a monotonic counter the server
bumped with `+1` on every upload. Two devices that both backed up
"from version 5" produced versions 6 and 7 with no record that they
were *siblings* — the second upload silently won, stacking on top
instead of being recognised as a branch. There was no way to detect
"the server moved on while you were offline."

### Decision

Add a `parent_version` pointer to each stored version, turning the
linear log into a DAG, and gate uploads on a client-declared base:

1. **Schema.** New nullable column on both backends:
   - self-hosted SQLite: migration `0015_snapshot_parent.sql`
     (`snapshots.parent_version INTEGER`).
   - cloud Postgres: migration `postgres/0017_save_version_parent.sql`
     (`save_versions.parent_version BIGINT`).
   `NULL` = root snapshot (first version of a save).

2. **Base declaration.** The client sends `base_version` — the
   version it built on — with each upload (multipart field for
   self-hosted; `UploadInit.base_version` JSON for cloud). The base is
   sourced from `SaveState.last_version_num`.

3. **Non-fast-forward gate.** The server reads the current `head`
   (latest version), and if the client declared a `base` that no
   longer equals `head`, the push diverged → **HTTP 409** with a
   structured body (`code`, `head_version`, `base_version`). Absent
   `base_version` (legacy clients, first upload) the server keeps the
   old append behaviour, so nothing breaks for old agents.

4. **Lineage on insert.** On a successful append,
   `parent_version = (head > 0).then_some(head)`; the new row records
   what it descends from. The DAG is exposed back through snapshot
   detail/list and the cloud manifest so a future resolver can read it.

5. **No retry on conflict (agent scheduler).** A 409 is not transient —
   retrying never fixes a divergence. The auto-sync retry loop treats
   `ApiError::Conflict` as terminal.

sqlx note: the touched self-hosted INSERT was converted to runtime
`sqlx::query()` to avoid regenerating the `.sqlx` offline cache; the
cloud path already uses runtime queries.

### What is explicitly NOT here

Conflict **resolution** is out of scope. This change only *detects*
divergence (the 409) and records lineage (the parent pointer). The
resolution model for binary saves is **keep-both, Steam-style** (let
the user choose / keep both branches), **not** a 3-way merge — saves
are opaque blobs, line-merging is meaningless. That resolver is the
next step and gets its own design when built.

### Consequences

- Multi-device users get correct divergence detection instead of
  last-writer-wins. The second device's push is rejected with enough
  context (head vs base) to drive a future "you have a branch" UI.
- Old clients keep working (graceful append when `base_version` is
  absent), so the rollout needs no flag-day.
- The auto-sync path (`hoard-agent` scheduler / desktop) must track
  the last-uploaded version per save to declare a correct base.
  Wiring `last_uploaded_version` into the scheduler `SaveSlot` is the
  remaining gap; until done, the auto path passes `None` (append, no
  gate) and the CLI path (which has `last_version_num`) is the one that
  exercises the gate.

---

## Part B — Signal-driven detection

### Context

ADR 0009 made detection **catalog-first**: the Ludusavi manifest
(>19k games) supplies save-path templates, and a filesystem heuristic
(`aggressive_discover`) only runs as a fallback for slugs the catalog
left without `found_paths`. That heuristic is narrow: it walks
`install_dir` + `drive_c/users/steamuser` only, and decides save-ness
with a boolean — `name_matches_save_pattern()` exact-matches a tiny
`SAVE_PATTERNS` set, optionally promoted to `Medium` by a recent
save-like file.

Two structural limits follow:

1. **Coverage.** Saves under the real user roots (AppData, LocalLow,
   Documents, Saved Games, `~/.local/share`, `~/.config`, Wine/Proton
   prefixes) are only ever found *if the catalog already names them*.
   A title the catalog doesn't list disappears.
2. **A boolean can't grade.** Folders whose name isn't literally
   "save(s)" are invisible, while the heuristic has no way to express
   "probably a save" vs "definitely a save," so it can neither
   auto-confirm strong evidence nor defer weak evidence to the user.

The deeper observation: the single most reliable save signal is not
the name at all — it's **temporal correlation between a live game
process and a folder being rewritten**. A folder rewritten exactly
while a game runs, and updated when it closes, is almost certainly a
save, regardless of its name or extension. The catalog can't see that;
only the running system can.

### Decision

**Invert the priority: automatic discovery first, catalog as a
justified safety net.** Replace the name boolean with a cumulative
multi-signal score `S ∈ [0,1]`; the dominant signal is
process↔write correlation (+0.50). The catalog stays, but degraded to
exactly four roles where local evidence is structurally insufficient:

1. **Attribution** of GUID/AppID-named folders (which game owns
   `userdata/` or `12345/`).
2. **Windows-registry saves** (Unity PlayerPrefs in
   `HKCU\Software\<Company>\<Product>` — invisible to any file walk).
3. **Saves inside `install_dir`** indistinguishable from assets.
4. **Opaque formats with no observed activity** (game never played
   under observation).

Plus the manifest is reused as a **name dictionary for labelling**
(fuzzy-match a deduced name to a canonical title for icon/metadata),
explicitly **not** as a path source. That label-only use must live in
its own module so it stays auditable.

Decision rule (hybrid engine):

```
score >= 0.60            → accept, source = Automatic;  attribute
0.35 <= score < 0.60     → if catalog corroborates path → source = Hybrid
                           else offer to user (learning loop)
score < 0.35             → discard
catalog path no root visited → if exists → accept, source = Catalog,
                                reason = "invisible_al_walk"
```

Every catalog fallback is **tagged with its reason** — the catalog
never leads discovery, it only corroborates the grey zone and fills
structurally invisible gaps.

### Phased plan (over `detection.rs`)

| Phase | Action | New modules |
|---|---|---|
| **0 — Roots** | Generalise `aggressive_discover()`/`walk_root_collecting()` to all user roots of the section above, not just `install_dir` + steamuser. | `roots.rs` |
| **1 — Scoring** | `classify_dir_as_save_like()` returns graded `f32`; `name_matches_save_pattern` → `name_signal()` (substring + `strsim::jaro_winkler`, multilingual vocab). Widen `SAVE_PATTERNS`; down-weight noisy exts (json/dat/xml). Raise `RECENT_SAVE_FILE_WINDOW` 90→180d. Drop `AGGRESSIVE_WALK_MAX_CANDIDATES`; raise/drop `AGGRESSIVE_WALK_TIMEOUT`. Keep `name_matches_slot_profile_user`, `dir_has_recent_save_file`, `is_skip_dir` (widen the last). | `scoring.rs` |
| **2 — Magic bytes** | Custom `infer` matchers for save signatures (Unreal GVAS, Unity ES3, SQLite, RIFF). SQLite/JSON only add weight alongside another signal. | `magic.rs` |
| **3 — Correlation** ⭐ | `hoard-watcher` (notify) → `dir → last_write`; `sysinfo` samples live processes + focused window. Injects +0.50. | `correlation.rs` |
| **4 — Attribution + learning** | Path segment / process / nearest `install_dir`+`appmanifest` / fuzzy-label; confirm-dismiss feeds weights + local `learned_patterns`. | `attribution.rs`, `learning.rs` |
| **5 — Hybrid + cache** | The decision rule above; incremental cache keyed by `(dir mtime, entry count)` for near-instant rescans. | `cache.rs` |

### Status of the work

Only the low-risk, in-pipeline slice of Phase 1 has landed so far,
marked `DETECCIÓN` in code: `SAVE_PATTERNS` widened with
`savedata`/`save data`/`save_data`/`savefile`/`savefiles`, and
`RECENT_SAVE_FILE_WINDOW` raised 90→180d. Everything else (graded
score, root generalisation, correlation, attribution, cache) is
planned, not built — and each constitutes the architectural deviation
this ADR authorises.

### Rollout gates (don't advance blind)

- **Phase 0+1 first, then benchmark.** Ground-truth = the Ludusavi
  manifest. Require **recall ≥ 70%** of manifest paths recovered
  *without* the catalog, at **precision ≥ 90%** (no auto-confirming
  config dirs). If unmet, reweight signals before advancing.
- **Add Phase 3 once 0/1 is stable** — correlation is what lifts
  recall on non-standard names and solves GUID attribution. If
  correlation alone pushes recall past ~90%, the catalog can drop to
  pure fallback earlier than planned.
- **Phase 2 (magic) last** — lowest marginal ROI, highest
  false-positive risk (SQLite).
- **Success metric for the whole arc:** per detected save, record its
  `source` (Automatic / Hybrid / Catalog). When the stable
  `source=Catalog` fraction falls below ~15–20%, "automatic by
  default" is achieved and the catalog is confined to its safety-net
  role.

### Consequences

- This is a real divergence from ADR 0009's catalog-first pipeline —
  hence this ADR. The catalog is not removed; its role narrows and
  every use of it becomes reason-tagged and measurable.
- New dependencies enter the agent: `strsim` (already in deps for
  fuzzy name match), `infer`, `sysinfo`, and heavier use of
  `hoard-watcher`. The walk gains I/O surface across the whole HOME,
  mitigated by aggressive pruning, bounded depth, a global ~20–30s
  first-scan timeout with cooperative cancellation, and the mtime
  cache for rescans.
- **Permanent blind spots** the catalog must keep covering: Windows
  registry saves (no file walk reaches them) and cold first scans of
  never-observed games (correlation = 0). These are justified
  fallbacks, not failures.
- The learning loop needs logging of confirm/dismiss from day one;
  without that labelled corpus there's no honest way to calibrate the
  Section 2 weights beyond intuition. The weights in the plan are
  starting points, not validated values.

### Alternatives considered and why not

- **Stay catalog-first (status quo).** Rejected: leaves every
  uncatalogued title invisible and can't use the strongest available
  signal (process correlation), which lives only at runtime.
- **Drop the catalog entirely, go fully automatic.** Rejected:
  attribution of GUID folders, Windows-registry saves, and
  install_dir saves are structurally unsolvable from local file
  evidence alone. The catalog earns its keep precisely in those gaps.
- **3-way merge for save conflicts (Part A).** Rejected: saves are
  opaque binaries; keep-both is the only honest model.
- **Detect Proton via `wineserver` process name (carried from ADR
  0009).** Still rejected for the same reason — `compatdata/<appid>`
  is a stronger, attributable signal.
