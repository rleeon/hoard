import { config } from '../config';
import type { BillingProvider } from './index';

export const lemonsqueezyBilling: BillingProvider = {
  checkoutUrl(plan, cycle, email) {
    const key = `${plan}_${cycle}` as keyof typeof config.billing.checkout;
    const base = config.billing.checkout[key];
    if (!base) return '#';
    const url = new URL(base);
    if (email) {
      url.searchParams.set('checkout[email]', email);
    }
    return url.toString();
  },
  customerPortalUrl() {
    return config.billing.customerPortal || '#';
  }
};
