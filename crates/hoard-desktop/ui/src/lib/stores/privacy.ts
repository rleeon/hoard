/**
 * Whether the account's email is shown in clear.
 *
 * Masked by default and on every start: the address is on screen in the account
 * page, the settings card and the sidebar, which is exactly what ends up in a
 * screenshot or a stream. One flag for the whole app, so revealing it in one
 * place does not leave it masked two centimetres away.
 */
import { writable } from "svelte/store";

export const emailRevealed = writable(false);
