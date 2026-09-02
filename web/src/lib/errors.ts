import type { _ } from 'svelte-i18n';
import { ApiError } from './api';

/**
 * svelte-i18n's `$_`. Its `MessageFormatter` type isn't exported, so we pull
 * it back off the store rather than hand-rolling a near-miss signature that
 * `$_` then fails to satisfy.
 */
type Translate = Parameters<Parameters<(typeof _)['subscribe']>[0]>[0];

/**
 * Turn a caught error into a sentence that names what actually went wrong.
 *
 * The rule: never replace a known reason with a generic one. Pages keep their
 * friendly headline ("Couldn't start the checkout") and render this underneath
 * as the concrete detail, the way the account page already does, so the user
 * can tell "your session expired" from "payments aren't configured" from "your
 * wifi is down", and can quote something useful in a bug report.
 *
 * Server messages come back in English (they're `&'static str`s in
 * `hoard-server/src/cloud/errors.rs`). That's deliberate: a verbatim reason
 * beats a translated non-answer. Only the three cases with no server sentence
 * to show, unreachable, expired session, bare HTTP status, get localized
 * copy of our own.
 *
 * `t` is svelte-i18n's `$_`, passed in so this stays a plain testable function
 * instead of reaching into a store.
 */
export function describeError(e: unknown, t: Translate): string {
  if (e instanceof ApiError) {
    // Never reached the server: DNS, CORS, offline, Fly cold-start timeout.
    if (e.status === 0) return t('err.unreachable');
    // The JWT died under us, most often because another client in the same
    // refresh-token family rotated it. Not the user's fault, and not a bug to
    // apologise for; it's a re-login.
    if (e.status === 401) return t('err.session_expired');
    if (e.detail) return `${e.status} · ${e.detail}`;
    return t('err.http', { values: { status: e.status } });
  }
  const msg = (e as Error)?.message;
  return msg && msg.trim() ? msg : t('common.error');
}
