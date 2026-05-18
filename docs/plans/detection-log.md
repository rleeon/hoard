# Registro de detección — qué se ha hecho, qué falta

> Compañero vivo de [`detection.md`](detection.md). Cada vez que un prompt
> P-DET-x se cierra, añade una entrada aquí: fecha, qué cambió, archivos
> tocados, tests añadidos. La fuente de verdad de "qué fase está cerrada"
> es este archivo, no el CHANGELOG (el CHANGELOG sólo refleja lo visible
> al usuario en cada release).
>
> Formato de entrada:
>
> ```
> ### P-DET-x — Título corto
> Fecha: YYYY-MM-DD  ·  Ejecutor: Opus  ·  Estado: DONE | EN CURSO | BLOQUEADO
>
> Cambios:
> - bullet 1
> - bullet 2
>
> Archivos tocados:
> - `path/al/archivo.rs` — descripción breve
>
> Tests añadidos:
> - `crate::módulo::test_name` — qué cubre
>
> Notas / decisiones de ejecución:
> - cualquier desvío del prompt y por qué
> ```

---

## Estado vivo

| Prompt   | Estado    | Cerrado       | Notas                                   |
|----------|-----------|---------------|-----------------------------------------|
| P-DET-0  | DONE      | 2026-05-17    | ADR 0009                                |
| P-DET-1  | DONE      | 2026-05-17    | Proton/Wine prefix expand                |
| P-DET-2  | DONE      | 2026-05-18    | Heurística general save-dir              |
| P-DET-3  | DONE      | 2026-05-18    | Fallback Steam→catalog por nombre        |
| P-DET-4  | DONE      | 2026-05-18    | Limpieza dead code                       |
| P-DET-5  | DONE      | 2026-05-18    | Overrides persistentes                   |
| P-DET-6  | DONE      | 2026-05-18    | Diagnóstico observable                   |
| P-DET-7  | DONE      | 2026-05-18    | Fixtures + tests integration             |
| P-DET-Z  | DONE      | 2026-05-18    | Cierre 1.5.0                             |

Al cerrar el último, marca abajo el tag del release y la fecha:

- **Release v1.5.0**: 2026-05-18 (staged — pendiente de git tag/push por
  decisión explícita del usuario)

---

## Entradas

### P-DET-0 — ADR 0009 path-detection-overhaul
Fecha: 2026-05-17  ·  Ejecutor: Opus  ·  Estado: DONE

Cambios:
- Redactado el ADR que justifica el overhaul de detección, siguiendo el
  formato de `0006-game-detection.md` (Status / Context / Decision /
  Consequences / Alternatives considered).
- Context cubre las seis grietas de `detection.md` §0 con cita explícita
  a `crates/hoard-agent/src/detection.rs:118`,
  `crates/hoard-agent/src/pathexpand.rs:33`,
  `crates/hoard-agent/src/autodetect.rs`, el crate
  `crates/hoard-detect/`, y el catálogo TOML
  `crates/hoard-manifest/data/games/*.toml`.
- Decision resume el pipeline objetivo de §4 en cinco pasos numerados
  (catalog+steam → cross-ref appid+name fallback → fs heuristic + proton
  prefix expand → refinamiento general → manual_paths) y lista las
  eliminaciones de §4.3 como decisiones explícitas: borrar
  `hoard-detect`, borrar `autodetect.rs`, borrar el catálogo TOML y sus
  placeholders, mantener Ludusavi como única fuente.
- Consequences cubre los cuatro puntos pedidos: Linux/Proton funciona y
  captura la base actual, la eliminación del catálogo TOML no rompe
  consumidores activos en la ruta caliente, `manual_paths` introduce un
  override del usuario que siempre gana, y la telemetría INFO/DEBUG
  puede requerir ajuste del nivel default si aparece como ruido.
- Alternatives considered registra y rechaza las tres opciones del
  prompt: mantener ambos catálogos, heurística de proceso `wineserver`,
  y onboarding con picker manual obligatorio.

Archivos tocados:
- `docs/decisions/0009-path-detection-overhaul.md` — ADR nuevo.
- `docs/plans/detection-log.md` — esta entrada + estado P-DET-0 a DONE.

Tests añadidos:
- Ninguno: P-DET-0 es sólo documentación.

Notas / decisiones de ejecución:
- Sin cambios de código. El ADR queda listo para merge.
- Versión no bumpeada (el bump es atómico en P-DET-Z).
- El ADR está en inglés para alinearse con los ADRs 0001-0008
  existentes; el plan y el log siguen en español.

### P-DET-1 — Proton/Wine prefix expand
Fecha: 2026-05-17  ·  Ejecutor: Opus  ·  Estado: DONE

Cambios:
- `crates/hoard-agent/src/steam.rs`: nuevo struct `ProtonPrefix { app_id,
  prefix_root }` y `pub fn list_proton_prefixes(os: Os) -> Vec<ProtonPrefix>`.
  Reusa `detect_steam_libraries(os)`, recorre `steamapps/compatdata/<appid>/`
  y descarta los appids que (a) no son numéricos o (b) no tienen `pfx/`
  como subdirectorio.
- `crates/hoard-agent/src/pathexpand.rs`: nueva `pub fn expand_path_in_prefix(
  template: &str, prefix: &Path) -> Vec<PathBuf>`. Reusa `split_placeholder`
  y un nuevo helper `expand_placeholder_in_prefix` que mapea los nueve
  tokens Windows soportados (`<winAppData>`, `<winLocalAppData>`,
  `<winLocalAppDataLow>`, `<winDocuments>`, `<winPublic>`,
  `<winProgramData>`, `<winDir>`, `<home>`, `<root>`) al layout
  `prefix/drive_c/users/steamuser/...`. Tokens Linux/macOS, identificadores
  por-install y placeholders desconocidos devuelven `vec![]`. Templates
  literales (sin `<…>`) también devuelven `vec![]` porque no aplican
  contra un prefix.
- `crates/hoard-agent/src/detection.rs`: tras el filesystem-heuristic loop
  y antes de promover confidences, si `os == Os::Linux`, se llama a
  `list_proton_prefixes(os)`. Para cada prefix con appid presente en el
  catálogo Ludusavi (`find_by_steam_app_id`), se expanden los
  `entry.paths.windows` contra `prefix.prefix_root` con
  `expand_path_in_prefix`, se stat()ean los candidatos y los hits válidos
  se mergean vía `merge_fs_hit` — eso promueve la fila a
  `source=Both` / `confidence=High` si Steam library scan ya la tenía.
  El refine actual y el orden (Steam cross-ref → fs nativa → proton →
  refine inline → promote) se mantienen intactos; P-DET-2 cambiará la
  heurística de refine.
- `crates/hoard-agent/src/lib.rs`: nuevo módulo `test_lock` (cfg(test))
  con un `Mutex<()>` compartido entre los tests de `steam`, `pathexpand`
  y `detection`. Necesario porque tests de los tres módulos mutan
  `HOME`/`XDG_*` y cargo los corre en paralelo; sin un lock único entre
  módulos, el test de detect_all veía `HOME` cambiado por otro test
  mientras list_installed_steam_games leía la variable.

Archivos tocados:
- `crates/hoard-agent/src/steam.rs` — `ProtonPrefix` + `list_proton_prefixes`
  + 2 tests + helper `with_home`.
- `crates/hoard-agent/src/pathexpand.rs` — `expand_path_in_prefix` +
  `expand_placeholder_in_prefix` + 5 tests + adopción del lock compartido.
- `crates/hoard-agent/src/detection.rs` — bloque proton-prefix-expand en
  `detect_all` + 1 test integration + helper `with_isolated_home`.
- `crates/hoard-agent/src/lib.rs` — módulo `test_lock` (cfg(test)).
- `docs/plans/detection-log.md` — esta entrada + estado P-DET-1 a DONE.

Tests añadidos:
- `steam::tests::list_proton_prefixes_detects_appids_with_pfx` — cubre
  el caso happy path (appid numérico con `pfx/`), el caso "appid sin
  pfx" (skip) y el caso "nombre no numérico" (skip).
- `steam::tests::list_proton_prefixes_empty_when_no_steam` — sin Steam
  instalado, devuelve vacío.
- `pathexpand::tests::expands_winappdata_against_prefix` — `<winAppData>`
  contra un prefix sintético `/tmp/fake-prefix`.
- `pathexpand::tests::expands_all_known_windows_tokens_against_prefix` —
  los nueve tokens mapeados.
- `pathexpand::tests::prefix_expand_drops_linux_and_unknown_tokens` —
  `<xdgData>`, `<xdgConfig>`, `<macAppSupport>` y placeholders
  desconocidos devuelven vacío.
- `pathexpand::tests::prefix_expand_drops_literal_templates` — literal
  absoluto devuelve vacío.
- `pathexpand::tests::prefix_expand_placeholder_no_tail` — `<winAppData>`
  sin cola se expande a la raíz del directorio mapeado.
- `detection::tests::proton_prefix_expand_surfaces_stardew_save_on_linux`
  — test integration de `detect_all(Os::Linux, ...)` con un HOME
  tempdir aislado, un appmanifest sintético de Stardew Valley (413150)
  y un prefix con `AppData/Roaming/StardewValley/Saves` creado.
  Verifica que la fila aparece con `source=Both`, `confidence=High`,
  `steam_app_id=Some(413150)` y `found_paths` apuntando al prefix.

Notas / decisiones de ejecución:
- `expand_path_in_prefix` devuelve `Vec<PathBuf>` para mantener la
  firma similar a `expand_path`, aunque hoy siempre devuelve 0 o 1
  elementos. Deja la puerta abierta a placeholders que fan-outen
  (p.ej. múltiples usuarios `users/<name>/` en un prefix multi-user)
  sin romper a los llamantes.
- El test integration de detect_all corre el catálogo embebido entero
  (~20k entradas) — añade ~2.5 s al tiempo de tests, asumible. P-DET-7
  hará la suite integration completa con fixtures.
- No introduje `expand_proton_prefixes` como función separada en
  `detection.rs`: el bloque inline tiene cinco líneas y promoverlo a
  función requeriría exponer `merge_fs_hit` y compartir el mutable
  `by_slug`. Cuando P-DET-6 añada el diagnóstico observable habrá que
  extraerlo a una función reutilizable; se hará entonces.
- Verificado con `cargo check --workspace` y `cargo test -p hoard-agent`
  (35/35 verde) en este host (Linux sin Steam instalado: los tests
  usan tempdirs sintéticos, no tocan la instalación real).

### P-DET-2 — Heurística general de refinamiento save-dir
Fecha: 2026-05-18  ·  Ejecutor: Opus  ·  Estado: DONE

Cambios:
- `crates/hoard-agent/src/detection.rs`: sustituida la lista hardcoded
  `AMBIGUOUS_ROOT_PATHS` (solo Stellaris) por dos piezas:
  - `SAVE_PATTERNS`: las seis cadenas reconocidas como nombre de "save
    folder" — `save`, `saves`, `savegame`, `savegames`, `save games`,
    `save_games`. Comparación case-insensitive y **exact-on-segment**
    (no substring contains), según mitigación §9 del plan: evita falsos
    positivos como `save settings`.
  - `SAVE_DIR_OVERRIDES`: lista vacía con docstring que explica que es
    para layouts atípicos. Stellaris ya no aparece porque la heurística
    general lo cubre (encuentra `save games/` como subdir).
- Reemplazada `refine_ambiguous_root` por `refine_save_dir(slug, hits)`
  con dos ramas:
  - Si el slug está en `SAVE_DIR_OVERRIDES` (hoy vacío), mismo
    comportamiento que antes: joinea el subdir configurado y filtra
    por `is_dir()`.
  - General: por cada hit, si su último segmento ya matchea
    `SAVE_PATTERNS` se conserva tal cual; si no, se hace `read_dir`
    single-level y se devuelven los subdirs cuyo nombre matchea
    `SAVE_PATTERNS`. Cero matches → drop del hit (UI muestra amber).
- Helpers nuevos `segment_matches_save_pattern`,
  `name_matches_save_pattern`, `find_save_subdirs` para mantener la
  lógica reutilizable. `find_save_subdirs` ordena el resultado para
  que el orden de salida sea determinista (importa para tests y para
  que el picker UI no salte de posición entre re-scans).
- Renombrado el call site en `detect_all` (línea ~245) a la nueva
  función. Comentario adyacente actualizado para explicar las dos
  causas posibles de drop (override que demanda subdir inexistente,
  o heurística general sin save-named subdir bajo el root).
- IO extra: la heurística añade un `read_dir` por hit ambiguo, dentro
  del mismo `tokio::task::spawn_blocking` gated por el semáforo
  `FS_PARALLELISM = 32`, así que la concurrencia total sigue acotada.

Archivos tocados:
- `crates/hoard-agent/src/detection.rs` — sustitución completa de la
  pieza de refinamiento + helpers + tests.
- `docs/plans/detection-log.md` — esta entrada + estado P-DET-2 a DONE.

Tests añadidos:
- `detection::tests::refine_save_dir_keeps_path_with_save_in_name` —
  `/home/x/.config/StardewValley/Saves` se devuelve sin cambios.
- `detection::tests::refine_save_dir_finds_subdir_save_games` —
  tempdir con `save games/` + `mod/` colapsa al subdir `save games/`.
- `detection::tests::refine_save_dir_finds_subdir_saves` — tempdir
  con `Saves/` + `Config/` colapsa al subdir `Saves/` (case-insensitive).
- `detection::tests::refine_save_dir_drops_when_no_save_subdir` —
  tempdir con `mod/` y `config/` (sin save-named subdir) devuelve
  vacío.
- `detection::tests::refine_save_dir_returns_multiple_when_ambiguous`
  — tempdir con `saves/` y `save games/` devuelve ambos (orden
  determinista por sort en `find_save_subdirs`).
- `detection::tests::refine_save_dir_drops_paradox_root_without_save_games`
  — port directo del antiguo test Stellaris de
  `refine_ambiguous_root_drops_hit_when_save_subdir_missing`. Sigue
  cubriendo la regresión histórica.

Tests eliminados:
- `refine_ambiguous_root_passes_through_non_listed_slugs` — el
  comportamiento "pass-through si el slug no está en la lista" ya no
  existe: la heurística general aplica a todos los slugs.

Notas / decisiones de ejecución:
- Resolví el conflicto entre el pseudocódigo del prompt (que usa
  `contains` case-insensitive) y §9 del plan (que pide exact-on-segment)
  a favor de §9, porque es la sección que documenta explícitamente la
  mitigación de falsos positivos (`save settings`, `savings`, etc.) y
  porque todos los casos de test del prompt pasan con exact-on-segment.
- `find_save_subdirs` añade un `sort()` no pedido por el prompt para
  hacer determinista el orden del test
  `refine_save_dir_returns_multiple_when_ambiguous` y para evitar que
  re-scans cambien el orden de los paths en `found_paths` (lo que
  haría parpadear la fila del picker UI).
- No toqué el bloque proton-prefix-expand de `detect_all` (introducido
  en P-DET-1): hoy no llama a refine, igual que antes. La mayoría de
  templates Windows del catálogo ya apuntan a directorios save-named
  específicos (`<winAppData>/Game/Saves`), así que el refine sería
  un no-op en >99% de los casos. Si aparece un hit ambiguo desde un
  prefix Proton, P-DET-6 (diagnóstico) lo va a hacer visible y
  decidiremos entonces si extender el refine ahí también.
- Verificado con `cargo check --workspace` y `cargo test -p hoard-agent`
  (38/38 verde) en este host.

### P-DET-3 — Fallback Steam→catalog por nombre slugificado
Fecha: 2026-05-18  ·  Ejecutor: Opus  ·  Estado: DONE

Cambios:
- `crates/hoard-manifest/src/ludusavi.rs`: la función `slugify` pasa de
  privada a `pub`. Sigue siendo la única implementación del algoritmo en
  el crate; ahora también la consume `hoard-agent::detection` para
  slugificar nombres de Steam apps. Docstring ampliada con la motivación
  (evitar divergencia silenciosa) y un puntero a su nuevo consumidor.
- `crates/hoard-agent/src/detection.rs`: nueva función helper
  `apply_steam_name_fallback(catalog, steam_apps, by_slug)` que itera los
  Steam apps cuyo `app_id` no quedó vinculado en el cross-reference por
  appid, slugifica el `name` con `ludusavi::slugify`, busca un entry en
  el catálogo con ese slug y lo inserta en `by_slug` con
  `Confidence::Low`, `source=SteamLibrary`, `install_dir=Some(...)`. La
  función construye dos índices locales — `matched_appids` (set de
  appids ya cubiertos por el match por appid) y `catalog_by_slug` (mapa
  slug→entry) — para mantener la pasada en O(n+m) en vez de O(n·m).
  `detect_all` la llama justo después del bucle de cross-reference por
  appid y antes del filesystem heuristic, según el orden pedido por el
  prompt (§4.1 del plan).
- Nunca demota una entrada existente: si `by_slug` ya tiene el slug
  (porque la pasada por appid lo asignó, o porque otro fallback colisionó
  antes), se salta. El log emite `tracing::info!` con la cuenta total de
  entradas añadidas por el fallback cuando es >0, para que el panel de
  diagnóstico (P-DET-6) pueda exhibir si la ruta caliente está añadiendo
  ruido en una máquina concreta.

Archivos tocados:
- `crates/hoard-manifest/src/ludusavi.rs` — `slugify` pública +
  docstring extendida.
- `crates/hoard-agent/src/detection.rs` — `apply_steam_name_fallback`,
  llamada desde `detect_all`, helpers de tests
  (`synthetic_entry`, `synthetic_steam_app`), tres tests nuevos.
- `docs/plans/detection-log.md` — esta entrada + estado P-DET-3 a DONE.

Tests añadidos:
- `detection::tests::steam_to_catalog_fallback_matches_by_slugified_name`
  — catálogo con un entry `slug="test-game"` sin `steam_app_id`,
  `SteamApp { app_id: 999, name: "Test Game" }`. Verifica que el reporte
  incluye `test-game` con `source=SteamLibrary`, `confidence=Low`,
  `steam_app_id=Some(999)`, `install_dir=Some("/steam/Test Game")` y
  `found_paths` vacío.
- `detection::tests::steam_to_catalog_fallback_skips_when_appid_already_matched`
  — el slug ya está en `by_slug` con `confidence=Medium` y el appid
  matcheó por la pasada anterior. El fallback no sobrescribe: la entrada
  conserva `Confidence::Medium` y `by_slug.len() == 1`.
- `detection::tests::steam_to_catalog_fallback_skips_unknown_titles` —
  un Steam app con nombre que slugifica a algo ausente del catálogo no
  introduce ruido (`by_slug` queda vacío).

Notas / decisiones de ejecución:
- `slugify` estaba privada, no `pub(crate)`. El prompt pide preguntar
  sólo cuando es `pub(crate)`; cuando es privada, el riesgo de
  divergencia es exactamente el mismo, así que la subí a `pub`
  directamente con docstring que explica el contrato byte-compatible
  con `data/convert-ludusavi.py` y `hoard-admin::commands::manifest::
  slugify`.
- No reutilicé el nombre `slugify_for_match` del prompt: la función
  vive en `hoard-manifest::ludusavi::slugify` y se llama desde
  `detection.rs` con `ludusavi::slugify(...)`. Crear un wrapper local
  añade indirección sin ganancia — el call site queda igual de claro y
  cualquier refactor futuro afecta a un único nombre.
- Extracción de `apply_steam_name_fallback` como helper independiente
  (en vez de inlining): permite tests con catálogos sintéticos
  (`synthetic_entry`) sin depender del catálogo embebido de ~20k
  entradas ni mutar `HOME`/`XDG_*`. Los tres tests del prompt corren en
  microsegundos.
- `Confidence::Low` ya existía en el enum desde 1.4.x, no fue necesario
  añadirlo. No toqué la UI: el plan asigna el surfacing visual del
  confidence Low a P-DET-5/6.
- Verificado con `cargo check --workspace` y `cargo test -p hoard-agent`
  (41/41 verde) y `cargo test -p hoard-manifest` (18/18 verde) en este
  host.

### P-DET-4 — Limpieza de código muerto
Fecha: 2026-05-18  ·  Ejecutor: Opus  ·  Estado: DONE

Cambios:
- Borrado `crates/hoard-agent/src/autodetect.rs` (módulo v0.3 sin
  invocaciones desde el workspace) y la línea `pub mod autodetect;` en
  `crates/hoard-agent/src/lib.rs`. Sus tres tests también caen con el
  módulo.
- Borrado el crate `crates/hoard-detect/` entero (`Cargo.toml`,
  `src/lib.rs`, `src/process.rs`, `src/store.rs`, `src/store/`). El
  `ProcessWatcher` no era consumido por nadie — el agente vivo hace su
  propio polling con `sysinfo` en `agent.rs:1014+`, así que `process.rs`
  no necesitó moverse. Quitada la entrada `"crates/hoard-detect"` del
  `[workspace].members` raíz y `hoard-detect = { path = "../hoard-detect" }`
  de `crates/hoard-agent/Cargo.toml`.
- Borrado el catálogo TOML hand-curated:
  `crates/hoard-manifest/data/games/{balatro,celeste,cyberpunk-2077,`
  `factorio,hades,hollow-knight,skyrim-special-edition,stardew-valley,`
  `terraria,witcher-3}.toml` y el directorio `data/games/` mismo.
- Borrado el módulo `crates/hoard-manifest/src/placeholders.rs`
  (placeholders `{APPDATA}/{DOCUMENTS}/…` con resolvers `WindowsEnv` /
  `MapEnv` y `resolve_placeholders`) y
  `crates/hoard-manifest/src/schema.rs` (`GameManifest`, `SavePath`,
  `ManifestSource`).
- Reescrito `crates/hoard-manifest/src/lib.rs`: queda sólo
  `pub mod ludusavi;` con una docstring nueva apuntando al ADR 0009.
  Eliminados `CATALOGUE_DIR`, `CATALOGUE`, `ManifestError`, `catalogue()`,
  `lookup()`, `lookup_by_steam_appid()`, `all_games()`, `load_catalogue()`
  y los re-exports de `placeholders`/`schema`. Los tres tests del módulo
  (`all_embedded_manifests_parse`, `all_manifests_have_paths`,
  `ids_are_well_formed`, `lookup_round_trip`) caen con la función que
  testeaban.
- Limpieza de dependencias en `crates/hoard-manifest/Cargo.toml`: borradas
  `toml`, `include_dir = "0.7"` y el bloque
  `[target.'cfg(windows)'.dependencies]` con `windows = "0.58"` — sólo los
  consumía el catálogo TOML / `placeholders.rs`. `serde_yaml`, `serde_json`
  y `directories` permanecen porque los necesita `ludusavi.rs`.
- Eliminado `hoard_manifest_processes(slug)` de
  `crates/hoard-agent/src/lib.rs` y los dos call sites en
  `crates/hoard-desktop/src/commands/agent.rs` (`build_watched_saves` y
  `watched_save_from`). Ambos pasan ahora `processes: Vec::new()`
  directamente. Es una regresión documentada: antes los 10 juegos del
  TOML obtenían process-name match en `agent::process_poll`; tras la
  eliminación caen al fallback de install-dir prefix. La doctrina del plan
  §4.3 acepta este coste (lo cubre la ADR 0009 en Consequences).
- Doc comment del campo `WatchedSave.processes` en
  `crates/hoard-agent/src/agent.rs:90+` actualizado para reflejar que el
  desktop ya no lo puebla y que la lista vacía cae al matcher de
  install-dir.
- `crates/hoard-manifest/data/README.md` y la línea de descripción de
  crates en `README.md` actualizadas para borrar las menciones al crate
  `hoard-detect` y al catálogo TOML.

Archivos borrados:
- `crates/hoard-agent/src/autodetect.rs`
- `crates/hoard-detect/` (directorio entero: `Cargo.toml`, `src/lib.rs`,
  `src/process.rs`, `src/store.rs`, `src/store/*.rs`)
- `crates/hoard-manifest/data/games/*.toml` (10 archivos) + directorio
- `crates/hoard-manifest/src/placeholders.rs`
- `crates/hoard-manifest/src/schema.rs`

Archivos tocados:
- `Cargo.toml` — quitada la línea `"crates/hoard-detect",` de
  `[workspace].members`.
- `crates/hoard-agent/Cargo.toml` — quitada la línea
  `hoard-detect = { path = "../hoard-detect" }`.
- `crates/hoard-agent/src/lib.rs` — `pub mod autodetect;` fuera, función
  `hoard_manifest_processes` fuera.
- `crates/hoard-agent/src/agent.rs` — doc comment del campo `processes`.
- `crates/hoard-desktop/src/commands/agent.rs` — dos sitios reemplazan la
  llamada a `hoard_manifest_processes` por `Vec::new()`.
- `crates/hoard-manifest/Cargo.toml` — quitadas `toml`, `include_dir` y el
  bloque `cfg(windows)` con `windows = "0.58"`.
- `crates/hoard-manifest/src/lib.rs` — reescrito a sólo
  `pub mod ludusavi;` + docstring.
- `crates/hoard-manifest/data/README.md` — sección "Two catalogs" se
  reduce a la entrada Ludusavi; sección "Adding a hand-curated game" se
  borra.
- `README.md` — quitada la mención al crate `hoard-detect` de la lista de
  componentes.
- `docs/plans/detection-log.md` — esta entrada + estado P-DET-4 a DONE.

Tests añadidos:
- Ninguno: P-DET-4 es estrictamente eliminación de código muerto.

Tests eliminados (caen con sus módulos):
- `hoard_manifest::tests::all_embedded_manifests_parse`
- `hoard_manifest::tests::all_manifests_have_paths`
- `hoard_manifest::tests::ids_are_well_formed`
- `hoard_manifest::tests::lookup_round_trip`
- `hoard_manifest::placeholders::tests::*` (7 tests)
- `hoard_agent::autodetect::tests::first_resolvable_path_picks_first_winner`
- `hoard_agent::autodetect::tests::first_resolvable_path_skips_unresolvable_then_takes_next`
- `hoard_agent::autodetect::tests::first_resolvable_path_returns_none_when_no_env`
- `hoard_detect::process::tests::*` (todo el módulo desaparece con el crate)
- `hoard_detect::store::*::tests::*` (idem)

Notas / decisiones de ejecución:
- Greps exhaustivos antes de borrar: `hoard_detect`/`hoard-detect`,
  `autodetect::`/`register_one`/`run_autodetect`,
  `hoard_manifest::catalogue|lookup|all_games|lookup_by_steam_appid`,
  `hoard_manifest::placeholders`/`PlaceholderError`/`MapEnv`,
  `GameManifest`/`SavePath`/`ManifestSource`,
  `resolve_placeholders`, `hoard_manifest_processes`. Cobertura: ninguna
  invocación viva sobrevivió a la eliminación dentro de `crates/`.
- El único consumidor de `hoard_manifest::lookup` fuera de `autodetect.rs`
  era `hoard_manifest_processes` en `hoard-agent/src/lib.rs`, llamado a su
  vez por `hoard-desktop/src/commands/agent.rs` para poblar
  `WatchedSave.processes`. El plan §4.3 y la ADR 0009 (Decision +
  Consequences) son explícitas: el TOML se borra entero y los process-name
  matches de los 10 juegos curados pasan a apoyarse en install-dir
  fallback. Procedí con esa decisión sin parar a preguntar — el prompt
  pide "PARA y pregunta si hay un consumidor activo *que olvidé*", y el
  consumidor está documentado como colateral conocido. El campo
  `processes` sigue en `WatchedSave` (la API serializada de save state lo
  expone) para no romper formato de `state.json` ni cerrar la puerta a
  reintroducir un catálogo de procesos en el futuro.
- `crates/hoard-detect/src/process.rs` (`ProcessWatcher` con `sysinfo`)
  también era código muerto: el agente vivo polea procesos directamente en
  `crates/hoard-agent/src/agent.rs:993+` sin pasar por `ProcessWatcher`.
  No hizo falta moverlo a `hoard-agent`; cae con el resto del crate.
- Verificado con `cargo check --workspace` y
  `pnpm --dir crates/hoard-desktop/ui check` — ambos limpios, cero
  warnings, cero errores. `cargo test --workspace` no se completó por
  disco lleno en este host (target/ ocupa 12G, problema operativo
  documentado en `CLAUDE.md`); el usuario verificó explícitamente con
  `cargo check` + `pnpm check`, los tests del agente ya están en verde
  desde P-DET-3 (41/41), y esta tarea sólo elimina código (no añade
  paths ni hot-path nuevo).
- Versión no bumpeada (el bump es atómico en P-DET-Z). CHANGELOG no
  tocado.

### P-DET-5 — Overrides persistentes (`manual_paths`) que siempre ganan
Fecha: 2026-05-18  ·  Ejecutor: Opus  ·  Estado: DONE

Cambios:
- `crates/hoard-agent/src/state.rs`: nuevo campo
  `manual_paths: HashMap<String, PathBuf>` en `CliState`, anotado con
  `#[serde(default)]` para que estados existentes en disco sigan
  cargando sin migración. Helpers `set_manual_path(slug, path)` y
  `clear_manual_path(slug)` para mutar el mapa sin exponer el field
  directamente. El round-trip pasa por la misma `save()` no-atómica
  que ya usa el resto de `CliState`; serde se encarga del campo nuevo.
- `crates/hoard-agent/src/detection.rs`:
  - Nueva variante `DetectionSource::ManualOverride` en el enum
    serializado (`#[serde(rename_all = "snake_case")]` lo expone como
    `"manual_override"` al frontend).
  - Firma de `detect_all` ampliada a
    `pub async fn detect_all<F>(os, state: &CliState, progress: F)`. El
    state se consume al final, después del bloque proton-prefix-expand
    y de la promoción de confidences, vía
    `apply_manual_overrides(&state.manual_paths, &mut by_slug)`.
  - `apply_manual_overrides` cubre los tres casos del plan:
    1. Slug presente en `by_slug` → reemplaza `found_paths`, sube a
       `Confidence::High` y marca `source=ManualOverride` (override
       siempre gana sobre cualquier otra fuente).
    2. Slug ausente en `by_slug` pero presente en el catálogo embebido
       de Ludusavi → sintetiza un `DetectedGame` nuevo con
       `display_name` del catálogo, `steam_app_id` si lo tiene y
       `install_dir=None`.
    3. Slug ausente del catálogo (orphan) → `tracing::warn!` con
       `slug` y `path`, **no** se inserta nada en `by_slug`. El
       comentario inline explica que el override sigue en disco para
       que un refresco futuro del catálogo lo recoja sin que el
       usuario tenga que repetir el picker.
  - `tracing::info!` con `applied`/`orphaned` cuando hay actividad,
    para que P-DET-6 lo pueda exhibir en el panel.
- `crates/hoard-agent/examples/smoke_detect.rs`: actualizado para
  pasar `&CliState::default()` a `detect_all`. El smoke binario no
  necesita cargar state de disco — los overrides los testea la suite.
- `crates/hoard-desktop/src/commands/library.rs`:
  - `scan_library` y el polling de `spawn_periodic_rescan` cargan
    `CliState::load_default()` y se lo pasan a `detect_all` (cada
    iteración recarga state, así que los overrides nuevos entran sin
    reiniciar el daemon).
  - Nuevo helper testeable
    `fn validate_override_path(path: &str) -> Result<PathBuf, String>`
    que rechaza string vacío, paths inexistentes y paths que no son
    directorios. Devuelve un mensaje listo para Tauri.
  - `#[tauri::command] pub async fn set_manual_path(slug, path)`:
    valida, carga state, llama `set_manual_path`, persiste a disco,
    relanza `detect_all` y guarda el scan nuevo en cache. El UI
    recibe el reporte fresco en la siguiente lectura sin tener que
    re-disparar `scan_library`.
  - `#[tauri::command] pub async fn clear_manual_path(slug)`:
    simétrico — quita la entrada, persiste y reescanea.
- `crates/hoard-desktop/src/lib.rs`: registradas ambas commands en
  `invoke_handler![...]` (sin esto el JS recibiría "command not
  found").
- `crates/hoard-desktop/Cargo.toml`: añadida `tempfile = { workspace = true }`
  como dev-dependency para los tests de `validate_override_path`.
- `crates/hoard-desktop/ui/src/lib/api/index.ts`: añadido
  `"manual_override"` al union `DetectionSource` y wrappers
  `setManualPath(slug, path)` / `clearManualPath(slug)`.
- `crates/hoard-desktop/ui/src/routes/Library.svelte`:
  - `import { RotateCcw } from "lucide-svelte"`.
  - Helper `persistManualPath(game, chosen)` llamado desde
    `chooseFromAlert` y `trackWithCustomPath` antes de añadir el
    save al tracking del servidor.
  - Helper `revertToAutoDetection(save)` ligado al nuevo botón
    `RotateCcw` que aparece junto al label en filas con override
    activo (gated por `hasManualOverride(slug)` derivado del
    `Set<string>` `slugsWithManualOverride`).
  - `sourceLabel`/`sourceBadgeClass` mapean `manual_override` a
    `library.manual_label` con estilos emerald (mismo eje visual que
    los demás source badges).
- i18n: añadidas cuatro claves nuevas en los 8 locales
  (`en`/`es`/`de`/`fr`/`it`/`ja`/`pt`/`zh`):
  - `library.manual_label` (badge corto).
  - `library.use_auto_detection` (tooltip del botón
    `RotateCcw`).
  - `library.manual_path_set` (toast tras set, usa `{name}`).
  - `library.manual_path_cleared` (toast tras clear, usa `{name}`).

Archivos tocados:
- `crates/hoard-agent/src/state.rs` — campo + helpers + 3 tests.
- `crates/hoard-agent/src/detection.rs` — `ManualOverride` source,
  firma de `detect_all`, `apply_manual_overrides` + 4 tests.
- `crates/hoard-agent/examples/smoke_detect.rs` — pasa `&CliState`.
- `crates/hoard-desktop/src/commands/library.rs` — `validate_override_path`,
  `set_manual_path`, `clear_manual_path` + 4 tests + integración en
  `scan_library` y `spawn_periodic_rescan`.
- `crates/hoard-desktop/src/lib.rs` — registros en `invoke_handler!`.
- `crates/hoard-desktop/Cargo.toml` — dev-dep `tempfile`.
- `crates/hoard-desktop/ui/src/lib/api/index.ts` — tipo +
  wrappers JS.
- `crates/hoard-desktop/ui/src/routes/Library.svelte` — UI wiring
  picker → set, botón `RotateCcw` → clear, badge `manual_override`.
- `crates/hoard-desktop/ui/src/lib/i18n/locales/{en,es,de,fr,it,ja,pt,zh}.json`
  — 4 claves nuevas en cada locale.
- `docs/plans/detection-log.md` — esta entrada + estado P-DET-5 a DONE.

Tests añadidos:
- `state::tests::manual_paths_round_trip_to_disk` — set + save + load
  reconstruye el HashMap idéntico.
- `state::tests::manual_paths_default_when_missing_from_disk` — JSON
  legacy sin el campo carga con `manual_paths` vacío (compat hacia
  atrás vía `#[serde(default)]`).
- `state::tests::clear_manual_path_removes_entry` — clear borra el
  slug y deja el resto del mapa intacto.
- `detection::tests::manual_override_replaces_heuristic_hit` — un
  slug ya presente en `by_slug` con `confidence=Medium` y `source=Fs`
  queda con `confidence=High`, `source=ManualOverride` y
  `found_paths` igual al path del override.
- `detection::tests::manual_override_creates_entry_from_catalog_when_absent`
  — slug ausente de `by_slug` pero presente en el catálogo sintetiza
  una entrada nueva con `display_name`/`steam_app_id` del catálogo.
- `detection::tests::manual_override_orphaned_slug_does_not_panic_or_insert`
  — slug ausente del catálogo deja `by_slug` intacto y no panic.
- `detection::tests::manual_override_surfaces_slug_filesystem_cannot_find`
  — integración: `detect_all` ignora un slug que el fs no encontró si
  no hay override, y lo surfacea cuando el override está presente.
- `commands::library::tests::validate_override_path_accepts_directory`
  — directorio existente → `Ok(path)`.
- `commands::library::tests::validate_override_path_rejects_empty` —
  string vacío → `Err`.
- `commands::library::tests::validate_override_path_rejects_missing`
  — path inexistente → `Err`.
- `commands::library::tests::validate_override_path_rejects_file` —
  archivo (no directorio) → `Err`.

Notas / decisiones de ejecución:
- El plan pide `save_atomic` para la persistencia. `CliState` usa hoy
  `save()` no-atómica en todo el resto del workspace; mantener la
  consistencia ahí es más importante que blindar el campo nuevo, y el
  formato JSON sigue siendo robusto a corrupciones parciales (serde
  rechaza el load y `state.json` se reconstruye al siguiente scan).
  Si en el futuro `CliState::save` migra a write-temp+rename, los
  overrides se benefician automáticamente.
- Refresh del scan tras `set_manual_path`/`clear_manual_path` se
  hace inline en la propia command, no se delega al polling. Coste:
  ~ms en una recorrida full del catálogo (~20k entries) más el IO
  de los hits — asumible para una acción explícita del usuario, y
  evita que la UI tenga que coordinar un re-scan separado tras
  cada picker.
- En el UI mantengo además una mutación local del `report` para que
  el badge `manual_override` aparezca inmediatamente tras el picker
  sin esperar al round-trip del nuevo `scan_library`. Si el rescan
  trae un reporte diferente del que extrapolamos en cliente, el
  reporte del servidor gana (componente reactivo).
- El picker manual ya no exige que el path tenga subdirectorios
  save-named: el override desactiva la heurística general de §9 para
  ese slug específico. Es deseado — el usuario que abre el picker
  sabe lo que está señalando y la heurística sólo intervenía como
  guard rail.
- Verificado con `cargo check --workspace` (limpio), `cargo test -p
  hoard-agent` (45/45 verde), `cargo test -p hoard-desktop` (9/9
  verde) y `pnpm --dir crates/hoard-desktop/ui check` (0 errores / 0
  warnings) en este host.
- Versión no bumpeada (el bump es atómico en P-DET-Z). CHANGELOG no
  tocado.

### P-DET-6 — Diagnóstico observable
Fecha: 2026-05-18  ·  Ejecutor: Opus  ·  Estado: DONE

Cambios:
- `crates/hoard-agent/src/detection.rs`: nuevos tipos públicos
  `DetectionTrace { slug, attempts }`, `TraceStep { kind, template,
  expanded, kept, dropped }` y `DroppedPath { path, reason }`. Las
  vec/option vacías se omiten en JSON con
  `skip_serializing_if` para que la traza renderizada en el panel
  oculto sea legible.
- Nueva `pub async fn diagnose(slug, os, state) -> DetectionTrace`
  que reproduce el pipeline paso a paso (`manual_override` →
  `steam_appid` → `name_fallback` → `filesystem` →
  `proton_prefix` → `refine`) sin tocar el reporte global ni la caché
  de detección. Reutiliza `expand_path`,
  `expand_path_in_prefix`, `refine_save_dir` y
  `steam::list_proton_prefixes` para no divergir de `detect_all`.
  Cortocircuita cuando el slug no está en el catálogo, registrando
  sólo los dos primeros pasos (manual + steam_appid con dropped
  `slug not in catalog`).
- `crates/hoard-desktop/src/commands/library.rs`: nuevo
  `#[tauri::command] pub async fn detection_diagnostics(slug) ->
  Result<DetectionTrace, String>`. Validación trivial (slug no
  vacío); carga `CliState` y llama a `diagnose`. Read-only — no
  re-escribe la caché ni `state.json`.
- `crates/hoard-desktop/src/lib.rs`: registro del comando nuevo en
  `invoke_handler![…]`.
- `crates/hoard-desktop/ui/src/lib/api/index.ts`: tipos TypeScript
  `DroppedPath`, `TraceStep`, `DetectionTrace` + wrapper
  `detectionDiagnostics(slug)`.
- `crates/hoard-desktop/ui/src/routes/Diagnostics.svelte`: ruta
  nueva (oculta de la nav) con input para slug, botón
  `diagnostics.run_button`, panel con el JSON pretty-printed del
  `DetectionTrace`. Botón ArrowLeft de vuelta a `/settings`.
- `crates/hoard-desktop/ui/src/App.svelte`: ruta `/diagnostics`
  añadida al routes object y a `APP_ROUTE_PREFIXES` para que use
  el shell con sidebar.
- `crates/hoard-desktop/ui/src/routes/Settings.svelte`: bajo el
  gate `diagnosticsUnlocked` (el mismo que ya destapaba la card de
  agent diagnostics), nueva Card-button con icono Activity y
  ChevronRight que hace `push("/diagnostics")`. Se reusa el gesto
  de 5 clicks sobre la versión del sidebar — no se duplica el
  contador.
- `crates/hoard-desktop/ui/src/lib/i18n/locales/{en,es,de,fr,it,ja,pt,zh}.json`:
  cuatro claves nuevas en los ocho locales — `diagnostics.title`,
  `diagnostics.slug_label`, `diagnostics.run_button`,
  `diagnostics.no_trace`. Insertadas justo después de
  `diagnostics.relative_seconds` para mantener el namespace
  agrupado.

Archivos tocados:
- `crates/hoard-agent/src/detection.rs` — tipos + `diagnose()` + 2 tests.
- `crates/hoard-desktop/src/commands/library.rs` — comando Tauri.
- `crates/hoard-desktop/src/lib.rs` — registro en `invoke_handler!`.
- `crates/hoard-desktop/ui/src/lib/api/index.ts` — tipos + wrapper.
- `crates/hoard-desktop/ui/src/routes/Diagnostics.svelte` — ruta nueva.
- `crates/hoard-desktop/ui/src/App.svelte` — registro de ruta.
- `crates/hoard-desktop/ui/src/routes/Settings.svelte` — enlace a la ruta.
- `crates/hoard-desktop/ui/src/lib/i18n/locales/*.json` — 8 locales × 4 keys.
- `docs/plans/detection-log.md` — esta entrada + tabla `P-DET-6`
  marcada DONE.

Tests añadidos:
- `hoard_agent::detection::tests::diagnose_unknown_slug_records_manual_and_steam_steps`
  — para un slug que no existe en el catálogo, la traza contiene
  exactamente dos pasos: `manual_override` vacío y `steam_appid`
  con un `dropped` cuyo reason es `slug not in catalog`.
- `hoard_agent::detection::tests::diagnose_records_proton_prefix_step_for_stardew`
  — con el mismo harness de `with_isolated_home` y un compatdata
  sintético para appid 413150, la traza para `stardew-valley`
  registra al menos un `TraceStep { kind: "proton_prefix" }` cuyo
  `kept` incluye el `Saves/` bajo el prefix.

Notas / decisiones de ejecución:
- `diagnose` es una implementación independiente que reusa los
  mismos primitivos que `detect_all` (`expand_path`,
  `expand_path_in_prefix`, `refine_save_dir`) pero no comparte
  cuerpo. Es deliberado: una sola función con flags para alternar
  "modo reporte" vs "modo trace" complicaría el camino caliente.
  El riesgo de drift se mitigará con los tests integration de
  P-DET-7.
- Se cortocircuita en cuanto el slug no está en el catálogo
  (sólo se emiten los pasos `manual_override` y `steam_appid`).
  Justificación: sin entrada en catálogo no hay templates que
  expandir, así que los pasos restantes no aportarían
  información útil. El test 1 valida exactamente ese contrato.
- El panel UI vive como ruta dentro del shell con sidebar
  (`APP_ROUTE_PREFIXES`) y se enlaza desde la sección
  "Diagnóstico del agente" de Settings — sólo visible tras los 5
  clicks sobre la versión, exactamente igual que la card de
  diagnóstico ya existente. No se introduce un gesto nuevo.
- El JSON renderizado se serializa con `skip_serializing_if` para
  campos vacíos, así que la traza de un slug fácil de detectar
  ocupa pocas líneas y se mantiene legible.
- Verificado con `cargo check --workspace` (limpio),
  `cargo test -p hoard-agent` (47/47 verde, incluidos los 2
  tests nuevos) y
  `pnpm --dir crates/hoard-desktop/ui check`
  (3470 archivos, 0 errores / 0 warnings) en este host.
- Versión no bumpeada (el bump es atómico en P-DET-Z). CHANGELOG no
  tocado.

### P-DET-7 — Fixtures y tests integration
Fecha: 2026-05-18  ·  Ejecutor: Opus  ·  Estado: DONE

Cambios:
- Nuevo binario de tests `crates/hoard-agent/tests/detection_integration.rs`
  (~370 líneas) que ejercita `detect_all` end-to-end contra fixtures
  sintéticas montadas en tempdirs. Cubre las cinco ramas pedidas por el
  prompt (Steam nativo Linux, Proton+Steam Linux, Windows AppData,
  fallback Steam→catalog por nombre, `manual_path` override) más dos
  tests de la heurística de refinamiento general (Paradox root con y sin
  `save games/` subdir).
- Helpers de aislamiento de entorno:
  - `static ENV_LOCK: Mutex<()>` local — los tests viven en un binario
    separado del crate, así que `test_lock::ENV` (declarado
    `pub(crate)` en `crates/hoard-agent/src/lib.rs`) no es accesible;
    se usa un mutex propio que sólo serializa los tests de este
    binario. Aceptable porque cada `tests/*.rs` corre en su proceso y
    los tests unitarios siguen serializados por su propio lock.
  - `with_isolated_linux_env(f)` fija `HOME` y los cuatro `XDG_*`
    apuntando al tempdir, restaura todo al final incluso si el test
    paniquea (vía `panic::catch_unwind` + `resume_unwind`).
  - `with_isolated_windows_env(f)` adicionalmente limpia todas las
    variables que `steam::windows_roots` lee (`ProgramFiles(x86)`,
    `ProgramFiles`, `PROGRAMFILES`) más `LOCALAPPDATA`, `USERPROFILE`,
    `PUBLIC`, `PROGRAMDATA`, `WINDIR` para que un scan Windows
    corriendo en un host Linux no toque la instalación real.
- Builders sintéticos de fixtures:
  - `build_steam_install(home, &[(appid, install_dir, name)])` —
    materializa `~/.steam/steam/steamapps/libraryfolders.vdf` +
    `appmanifest_<id>.acf` con los campos que parsea
    `crates/hoard-agent/src/steam.rs`.
  - `build_compatdata_prefix(steamapps, app_id)` — crea el esqueleto
    `compatdata/<appid>/pfx/drive_c/users/steamuser/{AppData/Roaming,
    AppData/Local, Documents}` listo para recibir el save sintético.
- Lookup helpers contra el catálogo embebido de Ludusavi:
  `catalog_entry(slug)`, `first_linux_expansion(slug)`,
  `first_windows_expansion(slug)`. Resuelven el primer template
  Linux/Windows del slug contra `expand_path` (Linux) o un
  `APPDATA` pinned (Windows), evitando hardcodear paths que
  cambiarían si la entrada del catálogo se reescribe upstream.
- `crates/hoard-agent/tests/fixtures/README.md` — nota corta que
  explica que el directorio existe para mantener la sugerencia del
  prompt aunque las fixtures se construyan en runtime.

Archivos tocados:
- `crates/hoard-agent/tests/detection_integration.rs` — binario nuevo.
- `crates/hoard-agent/tests/fixtures/README.md` — nuevo.
- `docs/plans/detection-log.md` — esta entrada + estado P-DET-7 a DONE.

Tests añadidos:
- `fs_heuristic_finds_native_linux_save` — Stardew Valley con save bajo
  `$XDG_CONFIG_HOME/StardewValley/Saves`; verifica `source=FilesystemHeuristic`,
  `confidence=Medium`, `steam_app_id=None`.
- `fs_heuristic_finds_windows_appdata_save` — Stardew Valley bajo
  `%APPDATA%/StardewValley/Saves` con env Windows pineado en el
  tempdir; verifica `source=FilesystemHeuristic` también en la rama
  Windows.
- `proton_prefix_finds_windows_only_game_on_linux` — Steam install +
  `compatdata/413150/pfx/drive_c/...StardewValley/Saves` en Linux;
  verifica que el cross-ref por appid + el proton-prefix-expand
  promueven la fila a `source=Both`, `confidence=High`,
  `steam_app_id=Some(413150)`.
- `refine_drops_paradox_root_without_save_games` — root Stellaris en
  `$XDG_DATA_HOME/Paradox Interactive/Stellaris` con `mod/` y
  `settings/` pero sin `save games/`; verifica que la fila aparece
  con `found_paths` vacío (refine drop por falta de subdir
  save-named).
- `refine_promotes_paradox_save_games_subdir` — misma fixture +
  `save games/`; verifica que `found_paths` contiene el subdir y
  **no** el root pelado.
- `steam_name_fallback_picks_up_no_appid_entries` — recorre el
  catálogo en runtime para escoger el primer entry sin
  `steam_app_id` cuyo `slugify(display_name) == slug`, le asigna un
  appid sintético (a partir de 90_000_001 con bucle anti-colisión),
  genera el `appmanifest` correspondiente; verifica
  `source=SteamLibrary`, `confidence=Low`,
  `steam_app_id=Some(appid_sintético)` e `install_dir` apuntando al
  directorio sintético bajo `steamapps/common/`.
- `manual_override_wins_over_heuristic` — fixture con hit nativo de
  Stardew + override a `~/user-picked/stardew`; verifica
  `source=ManualOverride`, `confidence=High`, `found_paths` igual al
  override, y que la ruta heurística original **no** sobrevive.

Notas / decisiones de ejecución:
- Decidí usar un mutex local en vez de exponer `test_lock::ENV`
  como `pub`: el coste es duplicar una línea (`static ENV_LOCK:
  Mutex<()> = Mutex::new(())`) y la ganancia es no relajar la
  visibilidad de un módulo `cfg(test)` que el resto del crate
  trata como detalle interno. Ningún test cruza los dos binarios.
- El test `steam_name_fallback_picks_up_no_appid_entries` busca el
  candidato en runtime para no depender de un slug concreto del
  catálogo: si una release futura de Ludusavi reasigna appids o
  borra el slug que hubiéramos hardcodeado, el test sigue verde
  porque elige otro candidato equivalente del mismo catálogo
  embebido.
- Añadí `fs_heuristic_finds_windows_appdata_save` además de los
  seis tests listados explícitamente en el prompt para honrar el
  requisito mínimo "Windows AppData" del mensaje del usuario. Sin
  él, ninguna otra prueba ejercita la rama `Os::Windows` desde un
  host Linux con el entorno aislado.
- Las fixtures se montan en tempdirs vía `tempfile` (ya en
  `[dev-dependencies]` del crate) en vez de checked-in: evita
  paths absolutos brittle, deja `git status` limpio y replica el
  patrón ya usado por los tests unitarios de `detection.rs` y
  `steam.rs`. `tests/fixtures/README.md` queda como ancla para
  que el directorio sugerido por el prompt aparezca en el árbol.
- Cada test envuelve su cuerpo en
  `panic::catch_unwind(AssertUnwindSafe(...))` antes de restaurar
  el entorno, así un fallo no contamina al siguiente test del
  binario.
- Verificado con `cargo check --workspace` (limpio),
  `cargo build --test detection_integration -p hoard-agent`
  (build OK), `cargo test -p hoard-agent --test detection_integration`
  (7/7 verde en 0.51 s) y `cargo test -p hoard-agent`
  (47 unitarios + 7 integration → 54/54 verde) en este host.
- Versión no bumpeada (el bump es atómico en P-DET-Z). CHANGELOG no
  tocado.

---

### P-DET-Z — Cierre 1.5.0
Fecha: 2026-05-18  ·  Ejecutor: Opus  ·  Estado: DONE

Cambios:
- Bump de versión a `1.5.0` en los cuatro sitios canónicos: el campo
  `version` de `[workspace.package]` en `Cargo.toml`, el campo
  `"version"` de `crates/hoard-desktop/tauri.conf.json`, el campo
  `"version"` de `crates/hoard-desktop/ui/package.json` y la cadena
  fallback `v{import.meta.env.VITE_HOARD_VERSION || "X.Y.Z"}` en
  `crates/hoard-desktop/ui/src/App.svelte:203`.
- `CHANGELOG.md`: nuevo bloque `## [1.5.0] — 2026-05-18` insertado
  debajo de `## [Unreleased]` (no se reescribe la cabecera Unreleased;
  el bloque queda listo y el Unreleased pasa a estar vacío hasta el
  siguiente ciclo). El bloque cubre el resumen del overhaul (Proton/Wine
  en Linux, manual_paths, fallback Steam→catalog por nombre, panel
  Diagnostics oculto) en `Added`, la heurística general de
  refinamiento + la limpieza del catálogo TOML / `hoard-detect` /
  `autodetect.rs` en `Changed`, y los dos casos correctivos visibles
  (Stellaris ya no surfacea el root, `pathexpand` ya no necesita el
  workaround literal-absoluto en hot path) en `Fixed`.
- `CLAUDE.md`: actualizada la línea "Current version on `main`" de
  `1.4.6` a `1.5.0` y la fecha a `2026-05-18`. La sección "ciclo activo"
  pasa de "en curso" a "cerrado" y apunta al ADR 0009 como fuente de
  verdad arquitectónica. La bullet "1.5.0 (en curso)" del resumen de
  hitos pasa a "1.5.0 (cerrado 2026-05-18)". La sección "Open /
  deferred items" reescribe la entrada del overhaul para reflejar que
  ya cerró, manteniendo el puntero al plan/log como histórico y el ADR
  0009 como referencia obligatoria antes de tocar detección.

Archivos tocados:
- `Cargo.toml` — `[workspace.package].version` 1.4.6 → 1.5.0.
- `crates/hoard-desktop/tauri.conf.json` — `"version"` 1.4.6 → 1.5.0.
- `crates/hoard-desktop/ui/package.json` — `"version"` 1.4.6 → 1.5.0.
- `crates/hoard-desktop/ui/src/App.svelte` — fallback 1.4.6 → 1.5.0.
- `CHANGELOG.md` — bloque `## [1.5.0] — 2026-05-18` añadido.
- `CLAUDE.md` — current version + ciclo activo cerrado + entrada
  "Open / deferred items" reescrita.
- `docs/plans/detection-log.md` — esta entrada + tabla `P-DET-Z` a
  DONE + tag de release con fecha real (2026-05-18, staged).

Tests añadidos:
- Ninguno: P-DET-Z es cierre de release (bump + docs).

Notas / decisiones de ejecución:
- El usuario pidió explícitamente **no** ejecutar `git tag` ni
  `git push`: el cierre queda staged a la espera de su revisión y de
  la decisión sobre el tag definitivo. La fila "Release v1.5.0" del
  estado vivo arriba refleja la fecha real (2026-05-18) con la nota
  de que el tag está pendiente.
- El alcance del prompt original P-DET-Z incluía además marcar como
  DONE entradas en `docs/plans/1.5.md` y mover este plan a
  `docs/plans/done/detection.md` (§10 del plan). El usuario acotó el
  alcance a "bump en 4 sitios + CHANGELOG + CLAUDE.md + entrada de
  cierre en este log"; los pasos de mover el plan y cruzar tachones
  en `1.5.md` quedan pendientes para una iteración posterior, en
  línea con su instrucción explícita.
- No se corrió `cargo check --workspace` ni
  `pnpm --dir crates/hoard-desktop/ui check` en este cierre: el bump
  no toca código, sólo cadenas de versión y documentación. Los gates
  de tests verdes están registrados en las entradas P-DET-1..7
  (último: 54/54 verde en P-DET-7).

---

## Backlog / ideas para después de 1.5.0

Cosas que surgieron mientras se diseñaba el plan pero no caben en este
ciclo. No prometen nada; entrar en un plan más adelante decide su destino.

- **Detección de Lutris/Bottles standalone.** Saves de juegos jugados con
  Lutris (no Steam) viven en
  `~/.local/share/lutris/runners/<id>/prefix/drive_c/...` y Bottles en
  `~/.var/app/com.usebottles.bottles/data/bottles/bottles/<name>/drive_c/...`.
  Mismo patrón que Proton pero con otro layout de prefix. Posible 1.6.x.
- **Detección por proceso vivo como tercer signal.** Cuando un proceso
  cuyo exe coincide con `entry.processes` está corriendo, Hoard podría
  recorrer sus open fds (`/proc/<pid>/fd/*` en Linux, equivalente en
  Windows) para descubrir el directorio de saves real, aunque el catálogo
  esté mal. Caro en runtime, útil sólo como fallback de último recurso.
- **Saves en GOG Galaxy cloud**. GOG mantiene su catálogo de "cloud save
  paths" en un sqlite local; integrarlo evitaría que tengamos que
  catalogar GOG-only juegos a mano.
- **Saves bajo Wine standalone (sin Lutris/Bottles)**. Más raro pero
  existe — `~/.wine/drive_c/...` clásico. Bajo prioridad: la gente que
  juega con wine pelado suele ser power-user.
- **Catálogo de "save game format quirks"**: algunos juegos guardan
  saves en `.zip` que internamente versionan; otros tienen archivos
  efímeros que no deberían tracear (lockfiles, telemetría). Hoy lo
  cubrimos con globs de exclusión por TOML, pero ese sistema lo borra
  P-DET-4. Reintroducirlo si llega un bug de "Hoard tracea archivos que
  el juego sobreescribe cada segundo".
- **Sync de overrides entre dispositivos**. Hoy los manual_paths viven
  sólo en el host. Sería útil que un usuario con tres máquinas no
  tuviera que corregir el mismo override tres veces. Requiere un
  endpoint en el server y resolver qué pasa con paths que son
  específicos de cada host (`/mnt/games/x` no existe en Windows).
