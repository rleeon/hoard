# 0015 — Hoard Cloud (SaaS opt-in)

- **Status**: Accepted
- **Date**: 2026-05-24
- **Context**: 1.6.0
- **Supersedes**: nada (extiende el modelo self-hosted, no lo
  reemplaza)

## Contexto

Hoard hoy es self-hosted puro: el user (o un amigo técnico) levanta
`hoard-server` en un VPS, abre puerto, configura reverse-proxy,
gestiona tokens. Eso filtra brutalmente a quién puede usarlo. El
objetivo del ciclo 1.6.0 es ofrecer un **SaaS opcional ("Hoard
Cloud")** que cualquier gamer pueda usar con login social y plan de
pago, manteniendo el self-hosted como ciudadano de primera clase.

Brief completo de exploración + razones de stack en
[`docs/cloud/CLOUD-HANDOFF.md`](../cloud/CLOUD-HANDOFF.md). Esta
ADR resume las decisiones arquitectónicas vinculantes.

## Decisiones

### D1 — Binario único con feature flag `cloud`

`hoard-server` mantiene su comportamiento actual (SQLite + bearer
tokens + storage en disco) por defecto. La feature `cloud` activa
dependencias opcionales (`sqlx/postgres`, `aws-sdk-s3`,
`jsonwebtoken`) y módulos nuevos (`cloud::auth`, `cloud::r2`,
`cloud::quota`, `cloud::webhooks`, rutas `/v1/cloud/...`).

Razón: evita divergir el código en dos binarios distintos. Un solo
crate, dos perfiles de build. Self-hosted users pagan cero coste de
compilación adicional.

### D2 — Postgres en paralelo, SQLite intacto

Migraciones SQLite existentes (`migrations/*.sql`) no se tocan.
Migraciones Postgres viven en `migrations/postgres/*.sql` y son la
fuente de verdad para Supabase. Schema cloud no es 1:1 con
self-hosted: añade `subscriptions`, RLS, hooks Supabase auth.

Pool DB en runtime: `enum DbPool { Sqlite(SqlitePool),
Postgres(PgPool) }` decidido por `config.database.backend`.
Consultas existentes contra SQLite siguen siendo `query!` macro
(compile-time check). Consultas cloud nuevas usan `query()` runtime
cuando son cross-backend, o están aisladas en módulo `cloud::db`
con `query!` Postgres.

### D3 — Storage abstracción `SnapshotStore`

Trait pequeño con `put`/`get`/`delete`/`presign_*`. Dos impl:

- `FsStore` (default self-hosted): wrap del comportamiento actual.
- `R2Store` (cloud): `aws-sdk-s3` apuntando al endpoint R2.
  Cliente sube directo a R2 con presigned PUT URL (el server no
  intermedia bytes).

Razón: R2 sin egress fees es **crítico** para el modelo de
negocio. Un user con 50 GB de saves descarga varias veces; con S3 /
Supabase Storage el coste de egress mata márgenes.

### D4 — Auth: Supabase JWT, no tokens nuestros

Cloud mode: el cliente obtiene un JWT de Supabase Auth tras OAuth
(Google/GitHub/Discord/Apple/email). El server valida firma contra
la JWKS pública de Supabase (cache de 1 h). No emitimos tokens
propios.

Self-hosted mode: sigue con bearer tokens en `api_tokens`. Cero
cambios.

### D5 — Pagos: Lemon Squeezy (Merchant of Record)

LS gestiona IVA UE por nosotros — crítico porque el user freelance
en España no puede manejarse con Stripe sin asesoría fiscal. Comisión
~5 % + Stripe-equivalent. La abstracción `payments` queda como
trait pequeño; Stripe sigue siendo plan B sin reescribir nada del
schema.

### D6 — Precios: Free real + Pro + Pro+

| Plan       | Mensual  | Anual    | Storage   | Dispositivos | Saves | Retención |
| ---------- | -------- | -------- | --------- | ------------ | ----- | --------- |
| Free       | 0 €      | 0 €      | 500 MB    | 1            | 3     | 7 días    |
| Pro        | 3,99 €   | 39 €     | 50 GB     | 5            | ∞     | 90 días   |
| Pro+       | 9,99 €   | 99 €     | 200 GB    | ∞            | ∞     | 365 días  |

Free es real (no demo). Anual = 20 % off. Sin trial.

### D7 — Deploy: Fly.io en `cdg`

Multi-stage Dockerfile, distroless runner. Region inicial `cdg`
(París, baja latencia EU). Autoscaling 1-3 máquinas. Healthcheck
`GET /v1/health`.

### D8 — Dominio: hoard.services

Decisión del user. `hoard.services` apunta a la landing Astro
(Vercel/Netlify free). `api.hoard.services` apunta a Fly.io
`hoard-server-cloud`.

### D9 — Compromiso "no lock-in"

Botón **Exportar todo** en `/account` produce un ZIP con todos los
saves del user en formato `.tar.zst` original (mismo que produce
`hoard-admin export`). El user puede importar ese ZIP a un
`hoard-server` self-hosted sin pérdida.

Botón **Borrar cuenta** marca soft-delete con 30 días de gracia,
luego purga snapshots de R2 y rows en Postgres.

Ambos botones son obligatorios para arrancar (GDPR + promesa
pública).

### D10 — Self-hosted no se rompe

Regression test obligatoria: `cargo check --workspace` (sin
features) y `cargo test --workspace` (sin features) deben pasar.
Cualquier PR cloud que rompa el binario sin feature `cloud` es
rejected.

## Consecuencias

- **+**: producto vendible a usuarios no técnicos; revenue path
  claro; self-hosted intacto.
- **+**: coste arrancable (free tiers Supabase + R2 + Fly + LS
  permiten ~50 usuarios activos a 0 €/mes).
- **+**: arquitectura modular — Lemon Squeezy → Stripe es swap de
  un módulo si hace falta.
- **−**: complejidad. Postgres + SQLite en mismo crate añade
  superficie de tests. Mitigamos con `enum DbPool` y feature flag.
- **−**: cambios en el flow de auth del cliente (deep links,
  keyring). Riesgo de fricción en primer launch — mitigado con
  onboarding nuevo de dos botones.

## Alternativas descartadas

- **Firebase**: vendor lock-in Google, NoSQL no encaja.
- **AWS Cognito**: complicado, latente.
- **Auth0**: caro pasado el free tier.
- **Stripe directo**: forzaría gestionar IVA por país.
- **Polar.sh**: joven, MoR limitado a algunos países UE.
- **Backblaze B2**: R2 sin egress es mejor para nuestro caso.
- **Dos binarios separados (`hoard-server` y `hoard-server-cloud`)**:
  diverge rápido. Feature flag es lo correcto.

## Referencias

- Handoff completo: [`docs/cloud/CLOUD-HANDOFF.md`](../cloud/CLOUD-HANDOFF.md)
- Plan: [`docs/plans/1.6-cloud.md`](../plans/1.6-cloud.md)
- ADR 0001 (SQLite): [`0001-use-sqlite.md`](0001-use-sqlite.md) —
  sigue válida para self-hosted.
- ADR 0002 (Bearer tokens): [`0002-bearer-tokens.md`](0002-bearer-tokens.md) —
  sigue válida para self-hosted.
