/// <reference types="vite/client" />

// Custom env vars we surface to the client bundle via vite's `define`.
// Keeping the types here means components don't need ad-hoc `as any` casts
// when reading `import.meta.env.VITE_HOARD_VERSION`.
interface ImportMetaEnv {
  readonly VITE_HOARD_VERSION: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
