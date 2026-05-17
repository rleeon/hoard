#!/usr/bin/env node
/**
 * Locale parity checker.
 *
 * For every locale under `crates/hoard-desktop/ui/src/lib/i18n/locales/`:
 *
 *   1. Parses as JSON. A parse error is a hard fail (the app would crash on
 *      svelte-i18n init otherwise).
 *   2. Compared against `en.json`, the source of truth. Missing keys → fail.
 *      Extra keys (present locally but absent in en) → warn only — sometimes
 *      they're stragglers from a partial revert and we'd rather see them than
 *      silently drop them in a CI run.
 *   3. Checks that `{name}`-style placeholders match the en version, so a
 *      translator can't accidentally drop a variable and trigger an "undefined
 *      interpolation" at runtime.
 *
 * No node_modules — uses only the standard library. Fast enough to run in the
 * pre-commit hook.
 */
import { readFileSync, readdirSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const localesDir = join(
  here,
  "..",
  "crates",
  "hoard-desktop",
  "ui",
  "src",
  "lib",
  "i18n",
  "locales",
);

const SOURCE = "en";

const errors = [];
const warnings = [];

function loadLocale(name) {
  const path = join(localesDir, `${name}.json`);
  let raw;
  try {
    raw = readFileSync(path, "utf8");
  } catch (e) {
    errors.push(`${name}: cannot read ${path}: ${e.message}`);
    return null;
  }
  try {
    return JSON.parse(raw);
  } catch (e) {
    errors.push(`${name}: invalid JSON — ${e.message}`);
    return null;
  }
}

/**
 * Extract the set of placeholder identifiers from an svelte-i18n message.
 *
 * Handles both simple placeholders (`{name}`) and ICU plural / select blocks
 * (`{count, plural, one {…} other {…}}`). Inside an ICU branch we recurse so
 * variables referenced from a branch body (`Restored v{version}`) count too,
 * but the branch *names* (`one`, `other`, `=0`) and any literal words that
 * happen to live there (`{line}`, `{lines}`) are correctly ignored — those
 * are translatable text, not placeholders.
 *
 * Returns a `Set<string>` so order-of-appearance and duplicate counts don't
 * matter for the comparison.
 */
function placeholders(str) {
  const out = new Set();
  if (typeof str !== "string") return out;
  let i = 0;
  while (i < str.length) {
    if (str[i] !== "{") {
      i++;
      continue;
    }
    // Find the matching '}' for this top-level '{' (depth-aware).
    let depth = 1;
    let j = i + 1;
    while (j < str.length && depth > 0) {
      if (str[j] === "{") depth++;
      else if (str[j] === "}") depth--;
      if (depth > 0) j++;
    }
    const content = str.slice(i + 1, j);
    if (/^[a-zA-Z_][a-zA-Z0-9_]*$/.test(content)) {
      // Simple `{name}` placeholder.
      out.add(content);
    } else {
      const icu = content.match(
        /^([a-zA-Z_][a-zA-Z0-9_]*)\s*,\s*(plural|select|selectordinal)\s*,/,
      );
      if (icu) {
        // The count / discriminator variable is itself a placeholder.
        out.add(icu[1]);
        // Walk the branch bodies and recurse so we still capture variables
        // referenced inside them (e.g. `{version}` inside `one {…}`).
        let rest = content.slice(icu[0].length);
        let k = 0;
        while (k < rest.length) {
          while (k < rest.length && rest[k] !== "{") k++;
          if (k >= rest.length) break;
          let d = 1;
          let bs = k + 1;
          let be = bs;
          while (be < rest.length && d > 0) {
            if (rest[be] === "{") d++;
            else if (rest[be] === "}") d--;
            if (d > 0) be++;
          }
          for (const p of placeholders(rest.slice(bs, be))) out.add(p);
          k = be + 1;
        }
      }
      // Otherwise: unrecognised pattern — leave it alone. We don't use other
      // ICU forms in this codebase.
    }
    i = j + 1;
  }
  return out;
}

function setsEqual(a, b) {
  if (a.size !== b.size) return false;
  for (const v of a) if (!b.has(v)) return false;
  return true;
}

function fmtSet(s) {
  return [...s].sort().join(", ");
}

const localeFiles = readdirSync(localesDir)
  .filter((f) => f.endsWith(".json"))
  .map((f) => f.replace(/\.json$/, ""));

const source = loadLocale(SOURCE);
if (!source) {
  console.error(`fatal: ${SOURCE}.json failed to load`);
  process.exit(1);
}
const sourceKeys = Object.keys(source);

for (const locale of localeFiles) {
  if (locale === SOURCE) continue;
  const data = loadLocale(locale);
  if (!data) continue;

  const localeKeys = new Set(Object.keys(data));

  // Missing keys → hard error.
  for (const key of sourceKeys) {
    if (!localeKeys.has(key)) {
      errors.push(`${locale}: missing key "${key}"`);
      continue;
    }
    const want = placeholders(source[key]);
    const got = placeholders(data[key]);
    if (!setsEqual(want, got)) {
      errors.push(
        `${locale}: placeholders for "${key}" diverge — en has [${fmtSet(
          want,
        )}], ${locale} has [${fmtSet(got)}]`,
      );
    }
  }

  // Extra keys → warning only.
  const sourceKeySet = new Set(sourceKeys);
  for (const key of localeKeys) {
    if (!sourceKeySet.has(key)) {
      warnings.push(`${locale}: stray key "${key}" not present in ${SOURCE}`);
    }
  }
}

for (const w of warnings) console.warn(`warn: ${w}`);
for (const e of errors) console.error(`error: ${e}`);

if (errors.length) {
  console.error(
    `\n${errors.length} i18n error(s) across ${localeFiles.length} locales`,
  );
  process.exit(1);
}

console.log(
  `i18n ok — ${localeFiles.length} locales, ${sourceKeys.length} keys${
    warnings.length ? `, ${warnings.length} warning(s)` : ""
  }`,
);
