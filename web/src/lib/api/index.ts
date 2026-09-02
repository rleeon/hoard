import { config } from '../config';
import { TERMS_VERSION } from '../legal';
import { auth } from '../auth';
import type { AccountProfile, BillingCycle, DeviceRow, PlanId, UsageEvent } from '../types';

/**
 * Thin wrapper over hoard-server-cloud. Endpoints map 1:1 to what the
 * Rust server exposes. Replacing the backend means changing this file.
 */

/**
 * A non-2xx response, carrying everything a caller needs to say something
 * true: `status` to branch on (401 → the session died, send them back to
 * login), `code` (the server's stable machine code) to match on without
 * string-sniffing a message, and `detail` (the server's own sentence) as the
 * concrete reason a user can quote in a bug report.
 *
 * `status === 0` is the special case for "the request never reached the
 * server", DNS, CORS, offline. It reads very differently to a user than a
 * 500, and conflating the two is how "the checkout is broken" and "your wifi
 * dropped" ended up as the same sentence.
 *
 * Every method below throws this on failure. They used to fail three
 * different ways, a flat `Error` whose message was the only clue, a
 * swallowed failure that returned `[]`, or no status check at all, so
 * "unlinking that device failed" and "you have no devices" rendered
 * identically, and a failed account deletion still signed you out and sent
 * you home as though it had worked.
 */
export class ApiError extends Error {
  constructor(
    message: string,
    readonly status: number,
    readonly detail: string,
    readonly code: string = ''
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

/**
 * Authenticated fetch that fails loudly. Returns the `Response` only on 2xx;
 * every other outcome throws an [`ApiError`] carrying the server's own
 * `{error, code}` body (see `hoard-server/src/cloud/errors.rs`).
 */
async function request(path: string, init: RequestInit = {}): Promise<Response> {
  const token = await auth.getAccessToken();
  const headers = new Headers(init.headers);
  if (token) headers.set('Authorization', `Bearer ${token}`);
  headers.set('Accept', 'application/json');

  let res: Response;
  try {
    res = await fetch(`${config.api.baseUrl}${path}`, { ...init, headers });
  } catch (e) {
    const msg = (e as Error).message;
    throw new ApiError(`${path}: ${msg}`, 0, msg, 'unreachable');
  }
  if (res.ok) return res;

  // Errors come back as `{error, code}`. Anything else, a proxy's HTML 502,
  // an empty body, still yields its raw text rather than nothing at all.
  const body = await res.text().catch(() => '');
  let detail = body;
  let code = '';
  try {
    const j = JSON.parse(body) as { error?: string; code?: string };
    if (typeof j.error === 'string') detail = j.error;
    if (typeof j.code === 'string') code = j.code;
  } catch {
    /* not JSON: keep the raw body, it's still better than nothing */
  }
  throw new ApiError(
    `${path} failed: ${res.status}${detail ? ` ${detail}` : ''}`,
    res.status,
    detail,
    code
  );
}

export const api = {
  async me(): Promise<AccountProfile> {
    const res = await request('/v1/me');
    const j = await res.json();
    // Field names map 1:1 onto the server's `Me` wire shape (see
    // hoard-server/src/cloud/routes/me.rs). They are NOT `plan_renews_at` /
    // `storage_bytes` / `devices_count`, using those (the old names) is why
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
    const res = await request('/v1/cloud/checkout', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ plan, interval })
    });
    const j = await res.json();
    return j.url as string;
  },

  async devices(): Promise<DeviceRow[]> {
    const res = await request('/v1/devices');
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
    await request(`/v1/devices/${id}`, { method: 'DELETE' });
  },

  /**
   * Approve a headless CLI's device-pairing request. The phone is signed in;
   * the server mints a fresh session for *this* user and hands it to the
   * waiting CLI. `code` is the short user_code shown by `hoard login`. A
   * wrong or expired code comes back as `ApiError` with `code === 'not_found'`,
   * which the page localizes.
   */
  async approveDevice(code: string): Promise<{ hostname: string | null }> {
    const res = await request('/v1/cloud/device/approve', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ user_code: code })
    });
    const j = await res.json();
    return { hostname: (j.hostname as string) ?? null };
  },

  async usageEvents(limit = 20): Promise<UsageEvent[]> {
    const res = await request(`/v1/usage?limit=${limit}`);
    const j = await res.json();
    return j.events ?? [];
  },

  async serverHealth(): Promise<{
    reachable: boolean;
    status: 'ok' | 'degraded' | null;
    version: string | null;
  }> {
    // Deliberately NOT `request`: this one's job is to *report* reachability,
    // so an outage is its expected result, not an exception.
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
    await request('/v1/me/export', { method: 'POST' });
  },

  async deleteAccount(): Promise<void> {
    await request('/v1/me', { method: 'DELETE' });
  },

  /**
   * Record that this account accepted the Terms. Called once a browser
   * sign-in completes, the tick happens on /login, before there is a session
   * to attach it to.
   *
   * The server is idempotent per (user, version) and rejects any version other
   * than the one it currently publishes, so an old cached bundle gets a 400
   * instead of quietly filing an acceptance of a text nobody read.
   */
  async acceptTerms(source: 'web' = 'web'): Promise<void> {
    await request('/v1/me/terms', {
      method: 'POST',
      body: JSON.stringify({ version: TERMS_VERSION, source })
    });
  }
};
