# r/linux_gaming launch post

**Title:** Hoard v0.2 — open-source save backup that handles Proton prefixes, emulators, and Wine bottles (Linux-native client)

---

I got tired of writing one-off rsync scripts for every new game. Steam
Cloud only covers Steam-distributed titles and even then it's flaky
across multiple machines. So I built Hoard.

**TL;DR:** a small self-hosted server + a Tauri desktop client that
detects installed games (including Proton prefixes), watches their
save folders, and uploads versioned snapshots when you stop playing.
You restore from a friendly UI with an optional safety backup, so
"oops, wrong version" is reversible.

**Why this might be your jam:**

- Native Linux. The desktop app ships as `.deb`, `.rpm`, and
  `.AppImage` — no Electron-with-extra-steps. Built in Rust + Svelte,
  ~30MB installed.
- Handles Proton's `steamapps/compatdata/<appid>/pfx/...` layout, plus
  the obvious Linux-native locations under `~/.local/share`,
  `~/.config`, `~/.var/app/...` for Flatpaks.
- Filesystem watcher debounces so it doesn't fight the game for I/O,
  and there's a process watcher that flushes a final backup the
  moment the game exits.
- The actual transport is a streaming `tar.zst` upload with per-file
  SHA256 verification on restore. No deduplication yet — saves are
  small enough that simple full snapshots are fine for now.

**What v0.2 covers:**

- Onboarding wizard (server URL → token → done).
- Auto-detection for the seeded catalog. Manual tracking works for
  anything else.
- Dashboard with live status (idle / scheduled / uploading /
  saved / failed-retrying / paused).
- Per-save history page: snapshot list, file inventory, restore with
  optional pre-restore safety backup, soft-delete with recoverability.
- Tray icon, autostart, close-to-tray, desktop notifications.
- In-app log viewer (level filter + copy-all) for bug reports.

**What it's not:**

- Not a syncthing replacement. The model is "snapshot at quiet times",
  not "every byte mirrored". This is on purpose — saves are small,
  versions matter, and you want explicit checkpoints.
- Not yet code-signed on Windows/macOS. Linux is fine.
- No deduplication or rolling-window prune in v0.2 — server-side
  retention is per-save count + days. Fine for households.

**Self-hosted means self-hosted.** The desktop app's only network
traffic is to your server. No phone-home, no analytics. There's an
opt-in (off by default) anonymous-event-counter toggle for future
versions; the privacy doc spells out exactly what would and wouldn't
be sent if you turned it on.

Code: <https://github.com/hoarddev/hoard> (AGPL-3.0)
Releases: <https://github.com/hoarddev/hoard/releases/tag/v0.2.0>
Docs: `docs/install.md` (server), `docs/install-client.md` (desktop),
`docs/privacy.md` (network behaviour).

Mod-friendly bug reports very welcome — this is single-developer
output and r/linux_gaming has the best testers on the internet.
