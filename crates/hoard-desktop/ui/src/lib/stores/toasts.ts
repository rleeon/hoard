/**
 * Tiny toast queue.
 *
 * `pushToast` adds a message; the `<Toaster />` component subscribes to the
 * list and renders them. Toasts auto-dismiss after `duration` ms; clicking
 * one dismisses it early.
 */

import { writable, type Readable } from "svelte/store";

export type ToastKind = "info" | "success" | "error";

export type Toast = {
  id: number;
  kind: ToastKind;
  message: string;
  duration: number;
  /** How many times this same message has fired while on screen. Rendered as
   * a "×N" badge so a flapping background job (e.g. a dead server retrying)
   * collapses into one toast instead of stacking hundreds. */
  count: number;
};

const internal = writable<Toast[]>([]);
let nextId = 1;
/** Per-id auto-dismiss timers, so a coalesced toast can reset its own clock. */
const timers = new Map<number, ReturnType<typeof setTimeout>>();

export const toasts: Readable<Toast[]> = { subscribe: internal.subscribe };

function armTimer(id: number, duration: number): void {
  const prev = timers.get(id);
  if (prev) clearTimeout(prev);
  if (duration > 0) {
    timers.set(
      id,
      setTimeout(() => dismissToast(id), duration),
    );
  }
}

export function pushToast(
  kind: ToastKind,
  message: string,
  duration = 4000,
): number {
  // Coalesce: if an identical (kind, message) toast is already showing, bump
  // its counter and refresh its timer rather than queuing a duplicate. This
  // keeps a misbehaving background loop from burying the UI in toasts.
  let existingId = -1;
  internal.update((list) => {
    const match = list.find((t) => t.kind === kind && t.message === message);
    if (match) {
      existingId = match.id;
      return list.map((t) =>
        t.id === match.id ? { ...t, count: t.count + 1, duration } : t,
      );
    }
    return list;
  });
  if (existingId !== -1) {
    armTimer(existingId, duration);
    return existingId;
  }

  const id = nextId++;
  internal.update((list) => [...list, { id, kind, message, duration, count: 1 }]);
  armTimer(id, duration);
  return id;
}

export function dismissToast(id: number): void {
  const t = timers.get(id);
  if (t) {
    clearTimeout(t);
    timers.delete(id);
  }
  internal.update((list) => list.filter((t) => t.id !== id));
}

export const toastInfo = (msg: string, ms?: number) =>
  pushToast("info", msg, ms);
export const toastSuccess = (msg: string, ms?: number) =>
  pushToast("success", msg, ms);
export const toastError = (msg: string, ms?: number) =>
  pushToast("error", msg, ms ?? 6000);
