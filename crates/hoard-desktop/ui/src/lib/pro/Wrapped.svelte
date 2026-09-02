<script lang="ts">
  // hoard-wrapple, a personal "year in play" recap.
  //
  // Two pieces:
  //   1. An identity strip (avatar + name shown; only the EMAIL is masked by
  //      default, with a small inline toggle sitting on it).
  //   2. A GitHub-contributions-style calendar: one square per day for the last
  //      year, shaded by how many HOURS you played that day, with the seven
  //      weekdays as rows flipped to read Sunday→Monday (Mon/Wed/Fri labelled).
  //
  // The agent clocks real playtime locally (time a tracked game's process was
  // running) and `syncPlaytime` pushes it to Hoard Cloud, then reads back the
  // DEVICE-MERGED aggregate, so the recap reflects every machine you sign in
  // from, not just this one. Signed out / offline it falls back to local data.
  import { onMount } from "svelte";
  import { tilt } from "../actions/tilt";
  import {
    currentUser,
    listTrackedSaves,
    syncPlaytime,
    cloudCurrentAccount,
    cloudRefreshAccount,
    cachedDetection,
    type UserInfo,
    type TrackedSave,
    type CloudAccountInfo,
  } from "../api";
  import Cover from "../components/Cover.svelte";
  import WrappedCard from "./WrappedCard.svelte";
  import {
    Eye,
    EyeOff,
    Server,
    Cloud,
    Flame,
    CalendarCheck,
    Gamepad2,
    Trophy,
    Crown,
    Clock,
    Sparkles,
    Camera,
    X,
  } from "@lucide/svelte";
  import { tr, fmtBytes } from "./lib";
  import { prefs } from "../stores/prefs";

  // --- identity ----------------------------------------------------------
  // The recap prefers the Hoard Cloud account (it carries the Google avatar +
  // email), and falls back to the self-hosted server user when there's no
  // cloud session. That's why a cloud-only user used to see "Sin sesión": the
  // card only read `currentUser()`.
  let user = $state<UserInfo | null>(null);
  let account = $state<CloudAccountInfo | null>(null);
  let revealed = $state(false);

  // Resolved identity: { name, sub, avatar, cloud }. `sub` is the secondary
  // line (email for cloud, server URL for self-hosted).
  const identity = $derived.by(() => {
    if (account) {
      return {
        name: account.display_name?.trim() || account.email,
        sub: account.email,
        avatar: account.avatar_url,
        cloud: true,
      };
    }
    if (user) {
      return {
        name: user.username,
        sub: user.server_url,
        avatar: null as string | null,
        cloud: false,
      };
    }
    return {
      name: tr({ es: "Sin sesión", en: "Signed out" }),
      sub: "—",
      avatar: null as string | null,
      cloud: false,
    };
  });

  const initials = $derived(
    (identity.name ?? "?")
      .split(/[\s._@.-]+/)
      .filter(Boolean)
      .slice(0, 2)
      .map((s) => s[0]?.toUpperCase() ?? "")
      .join("") || "?",
  );

  // Masked form of the email: keep its shape (@ and dots) but dot out every
  // other character. A static placeholder beats blurring the real text, blur
  // + transition on antialiased glyphs flickered the colours mid-toggle.
  const maskedSub = $derived((identity.sub ?? "").replace(/[^@.]/g, "•") || "—");

  // --- calendar ----------------------------------------------------------
  // `secs` = seconds played that day (the heatmap intensity).
  type Day = { key: string; date: Date; secs: number; inRange: boolean };

  let loading = $state(true);
  let weeks = $state<Day[][]>([]);
  let totalGames = $state(0);
  let totalBytes = $state(0);
  let mostPlayed = $state<string | null>(null);
  let mostPlayedSlug = $state<string | null>(null);

  // slug → Steam app id, read from the cached detection report (no scan). Used
  // only to show cover art; a miss just falls back to the initial-letter tile.
  let appIdBySlug = $state<Record<string, number>>({});

  // "Atesorado" = everything ever stored, including deleted saves. Prefer the
  // server's monotonic lifetime counter; fall back to the current footprint
  // when it's missing (old server) or offline.
  const hoardedBytes = $derived(
    account && account.lifetime_storage_bytes > 0
      ? account.lifetime_storage_bytes
      : totalBytes,
  );

  // Per-day game breakdown (day → slug → secs) and the day totals, both feed
  // the click-to-expand detail panel below.
  let dailyByGame = $state<Record<string, Record<string, number>>>({});
  let daysByKey = $state<Record<string, number>>({});
  // Day whose detail panel is open (its `key`), or null when none.
  let selectedKey = $state<string | null>(null);
  // La tarjeta compartible (el botón de la cámara, al final de la página).
  let showCard = $state(false);
  // Year filter, buttons for every year with any playtime, plus the current
  // year (so a fresh account still sees its own year). Latest first.
  let yearsAvailable = $state<number[]>([]);
  let selectedYear = $state<number>(new Date().getFullYear());

  const DAY_MS = 86_400_000;

  // Games are tracked by `game_slug`; the per-save `label` is the save *slot*
  // name (usually the default "main"), NOT the game. For a per-game recap we
  // render the game name by prettifying its slug, "planet-s" → "Planet S",
  // so it never reads "main".
  function prettySlug(slug: string): string {
    return (
      slug
        .split(/[-_]+/)
        .filter(Boolean)
        .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
        .join(" ") || slug
    );
  }

  function keyToDate(key: string): Date {
    const [y, m, d] = key.split("-").map(Number);
    return new Date(y, (m ?? 1) - 1, d ?? 1);
  }

  function toggleDay(key: string) {
    selectedKey = selectedKey === key ? null : key;
  }

  // Games played on the selected day (resolved to labels, sorted by time).
  // `dayTotal` is the day's full total; when it exceeds the listed sum the
  // difference predates the per-game breakdown (hidden `__other__` slug).
  const dayDetail = $derived.by(() => {
    if (!selectedKey) return null;
    const games = dailyByGame[selectedKey] ?? {};
    const rows = Object.entries(games)
      .filter(([, secs]) => secs > 0)
      .map(([slug, secs]) => ({
        slug,
        label: prettySlug(slug),
        appId: appIdBySlug[slug] ?? null,
        secs,
      }))
      .sort((a, b) => b.secs - a.secs);
    return {
      date: keyToDate(selectedKey),
      rows,
      dayTotal: daysByKey[selectedKey] ?? 0,
    };
  });

  function dayKey(d: Date): string {
    const y = d.getFullYear();
    const m = String(d.getMonth() + 1).padStart(2, "0");
    const day = String(d.getDate()).padStart(2, "0");
    return `${y}-${m}-${day}`;
  }

  // Five shades, scaled to *your* busiest day, GitHub-style relative
  // intensity, not a fixed clock-time bucket. A flat ">2h = darkest" bucket
  // made a 1h day and a 57h day render identically once both crossed it;
  // quartiles of `secs / maxSecs` fix that, since the max day always lands
  // in the top shade and short days stay light regardless of absolute hours.
  function level(secs: number, maxSecs: number): number {
    if (secs <= 0 || maxSecs <= 0) return 0;
    const ratio = secs / maxSecs;
    if (ratio <= 0.25) return 1;
    if (ratio <= 0.5) return 2;
    if (ratio <= 0.75) return 3;
    return 4;
  }

  const LEVEL_BG = [
    "bg-zinc-800/70",
    "bg-emerald-900/80",
    "bg-emerald-700",
    "bg-emerald-500",
    "bg-emerald-400",
  ];

  // Weekday rows in fully-reversed order, top→bottom:
  //   Sat, Fri, Thu, Wed, Tue, Mon, Sun.
  // Sunday (the week's first day) sits at the BOTTOM, not the top.
  // `ROW_ORDER` indexes each week (built Sun..Sat, so index == JS getDay) in
  // that display order; `WEEKDAYS` is the matching label column, like GitHub
  // we only label Mon / Wed / Fri and leave the rest blank.
  const ROW_ORDER = [6, 5, 4, 3, 2, 1, 0];
  const WEEKDAYS = [
    "",
    tr({ es: "vie", en: "Fri" }),
    "",
    tr({ es: "mié", en: "Wed" }),
    "",
    tr({ es: "lun", en: "Mon" }),
    "",
  ];

  function fmtDur(secs: number): string {
    if (secs <= 0) return "—";
    const h = Math.floor(secs / 3600);
    const m = Math.round((secs % 3600) / 60);
    if (h > 0) return m > 0 ? `${h} h ${m} min` : `${h} h`;
    return `${Math.max(1, m)} min`;
  }

  // derived stats over the visible range
  const stats = $derived.by(() => {
    // Chronological order for the streak math: `weeks` is reversed for the grid
    // (newest first), so flattening it jumps backwards a week at each boundary
    // and would split any streak that crosses a Sunday. Sort by date so the
    // run counters see real consecutive days.
    const days = weeks
      .flat()
      .filter((d) => d.inRange)
      .sort((a, b) => +a.date - +b.date);
    const active = days.filter((d) => d.secs > 0).length;
    const totalSecs = days.reduce((a, d) => a + d.secs, 0);
    let longest = 0;
    let run = 0;
    let current = 0;
    for (const d of days) {
      if (d.secs > 0) {
        run += 1;
        longest = Math.max(longest, run);
      } else run = 0;
    }
    // current streak: walk back from the last in-range day
    for (let i = days.length - 1; i >= 0; i--) {
      if (days[i].secs > 0) current += 1;
      else break;
    }
    const busiest = days.reduce<Day | null>(
      (best, d) => (!best || d.secs > best.secs ? d : best),
      null,
    );
    return { active, totalSecs, longest, current, busiest };
  });

  const monthLabels = $derived.by(() => {
    const out: { col: number; label: string }[] = [];
    let last = -1;
    weeks.forEach((w, col) => {
      const first = w.find((d) => d.inRange) ?? w[0];
      const m = first.date.getMonth();
      if (m !== last) {
        out.push({
          col,
          label: first.date.toLocaleDateString(undefined, { month: "short" }),
        });
        last = m;
      }
    });
    return out;
  });

  function fmtDay(d: Date): string {
    return d.toLocaleDateString(undefined, {
      day: "numeric",
      month: "short",
      year: "numeric",
    });
  }

  async function build() {
    try {
      user = await currentUser();
    } catch {
      user = null;
    }
    // Refresh `/v1/me` so `lifetime_storage_bytes` (the "Atesorado" source) is
    // current; fall back to the cached account offline / signed out.
    try {
      account = await cloudRefreshAccount();
    } catch {
      try {
        account = await cloudCurrentAccount();
      } catch {
        account = null;
      }
    }

    let saves: TrackedSave[] = [];
    try {
      saves = (await listTrackedSaves()).filter((s) => !s.orphan);
    } catch {
      saves = [];
    }
    totalGames = saves.length;
    // Current server footprint across snapshots, the fallback for "Atesorado"
    // when the lifetime counter isn't available (see `hoardedBytes`).
    totalBytes = saves.reduce((a, s) => a + (s.total_size_bytes || 0), 0);

    // Real per-day playtime, read from the server only: the device-merged
    // aggregate for this account (cloud, or the user's own server when
    // self-hosted). No local fallback, an unreachable server shows an empty
    // recap, never this single machine's local store.
    let days: Record<string, number> = {};
    let byGame: Record<string, number> = {};
    try {
      const pt = await syncPlaytime();
      days = pt.days ?? {};
      byGame = pt.by_game ?? {};
      dailyByGame = pt.daily_by_game ?? {};
    } catch {
      days = {};
      byGame = {};
      dailyByGame = {};
    }
    daysByKey = days;

    // Most-played game (all-time), resolved to its library label.
    let topSlug: string | null = null;
    let topSecs = 0;
    for (const [slug, secs] of Object.entries(byGame)) {
      if (secs > topSecs) {
        topSecs = secs;
        topSlug = slug;
      }
    }
    mostPlayed = topSlug ? prettySlug(topSlug) : null;
    mostPlayedSlug = topSlug;

    // Cover art: map each tracked slug to its Steam app id from the cached
    // detection report (already on disk, no scan). Covers pop in reactively;
    // a slug with no app id just keeps the initial-letter tile.
    try {
      const rep = await cachedDetection();
      const m: Record<string, number> = {};
      for (const g of rep?.games ?? []) {
        if (g.steam_app_id != null) m[g.slug] = g.steam_app_id;
      }
      appIdBySlug = m;
    } catch {
      appIdBySlug = {};
    }

    // Years with any playtime, plus the current year (so a fresh account
    // still shows its own year). Descending → latest first.
    const yrs = new Set<number>();
    for (const k of Object.keys(daysByKey)) {
      const y = Number(k.slice(0, 4));
      if (Number.isFinite(y)) yrs.add(y);
    }
    yrs.add(new Date().getFullYear());
    yearsAvailable = [...yrs].sort((a, b) => b - a);
    selectedYear = yearsAvailable[0];
    buildGrid();
    loading = false;
  }

  /** Rebuild the rolling 53-week grid ending today (today on the LEFT, reading
   *  into the past rightward, the original GitHub-style order). The grid
   *  always spans the last 365 days regardless of the selected year; the year
   *  filter only controls `inRange`, so days outside the chosen year stay in
   *  the bounding box (invisible, excluded from stats) but the layout never
   *  jumps. Pinning the end to today means the current day is always visible.
   *  When the selected year isn't the current one, we instead span that whole
   *  calendar year Jan→Dec so past years show their full history. */
  function buildGrid() {
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    const isCurrentYear = selectedYear === today.getFullYear();
    const end = isCurrentYear ? today : new Date(selectedYear, 11, 31);
    // Rolling 365-day window for the current year; Jan 1 → Dec 31 for past
    // years. Both back up to the Sunday on/before the start so columns align
    // to ISO weeks.
    const start = isCurrentYear
      ? new Date(today.getTime() - 364 * DAY_MS)
      : new Date(selectedYear, 0, 1);
    start.setDate(start.getDate() - start.getDay());

    const grid: Day[][] = [];
    let cursor = new Date(start);
    while (cursor <= end) {
      const week: Day[] = [];
      for (let d = 0; d < 7; d++) {
        const key = dayKey(cursor);
        const inYear = cursor.getFullYear() === selectedYear;
        // For the current year the rolling window also caps at today; for
        // past years the whole calendar year is in range.
        const inWindow = isCurrentYear
          ? inYear && cursor <= today
          : inYear;
        week.push({
          key,
          date: new Date(cursor),
          secs: daysByKey[key] || 0,
          inRange: inWindow,
        });
        cursor = new Date(cursor.getTime() + DAY_MS);
      }
      grid.push(week);
    }
    // Newest week first (today on the LEFT), reading into the past rightward,
    // the original order.
    grid.reverse();
    weeks = grid;
  }

  onMount(build);
</script>

<div class="mx-auto max-w-5xl px-6 py-8">
  <!-- header -->
  <div class="mb-6 flex items-center gap-3">
    <div
      class="grid h-11 w-11 place-items-center rounded-2xl bg-gradient-to-br from-emerald-500/20 to-teal-500/10 ring-1 ring-emerald-400/30"
    >
      <Sparkles size={22} class="text-emerald-300" />
    </div>
    <div>
      <h1 class="font-display text-2xl font-semibold tracking-tight text-zinc-50">
        {tr({ es: "Tu año jugando", en: "Your year in play" })}
      </h1>
      <p class="text-sm text-zinc-400">
        {tr({
          es: "Tu resumen personal, con tus horas de todos tus equipos.",
          en: "Your personal recap, with your hours from every device.",
        })}
      </p>
    </div>
  </div>

  <!-- identity card — masked by default, reveal like a password -->
  <div
    class="tilt relative overflow-hidden rounded-2xl border border-white/[0.08] bg-zinc-900/60 p-4"
    use:tilt
  >
    <!-- Soft glow via a pre-blurred radial-gradient, not `filter: blur()`:
         WebKitGTK (Linux, inside Wrapple) paints a black rectangle when a
         backdrop-filtered sibling repaints over a blur layer. A gradient is
         just a paint, no compositing filter, so the artifact never appears. -->
    <div
      class="pointer-events-none absolute -right-16 -top-16 h-40 w-40 [background:radial-gradient(closest-side,rgba(16,185,129,0.12),transparent)]"
    ></div>
    <div class="flex items-center gap-4">
      <div
        class="h-14 w-14 shrink-0 overflow-hidden rounded-2xl ring-1 ring-emerald-400/30"
      >
        {#if identity.avatar}
          <img
            src={identity.avatar}
            alt={identity.name}
            referrerpolicy="no-referrer"
            class="h-full w-full object-cover"
          />
        {:else}
          <div
            class="grid h-full w-full place-items-center bg-emerald-500/15 font-display text-lg font-bold text-emerald-300"
          >
            {initials}
          </div>
        {/if}
      </div>
      <div class="min-w-0 flex-1">
        <div class="truncate text-lg font-semibold text-zinc-50">
          {identity.name}
        </div>
        <!-- only the email is sensitive: masked by default, with a small
             inline toggle sitting right on it -->
        <div class="mt-0.5 flex items-center gap-1.5 text-xs text-zinc-400">
          {#if identity.cloud}<Cloud size={12} class="shrink-0" />{:else}<Server
              size={12}
              class="shrink-0"
            />{/if}
          <span class="truncate {revealed ? '' : 'select-none'}">
            {revealed ? identity.sub : maskedSub}
          </span>
          <button
            type="button"
            onclick={() => (revealed = !revealed)}
            class="grid h-4 w-4 shrink-0 place-items-center rounded text-zinc-500 transition hover:text-zinc-200"
            aria-label={revealed
              ? tr({ es: "Ocultar correo", en: "Hide email" })
              : tr({ es: "Mostrar correo", en: "Show email" })}
            title={revealed
              ? tr({ es: "Ocultar correo", en: "Hide email" })
              : tr({ es: "Mostrar correo", en: "Show email" })}
          >
            {#if revealed}<EyeOff size={11} />{:else}<Eye size={11} />{/if}
          </button>
        </div>
      </div>
    </div>

    <!-- small facts, not sensitive -->
    <div class="mt-4 grid grid-cols-3 gap-2 text-center">
      <div class="rounded-xl bg-white/[0.03] px-2 py-2.5">
        <div class="flex items-center justify-center gap-1 text-[11px] uppercase tracking-wide text-zinc-500">
          <Gamepad2 size={12} />{tr({ es: "Juegos", en: "Games" })}
        </div>
        <div class="mt-0.5 text-xl font-semibold text-zinc-100">{totalGames}</div>
      </div>
      <div class="rounded-xl bg-white/[0.03] px-2 py-2.5">
        <div class="text-[11px] uppercase tracking-wide text-zinc-500">
          {tr({ es: "Atesorado", en: "Hoarded" })}
        </div>
        <div class="mt-0.5 text-xl font-semibold text-zinc-100">{fmtBytes(hoardedBytes)}</div>
      </div>
      <div class="rounded-xl bg-white/[0.03] px-2 py-2.5">
        <div class="flex items-center justify-center gap-1 text-[11px] uppercase tracking-wide text-zinc-500">
          <Crown size={12} />{tr({ es: "Más jugado", en: "Most played" })}
        </div>
        <div
          class="mt-0.5 flex items-center justify-center gap-1.5"
          title={mostPlayed ?? ""}
        >
          {#if mostPlayedSlug}
            <Cover
              appId={appIdBySlug[mostPlayedSlug] ?? null}
              slug={mostPlayedSlug}
              name={mostPlayed ?? ""}
              class="h-5 w-8 rounded"
              initialClass="text-[10px]"
              fit="smart"
            />
          {/if}
          <span class="truncate text-sm font-semibold text-zinc-100">{mostPlayed ?? "—"}</span>
        </div>
      </div>
    </div>
  </div>

  <!-- activity calendar -->
  <!-- Lleva `tilt` como los otros dos paneles: los tres son la misma clase de
       tarjeta y sólo el primero lo tenía, así que al pasar el ratón por el
       Wrapped uno respondía y los otros dos no. `relative` es para el
       resplandor (`.tilt::after` va con `position:absolute; inset:0`); no
       cambia el contexto de los `absolute` de dentro, que ya cuelgan de sus
       propios contenedores `relative`. -->
  <div
    class="tilt relative mt-4 rounded-2xl border border-white/[0.08] bg-zinc-900/60 p-4"
    use:tilt
  >
    <div class="mb-3 flex flex-wrap items-end justify-between gap-3">
      <div>
        <h2 class="text-sm font-semibold text-zinc-100">
          {tr({ es: "Horas jugadas", en: "Hours played" })}
        </h2>
        <p class="text-xs text-zinc-500">
          {tr({
            es: "Cada cuadro es un día; el color, las horas. Haz clic en uno para ver a qué jugaste.",
            en: "Each square is a day; the shade is how long. Click one to see what you played.",
          })}
        </p>
      </div>
      <div class="flex items-center gap-3">
        <!-- Year filter — always visible (even with a single year) so the
             user can tell which year they're looking at. GitHub-style. -->
        <div
          class="flex gap-1 rounded-lg border border-white/[0.08] bg-zinc-950/40 p-1"
        >
          {#each yearsAvailable as y (y)}
            <button
              type="button"
              onclick={() => {
                selectedYear = y;
                selectedKey = null;
                buildGrid();
              }}
              class="rounded-md px-2.5 py-1 text-xs font-medium transition {selectedYear ===
              y
                ? 'bg-emerald-600/20 text-emerald-300 ring-1 ring-inset ring-emerald-600/40'
                : 'text-zinc-400 hover:text-zinc-200'}"
              >
                {y}
              </button>
            {/each}
          </div>
        <div class="text-right">
          <div class="text-2xl font-bold text-emerald-300">
            {Math.round(stats.totalSecs / 3600)}
          </div>
          <div class="text-[11px] uppercase tracking-wide text-zinc-500">
            {tr({ es: "horas", en: "hours" })}
          </div>
        </div>
      </div>
    </div>

    {#if loading}
      <div class="flex h-40 items-center justify-center">
        <Sparkles size={24} class="animate-pulse text-emerald-400" />
      </div>
    {:else}
      <div class="cal-scroll overflow-x-auto pb-5">
        <div class="inline-flex min-w-full gap-1">
          <!-- weekday labels (rows). pt-4 clears the month-labels row so each
               label sits beside its matching row of squares. -->
          <div
            class="flex shrink-0 flex-col gap-[3px] pr-0.5 pt-4 text-[9px] leading-3 text-zinc-500"
          >
            {#each WEEKDAYS as wl}
              <span class="flex h-3 items-center justify-end">{wl}</span>
            {/each}
          </div>
          <div class="inline-block">
            <!-- month labels -->
            <div class="relative mb-1 h-3 text-[10px] text-zinc-500">
              {#each monthLabels as m}
                <span class="absolute" style="left:{m.col * 15}px;">{m.label}</span>
              {/each}
            </div>
            <!-- grid: columns = weeks, rows = weekdays (Sun→Mon, flipped) -->
            <div class="flex gap-[3px]">
              {#each weeks as week}
                <div class="flex flex-col gap-[3px]">
                  {#each ROW_ORDER as ri}
                    {@const d = week[ri]}
                    {#if d.inRange}
                      <button
                        type="button"
                        onclick={() => toggleDay(d.key)}
                        class="block h-3 w-3 cursor-pointer rounded-[3px] p-0 {LEVEL_BG[
                          level(d.secs, stats.busiest?.secs ?? 0)
                        ]} transition hover:ring-2 hover:ring-emerald-300/60 {selectedKey ===
                        d.key
                          ? 'ring-2 ring-emerald-300'
                          : 'ring-1 ring-inset ring-white/[0.04]'}"
                        title="{fmtDur(d.secs)} · {fmtDay(d.date)}"
                        aria-label="{fmtDur(d.secs)} · {fmtDay(d.date)}"
                      ></button>
                    {:else}
                      <div class="h-3 w-3 rounded-[3px] opacity-0"></div>
                    {/if}
                  {/each}
                </div>
              {/each}
            </div>
          </div>
        </div>
      </div>

      <!-- legend + streaks -->
      <div class="mt-3 flex flex-wrap items-center justify-between gap-3">
        <div class="flex items-center gap-1.5 text-[11px] text-zinc-500">
          {tr({ es: "Menos", en: "Less" })}
          {#each LEVEL_BG as bg}
            <span class="h-3 w-3 rounded-[3px] {bg} ring-1 ring-inset ring-white/[0.04]"></span>
          {/each}
          {tr({ es: "Más", en: "More" })}
        </div>
        <div class="flex items-center gap-4 text-xs">
          <span class="inline-flex items-center gap-1.5 text-zinc-300">
            <CalendarCheck size={13} class="text-emerald-400" />
            {stats.active}
            {tr({ es: "días activos", en: "active days" })}
          </span>
          <span class="inline-flex items-center gap-1.5 text-zinc-300">
            <Flame size={13} class="text-amber-400" />
            {stats.longest}
            {tr({ es: "días racha", en: "day streak" })}
          </span>
        </div>
      </div>

      {#if $prefs && !$prefs.wrapple_telemetry}
        <!-- The recap reads only what this machine ships, so with the switch
             off there is nothing to read. Say so plainly instead of showing a
             convincing zero: the hours ARE still being counted locally, and
             they come back the moment it is turned on again. -->
        <div
          class="mt-3 flex items-center gap-2 rounded-xl bg-white/[0.03] px-3 py-2 text-xs text-zinc-400 ring-1 ring-white/[0.05]"
        >
          <Clock size={14} class="text-amber-300" />
          {tr({
            es: "Wrapple está desactivado en Ajustes › Privacidad. Tus horas se siguen contando en este equipo, pero no salen de él, así que aquí no hay nada que enseñar.",
            en: "Wrapple is turned off in Settings › Privacy. Your hours are still counted on this machine, but they never leave it, so there's nothing to show here.",
          })}
        </div>
      {:else if stats.totalSecs <= 0}
        <div
          class="mt-3 flex items-center gap-2 rounded-xl bg-white/[0.03] px-3 py-2 text-xs text-zinc-400 ring-1 ring-white/[0.05]"
        >
          <Clock size={14} class="text-emerald-300" />
          {tr({
            es: "Aún no hay horas registradas. Juega con Hoard abierto y se irán contando solas.",
            en: "No hours logged yet. Play with Hoard open and they'll start counting.",
          })}
        </div>
      {:else if stats.busiest && stats.busiest.secs > 0}
        <div
          class="mt-3 flex items-center gap-2 rounded-xl bg-gradient-to-r from-emerald-500/10 to-transparent px-3 py-2 text-xs text-zinc-300 ring-1 ring-emerald-400/15"
        >
          <Trophy size={14} class="text-amber-300" />
          {tr({ es: "Tu día más intenso:", en: "Your busiest day:" })}
          <span class="font-semibold text-zinc-100">{fmtDay(stats.busiest.date)}</span>
          <span class="text-zinc-500">({fmtDur(stats.busiest.secs)})</span>
        </div>
      {/if}
    {/if}
  </div>

  <!-- day detail — opens when a calendar square is clicked -->
  {#if dayDetail}
    <div
      class="tilt relative mt-4 overflow-hidden rounded-2xl border border-emerald-400/20 bg-zinc-900/60 shadow-[0_8px_30px_-12px_rgba(16,185,129,0.25)]"
      use:tilt
    >
      <!-- header band -->
      <div
        class="relative flex items-center justify-between gap-3 border-b border-white/[0.08] bg-gradient-to-r from-emerald-500/15 via-emerald-500/[0.04] to-transparent px-4 py-3"
      >
        <div
          class="pointer-events-none absolute -left-10 -top-10 h-28 w-28 [background:radial-gradient(closest-side,rgba(16,185,129,0.12),transparent)]"
        ></div>
        <div class="flex min-w-0 items-center gap-2.5">
          <div
            class="grid h-9 w-9 shrink-0 place-items-center rounded-xl bg-emerald-500/15 ring-1 ring-emerald-400/30"
          >
            <CalendarCheck size={17} class="text-emerald-300" />
          </div>
          <div class="min-w-0">
            <div class="text-[10px] font-medium uppercase tracking-wider text-emerald-300/80">
              {tr({ es: "Ese día jugaste a", en: "That day you played" })}
            </div>
            <h3 class="truncate text-sm font-semibold text-zinc-50">
              {fmtDay(dayDetail.date)}
            </h3>
          </div>
        </div>
        <div class="flex shrink-0 items-center gap-3">
          <div class="text-right">
            <div class="text-lg font-bold leading-none text-emerald-300">
              {fmtDur(dayDetail.dayTotal)}
            </div>
            <div class="mt-0.5 text-[10px] uppercase tracking-wide text-zinc-500">
              {tr({ es: "en total", en: "total" })}
            </div>
          </div>
          <button
            type="button"
            onclick={() => (selectedKey = null)}
            class="grid h-7 w-7 place-items-center rounded-lg border border-white/10 text-zinc-400 transition hover:bg-white/5 hover:text-white"
            aria-label={tr({ es: "Cerrar", en: "Close" })}
          >
            <X size={15} />
          </button>
        </div>
      </div>

      {#if dayDetail.rows.length > 0}
        <ul class="space-y-2 p-3">
          {#each dayDetail.rows as g (g.slug)}
            {@const pct =
              dayDetail.dayTotal > 0
                ? Math.round((g.secs / dayDetail.dayTotal) * 100)
                : 0}
            <li
              class="flex items-center gap-3 rounded-xl bg-white/[0.03] p-2 ring-1 ring-white/[0.05] transition hover:bg-white/[0.05] hover:ring-emerald-400/25"
            >
              <Cover
                appId={g.appId}
                slug={g.slug}
                name={g.label}
                class="h-11 w-[74px] rounded-lg"
                initialClass="text-lg"
                fit="smart"
              />
              <div class="min-w-0 flex-1">
                <div class="flex items-center justify-between gap-2">
                  <span class="truncate text-sm font-medium text-zinc-100">{g.label}</span>
                  <span class="shrink-0 text-sm font-semibold text-emerald-300">
                    {fmtDur(g.secs)}
                  </span>
                </div>
                <div class="mt-2 flex items-center gap-2">
                  <div class="h-1.5 flex-1 overflow-hidden rounded-full bg-white/[0.06]">
                    <div
                      class="h-full rounded-full bg-gradient-to-r from-emerald-500 to-emerald-300"
                      style="width:{Math.max(4, pct)}%"
                    ></div>
                  </div>
                  <span class="w-9 shrink-0 text-right text-[10px] tabular-nums text-zinc-500">
                    {pct}%
                  </span>
                </div>
              </div>
            </li>
          {/each}
        </ul>
      {:else if dayDetail.dayTotal > 0}
        <p class="px-4 py-3 text-xs text-zinc-400">
          {tr({
            es: "Jugaste este día, pero sin desglose por juego (horas previas a esta función).",
            en: "You played this day, but with no per-game breakdown (hours predate this feature).",
          })}
        </p>
      {:else}
        <p class="px-4 py-3 text-xs text-zinc-500">
          {tr({ es: "No jugaste este día.", en: "You didn't play this day." })}
        </p>
      {/if}
    </div>
  {/if}

  <!-- Cierre de la página: la barra de la cámara. Abre la tarjeta
       compartible — el mismo resumen, en una imagen que se puede enseñar. -->
  <!-- Es el tercer bloque a ancho completo de la página, así que responde como
       los otros dos aunque sea un botón: sin esto, pasar el ratón por el recap
       inclinaba las dos tarjetas de arriba y aquí no ocurría nada. -->
  <button
    type="button"
    onclick={() => (showCard = !showCard)}
    use:tilt
    class="tilt relative group mt-4 flex w-full items-center justify-center gap-2.5 rounded-2xl border px-4 py-3.5 transition {showCard
      ? 'border-emerald-400/40 bg-emerald-500/10 text-emerald-200'
      : 'border-white/[0.08] bg-zinc-900/60 text-zinc-300 hover:border-emerald-400/30 hover:bg-emerald-500/[0.06] hover:text-emerald-200'}"
    aria-expanded={showCard}
    title={tr({
      es: "Crea una imagen de tu resumen para compartir",
      en: "Turn your recap into a shareable image",
    })}
  >
    <span
      class="grid h-8 w-8 place-items-center rounded-xl bg-emerald-500/15 ring-1 ring-emerald-400/30 transition group-hover:bg-emerald-500/25"
    >
      <Camera size={17} class="text-emerald-300" />
    </span>
    <span class="text-sm font-medium">
      {showCard
        ? tr({ es: "Cerrar la tarjeta", en: "Close the card", de: "Karte schließen", fr: "Fermer la carte", it: "Chiudi la card", ja: "カードを閉じる", pt: "Fechar o card", zh: "关闭卡片" })
        : tr({ es: "Tu tarjeta para compartir", en: "Your shareable card", de: "Deine Karte zum Teilen", fr: "Ta carte à partager", it: "La tua card da condividere", ja: "共有用カード", pt: "Seu card para compartilhar", zh: "可分享的卡片" })}
    </span>
  </button>

  {#if showCard}
    <WrappedCard
      daysByKey={daysByKey}
      dailyByGame={dailyByGame}
      appIdBySlug={appIdBySlug}
      sessionName={identity.name}
      sessionAvatar={identity.avatar}
      totalGames={totalGames}
      hoardedBytes={hoardedBytes}
      onClose={() => (showCard = false)}
    />
  {/if}
</div>

<style>
  /* The day-grid scrolls horizontally only when the window is too narrow to
     show every square. Styling the scrollbar makes WebKit render a real,
     space-reserving bar (not an overlay), so it sits BELOW the squares instead
     of painting over the bottom row. */
  .cal-scroll {
    scrollbar-width: thin;
    scrollbar-color: rgba(113, 113, 122, 0.5) transparent;
  }
  .cal-scroll::-webkit-scrollbar {
    height: 8px;
  }
  .cal-scroll::-webkit-scrollbar-track {
    background: transparent;
  }
  .cal-scroll::-webkit-scrollbar-thumb {
    background-color: rgba(113, 113, 122, 0.45);
    border-radius: 9999px;
  }
  .cal-scroll::-webkit-scrollbar-thumb:hover {
    background-color: rgba(161, 161, 170, 0.7);
  }
</style>
