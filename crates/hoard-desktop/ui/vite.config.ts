import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";
import { fileURLToPath } from "node:url";
import pkg from "./package.json" with { type: "json" };

const here = (p: string) => fileURLToPath(new URL(p, import.meta.url));

// `$pro` resolves to the Pro UI (Hoard-Screen / Hoard-Wrapped). The code now
// lives in-repo (AGPL, open source); the real lock stays server-side, the
// overlay only unlocks against a Cloud entitlement, so shipping the source
// doesn't hand out the feature. `PRO` is therefore always true.
const proDir = here("./src/lib/pro");

// Tauri expects a fixed port, fail if that port is unavailable.
export default defineConfig(async () => ({
  plugins: [svelte(), tailwindcss()],
  resolve: {
    alias: { $pro: proDir },
  },
  // Surface the package version to the client bundle so the sidebar can
  // print "v1.3.0" without us having to keep two copies in sync.
  // `VITE_HOARD_PRO` is always true now that the Pro UI ships in-repo; the
  // real gate is the server entitlement, not the presence of the code.
  define: {
    "import.meta.env.VITE_HOARD_VERSION": JSON.stringify(pkg.version),
    "import.meta.env.VITE_HOARD_PRO": JSON.stringify(true),
    // DEV-only unlock for the owner's personal test builds. The public/CI build
    // never sets `HOARD_PRO_UNLOCK`, so the shipped binary stays server-gated.
    "import.meta.env.VITE_HOARD_PRO_UNLOCK": JSON.stringify(
      process.env.HOARD_PRO_UNLOCK === "1",
    ),
  },
  // Tauri ignores everything not in src, make all paths relative so the
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
