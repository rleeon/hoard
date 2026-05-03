# Contributing to Hoard

Thanks for the interest. This is a small project — issues and PRs are
welcome, but please read this first so we can make the most of each
other's time.

## Before opening an issue

- Search existing issues — there's a decent chance it's already filed.
- Include the exact `hoard --version` (or commit hash), OS, and the
  command you ran.
- Server-side issues: include relevant lines from `journalctl -u
  hoard-server` (or `docker compose logs server`) with timestamps.
- Reproduction steps trump everything else.

## Before opening a PR

- Open an issue first for non-trivial changes so we can agree on the
  shape before you write code.
- Run `cargo fmt --all`, `cargo clippy --workspace -- -D warnings`,
  `cargo test --workspace` — CI will block on all of these.
- One logical change per PR. Refactors and feature work go in separate
  PRs.
- New SQL: add a migration under `crates/hoard-server/migrations/`.
  **Never edit a previously-released migration.** Re-run
  `cargo sqlx prepare --workspace` and commit the updated `.sqlx/` so CI
  builds offline.
- Architectural changes: write an ADR in `docs/decisions/` (see
  existing ones for the format).

## Local development setup

```sh
# 1. Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
rustup component add clippy rustfmt

# 2. System deps (Debian/Ubuntu)
sudo apt install -y libssl-dev pkg-config

# 3. Build
SQLX_OFFLINE=true cargo build --workspace

# 4. Run a dev server
mkdir -p /tmp/hoard-data
cp deploy/config.toml.example /tmp/test-config.toml
$EDITOR /tmp/test-config.toml   # set data_dir to /tmp/hoard-data, port to 18082
cargo run -p hoard-server -- --config /tmp/test-config.toml

# 5. In another shell, drive it via the admin CLI + client CLI
cargo run -p hoard-admin -- --config /tmp/test-config.toml db migrate
cargo run -p hoard-admin -- --config /tmp/test-config.toml \
    user create alice --password 'dev'
cargo run -p hoard-admin -- --config /tmp/test-config.toml \
    token create alice --device dev
```

## Updating the SQLx offline cache

Any change to `sqlx::query!` / `sqlx::query_as!` macros requires
regenerating `.sqlx/`:

```sh
DATABASE_URL=sqlite:///tmp/hoard-data/hoard.db cargo sqlx prepare --workspace
git add .sqlx
```

Without a fresh cache, CI will fail with `set DATABASE_URL to use query
macros online`.

## Code style

- Errors at the API surface use `anyhow` or the typed `HoardError`.
  Library code (`hoard-core`) prefers `thiserror`.
- Public types in `hoard-core` are the contract between server and CLI;
  changing them is a breaking change.
- Logs: use `tracing` macros, not `println!`. Spans for request
  lifecycle, fields for IDs.
- SQL: prefer compile-checked `query!`/`query_as!` over runtime
  `query()`. Aggregate columns need explicit type hints
  (`as "name: i64"`) and `.unwrap_or(0)` since SQLite returns `NULL` for
  empty groups.

## License

By contributing you agree your changes will be released under the
project's [AGPL-3.0](LICENSE).
