import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

export default {
  preprocess: vitePreprocess(),
  // Note: runes mode is opt-in per component (using $state, $derived, etc.).
  // We do NOT force `compilerOptions.runes = true` here because some
  // dependencies (e.g. older releases of lucide-svelte) still use legacy
  // `$$props` syntax that's incompatible with global runes mode.
};
