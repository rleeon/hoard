# Hoard — developer guide

Practical notes for working on the codebase day-to-day. Setup instructions
already in [CONTRIBUTING.md](../CONTRIBUTING.md) are not repeated here.

## Task runner: `just`

The repo ships a [`justfile`](../justfile). Install once
(`cargo install just`) and you get:

| Recipe              | What it does                                                    |
| ------------------- | --------------------------------------------------------------- |
| `just dev`          | Tauri dev mode for the desktop app (Vite HMR).                  |
| `just ui`           | Just the UI dev server — no Tauri shell.                        |
| `just build`        | Production desktop bundle for the host platform.                |
| `just check`        | The full pre-commit gate (fmt-check + clippy + svelte-check).   |
| `just fmt`          | `cargo fmt --all`.                                              |
| `just fmt-check`    | Format check without modifying files.                           |
| `just clippy`       | `cargo clippy --workspace -- -D warnings`.                      |
| `just ui-check`     | `pnpm check` against `crates/hoard-desktop/ui`.                 |
| `just test`         | `cargo test --workspace` with `SQLX_OFFLINE=true`.              |
| `just compile`      | `cargo check --workspace` — fast smoke test after a refactor.   |
| `just i18n-check`   | Run `scripts/check-i18n.mjs` (locale parity + JSON parse).      |
| `just sqlx-prepare` | Regenerate `.sqlx/` (needs a live DB URL).                      |
| `just install-hooks`| Wire `core.hooksPath` to `.githooks/`.                          |
| `just clean`        | `cargo clean` + drop the Vite dist + node_modules cache.        |

Recipes are kept in sync with CI — if a check ships in CI it ships in
`just check` too.

## Git hooks

A pre-commit hook lives in [`.githooks/pre-commit`](../.githooks/pre-commit).
Enable it once after clone:

```sh
just install-hooks
```

(That just sets `git config core.hooksPath .githooks`.)

What runs: only the checks relevant to the staged tree. A doc-only commit
skips `clippy` and `svelte-check`; an i18n-only commit triggers only the
locale lint. Use `git commit --no-verify` if you genuinely need to bypass
the gate — CI still enforces everything either way.

## Editing the UI

- Svelte 5 runes everywhere (`$state`, `$derived`, `$props`, `{#snippet}`).
  Do not write legacy reactive `$:` blocks — `pnpm check` will not catch
  them, but they fight runes mode at runtime.
- Tailwind v4 with the emerald primary palette. Amber is reserved for
  pause / warning / update-available states; never use it for primary
  actions.
- Modal + Toaster live in `crates/hoard-desktop/ui/src/lib/components/`.
  Don't roll new modal markup — reach for `Modal.svelte`.
- Strings go through `svelte-i18n`: `import { _ } from "svelte-i18n"` then
  `$_("key.path", { values: { name } })`. Every key must exist in all
  eight locales under `crates/hoard-desktop/ui/src/lib/i18n/locales/`.
  `en.json` is the source of truth; `es.json` is the most visible (native
  user). `just i18n-check` enforces parity.

## Editing the Rust side

- Workspace deps in the top-level `Cargo.toml` under
  `[workspace.dependencies]`. Member crates use `{ workspace = true }`.
- `#[tauri::command]` functions live under
  `crates/hoard-desktop/src/commands/`. Each one must be listed in the
  `invoke_handler!` block in `crates/hoard-desktop/src/lib.rs`, otherwise
  the JS side gets a `"command not found"` runtime error.
- Use `tokio::process::Command`, not `std::process::Command`, from any
  async context.
- Errors crossing the API surface use `anyhow` / the typed `ApiError`;
  library code (`hoard-core`) prefers `thiserror`.

## SQLx offline cache

Any change to a `sqlx::query!` macro requires regenerating `.sqlx/`:

```sh
just sqlx-prepare sqlite:///tmp/hoard-data/hoard.db
git add .sqlx
```

CI runs offline (`SQLX_OFFLINE=true`); a stale cache shows up as
`set DATABASE_URL to use query macros online`.

## Useful workspace facts

- Current version is in `[workspace.package].version` at the repo root.
  The Rust crates inherit via `version.workspace = true`. The desktop UI
  carries its own copy in `crates/hoard-desktop/tauri.conf.json` and
  `crates/hoard-desktop/ui/package.json`, plus a fallback string in
  `App.svelte`. All four must agree on a release.
- `cargo check` puts ~10 GB in `target/`. `just clean` wipes it.
- The repository plan and roadmap for upcoming releases live in
  `version1-5.md` (now `docs/plans/1.5.md`) — read before starting any
  non-trivial feature.
