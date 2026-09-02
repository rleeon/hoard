<script lang="ts">
  /**
   * "Veo que te vas", la despedida de Pro. Salta una sola vez, en mitad de la
   * aplicación, cuando vemos por primera vez la cancelación: o una baja
   * programada (`cancel_at`, todavía en Pro) o la cuenta ya caída a Free sin
   * que hubiéramos visto la baja (ver `stores/planEvents.ts`).
   *
   * Lleva dentro el picker de *Liberar espacio* a propósito. Cancelar no es un
   * trámite: el almacenamiento baja a 2 GB y lo que no quepa se archiva. Ese
   * número el usuario no lo tiene en la cabeza el día que cancela, y la
   * aplicación sí, así que aquí se le enseña **con sus juegos y sus bytes**,
   * midiendo contra el límite al que va a caer y no contra el que aún tiene.
   * Puede elegir ahora qué se va a la caja negra, o volver a Pro y no tener que
   * elegir nada.
   *
   * Lo que **no** hace es esconder la parte buena: los dispositivos ilimitados
   * se los queda de por vida, y eso se dice arriba del todo.
   */
  import { _ } from "svelte-i18n";
  import { Infinity as InfinityIcon, HardDrive, MonitorPlay, Archive } from "@lucide/svelte";

  import Modal from "./Modal.svelte";
  import HeartsMark from "./HeartsMark.svelte";
  import LiberateStoragePanel from "./LiberateStoragePanel.svelte";
  import {
    archiveSaveCloud,
    cloud,
    openBillingPortal,
    refreshCloud,
  } from "../stores/cloud";
  import { toastError, toastSuccess } from "../stores/toasts";
  import { formatBytes } from "../utils/format";

  type Props = {
    open: boolean;
    onClose: () => void;
  };

  let { open, onClose }: Props = $props();

  /** Lo que da el plan Free hoy (`cloud/plans.rs`). Es el suelo al que cae una
   *  cuenta que cancela, y el que se usa mientras el servidor no diga otra cosa
   *  con `pending_storage_limit_bytes`. */
  const FREE_STORAGE_BYTES = 2 * 1024 * 1024 * 1024;

  const account = $derived($cloud.account);

  /** El límite al que va la cuenta, no el que tiene. Durante la ventana de
   *  gracia el servidor sigue aplicando el grande, así que medir con
   *  `storage_limit_bytes` diría "cabe todo" justo el día en que hay que
   *  decidir. */
  const targetLimit = $derived.by(() => {
    const pending = account?.pending_storage_limit_bytes ?? null;
    if (pending != null && pending > 0) return pending;
    if (account?.plan === "pro") return FREE_STORAGE_BYTES;
    const current = account?.storage_limit_bytes ?? 0;
    return current > 0 ? current : FREE_STORAGE_BYTES;
  });

  const used = $derived(account?.storage_used_bytes ?? 0);
  const overBytes = $derived(Math.max(0, used - targetLimit));

  /** Cuándo se aplica el recorte. El servidor manda la fecha exacta durante la
   *  gracia; si no la hay, la de la baja programada. */
  const changeDate = $derived.by(() => {
    const raw = account?.storage_limit_change_at ?? account?.cancel_at ?? null;
    if (!raw) return null;
    const d = new Date(raw);
    return Number.isNaN(d.getTime())
      ? null
      : d.toLocaleDateString(undefined, {
          year: "numeric",
          month: "long",
          day: "numeric",
        });
  });

  let busy = $state(false);
  let selected = $state<Set<string>>(new Set());
  let reloadKey = $state(0);

  async function archiveSelected() {
    const ids = [...selected];
    if (ids.length === 0 || busy) return;
    busy = true;
    try {
      for (const id of ids) {
        await archiveSaveCloud(id);
      }
      toastSuccess($_("liberate.done"));
      await refreshCloud().catch(() => {});
      reloadKey += 1; // relee el panel con las cifras nuevas
    } catch (e) {
      toastError(String(e));
    } finally {
      busy = false;
    }
  }

  async function manageBilling() {
    try {
      await openBillingPortal();
    } catch (e) {
      toastError(String(e));
    }
  }
</script>

<Modal
  {open}
  title={$_("pro_farewell.title")}
  dismissible={!busy}
  scrollBody
  onClose={busy ? () => {} : onClose}
>
  <div class="space-y-4">
    <div class="flex flex-col items-center gap-3 pt-1 text-center">
      <HeartsMark broken width={172} />
      <p class="max-w-sm text-sm leading-relaxed text-zinc-300">
        {$_("pro_farewell.subtitle")}
      </p>
    </div>

    <!-- Lo que se queda. Va primero: es verdad y es suyo. -->
    <div
      class="flex items-start gap-3 rounded-lg border border-emerald-500/30 bg-emerald-500/[0.07] p-3"
    >
      <span class="mt-0.5 shrink-0 text-emerald-400" aria-hidden="true">
        <InfinityIcon size={16} />
      </span>
      <div class="min-w-0">
        <p class="text-sm font-semibold text-zinc-100">
          {$_("pro_farewell.keep_title")}
        </p>
        <p class="mt-0.5 text-xs leading-relaxed text-zinc-400">
          {$_("pro_farewell.keep_body")}
        </p>
      </div>
    </div>

    <!-- Lo que se va. -->
    <ul class="space-y-2">
      <li
        class="flex items-start gap-3 rounded-lg border border-white/[0.08] bg-zinc-950/40 p-3"
      >
        <span class="mt-0.5 shrink-0 text-rose-400" aria-hidden="true">
          <HardDrive size={16} />
        </span>
        <div class="min-w-0">
          <p class="text-sm font-semibold text-zinc-100">
            {$_("pro_farewell.storage_title", {
              values: { size: formatBytes(targetLimit) },
            })}
          </p>
          <p class="mt-0.5 text-xs leading-relaxed text-zinc-400">
            {changeDate
              ? $_("pro_farewell.storage_body_on", {
                  values: { date: changeDate, used: formatBytes(used) },
                })
              : $_("pro_farewell.storage_body", {
                  values: { used: formatBytes(used) },
                })}
          </p>
        </div>
      </li>
      <li
        class="flex items-start gap-3 rounded-lg border border-white/[0.08] bg-zinc-950/40 p-3"
      >
        <span class="mt-0.5 shrink-0 text-rose-400" aria-hidden="true">
          <MonitorPlay size={16} />
        </span>
        <div class="min-w-0">
          <p class="text-sm font-semibold text-zinc-100">
            {$_("pro_farewell.screen_title")}
          </p>
          <p class="mt-0.5 text-xs leading-relaxed text-zinc-400">
            {$_("pro_farewell.screen_body")}
          </p>
        </div>
      </li>
    </ul>

    {#if overBytes > 0}
      <!-- El mismo picker del botón "Liberar espacio" de la aplicación, pero
           midiendo contra el límite al que la cuenta va a caer. -->
      <div class="rounded-lg border border-rose-500/40 bg-rose-500/[0.07] p-3">
        <p class="text-sm font-semibold text-rose-100">
          {$_("pro_farewell.over_title")}
        </p>
        <p class="mt-1 text-xs leading-relaxed text-rose-200/90">
          {$_("pro_farewell.over_body", {
            values: {
              used: formatBytes(used),
              limit: formatBytes(targetLimit),
              over: formatBytes(overBytes),
            },
          })}
        </p>
      </div>

      <LiberateStoragePanel
        {open}
        {busy}
        {reloadKey}
        limitOverride={targetLimit}
        bind:selected
      />

      <button
        type="button"
        onclick={archiveSelected}
        disabled={busy || selected.size === 0}
        class="flex w-full items-center justify-center gap-2 rounded-lg bg-red-600 px-4 py-2 text-sm font-semibold text-white transition-colors hover:bg-red-500 disabled:opacity-50"
      >
        <Archive size={15} />
        {busy
          ? $_("liberate.archiving")
          : $_("liberate.continue_n", { values: { count: selected.size } })}
      </button>

      <p
        class="rounded-lg border border-amber-500/40 bg-amber-500/10 p-2.5 text-xs text-amber-200/90"
      >
        {$_("liberate.explain_archive")}
      </p>
    {:else}
      <p
        class="rounded-lg border border-emerald-500/30 bg-emerald-500/[0.07] p-2.5 text-xs text-emerald-200/90"
      >
        {$_("pro_farewell.fits", {
          values: { used: formatBytes(used), limit: formatBytes(targetLimit) },
        })}
      </p>
    {/if}
  </div>

  {#snippet footer()}
    <div class="flex w-full items-center justify-between gap-3">
      <!-- Volver — negro, abajo a la izquierda -->
      <button
        type="button"
        onclick={onClose}
        disabled={busy}
        class="rounded-lg border border-white/10 bg-zinc-900 px-4 py-2 text-sm font-medium text-zinc-200 transition-colors hover:bg-zinc-800 disabled:opacity-50"
      >
        {$_("pro_farewell.back")}
      </button>

      <!-- Gestionar suscripción — abajo a la derecha -->
      <button
        type="button"
        onclick={manageBilling}
        disabled={busy}
        class="rounded-lg bg-emerald-600 px-4 py-2 text-sm font-semibold text-white transition-colors hover:bg-emerald-500 disabled:opacity-50"
      >
        {$_("account.manage_billing")}
      </button>
    </div>
  {/snippet}
</Modal>
