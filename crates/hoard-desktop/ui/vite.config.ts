import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";

// Tauri expects a fixed port, fail if that port is unavailable.
export default defineConfig(async () => ({
  plugins: [svelte(), tailwindcss()],
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
