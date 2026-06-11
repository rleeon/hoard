import { setupI18n } from '$lib/i18n';
import { startSessionTracking } from '$lib/stores/session';
import { browser } from '$app/environment';

setupI18n();
if (browser) startSessionTracking();

export const prerender = true;
export const ssr = true;
export const trailingSlash = 'never';
