# Contributing to Hoard

Thanks for your interest in contributing.

## AI tools

AI coding assistants (Cursor, Copilot, Claude, etc.) are welcome. You can use them to draft code and even write commit messages — just make sure the `Co-authored-by` trailer is never added for an AI tool. The human who reviews and pushes the patch is the sole author.

- **No co-author trailers.** Commits must not carry `Co-authored-by` lines for AI tools.
- **You are responsible.** Understand what the AI produced, test it, and ensure it follows the project's style before submitting.
- **No bulk AI-generated PRs.** Submissions that are clearly machine-generated without human review will be closed.
- **Use the AGENTS.md.** This file has our own rules to guide LLMs with project context and how to work with us.

## Getting started

1. Fork the repo and clone your fork.
2. Install Rust and the [prerequisites](#prerequisites).
3. Create a branch from `main`.

## Building

```sh
cargo build --workspace
```

The UI lives in `crates/hoard-desktop/ui/`:

```sh
pnpm --dir crates/hoard-desktop/ui install
pnpm --dir crates/hoard-desktop/ui build
```

## Before submitting a PR

- [ ] `cargo fmt --all`
- [ ] `cargo clippy --workspace -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] `pnpm --dir crates/hoard-desktop/ui check` (if you touched UI code)
- [ ] No AI-tool co-author trailers
- [ ] If you added/changed a `query!` or `query_as!`, regenerate and commit `.sqlx/`
- [ ] If you added a new SQL migration, do **not** modify any existing released migration
- [ ] Update `CHANGELOG.md` under `[Unreleased]` if the change is user-visible
- [ ] Update docs if you changed user-facing behavior

## Architecture

- **`hoard-agent`** — the sync engine, detection, backup/restore. Business logic lives here.
- **`hoardd`** — local service, owns the engine, serves IPC.
- **`hoard-cli`** / **`hoard-desktop`** — thin clients that talk to `hoardd`.
- **`hoard-server`** — HTTP server, owns the database.
- **`hoard-core`** — shared types and the sans-IO kernel.

If you find yourself copying an `if` between `hoard-cli` and `hoard-desktop`, it belongs in `hoard-agent`.

## Code style

- All prose in English (comments, docs, commit messages).
- Imperative commit subjects, lowercase, no conventional-commit prefixes.

## License

By contributing, you agree your contributions are licensed under AGPL-3.0.
