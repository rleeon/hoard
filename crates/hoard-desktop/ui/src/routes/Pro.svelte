<script lang="ts">
  /**
   * The Pro screen, where every padlock in the application leads.
   *
   * Pressing a locked item (Hoard-Screen in the rail, `ProGate`'s CTA, the trial
   * strip, the quota-full dialog) used to call `openUpgradePage`, which opens
   * `hoard.services/pricing` in the system browser. That throws the user out of the
   * application without having told them what they are buying: you press a menu item
   * and a browser opens on top of the game.
   *
   * The padlock now brings you here. This screen explains what Pro includes,
   * compares the two plans, and puts the payment (which does have to happen outside,
   * because the gateway is Polar and cannot be embedded) behind an explicit button
   * that also warns the browser will open. The jump outside stops being a surprise
   * and becomes a decision.
   *
   * `?feature=screen` (set by whoever brings us here) only serves to head the page
   * naming the feature the user was trying to open.
   */
  import { onMount } from "svelte";
  import { push, router } from "svelte-spa-router";
  import { _ } from "svelte-i18n";
  import {
    MonitorPlay,
    Sparkles,
    HardDrive,
    Laptop,
    Check,
    ArrowUpRight,
    Lock,
  } from "@lucide/svelte";

  import Button from "../lib/components/Button.svelte";
  import {
    cloud,
    hydrateCloud,
    openUpgradePage,
    openBillingPortal,
  } from "../lib/stores/cloud";
  import {
    entitlements,
    refreshEntitlements,
    activateFeature,
    featureDaysLeft,
    type FeatureKey,
  } from "../lib/stores/entitlements";

  onMount(async () => {
    if (!$cloud.hydrated) await hydrateCloud();
    await refreshEntitlements();
  });

  /** The feature that brought the user here, when they came from a specific padlock. */
  const fromFeature = $derived.by<FeatureKey | null>(() => {
    const qs = router.querystring;
    if (!qs) return null;
    const f = new URLSearchParams(qs).get("feature");
    return f === "screen" || f === "wrapple" ? f : null;
  });

  const featureLabel = $derived(
    fromFeature === "screen"
      ? $_("nav.hoard_screen")
      : fromFeature === "wrapple"
        ? $_("nav.hoard_wrapped")
        : null,
  );

  const account = $derived($cloud.account);
  const isPro = $derived(account?.plan === "pro");

  // An unstarted trial is the cheapest road for the user: if the feature they were
  // after still offers one, the main button starts it instead of sending them to
  // pay.
  const pending = $derived.by(() => {
    if (!fromFeature) return null;
    const fs = $entitlements?.features[fromFeature] ?? null;
    return fs?.state === "trial_available" ? fs : null;
  });

  let startingTrial = $state(false);
  async function startTrial() {
    if (!fromFeature || startingTrial) return;
    startingTrial = true;
    try {
      await activateFeature(fromFeature);
      // The trial is already running: send them back to the feature they wanted.
      push(fromFeature === "screen" ? "/hoard-screen" : "/hoard-wrapped");
    } finally {
      startingTrial = false;
    }
  }

  const perks = $derived([
    {
      icon: MonitorPlay,
      title: $_("nav.hoard_screen"),
      body: $_("upgrade.pro_feat_screen"),
    },
    {
      icon: Sparkles,
      title: $_("nav.hoard_wrapped"),
      body: $_("upgrade.pro_feat_wrapped"),
    },
    {
      icon: HardDrive,
      title: $_("upgrade.pro_feat_storage"),
      body: $_("upgrade.pro_feat_max_save"),
    },
    {
      icon: Laptop,
      title: $_("upgrade.pro_feat_devices"),
      body: $_("upgrade.pro_feat_history"),
    },
  ]);

  const freeFeatures = $derived([
    $_("upgrade.free_feat_storage"),
    $_("upgrade.free_feat_devices"),
    $_("upgrade.free_feat_sync"),
    $_("upgrade.free_feat_history"),
    $_("upgrade.free_feat_export"),
  ]);

  const proFeatures = $derived([
    $_("upgrade.pro_feat_wrapped"),
    $_("upgrade.pro_feat_screen"),
    $_("upgrade.pro_feat_storage"),
    $_("upgrade.pro_feat_devices"),
    $_("upgrade.pro_feat_max_save"),
  ]);
</script>

<div class="mx-auto max-w-3xl px-6 py-8 pr-24">
  <!-- ── Encabezado ─────────────────────────────────────────────── -->
  <div class="flex items-start gap-4">
    <div
      class="flex h-14 w-14 shrink-0 items-center justify-center rounded-2xl bg-emerald-500/10 text-emerald-400 ring-1 ring-emerald-500/30"
    >
      <Sparkles size={26} />
    </div>
    <div class="min-w-0 flex-1">
      <h1
        class="font-display text-[28px] font-semibold leading-tight tracking-[-0.02em] text-zinc-50"
      >
        {$_("pro_page.title")}
      </h1>
      <p class="mt-1 text-sm text-zinc-400">{$_("pro_page.subtitle")}</p>
    </div>
  </div>

  {#if featureLabel && !isPro}
    <!-- De dónde viene. Nombra la función en vez de dejarlo en un genérico
         "función Pro", que es lo que decía el candado anterior. -->
    <p
      class="panel mt-5 flex items-center gap-2 px-3.5 py-2.5 text-sm text-zinc-300"
    >
      <Lock size={14} class="shrink-0 text-zinc-500" />
      {$_("pro_page.locked_context", { values: { feature: featureLabel } })}
    </p>
  {/if}

  {#if isPro}
    <p
      class="mt-5 flex items-center gap-2 rounded-lg border border-emerald-500/30 bg-emerald-500/10 px-3.5 py-2.5 text-sm text-emerald-300"
    >
      <Check size={14} class="shrink-0" />
      {$_("pro_page.current_pro")}
    </p>
  {/if}

  <!-- ── Qué incluye ────────────────────────────────────────────── -->
  <h2 class="mt-8 text-xs font-semibold uppercase tracking-wider text-zinc-500">
    {$_("pro_page.whats_included")}
  </h2>
  <div class="mt-3 grid gap-2.5 sm:grid-cols-2">
    {#each perks as perk (perk.title)}
      <div
        class="panel flex items-start gap-3 p-4"
      >
        <perk.icon size={18} class="mt-0.5 shrink-0 text-emerald-400" />
        <div class="min-w-0">
          <p class="text-sm font-medium text-zinc-100">{perk.title}</p>
          <p class="mt-0.5 text-xs leading-relaxed text-zinc-400">{perk.body}</p>
        </div>
      </div>
    {/each}
  </div>

  <!-- ── Comparativa de planes ──────────────────────────────────── -->
  <h2 class="mt-8 text-xs font-semibold uppercase tracking-wider text-zinc-500">
    {$_("upgrade.title")}
  </h2>
  <div class="mt-3 grid gap-3 sm:grid-cols-2">
    <div
      class="panel flex flex-col gap-3 p-5"
    >
      <div class="flex items-baseline justify-between gap-2">
        <h3 class="text-sm font-semibold text-zinc-100">
          {$_("upgrade.free_name")}
        </h3>
        {#if account && !isPro}
          <span
            class="rounded-full bg-zinc-800 px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide text-zinc-300"
          >
            {$_("upgrade.current")}
          </span>
        {/if}
      </div>
      <p class="text-2xl font-semibold text-zinc-100">
        {$_("upgrade.free_price")}
      </p>
      <ul class="flex flex-col gap-1.5 text-xs text-zinc-400">
        {#each freeFeatures as feat (feat)}
          <li class="flex items-start gap-1.5">
            <Check size={12} class="mt-0.5 shrink-0 text-zinc-600" />
            <span>{feat}</span>
          </li>
        {/each}
      </ul>
    </div>

    <div
      class="panel flex flex-col gap-3 border-emerald-500/40 bg-emerald-500/5 p-5"
    >
      <div class="flex items-baseline justify-between gap-2">
        <h3
          class="flex items-center gap-1.5 text-sm font-semibold text-zinc-100"
        >
          <Sparkles size={13} class="text-emerald-400" />
          {$_("upgrade.pro_name")}
        </h3>
        {#if isPro}
          <span
            class="rounded-full bg-emerald-500/15 px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide text-emerald-300"
          >
            {$_("upgrade.current")}
          </span>
        {/if}
      </div>
      <p class="text-2xl font-semibold text-zinc-50">
        {$_("upgrade.pro_price")}
      </p>
      <ul class="flex flex-col gap-1.5 text-xs text-zinc-300">
        {#each proFeatures as feat (feat)}
          <li class="flex items-start gap-1.5">
            <Check size={12} class="mt-0.5 shrink-0 text-emerald-400" />
            <span>{feat}</span>
          </li>
        {/each}
      </ul>
    </div>
  </div>

  <!-- ── Acción ─────────────────────────────────────────────────── -->
  <!-- Tres caminos, uno solo visible a la vez: sin sesión hay que entrar
       primero; con una prueba sin estrenar lo barato es estrenarla; y si no,
       el pago. En los tres, la línea de debajo dice exactamente qué va a
       pasar antes de que pase. -->
  <div class="mt-8 flex flex-col items-start gap-2.5">
    {#if !account}
      <Button size="lg" onclick={() => push("/account")}>
        {$_("pro_page.signin_cta")}
      </Button>
      <p class="max-w-md text-xs leading-relaxed text-zinc-500">
        {$_("pro_page.signin_note")}
      </p>
    {:else if isPro}
      <Button variant="secondary" onclick={() => openBillingPortal()}>
        <ArrowUpRight size={15} />
        {$_("pro_page.manage_billing")}
      </Button>
    {:else if pending}
      <Button size="lg" loading={startingTrial} onclick={startTrial}>
        {$_("pro_page.trial_cta", {
          values: { n: featureDaysLeft(pending) },
        })}
      </Button>
      <p class="max-w-md text-xs leading-relaxed text-zinc-500">
        {$_("pro_page.trial_note")}
      </p>
    {:else}
      <Button size="lg" onclick={() => openUpgradePage("pro")}>
        <ArrowUpRight size={16} />
        {$_("pro_page.checkout_cta")}
      </Button>
      <p class="max-w-md text-xs leading-relaxed text-zinc-500">
        {$_("pro_page.checkout_note")}
      </p>
    {/if}

    <button
      type="button"
      onclick={() => push("/dashboard")}
      class="mt-1 rounded-md px-1 py-0.5 text-xs text-zinc-500 transition-colors hover:text-zinc-300"
    >
      {$_("pro_page.back")}
    </button>
  </div>
</div>
