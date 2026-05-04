# Hacker News launch post

**Title:** Show HN: Hoard – self-hosted Steam Cloud, but for any game

---

Hi HN,

Hoard is a small Rust HTTP server (Axum + SQLite) and a Tauri desktop
client that together replicate "Steam Cloud, but on a box you own and
for any game, modded or not."

The server is one binary + a config file + a directory. It accepts
streaming `tar.zst` uploads, stores them as versioned snapshots,
enforces per-user quotas, soft-deletes to a `trash/` dir for
recovery, and exposes a small JSON API behind opaque bearer tokens.

The desktop app (v0.2, today) auto-detects installed games on
Linux/Windows/macOS, watches their save folders with
`notify`+debouncer, snapshots them when the game exits or the
filesystem settles, and gives you a per-save history page with
restore + soft-delete + an optional pre-restore safety backup so
"undo restore" is one click.

A few things I think are interesting:

- **Atomic snapshots.** Every commit is `fs::rename` from `tmp/` to
  `data/<save>/<version>/` *inside* a SQLite transaction. Either you
  get a complete snapshot in the listing or you get nothing — there's
  no "half-restored" failure mode.
- **Restore is verifiable.** Every file in a snapshot has its SHA256
  recorded at upload; the client verifies on extract. Bit-rot or
  partial download is detected, not silently applied.
- **No global service.** The desktop app talks to *your* server and
  nothing else. There's no Hoard cloud, no analytics provider, no
  CDN. The privacy doc spells out every endpoint the client touches,
  so you can audit it in five minutes.
- **SQLite, deliberately.** The "deploy on a Pi in a closet" target is
  the design constraint — single-file backups, no Postgres operator,
  WAL + `synchronous=NORMAL` for durability that survives a yanked
  power cable. ADR 0001 explains the call.

What's not in v0.2 (and why):

- **No code signing on Windows/macOS yet.** Cert procurement is a
  paperwork problem, not an engineering one. ADR 0007 is the explicit
  decision to ship unsigned bundles with documented warnings rather
  than gate on signing.
- **No auto-updater.** Same story — an unsigned auto-updater is
  worse than a manual download because every "new version" dialog
  becomes a phishing surface. Coming with signing in v0.3.
- **No web UI.** CLI + desktop app cover both ends; a web UI would
  duplicate either or both.

Stack: Rust (Axum, SQLx, Tokio), Tauri 2 + Svelte 5 (runes), Tailwind
v4, SQLite. AGPL-3.0.

Repo: <https://github.com/hoarddev/hoard>
Release: <https://github.com/hoarddev/hoard/releases/tag/v0.2.0>

Happy to dig into anything — the snapshot lifecycle, the
process-watching heuristics, the soft-delete mechanics, why I picked
Tauri over a web UI, the threat model. Cheers.
