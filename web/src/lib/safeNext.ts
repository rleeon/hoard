/**
 * Sanitise a user-supplied `?next=` redirect target.
 *
 * `next` rides in the URL and ends up in `goto(...)` / the OAuth `redirectTo`,
 * so an attacker could craft `?next=//evil.com` or `?next=https://evil.com` to
 * turn the login flow into an open redirect (and, with the desktop handoff,
 * phish tokens). Only allow same-origin **relative** paths: a single leading
 * slash, no scheme, no protocol-relative `//host`, no backslashes.
 */
export function safeNext(raw: string | null | undefined, fallback = '/account'): string {
  if (!raw) return fallback;
  // Must be an absolute path on this origin. Reject protocol-relative (`//`,
  // `/\`) and anything not rooted at a single `/`.
  if (!raw.startsWith('/') || raw.startsWith('//') || raw.startsWith('/\\')) {
    return fallback;
  }
  // Defence in depth: reject whitespace / control chars that a parser might
  // strip to reveal a `//host` or scheme behind them.
  if (/[\s\u0000-\u001f]/.test(raw)) {
    return fallback;
  }
  return raw;
}
