# Plan — Eficiencia de almacenamiento (dedup + retención + ingesta)

> Implementa [ADR 0018](../decisions/0018-storage-efficiency-dedup-retention.md).
> Sin número de versión asignado todavía (la numeración del ciclo 1.x se
> descarriló — ver `1.5.md`). Se asigna al arrancar.

Origen: bug report OpenTTD (2026-05-30). 33 versiones × 16 autosaves ≈ 53 MB
para ~5 MB de datos únicos, y todas conservadas para siempre.

Tres fases independientes y desplegables por separado. Orden recomendado:
**Fase 1 (poda) primero** — barata, dentro de la arquitectura actual, frena
el sangrado ya. Fase 2 (dedup) es el gran ahorro pero toca esquema y cuota.
Fase 3 (barra + ingesta cliente) cierra el círculo de control de usuario.

---

## Estado actual (puntos de código)

- **Creación de snapshot**: `crates/hoard-server/src/routes/snapshots.rs::create`
  — escribe una copia física por versión en
  `data/<user>/<game>/<label>/v<n>/<rel_path>`; ya calcula `sha256` por
  archivo y lo guarda en `snapshot_files`.
- **Esquema**: `migrations/0004_snapshots.sql`, `0005_snapshot_files.sql`.
  `snapshots.is_pinned` y `deleted_at` ya existen.
- **Soft-delete / papelera**: `snapshots.rs::soft_delete` mueve `v<n>/` a
  `trash/<snapshot_id>/`; `cleanup.rs::run_periodic` purga por
  `trash_retention_days`.
- **Cuota**: `users.storage_used_bytes` = suma de `total_size_bytes` de
  snapshots vivos; se ajusta en create / soft_delete / restore.
- **Cliente backup**: `crates/hoard-agent/src/backup.rs::upload_directory`
  (lee archivos, multipart) y `agent.rs::schedule_backup` /
  `run_backup_with_retry` (debounce + reintentos).
- **Descarga/restore**: `snapshots.rs::download` arma tar.zst desde la
  carpeta `v<n>`; el restore por diff del cliente vive en
  `hoard-agent/src/restore.rs`.

---

## Fase 1 — Poda ponderada por antigüedad (eje B) ✅ Implementada (2026-05-30)

> Hecho: `hoard-server/src/retention.rs` (lógica pura + tests), enganchado en
> `cleanup.rs::run_once` (corre antes del purge de papelera), config
> `[retention].snapshot_pruning` + `data_saving`, política construida en
> `main.rs`. Soft-delete reutiliza papelera + cuota. Pendiente de Fase 3: que
> `data_saving` venga de la barra por-usuario en vez de config global.

**Objetivo**: dejar de conservar versiones viejas redundantes. Sin tocar
esquema ni cuota.

1. Módulo nuevo `hoard-server/src/retention.rs`:
   - `fn plan_prune(versions: &[SnapshotMeta], policy: &RetentionPolicy) -> Vec<SnapshotId>`
     puro y testeable: recibe la lista de versiones vivas (con `created_at`,
     `version_num`, `is_pinned`) y devuelve qué `snapshot_id` soft-deletear.
   - Algoritmo GFS (ADR 0018 Decisión 2): nunca poda `is_pinned` ni la más
     nueva; conserva `keep_recent` nuevas + 1 por hora/día/semana según
     `keep_hourly/daily/weekly`; si excede `byte_cap`, sigue podando la más
     vieja no-pinned.
2. `RetentionPolicy` con defaults (= barra en `k=0.3`, ver Fase 3). De
   momento constante en `config.toml` (`[retention]`), luego la pisa la barra
   por-usuario.
3. Enganchar en `cleanup.rs::run_periodic`: por cada save, cargar versiones
   vivas, `plan_prune`, ejecutar `soft_delete` reutilizando la lógica
   existente (transacción + mover a `trash/`). Disparar también tras un
   `create` exitoso (poda incremental barata: solo ese save).
4. Tests: ring de N versiones con timestamps sintéticos → asserts sobre qué
   sobrevive (incluido "pinned siempre", "última siempre", tope de bytes).

**Aceptación**: tras jugar OpenTTD, History muestra ~10 versiones
escalonadas en vez de 33 lineales; las pinned y la última intactas.

## Fase 2 — Dedup content-addressed (eje C) ✅ Implementada (2026-05-30)

> Hecho: migración `0013_blobs.sql` (`blobs(user_id, sha256, size_bytes,
> refcount, created_at)`, PK `(user_id, sha256)`). Módulo
> `hoard-server/src/blobs.rs`: `blob_path()` (`blobs/<user>/<sha[0:2]>/<sha>`)
> + `backfill_from_folders()` (migración idempotente de los `v<n>/` y
> `trash/<id>/` legacy a blobs, recálculo de cuota, ejecutada al arrancar en
> `main.rs`). `snapshots.rs::create` deduplica (incref por archivo,
> `INSERT … ON CONFLICT DO UPDATE refcount+1`, coloca el blob una vez,
> rollback de los blobs nuevos si falla el commit); la cuota sólo cobra los
> bytes nuevos deduplicados. `download` reconstruye el tar desde blobs
> (`append_path_with_name`). `soft_delete` y `restore` son puramente lógicos
> (sin mover carpetas, sin tocar cuota). `cleanup.rs::purge_trash` decrementa
> refcount, GC del blob a 0 (fichero + fila) y reembolsa los bytes liberados;
> el pruning (Fase 1) ya no mueve carpetas ni ajusta cuota. Test
> `cleanup::purge_decrements_refcount_and_gcs_at_zero` cubre refcount + GC +
> cuota. Dedup **por usuario** (el `user_id` va en la clave del blob).

**Objetivo**: almacenar cada blob único una sola vez. El gran ahorro.

1. Migración `0013_blobs.sql`:
   ```sql
   CREATE TABLE blobs (
     sha256     TEXT PRIMARY KEY NOT NULL,
     size_bytes INTEGER NOT NULL,
     refcount   INTEGER NOT NULL DEFAULT 0,
     created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
   );
   ```
   (`snapshot_files.sha256` ya es la FK lógica; no hace falta tocar esa
   tabla.)
2. Reescribir `snapshots.rs::create`: tras hashear cada archivo, en vez de
   dejarlo en `v<n>/`, mover el blob a `blobs/<sha[0:2]>/<sha>` solo si no
   existe (`INSERT … ON CONFLICT DO UPDATE refcount=refcount+1`); si ya
   existe, descartar el tmp e incrementar refcount. La carpeta `v<n>/`
   desaparece como almacenamiento físico — la versión es puramente la lista
   de `snapshot_files`.
3. Reescribir `download` y la ruta de `restore`/diff del cliente para
   reconstruir desde blobs por sha en vez de leer `v<n>/`.
4. `soft_delete` / purge: decrementar refcount; GC de blobs a refcount 0 en
   `cleanup.rs`. Mover a `trash` se vuelve lógico (refcount), no físico.
5. **Cuota — cambio de semántica**: `storage_used_bytes` pasa a ser suma de
   `size_bytes` de blobs con refcount>0 del usuario. Script de migración que
   recalcula por usuario. Actualizar la UI de uso de plan (ADR 0015) y los
   checks de quota en `create`.
6. Dedup **por usuario** (no cross-user): el path del blob incluye el
   `user_id` o se particiona el store por usuario, para no filtrar
   existencia de contenido entre cuentas.
7. Tests: subir dos snapshots que comparten 15/16 archivos → assert 1 copia
   física por blob, refcounts correctos, borrar uno no rompe el otro,
   GC solo al llegar a 0.

**Aceptación**: 33 versiones OpenTTD ocupan ~5 MB en `blobs/`, no ~53 MB.

## Fase 3 — Barra "ahorro de datos" + ingesta cliente (ejes A + B) ✅ Implementada (2026-05-30)

> Hecho: cliente `hoard-agent` con `AgentConfig.min_snapshot_interval_secs`
> + helper `agent::min_snapshot_interval_for(k)` (`lerp(k,5,600)`). El watcher
> ancla un suelo entre snapshots por save: tras un backup exitoso fija
> `SaveSlot.last_backup_at`, y el handler de eventos fs estira el `delay` hasta
> `last_backup_at + interval` (gana sobre el anti-inanición). Knob persistido en
> `Prefs.data_saving ∈ [0,1]` (default 0.3, `#[serde(default)]` retro-compat);
> comando Tauri `set_data_saving` (clamp 0..1) + wrapper `api.setDataSaving`;
> `start_agent` deriva `min_snapshot_interval_secs` de `data_saving`. UI: slider
> "Ahorro de datos" en Settings (sección Almacenamiento, izq "Guardar todo" →
> der "Máximo ahorro") con hint del trade-off, i18n en los 8 locales. El
> server-side `data_saving → RetentionPolicy` sigue viniendo de config global
> (Fase 1); cablear el knob por-usuario al server queda pendiente. **Skip por
> hash de conjunto: diferido** — en el caso OpenTTD el contenido cambia cada
> settle, así que el peso lo lleva el intervalo mínimo; el hashing del conjunto
> en cliente añade IO/complejidad sin pagar aquí. Se reabre si aparece el caso
> de reescrituras bit-idénticas.

**Objetivo**: dar el control al usuario y reducir versiones en origen.

1. Cliente (`hoard-agent`):
   - `min_snapshot_interval` en `AgentConfig`: suelo entre backups por save
     (tras éxito, no re-subir hasta pasado el intervalo; coalescer cambios).
   - Skip por hash de conjunto: antes de subir, comparar
     `{(rel_path, sha256)}` con el del último snapshot; si idéntico, no-op.
2. Persistencia del knob `data_saving ∈ [0,1]` (settings de usuario, mismo
   patrón que `auto_restore` / sliders de ADR 0014). Mapea a
   `min_snapshot_interval` (cliente) y a `RetentionPolicy` (server, Fase 1)
   vía las fórmulas de ADR 0018 Decisión 4.
3. UI: slider en `Settings.svelte` ("Ahorro de datos", izq "Guardar todo" →
   der "Máximo ahorro"), con texto de ayuda que explique el trade-off.
   i18n en los 8 locales (`en` fuente de verdad).
4. Default `k=0.3`.

**Aceptación**: mover la barra cambia visiblemente cadencia de versiones y
cuántas se conservan; el usuario entiende qué hace.

---

## Riesgos / cuidado

- **Migración de cuota (Fase 2)**: recalcular `storage_used_bytes` mal deja a
  usuarios sobre/infra-facturados. Hacerlo idempotente y verificable.
- **Refcount races**: incrementos/decrementos de `blobs.refcount` deben ir en
  la misma transacción que el `snapshot_files`. Nunca GC un blob fuera de tx.
- **Compatibilidad de descarga**: clientes viejos esperan `v<n>/`; el
  endpoint `download` debe seguir devolviendo el mismo tar.zst aunque por
  dentro lea de blobs.
- **Poda y multi-dispositivo**: la poda es server-side y global por save, así
  que es consistente entre dispositivos; no depende de qué cliente corre.
