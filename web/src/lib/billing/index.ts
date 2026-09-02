import type { BillingCycle, PlanId } from '../types';

/**
 * BillingProvider, swap-in interface for the payments backend. Hosted
 * checkout via a Merchant of Record. Swapping providers means writing one
 * new file and changing the export below.
 */
export interface BillingProvider {
  /** Returns the URL the user should be redirected to in order to upgrade. */
  checkoutUrl(plan: Exclude<PlanId, 'free'>, cycle: BillingCycle, email?: string): string;
  /** URL of the hosted billing portal (manage card, invoices, cancel). */
  customerPortalUrl(): string;
}

import { checkoutBilling } from './provider';

export const billing: BillingProvider = checkoutBilling;
