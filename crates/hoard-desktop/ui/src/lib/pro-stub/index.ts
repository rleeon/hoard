/**
 * Public stub for the Pro UI surface (`$pro`).
 *
 * The real implementations live in the **private** `hoard-pro` repo and are
 * copied into `../pro/` for official/Pro builds, where the `$pro` Vite alias
 * resolves to that directory instead of this one. This stub keeps the community
 * build compiling and Pro-free: `PRO === false` makes every gated route fall
 * back to `ProGate`, and the components render nothing.
 *
 * No Pro logic ever lives here — this file is intentionally inert.
 */
export { default as Screen } from "./Empty.svelte";
export { default as Wrapped } from "./Empty.svelte";

/** `true` only when the private Pro UI has been linked into this build. */
export const PRO: boolean = false;
