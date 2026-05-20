# 0013 — Restore por diff y auto-backup periódico en Modo Automático

## Status

Accepted, 2026-05-20. Lands in 1.5.4. Extends ADR
[0012](0012-ux-polish-and-automatic-mode.md) — el Modo Automático
persistido y el scheduler que aquel ADR introdujo siguen siendo la
base; este ADR cierra las grietas que el uso real expuso después de
1.5.3. No supersede ningún ADR previo; el pipeline de detección
(0009-0011) y el envelope `AppError` (0012) quedan intactos.

## Context

El ciclo 1.5.3 entregó el toggle persistido de Modo Automático, su
scheduler de fondo y la cascada `automatic_mode=true ⇒
auto_restore=true`. Tras una semana de uso el user reportó cuatro
grietas en el comportamiento end-to-end, recopiladas en
[`docs/plans/1.5.4.md`](../plans/1.5.4.md) §0 y verificadas en el recon
inicial del 2026-05-20 ([`docs/plans/1.5.4-log.md`](../plans/1.5.4-log.md)):

1. **El scheduler emite tick sin upload.**
   [`crates/hoard-desktop/src/commands/automatic.rs`](../../crates/hoard-desktop/src/commands/automatic.rs)
   líneas 55-86 dispara el evento Tauri `automatic-tick` cada N horas.
   El listener
   [`crates/hoard-desktop/ui/src/lib/stores/automatic.ts`](../../crates/hoard-desktop/ui/src/lib/stores/automatic.ts)
   líneas 143-161 reacciona ejecutando `runAutomaticSetup()`, que
   internamente solo hace scan-library + add_game_to_tracking +
   boot_agent. No fuerza backup de ningún save ya tracked. Si el
   watcher de `hoard-watcher` perdió un evento (boot frío, FS
   resincronizado en frío, save modificado mientras Hoard no corría),
   el snapshot remoto queda obsoleto indefinidamente — no hay
   catch-up.

2. **Auto-restore es destructivo o nada.**
   [`crates/hoard-agent/src/agent.rs`](../../crates/hoard-agent/src/agent.rs)
   líneas 549-587, 671-708 y 719-750 gatean el restore automático
   por `is_path_empty_or_missing(&local_path)`: si la carpeta de save
   local existe y tiene contenido, el flow no hace nada; si está
   vacía o no existe, descarga el tarball completo del último
   snapshot y lo escribe con `force=true`. No hay punto medio. El
   user describe el modelo mental que quiere: "si me falta algo del
   snapshot remoto, tráelo, pero no toques lo que ya tengo". El
   gate binario actual no expresa esa semántica.

3. **`set_automatic_mode` no refresca el store TS.**
   [`crates/hoard-desktop/src/commands/prefs.rs:120`](../../crates/hoard-desktop/src/commands/prefs.rs#L120)
   persiste el nuevo valor, aplica la cascada `auto_restore=true`,
   empuja el cambio al `AgentHandle` y retorna el `Prefs` actualizado
   al frontend. Sin embargo
   [`App.svelte:toggleAutomatic`](../../crates/hoard-desktop/ui/src/App.svelte)
   solo actualiza la variable local `automaticMode`: el store global
   `prefs` (al que `Settings.svelte` está suscrito) nunca recibe el
   `Prefs` retornado. Resultado: el user activa Modo Automático
   desde la sidebar, abre Settings y ve el toggle "Restauración
   automática" todavía en OFF, aunque `prefs.json` en disco ya lo
   tiene en `true`. La UI miente respecto al estado persistido.

4. **`dashboard.back_up = "Copiar"` en `es.json`.** Las otras siete
   locales usan el verbo correcto direccional ("Back up",
   "Sichern", "Sauvegarder", "Backup", "バックアップ", "Salvar",
   "备份"). Solo el español usa "Copiar", que es ambiguo entre
   portapapeles (`logs.copy`) y upload-to-cloud. Confunde porque el
   user español no tiene forma de saber a primera vista en qué
   dirección viaja el dato.

Las cuatro grietas comparten un patrón: el motor del Modo Automático
es estructuralmente correcto (scheduler corre, persistencia funciona,
restore baja un tarball, i18n existe) pero el contrato hacia el user
no se cumple porque falta el último tramo de cada flujo
(catch-up backup, semántica no destructiva, propagación reactiva,
verbo direccional).

## Decision

Cuatro piezas. Ninguna requiere ADR nuevo después: las cuatro caen
dentro del perímetro del Modo Automático ya descrito en 0012, y este
ADR solo refina semánticas.

1. **Auto-restore se vuelve diff no destructivo.** La condición
   binaria `is_path_empty_or_missing(&local_path)` se sustituye por
   `local_needs_restore(local_path, remote_snapshot)`, que retorna
   `true` si el snapshot remoto contiene **al menos un archivo cuyo
   path relativo no exista localmente**. El flow asociado
   (`run_auto_restore`) cambia de "descargar tarball completo con
   `force=true`" a "descargar tarball remoto a tempdir, walkar su
   contenido y copiar al `local_path` solo los archivos que no
   existan en local". Si un archivo existe local y remoto con bytes
   distintos, **no se toca el local** — el contrato es "ganan los
   bytes locales cuando difieren del remoto". Si local tiene
   archivos extra que no están en remoto, se dejan también (el
   watcher los subirá cuando detecte el cambio). El logging emite
   una línea `tracing::info!` con `restored=N skipped_existing=M`
   por save.

2. **`automatic-tick` añade paso "backup-stale".** Tras el actual
   scan + add_game_to_tracking + boot_agent, el handler del evento
   en `automatic.ts` recorre `listTrackedSaves()` y solicita backup
   explícito de cada uno via un nuevo wrapper
   `api.requestBackup(save.id)` que invoca `trigger_backup_for_save`
   en el agente. El agente ya dedupea backups concurrentes para el
   mismo save (lock en `AgentHandle`), así que el catch-up es
   idempotente respecto al watcher: si el watcher ya disparó el
   backup, la solicitud del tick se descarta; si el watcher perdió
   el evento, el tick lo recupera. Los errores se capturan por save
   y se loguean con `console.warn`, no abortan el tick.

3. **Reactividad de `toggleAutomatic` propaga el `Prefs` retornado al
   store global.** `App.svelte:toggleAutomatic` cambia de
   `await api.setAutomaticMode(next); automaticMode = next;` a
   `const updated = await api.setAutomaticMode(next); prefs.set(updated);
   automaticMode = updated.automatic_mode;`. `Settings.svelte`,
   suscrita a `$prefs`, ve el cambio sin reload. El comando
   `set_automatic_mode` sigue siendo single-source de verdad: el
   frontend confía en el `Prefs` que retorna, no consulta de nuevo.

4. **Rename `dashboard.back_up` a "Subir" solo en `es.json`.** Las
   otras siete locales ya usan el verbo direccional correcto y no
   se tocan. Se audita el resto de `es.json` por usos ambiguos de
   "copiar"/"copia" en contexto de save (no portapapeles) y se
   corrigen los call sites que aparezcan.

## Consequences

- **Mayor IO en cada tick del scheduler.** El paso "backup-stale"
  recorre todos los `tracked_saves` y solicita verificación de cada
  uno. Aceptable porque el tick default es de 6 horas y el agente
  dedupea, pero el costo crece linealmente con el tamaño de la
  biblioteca. Si en el futuro la biblioteca media excede ~200
  saves trackeados, conviene paginar o batchear; con el tamaño
  actual de uso (≤ 50 saves) es ruido.
- **El contrato "ganan los bytes locales si difieren" queda
  documentado y es la regla a la que se atiene auto-restore.** Esto
  significa que el user nunca pierde trabajo por sincronización
  automática, pero también que dos máquinas que diverjan
  silenciosamente acabarán con bytes distintos y ninguna las
  reconcilia automáticamente. Ese caso queda fuera del Modo
  Automático y se cubre con un futuro botón explícito
  "Reemplazar local con remoto" (no scope de 1.5.4).
- **El doc en `/home/insider/Desktop/hoard-modo-automatico.md`
  explica el contrato al user en lenguaje plano.** Cualquier
  desviación que el user observe respecto a ese doc es un bug
  reproducible; el doc se vuelve la spec funcional desde el lado
  del user.
- **`set_automatic_mode` sigue siendo single-source.** No se
  introduce un evento Tauri `prefs-changed` ni un canal de
  notificación async — el comando ya retorna el `Prefs` actualizado
  y eso basta. Si en un ciclo futuro aparecen writers async de
  `Prefs` (scheduler que persiste cambios sin frontend), entonces
  sí valdrá la pena el evento; hoy sería overkill.
- **Una sola locale tocada.** Renombrar `dashboard.back_up` en los
  ocho idiomas para "limpieza" sería innecesario porque siete ya
  están correctas; el riesgo de tocar inglés/alemán/etc. sin
  contexto cultural supera el beneficio cosmético.
- **El gate `local_needs_restore` requiere listar el snapshot
  remoto antes de decidir.** El check actual era barato
  (`fs::read_dir`); el nuevo requiere bajar el manifest del
  snapshot (o el tarball completo si no hay endpoint listing). Si
  el server expone `GET /v1/saves/:id/snapshots/:rev/manifest`, se
  usa eso; si no, el agente descarga el tarball, walka su entry
  list sin descomprimir y decide. La latencia por save sube de
  ms a centenas de ms; con el dedupe del agente y tick de 6 h,
  irrelevante.

## Alternatives considered and why not

- **Merge bidireccional con resolución de conflictos.** Rechazado.
  Los saves son binarios opacos para Hoard (Unity, Unreal, formatos
  propietarios). No hay forma genérica de fusionar dos versiones
  divergentes sin riesgo de corromper el archivo. Cualquier
  estrategia "más nuevo gana" abre la puerta a perder trabajo si el
  reloj de una máquina está desfasado. La política unidireccional
  "local manda cuando difiere" es la única segura sin conocimiento
  del formato.
- **Auto-restore total con `force=true` cuando hay snapshot
  remoto.** Rechazado. Es exactamente el comportamiento que el
  user quiere evitar: jugar offline, editar saves localmente, abrir
  Hoard y perder horas porque el snapshot remoto era anterior y
  sobreescribió todo. El gate binario actual lo mitigaba
  parcialmente (solo restauraba si vacío), pero a costa de no
  restaurar nunca el caso "borré una carpeta por accidente y quiero
  los archivos de vuelta". El diff no destructivo cubre ambos casos
  sin destruir trabajo.
- **Emit evento Tauri `prefs-changed` desde Rust en cada escritura.**
  Rechazado para este ciclo. El único writer hoy es
  `set_automatic_mode` y ya retorna el `Prefs` al frontend. El
  evento añadiría una segunda fuente de verdad (¿confía el
  frontend en el retorno o en el evento?) sin beneficio neto.
  Cuando aparezca un writer async (scheduler que modifique
  `Prefs` por sí solo) será el momento de añadirlo; hoy es
  ingeniería especulativa.
- **Rename `dashboard.back_up` en las ocho locales.** Rechazado.
  Las otras siete ya usan el verbo direccional correcto en su
  idioma. Tocarlas sin contexto nativo de cada locale (alemán,
  japonés, chino sobre todo) introduce más riesgo de regresión
  i18n que ganancia. El usuario es nativo español; el problema es
  exclusivo de `es.json`.
- **Auto-backup en cada tick aunque el watcher esté corriendo.**
  Sin dedupe sería un waste de IO; con dedupe (que ya existe en
  `AgentHandle`) es idempotente. Se eligió la versión con dedupe
  implícito porque el agent ya lo provee y reimplementarlo en el
  scheduler sería redundancia.
- **Hacer el diff comparando hashes en lugar de existencia de
  paths.** Más caro (hay que hashear local y comparar con manifest
  remoto), y no cambia el resultado porque la política es "no
  tocar local si difiere". El check por existencia de path es
  suficiente para decidir si descargar o no, y la política de no
  sobreescritura previene cualquier daño por colisión.
