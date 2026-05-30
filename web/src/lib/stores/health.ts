import { writable, type Readable } from 'svelte/store';
import { api } from '$lib/api';

export type HealthState = 'loading' | 'ok' | 'degraded' | 'down';

const TTL_MS = 30_000;

let cache: { state: HealthState; at: number } | null = null;
let inflight: Promise<HealthState> | null = null;

const store = writable<HealthState>('loading');

async function refresh(): Promise<HealthState> {
  if (inflight) return inflight;
  inflight = (async () => {
    try {
      const r = await api.serverHealth();
      const next: HealthState = !r.reachable ? 'down' : r.status === 'degraded' ? 'degraded' : 'ok';
      cache = { state: next, at: Date.now() };
      store.set(next);
      return next;
    } catch {
      cache = { state: 'down', at: Date.now() };
      store.set('down');
      return 'down';
    } finally {
      inflight = null;
    }
  })();
  return inflight;
}

export const health: Readable<HealthState> & { ensure: () => Promise<HealthState> } = {
  subscribe: store.subscribe,
  ensure() {
    if (cache && Date.now() - cache.at < TTL_MS) {
      store.set(cache.state);
      return Promise.resolve(cache.state);
    }
    return refresh();
  }
};
