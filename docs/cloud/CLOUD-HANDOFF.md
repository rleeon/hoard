# Hoard Cloud — Handoff para Opus

> Documento self-contained. Si lo estás leyendo, eres Opus arrancando
> el ciclo cloud de Hoard. Lee linealmente; no hace falta más contexto
> que el codebase y `CLAUDE.md`. Cuando termines de leer, sabrás:
>
> 1. Por qué existe este ciclo y qué cubre.
> 2. Stack decidido (con razones).
> 3. Modelo de precios y pagos.
> 4. Arquitectura: schema, auth, storage, deploy.
> 5. UI nueva: login, cuenta, upgrade.
> 6. Fases de implementación (P-CLD-*).
> 7. MCPs/skills/connectors que necesitas pedir al user.
> 8. Acciones de cuenta que el user tiene que hacer (no tú).
>
> Cuando hayas leído esto, abre un sub-plan formal en
> `docs/plans/1.6-cloud.md` y un ADR `docs/decisions/0015-hoard-cloud-saas.md`
> antes de tocar código.

---

## 1. Contexto y objetivo

Hoard hoy es **self-hosted**: el user (o un amigo técnico) corre
`hoard-server` en un VPS, abre puerto, pone reverse-proxy, gestiona
tokens y backups del Postgres/SQLite. Eso filtra brutalmente a
quien puede usarlo. El objetivo del ciclo cloud es:

> **Convertir Hoard en un SaaS opcional ("Hoard Cloud") que cualquier
> gamer pueda usar con login social y plan de pago, manteniendo el
> self-hosted como ciudadano de primera clase.**

Restricciones duras:

- **Self-hosted no se rompe.** El binario `hoard-server` que ya está
  en el repo sigue funcionando exactamente igual sin cuenta cloud.
  Cloud es **opt-in** desde la app (onboarding con dos botones).
- **Cero vendor lock-in para el user**: si Hoard Cloud cierra mañana,
  el user puede exportar sus saves (botón en Settings → "Descargar
  todo") y montar self-hosted. Esto cuenta como feature obligatoria
  en el plan, no opcional.
- **Sin tocar la lógica de sync.** El cliente (`hoard-agent`) ya
  habla con `hoard-server` por HTTP — Hoard Cloud es un
  `hoard-server` desplegado por nosotros con auth swappable. No hay
  un protocolo nuevo.

---

## 2. Stack — decidido

| Componente            | Elección                       | Por qué                                                                                     |
| --------------------- | ------------------------------ | ------------------------------------------------------------------------------------------- |
| **Auth + identidad**  | **Supabase Auth** (OAuth + email) | El user pidió Google/algo similar. Supabase Auth tiene Google, GitHub, Discord, Apple, email magic link out-of-the-box. JWT estándar verificable desde Rust con la pubkey JWKS. Free tier generoso para arrancar. |
| **DB metadatos**      | **Postgres (Supabase managed)**  | `hoard-server` ya usa `sqlx` — migrar de SQLite a Postgres es feature flag (`db_backend = "postgres"`). Supabase RLS por `user_id` es defensa en profundidad. |
| **Storage snapshots** | **Cloudflare R2**              | **Sin egress fees**. Crítico: un user con 50GB de saves va a re-descargar varias veces; con S3/Supabase Storage el coste de egress mata el modelo. R2 es S3-compatible (sigues usando `aws-sdk-s3` o `s3` crate). |
| **App backend**       | **Fly.io** (`hoard-server-cloud`) | Rust + WebSockets si los necesitamos + multi-region. Coste arrancable (~5€/mes la app mínima). Coolify/Hetzner para escalar después.|
| **Pagos**             | **Lemon Squeezy**              | Merchant of Record: gestiona IVA UE por nosotros (el user vive en España, eso solo ya cierra el caso). Checkout hospedado, webhooks limpios. Comisión ~5% + Stripe-equivalent. Alternativa cerrada: Paddle. **Stripe** quedó descartado porque exige gestionar IVA por país nosotros (complicado para freelance solo). |
| **CDN + DNS**         | **Cloudflare**                 | DNS, R2 en la misma cuenta, opcional Workers para edge caching. |
| **Email transaccional** | **Resend**                   | Para login magic link y notificaciones de pago. Free tier 3K mails/mes. |
| **Errores + métricas** | **Sentry** (cliente y server)  | Free tier 5K errores/mes. Activación opt-in respetando `prefs.anonymous_telemetry`. |
| **Landing + checkout** | **Astro static + Vercel/Netlify free** | Una landing sencilla con planes + redirect a Lemon Squeezy checkout. No hace falta SSR. Repo separado `hoard-web` o subdirectorio. |

**No** vamos a usar:

- Firebase (vendor lock-in Google, NoSQL no encaja con saves).
- AWS Cognito (complicado, latente).
- Auth0 (caro a partir del free tier).
- Polar.sh (todavía joven, MoR sólo en algunos países UE).
- Backblaze B2 (R2 sin egress es mejor para nuestro caso).

---

## 3. Modelo de precios

Decisión: **tres planes**, billing mensual y anual (anual = 20% off).
Precios en EUR, IVA incluido (Lemon Squeezy lo gestiona).

| Plan         | Mensual  | Anual    | Storage   | Dispositivos | Saves tracked | Retención versiones |
| ------------ | -------- | -------- | --------- | ------------ | ------------- | ------------------- |
| **Free**     | 0 €      | 0 €      | 500 MB    | 1            | 3             | 7 días              |
| **Pro**      | 3,99 €   | 39 €     | 50 GB     | 5            | ilimitados    | 90 días             |
| **Pro+**     | 9,99 €   | 99 €     | 200 GB    | ilimitados   | ilimitados    | 365 días            |

Filosofía:

- **Free es real, no demo.** Un user con 1-2 juegos cabe. Eso es lo
  que convierte: la gente se queda, recomienda, paga cuando crece.
- **Pro es el sweet spot.** 50 GB cubre saves de cualquier perfil
  realista — saves de juegos rara vez superan 500 MB cada uno.
- **Pro+ no es para gamers normales**, es para creadores de contenido
  / streamers con muchas máquinas. Existe sobre todo para hacer Pro
  parecer la opción obvia.
- **Anual con 20% off** porque la gente que paga anual cancela menos
  y el cashflow upfront paga R2/Supabase.
- **Sin trial gratis de Pro.** Free + Pro es la frontera; no
  queremos `payment failed → downgrade silencioso` flows.

Add-ons (futuro, ciclo 1.7+):

- **Family plan**: Pro x 4 cuentas, 24,99 €/mes.
- **Lifetime**: 249 € one-shot Pro+ vitalicio. Útil para promo
  ProductHunt/HN launch.

---

## 4. Arquitectura

```
              ┌─────────────────────────────────────┐
              │ Hoard Desktop (Tauri + Svelte)      │
              │ - login flow OAuth Supabase         │
              │ - JWT en keyring                    │
              │ - dos modos: cloud / self-hosted    │
              └────────────┬────────────────────────┘
                           │ HTTPS + Bearer JWT
                           │ (+ multipart snapshot uploads)
              ┌────────────▼────────────────────────┐
              │ hoard-server-cloud @ Fly.io         │
              │ (Rust + Axum, mismo binario que     │
              │ self-hosted, feature `cloud`)       │
              ├──────────────────────────────────────┤
              │ - verify Supabase JWT (JWKS cache)  │
              │ - quota check per user_id           │
              │ - presigned R2 URLs para upload     │
              │ - webhooks Lemon Squeezy (plan ↑↓)  │
              └────┬───────────────────────────┬────┘
                   │                            │
       ┌───────────▼──────────┐    ┌────────────▼───────────┐
       │ Supabase Postgres    │    │ Cloudflare R2          │
       │ - users, plans       │    │ - snapshots .tar.zst   │
       │ - saves, versions    │    │ - bucket per env       │
       │ - usage_quota        │    │ - lifecycle: 365d max  │
       └──────────────────────┘    └────────────────────────┘

                    ┌───────────────────────────┐
                    │ Lemon Squeezy (webhooks)  │
                    │ checkout, subscriptions   │
                    └───────────────────────────┘
```

### 4.1 Auth flow

1. User abre Hoard desktop por primera vez → onboarding pregunta
   "¿Hoard Cloud o servidor propio?".
2. Click Cloud → abre `https://hoard.cloud/auth/desktop?state=<nonce>`
   en navegador del sistema (no embebido — IETF RFC 8252).
3. Supabase Auth muestra Google/GitHub/Discord/Email. User completa.
4. Supabase redirige a `https://hoard.cloud/auth/desktop/callback?...`
   que renderiza una página "Vuelve a la app".
5. Esa página hace `window.location = "hoard://auth?token=<jwt>&refresh=<rt>"`.
6. Tauri tiene `tauri-plugin-deep-link` registrado para `hoard://`.
   Recibe el JWT, lo guarda en el keyring del SO (`keyring` crate o
   `tauri-plugin-keyring`).
7. App refresca `prefs.cloud_account = { user_id, email, plan }` y
   pinta el estado en la sidebar.

Token lifetime: access token 1h, refresh 30 días. El cliente
auto-renueva 5 min antes de expirar.

### 4.2 Schema Postgres (Supabase)

```sql
-- Supabase ya provee auth.users. Nosotros extendemos:

create table public.profiles (
  user_id uuid primary key references auth.users(id) on delete cascade,
  email text not null,
  display_name text,
  created_at timestamptz default now(),
  plan text not null default 'free' check (plan in ('free','pro','proplus')),
  plan_renews_at timestamptz,
  plan_cancel_at timestamptz,
  ls_customer_id text,       -- Lemon Squeezy customer
  ls_subscription_id text,   -- Lemon Squeezy subscription
  storage_bytes bigint not null default 0,
  devices_count int not null default 0,
  -- enforce RLS: user only sees own row
);

alter table public.profiles enable row level security;
create policy "self read" on public.profiles for select using (auth.uid() = user_id);

create table public.saves (
  save_id text primary key,             -- mantiene el formato hoy en self-hosted
  user_id uuid not null references auth.users(id) on delete cascade,
  game_slug text not null,
  label text,
  local_path_hint text,                 -- ofuscable, opcional
  latest_version_num bigint,
  created_at timestamptz default now(),
  updated_at timestamptz default now(),
  unique(user_id, game_slug, label)
);

create table public.versions (
  id bigserial primary key,
  save_id text not null references public.saves(save_id) on delete cascade,
  version_num bigint not null,
  size_bytes bigint not null,
  r2_key text not null,                 -- path en el bucket R2
  sha256 text not null,                 -- integridad
  created_at timestamptz default now(),
  unique(save_id, version_num)
);

create index versions_by_save on public.versions(save_id, version_num desc);

create table public.devices (
  id uuid default gen_random_uuid() primary key,
  user_id uuid not null references auth.users(id) on delete cascade,
  device_name text not null,
  device_kind text,                     -- 'desktop','steamdeck',...
  last_seen_at timestamptz default now(),
  created_at timestamptz default now()
);

create table public.usage_events (
  id bigserial primary key,
  user_id uuid not null,
  kind text not null,                   -- 'backup','restore','quota_block'
  bytes bigint,
  save_id text,
  at timestamptz default now()
);
create index usage_events_recent on public.usage_events(user_id, at desc);

-- triggers que mantienen profiles.storage_bytes coherente con la
-- suma de versions.size_bytes (insert/delete) — más rápido que
-- recalcular en cada quota check.
```

RLS en `saves`, `versions`, `devices`, `usage_events`: igual patrón
que `profiles` (`user_id = auth.uid()`).

### 4.3 hoard-server cambios

Feature flag de cargo:

```toml
[features]
default = []
cloud = ["sqlx/postgres", "aws-sdk-s3", "jsonwebtoken"]
```

Cambios concretos:

- **Auth middleware**: si feature `cloud`, valida `Authorization:
  Bearer <jwt>` contra JWKS de Supabase. Sin feature, comportamiento
  hoy (token estático).
- **DB layer**: `sqlx::Any` o feature-gated `Pool<Postgres>` /
  `Pool<Sqlite>`. La mayoría del código ya está parametrizado.
- **Storage layer**: nuevo trait `SnapshotStore`:
  ```rust
  trait SnapshotStore: Send + Sync {
      async fn presign_put(&self, key: &str, ttl: Duration) -> Result<Url>;
      async fn presign_get(&self, key: &str, ttl: Duration) -> Result<Url>;
      async fn head(&self, key: &str) -> Result<Option<Meta>>;
      async fn delete(&self, key: &str) -> Result<()>;
  }
  ```
  Impls: `FsStore` (self-hosted, mantiene comportamiento actual con
  `fs::rename`) y `R2Store` (cloud, presigned URLs para que el
  cliente suba directo a R2 sin pasar bytes por el server).
- **Quota middleware**: antes de `PUT /v1/snapshots/...`, consulta
  `profiles.plan` + `profiles.storage_bytes`. Si `requested_size +
  storage_bytes > plan_limit`, 402 con cuerpo `{"upgrade_url": "..."}`.
- **Webhooks Lemon Squeezy**: nuevo endpoint
  `POST /v1/webhooks/lemonsqueezy` que verifica `X-Signature` (HMAC
  con webhook secret) y actualiza `profiles.plan` /
  `plan_renews_at` / `plan_cancel_at`. Eventos a manejar:
  `subscription_created`, `subscription_updated`,
  `subscription_cancelled`, `subscription_expired`,
  `subscription_payment_failed`.

### 4.4 Cliente desktop cambios

Nuevo store `lib/stores/cloud.ts`:

```ts
export type CloudAccount = {
  user_id: string;
  email: string;
  display_name: string | null;
  plan: 'free' | 'pro' | 'proplus';
  plan_renews_at: string | null;
  storage_bytes: number;
  storage_limit_bytes: number;
  devices_count: number;
  devices_limit: number;
};
```

Comandos Tauri nuevos en `commands/cloud.rs`:

- `cloud_start_login() -> Url` — abre navegador a Supabase Auth.
- `cloud_complete_login(token: String) -> Result<CloudAccount>` —
  recibido por deep link.
- `cloud_logout()` — borra del keyring, limpia store.
- `cloud_refresh_account() -> Result<CloudAccount>` — pull manual.
- `cloud_open_billing_portal() -> Url` — abre Lemon Squeezy customer
  portal con SSO.
- `cloud_open_upgrade_url(plan: String) -> Url` — checkout para Pro
  o Pro+ con `?embed=true&prefill[email]=...`.
- `cloud_export_all() -> Result<PathBuf>` — descarga todo en .zip
  (compliance del compromiso de export).

---

## 5. UI nueva

### 5.1 Onboarding (primera ejecución)

`/onboarding` ya existe en App.svelte. Nueva primera pantalla:

```
┌────────────────────────────────────────────┐
│             Hoard                           │
│        Tus saves, siempre contigo           │
│                                              │
│   ┌──────────────────────────────────────┐  │
│   │     Empezar con Hoard Cloud          │  │
│   │  Gratis · Sincroniza en segundos     │  │
│   └──────────────────────────────────────┘  │
│                                              │
│   ┌──────────────────────────────────────┐  │
│   │     Usar mi propio servidor          │  │
│   │  Configurado con hoard-server        │  │
│   └──────────────────────────────────────┘  │
│                                              │
│         ¿Por qué dos opciones? →            │
└────────────────────────────────────────────┘
```

El second botón lleva al flow actual ("connect to your server").
El primero abre el browser y arranca login Supabase.

### 5.2 Sidebar — chip de cuenta

Pie de sidebar (`App.svelte`), debajo del version + update button:

```
┌────────────────────────────┐
│ [avatar]  Rai · Pro        │
│           37 GB / 50 GB    │
└────────────────────────────┘
```

Click → navega a `/account`.

Modo self-hosted: mismo slot dice "Servidor: home.lan:8080" sin
chip de plan.

### 5.3 Página `/account`

```
┌──────────────────────────────────────────────────────────────┐
│  Cuenta                                                       │
├──────────────────────────────────────────────────────────────┤
│                                                                │
│   [avatar grande]                                              │
│                                                                │
│   Rai León                                                     │
│   raileonoliva@gmail.com                                       │
│                                                                │
│   ┌──────────────────────────────────────────────────────┐    │
│   │  Plan actual                                          │    │
│   │  ────────────                                          │    │
│   │  Pro · 3,99 €/mes                                     │    │
│   │  Próxima factura: 19 jun 2026                         │    │
│   │                                                        │    │
│   │  37,2 GB de 50 GB usados   ████████████░░░░░░░ 74%   │    │
│   │  3 de 5 dispositivos                                  │    │
│   │                                                        │    │
│   │  [ Cambiar plan ]   [ Gestionar facturación ]         │    │
│   └──────────────────────────────────────────────────────┘    │
│                                                                │
│   Dispositivos vinculados                                      │
│   ─────────────────────────                                    │
│   • Sobremesa (Linux) — activo ahora                          │
│   • Steam Deck — hace 2 días                                  │
│   • Portátil (Windows) — hace 1 semana    [Desvincular]       │
│                                                                │
│   ┌──────────────────────────────────────────────────────┐    │
│   │ Exportar todo                                          │    │
│   │ Descarga un .zip con todos tus saves en formato       │    │
│   │ original. Sin formato propietario.    [ Descargar ]   │    │
│   └──────────────────────────────────────────────────────┘    │
│                                                                │
│   [ Cerrar sesión ]   [ Eliminar cuenta ]                     │
└──────────────────────────────────────────────────────────────┘
```

### 5.4 Modal "Cambiar plan"

3 columnas (Free / Pro / Pro+), toggle mensual/anual (anual con
"–20%" badge), CTA "Pasar a Pro" abre Lemon Squeezy checkout via
`shell.open(checkoutUrl)`. Tras pago, webhook actualiza plan; el
cliente lo recoge en `cloud_refresh_account` que se llama al
volver a foco la ventana (`onFocus`).

### 5.5 i18n

Todas las strings nuevas en 8 locales. Keys propuestas:

- `onboarding.cloud_cta`, `onboarding.self_hosted_cta`,
  `onboarding.cloud_blurb`, ...
- `account.title`, `account.plan_section_title`,
  `account.usage_label`, `account.devices_label`, ...
- `account.export_title`, `account.export_blurb`, ...
- `plan.free`, `plan.pro`, `plan.proplus`, `plan.month_suffix`,
  `plan.year_suffix`, `plan.annual_savings`, ...
- `auth.signed_out`, `auth.signing_in`, `auth.signed_in_as`,
  `auth.session_expired`, ...

---

## 6. Fases de implementación (P-CLD-*)

Cada fase = un commit + tag + push. Mismo flow que ciclos
anteriores. Plan formal en `docs/plans/1.6-cloud.md` (créalo
después de leer esto).

### P-CLD-0 — Setup cuentas y secretos (humano, no Claude)

Acciones que **el user** ejecuta. Claude solo documenta el
checklist y revisa que los secretos están bien guardados.

- [ ] Crear cuenta Supabase, proyecto `hoard-cloud-prod` (region
      eu-central). Anotar `SUPABASE_URL`, `SUPABASE_ANON_KEY`,
      `SUPABASE_SERVICE_ROLE_KEY`, JWKS URL.
- [ ] Crear cuenta Cloudflare, dominio `hoard.cloud` (o el que
      uses), bucket R2 `hoard-snapshots-prod` y `-staging`.
      Generar API tokens con permisos R2:Read + R2:Write para el
      bucket. Anotar `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY`,
      `R2_ENDPOINT`.
- [ ] Crear cuenta Lemon Squeezy. Crear store. Crear productos Free
      (no necesita item, solo etiqueta), Pro mensual, Pro anual,
      Pro+ mensual, Pro+ anual. Anotar product variant IDs.
      Generar webhook secret + API key.
- [ ] Crear cuenta Fly.io. `fly apps create hoard-server-cloud`.
      Apuntar `FLY_API_TOKEN`.
- [ ] Crear cuenta Resend. API key.
- [ ] Crear cuenta Sentry. Project Rust + project JS. DSNs.
- [ ] **Todos los secretos** en un password manager + GitHub
      Actions secrets para deploy.

### P-CLD-1 — ADR + plan + branding

- Crear ADR `0015-hoard-cloud-saas.md` con resumen de este handoff.
- Crear `docs/plans/1.6-cloud.md` con la tabla de prompts.
- Reservar dominio si no está.
- Renombrar planes histó­ricos: 1.6.0 era CAS storage → ahora es
  Hoard Cloud. CAS pasa a 1.7.0. Actualizar `CLAUDE.md` resumen.

### P-CLD-2 — Migración SQLite → Postgres (feature-gated)

- `hoard-server` añade feature `cloud`. Con feature off, sigue
  SQLite. Con feature on, espera `DATABASE_URL` Postgres.
- Migrations duplicadas: `migrations/sqlite/*.sql` y
  `migrations/postgres/*.sql`. La mayoría son iguales (`AUTOINCREMENT`
  → `bigserial`, `TEXT` igual, `BLOB` → `bytea`).
- CI corre tests contra ambos backends (matrix).

### P-CLD-3 — Trait `SnapshotStore` + impl R2

- Extraer la lógica actual de `fs::rename`-based storage a `FsStore`.
- Implementar `R2Store` con `aws-sdk-s3` (R2 es S3 compatible).
- Cliente sube directamente a R2 con presigned PUT URL: el server
  emite la URL después de validar quota; cliente sube; al
  completar, hace `POST /v1/snapshots/.../commit` con el sha256 +
  size; el server `head()` el objeto y confirma.
- Esto evita que los snapshots pasen por la RAM/banda del server.

### P-CLD-4 — Auth Supabase + JWT middleware

- Cargo dep `jsonwebtoken`. Cargar JWKS al boot, cachear, refrescar
  cada 1h.
- Middleware: extrae `Authorization: Bearer`, valida firma+exp,
  inyecta `UserId` en extensions.
- Routes públicas (`/v1/health`) sin auth; el resto requerido.
- Endpoint nuevo `POST /v1/profiles/sync` que el cliente llama al
  primer login para crear la fila en `profiles` si no existe.

### P-CLD-5 — Quota + Lemon Squeezy webhooks

- Tabla `plan_limits` (constants en código si prefieres):
  `{plan, storage_bytes, devices, retention_days, ...}`.
- Middleware quota antes de `presign_put`: rechaza con 402 +
  payload `{upgrade_url, current_plan, would_exceed}`.
- Endpoint `POST /v1/webhooks/lemonsqueezy` con verify HMAC.
- Estado interno máquina:
  `created → active → cancelled (at end of period) → expired`.
- Cron diario que recorre `profiles` con `plan_cancel_at < now()`
  y baja a `free` (también marca soft-quota: si excede storage al
  bajar, no borramos; bloqueamos uploads hasta que baje voluntario).

### P-CLD-6 — Cliente: login flow + tauri-plugin-deep-link

- `tauri-plugin-deep-link` registra `hoard://`.
- Onboarding nuevo botón.
- Comandos `cloud_*` listados arriba.
- Keyring storage via `keyring` crate (`tauri-plugin-stronghold`
  alternativa).

### P-CLD-7 — UI cuenta + upgrade + sidebar chip

- Página `/account` (Svelte route).
- Modal upgrade con tabla y checkout link.
- Sidebar chip + estado plan.
- i18n keys en los 8 locales.

### P-CLD-8 — Self-hosted side: API compat layer

- Si el user ha entrado a self-hosted (sin cloud), `/account` no
  existe; sidebar muestra "Servidor propio".
- Si el user ha entrado a cloud y quiere migrar a self-hosted:
  botón "Exportar todo y desconectar" + tutorial.

### P-CLD-9 — Landing + checkout web

- Repo separado `hoard-web` (Astro). Páginas: home, precios,
  privacy, terms, contact.
- Checkout = links directos a Lemon Squeezy (sin lógica server).
- Deploy en Vercel/Netlify free tier.
- Dominio `hoard.cloud` apunta aquí; subdominio `api.hoard.cloud`
  apunta a Fly.io `hoard-server-cloud`.

### P-CLD-10 — Observabilidad + cron de mantenimiento

- Sentry SDK en server + cliente.
- Endpoint `/metrics` (Prometheus) protegido por basic-auth.
- Cron en Fly.io machines `hoard-server-cloud --cron`:
  - quota recompute diario (sanity)
  - subscription expirados → free
  - soft-delete sweep > 90/365 días según plan
  - export ZIP cleanup tras 7 días

### P-CLD-Z — Release 1.6.0 + producto público

- Bump version 1.5.5 → 1.6.0. CHANGELOG. README con sección Cloud.
- Email a beta testers (si hay).
- Anuncio en r/selfhosted, r/Games, HN, ProductHunt.

---

## 7. MCPs / connectors / skills que Claude (Opus) necesita

> El user te tiene que conectar estos antes de que arranques en
> firme. Pídelos con esta lista.

### Esenciales (sin esto Opus no puede)

1. **Supabase MCP** — `npx -y @supabase/mcp-server` (o equivalente
   actual). Permite a Claude crear tablas, RLS policies, run queries
   directos sin pegar SQL en el dashboard.
   - **Acción del user**: instalar plugin con `claude mcp add
     supabase ...` y autorizar al proyecto.
   - **Tokens necesarios**: `SUPABASE_ACCESS_TOKEN` con scope al
     proyecto `hoard-cloud-prod`.
2. **GitHub MCP oficial** — el user ya lo tendrá probablemente.
   Claude lo usa para abrir PRs, leer issues, gestionar releases.
3. **Filesystem + Bash** — ya disponibles.

### Recomendados (productividad alta)

4. **Cloudflare MCP** — si existe oficial (`@cloudflare/mcp-server`).
   Si no, usar `Bash` con `wrangler` CLI logueado. Permite crear
   buckets R2, DNS records, tokens.
5. **Fly.io MCP** — no hay oficial al momento de escribir; usar
   `flyctl` desde `Bash` autenticado.
6. **Lemon Squeezy MCP** — no hay oficial; usar API REST con
   `curl`/`reqwest` desde tests. Documentar setup manual de
   productos en el panel.
7. **Stripe MCP** (fallback si descartas Lemon Squeezy) —
   `@stripe/agent-toolkit` o el MCP server oficial.
8. **Sentry MCP** — `@sentry/mcp-server`. Crear proyectos, ver
   issues.
9. **Resend MCP** — si existe; si no, API REST.
10. **WebSearch / WebFetch** — para consultar docs Supabase /
    Lemon Squeezy / R2 al detalle (versiones cambian).

### Skills/connectors a buscar en el registry

Cuando estés conectado, prueba:

- `search_mcp_registry({ query: "supabase" })`
- `search_mcp_registry({ query: "cloudflare r2" })`
- `search_mcp_registry({ query: "stripe" })`
- `search_mcp_registry({ query: "fly.io" })`
- `suggest_connectors({ context: "saas backend with supabase
   cloudflare lemon squeezy" })`

Si algún MCP no existe, usa `curl`/SDK desde Bash. La API REST de
todos estos servicios está bien documentada.

---

## 8. Acciones del user (no de Claude)

Estas no las puedes hacer tú porque requieren tarjeta, KYC, o
verificación humana:

- [ ] Registrarse en **Supabase**, **Cloudflare**, **Lemon
      Squeezy**, **Fly.io**, **Resend**, **Sentry**.
- [ ] Comprar dominio (`hoard.cloud` u otro). Apuntar NS a
      Cloudflare.
- [ ] Verificar identidad en Lemon Squeezy (KYC para cobrar).
- [ ] Aceptar T&Cs en cada proveedor.
- [ ] Configurar método de pago en Fly.io / Cloudflare / Supabase
      (todos arrancan en free pero piden tarjeta para no ser
      bloqueados al primer pico).
- [ ] Conectar los MCPs listados arriba.
- [ ] Generar y guardar API tokens en su password manager. Después
      pasarlos a Claude por mensaje único o vía Skill
      `update-config` para añadirlos a `~/.claude/settings.json` o
      a GitHub Actions secrets si los necesita en CI.

---

## 9. Riesgos y mitigaciones

- **Coste arranque cero a producto vivo**: Free tiers de Supabase
  (500MB DB), R2 (10GB free), Fly.io (3 shared-CPU machines) y
  Lemon Squeezy (sin coste fijo) permiten lanzar a 0 €/mes hasta
  ~50 users activos.
- **Privacidad de saves**: los snapshots son `.tar.zst` cifrados en
  reposo (R2 default encryption). Considerar **cifrado cliente-
  lado** opcional en ciclo 1.7 para usuarios paranoicos
  (clave derivada de password, server nunca la ve).
- **Spam de cuentas Free**: rate-limit creación a 1/IP/hora.
  Pedir verificación email obligatoria.
- **Lemon Squeezy se cae / sube precios**: la abstracción
  `payments` debería ser un trait. Stripe ya es la alternativa
  obvia con minimal cambio.
- **GDPR / DSAR**: ya cubierto por export all + delete account.
  Documentar en `docs/privacy.md` un cambio nuevo.
- **Vendor lock-in en Supabase**: Supabase es Postgres + Auth +
  Storage. Migrar a Postgres self-hosted + cualquier OIDC provider
  es 1-2 semanas si hace falta. No es lock-in duro.

---

## 10. Definition of done

El ciclo cloud está cerrado cuando:

1. Un user nuevo abre Hoard, ve onboarding cloud, hace login con
   Google, ve su Pro upgrade modal, paga, sube su primer save,
   y todo funciona en < 5 minutos.
2. `pnpm i18n:check` zero diff entre locales (incluyendo strings
   nuevas).
3. `cargo test --workspace --all-features` zero failures.
4. Self-hosted users existentes pueden seguir corriendo
   `hoard-server` sin cambios (regression test).
5. Exportar-todo produce un ZIP que un user puede importar a un
   `hoard-server` self-hosted con `hoard-admin import`.
6. CHANGELOG.md tiene su block `## [1.6.0]`.
7. ADR 0015 publicado, plan `1.6-cloud.md` con todos los prompts
   marcados `done`.

---

## 11. Notas finales para Opus

- **No tengas miedo de pedir al user que abra una cuenta o conecte
  un MCP.** Es trabajo del humano; no se hace solo. Si te bloquea
  algo, dilo claro.
- **Lee `CLAUDE.md` (workspace) y `docs/decisions/0009.md` (path
  detection)** antes de tocar el código de detección; lo cloud no
  toca esa parte pero saber el lay of the land ayuda.
- **No rompas self-hosted**. Cada PR debe tener un test de
  regresión donde el binario sin feature `cloud` se comporta como
  hoy.
- **Mantén `hoard-server-cloud` y `hoard-server` self-hosted como
  un único binario con feature flag**, no dos binarios. Eso evita
  divergencia.
- **Pricing es opinión, no ley**: si tras hablar con users beta los
  números cambian, actualiza este documento y el ADR.
- **El user es nativo español**. Errores y respuestas en español,
  cortas, sin emojis. Mismo estilo que el resto del proyecto.

Cuando termines de leer, escribe en chat:

> `Leído. Voy a empezar por P-CLD-0: listar al user las cuentas y
> MCPs que necesita conectar antes de seguir.`

y procede.
