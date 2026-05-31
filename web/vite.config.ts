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

export default defineConfig({
  plugins: [tailwindcss(), sveltekit()],
  define: {
    __HOARD_VERSION__: JSON.stringify(workspaceVersion())
  },
  server: {
    port: 5173,
    strictPort: false
  }
});
