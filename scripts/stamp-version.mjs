#!/usr/bin/env node
// Single-source version stamper.
//
// One number lives in three files that must agree:
//   1. Cargo.toml            -> [workspace.package] version
//   2. crates/hoard-desktop/tauri.conf.json -> "version"
//   3. crates/hoard-desktop/ui/package.json -> "version"
//
// Instead of editing all three by hand, run this with the target version
// (or no arg to derive it from the latest git tag):
//
//   node scripts/stamp-version.mjs 1.8.6
//   node scripts/stamp-version.mjs            # uses `git describe --tags`
//
// The public-facing version shown in the web and the app's updater is read
// live from github.com/rleeon/hoard/releases/latest, so the only manual step
// in a release is: stamp -> commit -> tag.

import { readFileSync, writeFileSync } from 'node:fs';
import { execSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));

function fromGitTag() {
  try {
    return execSync('git describe --tags --abbrev=0', { cwd: root })
      .toString()
      .trim()
      .replace(/^v/, '');
  } catch {
    return null;
  }
}

const arg = process.argv[2]?.replace(/^v/, '');
const version = arg || fromGitTag();

if (!version || !/^\d+\.\d+\.\d+$/.test(version)) {
  console.error(
    `Usage: node scripts/stamp-version.mjs <X.Y.Z>\n` +
      `Got: ${version ?? '(no version and no git tag found)'}`
  );
  process.exit(1);
}

function patch(rel, transform) {
  const path = new URL(rel, new URL('..', import.meta.url));
  const before = readFileSync(path, 'utf8');
  const after = transform(before);
  if (before === after) {
    console.log(`  unchanged: ${rel}`);
    return;
  }
  writeFileSync(path, after);
  console.log(`  stamped:   ${rel}`);
}

console.log(`Stamping version ${version}`);

// Cargo.toml: only the [workspace.package] version line. Anchored to the
// first `version = "..."` after the [workspace.package] header.
patch('Cargo.toml', (s) =>
  s.replace(
    /(\[workspace\.package\][\s\S]*?\nversion\s*=\s*")[^"]+(")/,
    `$1${version}$2`
  )
);

// JSON files: the top-level "version" key.
const jsonVersion = (s) => s.replace(/("version"\s*:\s*")[^"]+(")/, `$1${version}$2`);
patch('crates/hoard-desktop/tauri.conf.json', jsonVersion);
patch('crates/hoard-desktop/ui/package.json', jsonVersion);

console.log('Done. Next: commit, then `git tag v' + version + '`.');
