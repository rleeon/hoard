import { env } from '$env/dynamic/public';

function required(key: string, value: string | undefined): string {
  if (!value) {
    throw new Error(`Missing required public env var: ${key}`);
  }
  return value;
}

export const config = {
  supabase: {
    url: env.PUBLIC_SUPABASE_URL ?? '',
    anonKey: env.PUBLIC_SUPABASE_ANON_KEY ?? ''
  },
  api: {
    baseUrl: env.PUBLIC_API_URL ?? 'https://api.hoard.services'
  },
  billing: {
    customerPortal: env.PUBLIC_BILLING_PORTAL ?? '',
    checkout: {
      pro_monthly: env.PUBLIC_CHECKOUT_PRO_MONTHLY ?? '',
      pro_yearly: env.PUBLIC_CHECKOUT_PRO_YEARLY ?? ''
    }
  }
} as const;

export function assertConfig() {
  required('PUBLIC_SUPABASE_URL', config.supabase.url);
  required('PUBLIC_SUPABASE_ANON_KEY', config.supabase.anonKey);
}
