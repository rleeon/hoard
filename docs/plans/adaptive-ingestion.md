# Plan — Ingesta adaptativa por forma del save + detección de Steam Cloud

> Implementa [ADR 0019](../decisions/0019-adaptive-ingestion-and-steam-cloud-detection.md).
> Extiende —no supersede— ADRs 0009/0010/0011 (detección) y 0018
> (almacenamiento). Sin número de versión fijo (la numeración 1.x se
> descarriló — ver `1.5.md`). Se asigna al arrancar.

Origen: auditoría 2026-05-31. El pipeline asume **una sola forma de save**.
ADR 0018 cubrió "muchas versiones casi idénticas" (OpenTTD). Quedan tres
formas de save sin cubrir y dos grietas de detección.

Cuatro fases independientes y desplegables por separado, en orden de
ROI/riesgo. Decisión de implementación clave frente al ADR: el **chunking se
hace en el server** en v1 (no en cliente como insinuaba el borrador del ADR),
para no tocar el protocolo de subida ni duplicar el CDC en dos lenguajes. El
ADR se reconcilia con esto en su sección de cierre.

---

## Estado actual (puntos de código)

- **Subida cliente**: `hoard-agent/src/backup.rs::upload_directory` (L79) lee
  cada archivo entero a RAM (`tokio::fs::read`, L106) y los acumula como
  `Part`s de un único `multipart::Form` campo `files`. Snapshot entero en RAM.
- **Restore cliente**: `hoard-agent/src/restore.rs::download_snapshot` (L55)
  hace `read_to_end` (L139) de cada entry del tar a un `Vec`. Archivo grande →
  RAM.
- **Creación server**: `hoard-server/src/routes/snapshots.rs::create` (L106).
  `MAX_FILES_PER_SNAPSHOT = 1000` (L130) rechaza duro. Streama cada field a
  tmp + hashea SHA-256; coloca blob por `rename` (L374); refcount
  `INSERT … ON CONFLICT DO UPDATE refcount+1` (L344). Cuota sólo bytes nuevos.
- **Descarga server**: `snapshots.rs::download` (L619) reconstruye tar.zst
  desde blobs (`append_path_with_name`, L715, vía `ChannelWriter`).
- **Blobs**: `hoard-server/src/blobs.rs::blob_path` (L16) =
  `blobs/<user>/<sha[0:2]>/<sha>`. Patrón runtime `sqlx::query()` (no macro).
- **Detección**: `hoard-agent/src/detection.rs::detect_all` (L155),
  `refine_save_dir` (L798, sin validación por contenido), `merge_fs_hit` (L746).
  `steam::steam_user_dirs` (L359) existe pero nadie lo llama.
  `pathexpand.rs:185` descarta `<storeUserId>`/`<gameId>` (`Vec::new()`).
  `pathexpand.rs:32` rompe literales absolutos (`trim_start_matches('/')`).

---

## Fase 1 — Quick wins (sin esquema). Riesgo bajo

### 1a — Streaming en subida (`backup.rs`)
Reescribir `upload_directory` para enviar cada `Part` desde un stream de disco
(`reqwest::Body::wrap_stream` sobre `tokio_util::io::ReaderStream` de un
`tokio::fs::File`) en vez de `fs::read` a memoria. El progreso se mantiene
sumando tamaños conocidos del walk. No toca server ni formato.

### 1b — Streaming en restore (`restore.rs`)
Volcar cada entry del tar a disco por streaming. Verificación SHA-256 sin
buffer completo: hashear mientras se copia (`tokio::io::copy` a un writer que
envuelve el hasher + el fichero, o copy a fichero temporal y re-hash si hace
falta). Mantener el rechazo por mismatch.

### 1c — Steam Cloud + `<storeUserId>` (`detection.rs` + `pathexpand.rs`)
- Nueva etapa en `detect_all` tras el cross-ref Steam, antes del walker: para
  cada `SteamApp` instalado, expandir `userdata/<id>/<appid>/remote/` vía
  `steam::steam_user_dirs`; si existe en disco, `merge_fs_hit` con
  `Confidence::Medium`. Reusa refine + overrides.
- `expand_path` aprende a abanicar `<storeUserId>`/`<gameId>` sobre los dirs de
  usuario Steam descubiertos en vez de `Vec::new()`.

### 1d — Fix literal absoluto + comentarios stale (`pathexpand.rs`)
- L32: una plantilla literal absoluta se devuelve **tal cual** (sin
  `trim_start_matches`). Actualizar test `literal_path_passes_through`.
- Limpiar comentario falso de `<storeUserId>` (L185) y el de `winDocuments`
  (L146, OneDrive ya resuelto antes por `windows_known_folder`).

### 1e — Validación por contenido en `refine_save_dir` (`detection.rs`)
Cuando hay varios subdirs `save*` ambiguos o dispara el walker, puntuar
candidatos con `dir_has_recent_save_file` (extensión save-like + mtime < 90d)
y preferir el que la cumple. Sin tabla hardcodeada nueva.

**Aceptación Fase 1**: subir/restaurar un archivo de 2 GB no dispara la RAM;
un juego Steam-Cloud-only aparece en detección; literal absoluto se preserva.

## Fase 2 — Modo empaquetado (campo `pack`). Riesgo medio

Cliente: cuando `file_count > 500`, subir un único `tar` (sin comprimir) en un
campo multipart `pack` en vez de N campos `files`. Server: detectar `pack`,
desempaquetar a `tmp/` y hashear por-archivo **igual que hoy** (mismo dedup,
mismas filas, mismos blobs). `MAX_FILES_PER_SNAPSHOT` sube a 50 000 sólo para
el modo empaquetado; el por-archivo conserva su cap. No cambia el modelo de
almacenamiento.

**Aceptación**: save de 5 000 archivos de 1 KB sube y restaura; 1 round-trip.

## Fase 3 — Skip-by-hash de conjunto (opt-in cliente). Riesgo bajo

Antes de subir, comparar `{(rel_path, sha256)}` del directorio con el del
último snapshot (cacheado en `state.json` por save). Si idéntico, no-op.
Requiere hashear el set en cliente (barato; ya se va a hashear en server).
Cachear en `SaveState` un campo nuevo con `#[serde(default)]`.

**Aceptación**: re-disparar backup sin cambios reales no crea versión.

## Fase 4 — Chunking server-side (FastCDC in-house). Riesgo alto

Migración `0014_chunks.sql` (runtime `sqlx::query()`, no macro):
```sql
CREATE TABLE chunks (
  user_id    TEXT NOT NULL,
  sha256     TEXT NOT NULL,
  size_bytes INTEGER NOT NULL,
  refcount   INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
  PRIMARY KEY (user_id, sha256)
);
CREATE TABLE snapshot_file_chunks (
  snapshot_file_id TEXT NOT NULL,
  ordinal          INTEGER NOT NULL,
  chunk_sha256     TEXT NOT NULL,
  PRIMARY KEY (snapshot_file_id, ordinal)
);
```
- CDC in-house (gear-hash rolling, ventana ~1–4 MB, sin dep externa).
- `create`: si un archivo de la tmp supera el umbral (128 MB), trocearlo,
  incref de chunks (`INSERT … ON CONFLICT refcount+1`), colocar chunk en
  `chunks/<user>/<sha[0:2]>/<sha>` sólo si no existe, escribir filas
  `snapshot_file_chunks`. `snapshot_files.sha256` sigue siendo el SHA del
  archivo entero. Modelo mixto: archivos pequeños siguen como blob entero.
- `download`: reensamblar el tar leyendo blob o concatenando chunks según el
  archivo.
- GC: `cleanup.rs::purge_trash` decrementa refcount de chunks y borra a 0,
  igual que blobs.
- Cuota: `storage_used_bytes` = bytes de blobs **y** chunks únicos. Recálculo
  idempotente.
- Tests: ida y vuelta de archivo grande que muta parcialmente → assert dedup
  por chunk, refcounts, GC a 0, restore byte-idéntico.

**Aceptación**: archivo monolítico que reescribe pocos KB por versión ocupa
~tamaño-del-delta en `chunks/`, no el archivo entero por versión.

---

## Verificación por fase
`cargo check --workspace` + `pnpm --dir crates/hoard-desktop/ui check` tras
cada fase que toque código. `cargo test -p hoard-server` tras Fase 4. i18n en
los 8 locales si se toca UI (Fase 3 puede añadir un toggle).

## Riesgos / cuidado
- **Restore mixto (Fase 4)**: `download` debe distinguir archivo-blob de
  archivo-chunked. Tests E2E de ida y vuelta obligatorios.
- **Cuota (Fase 4)**: recálculo idempotente y verificable, como ADR 0018.
- **Refcount races**: incref/decref de `chunks` en la misma tx que
  `snapshot_file_chunks`. Nunca GC fuera de tx.
- **Compatibilidad de descarga**: clientes viejos siguen recibiendo el mismo
  tar.zst en todos los modos.
- **Self-hosted intacto**: eficiencia de almacenamiento es núcleo compartido,
  nada tras `--features cloud`.

---

## Resultados (2026-05-31) — todas las fases en `main`

**Fase 1** — `backup.rs` sube cada archivo por streaming desde disco
(`reqwest::Body::wrap_stream` sobre `ReaderStream`); `restore.rs` vuelca cada
entry a disco en bloques de 256 KB hasheando incrementalmente. Detección Steam
Cloud: nueva etapa en `detect_all` que expande `userdata/<id>/<appid>/remote/`
vía `steam_user_dirs` y mergea con `Confidence::Medium`; `expand_path` abanica
`<storeUserId>`/`<gameId>`. Literal absoluto se preserva (`pathexpand.rs:32`),
test `literal_path_passes_through` actualizado. `refine_save_dir` desambigua
con `dir_has_recent_save_file`.

**Fase 2** — Modo empaquetado: cuando `file_count > PACK_THRESHOLD` (500), el
cliente construye un tar por un pipe `tokio::io::duplex` y lo sube como campo
`pack`; el server lo desempaqueta por streaming (`StreamReader` +
`tokio_tar::Archive`) y hashea por-archivo igual que el modo normal.
`MAX_FILES_PACKED = 50 000` sólo para `pack`.

**Fase 3** — Skip-by-hash con firma barata `(rel_path, size, mtime)`
(`compute_set_signature`). `upload_directory_checked` devuelve `Skipped` si la
firma coincide con la previa. Caché: `SaveState.set_hash` (state.json,
`#[serde(default)]`) en CLI/desktop; `SaveSlot.last_set_hash` (memoria) en el
agente, vía un `BackupDone` por el canal de done.

**Fase 4** — Chunking **server-side** (desviación del borrador del ADR, ver su
§cierre). Migración `0014_chunks.sql` (`chunks` + `snapshot_file_chunks`). CDC
propio gear-hash en `crates/hoard-server/src/chunking.rs` (MIN 1 MiB / avg ~2
MiB / MAX 4 MiB, tabla `GEAR` por `const fn` splitmix64). En `create`, un
archivo `> CHUNK_THRESHOLD` (128 MiB) se planifica con `plan_chunks` (sólo
hashea, sin escribir → la cuota se chequea antes de tocar disco) y en la tx se
colocan los chunks ausentes (`place_chunk`, lee el rango del tmp). `download`
reensambla: blob entero o concatenación de chunks (stream de ≤4 MiB por chunk,
`append_data` con header gnu). GC en `purge_trash` decrementa refcount de
chunks y borra a 0, igual que blobs; cuota recalculada = blobs únicos + chunks
únicos.

**Verificación**: `cargo check --workspace` y `cargo check -p hoard-server
--features cloud` limpios; `cargo test -p hoard-server` 15/15 (incluye CDC
determinista, dedup tras edición local, round-trip de `place_chunk`, y GC de
chunks con refcount/cuota); `cargo test -p hoard-agent` 7/7 (incluye las
firmas de set). No se tocó UI, así que i18n queda igual.
