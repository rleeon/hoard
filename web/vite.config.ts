import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

// Single source of truth for the public version: the workspace Cargo.toml.
// The gh-pages deploy checks out the whole repo, so `../Cargo.toml` is
// available at build time. Falls back to "dev" when the file can't be read
// (e.g. the web dir built in isolation).
function workspaceVersion(): string {
  try {
    const cargo = readFileSync(
      fileURLToPath(new URL('../Cargo.toml', import.meta.url)),
      'utf8'
    );
    const m = cargo.match(/^\s*version\s*=\s*"([^"]+)"/m);
    return m ? m[1] : 'dev';
  } catch {
    return 'dev';
  }
}

// Date of the latest *released* version, read from the top dated CHANGELOG
// entry (skips the `## [Unreleased]` heading). Keeps the download page's
// "published <date>" line single-sourced alongside the version.
function releaseDate(): string {
  try {
    const log = readFileSync(
      fileURLToPath(new URL('../CHANGELOG.md', import.meta.url)),
      'utf8'
    );
    const m = log.match(/^##\s*\[\d+\.\d+\.\d+\][^\n]*?(\d{4}-\d{2}-\d{2})/m);
    return m ? m[1] : '';
  } catch {
    return '';
  }
}

export default defineConfig({
  plugins: [tailwindcss(), sveltekit()],
  define: {
    __HOARD_VERSION__: JSON.stringify(workspaceVersion()),
    __HOARD_RELEASE_DATE__: JSON.stringify(releaseDate())
  },
  server: {
    port: 5173,
    strictPort: false
  }
});
