/**
 * "Something just happened with your plan": the trigger for the two dialogs that
 * are only ever seen once, the thank-you when Pro is paid for and the farewell when
 * it is cancelled.
 *
 * The plan does not move because of anything that happens inside the application:
 * the payment and the cancellation happen in the browser, on Polar, and only the
 * result arrives here on the next `/v1/me`. So this is not an event, it is a
 * **difference**: every account snapshot is compared against the last one we saw for
 * that user and what changed decides which dialog is due.
 *
 * Storing it on disk (not in memory) is precisely the point: the user pays in the
 * browser, closes it, and opens Hoard again hours later. If the comparison lived in
 * the session, that start would have nothing to compare against and the thank-you
 * would never arrive, or worse, would arrive on every start.
 *
 * The marker's rules (per `user_id`, in `plan-events.json`):
 *
 *   - **With no previous marker, nothing is shown**, it is only seeded. A user who
 *     was already Pro when this shipped does not deserve a "thanks for paying" out
 *     of nowhere, and somebody who has just signed in already knows what they
 *     bought.
 *   - free to pro gives `thanks`.
 *   - A `cancel_at` appearing (still on Pro, with a scheduled downgrade) gives
 *     `farewell`. That is the moment the user leaves, even if the plan takes weeks
 *     to fall.
 *   - pro to free without having seen the `cancel_at` first also gives `farewell`:
 *     that is the case of somebody who cancelled with the application closed and
 *     only comes back once they are already on Free.
 *
 * The marker is written **when the decision to show the dialog is made**, not when
 * it is closed: if the process dies with the window open, "only once" is still only
 * once.
 */
import { LazyStore } from "@tauri-apps/plugin-store";
import { writable } from "svelte/store";

const STORE_FILE = "plan-events.json";

/** Última foto conocida del plan de un usuario. */
type Seen = {
  /** "free" | "pro" */
  plan: string;
  /** Whether it had a scheduled downgrade (`cancel_at`) the last time we looked. */
  cancel: boolean;
};

export type PlanEvent = "thanks" | "farewell";

/** The dialog waiting to be shown, or `null`. `App.svelte` consumes it. */
export const planEvent = writable<PlanEvent | null>(null);

export function dismissPlanEvent(): void {
  planEvent.set(null);
}

const store = new LazyStore(STORE_FILE);

/** The minimum needed from the account to tell two snapshots apart. */
type PlanSnapshot = {
  user_id: string;
  plan: string;
  cancel_at?: string | null;
};

/**
 * Compares the freshly arrived snapshot with the last one we saw for that account
 * and queues whichever dialog is due. Idempotent: when nothing changed it writes
 * nothing and shows nothing, so it can be called on every refresh (`/v1/me` is asked
 * for every 30 s from the sidebar).
 *
 * It never throws: it is decoration on top of the account refresh, and a disk
 * failure must not bring down the route that called it.
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

  // Write before showing (see the header).
  await persist();
  if (event) planEvent.set(event);
}
