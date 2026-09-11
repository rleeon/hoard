/**
 * Five quick taps on the version unlock the diagnostics screen for this session.
 *
 * Module state rather than a component's: the version shows in the sidebar, or
 * on Windows in the title bar, and those are two different Svelte roots.
 */
let taps = 0;
let last = 0;

export function tapVersion(unlockedMessage: string): void {
  const now = Date.now();
  // A pause over 1.5s starts the streak again: the gesture has to be
  // deliberate, a stray double click on idle UI must not drift towards unlocking.
  taps = now - last > 1500 ? 1 : taps + 1;
  last = now;
  if (taps >= 5) {
    sessionStorage.setItem("hoard-diagnostics-unlocked", "1");
    taps = 0;
    // Lazy import keeps the toast store out of the boot path.
    void import("./toasts").then(({ toastSuccess }) => toastSuccess(unlockedMessage));
  }
}
