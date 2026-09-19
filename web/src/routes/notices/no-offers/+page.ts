// Functional route, like /account and /link: no locale prefix, no SSR, and out
// of the sitemap. The whole page is one button plus a query parameter, and the
// text is deliberately English only, because so is the email it comes from.
export const prerender = true;
export const ssr = false;
