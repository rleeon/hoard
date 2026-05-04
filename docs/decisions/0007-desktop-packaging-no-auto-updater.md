# 0007 — Desktop packaging targets, no auto-updater for v0.2

**Status:** accepted, 2026-05-04

## Context

For the v0.2 launch we need to ship the Tauri desktop app to end users
across Linux, Windows, and macOS. The Tauri bundler can produce
`.deb / .rpm / .AppImage` (Linux), `.msi / .nsis .exe` (Windows), and
`.dmg / .app` (macOS) — and Tauri also provides a first-party
`tauri-plugin-updater` for in-app upgrades.

We want to know which targets to actually ship, and whether to enable
auto-updates from day one.

## Decision

**Bundle targets we publish on every release:**

- Linux: `deb`, `rpm`, `appimage`
- Windows: `nsis` (preferred for the launcher experience) and `msi`
  (preferred for IT-managed deployments)
- macOS: `dmg` (Intel + Apple Silicon as separate artifacts)

These are produced by `.github/workflows/release-desktop.yml`, which
runs on every `v*.*.*` tag alongside the existing server release
workflow.

**Auto-updater: deferred to v0.3.**

Tauri's auto-updater requires:

1. An ed25519 keypair where the **public** key is baked into the binary
   at compile time and the **private** key signs the release
   `latest.json` payload from CI.
2. Code-signing certificates on Windows (Authenticode) and macOS
   (Developer ID + notarization), or every update will be a SmartScreen
   / Gatekeeper warning that defeats the purpose of an unattended
   updater.

We don't have either yet, and rushing the keypair without
code-signing means users get a "trust this update" dialog on every
upgrade — strictly worse than telling them to download a new
installer themselves once a quarter.

For v0.2 we ship a **manual update path**: the Settings → About card
links to the GitHub Releases page, and the in-app version string lets
users compare. Once we have signing certs (rolled into a separate
pre-v0.3 work item) we'll flip on the updater plugin in a single
focused change.

## Consequences

- First-run on Windows shows the SmartScreen "unrecognised publisher"
  warning. Documented in `docs/install-client.md` as a known caveat.
- First-run on macOS requires the right-click → Open dance until
  notarization is in place. Same.
- No telemetry round-trip is needed for "is the user up to date?" —
  the dashboard / About card just shows the embedded version string.
- We avoid the worst-case scenario of an updater that delivers
  unsigned binaries to users who can't tell whether a "new version"
  dialog is legitimate.

## Alternatives considered

- **Ship the updater with self-signed bundles.** Rejected: the OS
  warnings are unchanged, so users get the same friction *and* a
  surface for fake-update phishing.
- **Use a third-party updater (Squirrel, WinSparkle).** Rejected: more
  moving parts, and still wants signed binaries to be useful.
- **Skip Windows / macOS entirely for v0.2.** Rejected: half the target
  audience (r/SteamDeck, r/linux_gaming) is on Linux, but the other
  half follows from r/selfhosted and uses everything. Better to ship
  unsigned binaries with clear docs than to leave Windows/macOS users
  out.
