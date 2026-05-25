import { writable } from 'svelte/store';
import { auth } from '../auth';
import type { AccountSession } from '../types';

export const session = writable<AccountSession | null | undefined>(undefined);
//                                                       ^ undefined = "still loading"

let started = false;

export function startSessionTracking() {
  if (started) return;
  started = true;
  auth.getSession().then((s) => session.set(s));
  auth.onAuthChange((s) => session.set(s));
}
