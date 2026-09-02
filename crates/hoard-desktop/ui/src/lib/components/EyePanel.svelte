<script lang="ts">
  /**
   * Eye dropdown, live overview of every device in the account.
   *
   * Each device shows: OS logo (emoji) + name + online dot + running game with
   * elapsed time. This machine is always first; other devices from the same
   * account appear below when the server exposes a live device list (see
   * vista.md for the backend spec).
   *
   * Super clean: one row per device, no noise.
   */
  import { _ } from "svelte-i18n";
  import { onMount } from "svelte";
  import { activity, status } from "../stores/agent";
  import { auth } from "../stores/auth";
  import { cloud } from "../stores/cloud";
  import { remoteDevices, refreshDevices } from "../stores/devices";
  import { customNames, hydrateGameNames } from "../stores/gameNames";
  import { prettifySlug } from "../utils/format";
  import { Gamepad2, Server } from "@lucide/svelte";
  import OsLogo from "./OsLogo.svelte";

  let { now }: { now: number } = $props();

  // Presence goes stale in 90s server-side (three missed beats), so a refresh
  // every 15s while the panel is open keeps a sibling's dot honest without
  // polling anything when nobody is looking. Cloud sessions also get Realtime
  // pushes; this just makes the first paint immediate.
  const REFRESH_MS = 15_000;

  onMount(() => {
    // The panel opens from the header, on any route, so it can't assume the
    // Library already pulled the display-name overrides off disk.
    void hydrateGameNames();
    void refreshDevices();
    const t = setInterval(() => void refreshDevices(), REFRESH_MS);
    return () => clearInterval(t);
  });

  // --- OS detection -------------------------------------------------------
  // Read from the <html> class set by App.svelte on boot. Returns the OS key
  // for OsLogo + a display name.
  type OsInfo = { key: "windows" | "linux" | "macos" | "unknown"; name: string };

  const thisOs = $derived.by<OsInfo>(() => {
    const html = document.documentElement;
    if (html.classList.contains("is-linux")) return { key: "linux", name: "Linux" };
    if (html.classList.contains("is-macos")) return { key: "macos", name: "macOS" };
    if (html.classList.contains("is-windows")) return { key: "windows", name: "Windows" };
    return { key: "unknown", name: "" };
  });

  // --- This machine's running games ---------------------------------------
  // One entry per *game*, not per tracked save: two saves of the same game are
  // a single session to the eye, exactly like the heartbeat this machine sends
  // out (see hoard-agent/src/presence.rs). Most recently started first, which
  // is also the order the server hands the other machines back in.
  const runningGames = $derived.by<PlayingGame[]>(() => {
    const bySlug = new Map<string, number>();
    for (const [saveId, a] of Object.entries($activity)) {
      if (a.state !== "running") continue;
      // The activity map is keyed by save id (a UUID). The slug rides along on
      // the `game_started` event; falling back to the key is only for an entry
      // that predates it, and it's the reason this panel used to name a game
      // "2fabb61b Fa7a 4e3f...".
      const slug = a.game_slug ?? saveId;
      const since = a.running_since ?? now;
      const prev = bySlug.get(slug);
      if (prev === undefined || since < prev) bySlug.set(slug, since);
    }
    return [...bySlug]
      .map(([slug, since]) => ({ name: gameName(slug), since }))
      .sort((a, b) => b.since - a.since);
  });

  /** The `os` a sibling reported in its headers → the logo + label pair. */
  function osFor(os: string | null | undefined): OsInfo {
    switch ((os ?? "").toLowerCase()) {
      case "linux":
        return { key: "linux", name: "Linux" };
      case "macos":
        return { key: "macos", name: "macOS" };
      case "windows":
        return { key: "windows", name: "Windows" };
      default:
        return { key: "unknown", name: "" };
    }
  }

  /** What to call a game on screen. Same rule as the Library cards: the
   *  user's own name for it if they set one, the prettified slug otherwise,
   *  so the panel and the cards never disagree about what a game is called.
   *  It applies to the other machines too: the override is this device's way
   *  of naming a slug, and the slug is the same everywhere. */
  function gameName(slug: string): string {
    return $customNames[slug] ?? prettifySlug(slug);
  }

  function fmtElapsed(since: number): string {
    const secs = Math.max(0, Math.floor((now - since) / 1000));
    if (secs < 60) return `${secs}s`;
    const m = Math.floor(secs / 60);
    if (m < 60) return `${m}m ${secs % 60}s`;
    const h = Math.floor(m / 60);
    return `${h}h ${m % 60}m`;
  }

  // --- Device model -------------------------------------------------------
  // This machine is always present and always first, painted from live local
  // state; the rest of the account comes from `GET /v1/devices` below.
  /** A game a device is running: display name + session start (epoch ms). */
  type PlayingGame = { name: string; since?: number };

  type Device = {
    name: string;
    os: OsInfo;
    online: boolean;
    /** Every game running there, most recently started first. Empty = idle. */
    games: PlayingGame[];
  };

  const thisDevice = $derived<Device>({
    name: $_("eye.this_machine"),
    os: thisOs,
    online: $status.running,
    games: runningGames,
  });

  // The rest of the account's machines, from `GET /v1/devices`. `this_device`
  // is dropped: this machine is rendered above from live local state, which is
  // always fresher than its own heartbeat echoed back by the server.
  //
  // `since` comes as RFC3339 anchored to the *server's* clock, which is what
  // makes the elapsed time trustworthy: a sibling with a skewed clock can't
  // claim it's been playing since tomorrow.
  const otherDevices = $derived<Device[]>(
    $remoteDevices
      .filter((d) => !d.this_device)
      .map((d) => ({
        name: d.device_name,
        os: osFor(d.os),
        online: d.online,
        // All of them: the server keeps up to eight per device and a machine
        // with two games open must read as two, not as whichever one started
        // last.
        games: (d.playing ?? []).map((g) => {
          const since = g.since ? Date.parse(g.since) : NaN;
          return {
            name: gameName(g.slug),
            since: Number.isNaN(since) ? undefined : since,
          };
        }),
      })),
  );

  const allDevices = $derived([thisDevice, ...otherDevices]);
</script>

<div class="space-y-1 px-3 py-3">
  <!-- Device rows -->
  {#each allDevices as d, i (i)}
    <div class="flex items-center gap-2.5 px-1 py-1.5">
      <!-- Online dot (green pulse when online + playing, green solid when
           online idle, grey when offline) -->
      <span class="relative flex h-2.5 w-2.5 shrink-0">
        {#if d.online && d.games.length > 0}
          <span class="absolute inline-flex h-full w-full animate-ping rounded-full bg-sky-400/60"></span>
        {/if}
        <span class="relative inline-flex h-2.5 w-2.5 rounded-full {d.online
          ? d.games.length > 0 ? 'bg-sky-400' : 'bg-emerald-400'
          : 'bg-zinc-600'}"></span>
      </span>
      <!-- OS logo — real SVG, tinted green when online, grey when offline -->
      <span class="shrink-0 {d.online ? 'text-emerald-400' : 'text-zinc-600'}">
        <OsLogo os={d.os.key} size={16} />
      </span>
      <!-- Device name -->
      <span class="flex-1 truncate text-xs font-medium text-zinc-200">
        {d.name}
        {#if d.os.name}<span class="text-zinc-500">· {d.os.name}</span>{/if}
      </span>
      <!-- Status: playing (sky) or online (green) or offline (grey). With two
           games open the clock moves down to each game's own row — one number
           up here couldn't say which session it was timing. -->
      {#if d.games.length === 1}
        <span class="shrink-0 font-mono text-[11px] tabular-nums text-sky-400">
          {fmtElapsed(d.games[0].since ?? now)}
        </span>
      {:else if d.online}
        <span class="shrink-0 text-[10px] font-medium uppercase tracking-wide text-emerald-400">
          {$_("eye.online")}
        </span>
      {:else}
        <span class="shrink-0 text-[10px] font-medium uppercase tracking-wide text-zinc-500">
          {$_("eye.offline")}
        </span>
      {/if}
    </div>
    <!-- Running games indented under the device, one row each -->
    {#each d.games as g (g.name)}
      <div class="flex items-center gap-2.5 py-0.5 pl-9 pr-1">
        <Gamepad2 size={12} class="shrink-0 text-sky-400" />
        <span class="flex-1 truncate text-[11px] text-zinc-400">{g.name}</span>
        {#if d.games.length > 1}
          <span class="shrink-0 font-mono text-[11px] tabular-nums text-sky-400">
            {fmtElapsed(g.since ?? now)}
          </span>
        {/if}
      </div>
    {/each}
  {/each}

  <!-- Self-hosted server (if connected, separate section) -->
  {#if $auth.user?.server_url}
    <div class="mt-2 flex items-center gap-2.5 border-t border-white/[0.06] px-1 pt-2.5">
      <span class="relative flex h-2.5 w-2.5 shrink-0">
        <span class="relative inline-flex h-2.5 w-2.5 rounded-full bg-emerald-400"></span>
      </span>
      <Server size={14} class="shrink-0 text-zinc-400" />
      <span class="flex-1 truncate text-xs font-medium text-zinc-200">
        {$auth.user.server_url.replace(/^https?:\/\//, "").replace(/\/.*$/, "")}
      </span>
      <span class="shrink-0 text-[10px] font-medium uppercase tracking-wide text-emerald-400">
        {$_("eye.online")}
      </span>
    </div>
  {/if}

  <!-- If this machine is online but nothing is running, a quiet hint -->
  {#if runningGames.length === 0 && $status.running}
    <p class="px-1 py-1 text-[11px] text-zinc-600">
      {$_("eye.nothing_playing")}
    </p>
  {/if}

  <!-- Cloud account device count (informational — shows how many devices are
       linked, even though we can't list them individually yet). -->
  {#if $cloud.account && $cloud.account.devices_used > 1}
    <p class="mt-1 px-1 text-[10px] text-zinc-600">
      {$cloud.account.devices_used} {$_("eye.devices_linked")}
    </p>
  {/if}
</div>
