/**
 * Helpers for surfacing the structured JSON errors that the cloud server
 * returns when a quota gate trips. The server uses three:
 *
 *   - **413 `save_too_large`** — per-save cap (`max_save_size_bytes`).
 *     Returned by `init_upload` before we presign R2 so the bytes never
 *     hit the wire.
 *   - **429 `bandwidth_limit_exceeded`** — rolling-window upload/download
 *     bandwidth quota. Body includes `retry_after_seconds`; we mirror
 *     that into a human time in the toast.
 *   - **402 `storage_quota_exceeded`** — total stored bytes across all
 *     saves. Returned by `init_upload` after the per-save cap check.
 *
 * Tauri reaches us with whatever the Rust side returned, which today is
 * usually a string from `format!("{err}")`. We try to parse it as JSON
 * first; if that fails we fall through to a generic message. Either way
 * the helper always returns a non-empty string so the caller can hand
 * it straight to `toastError`.
 *
 * Keeping this in one place means a new error code only needs to grow
 * one switch arm + one i18n key, instead of being duplicated across
 * every page that uploads or downloads.
 */
import { get } from "svelte/store";
import { _ } from "svelte-i18n";

/** Shape produced by `crates/hoard-server/src/cloud/*.rs` error types. */
type StructuredCloudError = {
  code?: string;
  error?: string;
  plan?: string;
  limit_bytes?: number;
  used_bytes?: number;
  requested_bytes?: number;
  window_secs?: number;
  retry_after_seconds?: number;
};

function parse(err: unknown): StructuredCloudError | null {
  if (typeof err !== "string") return null;
  // Tauri sometimes prefixes the body with the HTTP status; trim that
  // off so `JSON.parse` has a chance.
  const trimmed = err.trim();
  const braceIdx = trimmed.indexOf("{");
  if (braceIdx < 0) return null;
  try {
    const parsed = JSON.parse(trimmed.slice(braceIdx));
    if (parsed && typeof parsed === "object" && "code" in parsed) {
      return parsed as StructuredCloudError;
    }
  } catch {
    /* fall through */
  }
  return null;
}

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let i = -1;
  let v = n;
  do {
    v /= 1024;
    i += 1;
  } while (v >= 1024 && i < units.length - 1);
  return `${v.toFixed(v >= 10 ? 0 : 1)} ${units[i]}`;
}

function formatRetry(secs: number): string {
  const t = get(_);
  if (secs < 60) {
    return t("cloud_error.retry_seconds", { values: { count: secs } });
  }
  const minutes = Math.ceil(secs / 60);
  return t("cloud_error.retry_minutes", { values: { count: minutes } });
}

/**
 * Turn whatever Tauri threw into a localised, user-readable error string.
 * Falls back to the raw message (or a generic placeholder) when the
 * error isn't one of our structured shapes.
 */
export function formatCloudError(err: unknown): string {
  const t = get(_);
  const structured = parse(err);
  if (structured?.code) {
    switch (structured.code) {
      case "save_too_large":
        return t("cloud_error.save_too_large", {
          values: {
            requested: formatBytes(structured.requested_bytes ?? 0),
            limit: formatBytes(structured.limit_bytes ?? 0),
          },
        });
      case "bandwidth_limit_exceeded":
        return t("cloud_error.bandwidth_exceeded", {
          values: {
            retry: formatRetry(structured.retry_after_seconds ?? 60),
            limit: formatBytes(structured.limit_bytes ?? 0),
            window:
              Math.round((structured.window_secs ?? 900) / 60).toString() +
              "min",
          },
        });
      case "storage_quota_exceeded":
        return t("cloud_error.storage_exceeded", {
          values: {
            used: formatBytes(structured.used_bytes ?? 0),
            limit: formatBytes(structured.limit_bytes ?? 0),
          },
        });
    }
  }
  if (typeof err === "string" && err.length > 0) return err;
  if (err instanceof Error && err.message.length > 0) return err.message;
  return t("cloud_error.generic");
}

/** Returns the structured error code if recognisable, otherwise `null`. */
export function cloudErrorCode(err: unknown): string | null {
  return parse(err)?.code ?? null;
}
