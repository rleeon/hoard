/**
 * Global error dialog store.
 *
 * `showError(e)` accepts:
 *   - An `AppError` shape (`{ title, body, detail? }`) — what Rust commands
 *     return when they fail with `Result<_, AppError>`. Title/body are i18n
 *     keys resolved by svelte-i18n at render time.
 *   - A raw string — surfaced verbatim under the generic title. Kept for
 *     callers still on the `toastError(String(e))` pattern that haven't
 *     migrated to structured errors yet.
 *   - Anything else (unknown / Error instance) — coerced through
 *     `String(e)` and shown under the generic title, with the original in
 *     `detail` so the user can still see what went wrong technically.
 *
 * The dialog is rendered once at the root (App.svelte) and consumes this
 * store. Only one error is shown at a time — calling `showError` while
 * another is open replaces it. That's deliberate: the previous error is
 * usually the root cause of the new one and the user only needs to dismiss
 * once.
 */
import { writable } from "svelte/store";

export interface AppError {
  title: string;
  body: string;
  detail?: string | null;
}

export const errorDialog = writable<AppError | null>(null);

function isAppErrorShape(x: unknown): x is AppError {
  return (
    !!x &&
    typeof x === "object" &&
    "title" in x &&
    "body" in x &&
    typeof (x as { title: unknown }).title === "string" &&
    typeof (x as { body: unknown }).body === "string"
  );
}

export function showError(
  e: AppError | { title: string; body: string; detail?: string | null } | string | unknown,
): void {
  if (typeof e === "string") {
    errorDialog.set({ title: "common.error_title", body: e });
  } else if (isAppErrorShape(e)) {
    errorDialog.set({
      title: e.title,
      body: e.body,
      detail: e.detail ?? null,
    });
  } else {
    errorDialog.set({
      title: "common.error_title",
      body: "common.error_generic",
      detail: String(e),
    });
  }
}

export function dismissError(): void {
  errorDialog.set(null);
}
