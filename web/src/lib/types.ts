export type PlanId = 'free' | 'pro';
export type BillingCycle = 'monthly' | 'yearly';

export interface PlanLimits {
  id: PlanId;
  storageBytes: number;
  /** `null` = unlimited. */
  devices: number | null;
  /** `null` = unlimited. Unlimited on every tier post-1.6.1. */
  saves: number | null;
  /** Always `true` post-1.6.1. Kept as a field so a future tier with a
   *  rolling window can opt out without renaming. */
  versionHistoryForever: boolean;
  /** Per-save upload cap in bytes. Server returns 413 above this. */
  maxSaveSizeBytes: number;
  /** Rolling-window bandwidth quota in bytes (over `bandwidthWindowSecs`). */
  bandwidthQuotaBytes: number;
  bandwidthWindowSecs: number;
  priceMonthly: number;
  priceYearly: number;
}

export interface AccountSession {
  userId: string;
  email: string;
  displayName: string | null;
  avatarUrl: string | null;
}

export interface AccountProfile {
  userId: string;
  email: string;
  displayName: string | null;
  avatarUrl: string | null;
  plan: PlanId;
  planRenewsAt: string | null;
  planCancelAt: string | null;
  storageBytes: number;
  devicesCount: number;
}

export interface DeviceRow {
  id: string;
  deviceName: string;
  deviceKind: string | null;
  lastSeenAt: string;
  createdAt: string;
}

export interface UsageEvent {
  id: number;
  kind: string;
  bytes: number | null;
  saveId: string | null;
  at: string;
}
