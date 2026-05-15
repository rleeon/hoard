# 0008 — In-app updater (supersedes 0007 on the no-updater stance)

**Status:** accepted, 2026-05-15
**Supersedes:** parts of [0007](0007-desktop-packaging-no-auto-updater.md)
on the "no auto-updater for v0.x" decision. The bundle-targets matrix in
0007 is unchanged.

## Context

0007 deferred the desktop auto-updater for two reasons: we didn't have an
ed25519 release-signing keypair, and we didn't have Windows / macOS
code-signing certs. Without code-signing every "update" still triggers
a SmartScreen / Gatekeeper warning, so an unattended updater would have
shipped the same friction it was meant to remove.

A year of users later, the cost / benefit shifted:

- The friction we kept *was* the friction. Operators report tracking
  `hoard-server` on a VPS by hand and re-downloading the `.deb` for
  the desktop client every couple of weeks. Both fall off cliffs
  without a path of least resistance.
- Servers are not single-user surfaces. Auto-restarting a server in
  the middle of a sync is worse than a SmartScreen dialog. We need a
  manual server upgrade path even if the client gets an auto one.
- We still don't have code-signing certs. But the platform installers
  themselves (`.deb` / `.msi` / `.dmg`) already trigger their own
  authorization prompts — `pkexec` on Linux, UAC on Windows, the
  `.dmg` mount dialog on macOS. Re-using *those* prompts costs less
  than waiting for code-signing infrastructure that's still months
  out.

## Decision

**Desktop app: in-app updater shipped, no signing required.**

The desktop app already polled the GitHub releases API for the latest
version (used to colour the sidebar banner). On a "Yes, install" click
we now:

1. Pick the right asset for the host platform — `.deb` on Linux,
   `.msi` on Windows, `.dmg` on macOS — by extension match on the
   release asset list.
2. Download it to the OS download dir (or temp dir as fallback).
3. Hand the file to the platform installer:
   - Linux: `pkexec dpkg -i <path>` — `pkexec` shows a polkit prompt
     for the sudo password, the user sees one auth dialog.
   - Windows: `msiexec /i <path>` — UAC handles confirmation.
   - macOS: `open <path>` — mounts the DMG, user drags to
     `/Applications` as they would for a manual install.

If launching the installer fails we still surface the downloaded path
in a toast, so the user can finish manually rather than getting
stranded.

Tauri's first-party `tauri-plugin-updater` is *not* used. The plugin
wants the ed25519-signed `latest.json` flow, which still requires
infrastructure we don't have. Re-using GitHub Releases as the source
of truth keeps the artifact pipeline a single workflow.

**Server: explicit `hoard-server upgrade` subcommand, no daemon
self-update.**

Servers don't auto-update. The upgrade subcommand:

1. Fetches the latest release tarball
   (`hoard-{version}-linux-x86_64.tar.gz`) from the GitHub API.
2. Strict version compare — bails if running version ≥ latest.
3. Streams the tarball through `tokio_tar::Archive` over a
   `GzipDecoder`, extracts the `hoard-server` binary to
   `<target>/.hoard-server.upgrade.<pid>`, chmods 0o755.
4. Atomic `std::fs::rename()` over the running binary's path. The
   running process keeps its file descriptor; the new binary is in
   place for the next exec.
5. Prints `sudo systemctl restart hoard-server` as the next step.

Crucially, `hoard-server upgrade` does **not**:

- Load the server config. A broken config still upgrades.
- Touch the SQLite database.
- Restart the systemd unit. Init systems vary; an in-flight upload
  shouldn't get killed by the upgrader's choice of restart strategy.

The desktop app's `UpdateConfirmModal` knows that the server path is
not actionable from the client — for a server-side update it copies
`sudo hoard-server upgrade` to the clipboard and tells the user to
run it on their server box.

## Consequences

- Users on every supported platform can upgrade the desktop app
  without a terminal.
- The OS installer prompt is now part of the flow on purpose — it
  doubles as the "you authorised this" gate the absent code-signing
  cert would have provided.
- We don't need to maintain a separate `latest.json` artifact;
  releases stay as the GitHub Release page plus tagged assets.
- Server operators get a one-line upgrade that doesn't risk a config
  reload during transition. The trade-off is they have to remember
  the systemctl restart step — explicit in the success message.
- 0007's bundle-targets matrix still applies. The signing-cert
  alternatives section is also still accurate; we just no longer
  treat the absence of certs as a blocker for *some* form of
  in-app update.

## Alternatives considered

- **Tauri's plugin-updater with self-hosted `latest.json`.** Rejected
  for now — extra infra to maintain, no improvement over the GitHub
  Releases flow we already have. Reconsider once we have signing
  certs and want delta updates.
- **Server self-update on a timer.** Rejected for the reasons in the
  decision: a daemon should not pick its own restart moment. Even a
  cron-style "upgrade at 04:00" defeats users who don't run their
  server on a schedule we control.
- **Show a one-line `curl … | sudo dpkg -i` snippet instead of a
  client-driven install.** Rejected: the user already started in the
  GUI, and the GUI can call `pkexec` just as well as a copy-pasted
  shell command can. Server-side, the copy-paste *is* the UX —
  documented above.
