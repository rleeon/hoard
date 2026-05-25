import type { PlanId, PlanLimits } from './types';

const GB = 1024 * 1024 * 1024;
const MB = 1024 * 1024;

export const PLANS: Record<PlanId, PlanLimits> = {
  free: {
    id: 'free',
    storageBytes: 500 * MB,
    devices: 1,
    saves: 3,
    retentionDays: 7,
    priceMonthly: 0,
    priceYearly: 0
  },
  pro: {
    id: 'pro',
    storageBytes: 50 * GB,
    devices: 5,
    saves: null,
    retentionDays: 90,
    priceMonthly: 3.99,
    priceYearly: 39
  },
  proplus: {
    id: 'proplus',
    storageBytes: 200 * GB,
    devices: null,
    saves: null,
    retentionDays: 365,
    priceMonthly: 9.99,
    priceYearly: 99
  }
};

export const PLAN_ORDER: PlanId[] = ['free', 'pro', 'proplus'];

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
