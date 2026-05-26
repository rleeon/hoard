import { writable } from 'svelte/store';
import type { AccountSession } from '../types';

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
  void import('../auth').then(({ auth }) => {
    auth.getSession().then((s) => session.set(s));
    auth.onAuthChange((s) => session.set(s));
  });
}
