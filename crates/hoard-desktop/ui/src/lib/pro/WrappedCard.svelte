<script lang="ts">
  /**
   * The Hoard-Wrapped card: the same recap, but as an image you can show. A
   * picture, a name, a wry phrase based on the most-played game, four bits of
   * trivia, and a row of tiles for the chosen range (a week gives seven large
   * squares, a month one tile per day, a year one per month).
   *
   * Two decisions worth not undoing without thinking:
   *
   * 1. **Everything editable is local.** The picture is a PNG under the app-data dir
   *    and the rest lives in a store on this machine. It is not uploaded, not
   *    synced, and not in the account export. See `cardPrefs`.
   * 2. **What you see IS the canvas that gets saved.** The preview is not an HTML
   *    mock-up resembling the PNG: it is the PNG, drawn at 1x on screen and at 2x
   *    when saving (`cardCanvas`). That way there are no two designs to maintain and
   *    no surprises when sharing.
   *
   * The image carries the brand and `hoard.services` visibly and in the PNG metadata
   * Rust writes: if somebody uploads it anywhere, it goes signed.
   */
  import { onMount } from "svelte";
  import { _, locale } from "svelte-i18n";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import {
    Camera,
    ImagePlus,
    Trash2,
    Dices,
    Eye,
    X,
    Lock,
    Loader2,
  } from "@lucide/svelte";
  import { fmtBytes } from "./lib";
  import { pickQuote } from "./phrases";
  import {
    cardPrefs,
    cardPhotoUrl,
    hydrateCardPrefs,
    setCardName,
    setCardQuote,
    setCardRange,
    setCardAccent,
    setCardQuoteVisible,
    rerollQuote,
    setCardPhotoFromPath,
    clearCardPhoto,
    saveCardToGallery,
    type CardRange,
  } from "./cardPrefs.svelte";
  import {
    paint,
    renderToPng,
    loadImage,
    waitForFonts,
    CARD_W,
    CARD_H,
    cardPalette,
    type CardData,
    type Cube,
  } from "./cardCanvas";
  import { coverKey, coverSize, coverUrl } from "../stores/covers";
  import { ACCENT_STOPS, accentHue, gems } from "../stores/theme";
  import { toastError, toastSuccess } from "../stores/toasts";

  let {
    /** Seconds played per day, `YYYY-MM-DD` to seconds. */
    daysByKey = {},
    /** The per-day, per-game breakdown, so we know what was played in the range. */
    dailyByGame = {},
    /** Slug to Steam app id, for the cover art only. */
    appIdBySlug = {},
    /** Slug to the game's real name, from detection. */
    nameBySlug = {},
    /** The session's name: the one used until you type another. */
    sessionName = "",
    /** Avatar de la cuenta Cloud, si hay. La foto local manda sobre este. */
    sessionAvatar = null,
    /** Partidas guardadas y bytes atesorados, para los datos curiosos. */
    totalGames = 0,
    hoardedBytes = 0,
    onClose,
  }: {
    daysByKey?: Record<string, number>;
    dailyByGame?: Record<string, Record<string, number>>;
    appIdBySlug?: Record<string, number>;
    nameBySlug?: Record<string, string>;
    sessionName?: string;
    sessionAvatar?: string | null;
    totalGames?: number;
    hoardedBytes?: number;
    onClose: () => void;
  } = $props();

  let canvas = $state<HTMLCanvasElement | null>(null);
  let photoImg = $state<HTMLImageElement | null>(null);
  let coverImg = $state<HTMLImageElement | null>(null);
  let fontsReady = $state(false);
  let saving = $state(false);
  let busyPhoto = $state(false);

  const prefs = $derived(cardPrefs());
  const range = $derived(prefs.range);

  // ---- range to days
  function dayKey(d: Date): string {
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(
      d.getDate(),
    ).padStart(2, "0")}`;
  }

  function startOfToday(): Date {
    const d = new Date();
    d.setHours(0, 0, 0, 0);
    return d;
  }

  /** The days inside the range, oldest to today. */
  const rangeDays = $derived.by(() => {
    const today = startOfToday();
    const span = range === "week" ? 7 : range === "month" ? 30 : 365;
    // By calendar date: 24 h steps land on 23:00 the day before once a DST
    // change is in between, and every winter tile showed the wrong day.
    return Array.from({ length: span }, (_, i) => {
      const d = new Date(today.getFullYear(), today.getMonth(), today.getDate() - (span - 1 - i));
      return { date: d, key: dayKey(d), secs: daysByKey[dayKey(d)] || 0 };
    });
  });

  const loc = $derived($locale ?? "en");

  /** The tiles. How many depends on the range: 7 days, 30 days or 12 months. */
  const cubes = $derived.by<Cube[]>(() => {
    if (range === "year") {
      // One tile per calendar month the 365 days touch, adding only the days
      // inside the window: 12 or 13 tiles, and they sum to the same hours as the
      // numbers above them. Whole months used to reach past the window at one
      // end and fall short of it at the other.
      const months: { y: number; m: number; secs: number }[] = [];
      for (const d of rangeDays) {
        const y = d.date.getFullYear();
        const m = d.date.getMonth();
        const cur = months[months.length - 1];
        if (cur && cur.y === y && cur.m === m) cur.secs += d.secs;
        else months.push({ y, m, secs: d.secs });
      }
      return months.map((mo, i) => ({
        secs: mo.secs,
        label: new Date(mo.y, mo.m, 1).toLocaleDateString(loc, { month: "narrow" }),
        now: i === months.length - 1,
      }));
    }
    return rangeDays.map((d, i) => ({
      secs: d.secs,
      label:
        range === "week"
          ? d.date.toLocaleDateString(loc, { weekday: "narrow" })
          : String(d.date.getDate()),
      now: i === rangeDays.length - 1,
    }));
  });

  // --- datos curiosos del rango ------------------------------------------
  const facts = $derived.by(() => {
    const days = rangeDays;
    const totalSecs = days.reduce((a, d) => a + d.secs, 0);
    const active = days.filter((d) => d.secs > 0).length;
    let longest = 0;
    let run = 0;
    for (const d of days) {
      if (d.secs > 0) {
        run += 1;
        longest = Math.max(longest, run);
      } else run = 0;
    }
    // The most-played game and the distinct games, within the range.
    const bySlug: Record<string, number> = {};
    for (const d of days) {
      for (const [slug, secs] of Object.entries(dailyByGame[d.key] ?? {})) {
        bySlug[slug] = (bySlug[slug] ?? 0) + secs;
      }
    }
    let topSlug: string | null = null;
    let topSecs = 0;
    for (const [slug, secs] of Object.entries(bySlug)) {
      if (secs > topSecs) {
        topSecs = secs;
        topSlug = slug;
      }
    }
    return {
      totalSecs,
      active,
      longest,
      played: Object.values(bySlug).filter((s) => s > 0).length,
      topSlug,
      topSecs,
    };
  });

  function prettySlug(slug: string): string {
    return (
      slug
        .split(/[-_]+/)
        .filter(Boolean)
        .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
        .join(" ") || slug
    );
  }

  const topGameName = $derived(
    facts.topSlug ? (nameBySlug[facts.topSlug] ?? prettySlug(facts.topSlug)) : null,
  );

  // ---- accent
  // "app" follows Settings, live; otherwise the hue picked here, where `null`
  // is the stock emerald like Settings' first gem.
  const cardHue = $derived(prefs.accentMode === "app" ? $accentHue : prefs.accentHue);
  const palette = $derived(cardPalette(cardHue, ACCENT_STOPS));
  const swatch = (hue: number | null) => cardPalette(hue, ACCENT_STOPS).a400;

  function onCardHue(e: Event): void {
    const v = Number((e.currentTarget as HTMLInputElement).value);
    if (Number.isFinite(v)) setCardAccent(v);
  }

  const rangeLabel = $derived(
    range === "week"
      ? $_("wrapped.card_last_7_days")
      : range === "month"
        ? $_("wrapped.card_last_30_days")
        : $_("wrapped.card_last_365_days"),
  );

  /** The phrase: the one the user wrote, or the dice's, based on the game. */
  const quote = $derived.by(() => {
    void loc; // el idioma activo forma parte del resultado
    const own = prefs.quote.trim();
    if (own) return own;
    return $_(pickQuote(facts.topSlug, prefs.seed + (facts.topSlug?.length ?? 0)));
  });

  /** The card is made to be shown, so the default name is never the full email
   *  address: when the session only gives us the email, we keep what is before the
   *  at sign. The recap masks the address for a reason. */
  const suggestedName = $derived(sessionName.trim().split("@")[0].trim());

  const displayName = $derived(
    prefs.name.trim() ||
      suggestedName ||
      $_("wrapped.card_player"),
  );

  const initials = $derived(
    displayName
      .split(/[\s._@-]+/)
      .filter(Boolean)
      .slice(0, 2)
      .map((s) => s[0]?.toUpperCase() ?? "")
      .join("") || "?",
  );

  function fmtHours(secs: number): string {
    if (secs <= 0) return "0";
    const h = secs / 3600;
    if (h >= 10) return String(Math.round(h));
    return h.toFixed(1).replace(/\.0$/, "");
  }

  const cardData = $derived<CardData>({
    name: displayName,
    initials,
    avatar: photoImg,
    quote: prefs.showQuote ? quote : null,
    rangeLabel,
    cubes,
    stats: [
      {
        label: $_("wrapped.card_hours"),
        value: fmtHours(facts.totalSecs),
      },
      {
        label: $_("wrapped.card_active_days"),
        value: String(facts.active),
      },
      {
        label: $_("wrapped.card_streak"),
        value: String(facts.longest),
      },
      {
        label: $_("wrapped.card_games"),
        value: String(facts.played || totalGames),
      },
      {
        label: $_("wrapped.card_hoarded"),
        value: fmtBytes(hoardedBytes),
      },
    ],
    topGame: topGameName ? { label: topGameName, cover: coverImg } : null,
    topGameLabel: $_("wrapped.card_most_played"),
    cubesLabel: $_("wrapped.card_activity"),
    palette,
  });

  // ---- images
  // The local picture beats the account's avatar. The remote one is fetched with
  // CORS: if Google does not allow it, the image is dropped before it touches the
  // canvas (drawing it would taint the canvas and the save would fail).
  $effect(() => {
    const local = cardPhotoUrl();
    const remote = sessionAvatar;
    let cancelled = false;
    void (async () => {
      const img = local
        ? await loadImage(local)
        : remote
          ? await loadImage(remote, true)
          : null;
      if (!cancelled) photoImg = img;
    })();
    return () => {
      cancelled = true;
    };
  });

  $effect(() => {
    const slug = facts.topSlug;
    const key = coverKey(slug ? (appIdBySlug[slug] ?? null) : null, slug);
    let cancelled = false;
    void (async () => {
      if (key == null) {
        if (!cancelled) coverImg = null;
        return;
      }
      // The card paints at 2x, and its cover frame is 52×74 (`cardCanvas.ts`).
      const url = await coverUrl(key, coverSize(74 * 2));
      const img = url ? await loadImage(url) : null;
      if (!cancelled) coverImg = img;
    })();
    return () => {
      cancelled = true;
    };
  });

  // Repintado: cualquier cambio en los datos vuelve a dibujar la tarjeta.
  $effect(() => {
    if (!canvas || !fontsReady) return;
    paint(canvas, cardData, 2);
  });

  onMount(() => {
    void hydrateCardPrefs();
    void waitForFonts().then(() => (fontsReady = true));
  });

  async function choosePhoto() {
    if (busyPhoto) return;
    busyPhoto = true;
    try {
      const picked = await openDialog({
        multiple: false,
        directory: false,
        filters: [
          {
            name: $_("wrapped.card_images"),
            extensions: ["jpg", "jpeg", "png", "webp", "gif", "bmp"],
          },
        ],
      });
      if (typeof picked === "string") await setCardPhotoFromPath(picked);
    } catch (e) {
      toastError(String(e));
    } finally {
      busyPhoto = false;
    }
  }

  async function dropPhoto() {
    try {
      await clearCardPhoto();
    } catch (e) {
      toastError(String(e));
    }
  }

  /** "Take the picture": renders at 2x and leaves it in the system gallery. */
  async function shoot() {
    if (saving) return;
    saving = true;
    try {
      const png = renderToPng(cardData, 2);
      const path = await saveCardToGallery(png, topGameName);
      toastSuccess(
        $_("wrapped.card_saved_to", { values: { path } }),
      );
    } catch (e) {
      toastError(String(e));
    } finally {
      saving = false;
    }
  }

  const RANGES: { key: CardRange; label: string }[] = $derived([
    { key: "week", label: $_("wrapped.card_week") },
    { key: "month", label: $_("wrapped.card_month") },
    { key: "year", label: $_("wrapped.card_year") },
  ]);
</script>

<section class="mt-4 overflow-hidden rounded-2xl border border-emerald-400/20 bg-layer-1">
  <div
    class="flex items-center justify-between gap-3 border-b border-white/[0.08] px-4 py-3"
  >
    <div class="flex min-w-0 items-center gap-2.5">
      <Camera size={17} class="shrink-0 text-emerald-300" data-anim="pop" />
      <div class="min-w-0">
        <h3 class="truncate text-sm font-semibold text-zinc-50">
          {$_("wrapped.card_your_card")}
        </h3>
        <p class="flex items-center gap-1 text-[11px] text-zinc-500">
          <Lock size={10} />
          {$_("wrapped.card_local_only")}
        </p>
      </div>
    </div>
    <button
      type="button"
      onclick={onClose}
      class="grid h-7 w-7 place-items-center rounded-lg border border-white/[0.08] text-zinc-400 transition hover:border-white/25"
      aria-label={$_("wrapped.close")}
    >
      <X size={15} data-anim="pop" />
    </button>
  </div>

  <div class="p-4">
    <!-- La vista previa ES el PNG que se guarda. -->
    <div class="overflow-hidden rounded-2xl ring-1 ring-white/[0.08]">
      <canvas
        bind:this={canvas}
        width={CARD_W * 2}
        height={CARD_H * 2}
        class="block w-full"
        style="aspect-ratio: {CARD_W} / {CARD_H}"
        aria-label="{displayName} · {rangeLabel} · {fmtHours(facts.totalSecs)} h"
      ></canvas>
    </div>

    <!-- Every button here moves its icon on hover. -->
    <div class="mt-4 grid gap-3 sm:grid-cols-2">
      <div class="rounded-2xl border border-white/[0.08] bg-layer-2 p-3">
        <div class="mb-2 text-[11px] uppercase tracking-wide text-zinc-500">
          {$_("wrapped.card_photo")}
        </div>
        <div class="flex items-center gap-2">
          <button
            type="button"
            onclick={choosePhoto}
            disabled={busyPhoto}
            class="inline-flex items-center gap-1.5 rounded-lg border border-white/[0.08] px-3 py-1.5 text-xs font-medium text-zinc-200 transition hover:border-white/25 disabled:opacity-50"
          >
            {#if busyPhoto}<Loader2 size={13} class="animate-spin" />{:else}<ImagePlus size={13} data-anim="pop" />{/if}
            {$_("wrapped.card_choose_photo")}
          </button>
          {#if cardPhotoUrl()}
            <button
              type="button"
              onclick={dropPhoto}
              class="inline-flex items-center gap-1.5 rounded-lg border border-white/[0.08] px-3 py-1.5 text-xs text-zinc-300 transition hover:border-white/25"
            >
              <Trash2 size={13} data-anim="pop" />
              {$_("wrapped.card_remove")}
            </button>
          {/if}
        </div>
      </div>

      <div class="rounded-2xl border border-white/[0.08] bg-layer-2 p-3">
        <div class="mb-2 text-[11px] uppercase tracking-wide text-zinc-500">
          {$_("wrapped.card_name")}
        </div>
        <input
          type="text"
          value={prefs.name}
          maxlength="40"
          oninput={(e) => setCardName(e.currentTarget.value)}
          placeholder={suggestedName}
          class="w-full rounded-lg border border-white/[0.08] bg-layer-2 px-3 py-1.5 text-sm text-zinc-100 outline-none transition focus:border-emerald-500/50"
        />
      </div>

      <div class="rounded-2xl border border-white/[0.08] bg-layer-2 p-3">
        <div class="mb-2 text-[11px] uppercase tracking-wide text-zinc-500">
          {$_("wrapped.card_line")}
        </div>
        <div class="flex items-center gap-2">
          <input
            type="text"
            value={prefs.quote}
            maxlength="140"
            disabled={!prefs.showQuote}
            oninput={(e) => setCardQuote(e.currentTarget.value)}
            placeholder={quote}
            class="min-w-0 flex-1 rounded-lg border border-white/[0.08] bg-layer-2 px-3 py-1.5 text-sm text-zinc-100 outline-none transition focus:border-emerald-500/50 disabled:opacity-40"
          />
          <button
            type="button"
            onclick={rerollQuote}
            disabled={!prefs.showQuote}
            class="grid h-8 w-8 shrink-0 place-items-center rounded-lg border border-white/[0.08] text-zinc-300 transition hover:border-white/25 disabled:opacity-40"
            title={$_("wrapped.card_another_line")}
            aria-label={$_("wrapped.card_another_line")}
          >
            <Dices size={15} data-anim="spin" />
          </button>
          {#if prefs.showQuote}
            <button
              type="button"
              onclick={() => setCardQuoteVisible(false)}
              class="grid h-8 w-8 shrink-0 place-items-center rounded-lg border border-white/[0.08] text-zinc-300 transition hover:border-white/25"
              title={$_("wrapped.card_hide_line")}
              aria-label={$_("wrapped.card_hide_line")}
            >
              <Trash2 size={15} data-anim="pop" />
            </button>
          {:else}
            <button
              type="button"
              onclick={() => setCardQuoteVisible(true)}
              class="grid h-8 w-8 shrink-0 place-items-center rounded-lg border border-white/[0.08] text-zinc-300 transition hover:border-white/25"
              title={$_("wrapped.card_show_line")}
              aria-label={$_("wrapped.card_show_line")}
            >
              <Eye size={15} data-anim="pop" />
            </button>
          {/if}
        </div>
      </div>

      <div class="rounded-2xl border border-white/[0.08] bg-layer-2 p-3">
        <div class="mb-2 text-[11px] uppercase tracking-wide text-zinc-500">
          {$_("wrapped.card_what_to_show")}
        </div>
        <div class="flex gap-1 rounded-lg border border-white/[0.08] bg-layer-2 p-1">
          {#each RANGES as r (r.key)}
            <button
              type="button"
              onclick={() => setCardRange(r.key)}
              class="flex-1 rounded-md border px-2.5 py-1 text-xs font-medium transition {range === r.key
                ? 'border-transparent bg-emerald-600/20 text-emerald-300 ring-1 ring-inset ring-emerald-600/40'
                : 'border-transparent text-zinc-400 hover:border-white/20'}"
            >
              <span class="inline-block" data-anim="pop">{r.label}</span>
            </button>
          {/each}
        </div>
      </div>

      <!-- The card's own accent, kept on this machine: Settings' by default,
           or a gem (or any hue on the slider) just for the card. -->
      <div class="rounded-2xl border border-white/[0.08] bg-layer-2 p-3 sm:col-span-2">
        <div class="mb-2 text-[11px] uppercase tracking-wide text-zinc-500">
          {$_("settings.accent_label")}
        </div>
        <div class="flex flex-wrap items-center gap-2">
          <button
            type="button"
            onclick={() => setCardAccent("app")}
            aria-pressed={prefs.accentMode === "app"}
            class="inline-flex h-8 items-center gap-1.5 rounded-lg border px-2.5 text-xs transition {prefs.accentMode === 'app'
              ? 'border-[var(--color-accent)]/60 text-zinc-100'
              : 'border-white/[0.08] text-zinc-400 hover:border-white/25'}"
          >
            <span
              class="h-3.5 w-3.5 rounded-[3px]"
              style="background: {swatch($accentHue)}"
              data-anim="pop"
            ></span>
            {$_("wrapped.card_same_as_the_app")}
          </button>
          {#each gems as g (g.id)}
            {@const active = prefs.accentMode === "custom" && prefs.accentHue === g.hue}
            <button
              type="button"
              onclick={() => setCardAccent(g.hue)}
              aria-pressed={active}
              title={$_(g.labelKey)}
              aria-label={$_(g.labelKey)}
              class="grid h-8 w-8 place-items-center rounded-lg border transition {active
                ? 'border-[var(--color-accent)]/60'
                : 'border-white/[0.08] hover:border-white/25'}"
            >
              <span
                class="h-4 w-4 rounded-[3px]"
                style="background: {swatch(g.hue)}"
                data-anim="pop"
              ></span>
            </button>
          {/each}
        </div>
        <input
          type="range"
          min="0"
          max="359"
          step="1"
          value={cardHue ?? 160}
          oninput={onCardHue}
          class="hue-slider mt-3 w-full"
          style="--color-accent: {palette.accent}"
          aria-label={$_("settings.accent_label")}
        />
      </div>
    </div>

    <button
      type="button"
      onclick={shoot}
      disabled={saving}
      class="mt-3 flex w-full items-center justify-center gap-2 rounded-2xl border border-white/[0.08] bg-layer-2 px-4 py-3 text-sm font-semibold text-zinc-100 transition hover:border-emerald-400/30 disabled:opacity-60"
    >
      {#if saving}<Loader2 size={16} class="animate-spin" />{:else}<Camera size={16} class="text-emerald-300" data-anim="pop" />{/if}
      {$_("wrapped.card_save")}
    </button>
  </div>
</section>
