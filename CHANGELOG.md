# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

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
