<script lang="ts">
  /**
   * Side-by-side plan comparison used when a UI flow needs to nudge the
   * user to upgrade (e.g. a 402 from `/v1/cloud/saves`, a Settings click
   * on the "compare plans" link, or a sidebar chip on the Free tier).
   *
   * The two paid CTAs open the public marketing page with a `plan` query
   * param so the checkout can preselect Pro vs Pro+. Keeping the actual
   * checkout out-of-app means we don't need to embed Polar here.
   */
  import { _ } from "svelte-i18n";
  import { Sparkles, Check } from "@lucide/svelte";
  import Modal from "./Modal.svelte";
  import Button from "./Button.svelte";
  import { push } from "svelte-spa-router";
  import { cloud } from "../stores/cloud";

  type Props = {
    open: boolean;
    onClose: () => void;
    /** Optional context line, e.g. "Storage quota exceeded". Rendered
     *  above the plan grid in a subtle amber callout. */
    reason?: string | null;
  };

  let { open, onClose, reason = null }: Props = $props();

  const currentPlan = $derived($cloud.account?.plan ?? null);

  // El modal cierra y lleva a `/pro`, donde está la explicación completa y el
  // botón de pago con su aviso. Antes saltaba directo al navegador desde un
  // diálogo que el usuario no había ido a buscar (lo dispara un 402 del
  // servidor), que es la forma más brusca de sacarlo de la aplicación.
  function pick(_plan: "pro") {
    onClose();
    push("/pro");
  }

  type PlanCard = {
    key: "free" | "pro";
    name: string;
    price: string;
    features: string[];
    accent: string;
    cta?: () => void;
    ctaLabel?: string;
    highlighted?: boolean;
  };

  // Two tiers post-1.6.1. Pro+ dropped; the modal mirrors the public
  // pricing page (Free + Pro side-by-side). Features stay in i18n so
  // every locale can re-phrase the bullets independently.
  const plans = $derived<PlanCard[]>([
    {
      key: "free",
      name: $_("upgrade.free_name"),
      price: $_("upgrade.free_price"),
      features: [
        $_("upgrade.free_feat_storage"),
        $_("upgrade.free_feat_devices"),
        $_("upgrade.free_feat_sync"),
        $_("upgrade.free_feat_history"),
        $_("upgrade.free_feat_export"),
      ],
      accent: "border-zinc-700/60",
    },
    {
      key: "pro",
      name: $_("upgrade.pro_name"),
      price: $_("upgrade.pro_price"),
      features: [
        $_("upgrade.pro_feat_wrapped"),
        $_("upgrade.pro_feat_screen"),
        $_("upgrade.pro_feat_storage"),
        $_("upgrade.pro_feat_devices"),
        $_("upgrade.pro_feat_max_save"),
      ],
      accent: "border-emerald-500/40 bg-emerald-500/5",
      cta: () => pick("pro"),
      ctaLabel: $_("upgrade.pro_cta"),
      highlighted: true,
    },
  ]);
</script>

<Modal
  {open}
  {onClose}
  title={$_("upgrade.title")}
  description={$_("upgrade.subtitle")}
>
  {#if reason}
    <div
      class="mb-4 rounded-md border border-amber-500/40 bg-amber-500/10 px-3 py-2 text-sm text-amber-200"
    >
      {reason}
    </div>
  {/if}

  <div class="grid grid-cols-1 gap-3 md:grid-cols-2">
    {#each plans as plan (plan.key)}
      {@const isCurrent = currentPlan === plan.key}
      <div
        class="flex flex-col gap-3 rounded-lg border {plan.accent} p-4 {plan.highlighted
          ? 'shadow-md shadow-emerald-500/10'
          : ''}"
      >
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-1.5">
            {#if plan.highlighted}
              <Sparkles size={14} class="text-emerald-400" />
            {/if}
            <h3 class="text-sm font-semibold text-zinc-100">{plan.name}</h3>
          </div>
          {#if isCurrent}
            <span
              class="rounded-full bg-zinc-800 px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide text-zinc-300"
            >
              {$_("upgrade.current")}
            </span>
          {/if}
        </div>
        <p class="text-lg font-semibold text-zinc-100">{plan.price}</p>
        <ul class="flex flex-1 flex-col gap-1.5 text-xs text-zinc-300">
          {#each plan.features as feat (feat)}
            <li class="flex items-start gap-1.5">
              <Check size={12} class="mt-0.5 shrink-0 text-emerald-400" />
              <span>{feat}</span>
            </li>
          {/each}
        </ul>
        {#if plan.cta && !isCurrent}
          <Button
            variant={plan.highlighted ? "primary" : "secondary"}
            onclick={plan.cta}
          >
            {plan.ctaLabel}
          </Button>
        {/if}
      </div>
    {/each}
  </div>

  {#snippet footer()}
    <Button variant="ghost" onclick={onClose}>
      {$_("upgrade.maybe_later")}
    </Button>
  {/snippet}
</Modal>
