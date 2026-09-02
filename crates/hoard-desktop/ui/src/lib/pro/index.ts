/**
 * Pro UI surface (`$pro`), Hoard-Screen and Hoard-Wrapped.
 *
 * Open source, AGPL, shipped in-repo. `PRO` stays `true`; the real lock is
 * server-side (the overlay only unlocks against a Cloud entitlement), so
 * publishing this code doesn't hand out the feature, a self-hosted or patched
 * build can render the UI but the server never issues the entitlement/assets.
 */
export { default as Screen } from "./Screen.svelte";
export { default as Wrapped } from "./Wrapped.svelte";

export const PRO: boolean = true;
