# r/selfhosted launch post

**Title:** Hoard v0.2 — self-hosted Steam Cloud, but for any game (Linux/Win/macOS desktop app)

---

Hi r/selfhosted,

I built Hoard because I bounce between three machines and Steam Cloud
covers maybe 60% of my library. Everything off-Steam (GOG, itch.io,
emulators, modded everything) was either "manually rsync this folder
before bed" or just lost. I wanted Steam Cloud's UX with my own server
holding the data.

**Hoard is:**

- A small Rust HTTP server (Axum + SQLite) you point at a directory.
- A Tauri desktop client that auto-detects installed games, watches
  their save folders, and uploads versioned `tar.zst` snapshots
  whenever you stop playing.
- A CLI for the headless / NAS / "I'll script it" crowd.

**v0.2 is the desktop release.** v0.1 was server + CLI; v0.2 adds the
Tauri app I've been daily-driving for a few months.

**What you get:**

- One-click install for Linux (`.deb`, `.rpm`, `.AppImage`), Windows
  (`.exe` / `.msi`), macOS (Apple Silicon + Intel `.dmg`).
- Auto-detection for the seeded catalog (filesystem heuristics +
  Steam library parsing).
- Per-save status pills, per-game history, restore-with-undo
  (optional pre-restore safety snapshot).
- Tray icon that survives close-to-tray, autostart-at-login, the
  whole quiet-background-app dance.
- Soft-deleted snapshots are recoverable for the configurable
  retention window — no oh-no-I-clicked-delete moments.

**What you provide:**

- A box to run the server on. There's a hardened systemd unit, an
  `install.sh`, and a `docker-compose.yml`. SQLite + WAL handles a
  household just fine; for a friend group, throw a reverse proxy in
  front and you're done.
- TLS (use Caddy or your favourite). Hoard speaks plain HTTP and
  bearer tokens; TLS is your reverse proxy's job.

**What's *not* in v0.2:**

- Code-signing on Windows/macOS — first run shows the "unverified
  developer" warning. Workaround in the install docs. v0.3 fixes this.
- Auto-updater. Drop the new installer over the old one; local state
  survives.
- A web UI. CLI + desktop app cover both ends; a web UI is on the
  roadmap for v0.4.

**No telemetry by default.** The desktop app talks to *your* server
and nothing else; the privacy doc lists every endpoint and there's an
opt-in (off by default) for anonymous event counters once that lands.

Repo + docs: <https://github.com/hoarddev/hoard>
Releases: <https://github.com/hoarddev/hoard/releases/tag/v0.2.0>
`docs/install.md` for the server, `docs/install-client.md` for the
desktop app.

Happy to answer questions about the architecture, the threat model, or
why I picked SQLite over Postgres (short version: this needs to run on
a Pi).
