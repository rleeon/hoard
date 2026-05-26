# 0016 — LiveStatus, ActivityFeed y cadencia dual de cloud

- **Status**: Accepted
- **Date**: 2026-05-26
- **Context**: 1.7.0
- **Supersedes**: nada (extiende ADR
  [0012](0012-ux-polish-and-automatic-mode.md) y
  [0014](0014-conflict-aware-restore-and-game-activity-skip.md))

## Contexto

Tras 1.6.1 (Free/Pro + bandwidth limiter + tope por save + modo
backup-only) el Modo Automático funciona pero es invisible. El user
no tiene forma de saber:

- Si el watcher está armado y qué saves está siguiendo.
- Si la última subida funcionó, está esperando, o tropezó con la
  cuota.
- Si hay versiones nuevas en otra máquina pendientes de bajar.

Toda esa información ya existe en `hoard-agent` (vía
`AgentEvent::{BackupStarted, BackupSuccess, BackupScheduled, ...}` y
estados internos del watcher) pero se quema en logs. Esta ADR define
el contrato UI ↔ agent para exponerla en vivo, y desacopla la cadencia
del cloud poll del scheduler horario pesado que ya existe.

## Decisiones

### D1 — `agent://*` como bus de eventos primario UI ↔ shell

El forwarder de `hoard-desktop/src/commands/agent.rs` ya re-emitía
algunos eventos del agente al frontend bajo prefijo `agent://`. En
1.7.0 esto se eleva a contrato:

- Cada transición observable del ciclo de vida emite un topic
  estable. Los topics nuevos son:
  - `agent://watcher-armed` (synthetic, emitido desde el shell tras
    `agent::spawn`).
  - `agent://upload-started` (alias de `BackupStarted`).
  - `agent://upload-completed` (alias de `BackupSuccess`).
  - `agent://throttled` (`BackupScheduled { reason: FilesystemSettled }`).
  - `agent://cloud-pull-started` / `agent://cloud-pull-completed`
    (nuevos, emitidos por el poller — ver D3).
  - `agent://quota-reached` (emitido por el poller cuando el server
    devuelve 402 / quota error en una lectura).
  - `agent://offline` (emitido por el poller cuando un poll falla por
    red).
- El forwarder mantiene los topics legacy (`agent://backup-started`,
  etc.) para no romper consumers existentes; los nuevos coexisten.
- Los payloads son JSON serde con `#[derive(Serialize)]` y campos
  estables: `save_id`, `game_slug`, `bytes`, `error`.

Razón: una vez que la sidebar y el panel viven de eventos en lugar de
polling de estado, evita drift entre lo que la UI cree y lo que el
agente está haciendo. Topics versionados como strings dan margen para
añadir sin tocar el handshake.

### D2 — Estado vivo derivado en `lib/stores/live.ts`

Un único store de Svelte 5 con cuatro writables (`watcher`,
`cloudLoop`, `activityFeed`, `seenArmed`) y un derived `liveStatus`.
Suscribe a todos los topics `agent://*` al montar `App.svelte` y
mantiene:

- `watcher`: `{ state: 'unknown'|'watching'|'off', tracked: number }`.
- `cloudLoop`: `{ state: 'unknown'|'online'|'throttled'|'offline' }`.
- `activityFeed`: ring buffer FIFO acotado a `MAX_FEED_ENTRIES` (80).
- `seenArmed`: `Set<save_id>` para deduplicar el primer
  `watcher-armed` por save (el watcher rearma en cada
  `set-tracking`).

Razón: la UI consume el store; los componentes no se acoplan a la
forma de los topics. El cap del buffer evita memory leak con cadencia
agresiva.

### D3 — Cadencia dual: scheduler horario + cloud poll de bajo coste

ADR 0013 introdujo `AutomaticScheduler` con tick configurable
(`automatic_scan_interval_hours`, default 6h). Ese tick es **caro**:
re-escanea el catálogo, hace diff con el server y dispara restores
condicionales. Llevarlo a 10 s degradaría I/O sin razón.

Se introduce un segundo loop independiente:
**`cloud_pull` poller** en `hoard-desktop/src/commands/cloud_pull.rs`.

- Cadencia: `cloud_poll_interval_secs` (nuevo pref, default 10 s,
  slider 5..=300 en Settings).
- Trabajo por tick: un GET ligero al manifest del user (HEAD-style),
  comparar lista `(save_id, version)` con la última seeded, y emitir
  `agent://cloud-pull-completed` con `new_versions: number`.
- **No descarga nada.** Solo notifica. La razón es la condición de
  carrera con el user editando archivos localmente — un download
  silencioso podría sobrescribir progreso. El user (o el scheduler
  horario) decide si bajar.
- Estados emitidos:
  - HTTP 2xx → `cloudLoop.state = 'online'`.
  - HTTP 402 / "quota_reached" en payload → `quota-reached` +
    `cloudLoop.state = 'throttled'`.
  - Falla red → `offline` + `cloudLoop.state = 'offline'`.
- El primer poll tras login se considera "seeding": las versiones
  vistas no cuentan como nuevas (evita un spam de notificaciones al
  iniciar sesión).

Razón: separar "checar pulso del cloud" de "reconciliar saves" baja
el coste del feedback en vivo a ≈1 request HTTP de pocos KB cada 10 s.
El scheduler pesado sigue gobernando los restores.

### D4 — `live_activity_visible` como preferencia persistida

El panel `ActivityFeed.svelte` flota abajo-derecha. La preferencia
`live_activity_visible` (default `true`) se persiste como otras
prefs y se controla desde:

- Botón `ScrollText` en el header (toggle).
- Botón "Ocultar panel" dentro del propio panel.
- Toggle dedicado en Settings → sección Cloud.

Razón: usuarios power lo quieren siempre visible; usuarios casuales
agradecen ocultarlo y solo ver el LiveStatus de dos puntos en el
sidebar footer. Persistir entre sesiones evita que el user lo
re-ocultes cada vez.

### D5 — `cloud-pull-started` no se añade al feed

A 10 s de cadencia, escribir un evento "Polling cloud…" en el feed
visible dominaría la lista. Solo `cloud-pull-completed` con
`new_versions > 0` produce un row visible; el resto solo actualiza
`liveStatus`.

Razón: la UI debe destacar señal, no ruido. El estado en vivo del
sidebar ya refleja que el polling sigue.

## Consecuencias

- El user ve si Modo Automático está vivo desde el primer momento
  sin abrir Settings ni logs.
- El handshake UI ↔ shell pasa a depender de strings `agent://*`
  estables — cualquier cambio futuro de payload tiene que mantener
  compatibilidad o emitir un topic nuevo.
- Self-hosted no se rompe: el poller solo arranca cuando hay
  `cloud_account` registrada (`restart_if_signed_in`). En modo
  self-hosted puro el `cloudLoop.state` queda en `unknown` y
  `LiveStatus` no muestra el segundo punto.
- Tres bumps de tráfico por user activo: 1 GET cada 10 s ≈ 360 req/h
  cada cliente. Acotable subiendo `cloud_poll_interval_secs` (slider
  llega a 300 s).
- 36 nuevas i18n keys en 8 locales; landing (web/) ya cubre
  Free/Pro y no necesita cambios estructurales en esta ADR.
