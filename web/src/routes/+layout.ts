import { setupI18n } from '$lib/i18n';
import { startSessionTracking } from '$lib/stores/session';
import { browser } from '$app/environment';

setupI18n();
if (browser) startSessionTracking();

export const prerender = false;
export const ssr = false;
