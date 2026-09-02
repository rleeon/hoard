<script lang="ts">
  /**
   * "Gracias por apoyar Hoard", salta una sola vez, en mitad de la aplicación,
   * la primera vez que vemos la cuenta en Pro después de no estarlo (ver
   * `stores/planEvents.ts`).
   *
   * No es un recibo: el recibo lo manda Polar. Es la única pantalla que explica
   * qué acaba de comprar el usuario y,esto es lo importante, **qué parte se
   * queda para siempre y qué parte depende de seguir pagando**. Los
   * dispositivos ilimitados no vuelven a bajar aunque cancele (`first_pro_at`
   * es un marcador de un solo sentido en el servidor); los 100 GB y Hoard
   * Screen sí se van con la suscripción. Decirlo aquí, el día que paga y de
   * buen humor, es más honesto que dejar que lo descubra el día que cancela.
   */
  import { _ } from "svelte-i18n";
  import { Infinity as InfinityIcon, HardDrive, MonitorPlay } from "@lucide/svelte";

  import Modal from "./Modal.svelte";
  import HeartsMark from "./HeartsMark.svelte";

  type Props = {
    open: boolean;
    onClose: () => void;
  };

  let { open, onClose }: Props = $props();

  type Perk = {
    icon: typeof HardDrive;
    title: string;
    body: string;
    badge: string;
    /** Los que sobreviven a una cancelación se pintan en verde. */
    forever: boolean;
  };

  const perks = $derived<Perk[]>([
    {
      icon: InfinityIcon,
      title: $_("pro_thanks.devices_title"),
      body: $_("pro_thanks.devices_body"),
      badge: $_("pro_thanks.badge_forever"),
      forever: true,
    },
    {
      icon: HardDrive,
      title: $_("pro_thanks.storage_title"),
      body: $_("pro_thanks.storage_body"),
      badge: $_("pro_thanks.badge_with_pro"),
      forever: false,
    },
    {
      icon: MonitorPlay,
      title: $_("pro_thanks.screen_title"),
      body: $_("pro_thanks.screen_body"),
      badge: $_("pro_thanks.badge_with_pro"),
      forever: false,
    },
  ]);
</script>

<Modal {open} {onClose} title={$_("pro_thanks.title")}>
  <div class="space-y-4">
    <div class="flex flex-col items-center gap-3 pt-1 text-center">
      <HeartsMark width={172} />
      <p class="max-w-sm text-sm leading-relaxed text-zinc-300">
        {$_("pro_thanks.subtitle")}
      </p>
    </div>

    <ul class="space-y-2">
      {#each perks as perk (perk.title)}
        {@const Icon = perk.icon}
        <li
          class="flex items-start gap-3 rounded-lg border p-3 {perk.forever
            ? 'border-emerald-500/30 bg-emerald-500/[0.07]'
            : 'border-white/[0.08] bg-zinc-950/40'}"
        >
          <span
            class="mt-0.5 shrink-0 {perk.forever
              ? 'text-emerald-400'
              : 'text-zinc-400'}"
            aria-hidden="true"
          >
            <Icon size={16} />
          </span>
          <div class="min-w-0 flex-1">
            <div class="flex flex-wrap items-center gap-2">
              <span class="text-sm font-semibold text-zinc-100">{perk.title}</span>
              <span
                class="rounded-full px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide {perk.forever
                  ? 'bg-emerald-500/15 text-emerald-300'
                  : 'bg-zinc-800 text-zinc-400'}"
              >
                {perk.badge}
              </span>
            </div>
            <p class="mt-0.5 text-xs leading-relaxed text-zinc-400">{perk.body}</p>
          </div>
        </li>
      {/each}
    </ul>

    <p class="text-[11px] leading-relaxed text-zinc-500">
      {$_("pro_thanks.footnote")}
    </p>
  </div>

  {#snippet footer()}
    <button
      type="button"
      onclick={onClose}
      class="rounded-lg bg-emerald-600 px-4 py-2 text-sm font-semibold text-white transition-colors hover:bg-emerald-500"
    >
      {$_("pro_thanks.close")}
    </button>
  {/snippet}
</Modal>
