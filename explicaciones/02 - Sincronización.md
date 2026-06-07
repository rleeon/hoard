# Sincronización

> Cómo viaja una partida de tu PC al servidor y de vuelta a otro dispositivo.
> Dos backends conviven: **self-hosted** (tu `hoard-server`) y **Hoard Cloud**
> (`api.hoard.services`). El cliente elige según el `mode` de `/v1/health`.

```
  [Juego escribe save]
          │  (watcher detecta cambio)
          ▼
  schedule_backup ── debounce ──► run_backup_with_retry
          │
          ▼
  upload_directory_checked  ── ¿cambió de verdad? ──► sube snapshot
          │                                              │
          ▼                                              ▼
   (skip si nada cambió)                          servidor guarda
                                                        │
                                            otro dispositivo:
                                            download_snapshot ──► extrae
```

---

## SUBIDA

### 1. Disparo: watcher → debounce → backup

El watcher (`hoard-watcher`, basado en `notify`) ve que un fichero del save
cambió. No sube al instante: `schedule_backup` **debouncea** (espera a que el
juego deje de escribir) y luego llama a `run_backup_with_retry`.

`run_backup_with_retry` tiene reintentos con **backoff exponencial**
(`2^intento`, tope 5 min). Antes de subir hace un pre-check de carpeta vacía:
si el save está vacío, en vez de subir basura puede disparar un **auto-restore**
o devolver `BackupSkippedEmpty`.

También aquí se registra la **correlación** (proceso↔escritura) que alimenta la
detección (ver [[01 - Detección de saves]], fase 3).

### 2. El filtro skip-by-hash (`upload_directory_checked`)

Esta es la clave para no saturar la red. La firma persistida es
`"<barata>:<contenido>"`:

1. **Firma barata** = `(path, size, mtime)` de cada fichero
   (`compute_set_signature`). Si coincide con la anterior → `Skipped`: ni se
   lee un byte ni se toca la red. Es el caso común.
2. **La barata cambió** (normalmente sólo un bump de `mtime` porque el juego
   reescribe en un timer) → se leen los bytes una vez y se calcula la **firma de
   contenido** (`compute_content_signature`, SHA). Si el contenido es el mismo →
   `Unchanged`: no sube, pero refresca la firma para que el siguiente ciclo
   vuelva al camino rápido.
3. **Los bytes se movieron de verdad** → sube y devuelve `Uploaded`.

```
firma barata igual?  ──sí──►  Skipped   (0 lecturas, 0 red)
        │ no
firma contenido igual? ──sí──► Unchanged (1 lectura, 0 red)
        │ no
        └──────────────────►  Uploaded  (sube snapshot)
```

### 3. El empaquetado, según backend (`upload_directory`)

Se camina el directorio (`walk_source`) y según el destino:

**Self-hosted** — dos modos según número de ficheros (`PACK_THRESHOLD = 500`):
- ≤ 500 ficheros → **multipart** directo al servidor.
- > 500 ficheros → se empaqueta para no hacer miles de partes.

El servidor es content-addressed: deduplica por SHA de fichero entero + chunking
content-defined server-side, y guarda en blobs/chunks con refcount + GC (ADR
0018/0019). Lo que el cliente sube como snapshot, el servidor lo descompone y
sólo persiste lo que no tenía ya.

**Hoard Cloud** (`upload_directory_cloud`) — flujo de 3 pasos contra R2:
1. **init**: avisa al backend, que devuelve una URL **presigned**.
2. **PUT**: empaqueta todo en un `tar.zst` y lo sube directo a R2 con esa URL.
3. **commit**: confirma; aquí se registra el `sha256` del archivo entero (se usa
   luego para verificar la descarga).

---

## BAJADA

### `download_snapshot` (self-hosted)

1. `resolve_version` decide qué versión bajar (la última o una concreta).
2. Stream del `tar.zst` → `ZstdDecoder` (descomprime al vuelo, sin temp file).
3. **Verificación por fichero**: a medida que extrae, calcula el `SHA-256` de
   cada fichero y lo compara con el manifest del snapshot. Si uno no cuadra,
   aborta (`sha256 mismatch for <key>`).
4. `sanitize(path)` antes de escribir: rechaza rutas con `..`, absolutas o
   fuera del destino (anti path-traversal).

### `download_snapshot_cloud` (Hoard Cloud)

Hoard Cloud no expone manifest por-fichero, así que la verificación es distinta:
1. presigned **R2 GET** → descarga el `tar.zst` a un **fichero temporal**.
2. Verifica el **`sha256` del archivo entero** contra el que se registró en el
   commit. Si no cuadra, aborta sin extraer nada.
3. Sólo entonces: `ZstdDecoder` sobre el temp → extrae con el mismo `sanitize`.

> Diferencia clave: self-hosted verifica fichero-a-fichero mientras extrae;
> cloud verifica el archivo completo antes de tocar disco.

---

## Restauración consciente de conflictos (ADR 0014)

Cuando hay que escribir un save bajado encima de uno local, no se pisa a lo
bruto. `run_auto_restore` decide por **mtime** (con tolerancia de 1 s):

- Si el **local es más nuevo** (`local_mtime_wins`) → no se restaura, gana lo
  local (probablemente jugaste aquí más recientemente).
- Si gana el remoto → antes de sobrescribir, lo local se **respalda** en
  `<conflict_root>/<save_id>/<timestamp>/<rel>`. Nada se pierde: queda copia
  estilo "keep-both".

Los respaldos se limpian solos (`cleanup_old_conflicts`, retención 14 días).

Como los saves son binarios, **no hay merge a 3 vías**: un conflicto se resuelve
quedándote ambas copias, no fusionando.

---

## El poller de Hoard Cloud (`cloud_pull.rs`)

`CloudPullScheduler` sondea `/v1/cloud/sync` cada **10 s**. Es **sólo para la
UI**: refresca el estado (qué hay nuevo en la nube) pero **nunca sobrescribe
ficheros locales** por su cuenta. Si el token caduca (401), lo refresca y
reintenta. La restauración real sigue pasando por el flujo consciente de
conflictos de arriba.

---

## Resumen mental

> Subir: el watcher dispara, se debouncea, y `upload_directory_checked` decide
> en tres niveles (firma barata → firma contenido → subir) para no mover bytes
> en balde. Self-hosted dedup server-side; cloud empaqueta tar.zst a R2.
> Bajar: stream tar.zst + verificación SHA (por-fichero en self-hosted, archivo
> entero en cloud) + sanitize anti-traversal. Y nunca se pisa un save local más
> nuevo sin respaldarlo antes.
