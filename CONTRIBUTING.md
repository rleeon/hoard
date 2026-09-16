# Contributing to Hoard

Thanks for your interest in contributing.

## AI tools

AI assistance is allowed, but you are the author. Human-written sections must be yours and verified. Raw AI output is allowed only in the section named "AI Zone".

- **You vouch for every line outside AI Zone.** Understand it, test it, cut what you cannot defend.
- **No AI dumps outside AI Zone.** Huge, vague, or clearly unedited AI texts outside AI Zone will be closed without review.
- **AI Zone is unverified context.** It may help, but it is never treated as fact.
- **Issues:** one problem, with steps to reproduce and what you actually saw. No walls of guesses.
- **PRs:** only what you verified, including `CHANGELOG.md` entries. No co-author trailers for AI tools.
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
