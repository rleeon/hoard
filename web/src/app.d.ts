declare global {
  // Injected by Vite `define` from the workspace Cargo.toml version.
  const __HOARD_VERSION__: string;
  // Injected by Vite `define` from the latest dated CHANGELOG entry.
  const __HOARD_RELEASE_DATE__: string;
  // Injected by Vite `define`: the press kit files, measured, and the newest
  // released CHANGELOG block (src/lib/press/build.ts).
  const __HOARD_PRESS__: import('./lib/press/assets').PressBuild;

  namespace App {
    // interface Error {}
    // interface Locals {}
    // interface PageData {}
    // interface PageState {}
    // interface Platform {}
  }
}

export {};
