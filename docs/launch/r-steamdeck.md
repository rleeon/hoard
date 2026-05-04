# r/SteamDeck launch post

**Title:** Hoard — sync Deck saves to your home server, even for non-Steam games (open source, Linux native)

---

Steam Cloud is great when it works, but if you've ever:

- bounced between the Deck and a desktop, then watched a save go
  backwards because Steam picked the wrong copy,
- played a non-Steam game (emulators, GOG, itch.io ports, half the
  ROM-friendly Decky stuff) and just... lost progress when the SD card
  reseated,
- modded a game in a way that Steam Cloud refuses to sync,

…then Hoard is for you.

**What it is:** a tiny self-hosted save-sync server + a Linux-native
desktop app that auto-detects games, watches their save folders, and
uploads versioned snapshots to *your* server whenever you stop playing.
Your saves live on a box you control — a Pi, a NAS, an old laptop in a
closet, whatever you've got.

**Why it works well on Deck:**

- Native Linux build (`.deb`, `.rpm`, `.AppImage`). Runs in Desktop
  mode; the agent stays in the tray.
- Auto-detect picks up Proton prefixes (`steamapps/compatdata/<id>/...`)
  and known emulator save dirs (RetroArch, Dolphin, PCSX2, Yuzu before
  it was Yuzu).
- Backup happens *after* the game stops, so it doesn't compete with
  whatever's running. Debounced to ride out crash recoveries.
- Restore comes with an optional pre-restore safety snapshot — you can
  always undo a restore if it wasn't the version you wanted.

**What's in v0.2 (today):**

- Onboarding wizard, library auto-detection, dashboard with live
  status pills, per-save history with file inventory + restore.
- Tray icon, close-to-tray, autostart, desktop notifications.
- Soft-deleted snapshots recoverable until the retention window
  passes.

**What you'll need:**

- Somewhere to run the server (Docker compose example included; SQLite
  inside, no external DB).
- Five minutes to install the desktop app and paste a token.

**Things to know:**

- The desktop app isn't code-signed yet, so first run on Windows/macOS
  pops a warning. Linux users are fine. v0.3 fixes signing.
- The seeded game catalog is small (~10 games right now). If your
  game isn't there, you can still track its save folder manually —
  the auto-detect just doesn't pre-fill the path for you.

Repo: <https://github.com/hoarddev/hoard>
Releases: <https://github.com/hoarddev/hoard/releases/tag/v0.2.0>

This is open source (AGPL), self-hosted, no SaaS, no account, no
telemetry by default. AMA.
