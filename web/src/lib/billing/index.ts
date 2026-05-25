import type { BillingCycle, PlanId } from '../types';

/**
 * BillingProvider — swap-in interface for the payments backend.
 * Today: Lemon Squeezy (Merchant of Record, hosted checkout). Swapping
 * to Stripe means writing one new file and changing the export below.
 */
export interface BillingProvider {
  /** Returns the URL the user should be redirected to in order to upgrade. */
  checkoutUrl(plan: Exclude<PlanId, 'free'>, cycle: BillingCycle, email?: string): string;
  /** URL of the hosted billing portal (manage card, invoices, cancel). */
  customerPortalUrl(): string;
}

import { lemonsqueezyBilling } from './lemonsqueezy';

export const billing: BillingProvider = lemonsqueezyBilling;
