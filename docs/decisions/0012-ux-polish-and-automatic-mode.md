# 0012 — UX polish, Automatic Mode and detected-game blacklist

## Status

Accepted, 2026-05-19. Lands in 1.5.3. Extends ADRs
[0009](0009-path-detection-overhaul.md),
[0010](0010-aggressive-discovery-and-delete.md) and
[0011](0011-windows-detection-and-extra-prefixes.md) — the detection
pipeline established by those three is untouched. This ADR is 100%
UX, persistence-lightweight (one new field in `Prefs`, one new field
in `CliState`) and background-scheduling. Does not supersede any
antecedent.

## Context

The three-cycle detection overhaul (1.5.0–1.5.2) closed the
structural cracks of the pipeline. The recon that opened the 1.5.3
cycle (`docs/plans/1.5.3.md` §0) listed the six remaining UX
friction points that the post-overhaul Library exposes day-to-day.
Each one shares the same shape: a flow whose engine is correct but
whose surfacing to the user is rough.

1. **Updater errors render as raw Rust strings.**
   [`apply_desktop_update`](../../crates/hoard-desktop/src/commands/updates.rs#L260)
   returns `Result<_, String>` and the "no installer asset" path
   produces messages like
   `"no installer asset for this platform in release v1.5.2. Assets:
   hoard-1.5.2-linux-x86_64.tar.gz, hoard-1.5.2-linux-x86_64.tar.gz.sha256"`.
   The UI consumer
   ([`UpdateConfirmModal.svelte:67`](../../crates/hoard-desktop/ui/src/lib/components/UpdateConfirmModal.svelte#L67))
   forwards that string verbatim through `toastError(error)`, which
   takes a single string and renders it as a small ephemeral toast.
   The user reads a JSON-looking blob in a corner of the screen with
   no title, no separation between human summary and technical
   detail, no way to copy the detail without selecting text inside a
   toast that is about to disappear.

2. **Server upgrade has a button on Settings but not on the alert
   modal.** The Tauri command
   [`apply_server_update`](../../crates/hoard-desktop/src/commands/updates.rs#L497)
   already exists, already runs `pkexec sh -c "hoard-server upgrade"`,
   and is wired into `Settings.svelte` (lines 177-200). The same
   `UpdateConfirmModal` that handles the desktop-update flow has a
   server-update branch (`isServer`), but that branch only offers
   `navigator.clipboard.writeText("sudo hoard-server upgrade")` plus
   a toast pointing the user at a terminal. The infrastructure to
   run the upgrade in-app is one wiring away; the modal is the
   natural entry point because that is what opens from the sidebar
   amber alert.

3. **Detected games without `found_paths` cannot be dismissed.**
   When a scan surfaces a game whose template did not resolve into
   any on-disk path (Terraforming Mars, Lethal Company are recurring
   examples on the user's library), `Library.svelte` renders the
   "Sin partidas aún" badge and that's it — no trash icon, no
   dismiss control. The next scan re-detects the same slug and the
   card comes back. There is no API surface to express "ignore this
   slug" either at session scope or persisted; the only mitigation
   today is for the user to learn to ignore the badge.

4. **`Magic` is one-shot, not a state.**
   [`App.svelte:255-264`](../../crates/hoard-desktop/ui/src/App.svelte#L255)
   wires the Magic button to `runMagicSetup()` which executes
   `scan + auto-track + boot watcher` exactly once per press. There
   is no persistence and no scheduler — the user has to remember to
   press it again when they install a new game. The expected mental
   model is a *mode* the user toggles on once and forgets: when on,
   the scan re-runs on its own at a sane interval and auto-restore
   tags along by cascade.

5. **No indicator of plan usage in the shell.** The server already
   exposes `storage_used_bytes` and `storage_quota_bytes` on
   `/v1/auth/whoami` (see
   [`crates/hoard-server/migrations/0001_users.sql`](../../crates/hoard-server/migrations/0001_users.sql))
   and the client already polls them every 30s
   ([`stores/auth.ts:60`](../../crates/hoard-desktop/ui/src/lib/stores/auth.ts#L60)).
   The data is hydrated, just never rendered. A user on a quota'd
   plan has no way to know they are about to bump into it until a
   backup fails; a user on a local server has no visual confirmation
   that "no quota applies here".

6. **Sidebar and frame feel raw.** The `aside` shell in
   `App.svelte` is a flat `w-60 border-r` over `zinc-800` /
   `zinc-950`, nav items have a basic hover and no active-state
   accent, no separators between logical groups, no shadow or
   gradient to suggest depth. Window decorations are native
   (`crates/hoard-desktop/tauri.conf.json` → `"decorations": true`)
   and that is the right baseline — the polish needs to land inside
   the content area, not by replacing the OS chrome.

## Decision

Six pieces. None of them touches the detection pipeline; all of
them are UX surfaces over already-correct engines or thin
persistence add-ons on existing state files.

1. **Structured error envelope plus a single `ErrorDialog`.** A new
   type

   ```rust
   pub struct AppError {
       pub title: String,
       pub body: String,
       pub detail: Option<String>,
   }
   ```

   serializable to the JS side, replaces the bare-string error
   channel of `apply_desktop_update` and `apply_server_update` (and
   becomes available to any other command that wants legible
   surfaces). The "no installer asset" path constructs
   `AppError { title: "updates.error.title", body:
   "updates.error.no_installer", detail: Some(format!("Assets:
   {}", assets.join(", "))) }`; other failure modes (download,
   launch, signature) follow the same shape. A single Svelte
   component `lib/components/ErrorDialog.svelte` renders the modal
   with title, body and a collapsible "Ver detalles técnicos"
   region that exposes `detail` inside a `<pre>`. A store
   `lib/stores/error_dialog.ts` (`showError(error: AppError)`)
   lets any callsite raise the dialog without prop-drilling. The
   existing `toastError` stays for genuinely toast-shaped
   notifications (success-flavoured info, transient feedback);
   error envelopes go to the dialog.

2. **"Actualizar servidor" button on `UpdateConfirmModal`** with
   command-copy fallback. The `isServer` branch grows a guard
   `canRunServerUpgrade = $auth.user?.is_local_server === true &&
   platform() === "linux"`. When true, a green "Actualizar
   servidor" button calls the existing `apply_server_update`
   command, the outcome variant
   (`UpgradedAndRestarted` ⇒ success toast, `Upgraded` ⇒ info
   toast pointing the user at the restart step) is surfaced
   inline. When false (remote server, or non-Linux host), the
   modal keeps the current "Copiar comando" affordance — same
   behaviour as today, just no longer the only option for the
   local-Linux happy path.

3. **Detected-game blacklist with opt-in checkbox.** `CliState`
   grows

   ```rust
   #[serde(default)]
   pub ignored_slugs: HashSet<String>,
   ```

   with helpers `add_ignored_slug`, `remove_ignored_slug`,
   `is_ignored`. Three new Tauri commands
   (`ignore_detected_game`, `unignore_detected_game`,
   `list_ignored_slugs`) plus a filter step in
   `list_detected_games` that drops ignored slugs *after* full
   detection runs (so the walker still benefits from any install
   dir the ignored slug contributes). The Library card for an
   un-tracked detected game grows a red trash button that opens a
   new modal `IgnoreDetectedModal` with a single checkbox
   "Añadir a blacklist permanente", unchecked by default. The
   distinction is load-bearing: without the check, the dismiss is
   a session-scope `sessionDismissed: Set<string>` in the Library
   component and a future scan brings the card back; with the
   check, the slug persists to `CliState.ignored_slugs` and only
   reappears if the user reactivates it from a new
   `Settings.svelte` section "Juegos ignorados" that lists every
   blacklisted slug with a "Reactivar" button.

4. **Modo Automático: persistent toggle + scheduler + downward
   cascade.** `Prefs` gains

   ```rust
   #[serde(default)]
   pub automatic_mode: bool,
   #[serde(default = "default_scan_interval_hours")]
   pub automatic_scan_interval_hours: u32,
   ```

   with a default of `false` / `6` respectively. A new command
   `set_automatic_mode(enabled: bool)` persists the value and
   applies the *downward* cascade: turning the mode on sets
   `prefs.auto_restore = true` (so the user does not have to
   visit a second screen to get the obvious companion behaviour);
   turning it off leaves `auto_restore` exactly as it stands
   (the user's downstream preference survives independently). A
   Tokio scheduler lives in a new singleton module
   `commands/automatic.rs` keyed off an
   `Arc<Mutex<Option<JoinHandle>>>` stored in Tauri's managed
   state, started/stopped by the command. Each tick runs a
   `runMagicSetup`-equivalent server-side (scan + auto-track
   high-confidence detections), logging through `tracing::info!`.
   The current `App.svelte` Magic button is replaced by a wide
   pill that reads `Modo Automático · ON/OFF` with `bg-emerald-500`
   tint when on and `bg-rose-500` tint when off (no amber — amber
   is reserved for warnings per the project's UI conventions).
   The i18n surface is renamed: every `magic.*` key in the eight
   locale files becomes `automatic.*` (`automatic.title`,
   `automatic.on`, `automatic.off`, `automatic.activated`,
   `automatic.deactivated`, `automatic.scanning`,
   `automatic.help_tooltip`). The old keys are removed; this is
   a non-additive rename and every locale must be synced in the
   same prompt that ships the toggle.

5. **Plan-usage indicator above the toggle.** A small block in the
   sidebar sourced from `$auth.user.storage_used_bytes` /
   `storage_quota_bytes` (already hydrated). When
   `user.is_local_server === true` the block renders a `HardDrive`
   icon plus the localised string `sidebar.plan_local`
   ("Local"). Otherwise it renders three lines: "{used} /
   {quota}" using a new `formatBytes` helper, a 0-100% horizontal
   bar inside an `h-1.5 bg-zinc-800 rounded-full` track filled by
   a span coloured `bg-emerald-500` below 75% usage, `bg-amber-500`
   between 75-90%, `bg-rose-500` above 90%, and the percentage as
   a separate line. The colour ramp is intentional: emerald is
   the "all good" baseline, amber means "look at me before you do
   the next backup", rose means "this will fail soon". Updates
   ride on the existing 30s `refreshQuota` poll — no new
   network surface.

6. **Sidebar and frame polish without custom titlebar.** Native
   decorations stay (`tauri.conf.json: "decorations": true`).
   The polish lands inside the `aside`: a vertical gradient
   `bg-gradient-to-b from-zinc-950 via-zinc-950 to-zinc-900` or
   the equivalent inner-right shadow
   `shadow-[inset_-1px_0_0_0_rgba(255,255,255,0.04)]`, airier
   padding (`px-3 py-4`), tenuous `<hr class="border-zinc-800/60
   my-3">` separators between the nav group, the quota block and
   the toggle. Nav items get
   `hover:bg-zinc-800/60 transition-colors duration-150` and an
   active state expressed as `border-l-2 border-emerald-500
   bg-zinc-800/40` rather than a flat fill. An optional
   `platform()` lookup at mount adds `.is-linux` / `.is-macos` /
   `.is-windows` to `document.documentElement` so the stylesheet
   can apply per-OS micro-adjustments (font stack on Linux,
   reserved top padding for the macOS traffic lights when present)
   without conditional rendering. If `@tauri-apps/plugin-os` is
   not already a dependency, the platform-class step is skipped
   in this cycle rather than adding a new dependency for cosmetic
   gain.

## Consequences

- **One new field in `Prefs`, one new field in `CliState`.** Both
  carry `#[serde(default)]`, so existing on-disk
  `state.json` / `prefs` files keep loading byte-for-byte and the
  upgrade path needs no migration. `automatic_mode` defaults to
  `false`, `automatic_scan_interval_hours` to `6`, `ignored_slugs`
  to an empty set.
- **Background scheduling enters the desktop crate.** A singleton
  `JoinHandle` lives in Tauri-managed state for the lifetime of
  the app process. Start/stop transitions go through the
  `set_automatic_mode` command, which means there is exactly one
  place that owns the lifecycle and the rest of the codebase
  never reaches for it directly. Manual test path: enable Modo
  Automático, watch the `tracing::info!` line at the configured
  interval, disable, confirm the line stops.
- **Server-upgrade button depends on the `is_local_server` flag
  that the desktop already computes.** No new server endpoint;
  the existing `/v1/auth/whoami` payload is sufficient, the
  desktop already hydrates the flag into `$auth.user`. The
  button gates on the flag plus `platform() === "linux"`,
  matching the `pkexec` precondition.
- **Native decorations stay.** No custom titlebar means no
  per-OS traffic-light / minimise-maximise-close work; the polish
  is content-only. The optional per-OS class on the root keeps a
  small door open for cosmetic adjustments without locking us
  into chrome that has to be maintained.
- **Renaming `magic.*` → `automatic.*` is a breaking i18n
  change.** Eight locale files (`en`, `es`, `de`, `fr`, `it`,
  `ja`, `pt`, `zh`) must be synced in the same prompt that ships
  the toggle. Every `$_("magic.…")` call site must be grep'd
  before the keys are removed; `svelte-i18n` does not fail at
  compile time when a key is missing, so a stale reference would
  surface as a raw "magic.title" string in the UI. The rename
  is intentional and we accept the one-time cost.
- **The blacklist filter runs after detection completes.** This
  matters when an ignored slug happens to live in an install
  directory that the walker would otherwise reuse to discover
  unrelated games — running the filter before detection would
  drop the install-dir input. The post-filter location costs at
  most a `HashSet::contains` per detected game (negligible).
- **`AppError` is a new Tauri serialization shape but it is
  additive.** Commands that still return `Result<_, String>`
  keep working; only the migrated commands switch envelopes. JS
  callsites that already do `catch (e) { toastError(String(e)) }`
  keep functioning against the legacy commands and adopt
  `showError(e)` only where the Rust side now emits structured
  errors.

## Alternatives considered and why not

- **Custom titlebar with full window-control reimplementation.**
  Rejected. The user asked for an app that feels "acorde al
  sistema" — native decorations are by definition the most
  system-consistent option. Reimplementing minimise/maximise/close
  plus the macOS traffic lights cross-OS is a maintenance burden
  proportionate to three platforms, every Tauri minor version is
  a regression risk on a non-functional surface, and the closest
  precedents in our reference set (Linear, Notion) only do it on
  macOS and degrade gracefully — which still leaves us with the
  per-OS branching cost. Polish stays inside the content area;
  the OS owns the frame.
- **Blacklist on by default whenever a detected game is
  dismissed.** Rejected. The user mental model is that "tirar a
  la papelera este card" is a session-level action ("get this
  out of my way right now"); persisting it permanently without a
  visible opt-in would silently teach the detector to not
  re-surface the game even after the user installs the save
  manager that triggers the path resolution. The checkbox makes
  the persistence explicit and reversible from Settings.
- **Cascade auto-restore *off* when Modo Automático is turned
  off.** Rejected. The user articulated the cascade as one-way
  on purpose: turning the mode on should activate the obvious
  companion (auto-restore) so the user does not have to flip two
  switches, but turning the mode off should not undo a
  preference the user may have set independently. The asymmetry
  is the contract; reversing it would punish users who toggle
  Modo Automático off for a temporary reason (debugging a sync
  issue, travelling on metered data) by also disabling
  auto-restore behind their back.
- **Render plan usage as a single percentage number.** Rejected.
  A percentage by itself ("87%") is harder to read at a glance
  than a filled bar with a colour ramp — the bar's colour is the
  signal, the percentage is the precise read. The progress bar
  also accommodates the local-server case (the bar is absent
  entirely, the "Local" label takes its place) without changing
  the layout's vertical rhythm.
- **Inline updater errors as a styled block below the modal
  buttons.** Rejected. The "no installer asset" message
  observed in the wild contains nine paths plus version strings,
  totalling ~250 characters before line wraps — the inline block
  would either truncate (losing the detail the user needs to
  report the bug upstream) or expand the modal vertically beyond
  the viewport on small windows. A dedicated dialog with a
  scrollable `<pre>` for `detail` scales to arbitrary message
  lengths, gives the user a single piece of UI to dismiss, and
  is reusable for other long-error flows (backup failures,
  restore conflicts) that will eventually want the same surface.
