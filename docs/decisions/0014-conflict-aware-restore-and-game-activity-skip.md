# 0014 — Conflict-aware restore + skip mientras juegas

- **Status**: Accepted
- **Date**: 2026-05-20
- **Supersedes**: parts of ADR 0013 (restore-by-diff "local always wins")
- **Context**: 1.5.5

## Contexto

ADR 0013 introdujo restore-by-diff no destructivo con regla "local
siempre gana" en caso de conflicto. Tras probar 1.5.4 multi-dispositivo
el user reporta que la regla pierde data:

1. Usuario juega en máquina A → save bueno se sube.
2. Usuario abre el juego en máquina B sin sync previo. El juego
   escribe un save inicial vacío al ejecutar.
3. Tick automático en B descarga snapshot de A, compara byte-a-byte
   con el local vacío → conflict → conserva local. El save bueno se
   pierde porque el watcher lo sustituye en el próximo upload.

Decisiones de esta ADR:

## Decisión 1 — Mtime decide en conflicto + backup del perdedor local

`restore_files_into` ya no asume "local wins". Ahora:

- Bytes iguales → skip (sin cambios).
- Bytes distintos:
  - `local_mtime > remote_mtime + 1s` → local gana, sin tocar. El
    watcher lo subirá como nueva versión.
  - `remote_mtime > local_mtime` (o iguales con tolerancia 1s) →
    remoto gana. **Antes** de sobrescribir local, se mueve a
    `<state_dir>/conflicts/<save_id>/<rfc3339_ts>/<rel_path>`.

Razón: mtime es imperfecto (los relojes pueden estar mal) pero es el
único señal disponible sin hashes en el manifest. El backup pre-
sobrescritura garantiza que el user nunca pierde data: en el peor caso
puede recuperar manualmente desde la carpeta de conflictos. La
tolerancia de 1s evita flap por filesystems de baja resolución
(FAT32 redondea a 2s).

## Decisión 2 — `.hoard-conflicts/` con TTL 14 días, configurable

La carpeta `<state_dir>/conflicts/` se limpia al inicio de cada tick
automático: entries con `mtime` más viejos que `conflict_retention_days`
(default 14, configurable en Settings) se borran.

Razón: dar al user tiempo razonable para notar y recuperar manualmente,
sin acumular disco indefinidamente.

## Decisión 3 — Skip auto-restore si juego corriendo o save tocado <5min

`sweep_for_auto_restore` añade dos guards antes de spawn:

- Si `slot.is_running == true` (el process_poll detectó proceso match)
  → skip ese save.
- Si `slot.is_running == false` y `slot.processes.is_empty()` (sin
  match disponible) → comprobar mtime del `local_path`. Si <5min,
  skip por precaución.

Razón: el usuario no quiere que Modo Automático escriba sobre archivos
mientras el juego los tiene abiertos. La primera guard cubre el caso
con `processes` populated; la segunda es un fallback heurístico para
saves sin process match en el catálogo.

## Decisión 4 — Tick inmediato al activar Modo Automático

`AutomaticScheduler::start` ahora emite el primer `automatic-tick`
inmediatamente. Hasta 1.5.4 el primer tick se consumía en silencio
para evitar fire instantáneo al cargar — ahora se considera deseado:
si el user activa Modo Automático, quiere ver acción ya, no esperar
6h.

Razón: feedback directo del user "que todo pase en el momento no
esperar horas". El coste (un fetch + diff por save al toggle) es
aceptable y el tick programado siguiente sigue siendo a `interval_hours`
de distancia.

## Consecuencias

- Multi-dispositivo deja de perder data silenciosamente.
- Modo Automático se siente reactivo desde el primer segundo.
- Mientras juegas el sistema queda quieto; al cerrar el juego dispara
  backup vía `GameStopped` ya existente (ver
  [agent.rs:1269](../../crates/hoard-agent/src/agent.rs#L1269)).
- `conflicts/` puede crecer si el user tiene mtimes desincronizados
  permanentemente — la limpieza por TTL acota.
- Settings gana sliders para `automatic_scan_interval_hours` y
  `conflict_retention_days` (P-D155-5).
