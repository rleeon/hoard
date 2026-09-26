/**
 * The Hoard-Wrapped card's phrases.
 *
 * The card finishes with a phrase picked at random from the range's most-played
 * game: if the most-played one is a Fallout, any of them, out comes "war never
 * changes"; if we do not recognise the game, it draws from the generic bag. The user
 * can always write their own over it (it stays local) or roll the dice again.
 *
 * The phrases themselves are in the locale files, in the app's eight languages,
 * as `wrapped.quote.<game>_<n>`; this file only knows which game gets which.
 *
 * The matching is by *slug* and on token boundaries: the slug is wrapped in dashes
 * (`-elden-ring-`) and the patterns are tried against that shape, so `rust` matches
 * the game Rust but neither `rustler` nor `trust-issues`.
 */

type QuoteEntry = {
  /** Id estable, se usa para la semilla del dado y para depurar. */
  id: string;
  /** Se prueba contra `-<slug>-`. */
  test: RegExp;
  /** How many phrases it has in the locale files. */
  count: number;
};

/** Phrases per game. Adding a new one is an entry here plus its keys in the
 *  eight locale files. */
const BY_GAME: QuoteEntry[] = [
  { id: "fallout", test: /-fallout-/, count: 2 },
  { id: "rust", test: /-rust-/, count: 2 },
  { id: "minecraft", test: /-minecraft-/, count: 2 },
  { id: "skyrim", test: /-skyrim-|-elder-scrolls-/, count: 2 },
  { id: "elden-ring", test: /-elden-ring-/, count: 2 },
  { id: "souls", test: /-dark-souls-|-demons?-souls-|-bloodborne-|-sekiro-/, count: 2 },
  { id: "stardew", test: /-stardew-/, count: 2 },
  { id: "factorio", test: /-factorio-/, count: 2 },
  { id: "terraria", test: /-terraria-/, count: 2 },
  { id: "cyberpunk", test: /-cyberpunk-/, count: 2 },
  { id: "witcher", test: /-witcher-/, count: 2 },
  { id: "gta", test: /-grand-theft-auto-|-gta-/, count: 2 },
  { id: "red-dead", test: /-red-dead-/, count: 2 },
  { id: "hollow-knight", test: /-hollow-knight-|-silksong-/, count: 2 },
  { id: "hades", test: /-hades-/, count: 2 },
  { id: "no-mans-sky", test: /-no-mans?-sky-/, count: 2 },
  { id: "subnautica", test: /-subnautica-/, count: 2 },
  { id: "baldurs-gate", test: /-baldurs?-gate-/, count: 2 },
  { id: "civilization", test: /-civilization-|-sid-meiers-civ/, count: 2 },
  { id: "portal", test: /-portal-/, count: 2 },
  { id: "doom", test: /-doom-/, count: 2 },
  { id: "valheim", test: /-valheim-/, count: 2 },
];

/** The bag for games we do not recognise. */
const GENERIC_COUNT = 6;

function quoteKey(id: string, n: number): string {
  return `wrapped.quote.${id.replace(/-/g, "_")}_${n}`;
}

/** How many games the catalogue recognises. The UI uses it to boast. */
export const KNOWN_GAMES = BY_GAME.length;

/** Normalises a slug to `-token-token-` so it can be matched on boundaries. */
function bounded(slug: string): string {
  return `-${slug.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "")}-`;
}

/** The entry a slug maps to, or `null` when it is not in the catalogue. */
export function matchGame(slug: string | null | undefined): string | null {
  if (!slug) return null;
  const key = bounded(slug);
  return BY_GAME.find((g) => g.test.test(key))?.id ?? null;
}

/**
 * The bag of phrases for a slug. When we recognise the game, **only** its own come
 * out: the point is that somebody who plays a Fallout reads the line about war, not
 * a filler phrase. The generic ones are for when we do not know what they play.
 */
export function quotesFor(slug: string | null | undefined): string[] {
  const id = matchGame(slug);
  const entry = BY_GAME.find((g) => g.id === id);
  const [name, count] = entry ? [entry.id, entry.count] : ["generic", GENERIC_COUNT];
  return Array.from({ length: count }, (_, i) => quoteKey(name, i + 1));
}

/**
 * Picks a phrase deterministically: the same seed and the same game always give the
 * same phrase, so the card does not flicker between renders and the dice button is
 * no more than "bump the seed".
 */
export function pickQuote(slug: string | null | undefined, seed: number): string {
  const pool = quotesFor(slug);
  return pool[Math.abs(Math.trunc(seed)) % pool.length];
}
