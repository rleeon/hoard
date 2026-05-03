# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

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

[Unreleased]: https://github.com/rleeon/hoard/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/rleeon/hoard/releases/tag/v0.1.0
