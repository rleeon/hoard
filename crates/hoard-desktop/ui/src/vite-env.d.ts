/// <reference types="vite/client" />

// Custom env vars we surface to the client bundle via vite's `define`.
// Keeping the types here means components don't need ad-hoc `as any` casts
// when reading `import.meta.env.VITE_HOARD_VERSION`.
interface ImportMetaEnv {
  readonly VITE_HOARD_VERSION: string;
  /** `true` only in official Pro builds where the private `$pro` UI is linked. */
  readonly VITE_HOARD_PRO: boolean;
  /** DEV-only: forces the Pro features open without a server entitlement.
   *  Set via `HOARD_PRO_UNLOCK=1` for personal test builds; never in CI. */
  readonly VITE_HOARD_PRO_UNLOCK: boolean;
}

// `$pro` is a build-time alias (see `vite.config.ts`): the private Pro UI in a
// Pro build, the inert public stub otherwise. Both expose the same surface.
declare module "$pro" {
  import type { Component } from "svelte";
  export const Screen: Component;
  export const Wrapped: Component;
  export const Overlay: Component;
  export const PRO: boolean;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
