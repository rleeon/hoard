# Plan: Overhaul de detección de juegos y rutas de save

> Fuente única de verdad para todo lo relacionado con detección.
> Si algo en `docs/plans/1.5.md` contradice este documento, **gana este**.
> Si algo no cabe limpio en este documento, abre ADR nuevo y enlázalo aquí.
>
> Hoy: 2026-05-17. Versión `main`: 1.4.6. Este overhaul aterriza en 1.5.0.
> El CAS storage que antes ocupaba 1.5.0 se desplaza a 1.6.0 (ver §8).

---

## 0. Por qué este documento existe

El cliente pinta una lista preciosa de juegos detectados, pero la lógica que la
genera tiene seis grietas que el usuario nota a diario:

1. **Proton/Wine invisible en Linux.** `detect_all()` sólo expande
   `entry.paths.linux`. Los miles de juegos Windows-only que el usuario corre
   bajo Proton tienen sus saves en
   `~/.steam/steam/steamapps/compatdata/<appid>/pfx/drive_c/...`. Hoy esos
   juegos quedan fuera del informe salvo que tengan `paths.linux` separado.
2. **Refinamiento de "game root → save subdir" hardcoded a un juego.**
   [`crates/hoard-agent/src/detection.rs:118`](../../crates/hoard-agent/src/detection.rs#L118)
   sólo lista `stellaris`. Toda la familia Paradox (CK3, EU4, HoI4, Imperator,
   Victoria 3) y cualquier otro juego cuyo template Ludusavi apunte a la
   carpeta raíz tracean root entero (mods + config + saves indiscriminado), lo
   que dispara backups cada vez que el juego escribe un fichero de telemetría.
3. **Cross-reference Steam ↔ catálogo sólo por `steam_app_id`.** Si una
   entrada Ludusavi no trae appid, el juego sólo aparece por filesystem y
   únicamente si ya existen saves en disco. Steam-installados sin save todavía
   creado quedan fuera de la lista aunque Steam ya los conozca.
4. **`expand_path` rompe templates literales absolutos.**
   [`pathexpand.rs:33`](../../crates/hoard-agent/src/pathexpand.rs#L33) hace
   `trim_start_matches('/')` y devuelve un `PathBuf` relativo. El test
   `literal_path_passes_through` afirma ese bug como esperado.
5. **Dos sistemas de placeholders coexisten** (`{APPDATA}` en TOML
   hand-curated, `<winAppData>` en Ludusavi) pero la ruta caliente
   (`detection.rs`) ignora el catálogo TOML por completo. Lo mismo en
   `crates/hoard-agent/src/autodetect.rs` y `crates/hoard-detect/` — código
   muerto de v0.3 que sigue compilando y confunde a cualquiera que aterrice.
6. **No hay overrides persistentes del usuario.** Cuando Hoard adivina mal y
   el usuario corrige con el picker, esa corrección no se guarda como
   override del catálogo. Cada re-scan vuelve a sugerir mal.

Hay además un séptimo problema operativo: cuando la detección hace algo raro,
no hay forma de ver por qué. Ningún `trace`, ningún panel de diagnóstico.

---

## 1. Objetivos no-negociables

- **Cobertura Proton/Wine en Linux**. Cualquier juego Windows-only Steam
  jugado bajo Proton aparece con su save path correcto.
- **Cero "tracear el root del juego por error"**. Si el template apunta a una
  carpeta que mezcla saves con config/mods, Hoard o lo refina a la subcarpeta
  de saves o no propone path y deja que el usuario lo elija.
- **El usuario nunca corrige dos veces el mismo error de detección.** Si
  pica "Pick folder" y elige una ruta, esa elección persiste y los re-scan
  futuros la respetan.
- **Detección observable**. Hay un comando que dice por qué un juego está o
  no está en la lista, qué templates se expandieron, qué paths se descartaron
  y por qué.
- **El catálogo TOML hand-curated se elimina o se unifica con Ludusavi**.
  Mantener dos sistemas paralelos cuando uno está muerto es deuda.

## 2. No-goals

- Cifrado client-side. No toca.
- Cambiar el formato de snapshot (CAS). Se mueve a 1.6.0; ver §8.
- Detección de Lutris/Bottles standalone (fuera de Steam). Posible 1.6.x si
  alguien lo pide.
- Soportar saves en el registro de Windows. Sigue en 1.8.0 (P1.8.0-c).

---

## 3. Cómo tiene que ser la app — visión

Cuando un usuario instala Hoard por primera vez en una máquina con juegos ya
presentes, **el escaneo inicial encuentra todo lo que se puede encontrar
sin preguntar nada**. La lista que aparece en Library cumple estas
propiedades:

1. **Un juego = una fila**. Si Hoard sabe que el juego existe pero no sabe
   dónde guarda, la fila aparece con alerta amber "elige carpeta". Si sabe
   dónde guarda, la fila aparece con el path y la opción de empezar a
   trackear con un clic.
2. **El path propuesto es siempre el directorio de saves**, no el directorio
   de instalación, no la raíz del juego, no la carpeta de mods. Si Hoard no
   puede distinguir, la fila se trata como "elige carpeta" y nunca propone
   un path arriesgado.
3. **Proton/Wine es transparente.** Un Stardew Valley jugado con Proton en
   Linux se detecta igual de bien que un Stardew Valley nativo Linux.
4. **Lo que el usuario decide manualmente, manda.** Si el usuario pica el
   picker y elige `/home/x/SaveX`, Hoard guarda ese override y nunca más lo
   sugiere ni lo "corrige" en un re-scan.
5. **El re-scan no rompe nada.** Re-escanear nunca pierde overrides ni
   marca como "no encontrado" un juego que el usuario sigue jugando.
6. **Detección es debuggable.** El usuario power-user (o el dev) puede pedir
   "explícame por qué este juego no aparece" y recibe una respuesta concreta:
   "expandí estos templates, ninguno existe en disco, el appid no estaba en
   tu Steam library".
7. **El catálogo se actualiza.** Cuando Ludusavi publica nuevos juegos o
   corrige paths, el cliente puede traerse el delta sin esperar a una nueva
   release. (Esto ya existe en `ludusavi::save_runtime_override` — sólo hay
   que verificar que se ejerce desde un botón en Settings.)

Cualquier desviación de estas siete propiedades es un bug.

---

## 4. Arquitectura objetivo

### 4.1 Pipeline de detección (post-overhaul)

```
                   ┌─────────────────────────────┐
                   │ ludusavi::catalog()         │
                   │   ~20k entries              │
                   └──────────────┬──────────────┘
                                  │
        ┌─────────────────────────┼──────────────────────────┐
        │                         │                          │
        ▼                         ▼                          ▼
┌──────────────┐         ┌────────────────┐         ┌────────────────┐
│ steam scan   │         │ filesystem     │         │ proton scan    │
│  appmanifest │         │  heuristic     │         │  compatdata    │
│  → SteamApp  │         │  (linux/mac/   │         │  → expand      │
│              │         │   windows      │         │     windows    │
│              │         │   templates)   │         │     templates  │
│              │         │                │         │     against    │
│              │         │                │         │     prefix     │
└──────┬───────┘         └────────┬───────┘         └────────┬───────┘
       │                          │                          │
       └──────────────────────────┴──────────────────────────┘
                                  │
                                  ▼
                        ┌─────────────────────┐
                        │ merge by slug:      │
                        │  steam+fs ⇒ Both,   │
                        │  steam ⇒ steam,     │
                        │  fs ⇒ fs            │
                        └──────────┬──────────┘
                                   │
                                   ▼
                        ┌─────────────────────┐
                        │ refine save dir     │
                        │ (general heuristic) │
                        │  path contains save?│
                        │   yes → keep        │
                        │   no  → look for    │
                        │        subdir with  │
                        │        "save"/      │
                        │        "saves"/     │
                        │        "save games" │
                        └──────────┬──────────┘
                                   │
                                   ▼
                        ┌─────────────────────┐
                        │ apply user          │
                        │ overrides:          │
                        │  state.manual_paths │
                        │  > everything       │
                        └──────────┬──────────┘
                                   │
                                   ▼
                        ┌─────────────────────┐
                        │ DetectionReport     │
                        └─────────────────────┘
```

### 4.2 Datos persistidos

- **`state.json`** gana un mapa `manual_paths: HashMap<slug, PathBuf>`. Es la
  única forma persistente de override. Se escribe atómicamente como el resto
  del state.
- **`detection.json`** (ya existe desde 1.4.0) sigue cacheando el
  `DetectionReport` para arranque instantáneo.
- El catálogo embebido y su override en `<cache_dir>/hoard/ludusavi-catalog.json`
  no cambian.

### 4.3 Código a eliminar / unificar

- `crates/hoard-detect/` (todo el crate): muerto desde 1.0, mantiene su
  propio `DetectedGame` que confunde. Eliminar el crate del workspace; mover
  `process.rs` (que sí se usa, vía `hoard-agent`) a `hoard-agent/src/process.rs`
  o dejarlo donde está pero renombrar el crate a `hoard-process` para que el
  nombre no engañe.
- `crates/hoard-agent/src/autodetect.rs`: dead code v0.3 que sigue compilando.
  Verificar que `register_one`/`run_autodetect` no se invocan desde ningún
  sitio (grep en todo el workspace + `crates/hoard-desktop/`) y borrar el
  módulo entero, junto con sus tests.
- `crates/hoard-manifest/src/placeholders.rs` y `data/games/*.toml` (TOML
  hand-curated): el TOML no se consulta desde la ruta caliente actual.
  Decisión: borrar el TOML y los placeholders `{APPDATA}` y dejar **sólo** el
  catálogo Ludusavi y sus placeholders `<winAppData>`. Si se quiere mantener
  un "override hand-curated", se hace inyectando entradas extra en el
  catálogo Ludusavi al cargarlo, no como un sistema paralelo.

### 4.4 Telemetría

Cada paso del pipeline emite `tracing::debug!` o `tracing::trace!` con campos
estructurados (`slug`, `template`, `expanded_path`, `kept|dropped|refined`,
`reason`). Hay un comando Tauri nuevo `detection_diagnostics(slug: String)`
que reproduce el pipeline para un slug y devuelve un JSON con todo lo que
pasó. Lo consume un panel "Diagnóstico" en Settings, oculto detrás de 5
clicks en el número de versión del sidebar (mismo patrón que ya se planeó
para `agent_status` en P1.4.0-0).

---

## 5. Roadmap por sub-release

Todo dentro del ciclo 1.5.0. Cada P-DET-x es un prompt autocontenido que
otro Claude Opus puede ejecutar en una sesión limpia. Orden recomendado:

| Prompt    | Qué hace                                                  | Bloqueante para… |
|-----------|-----------------------------------------------------------|------------------|
| P-DET-0   | ADR 0009-path-detection-overhaul                           | todos los siguientes |
| P-DET-1   | Proton/Wine prefix expand en Linux                         | P-DET-2          |
| P-DET-2   | Heurística general de refinamiento save-dir                | —                |
| P-DET-3   | Fallback Steam→catalog por nombre cuando falta `appid`     | —                |
| P-DET-4   | Limpieza: borrar `autodetect.rs`, `hoard-detect/`, TOML    | —                |
| P-DET-5   | `manual_paths` override persistente + UI picker            | P-DET-6          |
| P-DET-6   | `detection_diagnostics` + panel diagnóstico (5-clicks)     | —                |
| P-DET-7   | Fixtures + tests integration                                | —                |
| P-DET-Z   | Cierre 1.5.0 (bump + CHANGELOG + CLAUDE.md)                 | resto cerrado    |

P-DET-2 y P-DET-3 pueden hacerse en paralelo después de P-DET-1.
P-DET-4 y P-DET-5 también son paralelizables.

---

## 6. Cómo registrar progreso

**Cada cierre de prompt actualiza** [`detection-log.md`](detection-log.md):
una entrada nueva con fecha, qué prompt cerró, qué cambió, qué archivos. El
log es la fuente de verdad para "qué se ha hecho" (no el CHANGELOG, que
sólo recoge el resumen visible al usuario en el bump).

**No bumpees versión en cada prompt.** El bump es atómico en P-DET-Z. Los
prompts intermedios dejan todo staged.

**No mergees a `main`** sin pasar `cargo check --workspace`,
`pnpm --dir crates/hoard-desktop/ui check`, y los tests del crate tocado.

---

## 7. Prompts ejecutables

Copia-pega cada bloque en una sesión nueva de Claude Code apuntando a
`/home/insider/hoard`. Cada uno empieza leyendo este plan y el log.

### P-DET-0 — ADR del overhaul

```
Lee docs/plans/detection.md y docs/plans/detection-log.md íntegros antes de
escribir nada. Tu tarea es redactar el ADR que justifica el overhaul.

Crea docs/decisions/0009-path-detection-overhaul.md siguiendo el formato de
0006-game-detection.md (Status / Context / Decision / Consequences /
Alternatives). Cubre:

- Context: las seis grietas listadas en §0 del plan. Cita archivos y líneas
  concretas (detection.rs:118, pathexpand.rs:33, autodetect.rs).
- Decision: la arquitectura objetivo de §4. Resume el pipeline en cinco
  líneas, no copies el ASCII art. Lista las eliminaciones de §4.3 como
  decisiones explícitas (borrar hoard-detect crate; borrar autodetect.rs;
  borrar catálogo TOML; mantener sólo Ludusavi).
- Consequences:
  - Linux/Proton funciona, lo que captura la mayoría de los usuarios actuales.
  - La eliminación del catálogo TOML rompe nada porque nadie lo consultaba
    desde la ruta caliente.
  - `manual_paths` introduce un punto de divergencia entre lo que el usuario
    elige y lo que el catálogo dice; el override del usuario gana siempre.
  - Telemetría incrementa los logs en INFO/DEBUG; ajusta el default si
    aparece como ruido en producción.
- Alternatives considered:
  - Mantener los dos catálogos: rechazado, deuda permanente sin upside.
  - Detectar Proton via heurística de nombre de proceso wineserver: rechazado,
    da falsos positivos con Lutris/Bottles, y compatdata/<appid> es señal
    mucho más directa.
  - Pedir al usuario que elija manualmente todos los paths la primera vez:
    rechazado, viola la promesa "Hoard funciona en frío" del README.

El ADR debe quedar listo para merge. NO toques código todavía.

Al cerrar: actualiza docs/plans/detection-log.md con una entrada DONE para
P-DET-0 (fecha = hoy, qué quedó, qué archivos tocaste). No bumpees versión.
```

### P-DET-1 — Proton/Wine prefix expand

```
Lee docs/plans/detection.md (especialmente §4.1 y §3) y docs/plans/
detection-log.md. Lee docs/decisions/0009-path-detection-overhaul.md si ya
existe (lo crea P-DET-0; si no, pregúntale al usuario antes de continuar).

Objetivo: cuando OS=Linux, expandir los templates Windows del catálogo
contra cada Proton prefix detectado en steamapps/compatdata. Resultado: un
Stardew Valley jugado con Proton aparece con su save path real en Library.

Cambios concretos:

1. crates/hoard-agent/src/steam.rs:
   - Nueva función pub fn list_proton_prefixes(os: Os) -> Vec<ProtonPrefix>.
     Reusa detect_steam_libraries(os). Para cada lib, lista
     steamapps/compatdata/<appid>/pfx (descarta los que no son directorio).
   - struct ProtonPrefix { app_id: u64, prefix_root: PathBuf } donde
     prefix_root apunta a la carpeta `pfx`. Pública porque la usa
     pathexpand.

2. crates/hoard-agent/src/pathexpand.rs:
   - Nueva variante de expand_path: pub fn expand_path_in_prefix(
       template: &str, prefix: &Path
     ) -> Vec<PathBuf>.
   - Mismo split_placeholder que ya tienes, pero el match de
     expand_placeholder mapea los tokens Windows contra el prefix:
       <winAppData>      -> prefix/drive_c/users/steamuser/AppData/Roaming
       <winLocalAppData> -> prefix/drive_c/users/steamuser/AppData/Local
       <winLocalAppDataLow> -> prefix/drive_c/users/steamuser/AppData/LocalLow
       <winDocuments>    -> prefix/drive_c/users/steamuser/Documents
       <winPublic>       -> prefix/drive_c/users/Public
       <winProgramData>  -> prefix/drive_c/ProgramData
       <winDir>          -> prefix/drive_c/windows
       <home>            -> prefix/drive_c/users/steamuser
       <root>            -> prefix/drive_c
   - Si el template no usa un token windows o no aplica al prefix,
     devolver vec![] (la ruta linux no se expande contra el prefix).

3. crates/hoard-agent/src/detection.rs::detect_all:
   - Después del Steam-cross-reference, si os == Os::Linux, llamar a
     list_proton_prefixes(os). Para cada prefix con app_id presente en el
     catálogo (find_by_steam_app_id), expandir el entry.paths.windows
     contra el prefix y añadir los hits a found_paths del slug
     correspondiente (promoviendo source a Both / confidence a High igual
     que merge_fs_hit).
   - Mantén el orden actual: cross-reference Steam → filesystem nativa →
     proton prefix → refine. (P-DET-2 cambia "refine" pero este prompt
     deja el refine actual intacto.)

4. Tests:
   - En pathexpand.rs: test expands_winappdata_against_prefix con un
     PathBuf sintético /tmp/fake-prefix; verificar que <winAppData>/Game
     resuelve a /tmp/fake-prefix/drive_c/users/steamuser/AppData/Roaming/Game.
   - En steam.rs: test list_proton_prefixes con un steamapps/compatdata/
     sintético bajo tempdir; verificar que detecta los appids correctos
     y descarta los que no tienen pfx/.
   - En detection.rs: extender los tests existentes con una variante
     proton (un compatdata sintético con AppData/Roaming/StardewValley
     creado); verificar que stardew-valley aparece en el reporte con
     source=Both y found_paths apuntando al prefix.

Verificación:
- cargo check --workspace limpio.
- cargo test -p hoard-agent verde.
- (manual, si tienes una máquina Linux con Steam+Proton) hoard scan
  detecta al menos un juego que sólo tiene paths Windows en el catálogo.

NO bumpees versión. Actualiza docs/plans/detection-log.md con la entrada
DONE de P-DET-1 (fecha, archivos tocados, tests añadidos).
```

### P-DET-2 — Heurística general de refinamiento save-dir

```
Lee docs/plans/detection.md §4.1 y §3, propiedad #2. Lee detection-log.md
para confirmar que P-DET-1 ya cerró.

Objetivo: sustituir la lista hardcoded AMBIGUOUS_ROOT_PATHS por una
heurística general que se aplica a TODOS los slugs. La lista se queda
sólo como tabla de overrides explícitos para casos atípicos.

Algoritmo (en pseudocódigo):

  fn refine_save_dir(slug: &str, hits: Vec<PathBuf>) -> Vec<PathBuf>:
    1. Si slug está en SAVE_DIR_OVERRIDES (la lista actual), usar el
       comportamiento actual de refine_ambiguous_root.
    2. Para cada hit:
       a. Si el último segmento del path contiene case-insensitive
          alguna de: "save", "saves", "savegame", "savegames",
          "save games", "save_games" → keep tal cual.
       b. Si no, listar entries del directorio (read_dir, single level).
          Filtrar los que son dir y cuyo nombre contiene case-insensitive
          alguno de los patrones de arriba.
          - Si hay exactamente 1 candidato: ese.
          - Si hay >1: todos.
          - Si hay 0: drop el hit (devolver vec[] para ese hit) y dejar
            que el UI muestre amber "elige carpeta".

Renombra AMBIGUOUS_ROOT_PATHS a SAVE_DIR_OVERRIDES con un comentario
explicando que es para casos atípicos: la regla general está en la
heurística. Stellaris se queda en la lista como override explícito
porque "save games" no contiene "save" pegado a "s" como sustring
de un segmento — wait, sí contiene: "save games".lower().contains("save")
es true. Verifícalo. Si la heurística general ya cubre Stellaris,
borra la entrada de SAVE_DIR_OVERRIDES (queda vacía la lista, eso
está bien).

Tests:
- refine_save_dir_keeps_path_with_save_in_name: un path
  /home/x/.config/StardewValley/Saves debe quedarse tal cual.
- refine_save_dir_finds_subdir_save_games: un tempdir con un subdir
  "save games" debe colapsar al subdir.
- refine_save_dir_finds_subdir_saves: idem con "Saves".
- refine_save_dir_drops_when_no_save_subdir: tempdir con sólo "mod/"
  y "config/" devuelve vec![].
- refine_save_dir_returns_multiple_when_ambiguous: tempdir con
  "saves/" y "save games/" devuelve ambos.
- Conserva los tests existentes de Stellaris adaptándolos al nombre
  nuevo de la función.

Cuidado: la heurística hace read_dir en el path candidato. Eso es IO
adicional en cada hit. Hoy ya se hace stat por candidato; un read_dir
extra por hit con cap de catálogo a ~20k es asumible pero pásalo por
el mismo Semaphore que el resto.

Verificación: cargo check + cargo test -p hoard-agent verde.

NO bumpees versión. Actualiza detection-log.md con entrada DONE
P-DET-2.
```

### P-DET-3 — Fallback Steam→catalog por nombre

```
Lee detection.md §0 punto 3 y §4.1. Lee detection-log.md.

Objetivo: cuando una SteamApp tiene un appid que no existe en el catálogo
Ludusavi, intentar match por nombre slugificado antes de descartarla.

Cambios en crates/hoard-agent/src/detection.rs:

1. Tras el cross-reference por appid (el bloque actual `for entry in
   catalog { Some(appid)... }`), añade un segundo loop que itera
   steam_apps no matcheadas hasta ahora:
     for app in &steam_apps:
       if by_slug.values().any(|g| g.steam_app_id == Some(app.app_id)):
         continue   // ya cubierto por el match por appid
       let slug = slugify_for_match(&app.name)
       if let Some(entry) = catalog.iter().find(|e| e.slug == slug):
         // mismo merge que hace el match por appid, pero con
         // confidence Low (el match por nombre es ambiguo).
         by_slug.insert(entry.slug.clone(), DetectedGame {
           ..., confidence: Confidence::Low, source: SteamLibrary,
           install_dir: Some(app.install_dir.clone()),
         });

2. slugify_for_match: copia el algoritmo de
   hoard_manifest::ludusavi::slugify (byte-compatible). Si está pub,
   reusa; si está pub(crate), pídele al usuario subirlo a pub. NO
   dupliques el código — duplicarlo invita a divergencia silenciosa.

3. Confidence::Low se introduce ahora si no existía. Si ya existe
   (revisa el enum), úsala.

4. La UI ya muestra confidence en algún lado (Library.svelte). No
   toques la UI aquí; el siguiente bloque de mejora visual va en
   P-DET-5/6.

Tests:
- steam_to_catalog_fallback_matches_by_slugified_name: catálogo
  con un entry slug "test-game" sin steam_app_id; steam_apps con
  un SteamApp { app_id: 999, name: "Test Game", ... }. Verificar
  que el reporte incluye "test-game" con source=SteamLibrary,
  confidence=Low, install_dir presente.
- steam_to_catalog_fallback_skips_when_appid_already_matched: si
  el catálogo tiene la entrada con steam_app_id que ya matcheó,
  el fallback no la duplica.
- steam_to_catalog_fallback_skips_unknown_titles: nombre Steam que
  no produce slug presente en el catálogo no introduce ruido.

Verificación: cargo check + cargo test -p hoard-agent verde.

NO bumpees versión. Actualiza detection-log.md (P-DET-3 DONE).
```

### P-DET-4 — Limpieza de código muerto

```
Lee detection.md §4.3. Lee detection-log.md.

Tarea: borrar/unificar el código de detección legacy que sigue compilando
y confunde la arquitectura.

Pasos:

1. Verifica que crates/hoard-detect/ no se usa desde la ruta caliente:
   - grep -r "hoard_detect" crates/ : esperado, sólo desde
     crates/hoard-agent/src/autodetect.rs (que también se borra abajo).
   - Si aparece desde hoard-desktop o cualquier otro sitio, PARA y
     pregunta al usuario antes de continuar.
   - Si sólo lo usa autodetect.rs, sigue.

2. Verifica que crates/hoard-agent/src/autodetect.rs no se usa:
   - grep -r "autodetect::" crates/ : si hay invocaciones, PARA.
   - grep -r "register_one\|run_autodetect" crates/ : mismo.
   - Si no aparece nadie, borra el módulo, elimina la entrada en
     hoard-agent/src/lib.rs, y borra los tests del módulo.

3. Borra crates/hoard-detect/ entero:
   - rm -rf crates/hoard-detect (excepto process.rs si se usa).
   - grep -r "hoard_detect::process" crates/ : si hoard-agent o
     hoard-watcher usan ProcessWatcher de aquí, NO borres process.rs.
     Muévelo a crates/hoard-agent/src/process.rs y reescribe los
     imports.
   - Quita "crates/hoard-detect" de Cargo.toml [workspace].members.
   - Quita "hoard-detect" de [dependencies] de cualquier crate que
     lo tenga.

4. Catálogo TOML hand-curated:
   - Lista los archivos en crates/hoard-manifest/data/games/*.toml.
   - grep -r "hoard_manifest::catalogue\|hoard_manifest::lookup\|
     hoard_manifest::all_games" crates/ : si nadie los consume,
     borra el directorio data/games entero, borra el módulo
     hoard_manifest::schema, hoard_manifest::placeholders, y las
     funciones catalogue/lookup/all_games de lib.rs.
   - Si aparece alguna invocación, PARA y pregunta. El plan es
     borrarlo, pero si hay un consumidor activo que olvidé,
     necesito saberlo antes.

5. cargo check --workspace tras cada paso. Si rompe, identifica qué
   import quedó colgando y repara antes de seguir.

6. Tests: cargo test --workspace verde.

7. Documenta en detection-log.md: lista los archivos borrados, los
   movidos, y las líneas eliminadas de Cargo.toml.

NO bumpees versión. Esta tarea es defensiva y reversible vía git.
```

### P-DET-5 — Overrides persistentes del usuario

```
Lee detection.md §3 propiedad #4, §4.2, y §0 punto 6. Lee detection-log.md.

Objetivo: cuando el usuario elige un path con el folder picker (ya sea
porque la detección dejó la fila en amber, o porque corrige una
sugerencia), Hoard guarda esa elección y la respeta en re-scans futuros.

Cambios:

1. crates/hoard-agent/src/state.rs:
   - CliState gana un campo
     pub manual_paths: HashMap<String, PathBuf>  // slug → path
   - serde default empty.
   - Métodos pub fn set_manual_path(&mut self, slug: &str, path: PathBuf)
     y pub fn clear_manual_path(&mut self, slug: &str).
   - Asegúrate de que save_atomic los persiste.

2. crates/hoard-agent/src/detection.rs::detect_all:
   - El último paso del pipeline (tras refine) lee los manual_paths del
     CliState. Para cada (slug, path) en manual_paths:
       - Si by_slug ya tiene una entry para slug, sustituir found_paths
         por vec![path.clone()] y poner confidence=High,
         source=DetectionSource::ManualOverride (variante nueva del enum).
       - Si by_slug no tiene entry pero el catálogo sí, crear una entry
         nueva con esos valores. (Caso: el usuario añade manualmente un
         juego que la heurística no encontraría jamás.)
       - Si el slug no existe en el catálogo, loggear WARN y dejarlo. No
         filtrarlo: el usuario sabrá si su override quedó huérfano cuando
         no aparece la fila.
   - El paso requiere acceso al state. detect_all no lo tiene hoy. Pasa
     un &CliState como argumento; el llamante en hoard-desktop ya lo
     tiene (commands/library.rs::scan_library carga CliState).

3. crates/hoard-desktop/src/commands/library.rs:
   - Nuevo comando #[tauri::command] pub async fn set_manual_path(
       state: State<AppState>, slug: String, path: String
     ) -> Result<(), String>:
     - Valida que path exista en disco y sea directorio.
     - Carga CliState, modifica, guarda atómico.
     - Refresca el detection cache (lanza un scan_library en background).
   - Nuevo comando clear_manual_path(slug) simétrico.
   - Registra ambos en invoke_handler! (lib.rs).

4. UI (Svelte):
   - Library.svelte o el modal que abre el folder picker: tras una
     elección exitosa, llamar a set_manual_path(slug, path).
   - En la fila tracked, añadir un menú "..." con opción
     "Volver a sugerencia automática" que llama clear_manual_path(slug).
   - i18n: claves library.use_auto_detection, library.manual_path_set,
     library.manual_path_cleared en los 8 locales (en/es/de/fr/it/ja/pt/zh).

Tests:
- En state.rs: round-trip de manual_paths a disco.
- En detection.rs: pasar un CliState con un manual_path para un slug que
  el filesystem no encuentra; verificar que aparece en el reporte con
  source=ManualOverride.
- Test del comando Tauri: invocar set_manual_path con un path
  inexistente devuelve error legible.

Verificación: cargo check + pnpm check + cargo test -p hoard-agent
verde.

NO bumpees versión. Actualiza detection-log.md (P-DET-5 DONE).
```

### P-DET-6 — Diagnóstico observable

```
Lee detection.md §4.4. Lee detection-log.md.

Objetivo: el usuario power-user (o el dev) puede preguntar "¿por qué
no aparece X en mi Library?" y obtener una respuesta concreta sin leer
código.

Cambios:

1. crates/hoard-agent/src/detection.rs:
   - Nuevo struct DetectionTrace { slug, attempts: Vec<TraceStep> }
     donde TraceStep es { kind: "steam_appid" | "filesystem" |
     "proton_prefix" | "name_fallback" | "refine" | "manual_override",
     template: Option<String>, expanded: Vec<String>, kept: Vec<String>,
     dropped: Vec<{ path: String, reason: String }> }.
   - Nueva función pub async fn diagnose(slug: &str, os: Os,
     state: &CliState) -> DetectionTrace. Reproduce el pipeline
     completo pero registra cada paso en la TraceStep en vez de
     escribir el reporte global.
   - El detect_all "real" sigue sin cambiar: la diagnose es independiente.

2. crates/hoard-desktop/src/commands/library.rs:
   - Nuevo #[tauri::command] pub async fn detection_diagnostics(
       state: State<AppState>, slug: String
     ) -> Result<DetectionTrace, String>.
   - Registralo en invoke_handler!.

3. UI:
   - Crear crates/hoard-desktop/ui/src/routes/Diagnostics.svelte
     (oculta de la nav). Input para slug, botón "Diagnosticar", panel
     con el JSON pretty-printed del DetectionTrace.
   - Activar la ruta /diagnostics tras 5 clicks rápidos en el número de
     versión del sidebar (App.svelte). Si ya existe ese gesto para
     agent_status (P1.4.0-0), reusa el contador y añade un menú con
     dos enlaces.
   - i18n: diagnostics.title, diagnostics.slug_label,
     diagnostics.run_button, diagnostics.no_trace en los 8 locales.

Tests:
- En detection.rs: diagnose para un slug que no existe devuelve un
  trace con un único TraceStep { kind: "manual_override", ... }
  vacío y otro { kind: "steam_appid", dropped: [...] }.
- En detection.rs: diagnose para un slug con un compatdata sintético
  registra el TraceStep de proton_prefix correctamente.

Verificación: cargo check + pnpm check + cargo test -p hoard-agent.

NO bumpees versión. Actualiza detection-log.md (P-DET-6 DONE).
```

### P-DET-7 — Fixtures y tests integration

```
Lee detection.md y detection-log.md íntegros.

Objetivo: tests integration que cubran el pipeline completo contra
fixtures sintéticas. Estos tests son los que detectarán regresiones
silenciosas en el futuro.

Estructura:

crates/hoard-agent/tests/
├── detection_integration.rs
└── fixtures/
    ├── steam-minimal/                 # tres appmanifests + libraryfolders.vdf
    │   └── steamapps/...
    ├── compatdata-stardew/             # un prefix con save de Stardew
    │   └── steamapps/compatdata/413150/pfx/drive_c/users/steamuser/...
    └── paradox-roots/                  # Stellaris + CK3 con su jerarquía
        └── ...

Tests en detection_integration.rs:

1. fs_heuristic_finds_native_linux_save: copia fixtures/, apunta HOME
   al tempdir, ejecuta detect_all(Os::Linux), verifica que el save
   esperado aparece.

2. proton_prefix_finds_windows_only_game_on_linux: con compatdata-stardew
   en el tempdir, detect_all(Os::Linux) reporta stardew-valley con
   found_paths apuntando al prefix.

3. refine_drops_paradox_root_without_save_games: con paradox-roots
   pero sin "save games/" creado, stellaris está en el reporte con
   found_paths vacío (UI mostraría amber).

4. refine_promotes_paradox_save_games_subdir: con "save games/"
   presente, found_paths apunta a la subcarpeta, no a la raíz.

5. steam_name_fallback_picks_up_no_appid_entries: con un appmanifest
   de un juego cuyo Ludusavi entry no trae steam_app_id, el match por
   nombre slugificado lo encuentra con confidence=Low.

6. manual_override_wins_over_heuristic: CliState con manual_paths
   = { slug: /custom/path }, /custom/path existe pero los heuristics
   apuntan a otro lado; reporte tiene /custom/path con source=
   ManualOverride.

Todos los tests usan tempfile::TempDir y con_env para aislar HOME/
XDG_*. Ninguno toca el HOME real.

cargo test -p hoard-agent --test detection_integration debe pasar.

Verificación: cargo test --workspace verde. CI lo arrastra gratis.

NO bumpees versión. Actualiza detection-log.md (P-DET-7 DONE).
```

### P-DET-Z — Cierre 1.5.0

```
Lee detection.md íntegro. Lee detection-log.md y verifica que todos los
prompts P-DET-0..7 están marcados DONE. Si alguno falta, PARA y reporta.

Pasos de cierre:

1. Bump de versión a 1.5.0 en:
   - Cargo.toml [workspace.package].version
   - crates/hoard-desktop/tauri.conf.json "version"
   - crates/hoard-desktop/ui/package.json "version"
   - crates/hoard-desktop/ui/src/App.svelte fallback v{import.meta.env...}

2. CHANGELOG.md: nuevo bloque ## [1.5.0] — <hoy> arriba del ## [Unreleased].
   Formato Keep a Changelog. Entradas:

   ### Added
   - Proton/Wine prefix detection on Linux: games installed via Proton
     now appear in Library with their save path resolved against the
     compatdata prefix.
   - Manual save-path overrides: a path picked from the folder dialog
     persists in state and survives re-scans.
   - Steam-to-catalog fallback by slugified name when the Ludusavi
     entry lacks `steam_app_id`.
   - Hidden Diagnostics panel (5-click on sidebar version) explains why
     a given slug is or isn't in the detection report.

   ### Changed
   - "Game root → save subdir" refinement is now a general heuristic
     applied to every slug, not the previous hardcoded Stellaris list.
     Paradox games (CK3, EU4, HoI4, Imperator, Victoria 3) no longer
     get their entire game-root backed up.
   - Internal cleanup: removed the dead `hoard-detect` crate, the v0.3
     `autodetect.rs` module, and the hand-curated TOML catalog. Only
     the Ludusavi catalog is consulted on the hot path.

   ### Fixed
   - Stellaris no longer surfaces the game root as a save path (covered
     by the general refinement above).

3. CLAUDE.md: actualiza la línea "Current version on main" a 1.5.0.
   Quita Stellaris del "Open / deferred items" si aún figura. Añade
   una sección breve "Detección" con dos frases describiendo el
   pipeline post-overhaul y un puntero a docs/plans/detection.md y
   docs/decisions/0009-path-detection-overhaul.md.

4. docs/plans/1.5.md: marca como DONE en la lista de prompts del
   ciclo lo que aplica (P1.4.0-d Stellaris se subsume por P-DET-2 +
   P-DET-7; P1.8.0-a Proton se subsume por P-DET-1). Añade nota
   arriba del documento aclarando que el ciclo 1.5.0 lo cubre
   detection.md y que CAS se desplazó a 1.6.0.

5. Verifica antes de tag:
   - cargo check --workspace limpio.
   - pnpm --dir crates/hoard-desktop/ui check limpio.
   - cargo test --workspace verde.
   - scripts/check-i18n.mjs (si existe ya) limpio.

6. No commitees ni hagas push. Deja todo staged para revisión del
   usuario. Reporta el diff resumen y espera instrucciones para tag y
   release.

Actualiza detection-log.md con la entrada DONE de P-DET-Z y la fecha
de la release.
```

---

## 8. Cómo encaja con el plan 1.5.md original

El plan [`1.5.md`](1.5.md) reservaba 1.5.0 para "CAS + observabilidad +
tests E2E". Este overhaul de detección es lo bastante grande para ser la
release 1.5.0 por sí solo, y atrasar Proton hasta 1.8.0 (P1.8.0-a) deja a
los usuarios Linux/Steam Deck sin detección utilizable durante todo el
ciclo. La reorganización es:

- **1.5.0** = overhaul de detección (este documento).
- **1.6.0** = CAS + dedup + observabilidad + tests E2E (lo que era 1.5.0
  original). Sigue documentado en `docs/plans/1.5.md` §3.1.5; sólo cambia
  el número de release.
- **1.7.0** = pulido del cliente (lo que era 1.6.0).
- **1.8.0** = robustez del servidor (lo que era 1.7.0).
- **1.9.0** = plataformas y avanzado (lo que era 1.8.0, menos Proton que
  ya cae en 1.5.0).

Cuando se cierre 1.5.0 (este overhaul), actualiza también `1.5.md` para
reflejar el desplazamiento de los hitos siguientes.

---

## 9. Riesgos y mitigaciones

- **Proton prefixes en cantidades grandes (un usuario con 200 juegos
  Proton)**: cada prefix añade ~10 stat() por slug catalogado. Con el
  Semaphore actual de 32 ya estamos OK, pero monitoriza el tiempo total
  de detect_all en una máquina con muchos prefixes. Si pasa de 5s,
  añade un cache de "este prefix ya lo probé, no tiene saves" reset por
  hora.
- **Heurística de refinamiento devuelve falsos positivos**: un juego con
  un subdir llamado "save settings" (no son saves reales). Mitigación:
  la heurística es estricta — la subcadena se compara contra
  ["save", "saves", "savegame", "savegames", "save games", "save_games"]
  exact-on-segment, no contains arbitrario. Si aparece un caso real,
  añadirlo a SAVE_DIR_OVERRIDES con su path correcto.
- **Borrar el catálogo TOML rompe un consumidor olvidado**: P-DET-4
  exige grep exhaustivo antes de borrar y pide confirmación al usuario
  si encuentra algo. Es reversible vía git si se nos cuela algo.
- **Overrides manuales huérfanos** (el usuario añadió un slug que ya no
  está en el catálogo): se loguea WARN y se mantiene la entrada en el
  state. No se borra; el usuario puede limpiarla desde la UI.

---

## 10. Cuando se cierre

Cuando este plan esté DONE (P-DET-Z mergeado a `main` con tag v1.5.0):

1. Mover `docs/plans/detection.md` a `docs/plans/done/detection.md` (crea
   el subdirectorio si no existe).
2. Mantener `docs/plans/detection-log.md` en su sitio: el log histórico
   se queda donde está.
3. Quitar las referencias "en curso" en CLAUDE.md y dejarlas como
   históricas (link directo al ADR 0009).
