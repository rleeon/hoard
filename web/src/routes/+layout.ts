import { setupI18n, waitLocale } from '$lib/i18n';
import { startSessionTracking } from '$lib/stores/session';
import { browser } from '$app/environment';

setupI18n();
if (browser) startSessionTracking();

// Block the first render until the active locale's messages are loaded, so the
// prerendered HTML is never a flash of raw keys. Locale-prefixed routes set the
// real locale in their own layout before this resolves on navigation.
export const load = async () => {
  await waitLocale();
};

export const prerender = true;
export const ssr = true;
export const trailingSlash = 'never';
