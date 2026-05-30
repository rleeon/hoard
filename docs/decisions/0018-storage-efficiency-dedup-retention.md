# 0018 — Eficiencia de almacenamiento: dedup + retención + ingesta

- **Status**: Accepted (Fases 1-3 implementadas; skip-by-hash del eje A
  diferido — ver plan Fase 3)
- **Date**: 2026-05-30
- **Supersedes**: parte del modelo "una versión = una copia física" implícito
  en ADR 0001 / esquema `0004_snapshots` + `0005_snapshot_files`.
- **Depende de**: backlog *content-addressed storage* (`docs/plans/1.5.md`
  §3.1.5), que esta ADR desbloquea y concreta.
- **Context**: bug report de OpenTTD (2026-05-30) — ver más abajo.

## Contexto

Probando Modo Automático con OpenTTD (autosave cada ~3-5 s) salieron dos
problemas de comportamiento (ya corregidos en `hoard-agent`, ver
[Anexo A](#anexo-a--correcciones-de-agente-ya-aplicadas)) y un tercero
**estructural** que es el motivo de esta ADR.

La carpeta que sincronizamos (`~/.local/share/openttd/save/`) contiene
`autosave/autosave0.sav … autosave15.sav`: el juego mantiene su **propio
buffer circular** de 16 partidas y rota — cada autosave nuevo pisa el slot
más viejo. Hoard versiona esa carpeta entera en cada settle del watcher.
Resultado real observado:

```
33 versiones × 16 archivos × ~1.6 MB  ≈  53 MB en disco
datos únicos reales                   ≈  ~5 MB
```

Son **dos historiales anidados**: 33 versiones Hoard, cada una con 16
autosaves, y entre `v32` y `v33` solo cambian 1-2 archivos — los otros
14-15 son idénticos bit a bit y se vuelven a almacenar enteros. ~10× de
desperdicio, creciendo cada minuto de juego, y todas las versiones se
conservan para siempre.

El modelo actual (`routes/snapshots.rs::create`) escribe una copia física
por versión en `data/<user>/<game>/<label>/v<n>/<rel_path>`. Ya calcula y
persiste `sha256` por archivo en `snapshot_files`, pero **no** lo usa para
deduplicar: los bytes se guardan repetidos.

## Tres ejes (no confundirlos — arreglan cosas distintas)

| Eje | Pregunta | Qué ahorra | Coste |
|-----|----------|-----------|-------|
| **A. Ingesta** | ¿Cuándo creamos un snapshot? | Menos versiones de entrada | Bajo |
| **B. Retención** | ¿Cuántas versiones conservamos? | Versiones viejas redundantes | Bajo |
| **C. Dedup (CAS)** | ¿Cómo almacenamos los bytes? | Archivos idénticos entre versiones | Alto (migración) |

La barra "ahorro de datos" que pidió el user vive en **A + B**. El ahorro
brutal del caso OpenTTD vive en **C**.

## Decisión 1 — Almacenamiento content-addressed (eje C)

Sustituir "una copia por versión" por un **blob store direccionado por
contenido**:

- Los bytes de cada archivo se guardan una sola vez en
  `blobs/<sha256[0:2]>/<sha256>` (sharding por prefijo para no saturar un
  solo directorio).
- `snapshot_files.sha256` pasa a ser la referencia al blob; ya existe, no
  hay que recalcular nada en el cliente ni en el server.
- Nueva tabla `blobs(sha256 PK, size_bytes, refcount, created_at)`. El
  `refcount` se incrementa al crear un `snapshot_files` que lo referencia y
  se decrementa al borrar (soft-delete cuenta como referencia viva hasta el
  purge real desde `trash`).
- GC: cuando `refcount` llega a 0 tras un purge, se borra el blob físico.

Consecuencias en cuota: `users.storage_used_bytes` deja de ser "suma de
`total_size_bytes` de snapshots vivos" y pasa a ser **suma de tamaños de
blobs únicos referenciados** por el usuario. Esto es un cambio de
contabilidad que afecta a planes Cloud (ADR 0015) y hay que migrarlo con
cuidado (ver plan, fase 2). El caso OpenTTD: 33 versiones referenciando
~50 blobs únicos en vez de 528 copias.

Por qué sha256 y no chunking FastCDC: los saves de juego son archivos
pequeños-medianos y enteros; la unidad natural de dedup es el archivo, no el
chunk. Dedup por archivo nos da casi todo el beneficio (el caso típico es
"un archivo cambió de N") con una fracción de la complejidad. FastCDC queda
fuera de scope; si algún día aparecen saves monolíticos enormes que mutan
parcialmente (un único `.sav` de 500 MB) se reabre como ADR aparte.

## Decisión 2 — Poda ponderada por antigüedad (eje B)

Reemplazar "conservar todo para siempre" por poda **GFS-style** (grandfather-
father-son), no por "1 de cada N" (que es tosco y tira granularidad
reciente). Para cada save, la política decide qué versiones sobreviven:

```
conserva siempre:
  - las is_pinned (nunca se podan, no cuentan a ningún cap)
  - la versión más nueva (jamás se borra la última)
luego, por ventanas de recencia:
  - todas las de la última sesión / última hora
  - 1 por hora, últimas keep_hourly horas
  - 1 por día,  últimos keep_daily días
  - 1 por semana, últimas keep_weekly semanas
finalmente, si se supera el tope de bytes del save:
  - soft-delete de la más vieja no-pinned hasta bajar del tope
```

La poda usa el `soft_delete` existente (mueve a `trash/`, se purga tras
`trash_retention_days`), así que **nada se borra irreversiblemente de
golpe** — el user tiene la papelera como red. Corre en el task periódico de
`cleanup.rs`.

Los parámetros `keep_*` y el tope de bytes salen de la barra (Decisión 4).

## Decisión 3 — Intervalo mínimo + skip por contenido idéntico (eje A)

Dos cambios en el cliente (`hoard-agent`):

1. **Intervalo mínimo entre snapshots por save** (`min_snapshot_interval`).
   Hoy el debounce es de 5 s y cualquier settle crea versión. Se añade un
   suelo: tras un backup exitoso, no se crea otro hasta pasado
   `min_snapshot_interval`. Los cambios intermedios se coalescen en el
   siguiente. Mata la cadencia de "una versión por minuto" sin perder el
   dato (siempre se sube el estado final del intervalo).

2. **Skip por hash de conjunto**: antes de subir, si el conjunto
   `{(rel_path, sha256)}` de la carpeta es idéntico al del último snapshot,
   no se crea versión (no-op silencioso). Evita versiones duplicadas exactas
   (p.ej. el juego reescribe un archivo con bytes idénticos, o un settle
   espurio del watcher). Nota: en OpenTTD el contenido sí cambia cada vez,
   así que aquí el peso lo lleva el intervalo mínimo, no el skip.

## Decisión 4 — Barra "ahorro de datos" (un mando 0..1)

Un único knob en Settings, `data_saving ∈ [0,1]` (UI: izquierda "Guardar
todo" → derecha "Máximo ahorro"), que escala los dos ejes baratos. Con
`lerp(k,a,b) = a + (b-a)·k`:

```
# Eje A — ingesta
min_snapshot_interval = lerp(k, 5s, 600s)      # 5s actual … 10min

# Eje B — retención (GFS)
keep_recent  = round(lerp(k, 20, 3))           # versiones nuevas intactas
keep_hourly  = round(lerp(k, 24, 6))
keep_daily   = round(lerp(k, 14, 3))
keep_weekly  = round(lerp(k,  8, 2))
byte_cap     = plan_quota · lerp(k, 1.00, 0.25) # opcional, tope por save
```

`k = 0` ≈ comportamiento actual (guarda agresivo, conserva mucho). `k = 1`
≈ solo lo esencial reciente. Default propuesto: `k = 0.3` (un poco de ahorro
de fábrica, porque el caso OpenTTD demuestra que el default de "guardar
todo" sorprende mal al user).

El plan Cloud (Free/Pro) puede mover el rango de `byte_cap` y el default,
pero la barra es del usuario en todos los planes.

## Consecuencias

- Caso OpenTTD: de ~53 MB a ~5 MB por dedup, y de 33 versiones a ~10 por
  poda. El feed deja de parecer descontrolado.
- `storage_used_bytes` cambia de semántica (suma de blobs únicos). Requiere
  migración de datos y recálculo; afecta a la UI de uso de plan.
- Self-hosted y Cloud comparten el blob store; el dedup es **por usuario**
  (no cross-user, para no filtrar existencia de saves entre cuentas).
- La poda nunca toca `is_pinned` ni la última versión: el user siempre
  puede fijar un hito y no perderlo.
- Nada se borra duro sin pasar por `trash/` + TTL.
- `download`/`restore` siguen reconstruyendo la carpeta `v<n>` a partir de
  `snapshot_files` + blobs; el tar.zst de descarga no cambia de formato.

## Alternativas descartadas

- **"1 de cada N" para la retención**: simple pero tira granularidad
  reciente (lo que más importa). GFS conserva densidad donde el user la
  necesita.
- **Excluir `autosave/` del tracking**: para muchos usuarios de OpenTTD el
  autosave *es* su partida. No podemos asumir que es ephemeral. Se descarta
  como regla global; podría ofrecerse como override por-juego más adelante.
- **Chunking FastCDC**: sobredimensionado para saves de archivos pequeños.
  Ver Decisión 1.

## Plan de implementación

Ver [`docs/plans/storage-efficiency.md`](../plans/storage-efficiency.md):
fases, migraciones, puntos de código y criterios de aceptación.

## Anexo A — correcciones de agente ya aplicadas

Antes de esta ADR se corrigieron en `hoard-agent/src/agent.rs` tres bugs de
comportamiento que el caso OpenTTD destapó (no son el problema estructural,
pero lo enmascaraban):

1. **Auto-restore mid-sesión** (riesgo de pisar progreso): el guard de
   "usuario jugando" del sweep dependía de `is_running` (falla si el nombre
   de proceso no casa con el manifest) y del mtime del *directorio* (no
   cambia en reescrituras in-place). Ahora gatea con la actividad real del
   watcher (`has_pending` + `last_fs_event_at`), inmune a ambos fallos.
   Refina la Decisión 3 de ADR 0014.
2. **Flood de "en cola — esperando…"**: `schedule_backup` re-emitía
   `BackupScheduled` en cada escritura, dejando filas huérfanas que nunca
   resolvían. Ahora solo emite en el flanco de subida.
3. **Inanición** ("se quedó todo en cola"): el debounce de 5 s se reiniciaba
   en cada escritura y nunca vencía. Nuevo tope `MAX_BACKUP_WAIT_SECS` (30 s)
   fuerza la subida aunque el juego no pare de escribir.
