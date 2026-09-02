/**
 * Version stamps for the binding legal documents.
 *
 * These are dates, not semver: what matters when a dispute lands is "which
 * text was in force the day this person clicked accept", and a date answers
 * that without a changelog. Bump the stamp **only** when the substance
 * changes, a typo fix that bumps it would re-prompt every user for nothing.
 *
 * `TERMS_VERSION` is what the clients send to `POST /v1/me/terms`, and the
 * server stores it verbatim. Keep it in sync with `TERMS_VERSION` in
 * `crates/hoard-core/src/wire.rs`, which is where the desktop app and the CLI
 * read theirs from.
 */
export const TERMS_VERSION = '2026-08-11';
export const PRIVACY_VERSION = '2026-08-11';

/** Human-readable date of the version stamp, per locale. */
export const LAST_UPDATED: Record<string, string> = {
  es: '11 de agosto de 2026',
  en: 'August 11, 2026'
};

/**
 * Identification of the provider, as required by art. 10 of Spain's LSSI-CE
 * (Ley 34/2002) for anyone running an information-society service from Spain:
 * name, tax ID, registered address and a direct means of contact.
 *
 * `TAX_ID` and `ADDRESS` are the two fields that cannot be derived from the
 * repo, fill them in before deploying the legal notice. The page renders a
 * visible placeholder while they are empty, on purpose: a legal notice that
 * quietly omits the address is worse than one that admits it is unfinished.
 */
export const OPERATOR = {
  name: 'Raimundo León Oliva',
  taxId: '', // TODO: NIF
  address: '', // TODO: domicilio completo (calle, CP, municipio, provincia)
  email: 'support@hoard.services',
  site: 'https://hoard.services'
};

export const operatorComplete = () => Boolean(OPERATOR.taxId && OPERATOR.address);
