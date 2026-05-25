import { config } from '../config';
import { auth } from '../auth';
import type { AccountProfile, DeviceRow, UsageEvent } from '../types';

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
    return {
      userId: j.user_id,
      email: j.email,
      displayName: j.display_name ?? null,
      avatarUrl: j.avatar_url ?? null,
      plan: j.plan,
      planRenewsAt: j.plan_renews_at ?? null,
      planCancelAt: j.plan_cancel_at ?? null,
      storageBytes: j.storage_bytes ?? 0,
      devicesCount: j.devices_count ?? 0
    };
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

  async serverHealth(): Promise<{ ok: boolean; version: string | null }> {
    try {
      const res = await fetch(`${config.api.baseUrl}/v1/health`);
      if (!res.ok) return { ok: false, version: null };
      const j = await res.json();
      return { ok: true, version: j.version ?? null };
    } catch {
      return { ok: false, version: null };
    }
  },

  async requestAccountExport(): Promise<void> {
    await authedFetch('/v1/me/export', { method: 'POST' });
  },

  async deleteAccount(): Promise<void> {
    await authedFetch('/v1/me', { method: 'DELETE' });
  }
};
