# 0019 — Ingesta adaptativa por forma del save + detección de Steam Cloud

## Status

**Aceptada e implementada (2026-05-31).** Las cuatro fases están en `main`;
plan y resultados en [`docs/plans/adaptive-ingestion.md`](../plans/adaptive-ingestion.md).
Extiende —no supersede— ADRs [0009](0009-path-detection-overhaul.md),
[0010](0010-aggressive-discovery-and-delete.md),
[0011](0011-windows-detection-and-extra-prefixes.md) (eje detección) y
[0018](0018-storage-efficiency-dedup-retention.md) (eje almacenamiento).
Concreta el chunking que 0018 dejó explícitamente "para un ADR aparte" y el
skip-by-hash que 0018 difirió.

> **Dos desviaciones frente a este borrador, decididas en implementación**
> (ver §"Actualización tras implementación" al final):
> 1. El **chunking se hace en el server**, no en el cliente: el cliente sube el
>    archivo entero (por streaming) y el server lo trocea. Evita duplicar el CDC
>    en dos lenguajes y no toca el protocolo de subida.
> 2. El CDC es **propio** (gear-hash, ~40 líneas en `chunking.rs`), sin la
>    dependencia `fastcdc`.
> 3. El skip-by-hash usa una firma **barata** `(rel_path, size, mtime)`, no
>    `(rel_path, sha256)`, para no releer todos los bytes en cliente.

## Contexto

La auditoría de 2026-05-31 (ver `docs/plans/` y el informe asociado) confirmó
que la detección y la ingesta tienen carencias que comparten una raíz:
**el pipeline asume una sola forma de save**. ADR 0018 ya resolvió el caso
"muchas versiones casi idénticas" (OpenTTD) con dedup por SHA de archivo +
poda GFS + intervalo mínimo. Quedan tres formas que el diseño único no cubre,
más dos grietas de detección.

### Almacenamiento — lo que el dedup-por-archivo NO cubre

1. **Un archivo monolítico grande que muta parcialmente** (una DB de save de
   cientos de MB que reescribe unos pocos KB cada partida). El dedup de
   [`blobs.rs`](../../crates/hoard-server/src/blobs.rs) es por **SHA de archivo
   entero**: cualquier cambio reescribe el SHA completo, así que cada versión
   almacena el archivo entero de nuevo. Ahorro ≈ 0. Es justo el caso que ADR
   0018 §Decisión 1 dejó fuera ("si aparecen saves monolíticos enormes que
   mutan parcialmente… se reabre como ADR aparte").

2. **Miles de archivos diminutos.** El cliente
   [`backup.rs::upload_directory`](../../crates/hoard-agent/src/backup.rs#L79)
   lee **cada archivo entero a memoria** (`tokio::fs::read`, línea 106) y los
   acumula todos como `Part`s de un único `multipart::Form`; el snapshot
   completo vive en RAM antes de enviarse. El server
   [`snapshots.rs::create`](../../crates/hoard-server/src/routes/snapshots.rs#L130)
   rechaza con `MAX_FILES_PER_SNAPSHOT = 1000` ("too many files in snapshot")
   y crea una fila `snapshot_files` + un `INSERT … ON CONFLICT` de `blobs` por
   archivo. Un save de 5 000 archivos de 1 KB ni siquiera sube: 400 KB reales,
   rechazo duro.

3. **Un archivo enorme (p.ej. 2 GB) que cambia poco.** Mismo problema de RAM:
   `backup.rs` lo carga entero (2 GB+ en cliente) y `restore.rs::download_snapshot`
   ([línea 139](../../crates/hoard-agent/src/restore.rs#L139)) hace
   `read_to_end` del entry entero a un `Vec` (2 GB+ en restore). Más el dedup-0
   del punto 1.

Límites actuales relevantes: `MAX_FILES_PER_SNAPSHOT = 1000`
(`snapshots.rs:130`), `max_snapshot_size_mb` (config, chequeado en streaming en
`snapshots.rs:223`), cuota por bytes de blobs únicos (ADR 0018).

### Detección — dos grietas concretas

4. **Steam Cloud (`userdata/<id>/<appid>/remote/`) se ignora.** El primitivo
   [`steam::steam_user_dirs`](../../crates/hoard-agent/src/steam.rs#L359) existe
   pero **ningún consumidor lo llama** (`detect_all` / `diagnose` no lo
   referencian). El placeholder `<storeUserId>` / `<gameId>` en
   [`pathexpand.rs:185-189`](../../crates/hoard-agent/src/pathexpand.rs#L185)
   devuelve `Vec::new()` (descarta la plantilla) con un comentario que afirma
   "detection.rs handles the wildcard case separately" — **falso**: nadie lo
   maneja. Los juegos cuyo único save vive en Steam Cloud remote no aparecen.

5. **Plantilla literal absoluta mal expandida.**
   [`pathexpand.rs:32`](../../crates/hoard-agent/src/pathexpand.rs#L32) hace
   `trim_start_matches('/')` y devuelve un `PathBuf` **relativo**
   (`/etc/games/foo` → `etc/games/foo`); el test `literal_path_passes_through`
   (línea 493) consagra el bug. Es la grieta #4 de ADR 0009 que quedó sin
   cerrar. Impacto bajo (Ludusavi casi no emite literales absolutos) pero es
   corrección barata.

## Decisión 1 — Ingesta adaptativa por forma del save (eje almacenamiento)

El cliente elige **modo de transporte** según `(file_count, total_size,
max_file_size)` medidos en `walk_source`. El dedup por contenido del server se
mantiene como invariante; cambia *cómo* llegan los bytes y *cómo* se almacenan
los archivos grandes.

| Forma | Disparador (umbral propuesto, configurable) | Modo |
|-------|---------------------------------------------|------|
| Normal | `file_count ≤ 500` y `max_file_size < 128 MB` | **Por-archivo** (actual) |
| Muchos archivos pequeños | `file_count > 500` | **Empaquetado** (tar stream) |
| Archivo monolítico grande | algún archivo `> 128 MB` | **Chunked** (FastCDC) |

1. **Streaming siempre (baseline, ortogonal al modo).** Reescribir
   `upload_directory` para enviar cada archivo con
   `reqwest::Body::wrap_stream` / `Part::stream` desde disco en vez de
   `fs::read` a memoria. Reescribir `restore.rs` para volcar cada entry del tar
   a disco por streaming en vez de `read_to_end`. Mata el techo de RAM de los
   casos (2) y (3). **No toca el server ni el formato.**

2. **Modo empaquetado.** Cuando el disparador de "muchos archivos" salta, el
   cliente sube un único `tar` (sin comprimir; los blobs ya se almacenan tal
   cual y la compresión la decide el server) con un campo multipart nuevo
   `pack` en vez de N campos `files`. El server detecta el campo `pack`,
   lo desempaqueta a `tmp/`, y **hashea por-archivo igual que hoy** (mismo
   dedup, mismas filas `snapshot_files`, mismos blobs). El cap
   `MAX_FILES_PER_SNAPSHOT` sube a un valor alto (p.ej. 50 000) sólo para el
   modo empaquetado; el por-archivo conserva su cap. Ahorra handles, RAM y
   round-trips multipart; **no cambia el modelo de almacenamiento**.

3. **Modo chunked (FastCDC) — sólo para archivos sobre el umbral.** Para un
   archivo `> 128 MB`, el cliente lo parte con FastCDC (content-defined
   chunking, ventana ~1–4 MB) y sube los chunks que el server no tenga ya.
   Nuevo almacenamiento:

   ```sql
   CREATE TABLE chunks (
     user_id    TEXT NOT NULL,
     sha256     TEXT NOT NULL,
     size_bytes INTEGER NOT NULL,
     refcount   INTEGER NOT NULL DEFAULT 0,
     created_at TEXT NOT NULL DEFAULT (...),
     PRIMARY KEY (user_id, sha256)
   );
   CREATE TABLE snapshot_file_chunks (
     snapshot_file_id TEXT NOT NULL,
     ordinal          INTEGER NOT NULL,
     chunk_sha256     TEXT NOT NULL,
     PRIMARY KEY (snapshot_file_id, ordinal)
   );
   ```

   Los chunks viven en `chunks/<user>/<sha[0:2]>/<sha>` (misma forma que
   `blobs/`). `snapshot_files.sha256` sigue siendo el SHA del **archivo
   entero** (para verificación en restore y skip-by-hash); cuando el archivo
   está chunked, sus filas en `snapshot_file_chunks` reconstruyen el contenido.
   GC de chunks por refcount idéntico al de blobs. Modelo **mixto**: archivos
   bajo el umbral siguen como blob entero; sólo los grandes se trocean. El
   `download` reensambla el tar.zst leyendo blobs o concatenando chunks según
   el archivo — **formato de descarga sin cambios**.

## Decisión 2 — Skip-by-hash de conjunto (eje A, revivido de ADR 0018)

Reactivar el skip-by-hash que ADR 0018 Fase 3 difirió, como **opt-in** barato
en cliente: antes de subir, si `{(rel_path, sha256)}` del directorio es
idéntico al del último snapshot conocido (cacheado en `state.json` por save),
no se crea versión (no-op). Cubre reescrituras bit-idénticas y settles espurios
del watcher. No pesa en OpenTTD (allí cambia el contenido), pero sí en juegos
que reescriben sin cambiar bytes.

## Decisión 3 — Detección de Steam Cloud + `<storeUserId>`

1. Nueva etapa en `detect_all` (tras el cross-ref Steam, antes del walker):
   para cada `SteamApp` instalado, expandir `userdata/<id>/<appid>/remote/`
   usando `steam::steam_user_dirs` y, si existe en disco, mergear como hit de
   filesystem (`merge_fs_hit`) con `Confidence::Medium`. Reusa todo el pipeline
   posterior (refine, overrides).

2. Enseñar a `expand_path` a abanicar `<storeUserId>` / `<gameId>` sobre los
   directorios de usuario descubiertos en vez de devolver `Vec::new()`. Así las
   plantillas Ludusavi que usan `<storeUserId>` (no pocas) dejan de caer al
   suelo. La etapa de (1) cubre el caso aunque el catálogo no tenga plantilla.

3. Corregir `pathexpand.rs:32`: una plantilla literal absoluta se devuelve
   **tal cual** (sin `trim_start_matches`), y se actualiza el test
   `literal_path_passes_through`. Limpiar el comentario stale de `<storeUserId>`
   y el de `winDocuments` (la resolución OneDrive ya corre primero vía
   `windows_known_folder`).

## Decisión 4 — Validación por contenido para desambiguar (eje detección)

Cuando `refine_save_dir` produce **varios** subdirectorios `save*` (ambigüedad)
o el walker agresivo dispara, puntuar candidatos con la señal que el walker ya
tiene (`dir_has_recent_save_file`: extensión save-like + mtime < 90 días) y
preferir el que la cumple. Reduce falsos positivos del tipo "save settings" /
carpetas de editor, sin tabla hardcodeada nueva.

## Consecuencias

- **Cobertura de detección sube** para juegos Steam-Cloud-only y plantillas
  `<storeUserId>` que hoy desaparecen. Sin tocar el catálogo (regla de ADR 0009
  intacta): Steam Cloud es una *fuente nueva*, igual que Epic/GOG en ADR 0011.
- **Los tres casos de forma de save** quedan cubiertos: tiny-files sin rechazo
  ni RAM, monolíticos con dedup real por chunk, grandes sin reventar memoria.
- **Migración (chunks)**: las tablas son aditivas; los blobs existentes no se
  tocan (modelo mixto). El `download` debe distinguir archivo-blob de
  archivo-chunked — riesgo de regresión en restore, mitigado con tests E2E de
  ida y vuelta. Clientes viejos siguen recibiendo el mismo tar.zst.
- **Cuota**: con chunking, `storage_used_bytes` pasa a contar bytes de blobs
  **y** chunks únicos. Recálculo idempotente como en ADR 0018 Fase 2.
- **Self-hosted intacto**: la eficiencia de almacenamiento es núcleo
  compartido, no Cloud; nada se mete tras `--features cloud`. El beneficio
  aplica a ambos despliegues. FastCDC añade dependencia (`fastcdc`) al server y
  cliente, sin C deps.
- **Coste de complejidad**: el modo empaquetado y el chunked añaden caminos en
  `create`/`download`. Mitigado manteniendo el por-archivo como default y los
  modos nuevos detrás de umbrales medidos, no de flags de usuario.

## Alternativas descartadas

- **Chunkear todo, no sólo lo grande.** Rechazado: para saves de archivos
  pequeños el chunking es sobredimensionado (ADR 0018 ya lo argumentó). El
  umbral por tamaño mantiene el coste donde paga.
- **Subir un único tar.zst siempre (también el caso normal).** Rechazado:
  perdería el dedup por-archivo del caso típico (un archivo cambió de N), que
  es justo lo que ADR 0018 optimizó. El empaquetado es transporte, no
  almacenamiento, y sólo para muchos-archivos.
- **Excluir/ignorar archivos enormes.** Rechazado: el save monolítico *es* la
  partida (RPGs con una DB única); no es opcional.
- **Delta binario (bsdiff) en vez de FastCDC.** Rechazado para v1: el delta
  necesita una versión base de referencia y complica la poda GFS (¿qué pasa al
  podar la base?). FastCDC es stateless por chunk y encaja con el refcount
  existente.
- **Subir el cap de archivos sin empaquetar.** Rechazado: no resuelve la RAM ni
  los handles ni las N filas/round-trips; sólo mueve el dolor.

## Plan de implementación

Ver `docs/plans/` (a redactar tras luz verde). Fases sugeridas, independientes
y desplegables por separado, en orden de ROI/riesgo:

- **Fase 1 (quick wins, sin esquema):** streaming en `upload_directory` +
  `restore.rs`; Steam Cloud + `<storeUserId>`; fix literal + comentarios;
  validación por contenido en `refine_save_dir`. Riesgo bajo.
- **Fase 2:** modo empaquetado (campo `pack`, desempaquetado server, cap alto).
  Riesgo medio (nuevo camino en `create`).
- **Fase 3:** skip-by-hash de conjunto (opt-in cliente). Riesgo bajo.
- **Fase 4:** chunking para archivos > umbral (tablas `chunks` /
  `snapshot_file_chunks`, GC, recálculo de cuota, `download` mixto, tests E2E).
  Riesgo alto (esquema + restore).

## Actualización tras implementación (2026-05-31)

Lo implementado coincide con el diseño salvo tres concreciones tomadas durante
el trabajo, todas a favor de menos superficie:

- **Chunking server-side, no cliente.** El borrador (Decisión 1 §3) decía "el
  cliente lo parte con FastCDC". Se implementó al revés: el cliente sube el
  archivo entero por streaming (`backup.rs`, sin techo de RAM gracias a la
  Fase 1) y el server lo trocea en `create`. Motivo: no duplicar el CDC en Rust
  cliente + server, no tocar el protocolo multipart, y mantener el dedup donde
  ya vive la cuota. El cliente nunca conoce los chunks; sólo el server.
- **CDC propio, sin `fastcdc`.** `crates/hoard-server/src/chunking.rs` implementa
  un gear-hash rolling (tabla `GEAR` de 256 u64 generada en `const fn` con
  splitmix64) con cortes en `MIN=1 MiB / avg≈2 MiB / MAX=4 MiB`. Boundaries
  deterministas y estables entre builds, así que un chunk dedup-ea contra el
  mismo contenido de otra versión. Sin dependencia externa ni C deps.
- **Skip-by-hash con firma barata.** Decisión 2 hablaba de `{(rel_path,
  sha256)}`. Se usó `(rel_path, size, mtime)` (`backup.rs::compute_set_signature`)
  para **no releer** los bytes del directorio en cada tick: el SHA por archivo
  ya lo calcula el server al subir; releerlo en cliente sólo para el skip
  duplicaría IO. Trade-off documentado: no detecta una reescritura que conserve
  tamaño *y* mtime cambiando bytes (raro en saves). El agente cachea la firma
  en memoria (`SaveSlot.last_set_hash`); CLI/desktop la persisten en
  `state.json` (`SaveState.set_hash`, `#[serde(default)]`).

El umbral de chunking vive en `chunking::CHUNK_THRESHOLD` (128 MiB) y el de
empaquetado en `backup.rs::PACK_THRESHOLD` (500 archivos). El modelo mixto
(blob entero ≤ umbral, chunked > umbral) hace que un mismo contenido nunca
viva en ambos stores: el umbral es función del tamaño, que es función del
contenido. GC, refcount y cuota tratan chunks y blobs de forma uniforme
(`cleanup.rs::purge_trash`, `blobs.rs` recálculo).
