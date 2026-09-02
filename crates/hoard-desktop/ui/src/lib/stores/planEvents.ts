/**
 * "Acaba de pasar algo con tu plan", el disparador de los dos diálogos que
 * sólo se ven una vez: el agradecimiento al pagar Pro y la despedida al
 * cancelarlo.
 *
 * El plan no se mueve por nada que ocurra dentro de la aplicación: el pago y la
 * cancelación pasan en el navegador, en Polar, y aquí sólo llega el resultado
 * en el siguiente `/v1/me`. Así que esto no es un evento, es una **diferencia**:
 * cada snapshot de cuenta se compara con el último que vimos de ese usuario y
 * lo que cambió decide qué diálogo toca.
 *
 * Guardarlo en disco (no en memoria) es justo el punto: el usuario paga en el
 * navegador, cierra, y vuelve a abrir Hoard horas después. Si la comparación
 * viviera en la sesión, ese arranque no tendría con qué comparar y el
 * agradecimiento no llegaría nunca, o, peor, llegaría en cada arranque.
 *
 * Reglas del marcador (por `user_id`, en `plan-events.json`):
 *
 *   - **Sin marcador previo → no se enseña nada**, sólo se siembra. Un usuario
 *     que ya era Pro cuando esto se instaló no merece un "gracias por pagar"
 *     de la nada, y quien acaba de iniciar sesión ya sabe lo que compró.
 *   - `free → pro`  → `thanks`.
 *   - Aparece `cancel_at` (sigue en Pro, con baja programada) → `farewell`.
 *     Ese es el momento en que el usuario se va, aunque el plan tarde semanas
 *     en caerse.
 *   - `pro → free` sin haber visto antes el `cancel_at` → `farewell` también:
 *     es el caso de quien canceló con la aplicación cerrada y sólo vuelve
 *     cuando ya está en Free.
 *
 * El marcador se escribe **al decidir enseñar el diálogo**, no al cerrarlo: si
 * el proceso se muere con la ventana abierta, "una sola vez" sigue siendo una
 * sola vez.
 */
import { LazyStore } from "@tauri-apps/plugin-store";
import { writable } from "svelte/store";

const STORE_FILE = "plan-events.json";

/** Última foto conocida del plan de un usuario. */
type Seen = {
  /** "free" | "pro" */
  plan: string;
  /** Si tenía una baja programada (`cancel_at`) la última vez que miramos. */
  cancel: boolean;
};

export type PlanEvent = "thanks" | "farewell";

/** Diálogo pendiente de enseñar, o `null`. Lo consume `App.svelte`. */
export const planEvent = writable<PlanEvent | null>(null);

export function dismissPlanEvent(): void {
  planEvent.set(null);
}

const store = new LazyStore(STORE_FILE);

/** Lo mínimo que hace falta de la cuenta para diferenciar dos snapshots. */
type PlanSnapshot = {
  user_id: string;
  plan: string;
  cancel_at?: string | null;
};

/**
 * Compara el snapshot recién llegado con el último que vimos de esa cuenta y
 * encola el diálogo que corresponda. Idempotente: si nada cambió no escribe ni
 * enseña nada, así que puede llamarse en cada refresco (`/v1/me` se pide cada
 * 30 s desde la barra lateral).
 *
 * Nunca lanza: es decoración sobre el refresco de cuenta, y un fallo de disco
 * no puede tumbar la ruta que lo llamó.
 */
export async function notePlanSnapshot(
  account: PlanSnapshot | null,
): Promise<void> {
  if (!account?.user_id) return;
  const key = `seen:${account.user_id}`;
  const next: Seen = { plan: account.plan, cancel: !!account.cancel_at };

  let prev: Seen | null;
  try {
    prev = (await store.get<Seen>(key)) ?? null;
  } catch (e) {
    console.warn("planEvents: no se pudo leer el marcador:", e);
    return;
  }

  const persist = async () => {
    if (prev && prev.plan === next.plan && prev.cancel === next.cancel) return;
    try {
      await store.set(key, next);
      await store.save();
    } catch (e) {
      console.warn("planEvents: no se pudo guardar el marcador:", e);
    }
  };

  // Primera vez que vemos esta cuenta: sembrar y callar.
  if (!prev) {
    await persist();
    return;
  }

  const isPro = next.plan === "pro";
  let event: PlanEvent | null = null;
  if (isPro && prev.plan !== "pro") {
    event = "thanks";
  } else if (next.cancel && !prev.cancel) {
    event = "farewell";
  } else if (!isPro && prev.plan === "pro" && !prev.cancel) {
    event = "farewell";
  }

  // Escribir antes de enseñar (ver cabecera).
  await persist();
  if (event) planEvent.set(event);
}
