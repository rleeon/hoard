# AGENTS.md

Rules for AI assistants working on this repo. Read this file before making any change.

## What this is

Hoard is a self-hosted game save backup and sync system. Rust + Tauri 2 + Svelte 5, AGPL-3.0.

The sync engine lives in a local service, **`hoardd`**, one per user, persistent, that survives closing the app. The desktop app and CLI are thin clients that talk to it over a socket.

```
crates/
├── hoard-core       sans-IO kernel: newtypes, IPC types, reducer
├── hoard-manifest   Ludusavi manifest parser
├── hoard-agent      engine: backup, restore, detection, talks to the server
├── hoardd           local service: sole owner of the engine, serves IPC
├── hoard-cli        `hoard` binary (thin client)
├── hoard-admin      server-side admin CLI
├── hoard-server     HTTP server (Axum), owns the database
├── hoard-screen     in-game overlay (Pro feature, desktop sidecar)
└── hoard-desktop    Tauri shell + Svelte UI (thin client)
    └── ui/          Svelte 5 + Tailwind v4 + Vite
```

## Hard rules

**Logic lives in `hoard-agent`.** Desktop and CLI are two frontends of the same engine. If you find yourself copying a business `if` between `hoard-desktop/src/commands/` and `hoard-cli/src/commands/`, that `if` is misplaced: move it to the agent. `hoard-agent` must not depend on Tauri or anything graphical.

**All prose in English.** Comments, doc comments, commit subjects and bodies, public docs. No retroactive mass translation: translate a file only when you are already editing it for another reason, and include the translation in the same commit. Never a translate-only commit — it breaks `git blame` and buys nothing.

**Commits: name only.** No `Co-Authored-By`, no mention of AI tools anywhere. Short subject, imperative, lowercase, English, no conventional-commit prefixes.

**User-provided text goes as-is.** If you want to suggest different wording, ask first; do not rewrite on your own.

**Ask before doing anything.** You may read and run tests locally, but you need permission before writing code.

## Tech stack

- **Language:** Rust (edition 2021)
- **Desktop:** Tauri 2 + Svelte 5 + Tailwind v4 + Vite
- **Server:** Axum + SQLx + SQLite
- **Compression:** zstd
- **Hashing:** SHA-256, Blake3
- **Auth:** Argon2
- **CLI:** clap
- **Async runtime:** tokio

## Testing

```sh
cargo test --workspace
pnpm --dir crates/hoard-desktop/ui test
```

The integration test suite lives in `crates/hoard-pruebas/`, outside git and outside the workspace. It is not needed for a clean clone.

## Verify before committing

```sh
cargo check --workspace
pnpm --dir crates/hoard-desktop/ui check
```

Zero errors and zero warnings on both. `cargo test` only if you touch code with tests. `pnpm build` only if you need to verify the bundle itself — `pnpm check` is enough for type errors.

## What never gets pushed

Everything already listed in `.gitignore`.

## Where things live

| You need | It's in |
|---|---|
| Engine, detection, backup/restore | `crates/hoard-agent/CLAUDE.md` |
| Server, auth, storage, migrations | `crates/hoard-server/CLAUDE.md` |
| Service, IPC, notifications, keystore | `crates/hoardd/CLAUDE.md` |
| Sans-IO kernel | `crates/hoard-core/CLAUDE.md` |
| Tauri plumbing, updater, sidecars | `crates/hoard-desktop/CLAUDE.md` |
| Svelte, Tailwind, i18n, modals, toasts | `crates/hoard-desktop/ui/CLAUDE.md` |
| Overlay Pro | `crates/hoard-screen/CLAUDE.md` |
| Public website | `web/CLAUDE.md` |
