# 0011 — Windows detection + extra prefixes (Lutris, Bottles)

## Status

Accepted, 2026-05-19. Lands in 1.5.2. Extends ADRs
[0009](0009-path-detection-overhaul.md) and
[0010](0010-aggressive-discovery-and-delete.md) — the post-overhaul
pipeline is unchanged in shape; this ADR adds new *sources* (non-Steam
launchers, additional wine prefixes), a new *expansion stage* (registry
paths) and *OneDrive-aware* Known Folder resolution. Does not supersede
either antecedent.

## Context

1.5.0 closed the structural cracks of the detection pipeline (ADR 0009)
and 1.5.1 added the aggressive walker plus fuzzy matching against the
Ludusavi catalog (ADR 0010). The result is a Library that resolves the
typical Steam-on-Linux and Steam-on-Windows user well, with Proton
prefixes covered on the Linux side.

The recon that opened the 1.5.2 cycle (`docs/plans/1.5.2.md` §0)
confirmed five gaps that disproportionately penalise Windows users and
non-Steam Linux gamers. Every one of them has the same shape: the
pipeline is silent because an *input* never reaches it, not because a
stage misbehaves.

1. **Non-Steam launchers are invisible.**
   [`crates/hoard-agent/src/steam.rs`](../../crates/hoard-agent/src/steam.rs)
   is the only source of "what games are installed on this host". The
   public surface is `list_installed_steam_games` at
   [`crates/hoard-agent/src/steam.rs:84`](../../crates/hoard-agent/src/steam.rs#L84)
   and `list_proton_prefixes` at
   [`crates/hoard-agent/src/steam.rs:153`](../../crates/hoard-agent/src/steam.rs#L153).
   Games purchased on Epic Games Store, GOG Galaxy, Microsoft Store /
   Xbox Game Pass, EA Play, Ubisoft Connect or Battle.net never enter
   the cross-reference, so they only surface if the filesystem
   heuristic stumbles into the install dir. The comment that promises
   broader coverage (`crates/hoard-agent/src/detection.rs:16` —
   "Catches GOG, Epic, DRM-free") is aspirational.

2. **Registry paths in the Ludusavi catalog are dropped silently.**
   Upstream Ludusavi documents save locations in two parallel fields:
   `files:` for filesystem templates and `registry:` for paths the
   game reads from `HKEY_*` (typical of Bethesda Creation-engine titles
   and several Paradox classics that persist `SaveLocation` in the
   registry). The YAML parser at
   [`crates/hoard-manifest/src/ludusavi.rs:277-300`](../../crates/hoard-manifest/src/ludusavi.rs#L277)
   only deserializes `alias`, `files` and `steam` —
   `YamlEntry { alias, files, steam }` has no `registry` field — and
   `transform_files` at
   [`crates/hoard-manifest/src/ludusavi.rs:397`](../../crates/hoard-manifest/src/ludusavi.rs#L397)
   never sees registry keys. Every save path that lives behind a
   registry indirection is invisible to detection.

3. **`<winSavedGames>` is not a known token.** Windows Vista and later
   ship the `%USERPROFILE%\Saved Games` Known Folder (`FOLDERID_SavedGames`),
   which a non-trivial number of modern games use as the canonical
   per-user save root. The placeholder table in
   [`crates/hoard-agent/src/pathexpand.rs:116-178`](../../crates/hoard-agent/src/pathexpand.rs#L116)
   does not list `winSavedGames`, so any catalog template using that
   token falls through to the `trace!("unknown path placeholder")`
   branch at line 165 and is dropped.

4. **OneDrive redirects Known Folders and the expander does not
   notice.** Modern Windows installs frequently redirect Documents,
   AppData, Pictures and friends to
   `C:\Users\<user>\OneDrive\<folder>` via the per-user shell folder
   table at
   `HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\User Shell Folders`.
   The expander at
   [`crates/hoard-agent/src/pathexpand.rs:125-132`](../../crates/hoard-agent/src/pathexpand.rs#L125)
   resolves `<winDocuments>` as `home_dir().join("Documents")` and
   `home_dir` at
   [`crates/hoard-agent/src/pathexpand.rs:187-199`](../../crates/hoard-agent/src/pathexpand.rs#L187)
   reads `HOME` then `USERPROFILE`. Neither path consults Shell
   Folders, so `<winDocuments>/<game>` points at a directory that
   either does not exist (no Documents in `%USERPROFILE%`) or is the
   wrong one (a Documents the game never writes to because OneDrive
   owns the live copy). The docstring on `winDocuments` even
   acknowledges the gap explicitly ("can be redirected via Known
   Folders, but the env-based fallback is what 99% of installs use") —
   a "99% of installs" assumption that is no longer true on modern
   Windows 10/11 with OneDrive enabled by default.

5. **Lutris and Bottles prefixes are invisible.** 1.5.0 brought Proton
   coverage with `list_proton_prefixes`. The same shape applies to
   Lutris (`~/.local/share/lutris/runners/wine/<runner>/prefixes/<id>/`)
   and Bottles (`~/.local/share/bottles/bottles/<name>/` native plus
   `~/.var/app/com.usebottles.bottles/data/bottles/bottles/<name>/`
   flatpak), but no equivalent enumerator exists. Users who play
   Windows-only titles outside Steam on Linux get the same empty
   Library the Steam-on-Linux user used to see before 1.5.0.

## Decision

Adopt the 1.5.2 architecture from `docs/plans/1.5.2.md` §4. Six pieces,
each behind cfg-gated platform-specific code with no-op cross-platform
stubs so the orchestration in `detect_all` stays cfg-free.

1. **New module `crates/hoard-agent/src/launchers.rs`** with unified
   parsers for Epic Games, GOG Galaxy and Microsoft Store / Xbox Game
   Pass. Public surface:

   ```rust
   pub enum LauncherKind { Epic, Gog, MicrosoftStore }
   pub struct LauncherApp {
       pub launcher: LauncherKind,
       pub app_id: String,
       pub name: String,
       pub install_dir: PathBuf,
   }
   pub fn list_installed_epic_games(os: Os) -> Vec<LauncherApp>;
   pub fn list_installed_gog_games(os: Os) -> Vec<LauncherApp>;
   pub fn list_installed_msstore_games(os: Os) -> Vec<LauncherApp>;
   ```

   Epic reads `%PROGRAMDATA%\Epic\EpicGamesLauncher\Data\Manifests\*.item`
   JSON manifests (`InstallLocation`, `DisplayName`, `CatalogItemId`).
   GOG opens `%LOCALAPPDATA%\GOG.com\Galaxy\storage\galaxy-2.0.db` via
   `rusqlite` and probes the install-product tables Galaxy ships
   (schema drifts between Galaxy releases; the parser tries the known
   table names and returns an empty Vec rather than panicking when
   the schema does not match). Microsoft Store enumerates
   `HKCU\Software\Microsoft\GamingServices\PackageRepository\Package`
   and harvests install dirs by package family name. All three return
   `Vec::new()` on non-Windows.

2. **Ludusavi schema extended to capture `registry:`.**
   `LudusaviEntry` grows `pub registry: Vec<RegistryPath>` (default
   empty, `#[serde(default)]`) with

   ```rust
   pub struct RegistryPath {
       pub key: String,             // e.g. "HKEY_CURRENT_USER/Software/Foo/Bar"
       pub value: Option<String>,   // None ⇒ read the default value
   }
   ```

   `YamlEntry` and `convert_yaml_to_catalog` learn to parse the
   `registry:` block in the upstream YAML (a map keyed by registry
   key, with the same `when:`/`tags:` substructure as files), mapping
   each key into a `RegistryPath` with `value = None`. The embedded
   compact JSON is regenerated to include the field so existing
   on-disk caches keep parsing under `#[serde(default)]`.

3. **`expand_registry_path` in `pathexpand.rs`, Windows-only.**
   Signature:

   ```rust
   #[cfg(windows)]
   pub fn expand_registry_path(reg: &RegistryPath) -> Vec<PathBuf>;
   #[cfg(not(windows))]
   pub fn expand_registry_path(_reg: &RegistryPath) -> Vec<PathBuf> { Vec::new() }
   ```

   Implementation splits the hive prefix from the subkey, opens the
   hive via `winreg::RegKey::predef`, reads `reg.value` (or the
   default value when `None`) as a string. Literal absolute strings
   are returned as-is; templates that still contain Ludusavi
   placeholders re-enter `expand_path` recursively. Missing keys,
   unreadable values and non-string value types return an empty Vec
   — never panic, never log above `debug!`.

   The `winreg` dependency lives in `crates/hoard-agent/Cargo.toml`
   under `[target.'cfg(windows)'.dependencies]` so non-Windows builds
   keep their current compile profile.

4. **`<winSavedGames>` token plus OneDrive-aware Known Folders.**
   `expand_placeholder` learns `winSavedGames`, mapping to
   `FOLDERID_SavedGames` via `SHGetKnownFolderPath` on Windows
   (`windows-sys` with `Win32_UI_Shell` + `Win32_System_Com` features,
   also cfg-gated to Windows), with a `%USERPROFILE%\Saved Games`
   fallback when the call fails. The prefix-mapped counterpart in
   `expand_placeholder_in_prefix` maps it to
   `users/steamuser/Saved Games` for Wine/Proton consistency.

   For `<winDocuments>`, `<winAppData>`, `<winLocalAppData>`,
   `<winLocalAppDataLow>`, `<winPublic>` and `<winProgramData>`, the
   Windows branch consults
   `HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\User Shell Folders`
   first (values `Personal`, `AppData`, `Local AppData`, …), expands
   any `%USERPROFILE%` placeholders found in the registry value, and
   falls back to the current env-var resolution only when the
   registry key is missing or unreadable. The helper
   `windows_known_folder(token) -> Option<PathBuf>` is `#[cfg(windows)]`
   and the non-Windows match keeps its current behaviour byte-for-byte.

5. **New module `crates/hoard-agent/src/wine_prefixes.rs`.** Absorbs
   Proton enumeration and adds Lutris and Bottles. Public surface:

   ```rust
   pub enum PrefixKind { Proton, Lutris, Bottles }
   pub struct WinePrefix {
       pub kind: PrefixKind,
       pub identifier: String,       // appid for Proton, slug for the rest
       pub prefix_root: PathBuf,
   }
   pub fn list_wine_prefixes(os: Os) -> Vec<WinePrefix>;
   ```

   Proton path delegates to `steam::list_proton_prefixes` (kept as the
   canonical primitive — already covered by the tests at
   `crates/hoard-agent/src/steam.rs:488-515`). Lutris walks
   `~/.local/share/lutris/runners/wine/*/prefixes/*` (each subdir is
   one prefix; the dir name becomes the identifier slug). Bottles
   walks both the native (`~/.local/share/bottles/bottles/*`) and
   flatpak (`~/.var/app/com.usebottles.bottles/data/bottles/bottles/*`)
   roots, where the bottle directory itself is the prefix (`drive_c/`
   lives directly inside). Non-Linux returns Proton-only or empty.

6. **Aggressive walker (1.5.1) fed by the new inputs.** `detect_all`
   in `crates/hoard-agent/src/detection.rs` builds
   `install_dir_by_slug: HashMap<String, PathBuf>` from Steam, Epic,
   GOG and Microsoft Store after cross-referencing each launcher app
   against the catalog (exact slug, then fuzzy threshold 0.15 as in
   ADR 0010), and `prefix_root_by_slug` from the unified wine-prefix
   list. The walker call site is unchanged in shape — same denylist,
   same depth-4 cap, same `Confidence::Low`/`Medium` promotion rules.

## Consequences

- **Windows coverage rises sharply.** The Big-3 non-Steam launchers
  (Epic, GOG Galaxy, Microsoft Store / Game Pass) plus registry-backed
  saves cover the long tail of "I bought this game and Hoard does not
  see it" reports that 1.5.1 left unresolved. Combined with OneDrive
  awareness, the typical modern Windows install no longer needs the
  user to override `<winDocuments>` by hand.
- **Linux coverage gains Lutris and Bottles.** Same shape and
  ergonomics as the 1.5.0 Proton story, no new user-facing concepts.
- **New Windows-only dependencies expand the build surface.** `winreg`
  (lightweight, no C deps), `windows-sys` with two narrow feature
  flags (Shell + COM, both header-only), and `rusqlite` with the
  `bundled` feature (the heavyweight: compiles SQLite C). All three
  are gated to `[target.'cfg(windows)'.dependencies]`; the Linux and
  macOS build profile picks up zero new crates. The non-trivial cost
  is Windows CI/build time; mitigated by `rusqlite` `bundled` being
  the only large addition and shipping a pre-vendored SQLite that
  does not require system libs.
- **`#[cfg(windows)]` surface grows; orchestration stays cfg-free.**
  Every new Windows-specific function has a non-Windows counterpart
  with identical signature that returns `Vec::new()` or its
  equivalent. `detect_all` calls the public APIs unconditionally, so
  the pipeline keeps a single code path across platforms (the same
  contract ADR 0009 established).
- **Registry expand can read user-controlled data.** Mitigated by
  treating registry values as untrusted: the function only returns
  paths after they parse as `PathBuf`, only stats them through the
  same heuristic the rest of the pipeline uses (never executes, never
  opens for write), and never propagates registry errors above
  `tracing::debug!`. A malicious registry value at worst causes a
  spurious "no save folder yet" amber card — the same failure mode
  as a catalog template miss, which the UI already handles.
- **Fuzzy match across all launchers, not just Steam.** ADR 0010's
  0.15 normalised-Levenshtein threshold runs against Epic/GOG/MS
  display names too. The empirical bound from 0010 still applies
  (Civilization V vs Civilization VI ≈ 0.07 ⇒ exact-slug match wins
  first), so the threshold does not need to change. The N×M cost
  rises proportionally with installed-game count, which is bounded
  in practice (few hundred per host) and runs once per Library scan.
- **Saved Games + Known Folders resolution adds one registry read per
  Known-Folder placeholder per scan.** Cached in a
  `WindowsKnownFolders` struct populated once at scan start, so the
  added I/O is six registry reads total. Trivial cost.
- **No catalog format break.** ADR 0009's single-source rule
  (Ludusavi is the truth) is preserved. The `registry:` field is
  additive in `LudusaviEntry`; old cached catalogs without it still
  deserialize under `#[serde(default)]`.
- **No persistence delta.** `state.json`, `detection.json` and the
  cached catalog file shape are unchanged. `manual_paths` continues
  to win over every other source (ADR 0009 contract).

## Alternatives considered and why not

- **Cover EA Play, Ubisoft Connect and Battle.net in this cycle.**
  Rejected for 1.5.2. Each ships a proprietary, undocumented manifest
  store (EA: `IGOProxy.exe` config; Ubisoft: `uplay_install.state`
  binary blob; Battle.net: `product.db` Heroes-of-Newerth-era
  protobuf). The parsers would consume a disproportionate share of
  the cycle for a tail-end ROI — the Big-3 (Epic, GOG, Microsoft)
  cover the bulk of non-Steam Windows installs in 2026 telemetry.
  Deferred to 1.5.3 or 1.6.x if the demand surfaces.
- **Shell out to PowerShell `Get-AppxPackage` for Microsoft Store
  detection.** Rejected. Spawning PowerShell adds 200-800ms per scan
  (startup time alone), depends on the user's execution-policy
  configuration (signed-scripts-only hosts refuse to run our query),
  and produces text the Rust side has to parse heuristically. Reading
  the GamingServices package registry directly is faster (microseconds),
  fail-safe (missing key ⇒ empty Vec), and uses the same `winreg`
  dependency the registry-expand stage already requires.
- **Inline registry paths into the embedded catalog rather than a
  separate `registry: Vec<RegistryPath>` field.** Rejected. Folding
  registry keys into `files:` strings (e.g. `"HKCU/...":` entries
  next to `<winAppData>/...`) breaks compatibility with upstream
  Ludusavi YAML and forces every future refresh of the runtime
  override to run a custom transformer. The separate field
  round-trips byte-for-byte with upstream and keeps
  `convert_yaml_to_catalog` a thin transform.
- **Hot-swap `list_proton_prefixes` for `list_wine_prefixes` and
  delete the Proton-only function.** Rejected. The existing function
  is `pub`, exercised by `list_proton_prefixes_detects_appids_with_pfx`
  / `list_proton_prefixes_empty_when_no_steam` at
  `crates/hoard-agent/src/steam.rs:488-515`, and referenced by
  callers outside the immediate detection module. Keeping it as the
  canonical primitive that `wine_prefixes::list_wine_prefixes`
  delegates to preserves API stability and the test coverage we
  already have, at zero runtime cost.
- **Resolve Known Folders via `directories::BaseDirs` (or `dirs`)
  instead of `SHGetKnownFolderPath` + Shell Folders registry.**
  Rejected. Both crates resolve only a small subset of Known Folders
  (Documents, AppData, Local) and use the same env-var lookup the
  current code already uses — they would not pick up OneDrive
  redirects either. `SHGetKnownFolderPath` is the only OS-supported
  way to get the live, redirect-aware Known Folder path, and reading
  `User Shell Folders` registry is the documented fallback when the
  API is unavailable.
- **Drop OneDrive-redirected paths when the env var and the registry
  disagree.** Rejected. The registry value is the source of truth
  for "where Windows itself believes Documents lives right now"; the
  game uses the same APIs the registry feeds. If the path does not
  exist on disk, the filesystem heuristic naturally falls through to
  the next candidate (`stat` returning `NotFound` is already handled
  by the merge logic). No reason to second-guess the OS.
