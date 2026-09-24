/// <reference types="vite/client" />

// Custom env vars we surface to the client bundle via vite's `define`.
// Keeping the types here means components don't need ad-hoc `as any` casts
// when reading `import.meta.env.VITE_HOARD_VERSION`.
interface ImportMetaEnv {
  readonly VITE_HOARD_VERSION: string;
}

// `$pro` is a build-time alias for `src/lib/pro` (see `vite.config.ts`).
declare module "$pro" {
  import type { Component } from "svelte";
  export const Wrapped: Component;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
