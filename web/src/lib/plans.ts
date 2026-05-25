import type { PlanId, PlanLimits } from './types';

const GB = 1024 * 1024 * 1024;
const MB = 1024 * 1024;

/**
 * Plan limits for the landing/pricing/account UI. Must match the server's
 * `crates/hoard-server/src/cloud/plans.rs` 1:1 — when one changes, the
 * other has to follow in the same release. Two tiers post-1.6.1; Pro+ was
 * dropped.
 */
export const PLANS: Record<PlanId, PlanLimits> = {
  free: {
    id: 'free',
    storageBytes: 1 * GB,
    devices: 3,
    saves: null,
    versionHistoryForever: true,
    maxSaveSizeBytes: 200 * MB,
    bandwidthQuotaBytes: 500 * MB,
    bandwidthWindowSecs: 15 * 60,
    priceMonthly: 0,
    priceYearly: 0
  },
  pro: {
    id: 'pro',
    storageBytes: 50 * GB,
    devices: null,
    saves: null,
    versionHistoryForever: true,
    maxSaveSizeBytes: 2 * GB,
    bandwidthQuotaBytes: 1 * GB,
    bandwidthWindowSecs: 15 * 60,
    priceMonthly: 4.49,
    priceYearly: 35.99
  }
};

export const PLAN_ORDER: PlanId[] = ['free', 'pro'];

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < MB) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < GB) return `${(bytes / MB).toFixed(1)} MB`;
  return `${(bytes / GB).toFixed(1)} GB`;
}

export function formatPlanQuota(plan: PlanId): string {
  const limit = PLANS[plan].storageBytes;
  if (limit >= GB) return `${Math.round(limit / GB)} GB`;
  return `${Math.round(limit / MB)} MB`;
}

/** "200 MB" / "2 GB" — used in plan-card feature list. */
export function formatMaxSaveSize(plan: PlanId): string {
  const limit = PLANS[plan].maxSaveSizeBytes;
  if (limit >= GB) return `${Math.round(limit / GB)} GB`;
  return `${Math.round(limit / MB)} MB`;
}

/** "500 MB" / "1 GB" — used in plan-card feature list. */
export function formatBandwidthQuota(plan: PlanId): string {
  const limit = PLANS[plan].bandwidthQuotaBytes;
  if (limit >= GB) return `${Math.round(limit / GB)} GB`;
  return `${Math.round(limit / MB)} MB`;
}

export function usagePercent(used: number, plan: PlanId): number {
  const limit = PLANS[plan].storageBytes;
  if (limit === 0) return 0;
  return Math.min(100, Math.round((used / limit) * 100));
}

export function daysUntil(iso: string | null): number | null {
  if (!iso) return null;
  const target = new Date(iso).getTime();
  const now = Date.now();
  const ms = target - now;
  if (ms < 0) return 0;
  return Math.ceil(ms / (1000 * 60 * 60 * 24));
}
