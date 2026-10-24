// What the in-app pricing page says about money. A copy of
// `web/src/lib/funding.ts`: change both together, once a month.
export const FUNDING = {
  /** What one user costs a month on average, paid by the maintainer. */
  perUserEur: 0.0991,
  /** Free users one Pro helps pay for. A range on purpose: a Pro user can
   *  use up everything they pay, so the real number is still open. */
  proCoversFree: [5, 10] as const,
  url: "https://github.com/rleeon/hoard/blob/main/FUNDING.md",
};
