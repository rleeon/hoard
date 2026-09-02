/**
 * Live device list for the Eye panel, fed by the `hoard://devices` Tauri
 * event.
 *
 * The Rust side (commands/cloud_feed.rs) fetches `GET /v1/devices` whenever
 * Supabase Realtime pushes a `devices` change (a sibling machine's heartbeat,
 * a game starting, a closing beat), plus a timed fallback, and re-emits the
 * response body as the event payload. This store just mirrors the latest
 * snapshot; there is nothing to persist (presence is live by definition).
 *
 * A self-hosted session gets no Realtime push, so nothing arrives on its own:
 * there, `refreshDevices()` below asks the server directly while the panel is
 * open. Same endpoint, same shape, only who does the asking changes.
 */
import { writable } from "svelte/store";

/** One game a remote device is running: slug + RFC3339 session start. */
export type RemotePlaying = {
  slug: string;
  since?: string | null;
};

/** Wire shape of one row of `GET /v1/devices` (see cloud/routes/me.rs). */
export type RemoteDevice = {
  id: string;
  device_name: string;
  device_kind?: string | null;
  /** "linux" | "windows" | "macos", free-form, from the client's header. */
  os?: string | null;
  last_seen_at?: string | null;
  created_at?: string | null;
  /** Heartbeat fresh and no closing beat received. */
  online: boolean;
  /** Games running right now, most recently started first; only ever
   *  populated while online. Empty = idle. */
  playing?: RemotePlaying[] | null;
  /** True for the row matching this machine's fingerprint. */
  this_device: boolean;
};

export const remoteDevices = writable<RemoteDevice[]>([]);

let listener: (() => void) | null = null;

/** Subscribe to `hoard://devices` Tauri events, then ask Rust for a fresh
 *  snapshot. The refresh matters: the boot-time fetch may emit before this
 *  listener is armed (lost), and without it the Eye panel would stay empty
 *  until the next heartbeat or poll tick. Call once at boot (next to
 *  `initServerNotifications`); idempotent. */
export async function initDevicesFeed(): Promise<void> {
  if (listener) return;
  try {
    const { listen } = await import("@tauri-apps/api/event");
    const unlisten = await listen<{ devices?: RemoteDevice[] }>(
      "hoard://devices",
      (e) => {
        remoteDevices.set(e.payload?.devices ?? []);
      },
    );
    listener = unlisten;
  } catch {
    /* Tauri not available (e.g. dev in browser), no-op */
  }
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("devices_refresh");
  } catch {
    /* signed out / browser dev, the realtime/poll path covers it later */
  }
}

/**
 * Ask the server for the device list right now and publish it.
 *
 * The Eye panel calls this while it's open. It's the only source on a
 * self-hosted session (no Realtime to push it) and a cheap freshener on a cloud
 * one, the endpoint is the same in both, and the Rust side picks whichever
 * session is signed in.
 *
 * Silent on failure by design: a device list that didn't load must never paint
 * an error over the panel. The list simply stays as it was.
 */
export async function refreshDevices(): Promise<void> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    const out = await invoke<{ devices?: RemoteDevice[] }>("devices_list");
    remoteDevices.set(out?.devices ?? []);
  } catch {
    /* signed out, server too old, or browser dev, keep what we have */
  }
}
