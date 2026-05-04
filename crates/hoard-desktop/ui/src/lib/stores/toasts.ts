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
};

const internal = writable<Toast[]>([]);
let nextId = 1;

export const toasts: Readable<Toast[]> = { subscribe: internal.subscribe };

export function pushToast(
  kind: ToastKind,
  message: string,
  duration = 4000,
): number {
  const id = nextId++;
  internal.update((list) => [...list, { id, kind, message, duration }]);
  if (duration > 0) {
    setTimeout(() => dismissToast(id), duration);
  }
  return id;
}

export function dismissToast(id: number): void {
  internal.update((list) => list.filter((t) => t.id !== id));
}

export const toastInfo = (msg: string, ms?: number) =>
  pushToast("info", msg, ms);
export const toastSuccess = (msg: string, ms?: number) =>
  pushToast("success", msg, ms);
export const toastError = (msg: string, ms?: number) =>
  pushToast("error", msg, ms ?? 6000);
