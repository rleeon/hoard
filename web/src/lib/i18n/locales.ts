/**
 * Single source of truth for the web's locales. Mirrors the 8 locales the
 * desktop app ships (en, es, de, fr, it, ja, pt, zh). English is the default
 * and is served WITHOUT a URL prefix (`/pricing`); every other locale is
 * served under its prefix (`/es/pricing`, `/de/...`) so each language gets its
 * own indexable URL.
 */
export const DEFAULT_LOCALE = 'en';

export const LOCALES = ['en', 'es', 'de', 'fr', 'it', 'ja', 'pt', 'zh'] as const;
export type Locale = (typeof LOCALES)[number];

/** Every locale except the default — these are the ones that carry a prefix. */
export const PREFIXED_LOCALES: readonly Locale[] = LOCALES.filter((l) => l !== DEFAULT_LOCALE);

/** Native names for the language switcher. */
export const LOCALE_NAMES: Record<Locale, string> = {
  en: 'English',
  es: 'Español',
  de: 'Deutsch',
  fr: 'Français',
  it: 'Italiano',
  ja: '日本語',
  pt: 'Português',
  zh: '中文'
};

/** BCP-47 tags for <link rel="alternate" hreflang>. */
export const HREFLANG: Record<Locale, string> = {
  en: 'en',
  es: 'es',
  de: 'de',
  fr: 'fr',
  it: 'it',
  ja: 'ja',
  pt: 'pt',
  zh: 'zh'
};

/** og:locale values (Open Graph wants `xx_YY`). */
export const OG_LOCALE: Record<Locale, string> = {
  en: 'en_US',
  es: 'es_ES',
  de: 'de_DE',
  fr: 'fr_FR',
  it: 'it_IT',
  ja: 'ja_JP',
  pt: 'pt_PT',
  zh: 'zh_CN'
};

export const SITE_URL = 'https://hoard.services';

export function isLocale(x: string | null | undefined): x is Locale {
  return !!x && (LOCALES as readonly string[]).includes(x);
}

/** URL prefix for a locale: '' for the default, '/xx' otherwise. */
export function localePrefix(l: Locale): string {
  return l === DEFAULT_LOCALE ? '' : `/${l}`;
}

/** Prepend the locale prefix to an app path. `withLocale('/pricing', 'es')` → '/es/pricing'. */
export function withLocale(path: string, l: Locale): string {
  const prefix = localePrefix(l);
  if (path === '/') return prefix || '/';
  return `${prefix}${path}`;
}

/** Drop the locale prefix from a pathname. `/es/pricing` → '/pricing', `/es` → '/'. */
export function stripLocale(pathname: string): string {
  const seg = pathname.split('/')[1];
  if ((PREFIXED_LOCALES as readonly string[]).includes(seg)) {
    return pathname.slice(seg.length + 1) || '/';
  }
  return pathname || '/';
}
