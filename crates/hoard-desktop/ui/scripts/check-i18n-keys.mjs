#!/usr/bin/env node
// Compares the 8 JSON locales against en.json. It exits 1 when:
// - a key exists in en.json but is missing from another locale
// - a key exists in another locale that is not in en.json (an orphan key)
// The script only reads and compares keys, never values.

import { readFileSync, readdirSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const localesDir = join(here, "..", "src", "lib", "i18n", "locales");
const files = readdirSync(localesDir).filter((f) => f.endsWith(".json"));

const load = (file) => JSON.parse(readFileSync(join(localesDir, file), "utf8"));
const en = load("en.json");
const enKeys = new Set(Object.keys(en));

let problems = 0;
for (const file of files) {
  if (file === "en.json") continue;
  const data = load(file);
  const keys = new Set(Object.keys(data));
  const missing = [...enKeys].filter((k) => !keys.has(k));
  const orphan = [...keys].filter((k) => !enKeys.has(k));
  if (missing.length || orphan.length) {
    console.error(`${file}:`);
    if (missing.length)
      console.error(`  missing (${missing.length}): ${missing.join(", ")}`);
    if (orphan.length)
      console.error(`  orphan (${orphan.length}): ${orphan.join(", ")}`);
    problems++;
  }
}

if (problems > 0) {
  console.error(`\n${problems} locale(s) out of sync with en.json`);
  process.exit(1);
}
console.log(`all ${files.length} locales in sync (${enKeys.size} keys)`);
