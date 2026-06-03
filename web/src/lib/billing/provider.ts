import { config } from '../config';
import type { BillingProvider } from './index';

/**
 * Generic checkout provider. Reads hosted-checkout URLs from public env
 * (`PUBLIC_CHECKOUT_*`) and appends the customer email as a query param so
 * the Merchant of Record can prefill it. Provider-agnostic — point the env
 * vars at whatever payment backend is live.
 */
export const checkoutBilling: BillingProvider = {
  checkoutUrl(plan, cycle, email) {
    const key = `${plan}_${cycle}` as keyof typeof config.billing.checkout;
    const base = config.billing.checkout[key];
    if (!base) return '#';
    const url = new URL(base);
    if (email) {
      url.searchParams.set('email', email);
    }
    return url.toString();
  },
  customerPortalUrl() {
    return config.billing.customerPortal || '#';
  }
};
