# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

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

[Unreleased]: https://github.com/rleeon/hoard/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/rleeon/hoard/releases/tag/v0.2.0
[0.1.0]: https://github.com/rleeon/hoard/releases/tag/v0.1.0
