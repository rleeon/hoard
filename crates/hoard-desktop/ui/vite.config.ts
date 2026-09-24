import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";
import { fileURLToPath } from "node:url";
import pkg from "./package.json" with { type: "json" };

const here = (p: string) => fileURLToPath(new URL(p, import.meta.url));

// `$pro` resolves to the Hoard-Wrapped UI.
const proDir = here("./src/lib/pro");

// Tauri expects a fixed port, fail if that port is unavailable.
export default defineConfig(async () => ({
  plugins: [svelte(), tailwindcss()],
  resolve: {
    alias: { $pro: proDir },
  },
  // Surface the package version to the client bundle so the sidebar can
  // print "v1.3.0" without us having to keep two copies in sync.
  define: {
    "import.meta.env.VITE_HOARD_VERSION": JSON.stringify(pkg.version),
  },
  // Tauri ignores everything not in src, make all paths relative so the
  // built bundle works when loaded via the file:// scheme.
  base: "./",
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    // `HOARD_DEV_HOST` is the address the *app* will reach this dev server at,
    // for the split setup: Vite here, the Tauri window on another machine over
    // the LAN (or Tailscale), so a UI change is on screen without rebuilding
    // anything or copying the tree. Unset, it binds to localhost as always.
    host: process.env.HOARD_DEV_HOST ? true : false,
    hmr: {
      protocol: "ws",
      // Tauri expects HMR to point back at the dev server, which is this
      // machine's address as seen from wherever the window runs.
      host: process.env.HOARD_DEV_HOST ?? "localhost",
      port: 1421,
    },
    watch: {
      // Don't watch the Rust src or generated files.
      ignored: ["**/src-tauri/**", "**/target/**"],
    },
  },
}));
