import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";
import { fileURLToPath } from "node:url";
import { existsSync } from "node:fs";
import pkg from "./package.json" with { type: "json" };

const here = (p: string) => fileURLToPath(new URL(p, import.meta.url));

// `$pro` resolves to the private Pro UI when it's been linked into this build
// (a Pro build copies `hoard-pro/ui/pro/` → `src/lib/pro/`), otherwise to the
// inert public stub. This is the single switch that turns the Pro features on
// for official builds without the Pro code ever living in the public repo.
const proLinked = existsSync(here("./src/lib/pro/index.ts"));
const proDir = proLinked ? here("./src/lib/pro") : here("./src/lib/pro-stub");

// Tauri expects a fixed port, fail if that port is unavailable.
export default defineConfig(async () => ({
  plugins: [svelte(), tailwindcss()],
  resolve: {
    alias: { $pro: proDir },
  },
  // Surface the package version to the client bundle so the sidebar can
  // print "v1.3.0" without us having to keep two copies in sync.
  // `VITE_HOARD_PRO` lets components branch on whether the Pro layer is present.
  define: {
    "import.meta.env.VITE_HOARD_VERSION": JSON.stringify(pkg.version),
    "import.meta.env.VITE_HOARD_PRO": JSON.stringify(proLinked),
    // DEV-only unlock for the owner's personal test builds. The public/CI build
    // never sets `HOARD_PRO_UNLOCK`, so the shipped binary stays server-gated.
    "import.meta.env.VITE_HOARD_PRO_UNLOCK": JSON.stringify(
      process.env.HOARD_PRO_UNLOCK === "1",
    ),
  },
  // Tauri ignores everything not in src — make all paths relative so the
  // built bundle works when loaded via the file:// scheme.
  base: "./",
  clearScreen: false,
  build: {
    rollupOptions: {
      // Second entry point: the Pro overlay window loads `overlay.html`, kept
      // isolated from the main app so it never spins up the auth/cloud/agent
      // pollers. Harmless empty page in the community build.
      input: {
        main: here("./index.html"),
        overlay: here("./overlay.html"),
      },
    },
  },
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
