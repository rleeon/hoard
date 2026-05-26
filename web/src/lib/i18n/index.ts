import { browser } from '$app/environment';
import { init, addMessages, locale, getLocaleFromNavigator } from 'svelte-i18n';
import en from './en.json';
import es from './es.json';

addMessages('en', en);
addMessages('es', es);

function pickInitialLocale(): string {
  if (!browser) return 'en';
  const detected = getLocaleFromNavigator() ?? 'en';
  return detected.toLowerCase().startsWith('es') ? 'es' : 'en';
}

export function setupI18n() {
  init({
    fallbackLocale: 'en',
    initialLocale: pickInitialLocale()
  });
}

export { locale };
