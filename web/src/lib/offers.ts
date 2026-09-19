/**
 * The "no offers for Pro in my emails" choice, carried across sign-in.
 *
 * It is ticked on /login and has to survive the round trip to the identity
 * provider, which leaves this origin, so it rides in localStorage until
 * /auth/callback, which has a session to record it against, takes it. A magic
 * link opened in another browser arrives without it; that reader still has the
 * refusal link at the foot of every email, which is the one the law requires
 * in any case.
 */
const KEY = 'hoard:no_offers';

export function rememberNoOffers(on: boolean): void {
  try {
    if (on) localStorage.setItem(KEY, '1');
    else localStorage.removeItem(KEY);
  } catch {
    // Private window or blocked storage: see above, the email link covers it.
  }
}

/** Read the choice and forget it, so it is recorded once and never leaks into
 *  the next account signed in from the same browser. */
export function takeNoOffers(): boolean {
  try {
    const on = localStorage.getItem(KEY) === '1';
    localStorage.removeItem(KEY);
    return on;
  } catch {
    return false;
  }
}
