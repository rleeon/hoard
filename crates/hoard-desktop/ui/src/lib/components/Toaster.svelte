<script lang="ts">
  /**
   * Bottom-right stack of toast notifications.
   *
   * Listens to the global `toasts` store; each entry renders with an icon
   * matching its kind. Click to dismiss early; otherwise they auto-fade
   * after their `duration`.
   */
  import { fly } from "svelte/transition";
  import { CheckCircle2, AlertCircle, Info, X } from "lucide-svelte";
  import { _ } from "svelte-i18n";
  import { toasts, dismissToast, type Toast } from "../stores/toasts";

  const ICONS = {
    info: Info,
    success: CheckCircle2,
    error: AlertCircle,
  } as const;

  const TINTS = {
    info: "border-zinc-700 bg-zinc-900/95 text-zinc-100",
    success: "border-emerald-500/40 bg-emerald-500/10 text-emerald-100",
    error: "border-red-500/40 bg-red-500/10 text-red-100",
  } as const;

  const ICON_TINTS = {
    info: "text-zinc-300",
    success: "text-emerald-400",
    error: "text-red-400",
  } as const;
</script>

<div
  class="pointer-events-none fixed bottom-4 right-4 z-50 flex w-[min(24rem,calc(100vw-2rem))] flex-col gap-2"
  role="region"
  aria-label={$_("common.notifications")}
  aria-live="polite"
>
  {#each $toasts as toast (toast.id)}
    {@const Icon = ICONS[toast.kind]}
    <div
      class="pointer-events-auto flex items-start gap-3 rounded-lg border px-4 py-3 shadow-lg backdrop-blur {TINTS[
        toast.kind
      ]}"
      transition:fly={{ y: 12, duration: 180 }}
    >
      <span class={ICON_TINTS[toast.kind]} aria-hidden="true">
        <Icon size={18} />
      </span>
      <p class="flex-1 text-sm leading-snug">
        {toast.message}
        {#if toast.count > 1}
          <span
            class="ml-1.5 inline-flex items-center rounded-full bg-white/10 px-1.5 py-0.5 text-xs font-medium tabular-nums"
          >×{toast.count}</span>
        {/if}
      </p>
      <button
        type="button"
        class="-mr-1 -mt-1 shrink-0 rounded p-1 text-zinc-400 hover:text-zinc-100 focus:outline-none focus-visible:ring-2 focus-visible:ring-zinc-500"
        aria-label={$_("common.dismiss")}
        onclick={() => dismissToast(toast.id)}
      >
        <X size={14} />
      </button>
    </div>
  {/each}
</div>
