import { config } from '../config';
import { auth } from '../auth';
import type { AccountProfile, BillingCycle, DeviceRow, PlanId, UsageEvent } from '../types';

/**
 * Thin wrapper over hoard-server-cloud. Endpoints map 1:1 to what the
 * Rust server exposes. Replacing the backend means changing this file.
 */

async function authedFetch(path: string, init: RequestInit = {}): Promise<Response> {
  const token = await auth.getAccessToken();
  const headers = new Headers(init.headers);
  if (token) headers.set('Authorization', `Bearer ${token}`);
  headers.set('Accept', 'application/json');
  return fetch(`${config.api.baseUrl}${path}`, { ...init, headers });
}

export const api = {
  async me(): Promise<AccountProfile> {
    const res = await authedFetch('/v1/me');
    if (!res.ok) throw new Error(`me failed: ${res.status}`);
    const j = await res.json();
    // Field names map 1:1 onto the server's `Me` wire shape (see
    // hoard-server/src/cloud/routes/me.rs). They are NOT `plan_renews_at` /
    // `storage_bytes` / `devices_count` — using those (the old names) is why
    // the account page showed 0 for everything.
    return {
      userId: j.user_id,
      email: j.email,
      displayName: j.display_name ?? null,
      avatarUrl: j.avatar_url ?? null,
      plan: j.plan,
      subscriptionStatus: j.subscription_status ?? null,
      planRenewsAt: j.renews_at ?? null,
      planCancelAt: j.cancel_at ?? null,
      storageBytes: j.storage_used_bytes ?? 0,
      storageLimitBytes: j.storage_limit_bytes ?? 0,
      devicesCount: j.devices_used ?? 0,
      devicesLimit: j.devices_limit ?? 0
    };
  },

  /**
   * Create a Polar checkout session server-side. The server reads the user_id
   * + email from the JWT, stamps `metadata.user_id` so the webhook can map the
   * subscription back, and returns the hosted checkout URL to redirect to.
   */
  async createCheckout(plan: Exclude<PlanId, 'free'>, cycle: BillingCycle): Promise<string> {
    const interval = cycle === 'yearly' ? 'year' : 'month';
    const res = await authedFetch('/v1/cloud/checkout', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ plan, interval })
    });
    if (!res.ok) {
      const detail = await res.text().catch(() => '');
      throw new Error(`checkout failed: ${res.status} ${detail}`);
    }
    const j = await res.json();
    return j.url as string;
  },

  async devices(): Promise<DeviceRow[]> {
    const res = await authedFetch('/v1/devices');
    if (!res.ok) return [];
    const j = await res.json();
    return (j.devices ?? []).map((d: Record<string, unknown>) => ({
      id: d.id as string,
      deviceName: (d.device_name as string) ?? 'Unknown',
      deviceKind: (d.device_kind as string) ?? null,
      lastSeenAt: d.last_seen_at as string,
      createdAt: d.created_at as string
    }));
  },

  async unlinkDevice(id: string): Promise<void> {
    await authedFetch(`/v1/devices/${id}`, { method: 'DELETE' });
  },

  async usageEvents(limit = 20): Promise<UsageEvent[]> {
    const res = await authedFetch(`/v1/usage?limit=${limit}`);
    if (!res.ok) return [];
    const j = await res.json();
    return j.events ?? [];
  },

  async serverHealth(): Promise<{
    reachable: boolean;
    status: 'ok' | 'degraded' | null;
    version: string | null;
  }> {
    try {
      const res = await fetch(`${config.api.baseUrl}/v1/health`);
      // The server answered but with an error code: it's up but not well.
      if (!res.ok) return { reachable: true, status: 'degraded', version: null };
      const j = await res.json();
      const status = j.status === 'degraded' ? 'degraded' : 'ok';
      return { reachable: true, status, version: j.version ?? null };
    } catch {
      // Network/CORS failure: nothing answered → treat as a hard outage.
      return { reachable: false, status: null, version: null };
    }
  },

  async requestAccountExport(): Promise<void> {
    await authedFetch('/v1/me/export', { method: 'POST' });
  },

  async deleteAccount(): Promise<void> {
    await authedFetch('/v1/me', { method: 'DELETE' });
  }
};
