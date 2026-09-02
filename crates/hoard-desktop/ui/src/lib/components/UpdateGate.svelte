<script lang="ts">
  /**
   * La pantalla de "actualizando Hoard", lo que hace que la app se actualice
   * **al abrirse**, como Steam o Discord.
   *
   * Sólo aparece cuando no queda alternativa, y hay tres motivos distintos:
   *
   * 1. **Se acabó el plazo** (`mandatory`). El servicio lleva dos días
   *    intentando aplicar algo que necesita a alguien delante, un `.deb` que
   *    quiere polkit, un `.dmg` que quiere una mano,, y aquí está ese alguien.
   *    No se puede cerrar: es el escalón que el plazo existe para provocar.
   * 2. **El servicio ya se actualizó y esta ventana se quedó atrás.** Pasa por
   *    diseño: el servicio releva los binarios en silencio y se reinicia, pero
   *    a una ventana abierta no la puede tocar. Sin este aviso el usuario sigue
   *    en la versión vieja hasta que cierre la app por su cuenta.
   * 3. **Está aplicándose ahora mismo.** No se bloquea por gusto: los binarios
   *    se están sustituyendo debajo, y dejar seguir clicando en una app cuyo
   *    motor está reiniciándose sólo produce errores que no significan nada.
   *
   * Lo que **no** hace: aparecer cada vez que sale una versión. El caso normal
   * es silencioso (el servicio baja y aplica, y el usuario se entera por el
   * número de versión); el caso "hay algo listo y es opcional" es la insignia
   * ámbar de siempre en la barra lateral. Un modal por release sería el
   * problema contrario.
   */
  import { onMount, onDestroy } from "svelte";
  import { _ } from "svelte-i18n";
  import { Loader2, Download, RefreshCw } from "@lucide/svelte";
  import { invoke } from "@tauri-apps/api/core";

  import {
    applyStagedUpdate,
    fetchServiceUpdate,
    serviceUpdate,
    windowIsBehind,
    type UpdateState,
  } from "../stores/updates";
  import { APP_VERSION } from "../version";

  /** Cada cuánto se re-pregunta mientras el gate está en pantalla. Corto a
   *  propósito: aquí el usuario está mirando una barra que no se mueve. */
  const TICK_MS = 2_000;

  let timer: ReturnType<typeof setInterval> | null = null;
  let working = $state(false);
  let failed = $state<string | null>(null);

  const svc = $derived<UpdateState | null>($serviceUpdate);
  const phase = $derived(svc?.phase.phase ?? "up_to_date");

  /** Esta ventana corre un binario más viejo que el del servicio. */
  const behind = $derived(windowIsBehind(svc, APP_VERSION));

  /** El servicio está tocando los binarios ahora mismo. */
  const busy = $derived(phase === "applying" || phase === "restarting");

  /** ¿Se enseña algo? */
  const visible = $derived(
    !!svc && (svc.mandatory || behind || busy || working),
  );

  /** ¿Se puede cerrar? Sólo el caso "ya se actualizó, reinicia cuando quieras"
   *  admite un no; el plazo vencido y una instalación en curso, no. */
  const dismissible = $derived(behind && !svc?.mandatory && !busy);
  let dismissed = $state(false);

  const showing = $derived(visible && !(dismissible && dismissed));

  onMount(() => {
    void nudgeOnOpen();
    timer = setInterval(() => void fetchServiceUpdate(), TICK_MS);
  });

  /**
   * "Que se actualice al abrirse", literalmente.
   *
   * Casi siempre no hay nada que hacer aquí: el servicio ya aplicó la
   * actualización antes de que nadie abriera nada, así que esta ventana **ya
   * es** la nueva. Esto cubre el hueco, el servicio la tenía bajada y estaba
   * esperando su próximo ciclo, o esperando a que cerraras un juego. Abrir la
   * app es la señal de que ahora es buen momento.
   *
   * Sólo cuando se aplica sola (`unattended`): sin diálogos, sin privilegios,
   * sin nada que aprobar. La vía que necesita un humano se ofrece, no se
   * dispara, y cuando vence el plazo, es este mismo componente el que tapa la
   * pantalla y lo pide.
   */
  async function nudgeOnOpen() {
    const s = await fetchServiceUpdate();
    if (!s || !s.unattended || !s.staged) return;
    if (s.phase.phase !== "ready" && s.phase.phase !== "waiting") return;
    try {
      await applyStagedUpdate(s.staged);
    } catch (e) {
      // Silencioso a propósito: nadie ha pedido esto, así que nadie merece un
      // error por ello. El ciclo de fondo del servicio lo reintenta.
      console.warn("update nudge on open failed:", e);
    }
  }

  onDestroy(() => {
    if (timer) clearInterval(timer);
    timer = null;
  });

  async function install() {
    working = true;
    failed = null;
    try {
      await applyStagedUpdate(svc?.latest ?? undefined);
    } catch (e) {
      // Lo más habitual aquí no es un fallo del updater: es que el usuario
      // canceló el diálogo de privilegios. Se dice y se deja reintentar en vez
      // de dejar la pantalla girando para siempre.
      failed = e instanceof Error ? e.message : String(e);
      working = false;
    }
  }

  async function restartApp() {
    try {
      await invoke("restart_app");
    } catch (e) {
      console.warn("relaunch failed:", e);
      failed = e instanceof Error ? e.message : String(e);
    }
  }

  /** El titular, que es lo único que casi nadie va a leer entero. */
  const title = $derived(
    busy || working
      ? $_("update_gate.installing_title")
      : behind
        ? $_("update_gate.restart_title")
        : $_("update_gate.required_title"),
  );

  const body = $derived(
    busy || working
      ? $_("update_gate.installing_body")
      : behind
        ? $_("update_gate.restart_body", {
            values: { version: svc?.current ?? "" },
          })
        : $_("update_gate.required_body", {
            values: { version: svc?.latest ?? "" },
          }),
  );
</script>

{#if showing}
  <!-- Por encima de todo, incluidos los modales: si esto está en pantalla es
       porque nada de lo de debajo puede seguir su curso. -->
  <div
    class="fixed inset-0 z-[100] flex items-center justify-center bg-zinc-950/95 backdrop-blur-sm"
    role="dialog"
    aria-modal="true"
    aria-labelledby="update-gate-title"
  >
    <div class="w-full max-w-md px-8 text-center">
      <div class="mb-6 flex justify-center">
        {#if busy || working}
          <Loader2 class="h-10 w-10 animate-spin text-emerald-500" />
        {:else if behind}
          <RefreshCw class="h-10 w-10 text-emerald-500" />
        {:else}
          <Download class="h-10 w-10 text-amber-500" />
        {/if}
      </div>

      <h1
        id="update-gate-title"
        class="mb-3 text-xl font-semibold text-zinc-100"
      >
        {title}
      </h1>
      <p class="mb-8 text-sm leading-relaxed text-zinc-400">{body}</p>

      {#if failed}
        <p
          class="mb-6 rounded-md border border-red-500/40 bg-red-500/10 px-4 py-3 text-left text-xs text-red-300"
        >
          {failed}
        </p>
      {/if}

      {#if !busy && !working}
        <div class="flex flex-col gap-3">
          {#if behind}
            <button
              class="w-full rounded-md bg-emerald-600 px-4 py-2.5 text-sm font-medium text-white transition hover:bg-emerald-500"
              onclick={restartApp}
            >
              {$_("update_gate.restart_now")}
            </button>
            {#if dismissible}
              <button
                class="w-full rounded-md px-4 py-2.5 text-sm text-zinc-400 transition hover:text-zinc-200"
                onclick={() => (dismissed = true)}
              >
                {$_("update_gate.later")}
              </button>
            {/if}
          {:else}
            <button
              class="w-full rounded-md bg-emerald-600 px-4 py-2.5 text-sm font-medium text-white transition hover:bg-emerald-500"
              onclick={install}
            >
              {$_("update_gate.install_now")}
            </button>
            <p class="text-xs text-zinc-500">
              {$_("update_gate.privileges_hint")}
            </p>
          {/if}
        </div>
      {/if}
    </div>
  </div>
{/if}
