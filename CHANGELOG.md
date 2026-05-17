# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [1.4.4] — 2026-05-17

Settings UX nit reported right after 1.4.3 shipped: the new auto-restore
toggle was hiding in its own "Sync" section, which is a section
masquerading as a category when it only holds one switch.

### Changed

- **Auto-restore toggle promoted into the General section.** It now
  sits next to "Minimize to tray" — same category of "how Hoard behaves
  day to day" — so users actually find it without having to scroll past
  Language / Startup / Notifications / Privacy looking for a Sync
  heading that barely held one row.

### Removed

- `settings.section_sync` translation key (no longer rendered). The
  eight locale files lose the now-orphaned heading string.

## [1.4.3] — 2026-05-17

Two related bugs the 1.4.2 auto-restore feature surfaced once users
started exercising the "save folder went missing" path in anger:

### Fixed

- **Empty folders no longer push an empty snapshot to the server.** A
  user reported deleting their local save and watching the agent fire a
  backup that "failed because there was nothing to upload". The fs
  watcher *does* fire on deletes (that's the same inotify event you get
  on writes), so `schedule_backup` got armed and then `upload_directory`
  walked an empty tree. We now pre-check the local path inside
  `run_backup_with_retry`: if the folder is missing or contains zero
  entries we skip the upload entirely. Pushing an empty snapshot would
  have silently rotated the last good copy on the server out from
  under the user the next time they looked at History — much worse
  than the visible failure the bug originally caused.
- **Auto-restore now triggers on the fs path, not just on add.** 1.4.2
  only restored when the agent attached to a save with an empty folder.
  If the folder went empty mid-session (uninstall, manual cleanup), the
  agent kept trying to back it up forever. With `auto_restore = true`,
  the same empty-folder pre-check now spawns a restore from the latest
  server snapshot and re-arms the fs watcher against the repopulated
  directory.

### Added

- **`AgentEvent::BackupSkippedEmpty`** + `agent://backup-skipped-empty`
  Tauri channel. Fires when `auto_restore = false` and the local folder
  is empty at backup time. The UI shows an info toast pointing the user
  at the Settings toggle — that way "nothing happened" doesn't read as
  "the agent is broken".
- Eight-locale translation for the new toast string.

### Notes

- The pre-check uses the same `is_path_empty_or_missing` helper as the
  on-attach auto-restore path, so the bar to write user data is
  identical: a populated folder is never touched, and a folder we
  can't enumerate (NFS hiccup) is treated as not-empty rather than
  not-empty-so-overwrite.

## [1.4.2] — 2026-05-17

Opt-in cloud restore on add. The first concrete step of the 1.5.0 client
polish track: when you attach a tracked save whose local folder doesn't
exist or is empty (fresh install of the game, new machine, accidentally
wiped folder), the agent can now pull the latest server snapshot in the
background instead of leaving the slot empty until the user remembers
to "Restore" manually.

### Added

- **`Prefs.auto_restore` + Settings → Sync section.** Off by default —
  silently writing files under the user's `~` is the kind of side-effect
  that earns trust slowly, so it's behind an explicit toggle. The new
  *Sync* section lives between *Startup* and *Notifications* in
  Settings, with a one-line explanation of what gets restored and when.
- **`AgentEvent::SaveAutoRestored` / `SaveAutoRestoreFailed`.** Emitted
  by `hoard-agent` after the background restore lands (or fails). The
  desktop subscribes to `agent://save-auto-restored` and
  `agent://save-auto-restore-failed` and pops an in-app toast so the
  user can see that files appeared without having to refresh the page.
- **8 locales kept in sync.** Five new strings (toast success, toast
  failure, section header, toggle label, toggle description) translated
  into en/es/de/fr/it/ja/pt/zh.

### Changed

- `handle_add` in `hoard-agent` now takes the api client + event sender
  so it can spawn an auto-restore task when the local path is empty.
  The new internal `RearmWatcher` command re-attaches the fs debouncer
  to the now-populated folder so subsequent saves are picked up.

### Notes

- Restore is gated by `is_path_empty_or_missing`: a populated folder
  is never touched, and a folder we can't enumerate (NFS hiccup) is
  treated as not-empty rather than not-empty-so-overwrite. The bar to
  write user data is "we're 100% sure there's nothing there".
- Failure is final: a network error or sha mismatch surfaces as a toast
  and the slot is left untouched. The user can re-attempt manually
  from History.

## [1.4.1] — 2026-05-17

Emergency follow-up to 1.4.0. Two bugs in the in-app upgrade flow that
only surfaced once users tried it against the actual GitHub release:

### Fixed

- **App refused to launch after upgrade.** `setup()` called
  `commands::library::spawn_periodic_rescan`, which used `tokio::spawn`
  before Tauri had entered its event loop — so the very first thing the
  1.4.0 binary did was panic with *"there is no reactor running, must be
  called from the context of a Tokio 1.x runtime"*. On Windows this
  manifested as an instant exit with no window ever painting; on Linux
  the .deb installed cleanly but reopening from the terminal printed
  the panic and bailed. Replaced with `tauri::async_runtime::spawn`,
  matching the sibling helper `auto_update_catalog_in_background`.
- **Old process kept running after `dpkg -i` / `msiexec`.**
  `apply_desktop_update` returned `InstallerLaunched` without telling
  the app to exit, so the user stayed on the 1.3.5 window even though
  the new binary was already on disk. On Windows this also blocked
  msiexec from overlaying the running `.exe` cleanly. After a
  successful installer launch we now wait 1.5 s (long enough for the
  frontend to paint the "installer launched" toast), then on Linux
  spawn the freshly-installed binary via `setsid` so it outlives us
  and call `app.exit(0)`. Windows and macOS just exit — `msiexec` is
  still running async and the .exe is mid-replace, and `open` on
  macOS hands Finder the .dmg; relaunching either would race.

## [1.4.0] — 2026-05-17

Reliability + polish cycle. The big one: auto-backup was silently broken
for any game whose Ludusavi entry has no `processes` list and isn't a
Steam install — the filesystem watcher was being armed lazily on
`GameStarted`, which never fired for those titles, so the Dashboard pill
stayed "Inactivo" forever even while the user was saving in-game. Fixed
by arming the watcher unconditionally on `handle_add` and demoting
`process_poll` to a pure UI signal. Plus: the detection report now
survives restarts, the sidebar nav re-translates with the rest of the
UI, the Dashboard pill no longer lies on cold boot, and the desktop
update probe runs on a 6h timer instead of only at launch.

### Added

- **Persistent detection cache.** `DetectionReport` now serialises to
  `cache.json` alongside `CliState`, with a 24h auto-rescan and an
  explicit "Re-escanear" button on the Library page. Restarting the app
  no longer wipes the scan — the Library hydrates from disk before the
  first scan completes. (`crates/hoard-desktop/src/state.rs`,
  `crates/hoard-desktop/src/commands/library.rs`,
  `crates/hoard-desktop/ui/src/routes/Library.svelte`)
- **Periodic in-app update poller.** Beyond the boot probe, the desktop
  app now re-checks for client and server updates every 6 hours with
  exponential backoff on failure (24 h cap), so long-running sessions
  pick up releases shipped after launch. `App.svelte` consumes the
  result via `$derived($lastReport)`; the timer is cancelled on unmount.
  (`crates/hoard-desktop/ui/src/lib/stores/updates.ts`,
  `crates/hoard-desktop/ui/src/App.svelte`)
- **`just` task runner + pre-commit hook.** New `justfile` at the repo
  root with `dev / check / test / i18n-check / sqlx-prepare /
  install-hooks` recipes; `just install-hooks` points `core.hooksPath`
  at `.githooks/`. The hook itself is scope-aware — `cargo fmt --check
  && clippy` only when Rust files staged, `pnpm check` only when UI
  files staged, `node scripts/check-i18n.mjs` only when locale JSON
  changes — so doc-only commits don't pay the full price.
  (`justfile`, `.githooks/pre-commit`, `docs/dev.md`, `CONTRIBUTING.md`)
- **i18n parity linter.** `scripts/check-i18n.mjs` is pure-Node (no
  deps): for each locale it JSON-validates, diffs the key set against
  `en.json` (missing = error, extra = warning), and verifies
  `{placeholder}` parity with a depth-aware parser that understands ICU
  `plural` / `select` blocks (so branch literals like `{Zeile}` inside
  `{count, plural, one {…}}` don't get mistaken for variables). Wired
  into the pre-commit hook and the `clippy` CI job.
  (`scripts/check-i18n.mjs`, `.github/workflows/ci.yml`)
- **CI hardening.** New `sqlx-check` job runs `cargo sqlx prepare
  --workspace --check` so a missing offline cache fails the PR;
  `cargo-deny` job (Embark action v2) gates licenses / advisories /
  sources / bans; `cargo-machete` flags unused workspace deps. Build
  matrix split into Linux (full workspace + tests) and Windows / macOS
  (excludes `hoard-desktop` — the Tauri `generate_context!()` macro
  needs the frontend, which the release-desktop workflow already
  exercises). (`.github/workflows/ci.yml`, `deny.toml`)
- **Developer guide.** `docs/dev.md` enumerates every just recipe, the
  hook setup, UI and Rust conventions, and the SQLx offline-cache
  workflow.

### Fixed

- **Auto-backup no longer requires the game to be "running".** The fs
  watcher is now armed unconditionally in `handle_add` and survives the
  game starting/stopping. `process_poll` is kept for UI signalling
  (`GameStarted` / `GameStopped` → activity pills, "starting agent"
  state in Magic), but it no longer gates filesystem watching. Heavy
  `tracing::info!` was added at watcher-arm, fs-event, backup-schedule,
  and process transitions so the next silent-failure mode is caught
  immediately instead of two releases later.
  (`crates/hoard-agent/src/agent.rs`)
- **Dashboard pill on first render.** `pillFor()` now falls back to
  `dashboard.pill_saved` (`v{n} guardado`) when `$activity` is empty
  but `tracked.last_version_num > 0`, or to a new
  `dashboard.pill_no_backup` ("Sin copia aún") when there's genuinely
  no snapshot yet. The old behaviour wrongly reported "Inactivo" for
  every save until the agent emitted its first event.
  (`crates/hoard-desktop/ui/src/routes/Dashboard.svelte`,
  `crates/hoard-desktop/ui/src/lib/i18n/locales/*.json`)
- **Stellaris (and similar) now show the "pick a folder" alert.** The
  detector previously trusted the absolute `<winDocuments>\Paradox
  Interactive\Stellaris` path it derived from the manifest even when
  that folder didn't actually exist on the machine, so the user got a
  "Track" button that backed up an empty directory. Detection now
  verifies the candidate path exists and is non-empty before populating
  `found_paths`; otherwise the card falls back to the same amber
  no-save-folder alert other Steam-only matches get.
  (`crates/hoard-detect/src/filesystem.rs`,
  `crates/hoard-desktop/src/commands/library.rs`)
- **Sidebar nav labels re-translate on language change.** `App.svelte`
  was hard-coding the English strings on the `sidebarItems` array; they
  now go through `$_()` at render time via a `labelKey` indirection,
  so switching language in Settings updates the rail instantly.
  (`crates/hoard-desktop/ui/src/App.svelte`,
  `crates/hoard-desktop/ui/src/lib/i18n/locales/*.json`)

### Changed

- **Rename save label.** Saves grow a `PATCH /v1/saves/{id}` endpoint
  on the server and a "Renombrar" item on the History page header.
  Tracked local state migrates the label too; snapshot history is
  preserved untouched. Drag-along from the 0.2 known-limitations list.
  (`crates/hoard-server/src/routes/saves.rs`,
  `crates/hoard-agent/src/api.rs`,
  `crates/hoard-desktop/src/commands/library.rs`,
  `crates/hoard-desktop/ui/src/routes/History.svelte`)
- **i18n gap fill.** All eight locales gained the 11 keys behind the
  Library "no save folder" alert and the untrack confirmation modal
  (`library.no_save_alert_*`, `library.untrack_*`) plus the new
  `dashboard.pill_no_backup`. `settings.about_line_1` bumped to "Hoard
  1.4.0" across the board. Final linter state:
  `i18n ok — 8 locales, 287 keys`.
  (`crates/hoard-desktop/ui/src/lib/i18n/locales/*.json`)

## [1.3.5] — 2026-05-15

In-app updates land. The desktop app already knew when a newer release was
out, but the user still had to download the `.deb` and run `dpkg -i` by
hand. That ends here — the sidebar surfaces an amber alert button next to
the version when GitHub has something newer, clicking it opens a
confirmation modal, and "Yes" launches the OS installer. The server is
kept manual on purpose (it shouldn't self-restart while it might be
serving sync traffic) but gains a `hoard-server upgrade` subcommand so
the operator runs one command instead of editing systemd by hand.

### Added

- **In-app desktop updater.** The sidebar's update-available banner moves
  to a small amber alert button next to the version string (same visual
  vocabulary as the "Sin carpeta" alert). Clicking it pops a confirmation
  modal showing the current and target versions, with **Sí** (green) /
  **No** (red) buttons. Sí downloads the appropriate release asset for
  the host platform (`.deb` on Linux, `.msi` on Windows, `.dmg` on macOS)
  and hands it to the OS installer — `pkexec dpkg -i`, `msiexec /i`,
  `open` — so the user never opens a terminal. If launching the
  installer fails we still surface the downloaded path so they can run
  it manually.
  (`crates/hoard-desktop/src/commands/updates.rs`,
  `crates/hoard-desktop/ui/src/lib/components/UpdateConfirmModal.svelte`,
  `crates/hoard-desktop/ui/src/App.svelte`)
- **`hoard-server upgrade` subcommand.** Fetches the latest GitHub
  release, downloads the linux-x86_64 tarball, atomically swaps the
  `hoard-server` binary in place, and prints a hint to restart the
  systemd unit. Does not load config or touch the database, so a broken
  config still upgrades cleanly. Server self-restart is deliberately not
  attempted — distro init systems vary too much and an in-flight sync
  shouldn't get killed mid-upload by the upgrader.
  (`crates/hoard-server/src/upgrade.rs`,
  `crates/hoard-server/src/main.rs`)

### Changed

- **Update banner replaced by an icon button.** The previous full-width
  amber banner above the sidebar's Magic button is gone; its replacement
  is a 7×7 alert icon next to the Hoard version. Tighter, less noisy,
  and the click target is now the obvious one. The server-update path
  still doesn't auto-install — it copies `sudo hoard-server upgrade` to
  the clipboard so the user runs it on their server box.
  (`crates/hoard-desktop/ui/src/App.svelte`,
  `crates/hoard-desktop/ui/src/lib/i18n/locales/*.json`)

## [1.3.4] — 2026-05-14

Small UX gap on the Library page. When auto-detection finds a save folder
the "Track" button used to commit to that exact path with no way to
override — fine for Stardew, painful for Stellaris on Windows where the
detected `<winDocuments>\Paradox Interactive\Stellaris` may not be where
the user actually keeps their campaigns.

### Added

- **Pick a different folder when tracking.** A small folder icon sits
  next to the "Track" button on every detected game whose save path was
  found automatically. Clicking it pops the OS folder picker instead of
  auto-committing, so users can override the auto-detected path before
  Hoard starts watching. Same code path as the existing pick-from-alert
  flow; no surprise dialogs.
  (`crates/hoard-desktop/ui/src/routes/Library.svelte`,
  `crates/hoard-desktop/ui/src/lib/i18n/locales/*.json`)

## [1.3.3] — 2026-05-14

Brand refresh and a small but irritating tracking bug. The accent colour
moves from amber to a medium-dark emerald that contrasts better with the
dragon mascot; amber is kept exclusively for warnings (pause badge,
restore overwrite banner, near-quota meter, update-available nag, WARN
log lines).

### Fixed

- **Destracking and re-tracking the same game now works.** Stopping
  tracking only clears the local `CliState` row — by design, so server
  snapshots survive a fresh machine. But `list_tracked_saves` was
  returning every save the server knew about for the user, including
  destracked ones, so on the next app launch a ghost "Tracked" card came
  back. Worse, the Library detection card thought the game was still
  being watched and suppressed the amber "no save folder" alert, which
  is the entry point for re-picking the folder. The command now filters
  by local-state presence, so destracked games disappear cleanly.
  (`crates/hoard-desktop/src/commands/library.rs`)
- **Re-tracking after a destrack no longer fails with a 409.** The
  server enforces `UNIQUE(user_id, game_slug, label)`, so the second
  `create_save` returned a conflict the desktop surfaced as an opaque
  error. `add_game_to_tracking` now catches the conflict, finds the
  existing server save via `list_saves`, and re-links it locally —
  preserving the original snapshot history for the user.
  (`crates/hoard-desktop/src/commands/library.rs`)

### Changed

- **Accent colour amber → emerald.** `--color-accent` /
  `--color-accent-hover` now resolve to `emerald-600` / `emerald-500`;
  the `Button` primary variant, `Input` focus ring, `SettingsRow`
  toggle, wizard logo and progress dots, Library scan progress bar,
  History restore progress + checkboxes, Dashboard empty-state icon,
  sidebar logo + magic-setup button, and OnboardingDone admin badge all
  follow. Warning amber is preserved on update banners, WARN log lines,
  medium-confidence detection badges, paused-save badges, restore
  warnings, the near-quota meter, and the no-save alert chip.
  (`crates/hoard-desktop/ui/src/app.css`,
  `crates/hoard-desktop/ui/src/lib/components/*.svelte`,
  `crates/hoard-desktop/ui/src/routes/*.svelte`,
  `crates/hoard-desktop/ui/src/App.svelte`)
- **Spanish copy: pluralise "carpeta de partidas".** The UI used a mix
  of singular and plural ("carpeta de partida" vs. "carpeta de
  partidas") for the same concept; everything is now plural for
  consistency with the History page label. Also fixed *"Hoard no sabe
  dónde guarda partidas {name}"* → *"Hoard no sabe dónde guarda las
  partidas de {name}"* and *"monitorea"* → *"monitoriza"* in the
  magic-setup tooltip/subtitle.
  (`crates/hoard-desktop/ui/src/lib/i18n/locales/es.json`)

## [1.3.2] — 2026-05-14

UX cleanup around the Library page: stop ambushing the user with a folder
picker, and let them untrack a game without dropping to the CLI.

### Added

- **Untrack button on tracked-game cards.** Both the tracked-games strip
  at the top of the Library page and the green "Tracked" badge on
  detection cards now expose a trash icon. Click → confirmation modal
  ("Stop tracking {name}?") that makes clear snapshots on the server are
  preserved. The destructive action calls the existing `untrack_save`
  Tauri command, removes the entry from the local list, and toasts.
  (`crates/hoard-desktop/ui/src/routes/Library.svelte`,
  `crates/hoard-desktop/ui/src/lib/i18n/locales/*.json`)
- **Explicit "no save folder" alert.** Steam-only matches with no
  detected save folder used to silently pop the OS folder picker the
  moment the user clicked "Track" — disorienting if you didn't expect a
  native dialog. Those cards now show an amber `AlertTriangle` button
  instead. Clicking it opens a modal explaining *why* Hoard doesn't have
  a path yet (game never launched on this machine, or saves live outside
  the catalog) and surfaces the Steam install dir as a hint. The folder
  picker only opens when the user explicitly clicks "Choose save
  folder…" in the modal. (`crates/hoard-desktop/ui/src/routes/Library.svelte`)

### Changed

- **`track()` no longer auto-opens the folder picker.** With no
  `found_paths` candidate it now opens the alert modal instead. The
  picker is still reachable via the modal's primary button.
  (`crates/hoard-desktop/ui/src/routes/Library.svelte`)

## [1.3.1] — 2026-05-14

Hotfix for a path-detection bug that caused several Steam-installed games
(Cell to Singularity, Stellaris, …) to be backed up by their **install
directory** rather than their save directory — the user saw 600 MB
snapshots full of the game binaries.

### Fixed

- **Steam matches no longer leak the install directory into `found_paths`.**
  `detect_all` previously seeded the cross-reference map with
  `found_paths: vec![app.install_dir.clone()]`, so any catalog entry that
  matched a Steam appid carried the install dir at index 0. The UI's
  `track()` reads `found_paths[0]` as the local path to back up, which
  meant the snapshot consumed the entire game folder. Steam-only matches
  now leave `found_paths` empty (the UI falls back to the folder picker
  with `library.no_save_folder_yet`), and the install dir is preserved
  separately on a new `DetectedGame.install_dir` field for future UI hints.
  When the filesystem heuristic later fires for the same slug,
  `merge_fs_hit` populates `found_paths` from real save-path templates
  only. (`crates/hoard-agent/src/detection.rs`,
  `crates/hoard-desktop/ui/src/lib/api/index.ts`)

## [1.3.0] — 2026-05-09

Three small features that together make the desktop app feel less like a
toolbox and more like an appliance: the server self-heals when a client
knows about a game it doesn't, the app nags when a newer client or server
is available, and a one-click "magic" button does the whole detect →
track → start-agent dance for users who don't want to think about it.

### Added

- **Server self-heal of unknown games.** When the desktop client tries
  to track a game whose slug the server's catalog doesn't know yet (e.g.
  the server is on an older Ludusavi snapshot), the client now sends
  along the `display_name` and optional `steam_app_id` it already has.
  The server inserts a stub games row (`imported_from = 'client-supplied'`,
  `ON CONFLICT(slug) DO NOTHING`) and proceeds with the save. Old clients
  without these fields still get the original 422, so the change is
  backwards-compatible. (`crates/hoard-server/src/routes/saves.rs`,
  `crates/hoard-agent/src/api.rs`,
  `crates/hoard-desktop/src/commands/library.rs`,
  `crates/hoard-desktop/ui/src/routes/Library.svelte`)
- **Update checker for client and server.** A new `check_for_updates`
  Tauri command probes the GitHub releases API for the latest hoard tag
  and the configured server's `/v1/health` for its running version. Both
  probes run in parallel and tolerate `v` prefixes, prerelease suffixes,
  and double-digit components. The sidebar shows a small amber banner
  above the magic button when either side has an update available; the
  banner deep-links to `/settings`. Failures are silent — a network blip
  just leaves the banner hidden. (`crates/hoard-desktop/src/commands/updates.rs`,
  `crates/hoard-desktop/ui/src/lib/stores/updates.ts`,
  `crates/hoard-desktop/ui/src/App.svelte`)
- **Magic auto-setup button.** A new amber Sparkles button at the bottom
  of the sidebar runs `scan_library` → tracks every detection with
  `confidence === "high"` and at least one found path → boots the agent.
  Per-game errors are reported via toasts but don't abort the rest. The
  button shows phase-aware labels (`detecting`, `tracking 3/12`,
  `starting agent`) and is intentionally limited to high-confidence hits
  to avoid filling the server with false positives.
  (`crates/hoard-desktop/ui/src/lib/stores/magic.ts`,
  `crates/hoard-desktop/ui/src/App.svelte`)
- **Single-source version label in the sidebar.** Vite now injects
  `package.json`'s version into the bundle via `import.meta.env.VITE_HOARD_VERSION`,
  so the sidebar `v1.3.0` line stays in sync with the workspace version
  without a hand-maintained constant. (`crates/hoard-desktop/ui/vite.config.ts`,
  `crates/hoard-desktop/ui/src/vite-env.d.ts`,
  `crates/hoard-desktop/ui/src/App.svelte`)
- **i18n keys for the new surfaces.** Ten new keys (`magic.*` and
  `updates.*`) added to all eight locales (en, es, fr, de, it, pt, ja,
  zh).

## [1.2.2] — 2026-05-09

Hotfix #2 for v1.2.0: the v1.2.1 build no longer panicked, but instead
opened to a blank window (just the body background). Root cause:
`svelte-i18n`'s `init()` only *queues* the locale-dictionary load, so
the very first render reached `$_(...)` while no messages were loaded
yet, the formatter threw "Cannot format a message without first setting
the initial locale", and Svelte unwound the entire mount silently.
Fixed by awaiting `waitLocale()` before calling `mount()`.

### Fixed

- **App opened to a blank window** on every platform after v1.2.1. The
  body background colour was visible because `<body>` ships with a
  Tailwind class, but `#app` stayed empty. Mounting now waits for the
  active locale's dictionary to load. (`crates/hoard-desktop/ui/src/main.ts`,
  `crates/hoard-desktop/ui/src/lib/i18n/index.ts`)

## [1.2.1] — 2026-05-09

Hotfix for v1.2.0: the app crashed on launch with
`there is no reactor running, must be called from the context of a Tokio
1.x runtime`. The auto-update of the Ludusavi catalog was being spawned
with `tokio::spawn` from `setup()`, which runs before Tauri enters its
event loop and therefore has no ambient Tokio runtime. Switched to
`tauri::async_runtime::spawn` which is always available.

### Fixed

- **App crashed instantly on startup** on every platform (Linux/Windows/
  macOS). On Windows the process exited before the window appeared, with
  no console output. (`crates/hoard-desktop/src/commands/catalog.rs`)

## [1.2.0] — 2026-05-08

Desktop UX overhaul: friendlier path handling, restore-anywhere, and full
internationalisation.

### Added

- **Multi-language UI.** The desktop app now ships with translations for
  English, Spanish, French, German, Portuguese, Italian, Japanese, and
  Simplified Chinese. The language is auto-detected from the OS and can be
  changed at any time from **Settings → Language**.
- **Restore to any folder.** When restoring a snapshot for a save that
  isn't tracked on the current machine yet (e.g. you pulled it from
  another device), the app now opens a folder picker and remembers the
  choice — no more "Re-track from the Library" dead end.
- **Native folder pickers.** "Edit folder" on the History page and "Track
  this game" in the Library both grow a *Browse…* button that opens the
  OS folder dialog instead of forcing you to hand-type the path.

### Changed

- **Auto-create missing folders.** Specifying a save folder that doesn't
  exist yet (typed path, picker, or restore destination) now creates it
  for you instead of failing with *"doesn't exist on this machine — pick
  a different folder"*. Useful when restoring saves before installing
  the game.
- **Snapshot labels include a timestamp.** History rows now read
  `save_v3 · 2026-05-08 14:30` so the version line is self-describing
  and copy-pastable into bug reports.
- **Release pipeline.** Dropped the retired `macos-13` (Intel) runner
  from the desktop matrix and switched the publish gate to
  `success() || failure()`, so a stuck-in-queue runner can no longer
  block the rest of the platforms from publishing.

### Fixed

- Restore from the desktop UI now works end-to-end without falling back
  to the CLI — previously the download path resolved against
  `CliState` only, and any save without a local mapping erred out.
- `set_save_local_path` no longer rejects paths that haven't been
  created yet — it `mkdir -p`s them.

## [1.0.0] — 2026-05-08

First stable release. The desktop app, server, and CLI are now considered
stable; the HTTP API and on-disk schema will only change in
backwards-compatible ways within the 1.x line.

This release rolls up the v0.3 phase work (manifest catalog,
process-name detection, storage quota UI, packaging hardening) into a
finalised, signed-off product. From this point forward, official
Windows / Linux / macOS installers are published on every tag —
**users do not have to compile from source**.

### Added

- **Pre-built installers** for every platform on every tagged release
  (see the [Releases page](https://github.com/rleeon/hoard/releases/latest)):
  - **Windows**: NSIS `.exe` setup + `.msi` (MSI installer). Per-user
    install — no admin privileges required.
  - **Linux**: `.deb`, `.rpm`, and AppImage.
  - **macOS**: `.dmg` for both Intel and Apple Silicon.
  - Server tarball: `hoard-1.0.0-linux-x86_64.tar.gz` with the
    headless `hoard-server`, `hoard-admin`, and `hoard` CLI binaries.
  - SHA256 checksums alongside every artifact.
- **Game-detection upgrades** (rolled up from v0.3 phases 1–4b):
  - New `hoard-manifest` crate parses the
    [Ludusavi manifest](https://github.com/mtkennerly/ludusavi-manifest)
    YAML so the catalog covers thousands of titles instead of the
    seeded 10.
  - New `hoard-detect` crate combines filesystem heuristics, Steam
    library parsing (`libraryfolders.vdf` + `appmanifest_*.acf`), and
    process-name matching to identify which save folders belong to
    which game.
  - New `hoard-watcher` crate exposes the live filesystem +
    process watchers as a reusable library so both the desktop agent
    and any future headless daemon share the exact same change-
    detection logic.
  - Lazy `notify` watcher registration: the agent only opens an
    inotify/FSEvents handle when the user actually starts tracking a
    save, dramatically lowering the FD footprint on machines with
    hundreds of detected games.
- **Storage quota UI** (v0.3 phase 4a–5):
  - `whoami` now returns `storage_used_bytes` and
    `storage_quota_bytes`; the desktop app surfaces this as a
    quota bar on the Dashboard.
  - Per-game disk-usage breakdown on the Library page.
- **NSIS per-user install** (v0.3 phase 6): the Windows installer now
  defaults to `currentUser` install mode and a single-language
  English UI, removing the elevation prompt and the language picker
  on first run.

### Changed

- Workspace version bumped to `1.0.0` across every crate.
- README and `docs/install-client.md` updated to point users at the
  pre-built installers as the recommended install path; building
  from source is now an "advanced" option.
- Release CI made portable across runners: macOS bundles now hash
  with `shasum -a 256` (GNU `sha256sum` is not available on macOS
  runners), Linux still uses `sha256sum`. Outputs are byte-identical.
- CI installs `libdbus-1-dev` on the slim server-release runner so
  the CLI's `keyring` dependency builds even outside the
  desktop-runner's GTK stack.

### Fixed

- Tauri icon decoding: regenerated `icon.ico` and the seeded PNGs as
  8-bit RGBA (PNG color_type=6) so Tauri's image pipeline accepts
  them on every platform.
- Release-desktop workflow: Tauri-action's `beforeBuildCommand`
  resolution now finds a `package.json` at the repo root via a thin
  shim, fixing first-tag builds on a fresh checkout.
- `whoami` SQLx offline cache refreshed for the new quota query so
  CI no longer fails with `SQLX_OFFLINE` set.

### Stability commitment

From 1.0.0 onward:

- The HTTP API will only change in backwards-compatible ways within
  the 1.x series. Breaking changes go in 2.0 with a migration note.
- The on-disk snapshot layout (server-side `data/` and `trash/`
  trees) is stable. Old snapshots remain restorable across upgrades.
- The CLI flag surface is stable. New flags may appear; existing
  flags will not be removed without a deprecation cycle.

### Known limitations

- No code-signing on Windows or macOS yet — first-run shows the OS
  "unverified developer" warning. Documented workaround in
  [`docs/install-client.md`](docs/install-client.md#install).
- No auto-updater — install new versions over the top from the
  Releases page. See ADR 0007. Auto-update is on the 1.x roadmap
  once we have signing certificates.

## [0.2.0] — 2026-05-04

The desktop app release. v0.2 ships a Tauri + Svelte client for Linux,
Windows, and macOS that auto-detects installed games, watches their
save folders, uploads versioned snapshots in the background, and lets
you restore previous versions from a friendly UI. The server protocol
is unchanged from 0.1.x.

### Added

- **Desktop app** (`hoard-desktop`, Tauri 2 + Svelte 5):
  - **Onboarding wizard**: server URL probe (`/health`), token paste,
    automatic library scan on completion. Tokens stored in the OS
    secret store (Secret Service / DPAPI / Keychain).
  - **Library / detection**: filesystem heuristics + Steam library
    parsing identify candidate save folders for the seeded catalog.
    Tracking a save persists locally and creates the server-side
    `(game_slug, label)` namespace.
  - **Live agent**: filesystem watcher (`notify` + debouncer)
    triggers a backup on settled changes; process watcher
    (`sysinfo`) flushes immediately when the game stops. Both are
    debounced to avoid hammering the server.
  - **Dashboard**: per-save status pills (idle / scheduled /
    uploading / saved / failed-retrying / paused), a "Back up now"
    override, and a quick link to per-save history.
  - **History page**: snapshot list with file inventory and total
    size. **Restore** flow includes an optional pre-backup safety
    snapshot (default ON) and shows determinate progress for both
    phases. Soft-deleted snapshots are recoverable from the same
    page during the retention window.
  - **Manual controls**: pause/resume tracking per save, edit the
    local save folder path, force-backup, untrack.
  - **Logs viewer**: tail of the rolling daily file appender
    (`agent.log.YYYY-MM-DD`) with level filter and copy-all.
  - **Tray icon** (Linux/Windows/macOS): live state, "Backup all
    now", "Pause all", "Open dashboard", "Quit".
  - **Notifications**: per-event desktop notifications, individually
    toggle-able (success on, failure on, by default).
  - **Settings**: close-to-tray, autostart at login, start
    minimised, success/failure notifications, anonymous telemetry
    (off by default — see `docs/privacy.md`).
- **Packaging**: bundle targets for `.deb`, `.rpm`, `.AppImage`,
  `.nsis` setup `.exe`, `.msi`, and `.dmg` (Intel + Apple Silicon).
  New `release-desktop.yml` workflow runs on every `v*.*.*` tag.
- **Docs**: `docs/install-client.md` (per-platform install,
  uninstall, troubleshooting) and `docs/privacy.md` (every network
  call the desktop app makes, what it stores locally, and the
  opt-in telemetry contract).

### Changed

- Tracing setup now layers a daily-rotating file appender alongside
  stdout — required for the in-app Logs viewer. Affects only the
  desktop binary; the server logs identically to 0.1.x.

### Known limitations

- No code-signing on Windows or macOS yet — first-run shows the OS
  "unverified developer" warning. Documented workaround in
  `docs/install-client.md`.
- No auto-updater; install new versions over the top from the
  GitHub Releases page. See ADR 0007 for the rationale and the
  rollout plan in v0.3.
- Catalog is still the same 10 seeded games as 0.1.0. Multi-instance
  detection (e.g. two copies of Stardew Valley with different mods)
  is supported via labels but the UI doesn't surface a "rename
  label" action yet.

## [0.1.0] — 2026-05-03

First public release. Functionally complete end-to-end backup + restore
flow with versioned snapshots, soft delete, and per-user quotas. **API and
on-disk schema may still change in 0.x; expect to wipe and recreate at
least once before 1.0.**

### Added

- **Server** (`hoard-server`): Axum HTTP server backed by SQLite (WAL,
  `synchronous=NORMAL`, `foreign_keys=ON`) with embedded migrations.
- **Auth**: opaque bearer tokens (`hoard_v1_<64 hex>`), SHA256-hashed in
  the DB. `last_used_at` updated in the background. Argon2id passwords.
- **Games catalog**: 10 seeded games with kebab-case slugs, search by
  substring of slug or display name.
- **Saves**: per-user namespaces scoped to `(game_slug, label)` with
  per-save snapshot count, latest version, and total size.
- **Snapshots**: streaming multipart upload with per-file SHA256, atomic
  commit (`fs::rename` from `tmp/` into `data/` inside a SQLite
  transaction). Streaming `tar.zst` download built on the fly.
  Path traversal hardened. Soft delete moves directories to `trash/`;
  `restore` moves them back. Periodic cleanup task purges old `tmp/` and
  expired trash.
- **Quotas**: per-user `storage_quota_bytes` (default 100 GiB),
  enforced at upload time.
- **Audit log**: every snapshot create/delete/restore writes a row.
- **Admin CLI** (`hoard-admin`): `db {status,migrate,vacuum}`,
  `user {create,list,delete}` (Argon2id, optional TTY prompt),
  `token {create,list,revoke}`, `game {add,list,remove}`.
- **Client CLI** (`hoard`): `config`, `login/logout/whoami`, `status`,
  `games {search,show}`, `save {create,list,show,delete}`,
  `snapshots {list,delete,undelete}`, `backup` (with progress bar +
  `--remember`), `restore` (streaming zstd decode + tar extract + per-file
  SHA256 verification).
- **Packaging**: hardened systemd unit, idempotent `install.sh` /
  `uninstall.sh [--purge]`, multi-stage Dockerfile, docker-compose with
  named volume + `/v1/health` healthcheck.
- **CI**: `cargo fmt --check`, `cargo clippy -- -D warnings`,
  `cargo build --workspace`, `cargo test --workspace`.
- **Release workflow**: tag-driven release builds Linux x86_64 binaries
  and attaches them + checksums to a GitHub Release.

### Known limitations

- No Windows binaries yet (cross-compilation target wired but not
  CI-tested).
- No web UI.
- No multi-tenant / public registration flow — bring your own admin and
  hand out tokens.
- No rate limiting; put a reverse proxy in front for that.
- Single SQLite database; no replication. Back up the file.

[Unreleased]: https://github.com/rleeon/hoard/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/rleeon/hoard/releases/tag/v1.0.0
[0.2.0]: https://github.com/rleeon/hoard/releases/tag/v0.2.0
[0.1.0]: https://github.com/rleeon/hoard/releases/tag/v0.1.0
