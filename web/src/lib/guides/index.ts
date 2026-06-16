import { marked } from 'marked';
import { LOCALES, DEFAULT_LOCALE, type Locale } from '$lib/i18n/locales';

/**
 * File-based guide content. Each guide is a folder under `content/<slug>/` with
 * one Markdown file per locale (`en.md`, `es.md`, …). Frontmatter carries the
 * title/description/order; the body is rendered to HTML at build time. Routing
 * uses a fixed `/guides/<slug>` segment in every language — what ranks is the
 * localized title/H1/body + hreflang, not a translated URL — so adding a guide
 * is just dropping 8 Markdown files in, no routing changes.
 */

export type GuideMeta = {
  slug: string;
  title: string;
  description: string;
  /** Lower sorts first in the index. */
  order: number;
  /** ISO date of the last meaningful edit; feeds JSON-LD + sitemap. */
  updated: string;
};

export type Guide = GuideMeta & { html: string };

marked.setOptions({ gfm: true, breaks: false });

const raw = import.meta.glob('./content/*/*.md', {
  query: '?raw',
  import: 'default',
  eager: true
}) as Record<string, string>;

/** Minimal `key: value` frontmatter parser (no YAML dep, no Buffer). */
function split(src: string): { meta: Record<string, string>; body: string } {
  const m = src.match(/^---\r?\n([\s\S]*?)\r?\n---\r?\n?([\s\S]*)$/);
  if (!m) return { meta: {}, body: src };
  const meta: Record<string, string> = {};
  for (const line of m[1].split(/\r?\n/)) {
    const i = line.indexOf(':');
    if (i === -1) continue;
    const k = line.slice(0, i).trim();
    let v = line.slice(i + 1).trim();
    if (
      (v.startsWith('"') && v.endsWith('"')) ||
      (v.startsWith("'") && v.endsWith("'"))
    )
      v = v.slice(1, -1);
    meta[k] = v;
  }
  return { meta, body: m[2] };
}

// registry[slug][locale] = Guide
const registry: Record<string, Partial<Record<Locale, Guide>>> = {};

for (const [path, src] of Object.entries(raw)) {
  const m = path.match(/\/content\/([^/]+)\/([^/]+)\.md$/);
  if (!m) continue;
  const [, slug, lang] = m;
  if (!(LOCALES as readonly string[]).includes(lang)) continue;
  const { meta, body } = split(src);
  (registry[slug] ??= {})[lang as Locale] = {
    slug,
    title: meta.title ?? slug,
    description: meta.description ?? '',
    order: Number(meta.order ?? 999),
    updated: meta.updated ?? '',
    html: marked.parse(body.trim()) as string
  };
}

/** Resolve a guide for a locale, falling back to English if untranslated. */
export function getGuide(slug: string, locale: Locale): Guide | null {
  const byLang = registry[slug];
  if (!byLang) return null;
  return byLang[locale] ?? byLang[DEFAULT_LOCALE] ?? null;
}

/** Index listing for a locale (with English fallback), sorted by `order`. */
export function listGuides(locale: Locale): GuideMeta[] {
  return Object.keys(registry)
    .map((slug) => getGuide(slug, locale))
    .filter((g): g is Guide => g !== null)
    .sort((a, b) => a.order - b.order || a.title.localeCompare(b.title));
}

/** Every guide slug (for prerender entries). */
export function guideSlugs(): string[] {
  return Object.keys(registry);
}
