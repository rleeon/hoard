<script lang="ts">
  /**
   * La tarjeta de Hoard-Wrapped: el mismo resumen, pero en una imagen que se
   * puede enseñar. Foto, nombre, una frase con guasa según el juego más
   * jugado, cuatro datos curiosos y una fila de cubitos con el rango elegido
   * (semana → siete cuadros grandes, mes → un cubo por día, año → uno por mes).
   *
   * Dos decisiones que conviene no deshacer sin pensarlo:
   *
   * 1. **Todo lo editable es local.** La foto es un PNG bajo el app-data dir y
   *    el resto vive en un `store` de este equipo, no se sube, no se
   *    sincroniza, no sale en el export de la cuenta. Ver `cardPrefs`.
   * 2. **Lo que se ve ES el canvas que se guarda.** La vista previa no es una
   *    maqueta HTML parecida al PNG: es el PNG, dibujado a 1× en pantalla y a
   *    2× al guardar (`cardCanvas`). Así no hay dos diseños que mantener ni
   *    sorpresas al compartir.
   *
   * La imagen lleva la marca y `hoard.services` a la vista y también en los
   * metadatos PNG que escribe Rust: si alguien la sube a cualquier sitio, va
   * firmada.
   */
  import { onMount } from "svelte";
  import { locale } from "svelte-i18n";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import {
    Camera,
    ImagePlus,
    Trash2,
    Dices,
    X,
    Lock,
    Loader2,
  } from "@lucide/svelte";
  import { tr, fmtBytes } from "./lib";
  import { pickQuote } from "./phrases";
  import {
    cardPrefs,
    cardPhotoUrl,
    hydrateCardPrefs,
    setCardName,
    setCardQuote,
    setCardRange,
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
    type CardData,
    type Cube,
  } from "./cardCanvas";
  import { coverKey, coverUrl } from "../stores/covers";
  import { toastError, toastSuccess } from "../stores/toasts";

  let {
    /** Segundos jugados por día, `YYYY-MM-DD` → segundos. */
    daysByKey = {},
    /** Desglose por día y juego, para saber a qué se jugó en el rango. */
    dailyByGame = {},
    /** slug → app id de Steam, solo para la carátula. */
    appIdBySlug = {},
    /** Nombre de la sesión: el que se usa mientras no escribas otro. */
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
    sessionName?: string;
    sessionAvatar?: string | null;
    totalGames?: number;
    hoardedBytes?: number;
    onClose: () => void;
  } = $props();

  const DAY_MS = 86_400_000;

  let canvas = $state<HTMLCanvasElement | null>(null);
  let photoImg = $state<HTMLImageElement | null>(null);
  let coverImg = $state<HTMLImageElement | null>(null);
  let fontsReady = $state(false);
  let saving = $state(false);
  let busyPhoto = $state(false);

  const prefs = $derived(cardPrefs());
  const range = $derived(prefs.range);

  // --- rango → días -------------------------------------------------------
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

  /** Los días que entran en el rango, del más antiguo al de hoy. */
  const rangeDays = $derived.by(() => {
    const today = startOfToday();
    const span = range === "week" ? 7 : range === "month" ? 30 : 365;
    return Array.from({ length: span }, (_, i) => {
      const d = new Date(today.getTime() - (span - 1 - i) * DAY_MS);
      return { date: d, key: dayKey(d), secs: daysByKey[dayKey(d)] || 0 };
    });
  });

  const loc = $derived($locale ?? "en");

  /** Los cubitos. El número depende del rango: 7 días, 30 días o 12 meses. */
  const cubes = $derived.by<Cube[]>(() => {
    const today = startOfToday();
    if (range === "year") {
      // Doce meses: el actual y los once anteriores.
      const out: Cube[] = [];
      for (let i = 11; i >= 0; i--) {
        const m = new Date(today.getFullYear(), today.getMonth() - i, 1);
        const prefix = `${m.getFullYear()}-${String(m.getMonth() + 1).padStart(2, "0")}-`;
        let secs = 0;
        for (const [k, v] of Object.entries(daysByKey)) {
          if (k.startsWith(prefix)) secs += v;
        }
        out.push({
          secs,
          label: m.toLocaleDateString(loc, { month: "narrow" }),
          now: i === 0,
        });
      }
      return out;
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
    // Juego más jugado y juegos distintos, dentro del rango.
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

  const topGameName = $derived(facts.topSlug ? prettySlug(facts.topSlug) : null);

  const rangeLabel = $derived(
    range === "week"
      ? tr({ es: "Últimos 7 días", en: "Last 7 days", de: "Letzte 7 Tage", fr: "7 derniers jours", it: "Ultimi 7 giorni", ja: "直近7日間", pt: "Últimos 7 dias", zh: "最近 7 天" })
      : range === "month"
        ? tr({ es: "Últimos 30 días", en: "Last 30 days", de: "Letzte 30 Tage", fr: "30 derniers jours", it: "Ultimi 30 giorni", ja: "直近30日間", pt: "Últimos 30 dias", zh: "最近 30 天" })
        : tr({ es: "Último año", en: "Last year", de: "Letztes Jahr", fr: "Cette année", it: "Ultimo anno", ja: "この1年", pt: "Último ano", zh: "最近一年" }),
  );

  /** La frase: la que escribió el usuario, o la del dado según el juego. */
  const quote = $derived.by(() => {
    void loc; // el idioma activo forma parte del resultado
    const own = prefs.quote.trim();
    if (own) return own;
    return tr(pickQuote(facts.topSlug, prefs.seed + (facts.topSlug?.length ?? 0)));
  });

  /** La tarjeta se hace para enseñarla, así que el nombre por defecto nunca
   *  es el correo entero: si la sesión solo nos da el email, nos quedamos con
   *  lo de delante de la arroba. El resumen enmascara el correo por algo. */
  const suggestedName = $derived(sessionName.trim().split("@")[0].trim());

  const displayName = $derived(
    prefs.name.trim() ||
      suggestedName ||
      tr({ es: "Jugador", en: "Player", de: "Spieler", fr: "Joueur", it: "Giocatore", ja: "プレイヤー", pt: "Jogador", zh: "玩家" }),
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
    quote,
    rangeLabel,
    cubes,
    stats: [
      {
        label: tr({ es: "Horas", en: "Hours", de: "Stunden", fr: "Heures", it: "Ore", ja: "時間", pt: "Horas", zh: "小时" }),
        value: fmtHours(facts.totalSecs),
      },
      {
        label: tr({ es: "Días activos", en: "Active days", de: "Aktive Tage", fr: "Jours actifs", it: "Giorni attivi", ja: "プレイ日数", pt: "Dias ativos", zh: "活跃天数" }),
        value: String(facts.active),
      },
      {
        label: tr({ es: "Racha", en: "Streak", de: "Serie", fr: "Série", it: "Serie", ja: "連続記録", pt: "Sequência", zh: "连续天数" }),
        value: String(facts.longest),
      },
      {
        label: tr({ es: "Juegos", en: "Games", de: "Spiele", fr: "Jeux", it: "Giochi", ja: "ゲーム数", pt: "Jogos", zh: "游戏数" }),
        value: String(facts.played || totalGames),
      },
      {
        label: tr({ es: "Atesorado", en: "Hoarded", de: "Gehortet", fr: "Amassé", it: "Accumulato", ja: "保管量", pt: "Guardado", zh: "已收藏" }),
        value: fmtBytes(hoardedBytes),
      },
    ],
    topGame: topGameName ? { label: topGameName, cover: coverImg } : null,
    topGameLabel: tr({ es: "Más jugado", en: "Most played", de: "Meistgespielt", fr: "Le plus joué", it: "Più giocato", ja: "最多プレイ", pt: "Mais jogado", zh: "玩得最多" }),
    cubesLabel: tr({ es: "Actividad", en: "Activity", de: "Aktivität", fr: "Activité", it: "Attività", ja: "アクティビティ", pt: "Atividade", zh: "活跃度" }),
    tagline: tr({
      es: "Copias automáticas de tus partidas",
      en: "Automatic backups for your game saves",
      de: "Automatische Backups für deine Spielstände",
      fr: "Sauvegardes automatiques de tes parties",
      it: "Backup automatici dei tuoi salvataggi",
      ja: "セーブデータの自動バックアップ",
      pt: "Backups automáticos dos seus saves",
      zh: "游戏存档自动备份",
    }),
  });

  // --- imágenes -----------------------------------------------------------
  // La foto local manda sobre el avatar de la cuenta. El remoto se pide con
  // CORS: si Google no lo permite, la imagen se descarta antes de tocar el
  // canvas (dibujarla lo dejaría "tainted" y el guardado fallaría).
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
      const url = await coverUrl(key);
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
            name: tr({ es: "Imágenes", en: "Images", de: "Bilder", fr: "Images", it: "Immagini", ja: "画像", pt: "Imagens", zh: "图片" }),
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

  /** "Sacar la foto": renderiza a 2× y la deja en la galería del sistema. */
  async function shoot() {
    if (saving) return;
    saving = true;
    try {
      const png = renderToPng(cardData, 2);
      const path = await saveCardToGallery(png, topGameName);
      toastSuccess(
        tr({
          es: `Guardada en ${path}`,
          en: `Saved to ${path}`,
          de: `Gespeichert unter ${path}`,
          fr: `Enregistrée dans ${path}`,
          it: `Salvata in ${path}`,
          ja: `${path} に保存しました`,
          pt: `Salva em ${path}`,
          zh: `已保存至 ${path}`,
        }),
      );
    } catch (e) {
      toastError(String(e));
    } finally {
      saving = false;
    }
  }

  const RANGES: { key: CardRange; label: string }[] = $derived([
    { key: "week", label: tr({ es: "Semana", en: "Week", de: "Woche", fr: "Semaine", it: "Settimana", ja: "1週間", pt: "Semana", zh: "一周" }) },
    { key: "month", label: tr({ es: "Mes", en: "Month", de: "Monat", fr: "Mois", it: "Mese", ja: "1か月", pt: "Mês", zh: "一个月" }) },
    { key: "year", label: tr({ es: "Año", en: "Year", de: "Jahr", fr: "Année", it: "Anno", ja: "1年", pt: "Ano", zh: "一年" }) },
  ]);
</script>

<section
  class="mt-4 overflow-hidden rounded-2xl border border-emerald-400/20 bg-zinc-900/60 shadow-[0_8px_30px_-12px_rgba(16,185,129,0.25)]"
>
  <!-- cabecera -->
  <div
    class="relative flex items-center justify-between gap-3 border-b border-white/[0.08] bg-gradient-to-r from-emerald-500/15 via-emerald-500/[0.04] to-transparent px-4 py-3"
  >
    <div class="flex min-w-0 items-center gap-2.5">
      <div
        class="grid h-9 w-9 shrink-0 place-items-center rounded-xl bg-emerald-500/15 ring-1 ring-emerald-400/30"
      >
        <Camera size={17} class="text-emerald-300" />
      </div>
      <div class="min-w-0">
        <h3 class="truncate text-sm font-semibold text-zinc-50">
          {tr({ es: "Tu tarjeta", en: "Your card", de: "Deine Karte", fr: "Ta carte", it: "La tua card", ja: "あなたのカード", pt: "Seu card", zh: "你的卡片" })}
        </h3>
        <p class="flex items-center gap-1 text-[11px] text-zinc-500">
          <Lock size={10} />
          {tr({
            es: "Foto y nombre solo en este equipo",
            en: "Photo and name stay on this device",
            de: "Foto und Name bleiben auf diesem Gerät",
            fr: "Photo et nom restent sur cet appareil",
            it: "Foto e nome restano su questo dispositivo",
            ja: "写真と名前はこの端末だけに保存されます",
            pt: "Foto e nome ficam só neste dispositivo",
            zh: "照片和名称仅保存在本机",
          })}
        </p>
      </div>
    </div>
    <button
      type="button"
      onclick={onClose}
      class="grid h-7 w-7 place-items-center rounded-lg border border-white/10 text-zinc-400 transition hover:bg-white/5 hover:text-white"
      aria-label={tr({ es: "Cerrar", en: "Close" })}
    >
      <X size={15} />
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

    <!-- controles -->
    <div class="mt-4 grid gap-3 sm:grid-cols-2">
      <!-- foto -->
      <div class="rounded-xl bg-white/[0.03] p-3 ring-1 ring-white/[0.05]">
        <div class="mb-2 text-[11px] uppercase tracking-wide text-zinc-500">
          {tr({ es: "Foto", en: "Photo", de: "Foto", fr: "Photo", it: "Foto", ja: "写真", pt: "Foto", zh: "照片" })}
        </div>
        <div class="flex items-center gap-2">
          <button
            type="button"
            onclick={choosePhoto}
            disabled={busyPhoto}
            class="inline-flex items-center gap-1.5 rounded-lg bg-emerald-600 px-3 py-1.5 text-xs font-medium text-white transition hover:bg-emerald-500 disabled:opacity-50"
          >
            {#if busyPhoto}<Loader2 size={13} class="animate-spin" />{:else}<ImagePlus size={13} />{/if}
            {tr({ es: "Elegir foto", en: "Choose photo", de: "Foto wählen", fr: "Choisir une photo", it: "Scegli foto", ja: "写真を選ぶ", pt: "Escolher foto", zh: "选择照片" })}
          </button>
          {#if cardPhotoUrl()}
            <button
              type="button"
              onclick={dropPhoto}
              class="inline-flex items-center gap-1.5 rounded-lg border border-white/10 px-3 py-1.5 text-xs text-zinc-300 transition hover:bg-white/5"
            >
              <Trash2 size={13} />
              {tr({ es: "Quitar", en: "Remove", de: "Entfernen", fr: "Retirer", it: "Rimuovi", ja: "削除", pt: "Remover", zh: "移除" })}
            </button>
          {/if}
        </div>
      </div>

      <!-- nombre -->
      <div class="rounded-xl bg-white/[0.03] p-3 ring-1 ring-white/[0.05]">
        <div class="mb-2 text-[11px] uppercase tracking-wide text-zinc-500">
          {tr({ es: "Nombre", en: "Name", de: "Name", fr: "Nom", it: "Nome", ja: "名前", pt: "Nome", zh: "名称" })}
        </div>
        <input
          type="text"
          value={prefs.name}
          maxlength="40"
          oninput={(e) => setCardName(e.currentTarget.value)}
          placeholder={suggestedName}
          class="w-full rounded-lg border border-white/10 bg-zinc-950/50 px-3 py-1.5 text-sm text-zinc-100 outline-none transition focus:border-emerald-500/50"
        />
      </div>

      <!-- frase -->
      <div class="rounded-xl bg-white/[0.03] p-3 ring-1 ring-white/[0.05]">
        <div class="mb-2 text-[11px] uppercase tracking-wide text-zinc-500">
          {tr({ es: "Frase", en: "Line", de: "Spruch", fr: "Phrase", it: "Frase", ja: "ひとこと", pt: "Frase", zh: "标语" })}
        </div>
        <div class="flex items-center gap-2">
          <input
            type="text"
            value={prefs.quote}
            maxlength="140"
            oninput={(e) => setCardQuote(e.currentTarget.value)}
            placeholder={quote}
            class="min-w-0 flex-1 rounded-lg border border-white/10 bg-zinc-950/50 px-3 py-1.5 text-sm text-zinc-100 outline-none transition focus:border-emerald-500/50"
          />
          <button
            type="button"
            onclick={rerollQuote}
            class="grid h-8 w-8 shrink-0 place-items-center rounded-lg border border-white/10 text-zinc-300 transition hover:bg-white/5 hover:text-emerald-300"
            title={tr({ es: "Otra frase", en: "Another line", de: "Anderer Spruch", fr: "Autre phrase", it: "Un'altra frase", ja: "別のひとこと", pt: "Outra frase", zh: "换一句" })}
            aria-label={tr({ es: "Otra frase", en: "Another line" })}
          >
            <Dices size={15} />
          </button>
        </div>
      </div>

      <!-- rango -->
      <div class="rounded-xl bg-white/[0.03] p-3 ring-1 ring-white/[0.05]">
        <div class="mb-2 text-[11px] uppercase tracking-wide text-zinc-500">
          {tr({ es: "Qué se muestra", en: "What to show", de: "Was gezeigt wird", fr: "Ce qui s'affiche", it: "Cosa mostrare", ja: "表示する期間", pt: "O que mostrar", zh: "显示范围" })}
        </div>
        <div class="flex gap-1 rounded-lg border border-white/[0.08] bg-zinc-950/40 p-1">
          {#each RANGES as r (r.key)}
            <button
              type="button"
              onclick={() => setCardRange(r.key)}
              class="flex-1 rounded-md px-2.5 py-1 text-xs font-medium transition {range === r.key
                ? 'bg-emerald-600/20 text-emerald-300 ring-1 ring-inset ring-emerald-600/40'
                : 'text-zinc-400 hover:text-zinc-200'}"
            >
              {r.label}
            </button>
          {/each}
        </div>
      </div>
    </div>

    <!-- sacar la foto -->
    <button
      type="button"
      onclick={shoot}
      disabled={saving}
      class="mt-3 flex w-full items-center justify-center gap-2 rounded-xl bg-emerald-600 px-4 py-3 text-sm font-semibold text-white transition hover:bg-emerald-500 disabled:opacity-60"
    >
      {#if saving}<Loader2 size={16} class="animate-spin" />{:else}<Camera size={16} />{/if}
      {tr({
        es: "Sacar la foto y guardarla en la galería",
        en: "Take the photo and save it to your gallery",
        de: "Foto machen und in der Galerie speichern",
        fr: "Prendre la photo et l'enregistrer dans la galerie",
        it: "Scatta la foto e salvala nella galleria",
        ja: "写真を撮ってギャラリーに保存",
        pt: "Tirar a foto e salvar na galeria",
        zh: "拍照并保存到图库",
      })}
    </button>
  </div>
</section>
