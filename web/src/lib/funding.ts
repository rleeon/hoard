/**
 * The money figures shown on /pricing. They live here, not in the locale
 * files, so the monthly update is one edit instead of eight. FUNDING.md at
 * the repo root has the month-by-month detail.
 */
export const FUNDING = {
  /** What one user costs a month on average, paid by the maintainer. */
  perUserEur: 0.0991,
  /** Free users one Pro helps pay for. A range on purpose: a Pro user can
   *  use up everything they pay, so the real number is still open. */
  proCoversFree: [5, 10] as const,
  url: 'https://github.com/rleeon/hoard/blob/main/FUNDING.md'
};
