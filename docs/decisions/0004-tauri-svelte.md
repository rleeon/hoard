# 0004 — Tauri 2 + Svelte 5 + Tailwind v4 for the desktop client

Date: 2026-05-03
Status: accepted

## Context

v0.1 shipped a server, an admin CLI, and a developer-oriented `hoard` CLI.
v0.2's goal is the opposite audience: a gamer who installs Hoard once,
clicks through a setup wizard, and forgets it's running while it auto-syncs
their saves. That means a real GUI, a system-tray background process, and
binaries for both Linux and Windows (we treat macOS as best-effort for now
because we don't have hardware to test on).

The decision boils down to two questions:

1. **What stack for the desktop app?** Electron, Tauri, or native (GTK/Qt)?
2. **What stack for the frontend?** React, Svelte, Vue, plain HTML?

We need to pick now because changing later means rewriting most of the
client. Both choices are load-bearing for everything in v0.2.

## Decision

**Tauri 2.x for the shell. Svelte 5 + TypeScript + Tailwind v4 for the UI.**

Concretely:

- New crate `crates/hoard-desktop` is a Tauri 2 binary that owns the
  window, system tray, autostart registration, and the `#[tauri::command]`
  bridge.
- Frontend lives in `crates/hoard-desktop/ui/` as a Vite-built Svelte app
  bundled into the Tauri binary at compile time.
- Shared client logic (HTTP API, config, state, snapshot upload/download)
  lives in `crates/hoard-agent` so the CLI (`hoard-cli`) and the desktop
  app share the same code paths.
- Plugins enabled from day one: `single-instance`, `autostart`,
  `notification`, `dialog`, `os`, `shell`. Everything else is opt-in
  per-phase.

## Considered alternatives

### Electron + React (the obvious choice)

The path of least resistance — every tutorial, every Stack Overflow
answer, every job posting. We ruled it out for three reasons:

1. **Binary size.** A trivial Electron app is ~150 MB unpacked. Tauri's
   equivalent is ~15 MB. For an app that lives in the system tray and is
   installed by people who self-host because they care about resource
   usage, "Discord eats my RAM" is the meme we explicitly want to avoid.
2. **Idle RAM.** Electron's main + renderer + GPU processes idle around
   250–350 MB. Tauri reuses the OS webview, so the same idle is roughly
   80–120 MB. Multiply by "always running" and the difference is real.
3. **Audience fit.** The r/selfhosted / r/linux_gaming crowd actively
   penalises Electron. Tauri lands on HN front pages on its own merit.
   Picking Tauri is also a marketing signal that we're different from
   Steam Cloud.

### Native GTK or Qt

Smaller binaries, no webview overhead, "real" native feel. Ruled out
because:

- Cross-compiling GTK to Windows is painful in 2026 — gtk-rs works but
  the bundling story is fragile and undocumented for novices.
- Qt's licensing (LGPL with attribution, or commercial) adds friction for
  a copyleft self-hosted project.
- "Native by default" actually looks dated on Linux distros where the
  theming is inconsistent and on Windows 11 where the native style is
  WinUI/Fluent which Qt/GTK approximate poorly.

### Svelte vs React

For the frontend we picked Svelte over React because:

- **Less boilerplate per feature.** Reactive state is a `$state(...)`
  rune, no `useState` + `useEffect` ceremony. The first onboarding wizard
  needs ~6 components; React adds 30 % more code for the same UI.
- **Smaller bundle.** ~40 KB gzipped for a non-trivial Svelte app vs
  ~180 KB minimum for React + ReactDOM. Matters because the bundle is
  loaded by the embedded webview at every cold start.
- **Easier for a junior dev.** The author of this project is in the
  middle of "DAM" (Spanish vocational software degree). Svelte's HTML +
  `<script>` + `<style>` model is closer to what they learned than
  React's JSX + hooks model.

### Tailwind v4 vs traditional CSS

Tailwind v4's zero-config mode (`@import "tailwindcss"`) eliminates the
`tailwind.config.js` file we never wanted to maintain. Combined with the
`@theme` block in `app.css` we get one place to define colour tokens.
Alternatives (CSS modules, vanilla-extract, plain BEM) all require more
infrastructure for less consistency.

## Consequences

### Pros

- Small binaries; `cargo tauri build` produces a ~15 MB `.deb` and
  ~12 MB `.msi` once `lto = true` is set in release profile.
- Same Rust workspace; `hoard-desktop` reuses everything in
  `hoard-agent`. No FFI, no IPC across language boundaries, no
  duplicated DTOs.
- Svelte's reactivity model + Tailwind utility classes mean a UI change
  is usually a single file diff — short feedback loops while iterating
  on UX.
- The Tauri 2 plugin ecosystem covers everything we need for v0.2
  (tray, autostart, notifications) with one-line registrations.

### Cons

- **Smaller community than Electron + React.** Some Stack Overflow
  questions have one answer instead of fifty. The mitigation is that
  Tauri's official Discord is responsive and the docs are good.
- **Webview compatibility quirks per OS.** On Linux we depend on
  WebKitGTK 4.1; on Windows we use WebView2 (Edge). They mostly
  behave the same but `prefers-color-scheme`, file:// quirks, and CSS
  paint timing differ. We test on both from phase 0.
- **GNOME tray icons.** Plain GNOME doesn't show tray icons; users need
  the AppIndicator extension. We document this in the install guide
  rather than ship a workaround.
- **macOS is best-effort.** We don't own a Mac. The Tauri builder will
  produce a `.dmg` from CI, but we won't claim "supported" until someone
  with hardware verifies the flow.
- **Svelte runes mode is opt-in per component.** Some libraries
  (lucide-svelte ≤ 0.46) still emit the legacy `$$props` pattern that
  isn't compatible with global runes mode. Solution: don't force
  `compilerOptions.runes = true`; using `$state` etc. in our own
  components automatically opts them in, while third-party legacy
  components keep working.

## Validation

This ADR is accepted for v0.2. We commit to revisiting it before v0.3 if
any of the following happen:

- Idle RAM on Linux climbs above 200 MB after profiling.
- Tauri 2 ships breaking changes that block more than one minor release.
- A contributor with macOS access reports the Mac flow is fundamentally
  broken (vs. just rough).
