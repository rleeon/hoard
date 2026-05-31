declare global {
  // Injected by Vite `define` from the workspace Cargo.toml version.
  const __HOARD_VERSION__: string;
  // Injected by Vite `define` from the latest dated CHANGELOG entry.
  const __HOARD_RELEASE_DATE__: string;

  namespace App {
    // interface Error {}
    // interface Locals {}
    // interface PageData {}
    // interface PageState {}
    // interface Platform {}
  }
}

export {};
