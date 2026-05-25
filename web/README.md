# hoard-web

Landing + customer dashboard for **Hoard Cloud** (the SaaS side of the
project). The same `hoard-server` binary runs both Hoard Cloud and any
self-hosted install — this app is the public face of the managed
version.

Stack: **SvelteKit** + **Tailwind v4** + **Supabase Auth** (Google OAuth
+ email magic link) + **Lemon Squeezy** (hosted checkout + customer
portal). Same Svelte 5 runes / Tailwind palette / `svelte-i18n` setup
as the desktop app, so components and locales can be shared if needed.

## Run locally

```sh
cp .env.example .env
$EDITOR .env                  # set PUBLIC_SUPABASE_ANON_KEY + checkout URLs
pnpm install
pnpm dev
```

Open <http://localhost:5173>. The desktop preview hook will surface
each saved file in the panel automatically.

## Environment

Only `PUBLIC_*` variables — everything in this app is client-bundled
and **must never** receive service-role secrets. Backend secrets
(database URL, R2 keys, Lemon Squeezy API key + webhook secret) live
in the Rust server's `.env` only.

| Var | Purpose |
| --- | --- |
| `PUBLIC_SUPABASE_URL` | Supabase project URL |
| `PUBLIC_SUPABASE_ANON_KEY` | Public anon key (safe to expose) |
| `PUBLIC_API_URL` | `hoard-server-cloud` base URL (`api.hoard.services`) |
| `PUBLIC_LS_CHECKOUT_*` | Lemon Squeezy variant checkout links |
| `PUBLIC_LS_CUSTOMER_PORTAL` | Lemon Squeezy hosted billing portal |

## Provider swap

Auth, billing, and the backend API are behind interfaces so swapping
providers is a one-file change:

- `src/lib/auth/index.ts` — `AuthProvider` interface; current impl in
  `auth/supabase.ts`. Swap to Clerk/Auth0 = new file + change the
  export at the bottom of `index.ts`.
- `src/lib/billing/index.ts` — `BillingProvider` interface; current
  impl in `billing/lemonsqueezy.ts`. Swap to Stripe = same pattern.
- `src/lib/api/index.ts` — thin wrapper over `hoard-server`. If the
  backend ever moves off Rust/Axum, only this file changes.

No component imports the impl modules directly — always
`import { auth } from '$lib/auth'` / `'$lib/billing'` / `'$lib/api'`.

## Routes

| Path | Auth | What |
| --- | --- | --- |
| `/` | public | Landing (hero, features, CTA) |
| `/pricing` | public | Plans + monthly/yearly toggle + checkout |
| `/help` | public | FAQ + contact, live status pill |
| `/login` | public | Google OAuth + email magic link |
| `/auth/callback` | public | Supabase redirect target |
| `/account` | required | Overview: plan, usage, devices, danger zone |

`/account` requires a Supabase session; the layout guards with a
redirect to `/login?next=/account`.

## i18n

`en` + `es` shipped. Add more languages by dropping a JSON in
`src/lib/i18n/locales/` and registering it in `i18n/index.ts`. Keys
mirror the desktop app's flat `section.key` style.

## Deploy

`adapter-auto` is wired. Push to Vercel or Cloudflare Pages and it
picks the right adapter. Domain target: `hoard.services` (production)
+ `api.hoard.services` (backend, separate deploy from this repo —
`crates/hoard-server` on Fly.io).
