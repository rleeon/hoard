import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

export default {
  preprocess: vitePreprocess(),
  compilerOptions: {
    // Runes across the whole app. It was not possible before: `lucide-svelte` 0.4x
    // still used `$$props`, legacy syntax that runes mode forbids, and since
    // `compilerOptions` reaches the dependencies' `.svelte` files too, turning it on
    // here broke the icons' compilation. That package was deprecated; its
    // replacement (`@lucide/svelte` 1.x) and `svelte-spa-router` 5 are Svelte 5
    // natives, so nothing legacy is left in the tree.
    //
    // What it buys: the compiler stops emitting the compatibility bridge (mutable
    // props, `$$restProps`, invalidation by assignment) and compiles with direct
    // signals. It also turns any relapse into `export let` or `$:` into a compile
    // error, which is the rule CLAUDE.md had been asking for by hand.
    runes: true,
  },
};
