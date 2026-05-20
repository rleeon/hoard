#!/usr/bin/env node
// Compara los 8 locales JSON contra en.json. Falla con exit 1 si:
// - una key existe en en.json pero falta en otro locale
// - una key existe en otro locale que no está en en.json (key huérfana)
// El script solo lee + compara claves, no valores.

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
