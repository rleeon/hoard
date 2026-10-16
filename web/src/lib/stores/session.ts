import { writable } from 'svelte/store';
import type { AccountSession } from '../types';
import { isChunkError, reloadAfterChunkError } from '../chunkReload';

export const session = writable<AccountSession | null | undefined>(undefined);
//                                                       ^ undefined = "still loading"

let started = false;

// Lazy-import the auth module so Supabase + auth code stay out of the
// initial chunk graph for routes that never touch authentication
// (home, pricing, help, legal). Only pulled when a page actually needs
// to know whether the user is signed in.
export function startSessionTracking() {
  if (started) return;
  started = true;
  import('../auth')
    .then(({ auth }) => {
      auth.getSession().then((s) => session.set(s));
      auth.onAuthChange((s) => session.set(s));
    })
    .catch((e) => {
      // Without this the store stays `undefined` for good and the nav keeps
      // its loading placeholder, a faint empty box where the account chip goes.
      if (isChunkError(e) && reloadAfterChunkError()) return;
      session.set(null);
    });
}
