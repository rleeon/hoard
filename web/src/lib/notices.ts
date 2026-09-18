/**
 * The service emails that have a page here, keyed by the slug the server puts
 * in their footer (`cloud/email/mod.rs`, the `notice` field of each template).
 *
 * Keep this list and the templates in step: a slug the server links to and
 * this file does not know about is a 404 in a message somebody already
 * received, and a 404 the CDN then caches.
 */
export const NOTICES = [
  'storage-purge-started',
  'storage-full',
  'save-too-large',
  'archive-expiring',
  'devices-full',
  'export-ready'
] as const;

export type NoticeSlug = (typeof NOTICES)[number];

export function isNoticeSlug(s: string): s is NoticeSlug {
  return (NOTICES as readonly string[]).includes(s);
}

/** i18n key base for a notice: `notices.<underscored slug>.…`. */
export function noticeKey(slug: NoticeSlug): string {
  return `notices.${slug.replace(/-/g, '_')}`;
}

/**
 * Which call to action a notice's page offers, and where it points.
 *
 * These hrefs are used raw, never through `localeHref`: `/account` is a
 * functional route and lives outside the `[[lang]]` tree, so a prefixed
 * `/es/account` is a 404 that only the prerender catches.
 */
export const NOTICE_CTA: Record<NoticeSlug, { key: string; href: string }> = {
  'storage-purge-started': { key: 'notices.cta_storage', href: '/account' },
  'storage-full': { key: 'notices.cta_storage', href: '/account' },
  'save-too-large': { key: 'notices.cta_account', href: '/account' },
  'archive-expiring': { key: 'notices.cta_account', href: '/account' },
  'devices-full': { key: 'notices.cta_devices', href: '/account' },
  'export-ready': { key: 'notices.cta_account', href: '/account' }
};
