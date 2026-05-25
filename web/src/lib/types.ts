export type PlanId = 'free' | 'pro' | 'proplus';
export type BillingCycle = 'monthly' | 'yearly';

export interface PlanLimits {
  id: PlanId;
  storageBytes: number;
  devices: number | null;
  saves: number | null;
  retentionDays: number;
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
