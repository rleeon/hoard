// What the in-app pricing page shows. A copy of `web/src/lib/plans.ts`, and like
// that one it has to match `crates/hoard-server/src/cloud/plans.rs`: the server
// enforces the limits, these only describe them. Change all three together.

const GB = 1024 * 1024 * 1024;
const MB = 1024 * 1024;

export type PlanId = "free" | "pro";
/** `once` is the one-year pass: paid a single time, never renewed. */
export type BillingCycle = "monthly" | "yearly" | "once";

export const PLANS = {
  free: {
    storageBytes: 2 * GB,
    devices: 3,
    maxSaveSizeBytes: 1 * GB,
    maxSaveSizeCeilingBytes: 1536 * MB,
  },
  pro: {
    storageBytes: 100 * GB,
    devices: null,
    maxSaveSizeBytes: 10 * GB,
    maxSaveSizeCeilingBytes: 10 * GB,
    priceMonthly: 1.99,
    // Monthly x 10, two months free.
    priceYearly: 19.99,
    // The pass costs the same as a year of subscription: what it sells is a
    // card that is never charged again, not a discount.
    priceOnce: 19.99,
  },
} as const;

const gb = (bytes: number) => `${(bytes / GB).toFixed(1).replace(/\.0$/, "")} GB`;

export function formatStorage(plan: PlanId): string {
  return gb(PLANS[plan].storageBytes);
}

/** The default per-save cap: "1 GB" on Free, "10 GB" on Pro. */
export function formatSaveSize(plan: PlanId): string {
  return gb(PLANS[plan].maxSaveSizeBytes);
}

/** "10 GB" on Pro, "1 GB – 1.5 GB" on Free, where the user can move the cap. */
export function formatSaveSizeRange(plan: PlanId): string {
  const p = PLANS[plan];
  if (p.maxSaveSizeCeilingBytes <= p.maxSaveSizeBytes) return gb(p.maxSaveSizeBytes);
  return `${gb(p.maxSaveSizeBytes)} – ${gb(p.maxSaveSizeCeilingBytes)}`;
}
