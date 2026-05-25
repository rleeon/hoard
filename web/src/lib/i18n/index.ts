import { browser } from '$app/environment';
import { init, register, locale, getLocaleFromNavigator } from 'svelte-i18n';

register('en', () => import('./en.json'));
register('es', () => import('./es.json'));

export function setupI18n() {
  init({
    fallbackLocale: 'en',
    initialLocale: browser ? getLocaleFromNavigator() : 'en'
  });
}

export { locale };
