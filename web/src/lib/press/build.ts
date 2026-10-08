import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { PRESS_ASSETS, type PressBuild, type PressRelease } from './assets';

// Build-time only: `vite.config.ts` turns this into the `__HOARD_PRESS__`
// define and the zip endpoint reads the same files. Nothing here may reach a
// browser bundle (node:fs).

/** Pixel size read from the file header, so a swapped image never ships with
 *  the old dimensions on its <img>. */
function measure(b: Buffer): { width: number; height: number } | null {
  if (b.length >= 24 && b.readUInt32BE(0) === 0x89504e47) {
    return { width: b.readUInt32BE(16), height: b.readUInt32BE(20) };
  }
  if (b.length >= 10 && b.toString('latin1', 0, 4) === 'GIF8') {
    return { width: b.readUInt16LE(6), height: b.readUInt16LE(8) };
  }
  if (b.length >= 30 && b.toString('latin1', 0, 4) === 'RIFF' && b.toString('latin1', 8, 12) === 'WEBP') {
    const chunk = b.toString('latin1', 12, 16);
    if (chunk === 'VP8 ') {
      return { width: b.readUInt16LE(26) & 0x3fff, height: b.readUInt16LE(28) & 0x3fff };
    }
    if (chunk === 'VP8L') {
      const bits = b.readUInt32LE(21);
      return { width: (bits & 0x3fff) + 1, height: ((bits >>> 14) & 0x3fff) + 1 };
    }
    if (chunk === 'VP8X') {
      return { width: b.readUIntLE(24, 3) + 1, height: b.readUIntLE(27, 3) + 1 };
    }
    return null;
  }
  if (b.length >= 4 && b[0] === 0xff && b[1] === 0xd8) {
    let i = 2;
    while (i + 9 < b.length && b[i] === 0xff) {
      const marker = b[i + 1];
      // SOF0..SOF15, minus DHT (C4), JPG (C8) and DAC (CC), carry the size.
      if (marker >= 0xc0 && marker <= 0xcf && ![0xc4, 0xc8, 0xcc].includes(marker)) {
        return { width: b.readUInt16BE(i + 7), height: b.readUInt16BE(i + 5) };
      }
      i += 2 + b.readUInt16BE(i + 2);
    }
  }
  return null;
}

/**
 * The newest *released* block of the CHANGELOG (the `[Unreleased]` heading has
 * no version number, so it never matches). The paragraph between the heading
 * and the first `###` is the maintainer's own summary and is passed through
 * as written; from each section only the bold lead of every bullet is kept,
 * which is the headline a reader skims.
 */
export function latestRelease(changelog: string): PressRelease | null {
  const head = /^## \[(\d+\.\d+\.\d+)\][^\n]*?(\d{4}-\d{2}-\d{2})[^\n]*$/m.exec(changelog);
  if (!head) return null;
  const rest = changelog.slice(head.index + head[0].length);
  const end = rest.search(/^## /m);
  const block = end === -1 ? rest : rest.slice(0, end);

  const parts = block.split(/^### +(.+)$/m);
  const summary = parts[0]
    .split('\n')
    .map((l) => l.trim())
    .filter(Boolean)
    .join(' ');

  const sections: PressRelease['sections'] = [];
  for (let i = 1; i < parts.length; i += 2) {
    const bullets = parts[i + 1].split(/^- /m).slice(1);
    const items = bullets
      .map((bullet) => bullet.replace(/\s+/g, ' ').trim())
      .map((text) => /^\*\*(.+?)\*\*/.exec(text)?.[1] ?? text)
      .filter(Boolean);
    if (items.length) sections.push({ name: parts[i].trim(), items });
  }

  return { version: head[1], date: head[2], summary, sections };
}

export function readPressAsset(webRoot: string, file: string): Buffer {
  try {
    return readFileSync(join(webRoot, 'static', file));
  } catch {
    throw new Error(`press kit: static/${file} is listed in src/lib/press/assets.ts but does not exist`);
  }
}

export function pressBuild(webRoot: string): PressBuild {
  const assets = PRESS_ASSETS.map((asset) => {
    const bytes = readPressAsset(webRoot, asset.file);
    const size = measure(bytes);
    return { ...asset, bytes: bytes.length, width: size?.width ?? null, height: size?.height ?? null };
  });
  let release: PressRelease | null = null;
  try {
    release = latestRelease(readFileSync(join(webRoot, '..', 'CHANGELOG.md'), 'utf8'));
  } catch {
    // The web built on its own, without the repo around it: no release block.
  }
  return { assets, release };
}
