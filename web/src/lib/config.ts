import {
  PUBLIC_SUPABASE_URL,
  PUBLIC_SUPABASE_ANON_KEY,
  PUBLIC_API_URL,
  PUBLIC_LS_CHECKOUT_PRO_MONTHLY,
  PUBLIC_LS_CHECKOUT_PRO_YEARLY,
  PUBLIC_LS_CHECKOUT_PROPLUS_MONTHLY,
  PUBLIC_LS_CHECKOUT_PROPLUS_YEARLY,
  PUBLIC_LS_CUSTOMER_PORTAL
} from '$env/static/public';

export const config = {
  supabase: {
    url: PUBLIC_SUPABASE_URL,
    anonKey: PUBLIC_SUPABASE_ANON_KEY
  },
  api: {
    baseUrl: PUBLIC_API_URL || 'https://api.hoard.services'
  },
  billing: {
    customerPortal: PUBLIC_LS_CUSTOMER_PORTAL,
    checkout: {
      pro_monthly: PUBLIC_LS_CHECKOUT_PRO_MONTHLY,
      pro_yearly: PUBLIC_LS_CHECKOUT_PRO_YEARLY,
      proplus_monthly: PUBLIC_LS_CHECKOUT_PROPLUS_MONTHLY,
      proplus_yearly: PUBLIC_LS_CHECKOUT_PROPLUS_YEARLY
    }
  }
} as const;

export function assertConfig() {
  if (!config.supabase.url) throw new Error('Missing PUBLIC_SUPABASE_URL');
  if (!config.supabase.anonKey) throw new Error('Missing PUBLIC_SUPABASE_ANON_KEY');
}
