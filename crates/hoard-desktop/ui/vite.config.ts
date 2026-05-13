import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";
import pkg from "./package.json" with { type: "json" };

// Tauri expects a fixed port, fail if that port is unavailable.
export default defineConfig(async () => ({
  plugins: [svelte(), tailwindcss()],
  // Surface the package version to the client bundle so the sidebar can
  // print "v1.3.0" without us having to keep two copies in sync.
  define: {
    "import.meta.env.VITE_HOARD_VERSION": JSON.stringify(pkg.version),
  },
  // Tauri ignores everything not in src — make all paths relative so the
  // built bundle works when loaded via the file:// scheme.
  base: "./",
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: false,
    // Tauri expects HMR to point back at the dev server.
    hmr: {
      protocol: "ws",
      host: "localhost",
      port: 1421,
    },
    watch: {
      // Don't watch the Rust src or generated files.
      ignored: ["**/src-tauri/**", "**/target/**"],
    },
  },
}));
