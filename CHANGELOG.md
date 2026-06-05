# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [1.9.12] — 2026-06-05

### Changed
- **Rediseño visual de las pantallas del escritorio.** Replanteo por vista
  conservando el sistema «Obsidian Vault» (paleta emerald, tipografía Geist,
  atmósfera) y sin tocar la lógica ni añadir cadenas nuevas (los 8 locales
  quedan intactos):
  - **Panel**: cada partida muestra un tile con su inicial y anillo emerald,
    jerarquía clara entre nombre y ruta, chips con borde fino y un punto de
    estado del agente que late.
  - **Biblioteca**: tarjetas de juegos monitorizados y detectados sobre
    superficie obsidian con borde fino y hover emerald; barra de escaneo con
    brillo.
  - **Historial**: cabecera con avatar del juego y línea de tiempo de
    versiones en tarjetas de borde fino.
  - **Ajustes**: cabecera editorial y divisores a borde fino.
  - **Cuenta** y **Mapa**: bloques de uso a borde fino y cabeceras editoriales.

## [1.9.11] — 2026-06-05

### Fixed
- **El modo automático no hacía nada hasta apagarlo y encenderlo a mano.** El
  escaneo de arranque solo corría para usuarios self-hosted (`$auth.user`); un
  usuario de Hoard Cloud (login con Gmail, sin servidor propio) tenía la sesión
  en `$cloud.account` y se saltaba el arranque, así que la app abría con el
  toggle en ON sin escanear ni respaldar nada hasta el siguiente intervalo. Ahora
  el arranque vale para cualquiera de las dos sesiones.
- **Parpadeo de «agente apagado» en la nube.** Un barrido de backup que tocaba
  N partidas hacía que Supabase Realtime empujara N `UPDATE` casi simultáneos y
  `cloud_pull::kick()` lanzaba N pulls concurrentes a `/v1/cloud/sync`; al chocar
  refrescando el JWT (token de un solo uso) un timeout transitorio emitía
  `agent://offline`. Añadido un *gate* single-flight: a lo sumo un pull en vuelo
  y una ráfaga de kicks colapsa en un único re-pull.
- **El error «sin instalador» del updater asustaba sin motivo.** Cuando la
  release aún se está compilando en CI los `.deb`/`.msi` no existen todavía;
  ahora el mensaje dice que la versión se está preparando y que reintentes en
  unos 5 minutos en vez de sonar a fallo.

### Changed
- **Tipografía de títulos de vuelta a Geist Sans.** La display serif (Fraunces)
  se veía fina y fuera de sitio en la cabecera; cabeceras y marca vuelven al
  sans con tracking ajustado.
- **Mapa estilo Obsidian.** Se acabó la deriva perpetua: los orbes se quedan
  quietos y solo se mueven para *separarse* cuando se solapan, y luego la
  simulación se duerme. Puedes arrastrar cada juego con el ratón (su rama lo
  sigue). Quitado el botón de **restaurar** del panel lateral (peligroso).
- **El mapa muestra toda la biblioteca.** Además de las partidas rastreadas
  aparecen los juegos detectados en disco como orbes atenuados («Sin rastrear»);
  al pulsarlos saltas a la Biblioteca para añadirlos. Las carátulas de Steam se
  recortan dentro del orbe.

## [1.9.10] — 2026-06-05

### Fixed
- **Saves en `Saved Games` no se detectaban bajo Proton/Wine en Linux.** El
  mapeo de placeholders dentro del prefijo no contemplaba `<winSavedGames>`, así
  que juegos que guardan en `%USERPROFILE%\Saved Games` (p. ej. Planet S) nunca
  se buscaban ahí y la detección caía al stub de Steam Cloud con confianza
  «Baja». Añadido `winSavedGames → drive_c/users/steamuser/Saved Games`.
- **El detalle de un snapshot en la nube dejaba un hueco en blanco.** La nube
  guarda la copia como un único archivo comprimido sin índice por archivo, así
  que la lista salía vacía sin explicación. Ahora muestra un aviso claro.

### Added
- **Mapa de saves en canvas con físicas.** El constelario pasó de SVG reactivo
  (que se atascaba al hacer zoom) a un `<canvas>` con simulación: orbes y nodos
  flotan, se repelen entre sí (force-directed) y se separan solos. Los nodos se
  dibujan a radio constante en pantalla, así que ya no desaparecen al alejar.
  Zoom alrededor del cursor, paneo por arrastre y LOD en las etiquetas.

### Fixed
- **Partida duplicada en el panel sin forma de quitarla** (p. ej. dos `openttd`).
  En la nube el listado iteraba todas las filas locales; como el servidor impone
  unicidad por `(game_slug, label)`, la fila sobrante no era válida. `list_tracked_saves`
  ahora hace auto-saneado: colapsa duplicados quedándose con la fila buena.
- **La foto de perfil seguía sin salir en /cuenta ni en el menú.** La causa real
  no era CSS: el servidor de producción estaba en 1.9.3 (sin la extracción del
  avatar del JWT, añadida en 1.9.6) y el `avatar_url` estaba a NULL en la BD.
  Redesplegado el server y backfill del avatar/nombre desde el metadata de OAuth.

### Changed
- **Rediseño «Obsidian Vault».** Tipografía propia auto-hospedada (Fraunces para
  titulares, Geist Sans/Mono), atmósfera con dos focos esmeralda y grano sutil,
  superficies de cristal con bordes finos. Menos look «plástico/IA».

## [1.9.8] — 2026-06-05

### Added
- **Borrar partidas de la nube para liberar espacio.** Hoard Cloud no tenía
  forma de borrar; intentarlo devolvía «Deleting snapshots isn't supported on
  Hoard Cloud». Nuevo endpoint `DELETE /v1/cloud/saves/:save_id` que purga los
  blobs en R2 y borra la partida (el trigger devuelve el `storage_bytes`). Como
  la nube solo guarda la última versión, borrar elimina la partida entera; la UI
  lo confirma con un diálogo propio y botón rojo.

### Fixed
- **Botón «Mapa» bloqueado en el sidebar.** Faltaba `/map` en la lista de rutas
  habilitadas; quedaba deshabilitado pese a existir la vista.
- **La foto de Gmail no salía junto al usuario en el sidebar.** El avatar no
  llevaba `referrerpolicy="no-referrer"`, y `lh3.googleusercontent.com` responde
  403 si se manda referer. Añadido eso más un fallback a la inicial si la imagen
  falla.

## [1.9.7] — 2026-06-04

### Added
- **Mapa de saves (vista nueva).** Una constelación estilo Obsidian: cada juego
  es un orbe aislado, de él salen ramas (una por partida/`label`) y cada rama es
  una cadena de nodos save (las versiones). SVG puro, sin librerías de grafos,
  layout determinista (filotaxis para separar los orbes, brazos radiales para las
  ramas), zoom/pan con niveles de detalle y panel lateral para restaurar una
  versión o abrir el historial. Solo lectura sobre los datos que ya existen.

### Fixed
- **OpenTTD (y cualquier juego en la nube) se duplicaba al re-trackear.** En modo
  nube `add_game_to_tracking` generaba un UUID nuevo en cada llamada en vez de
  reutilizar el save existente por `(game_slug, label)`; ahora deduplica y solo
  refresca la ruta local.

## [1.9.6] — 2026-06-04

### Fixed
- **Autostart no arrancaba en Linux.** El plugin escribe
  `~/.config/autostart/<app>.desktop` pero no crea el directorio; en un perfil
  XDG limpio `enable()` fallaba en silencio y Hoard nunca arrancaba al iniciar
  sesión. Ahora se crea el directorio antes de habilitar y el autostart se
  activa por defecto al terminar el onboarding.
- **No se podía "olvidar" el servidor local.** `signOut()` no limpiaba el
  estado del wizard, así que la URL del servidor revivía al reabrir la app. Al
  cerrar sesión ahora se borra también el onboarding persistido.
- **Tarjeta "Sin sesión" en Settings para usuarios solo-nube.** La sección de
  servidor self-hosted se mostraba con "Sin sesión" aunque el usuario sólo
  tuviera cuenta en la nube. Ahora se oculta cuando no hay sesión self-hosted y
  se renombra a "Servidor propio".
- **La foto de cuenta nunca aparecía.** El servidor cloud sólo guardaba el email
  en `profiles`, dejando `avatar_url`/`display_name` en NULL. Ahora se extraen
  del `user_metadata` del JWT (avatar/picture + full_name/name) y se persisten
  con COALESCE, y la página de cuenta los pinta (con inicial de fallback).
- **URL de documentación incorrecta** en `hoard-server.service`
  (`insider/hoard` → `rleeon/hoard`).

### Changed
- **Toasts idénticos se fusionan** en uno solo con contador `×N` en vez de
  apilar cientos, para que un trabajo de fondo que falla en bucle (p. ej. un
  servidor caído reintentando) no entierre la UI.
- **Intervalo de comprobación de la nube hasta 2 s en Pro.** El piso baja de 5 s
  a 2 s para cuentas de pago (el slider expone ese extremo sólo a Pro; gratis
  sigue en 5 s). La ventana de ancho de banda del servidor es el límite real
  contra saturación.

## [1.9.5] — 2026-06-04

### Fixed
- **Los saves bajados de la nube se marcaban siempre como más recientes que los
  locales.** La extracción del `.tar.zst` escribía cada archivo con
  `File::create`, que estampa `mtime=ahora`, sin reaplicar el mtime original que
  el header del tar conserva. El diff consciente de conflictos del auto-restore
  (`local_mtime_wins`) comparaba ese mtime falseado y el remoto ganaba siempre,
  pisando progreso local más nuevo. Ahora se reaplica el mtime de origen al
  extraer, en las dos rutas (self-hosted y cloud).
- **`refresh_token_already_used` (400) tras horas de uso forzaba re-login.**
  Cuando el reuse-detection de Supabase rechazaba un refresh token ya rotado por
  otro flight (poller + realtime + acción del usuario coincidiendo), la sesión
  caía. Ahora ese caso se detecta y se recupera releyendo las credenciales de
  disco: si otro flight ya rotó y persistió un token distinto, se adopta en vez
  de tirar la sesión.
- **Un id numérico de cuenta se usaba como nombre de juego.** En rutas tipo
  `…/Gaddy Games/Plan B Terraform/<steamid64>/saves` la atribución catalog-free
  cogía el SteamID (17 dígitos) como nombre. Ahora salta los segmentos
  puramente numéricos (ids de cuenta/perfil) y sigue subiendo hasta el título.

### Added
- **Push Realtime de Supabase para sync casi-instantáneo entre dispositivos.**
  Un WebSocket suscrito a `saves` dispara un pull off-cadence en cuanto otro
  dispositivo sube una versión, bajando la latencia percibida de hasta un
  intervalo de poll (10 s) a ~1 s. Es un acelerador best-effort: el poll
  temporizado sigue siendo el fallback si el socket cae.
- **Barrera de sync pre-lanzamiento.** Al arrancar un juego, el agente tira del
  último snapshot remoto antes de que empiece a escribir, para que un relevo
  entre dispositivos (juega en uno, te sientas en otro y lanzas) tenga el
  progreso ya presente. Respeta todas las guardas "el usuario está aquí"
  (cambios sin volcar, evento fs reciente, restore en curso, cooldown) y usa el
  mismo restore consciente de conflictos.

### Changed
- **Backoff largo para auto-restores que dan 404.** Un save trackeado en local
  pero ausente del backend actual (arrastrado de otra cuenta, `state.json`
  obsoleto) se reintentaba cada 60 s para siempre, inundando el log de WARNs.
  Ahora se espacia a ~1 h y baja a nivel DEBUG; se auto-cura si el save aparece.

## [1.9.4] — 2026-06-04

### Fixed
- **El descubrimiento catalog-free (fase 4, ADR 0020) sacaba juegos fantasma
  de los propios datos de Hoard.** El walk recorría `~/.local/share` entera,
  incluyendo `<state_dir>/hoard/conflicts/<id>/<ts>/autosave` (backups del
  auto-restore consciente de conflictos, que son bytes de save copiados
  verbatim) y la papelera (`~/.local/share/Trash/...`). Ambos puntuaban
  save-like y afloraban como "juegos" nombrados con el timestamp del backup o
  con `files`. Ahora el walk salta cualquier ruta dentro del `state_dir` de
  Hoard o con componente `Trash`/`.Trash`, tanto en fase 4 como en el
  descubrimiento agresivo por slug.

## [1.9.3] — 2026-06-04

### Fixed
- **`/account` en la web mostraba todo a cero (espacio usado, dispositivos).**
  El cliente leía campos que el servidor no manda (`storage_bytes`,
  `devices_count`, `plan_renews_at`); la forma real de `/v1/me` es
  `storage_used_bytes` / `devices_used` / `renews_at` + sus límites. Corregido
  el mapeo y, de paso, la página usa ahora los límites que devuelve el servidor
  (no los locales) para el plan, el almacenamiento y el tope de dispositivos.

### Added
- **Listado y desvinculación de dispositivos en cloud.** Nuevos endpoints
  `GET /v1/devices` y `DELETE /v1/devices/:id` (ambos con alcance al usuario del
  JWT). La página de cuenta ya los llamaba pero no existían en el servidor, así
  que la lista salía vacía. Al desvincular se recalcula `profiles.devices_count`.
- **Flujo de compra de Pro con Polar.** El botón "Comprar Pro" ya no apunta a un
  enlace muerto: lleva a una pantalla de confirmación (`/checkout`). Si no hay
  sesión, manda a login y vuelve a la compra (no a la cuenta). Si la hay,
  pregunta si es la cuenta correcta — verde "Continuar" a la derecha, negro
  "Cambiar de cuenta" a la izquierda (va a `/account` al ancla de la zona de
  sesión/borrado). "Continuar" crea la sesión de checkout en el servidor
  (`POST /v1/cloud/checkout`) con el `access_token` de Polar, estampando
  `metadata.user_id` y `external_customer_id` desde el JWT — imposible falsear el
  usuario — y redirige al checkout alojado de Polar. El webhook ya existente lee
  ese `user_id` para activar el plan.

## [1.9.2] — 2026-06-04

### Fixed
- **Contador de dispositivos clavado en "0 / N".** El registro de dispositivos
  nunca se implementó en el servidor cloud: la tabla `devices` solo se leía y
  `profiles.devices_count` jamás se escribía, así que la cuenta marcaba 0
  aunque estuvieras usando la app. Ahora el cliente declara su identidad
  (fingerprint estable = hash de `/etc/machine-id` + hostname, ya usado para los
  logs) en cabeceras al pedir `/v1/me`, y el servidor hace upsert en `devices`
  (clave `(user_id, fingerprint)`, refresca `last_seen_at`) y recalcula
  `devices_count`. El límite del plan no se aplica todavía aquí — solo se mantiene
  el conteo veraz, para que un descuadre nunca pueda bloquear al usuario.

## [1.9.1] — 2026-06-04

### Fixed
- **Snapshots cloud que mostraban "0 archivos" junto a un tamaño no nulo.** La
  subida cloud empaqueta el save en un `tar.zst` opaco y el protocolo nunca le
  decía al servidor cuántos archivos contenía, así que la fila de versión
  quedaba con `file_count = 0` y el Historial lo pintaba tal cual. Añadida la
  columna `file_count` a `save_versions` (migración Postgres 0018), `file_count`
  al `UploadInit`, y `latest_file_count` al manifest de `/v1/cloud/sync`; el
  cliente ya lo declara y el Historial lo muestra. Los snapshots subidos antes
  del arreglo se quedan en 0 (no se rellena desde R2). El dato siempre estuvo
  intacto: solo era el contador.

## [1.9.0] — 2026-06-04

### Added
- **Detección catalog-free por correlación (ADR 0020, fases 3+4 cerradas).**
  El bucle de correlación proceso↔escritura ya influye en el descubrimiento:
  `classify_dir_as_save_like` suma el bonus de +0.50 cuando el
  `CorrelationStore` corrobora una carpeta, y desbloquea `Confidence::High`
  para los dirs atribuidos a un proceso de juego (antes tope en `Medium`). La
  nueva fase 4 (`discover_unattributed`) recorre los roots de usuario una sola
  vez, puntúa con correlación y aflora saves que ningún catálogo/Steam
  reclamaba (nombres GUID, idiomas no ingleses, indies fuera de Ludusavi),
  atribuyendo cada uno a un juego por el proceso que lo escribió. Con verja de
  precisión: en roots amplios un match débil sólo-por-nombre no crea juegos
  fantasma.

## [1.8.7] — 2026-06-04

### Fixed
- **Sesión cloud que se invalidaba sola (`refresh_token_not_found`).** Varias
  llamadas autenticadas podían renovar el token de Supabase a la vez; como
  GoTrue rota el refresh token en cada uso y detecta reutilización, la carrera
  revocaba toda la familia de tokens y dejaba la cuenta caída hasta volver a
  iniciar sesión. `refresh_active_session` ahora es single-flight con una
  ventana de reutilización de 30 s, así que renovaciones concurrentes comparten
  el mismo resultado en vez de pisarse.
- **Update en Linux que no surtía efecto hasta reabrir.** Tras `dpkg -i`, el
  binario se reemplaza y `std::env::current_exe()` pasa a resolver
  `.../hoard-desktop (deleted)`, por lo que el relanzado fallaba en silencio.
  Ahora capturamos la ruta del ejecutable **antes** de instalar y saneamos el
  sufijo `" (deleted)"`, de modo que el relanzado arranca el binario recién
  instalado.
- **Dropdowns en blanco en Linux (selector de idioma y filtro de nivel de
  logs).** webkit2gtk renderiza los `<select>` y su popup con GTK e ignora el
  tema oscuro, dejándolos claro-sobre-claro. Fijado `color-scheme: dark` y
  colores explícitos de texto/fondo en `select`/`option`.

## [1.8.6] — 2026-06-03

### Added
- **Detección dirigida por señales (ADR 0020, fases 0/1/3).** Primer paso para
  invertir el pipeline catalog-first hacia descubrimiento automático con el
  catálogo como red de seguridad. El walk agresivo ya no clasifica carpetas con
  un booleano name-only: ahora puntúa cada candidata con un score multi-señal
  `S ∈ [0,1]` (`scoring.rs`) que combina nombre (vocabulario multilingüe),
  contenido (extensiones save fuertes/débiles/ruidosas, imágenes), recencia y
  señales negativas (config/cache/screenshots). Cutoffs `S ≥ 0.60` confirmado /
  `0.35 ≤ S < 0.60` posible / `< 0.35` descartado. Añadidos los roots de
  descubrimiento por SO (`roots.rs`) y, la pieza clave, la **correlación
  proceso↔escritura** (`correlation.rs`, +0.50): cuando un save vigilado se
  reescribe, el agente muestrea los procesos de juego vivos y persiste la
  correlación en `correlation.json`, sentando la base para descubrir y atribuir
  saves de nombre opaco (GUID) que el name-signal jamás atraparía (~6% de recall
  solo por nombre, medido sobre el manifest). La integración completa
  (observador sobre roots amplios + aprendizaje) llega en fases posteriores.
- **DAG de versiones (base para sync tipo git entre dispositivos).** Cada
  snapshot/versión registra ahora su `parent_version` (la versión de la que
  desciende; `NULL` = raíz), convirtiendo el log lineal `version_num` en un
  grafo. Con ello el servidor detecta un *push divergente* (non-fast-forward):
  si el cliente declara una `base_version` que ya no es el head —porque otro
  dispositivo subió entre medias— la subida se rechaza con `409
  non_fast_forward` (con `head_version` para reconciliar) en lugar de pisar la
  otra línea con last-writer-wins. Implementado en ambos backends (SQLite
  self-hosted en `routes/snapshots.rs`, Postgres cloud en
  `cloud/routes/saves.rs`), expuesto en las respuestas (`parent_version` en
  el resumen de snapshot y `latest_parent_version` en el manifest
  `/v1/cloud/sync`). El cliente CLI ya manda su base (`last_version_num`); el
  auto-path del agente registra el parent pero todavía no envía base (la
  resolución de conflicto con "guardar ambas" llega después). Migraciones
  `0015_snapshot_parent.sql` y `postgres/0017_save_version_parent.sql`.

## [1.8.5] — 2026-06-02

### Added
- **Protocolo de subida a Hoard Cloud en el cliente.** El cliente desktop/CLI
  ahora habla el protocolo `/v1/cloud/*` de `api.hoard.services` (init →
  PUT presignado a R2 → commit) además del self-hosted, ramificando según
  `/v1/health` `mode`. Antes apuntaba el cliente self-hosted contra la API
  cloud, que no tiene `/v1/saves`, y daba 404 ("doesn't look like a Hoard
  server") al monitorizar juegos con la cuenta de Gmail. Subida, restauración
  (descarga presignada + verificación sha256 del archivo completo), historial
  (vía manifest `/v1/cloud/sync`) y alta de juegos funcionan ya en modo cloud.

## [1.8.4] — 2026-06-02

### Changed
- **Onboarding: nuevo paso "cómo quieres entrar" antes de pedir el servidor.**
  La bienvenida ya no lleva directo a meter la URL de un servidor self-hosted.
  Ahora aparece un selector con dos opciones: **Autoalojado** (conéctate a tu
  propio servidor) y **Entrar con Google** (Hoard Cloud, sin configurar nada).
  La pantalla de URL del servidor solo sale si eliges autoalojado.

### Fixed
- **Los usuarios solo-cloud (Gmail) ya aterrizan en la app al reabrirla.** El
  arranque solo comprobaba la sesión self-hosted (`$auth.user`), así que una
  sesión cloud sin servidor propio volvía al asistente de onboarding en cada
  inicio. Ahora, sin sesión self-hosted pero con cuenta cloud, se va directo a
  `/account`.
- **Corregido "tu datos" → "tus datos"** en la bienvenida (ES). El tagline
  reutilizaba una sola palabra "tu" para "tu servidor" y "tu datos"; ahora es
  una frase única y gramaticalmente correcta en los 8 idiomas.

## [1.8.3] — 2026-06-02

### Fixed
- **El "actualizar servidor" desde la app ya funciona.** La firma de releases
  (ADR 0017) estaba a medias: `upgrade.rs` llevaba una clave pública
  *placeholder* y el workflow no firmaba porque faltaba el secret
  `MINISIGN_SECRET_KEY`. Resultado: todo release salía sin `.minisig` y el
  verificador —que falla cerrado— rechazaba instalar el binario ("release is
  missing the signature asset"). Ahora hay una clave minisign real
  (`F648761D67BD389E`), su secreta vive en los secrets de Actions y cada
  `hoard-*-linux-x86_64.tar.gz` se firma en CI. Un test nuevo valida que la
  clave embebida parsea, para no volver a publicar un binario que no puede
  verificar nada.

### Fixed
- **La cuenta de Hoard Cloud ya no "caduca" al reiniciar el PC.** El access
  token de Supabase es un JWT de vida corta (~1 h) y nunca se renovaba, así que
  cualquier llamada posterior a su expiración —el botón "Refrescar" (`/v1/me`) o
  el poller (`/v1/cloud/sync`)— recibía un 401 y parecía sesión muerta, aunque
  el refresh token seguía válido sin usarse. Ahora, ante un 401, se canjea el
  refresh token contra Supabase GoTrue para obtener un par nuevo, se persisten
  los tokens rotados y se reintenta una vez. Esto arregla tanto el aviso de
  "sesión caducada, inicia de nuevo" como el puntito "Servidor caído" en la
  barra lateral cuando había sesión cloud activa.
- **La página de descargas web ya no muestra una versión congelada.** La
  versión y la fecha estaban hardcodeadas a `1.7.0 · 2026-05-20`; ahora se
  inyectan en build desde `Cargo.toml` y la entrada datada más reciente del
  `CHANGELOG.md`, alineadas con el resto del sitio.
- **El epígrafe "Download" de `/download` estaba en inglés** aunque el resto de
  la página estuviera en español; ahora usa la clave i18n `nav.download`.
- **Login de Hoard Cloud funciona con navegadores en sandbox (snap/flatpak).**
  El handoff por esquema `hoard://` se perdía silenciosamente con el Firefox de
  snap (el navegador por defecto en Ubuntu): un navegador confinado no puede
  despachar esquemas custom al host, así que el botón "vuelve a Hoard" no hacía
  nada. Ahora el flujo usa un **redirect loopback** (RFC 8252): la app levanta
  un listener efímero en `http://127.0.0.1:<puerto>`, pasa el puerto al flujo
  web (`/login?desktop=1&port=N`) y el callback rebota los tokens a esa URL —
  que los navegadores confinados sí abren. El esquema `hoard://` queda como
  fallback (navegadores no confinados y macOS). Reusa el mismo camino interno
  `deep-link://new-url`, así que el resto del login no cambia.

### Changed
- **Skip-by-hash de backup ahora confirma por contenido antes de saltar.** La
  firma de set pasa a ser un compuesto `<barato>:<contenido>`: el camino rápido
  sigue siendo `(ruta, tamaño, mtime)` sin leer bytes, pero cuando esa firma
  cambia (juegos/daemons que reescriben saves por temporizador, bumpeando
  mtime sin tocar bytes) se computa una firma de contenido y, si coincide, se
  refresca el compuesto sin crear snapshot redundante (`BackupResult::Unchanged`).

## [1.8.1] — 2026-05-31

### Fixed
- **Login de Hoard Cloud llega a la app en Linux/Windows (handoff robusto).**
  Se endurece el camino del deep link de OAuth de extremo a extremo: (1) el
  `hoard://auth/callback` lleva los tokens en la query en vez del fragment
  (`#`), que los handlers de esquema de Linux/Windows suelen descartar; (2)
  Rust bufferiza la URL capturada en arranque en frío (llega como argumento de
  lanzamiento antes de que el webview monte su listener) y el frontend la drena
  al montar vía `cloud_take_pending_deep_link`; (3) se escanea `argv` también
  en `setup()` (primera instancia, donde el callback de single-instance no
  dispara) además del callback de single-instance (app ya abierta) y
  `on_open_url` (macOS). El buffer se limpia tras un login exitoso y el
  frontend deduplica evento vs. drenado.
- **Detección de nuevas versiones del desktop sin sesión de servidor.** El
  chequeo de actualizaciones estaba acoplado a la sesión self-hosted, así que
  con solo sesión de Hoard Cloud (o sin sesión) nunca aparecía el aviso de
  actualizar la app — había que abrir Ajustes y forzarlo. Ahora se prueba
  siempre al arrancar y el poller corre toda la sesión.
- **"Mejorar plan" ya no da 404.** Apuntaba a `https://hoard.services/upgrade`,
  ruta inexistente; ahora abre `/pricing` (donde están los botones de checkout).
  "Gestionar facturación" abre `/account` en vez del inexistente `/billing`.
- **Botón "Actualizar" de la cabecera de Cuenta renombrado a "Refrescar"** (ES)
  para no confundirse con actualizar la app; solo recarga los datos del plan.

## [1.8.0] — 2026-05-31

### Added
- **Ingesta adaptativa por forma del save** (ADR 0019). El cliente sube por
  streaming (sin cargar el snapshot entero en RAM) y elige transporte según la
  forma: por-archivo (normal), tar empaquetado en un campo `pack` cuando hay
  más de 500 archivos (sube el cap a 50 000), o archivo entero para los
  monolíticos. Restore también vuelca a disco por streaming.
- **Chunking content-defined server-side** para archivos grandes (> 128 MiB).
  Un save monolítico que reescribe unos pocos KB por versión ya sólo almacena
  el delta: el server lo trocea con un CDC gear-hash propio y deduplica por
  chunk (tablas `chunks` / `snapshot_file_chunks`, GC y cuota uniformes con los
  blobs). La descarga reensambla de forma transparente; clientes viejos reciben
  el mismo `tar.zst`.
- **Skip-by-hash de conjunto.** Antes de subir, si la firma barata
  `(rel_path, size, mtime)` del directorio coincide con la del último backup, no
  se crea versión (cubre settles espurios del watcher).
- **Detección de Steam Cloud.** Los juegos cuyo save vive en
  `userdata/<id>/<appid>/remote/` ahora se detectan; las plantillas Ludusavi con
  `<storeUserId>`/`<gameId>` se expanden sobre los usuarios Steam descubiertos.

### Fixed
- Plantilla de ruta literal absoluta se preserva tal cual (ya no se convertía en
  relativa).
- **Login OAuth llega a la app aunque ya esté abierta (Linux/Windows).** El
  `hoard://auth/callback#…` que el SO entrega como argumento de un segundo
  lanzamiento se reenvía ahora por el evento `deep-link://new-url`; antes se
  perdía en silencio (el handler `on_open_url` solo dispara en arranque en
  frío/macOS) y la sesión nunca llegaba.
- **Página de callback web ya no se queda con el spinner congelado.** Tras ceder
  el control al handler `hoard://` del SO se muestra un estado de éxito con
  checkmark estático y un enlace para reabrir la app manualmente.
- **La web refleja la versión real.** El número sale del `Cargo.toml` del
  workspace en tiempo de build (`__HOARD_VERSION__`) en vez de un literal
  desactualizado.
- **Cuenta sin botón "Atrás" redundante** ahora que es la pestaña principal de
  inicio.

## [1.7.2] — 2026-05-31

### Changed
- **Barra lateral reorganizada.** Se quita "Historial" (duplicaba el Panel)
  y se añade arriba de Biblioteca un botón de cuenta: "Iniciar sesión" sin
  sesión cloud y "Inicio" (cuenta y estado del plan, solo lectura) cuando hay
  sesión. La línea de tiempo por partida sigue accesible desde el Panel y la
  Biblioteca (`/history/:id`).

### Fixed
- **Conectar a tu servidor acepta un host pelado.** El campo antepone
  `https://` automáticamente, así que ya no exige escribir el esquema; un
  texto con espacios (un nombre amigable) sigue rechazándose con un aviso
  claro en vez de un error de red confuso.
- **Salir del asistente de servidor con sesión activa.** "Atrás" devuelve a
  la app en vez de atrapar al usuario en el flujo de onboarding.
- **Actualización del servidor en Hoard Cloud.** Se oculta el panel de
  actualización cuando la conexión apunta al backend gestionado
  (`*.hoard.services` / `*.fly.dev`): no expone `/v1/admin/upgrade`, lo que
  causaba el "HTTP 404 Not Found" al pulsar el botón en Windows. Nuevo flag
  `is_cloud_server` clasificado en el login.

## [1.7.1] — 2026-05-31

### Added
- **Webhooks de Polar (Merchant of Record).** Alternativa a Lemon Squeezy:
  endpoint `POST /v1/webhooks/polar` con verificación de firma Standard
  Webhooks (HMAC-SHA256 sobre `{id}.{ts}.{body}`), mapeo de eventos
  `subscription.*` al enum de estado y cascada a `profiles.plan`. Ambos
  proveedores conviven. Producto y plan se resuelven por `product_id` desde
  `[cloud.polar]`.
- **CORS en el servidor cloud.** `CorsLayer` permite a `hoard.services` (y
  localhost en dev) leer la API cross-origin con Bearer token; sin esto el
  navegador bloqueaba la respuesta y la web mostraba "degradado" en falso.
- **Estado de servicio real de 3 estados en la web.** El punto de estado
  distingue ok (verde), degradado (ámbar) y caído (rojo) en vez del binario
  anterior. `/v1/health` ahora hace un ping a Postgres (`SELECT 1`, timeout
  2 s) y reporta `degraded` si la DB falla.
- **Cuenta en la barra lateral del desktop.** Sesión, avatar y botón
  "Mejorar plan".
- **Almacenamiento content-addressed con deduplicación** (ADR 0018, eje C).
  Los bytes de cada archivo se guardan una sola vez por usuario en
  `blobs/<user>/<sha[0:2]>/<sha>`; una versión pasa a ser sólo su lista de
  `snapshot_files` apuntando a blobs por sha256. Archivos idénticos entre
  versiones comparten un blob, con `refcount` y GC cuando llega a 0. La cuota
  (`storage_used_bytes`) pasa a contar bytes de blobs únicos, así que un
  re-subido casi-idéntico (el caso OpenTTD: 16 autosaves donde sólo cambian
  1-2) apenas suma. Migración `0013_blobs.sql` + `hoard-server/src/blobs.rs`,
  con backfill idempotente al arrancar que convierte los `v<n>/` y
  `trash/<id>/` legacy a blobs y recalcula la cuota. Dedup por usuario (sin
  cruce entre cuentas). El `download` reconstruye el mismo tar.zst desde
  blobs; soft-delete/restore pasan a ser puramente lógicos.
- **Poda de snapshots ponderada por antigüedad** (ADR 0018, eje B). El
  cleanup horario del server adelgaza versiones redundantes con un esquema
  GFS (grandfather-father-son): conserva densas las recientes y dispersas las
  viejas (1 por hora/día/semana). Las pinned y la última versión nunca se
  podan; lo podado va a la papelera (recuperable, se purga por
  `trash_retention_days`). Configurable en `[retention]`:
  `snapshot_pruning` (default on) y `data_saving` (knob 0..1, default 0.3).
  Nuevo módulo `hoard-server/src/retention.rs` con la lógica pura testeada.
- **Barra "Ahorro de datos" en Settings** (ADR 0018, eje A). Un único knob
  `data_saving ∈ [0,1]` (izq "Guardar todo" → der "Máximo ahorro", default
  0.3) que pone un suelo entre snapshots por save: el cliente espacía las
  subidas de 5 s hasta 10 min según la barra
  (`min_snapshot_interval = lerp(k, 5s, 600s)`), coalescendo los cambios
  intermedios sin perder el estado final. Mata la cadencia de "una versión por
  minuto" del autosave (caso OpenTTD). Persistido en `prefs.json`
  (`data_saving`), slider en los 8 locales. La misma barra escala la retención
  GFS del server.

### Fixed
- **Auto-restore ya no se dispara mientras juegas.** El guard de "usuario
  jugando" del sweep de auto-restore dependía de `is_running` (falla cuando
  el nombre de proceso no casa con el manifest, p.ej. OpenTTD) y del mtime
  del directorio (no cambia en reescrituras in-place). Ahora gatea con la
  actividad real del watcher (`has_pending` + `last_fs_event_at`), inmune a
  ambos fallos. Evita que el Modo Automático reintroduzca autosaves rotados
  encima de una partida activa. Refina ADR 0014 §3.
- **Feed de actividad: fin del flood de "en cola — esperando…".**
  `schedule_backup` re-emitía `BackupScheduled` en cada escritura, dejando
  filas huérfanas que nunca resolvían. Ahora solo emite en el flanco de
  subida; la fila se cierra cuando la subida completa.
- **Backups ya no se quedan en cola indefinidamente.** Un juego que escribe
  cada segundo reiniciaba el debounce de 5 s eternamente y la subida nunca
  vencía. Nuevo tope `MAX_BACKUP_WAIT_SECS` (30 s) fuerza la subida aunque
  las escrituras no paren.
- **Navegación SPA de la web.** Los enlaces (`/help`, etc.) no cambiaban de
  vista hasta recargar: la View Transition esperaba a `navigation.complete`
  antes de resolver, bloqueando el cambio. Reordenado para resolver primero.

### Docs
- ADR 0018 + plan `storage-efficiency.md`: rediseño de almacenamiento
  (dedup content-addressed + poda por antigüedad + barra "ahorro de datos")
  motivado por el caso OpenTTD (33 versiones ≈ 53 MB para ~5 MB únicos).
  Fase 1 implementada; fases 2-3 pendientes.

## [1.7.0] — 2026-05-26

Modo Automático sale del fondo del escritorio y pasa a ser visible.
Dos componentes nuevos en la UI — un indicador en vivo en la sidebar
y un panel de actividad flotante — leen un bus de eventos `agent://*`
estable y muestran qué está vigilando el watcher, qué sube o baja en
ese momento, y si la cuota o la red rompen algo. El cloud poll se
desacopla del scheduler horario pesado y corre en su propia cadencia
configurable (default 10 s) sin tocar disco.

### Added
- `LiveStatus.svelte`: widget de dos puntos en el footer del
  sidebar. Uno cubre el estado del watcher (watching / off /
  unknown) y otro el del cloud poll (online / throttled / offline /
  unknown). Tooltip granular con el conteo de saves seguidos.
- `ActivityFeed.svelte`: panel flotante abajo-derecha (toggle por
  botón `ScrollText` en el header o desde Settings) con las últimas
  ~50 entradas: watcher armado, juego iniciado/parado, subida en
  curso/completada/fallida, auto-restore, versiones nuevas en
  cloud, throttled, quota_reached, offline/online. Timestamps
  relativos refrescados cada segundo.
- `lib/stores/live.ts`: single source of truth para la UI viva.
  Suscribe a todos los topics `agent://*` al montar `App.svelte` y
  desuscribe al destruir. Ring buffer FIFO acotado a 80 entradas;
  `seenArmed: Set<save_id>` dedupe el primer `watcher-armed` por
  save tras re-armado del watcher.
- `crates/hoard-desktop/src/commands/cloud_pull.rs`: poller dedicado
  al cloud, cadencia `cloud_poll_interval_secs` (default 10 s,
  slider 5..=300 s en Settings). Un GET ligero al manifest del user
  por tick; compara `(save_id, version)` con la última seeded y
  emite `agent://cloud-pull-completed` con `new_versions`. **No
  descarga nada** — sólo notifica; el scheduler horario sigue
  gobernando los restores reales. El primer poll tras login se
  considera seeding y no cuenta como "nuevas versiones" (evita
  spam de notificaciones al iniciar sesión).
- Topics `agent://*` nuevos: `watcher-armed`, `upload-started`,
  `upload-completed`, `cloud-pull-started`, `cloud-pull-completed`,
  `quota-reached`, `throttled`, `offline`. Los topics legacy
  (`backup-started`, `backup-success`, etc.) coexisten para no
  romper consumers existentes.
- Pref `cloud_poll_interval_secs` (5..=300, default 10) y
  `live_activity_visible` (default true). Settings → Cloud expone
  ambas: slider del intervalo de poll y toggle del panel.
- 36 nuevas claves i18n mirroreadas en los 8 locales:
  `settings.cloud_poll_*`, `settings.live_activity_*`, `status.*`,
  `activity.*`.
- ADR [0016](docs/decisions/0016-live-status-and-dual-cadence.md):
  contrato del bus `agent://*`, store derivado, cadencia dual
  (scheduler horario + cloud poll de 10 s), razón por la que el
  poller no descarga.

### Changed
- `commands/agent.rs` re-emite los `AgentEvent` también con los
  topics nuevos (`upload-started`, `upload-completed`, `throttled`)
  además de los legacy. `BackupScheduled { reason:
  FilesystemSettled }` se sirve también como `agent://throttled`.
- `App.svelte` arranca el poller con `subscribeLive()` después de
  `hydratePrefs` y lo apaga en `onDestroy`. Nuevo botón
  `ScrollText` en el header al lado del icono de actualización.
- Fallback de versión en `App.svelte`: `"1.6.1"` → `"1.7.0"`.

## [1.6.1] — 2026-05-25

Hoard Cloud entra en escena con dos planes (Free y Pro), tope por
partida, ancho de banda con ventana móvil de 15 min, y modo ahorro
para usar un dispositivo como caja fuerte de subida pura. El
self-hosted no se toca: todo el cloud feature vive detrás del flag
`--features cloud` del servidor, y el cliente Tauri sigue hablando
con `hoard-server` igual que antes.

### Added
- Plan Free 1.6.1: 1 GB de almacenamiento, 3 dispositivos, partidas
  ilimitadas, historial de versiones para siempre, hasta 200 MB por
  partida, 500 MB cada 15 min de ancho de banda.
- Plan Pro 1.6.1: 50 GB, dispositivos ilimitados, partidas
  ilimitadas, historial para siempre, hasta 2 GB por partida, 1 GB
  cada 15 min de ancho de banda. 4,49 €/mes o 35,99 €/año.
- `crates/hoard-server/src/cloud/bandwidth.rs`: módulo nuevo con
  contador minute-bucketed (`bandwidth_usage` table) que suma sobre
  la ventana del plan para enforzar el quota. `check()` antes del
  PUT presignado, `record()` después del commit, `cleanup_old()`
  como tarea en `tokio::spawn` cada 10 min borrando buckets >1h.
  Falla abierto si la DB tiene un mal momento — preferimos servir
  bytes que bloquear clientes pagos por un timeout en Postgres.
- 413 estructurado `save_too_large` en `POST /v1/cloud/saves`
  cuando `size_bytes > limits.max_save_size_bytes`. El cliente
  recibe el quota y los bytes pedidos para mostrar un toast claro.
- 429 estructurado `bandwidth_limit_exceeded` con header
  `Retry-After` calculado a partir del primer bucket en salir de
  la ventana de 15 min.
- Flag `backup_only` per-save: cuando una partida sube con este
  flag, el servidor la oculta del manifest que devuelve
  `/v1/cloud/sync` a los demás dispositivos. Sigue siendo
  descargable por id directo y sigue acumulando versiones para
  este dispositivo. Vivirá con un toggle por tarjeta en la
  Library cuando aterrice el cliente cloud nativo (1.7.0).
- Pref global `cloud_savings_mode` (default `false`) más una
  sección "Hoard Cloud" en Settings que sólo se renderiza para
  usuarios cloud-signed-in. Cuando está activo, cada nueva subida
  hereda `backup_only = true` para este dispositivo — pensado para
  un portátil que tratas como caja fuerte.
- Account.svelte gana tres tarjetas (history, max_save_size,
  bandwidth) que reemplazan la antigua de retention.
- Helper `lib/utils/cloudErrors.ts` que parsea las respuestas
  413/429/402 del servidor y las traduce a strings localizados
  para toasts.
- Migración Postgres `0014_simplify_plans_and_backup_only_and_bandwidth.sql`
  aplicada contra Supabase prod (project `zddepgqdiuhhzqdimsks`):
  migra filas `proplus` → `pro`, añade columna `saves.backup_only`
  con índice parcial, crea tabla `bandwidth_usage` con RLS.
- Hero, FAQ, CTA y pricing de la landing (`web/`) reescritos para
  la línea 1.6.1: dos planes lado a lado, "Forever history" como
  bullet propio, dos FAQ nuevas (per-save size + bandwidth).

### Changed
- `Plan` enum del servidor pasa a `{ Free, Pro }`. `from_str`
  grandfathea `"proplus"`, `"pro+"` y `"pro_plus"` → `Pro` para
  que cualquier fila o JWT cacheado siga deserializando.
- `PlanLimits` reemplaza `retention_days` por
  `version_history_forever` y añade `max_save_size_bytes`,
  `bandwidth_window_secs`, `bandwidth_quota_bytes`.
- `Me` struct (`/v1/me`) y `CloudAccount` (Tauri command +
  TS type) reflejan el nuevo shape. Los locales se sincronizan
  para los 8 idiomas (en, es, de, fr, it, ja, pt, zh).
- `UpgradePlanModal` colapsa a dos columnas; precio Pro
  formateado con 2 decimales para "4,49 €".
- `planLabel` en el desktop store dobla `"proplus"` → "Pro" para
  cualquier JSON cacheado de una sesión anterior.

### Removed
- Plan Pro+ (tier 9,99 €). Los suscriptores actuales se migran a
  Pro vía la migración 0014; el JWT y el webhook handler aceptan
  los tokens viejos por compatibilidad.
- `retention_days` en todos los planes (Pro y Free ahora son
  "forever"). El campo desaparece del JSON de `/v1/me`.

## [1.5.5] — 2026-05-20

Modo Automático endurecido: resolución de conflictos por mtime con
backup local antes de sobrescribir, skip mientras juegas, tick
inmediato al activar, sliders configurables en Settings, y smoke
test de i18n.

### Added
- Conflict-aware restore: si los bytes locales y remotos difieren,
  el más nuevo (por mtime) gana. Si gana el remoto, la copia local
  se mueve a `~/.local/share/hoard/conflicts/<save>/<ts>/` antes de
  sobrescribir. TTL configurable (default 14 días) limpia la
  carpeta en cada tick.
- Sliders en Settings → Modo Automático: "Intervalo de escaneo"
  (1-24h) y "Conservar copias de conflicto" (1-30d). Los valores
  se persisten en `prefs.json` y se aplican al instante.
- Toast al activar/desactivar Modo Automático.
- Toast informativo cuando se guardan copias locales por conflicto,
  con la ruta de la carpeta.
- `AgentEvent::SaveConflictsBackedUp` y comandos
  `set_scheduler_interval` / `set_conflict_retention`.
- Smoke test `pnpm i18n:check` que verifica que los 8 locales
  tienen las mismas claves que `en.json`.
- ADR [0014](docs/decisions/0014-conflict-aware-restore-and-game-activity-skip.md)
  documenta las 4 decisiones (mtime+backup, TTL 14d, skip al jugar,
  tick inmediato).

### Changed
- `restore_files_into` ya no asume "local always wins" — compara
  mtime y decide caso por caso. Supersede en parte la regla de
  ADR 0013.
- `AutoRestoreOutcome` reemplaza `files_conflicts` por
  `conflicts_local_wins` + `conflicts_backed_up` + `conflict_dir`.
- `AutomaticScheduler::start` emite el primer `automatic-tick`
  inmediatamente (antes consumía el primer tick para evitar fire
  instantáneo).
- `sweep_for_auto_restore` y `handle_add` saltan la restauración
  si el juego está corriendo (`slot.is_running`) o, sin match de
  proceso, si el directorio fue tocado en los últimos 5 minutos.
- PT locale: rename de "Copiar"/"A copiar" → "Enviar"/"A enviar"
  en strings de acción direccional (sustantivo "cópia" intacto).

### Notes
- `@tauri-apps/plugin-os` queda diferido a 1.6.0 — el heurístico
  `navigator.userAgent` actual funciona en práctica y no aporta
  riesgo visible.

## [1.5.4] — 2026-05-20

Modo Automático que de verdad sincroniza: auto-restore por diff
no destructivo, backup-stale en cada tick, reactividad inmediata
del toggle en Settings, y rename "Copiar" → "Subir" en español.

### Added
- Auto-restore por diff: si faltan archivos del snapshot remoto
  en local, se descargan; los archivos locales nunca se
  sobreescriben (gana local en conflicto).
- Backup-stale en cada tick del Modo Automático: tras
  scan + track, se fuerza `backup_now` por cada save tracked
  como catch-up periódico.
- Doc en el Escritorio (`hoard-modo-automatico.md`) que explica
  el flujo completo del Modo Automático parte por parte.

### Changed
- "Copiar" pasa a "Subir" en los call sites de save-sync en
  español (Dashboard, History). Otros locales ya tenían el
  verbo correcto.
- `set_automatic_mode` ahora propaga el `Prefs` retornado al
  store global del frontend, así Settings refleja
  `auto_restore = true` al instante al activar Modo Automático.
- `is_path_empty_or_missing` deja de ser el gate de
  auto-restore — se sustituye por reconciliación por diff
  (cooldown 60 s y dedup intactos).

### Fixed
- Bug donde el toggle Restauración automática en Settings no
  reflejaba el cambio de `auto_restore` tras activar Modo
  Automático desde la sidebar.
- Bug donde el Modo Automático nunca subía nada por sí solo
  (solo escaneaba + trackeaba; los uploads dependían del
  watcher, que podía perder eventos).

## [1.5.3] — 2026-05-19

UX polish después del overhaul de detección: errores legibles,
server upgrade ejecutable, blacklist de juegos detectados, Modo
Automático persistido, indicador de uso de plan y sidebar pulida
manteniendo decoraciones nativas.

### Added
- Dialog de errores con título + cuerpo + detalles colapsables;
  reemplaza los toasts crudos del updater (cliente y servidor).
- Botón "Actualizar servidor" ejecutable en el modal updater
  cuando el servidor es local y la plataforma es Linux. Fallback
  a "Copiar comando" en cualquier otro caso.
- Blacklist de juegos detectados con confirmación opt-in
  ("Añadir a blacklist permanente"). Reactivación desde Settings
  → "Juegos ignorados".
- **Modo Automático** (toggle on/off persistido). Activarlo
  fuerza `auto_restore = true` y arranca un scheduler que
  re-escanea cada N horas en background. Desactivarlo no toca
  `auto_restore` (independencia hacia abajo).
- Indicador de uso de plan en la sidebar: "Servidor local" si el
  server es self-hosted, o barra 0-100% con ramp emerald/amber/rose
  si está en nube.
- Sidebar y frame pulidos: gradiente sutil, separadores tenues,
  border-l-2 emerald en active state, hover suave con
  transition-colors. Decoraciones nativas intactas.

### Changed
- "Configurar todo" pasa a llamarse "Modo Automático" en los
  ocho locales y deja de ser acción one-shot.
- Errores del updater dejan de mostrarse como strings JSON crudos
  en un toast.

### Fixed
- Juegos detectados sin save path ya se pueden eliminar de la UI.
- Sidebar y items de nav ahora responden visualmente al cursor.

## [1.5.2] — 2026-05-19

Tercer ciclo de detección, foco principal Windows. 1.5.0 y 1.5.1
dejaron huecos del lado del SO mayoritario: launchers no-Steam
invisibles, paths en registry descartados silenciosamente, "Saved
Games" sin token, OneDrive redirigiendo Documents sin que la app se
entere. Linux recibe paridad con Proton añadiendo Lutris y Bottles.

### Added

- Detección de juegos instalados por Epic Games, GOG Galaxy y
  Microsoft Store / Xbox Game Pass: parsers nuevos en
  `launchers.rs` que enumeran manifests `.item` de Epic, el sqlite
  `galaxy-2.0.db` de GOG y el subárbol `GamingServices` del registro
  para MS Store. Cada launcher emite `LauncherApp { app_id, name,
  install_dir }` y se cruza contra el catálogo Ludusavi por slug
  exacto o fuzzy match igual que las entradas Steam.
- Expansión de paths del registro de Windows declarados en el catálogo
  Ludusavi. El parser de `hoard-manifest` captura ahora el campo
  `registry:` de cada entry, y `pathexpand::expand_registry_path` lee
  el value real del hive (`HKEY_CURRENT_USER` / `HKEY_LOCAL_MACHINE`)
  para resolverlo como path candidato. Antes esos paths se
  descartaban silenciosamente en el deserializador.
- Token `<winSavedGames>` en `pathexpand`, mapeado a
  `%USERPROFILE%\Saved Games`. Cubre juegos modernos que usan la
  carpeta oficial Vista+ que Ludusavi expresa con ese placeholder.
- Known Folders sensibles a OneDrive en Windows: `<winDocuments>`,
  `<winAppData>`, `<winLocalAppData>`, `<winPublic>`,
  `<winProgramData>` ahora consultan
  `HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\User Shell Folders`
  para resolver al destino real, capturando la redirección a
  `OneDrive\Documents` que hacen las instalaciones modernas. Si la
  key no existe, fallback al comportamiento previo basado en env vars.
- Detección de prefixes Lutris y Bottles en Linux. Módulo nuevo
  `wine_prefixes.rs` que envuelve `list_proton_prefixes` y suma walks
  de `~/.local/share/lutris/runners/wine/.../prefixes/` y
  `~/.local/share/bottles/bottles/` (más el path flatpak
  `~/.var/app/com.usebottles.bottles/data/bottles/bottles/`). Cada
  prefix detectado entra al walker agresivo y al expander de save
  paths como un Proton más.

### Changed

- El walker agresivo (introducido en 1.5.1) ahora se activa también
  para juegos no-Steam (Epic / GOG / Microsoft Store) y para prefixes
  Lutris / Bottles, no sólo Steam / Proton. El mapa
  `prefix_root_by_slug` se construye a partir de
  `list_wine_prefixes`, así que los tres tipos de prefix alimentan el
  walker con el mismo flujo.

### Fixed

- Paths del registro en entradas Ludusavi ya no se descartan
  silenciosamente. Juegos cuyo save path Ludusavi documenta sólo en
  el campo `registry:` (Skyrim classic, varios Paradox antiguos)
  vuelven a resolverse en Windows.
- Usuarios de Windows con OneDrive activo ya no ven `<winDocuments>`
  redirigido a paths inexistentes. El expander consulta primero el
  registro de Shell Folders, así que la carpeta real
  (`C:\Users\<user>\OneDrive\Documents` o
  `C:\Users\<user>\Documents` según el sistema) gana sobre la
  derivación naive desde `%USERPROFILE%`.

## [1.5.1] — 2026-05-18

Follow-up to 1.5.0. The detection overhaul closed the six structural
gaps, but day-to-day use surfaced three remaining sharp edges: games
the pipeline could see but couldn't resolve to a save path, the inability
to recover from a wrong track without touching `hoard-admin` or the
server DB by hand, and saves stranded on the server after a reinstall
or PC swap with no UI to manage them. 1.5.1 attacks those three.

### Added

- Aggressive walker for installed games the normal pipeline can't
  resolve to a save path. Runs only when the catalog plus filesystem
  heuristic leave a slug with no candidates; bounded by depth 4, a
  1.5 s timeout per root, and a cap of 5 candidates. Confidence stays
  `Low` (or `Medium` when a recent save-like file backs the directory),
  so it complements the catalog rather than overriding it.
- Fuzzy match against the Ludusavi catalog when a Steam app has no
  `steam_app_id` entry and the exact slug also misses. Normalized
  Levenshtein with a 0.15 threshold; ties prefer the entry with a
  Steam app id. The slug-exact lookup still wins first, fuzzy only
  fires as the last fallback before giving up.
- "Eliminar juego" button in the tracked-saves strip that calls the
  existing `DELETE /v1/saves/{id}` endpoint server-side, then clears
  `CliState.saves` and any manual override for the slug. A confirmation
  modal spells out that the snapshots are gone and the action cannot
  be undone.
- Orphan saves (rows that exist on the server but have no local
  `CliState` entry, e.g. after a reinstall or PC swap) are now visible
  in Library with a discreet "Sin estado local" badge. The new red
  delete button is the only actionable control on them; the local
  untrack button is disabled because there is nothing local to remove.

### Changed

- `list_tracked_saves` no longer filters by the local `CliState`. The
  command emits every save the server owns; rows missing locally are
  returned with `orphan: true` so the UI can render them with the
  badge and the disabled-untrack treatment.

### Fixed

- Recovering from a bad track (wrong path, renamed game, stale
  fixture) is now possible from the UI: click the red button, confirm,
  re-scan and re-add cleanly. Previously the only options were to edit
  the server database or run `hoard-admin` by hand, because a local
  untrack left the server row behind and `add_game_to_tracking` would
  swallow the 409 and re-link to the bad row on the next attempt.

## [1.5.0] — 2026-05-18

Detection overhaul. The Library used to lie quietly on Linux: any game
played through Proton was invisible, every Paradox title tracked its
entire game-root instead of just `save games/`, and a user correcting a
wrong path had to do it again on every re-scan. This release rebuilds
the detection pipeline end-to-end against those failure modes.

### Added

- Proton/Wine prefix detection on Linux: games installed via Proton now
  appear in Library with their save path resolved against the
  `steamapps/compatdata/<appid>/pfx` prefix. Stardew Valley played
  through Proton is detected as well as a native Linux install.
- Manual save-path overrides: a path picked from the folder dialog
  persists in `state.json` under `manual_paths` and survives re-scans.
  A "Volver a sugerencia automática" action restores the heuristic
  pick. Overrides always win over filesystem heuristics and catalog
  matches.
- Steam-to-catalog fallback by slugified name when the Ludusavi entry
  lacks `steam_app_id`. Confidence is `Low` so the UI can surface the
  ambiguity if needed.
- Hidden Diagnostics panel (5-click on the sidebar version number,
  same gesture that destapaba `agent_status`) explains step-by-step
  why a given slug is or isn't in the detection report: templates
  expanded, paths kept, paths dropped and the reason.

### Changed

- "Game root → save subdir" refinement is now a general heuristic
  applied to every slug, not the previous hardcoded Stellaris list.
  Paradox games (CK3, EU4, HoI4, Imperator, Victoria 3) no longer get
  their entire game-root backed up — only the `save games/` subdir.
  The exact-on-segment match against `save`, `saves`, `savegame`,
  `savegames`, `save games`, `save_games` keeps false positives like
  `save settings` out.
- Internal cleanup: removed the dead `hoard-detect` crate, the v0.3
  `autodetect.rs` module, and the hand-curated TOML catalog
  (`crates/hoard-manifest/data/games/*.toml` plus
  `placeholders.rs` / `schema.rs`). Only the Ludusavi catalog is
  consulted on the hot path. The workspace shrinks from 9 to 8
  crates.

### Fixed

- Stellaris no longer surfaces the game root as a save path (covered
  by the general refinement above).
- `pathexpand` no longer needs the literal-absolute workaround on the
  hot path: detection routes literal templates through the prefix
  expander, which returns empty for absolute literals rather than
  silently stripping the leading slash.

## [1.4.6] — 2026-05-17

Follow-up after 1.4.5. Two reports landed within a day of shipping:
the new self-hosted server panel never appeared even with a local
server, and auto-restore — promoted in 1.4.3 as the safety net for
empty save folders — only ever fired in two narrow moments (right
after the user added the save, and right before a scheduled upload),
which left big gaps for the kind of failures it's supposed to cover.

### Changed

- **Auto-restore is now a continuous reconciliation loop, not a
  one-shot.** Every process-poll tick (2 s by default) the agent
  sweeps every tracked slot, finds any save whose local folder is
  empty/missing while `auto_restore = true`, and starts a restore
  if no attempt is already running and the 60 s cooldown has
  elapsed. Catches the cases the event-driven paths missed:
  uninstall while Hoard was closed, network blip during an earlier
  attempt, user just turned the toggle on, fs event swallowed by
  the kernel. Per-slot `restoring` flag prevents double-spawn; the
  cooldown prevents a misbehaving server from burning rate limits
  in a loop.
- **The Settings auto-restore toggle now reaches the running agent
  without an app restart.** Flipping it pushes a new
  `AgentCommand::SetAutoRestore` into the live loop; a `false → true`
  flip also kicks an immediate reconciliation sweep so any
  already-empty slot gets restored right then instead of waiting
  for the next tick.
- **Hoard-server panel in Settings → Advanced is shown to every
  signed-in user**, not just `is_local_server == true`. The
  RFC1918/localhost/.local heuristic missed public-DNS self-hosted
  boxes ("hoard.mydomain.com"), so the upgrade panel never
  appeared for users who terminate TLS in front of their own
  server. We'll re-gate properly once a real cloud-hosted instance
  exists; until then any signed-in user sees the panel.

### Fixed

- Settings now triggers an update probe on mount when `lastReport`
  is empty, so the new server panel renders its current/latest
  version on first visit instead of "Comprobando…" until the user
  clicks "Recheck".

## [1.4.5] — 2026-05-17

Tray-resident sessions could go days without ever finding out a new
desktop release had landed: the in-app update probe only ran at boot
and the sidebar amber badge is invisible while the window is hidden.
Plus the only way to upgrade a self-hosted server lived behind the
update modal, which never appeared if the *client* itself was already
current.

### Changed

- **Update poll interval: 6h → 30 min.** GitHub's unauthenticated API
  budget allows 60 req/h; 30 min cadence is still two orders of
  magnitude under that. The exponential backoff cap also dropped from
  24h to 6h so a transient outage no longer freezes detection for a
  full day.

### Added

- **Native OS notification on new desktop release.** The poll now fires
  a `sendNotification` banner the first time it sees a version the user
  hasn't been notified about. Works even when Hoard is minimised to the
  tray — the original "you have to reopen the app to learn there's an
  update" complaint. Persisted in the new
  `prefs.last_update_notified_version` field so reopening the app
  doesn't re-banner for a release the user already saw.
- **Hoard-server panel in Settings → Advanced** (self-hosted only).
  Shows the server's address and current version, flags an update if
  the running server is behind the client, and surfaces the
  `sudo hoard-server upgrade` command + a "Copy" button without making
  the user hunt for the update modal. Gated on
  `auth.user.is_local_server` — the existing RFC1918/localhost/.local
  classifier — so a future cloud-hosted Hoard instance won't show this
  panel.
- Eight-locale translations for the new notification and server-panel
  strings.

### Notes

- The server-update path is still entirely manual (the server must
  never self-update; that decision predates 1.4.5 and is intentional).
  The new panel just removes the modal-hunting required to find the
  upgrade command.

## [1.4.4] — 2026-05-17

Settings UX nit reported right after 1.4.3 shipped: the new auto-restore
toggle was hiding in its own "Sync" section, which is a section
masquerading as a category when it only holds one switch.

### Changed

- **Auto-restore toggle promoted into the General section.** It now
  sits next to "Minimize to tray" — same category of "how Hoard behaves
  day to day" — so users actually find it without having to scroll past
  Language / Startup / Notifications / Privacy looking for a Sync
  heading that barely held one row.

### Removed

- `settings.section_sync` translation key (no longer rendered). The
  eight locale files lose the now-orphaned heading string.

## [1.4.3] — 2026-05-17

Two related bugs the 1.4.2 auto-restore feature surfaced once users
started exercising the "save folder went missing" path in anger:

### Fixed

- **Empty folders no longer push an empty snapshot to the server.** A
  user reported deleting their local save and watching the agent fire a
  backup that "failed because there was nothing to upload". The fs
  watcher *does* fire on deletes (that's the same inotify event you get
  on writes), so `schedule_backup` got armed and then `upload_directory`
  walked an empty tree. We now pre-check the local path inside
  `run_backup_with_retry`: if the folder is missing or contains zero
  entries we skip the upload entirely. Pushing an empty snapshot would
  have silently rotated the last good copy on the server out from
  under the user the next time they looked at History — much worse
  than the visible failure the bug originally caused.
- **Auto-restore now triggers on the fs path, not just on add.** 1.4.2
  only restored when the agent attached to a save with an empty folder.
  If the folder went empty mid-session (uninstall, manual cleanup), the
  agent kept trying to back it up forever. With `auto_restore = true`,
  the same empty-folder pre-check now spawns a restore from the latest
  server snapshot and re-arms the fs watcher against the repopulated
  directory.

### Added

- **`AgentEvent::BackupSkippedEmpty`** + `agent://backup-skipped-empty`
  Tauri channel. Fires when `auto_restore = false` and the local folder
  is empty at backup time. The UI shows an info toast pointing the user
  at the Settings toggle — that way "nothing happened" doesn't read as
  "the agent is broken".
- Eight-locale translation for the new toast string.

### Notes

- The pre-check uses the same `is_path_empty_or_missing` helper as the
  on-attach auto-restore path, so the bar to write user data is
  identical: a populated folder is never touched, and a folder we
  can't enumerate (NFS hiccup) is treated as not-empty rather than
  not-empty-so-overwrite.

## [1.4.2] — 2026-05-17

Opt-in cloud restore on add. The first concrete step of the 1.5.0 client
polish track: when you attach a tracked save whose local folder doesn't
exist or is empty (fresh install of the game, new machine, accidentally
wiped folder), the agent can now pull the latest server snapshot in the
background instead of leaving the slot empty until the user remembers
to "Restore" manually.

### Added

- **`Prefs.auto_restore` + Settings → Sync section.** Off by default —
  silently writing files under the user's `~` is the kind of side-effect
  that earns trust slowly, so it's behind an explicit toggle. The new
  *Sync* section lives between *Startup* and *Notifications* in
  Settings, with a one-line explanation of what gets restored and when.
- **`AgentEvent::SaveAutoRestored` / `SaveAutoRestoreFailed`.** Emitted
  by `hoard-agent` after the background restore lands (or fails). The
  desktop subscribes to `agent://save-auto-restored` and
  `agent://save-auto-restore-failed` and pops an in-app toast so the
  user can see that files appeared without having to refresh the page.
- **8 locales kept in sync.** Five new strings (toast success, toast
  failure, section header, toggle label, toggle description) translated
  into en/es/de/fr/it/ja/pt/zh.

### Changed

- `handle_add` in `hoard-agent` now takes the api client + event sender
  so it can spawn an auto-restore task when the local path is empty.
  The new internal `RearmWatcher` command re-attaches the fs debouncer
  to the now-populated folder so subsequent saves are picked up.

### Notes

- Restore is gated by `is_path_empty_or_missing`: a populated folder
  is never touched, and a folder we can't enumerate (NFS hiccup) is
  treated as not-empty rather than not-empty-so-overwrite. The bar to
  write user data is "we're 100% sure there's nothing there".
- Failure is final: a network error or sha mismatch surfaces as a toast
  and the slot is left untouched. The user can re-attempt manually
  from History.

## [1.4.1] — 2026-05-17

Emergency follow-up to 1.4.0. Two bugs in the in-app upgrade flow that
only surfaced once users tried it against the actual GitHub release:

### Fixed

- **App refused to launch after upgrade.** `setup()` called
  `commands::library::spawn_periodic_rescan`, which used `tokio::spawn`
  before Tauri had entered its event loop — so the very first thing the
  1.4.0 binary did was panic with *"there is no reactor running, must be
  called from the context of a Tokio 1.x runtime"*. On Windows this
  manifested as an instant exit with no window ever painting; on Linux
  the .deb installed cleanly but reopening from the terminal printed
  the panic and bailed. Replaced with `tauri::async_runtime::spawn`,
  matching the sibling helper `auto_update_catalog_in_background`.
- **Old process kept running after `dpkg -i` / `msiexec`.**
  `apply_desktop_update` returned `InstallerLaunched` without telling
  the app to exit, so the user stayed on the 1.3.5 window even though
  the new binary was already on disk. On Windows this also blocked
  msiexec from overlaying the running `.exe` cleanly. After a
  successful installer launch we now wait 1.5 s (long enough for the
  frontend to paint the "installer launched" toast), then on Linux
  spawn the freshly-installed binary via `setsid` so it outlives us
  and call `app.exit(0)`. Windows and macOS just exit — `msiexec` is
  still running async and the .exe is mid-replace, and `open` on
  macOS hands Finder the .dmg; relaunching either would race.

## [1.4.0] — 2026-05-17

Reliability + polish cycle. The big one: auto-backup was silently broken
for any game whose Ludusavi entry has no `processes` list and isn't a
Steam install — the filesystem watcher was being armed lazily on
`GameStarted`, which never fired for those titles, so the Dashboard pill
stayed "Inactivo" forever even while the user was saving in-game. Fixed
by arming the watcher unconditionally on `handle_add` and demoting
`process_poll` to a pure UI signal. Plus: the detection report now
survives restarts, the sidebar nav re-translates with the rest of the
UI, the Dashboard pill no longer lies on cold boot, and the desktop
update probe runs on a 6h timer instead of only at launch.

### Added

- **Persistent detection cache.** `DetectionReport` now serialises to
  `cache.json` alongside `CliState`, with a 24h auto-rescan and an
  explicit "Re-escanear" button on the Library page. Restarting the app
  no longer wipes the scan — the Library hydrates from disk before the
  first scan completes. (`crates/hoard-desktop/src/state.rs`,
  `crates/hoard-desktop/src/commands/library.rs`,
  `crates/hoard-desktop/ui/src/routes/Library.svelte`)
- **Periodic in-app update poller.** Beyond the boot probe, the desktop
  app now re-checks for client and server updates every 6 hours with
  exponential backoff on failure (24 h cap), so long-running sessions
  pick up releases shipped after launch. `App.svelte` consumes the
  result via `$derived($lastReport)`; the timer is cancelled on unmount.
  (`crates/hoard-desktop/ui/src/lib/stores/updates.ts`,
  `crates/hoard-desktop/ui/src/App.svelte`)
- **`just` task runner + pre-commit hook.** New `justfile` at the repo
  root with `dev / check / test / i18n-check / sqlx-prepare /
  install-hooks` recipes; `just install-hooks` points `core.hooksPath`
  at `.githooks/`. The hook itself is scope-aware — `cargo fmt --check
  && clippy` only when Rust files staged, `pnpm check` only when UI
  files staged, `node scripts/check-i18n.mjs` only when locale JSON
  changes — so doc-only commits don't pay the full price.
  (`justfile`, `.githooks/pre-commit`, `docs/dev.md`, `CONTRIBUTING.md`)
- **i18n parity linter.** `scripts/check-i18n.mjs` is pure-Node (no
  deps): for each locale it JSON-validates, diffs the key set against
  `en.json` (missing = error, extra = warning), and verifies
  `{placeholder}` parity with a depth-aware parser that understands ICU
  `plural` / `select` blocks (so branch literals like `{Zeile}` inside
  `{count, plural, one {…}}` don't get mistaken for variables). Wired
  into the pre-commit hook and the `clippy` CI job.
  (`scripts/check-i18n.mjs`, `.github/workflows/ci.yml`)
- **CI hardening.** New `sqlx-check` job runs `cargo sqlx prepare
  --workspace --check` so a missing offline cache fails the PR;
  `cargo-deny` job (Embark action v2) gates licenses / advisories /
  sources / bans; `cargo-machete` flags unused workspace deps. Build
  matrix split into Linux (full workspace + tests) and Windows / macOS
  (excludes `hoard-desktop` — the Tauri `generate_context!()` macro
  needs the frontend, which the release-desktop workflow already
  exercises). (`.github/workflows/ci.yml`, `deny.toml`)
- **Developer guide.** `docs/dev.md` enumerates every just recipe, the
  hook setup, UI and Rust conventions, and the SQLx offline-cache
  workflow.

### Fixed

- **Auto-backup no longer requires the game to be "running".** The fs
  watcher is now armed unconditionally in `handle_add` and survives the
  game starting/stopping. `process_poll` is kept for UI signalling
  (`GameStarted` / `GameStopped` → activity pills, "starting agent"
  state in Magic), but it no longer gates filesystem watching. Heavy
  `tracing::info!` was added at watcher-arm, fs-event, backup-schedule,
  and process transitions so the next silent-failure mode is caught
  immediately instead of two releases later.
  (`crates/hoard-agent/src/agent.rs`)
- **Dashboard pill on first render.** `pillFor()` now falls back to
  `dashboard.pill_saved` (`v{n} guardado`) when `$activity` is empty
  but `tracked.last_version_num > 0`, or to a new
  `dashboard.pill_no_backup` ("Sin copia aún") when there's genuinely
  no snapshot yet. The old behaviour wrongly reported "Inactivo" for
  every save until the agent emitted its first event.
  (`crates/hoard-desktop/ui/src/routes/Dashboard.svelte`,
  `crates/hoard-desktop/ui/src/lib/i18n/locales/*.json`)
- **Stellaris (and similar) now show the "pick a folder" alert.** The
  detector previously trusted the absolute `<winDocuments>\Paradox
  Interactive\Stellaris` path it derived from the manifest even when
  that folder didn't actually exist on the machine, so the user got a
  "Track" button that backed up an empty directory. Detection now
  verifies the candidate path exists and is non-empty before populating
  `found_paths`; otherwise the card falls back to the same amber
  no-save-folder alert other Steam-only matches get.
  (`crates/hoard-detect/src/filesystem.rs`,
  `crates/hoard-desktop/src/commands/library.rs`)
- **Sidebar nav labels re-translate on language change.** `App.svelte`
  was hard-coding the English strings on the `sidebarItems` array; they
  now go through `$_()` at render time via a `labelKey` indirection,
  so switching language in Settings updates the rail instantly.
  (`crates/hoard-desktop/ui/src/App.svelte`,
  `crates/hoard-desktop/ui/src/lib/i18n/locales/*.json`)

### Changed

- **Rename save label.** Saves grow a `PATCH /v1/saves/{id}` endpoint
  on the server and a "Renombrar" item on the History page header.
  Tracked local state migrates the label too; snapshot history is
  preserved untouched. Drag-along from the 0.2 known-limitations list.
  (`crates/hoard-server/src/routes/saves.rs`,
  `crates/hoard-agent/src/api.rs`,
  `crates/hoard-desktop/src/commands/library.rs`,
  `crates/hoard-desktop/ui/src/routes/History.svelte`)
- **i18n gap fill.** All eight locales gained the 11 keys behind the
  Library "no save folder" alert and the untrack confirmation modal
  (`library.no_save_alert_*`, `library.untrack_*`) plus the new
  `dashboard.pill_no_backup`. `settings.about_line_1` bumped to "Hoard
  1.4.0" across the board. Final linter state:
  `i18n ok — 8 locales, 287 keys`.
  (`crates/hoard-desktop/ui/src/lib/i18n/locales/*.json`)

## [1.3.5] — 2026-05-15

In-app updates land. The desktop app already knew when a newer release was
out, but the user still had to download the `.deb` and run `dpkg -i` by
hand. That ends here — the sidebar surfaces an amber alert button next to
the version when GitHub has something newer, clicking it opens a
confirmation modal, and "Yes" launches the OS installer. The server is
kept manual on purpose (it shouldn't self-restart while it might be
serving sync traffic) but gains a `hoard-server upgrade` subcommand so
the operator runs one command instead of editing systemd by hand.

### Added

- **In-app desktop updater.** The sidebar's update-available banner moves
  to a small amber alert button next to the version string (same visual
  vocabulary as the "Sin carpeta" alert). Clicking it pops a confirmation
  modal showing the current and target versions, with **Sí** (green) /
  **No** (red) buttons. Sí downloads the appropriate release asset for
  the host platform (`.deb` on Linux, `.msi` on Windows, `.dmg` on macOS)
  and hands it to the OS installer — `pkexec dpkg -i`, `msiexec /i`,
  `open` — so the user never opens a terminal. If launching the
  installer fails we still surface the downloaded path so they can run
  it manually.
  (`crates/hoard-desktop/src/commands/updates.rs`,
  `crates/hoard-desktop/ui/src/lib/components/UpdateConfirmModal.svelte`,
  `crates/hoard-desktop/ui/src/App.svelte`)
- **`hoard-server upgrade` subcommand.** Fetches the latest GitHub
  release, downloads the linux-x86_64 tarball, atomically swaps the
  `hoard-server` binary in place, and prints a hint to restart the
  systemd unit. Does not load config or touch the database, so a broken
  config still upgrades cleanly. Server self-restart is deliberately not
  attempted — distro init systems vary too much and an in-flight sync
  shouldn't get killed mid-upload by the upgrader.
  (`crates/hoard-server/src/upgrade.rs`,
  `crates/hoard-server/src/main.rs`)

### Changed

- **Update banner replaced by an icon button.** The previous full-width
  amber banner above the sidebar's Magic button is gone; its replacement
  is a 7×7 alert icon next to the Hoard version. Tighter, less noisy,
  and the click target is now the obvious one. The server-update path
  still doesn't auto-install — it copies `sudo hoard-server upgrade` to
  the clipboard so the user runs it on their server box.
  (`crates/hoard-desktop/ui/src/App.svelte`,
  `crates/hoard-desktop/ui/src/lib/i18n/locales/*.json`)

## [1.3.4] — 2026-05-14

Small UX gap on the Library page. When auto-detection finds a save folder
the "Track" button used to commit to that exact path with no way to
override — fine for Stardew, painful for Stellaris on Windows where the
detected `<winDocuments>\Paradox Interactive\Stellaris` may not be where
the user actually keeps their campaigns.

### Added

- **Pick a different folder when tracking.** A small folder icon sits
  next to the "Track" button on every detected game whose save path was
  found automatically. Clicking it pops the OS folder picker instead of
  auto-committing, so users can override the auto-detected path before
  Hoard starts watching. Same code path as the existing pick-from-alert
  flow; no surprise dialogs.
  (`crates/hoard-desktop/ui/src/routes/Library.svelte`,
  `crates/hoard-desktop/ui/src/lib/i18n/locales/*.json`)

## [1.3.3] — 2026-05-14

Brand refresh and a small but irritating tracking bug. The accent colour
moves from amber to a medium-dark emerald that contrasts better with the
dragon mascot; amber is kept exclusively for warnings (pause badge,
restore overwrite banner, near-quota meter, update-available nag, WARN
log lines).

### Fixed

- **Destracking and re-tracking the same game now works.** Stopping
  tracking only clears the local `CliState` row — by design, so server
  snapshots survive a fresh machine. But `list_tracked_saves` was
  returning every save the server knew about for the user, including
  destracked ones, so on the next app launch a ghost "Tracked" card came
  back. Worse, the Library detection card thought the game was still
  being watched and suppressed the amber "no save folder" alert, which
  is the entry point for re-picking the folder. The command now filters
  by local-state presence, so destracked games disappear cleanly.
  (`crates/hoard-desktop/src/commands/library.rs`)
- **Re-tracking after a destrack no longer fails with a 409.** The
  server enforces `UNIQUE(user_id, game_slug, label)`, so the second
  `create_save` returned a conflict the desktop surfaced as an opaque
  error. `add_game_to_tracking` now catches the conflict, finds the
  existing server save via `list_saves`, and re-links it locally —
  preserving the original snapshot history for the user.
  (`crates/hoard-desktop/src/commands/library.rs`)

### Changed

- **Accent colour amber → emerald.** `--color-accent` /
  `--color-accent-hover` now resolve to `emerald-600` / `emerald-500`;
  the `Button` primary variant, `Input` focus ring, `SettingsRow`
  toggle, wizard logo and progress dots, Library scan progress bar,
  History restore progress + checkboxes, Dashboard empty-state icon,
  sidebar logo + magic-setup button, and OnboardingDone admin badge all
  follow. Warning amber is preserved on update banners, WARN log lines,
  medium-confidence detection badges, paused-save badges, restore
  warnings, the near-quota meter, and the no-save alert chip.
  (`crates/hoard-desktop/ui/src/app.css`,
  `crates/hoard-desktop/ui/src/lib/components/*.svelte`,
  `crates/hoard-desktop/ui/src/routes/*.svelte`,
  `crates/hoard-desktop/ui/src/App.svelte`)
- **Spanish copy: pluralise "carpeta de partidas".** The UI used a mix
  of singular and plural ("carpeta de partida" vs. "carpeta de
  partidas") for the same concept; everything is now plural for
  consistency with the History page label. Also fixed *"Hoard no sabe
  dónde guarda partidas {name}"* → *"Hoard no sabe dónde guarda las
  partidas de {name}"* and *"monitorea"* → *"monitoriza"* in the
  magic-setup tooltip/subtitle.
  (`crates/hoard-desktop/ui/src/lib/i18n/locales/es.json`)

## [1.3.2] — 2026-05-14

UX cleanup around the Library page: stop ambushing the user with a folder
picker, and let them untrack a game without dropping to the CLI.

### Added

- **Untrack button on tracked-game cards.** Both the tracked-games strip
  at the top of the Library page and the green "Tracked" badge on
  detection cards now expose a trash icon. Click → confirmation modal
  ("Stop tracking {name}?") that makes clear snapshots on the server are
  preserved. The destructive action calls the existing `untrack_save`
  Tauri command, removes the entry from the local list, and toasts.
  (`crates/hoard-desktop/ui/src/routes/Library.svelte`,
  `crates/hoard-desktop/ui/src/lib/i18n/locales/*.json`)
- **Explicit "no save folder" alert.** Steam-only matches with no
  detected save folder used to silently pop the OS folder picker the
  moment the user clicked "Track" — disorienting if you didn't expect a
  native dialog. Those cards now show an amber `AlertTriangle` button
  instead. Clicking it opens a modal explaining *why* Hoard doesn't have
  a path yet (game never launched on this machine, or saves live outside
  the catalog) and surfaces the Steam install dir as a hint. The folder
  picker only opens when the user explicitly clicks "Choose save
  folder…" in the modal. (`crates/hoard-desktop/ui/src/routes/Library.svelte`)

### Changed

- **`track()` no longer auto-opens the folder picker.** With no
  `found_paths` candidate it now opens the alert modal instead. The
  picker is still reachable via the modal's primary button.
  (`crates/hoard-desktop/ui/src/routes/Library.svelte`)

## [1.3.1] — 2026-05-14

Hotfix for a path-detection bug that caused several Steam-installed games
(Cell to Singularity, Stellaris, …) to be backed up by their **install
directory** rather than their save directory — the user saw 600 MB
snapshots full of the game binaries.

### Fixed

- **Steam matches no longer leak the install directory into `found_paths`.**
  `detect_all` previously seeded the cross-reference map with
  `found_paths: vec![app.install_dir.clone()]`, so any catalog entry that
  matched a Steam appid carried the install dir at index 0. The UI's
  `track()` reads `found_paths[0]` as the local path to back up, which
  meant the snapshot consumed the entire game folder. Steam-only matches
  now leave `found_paths` empty (the UI falls back to the folder picker
  with `library.no_save_folder_yet`), and the install dir is preserved
  separately on a new `DetectedGame.install_dir` field for future UI hints.
  When the filesystem heuristic later fires for the same slug,
  `merge_fs_hit` populates `found_paths` from real save-path templates
  only. (`crates/hoard-agent/src/detection.rs`,
  `crates/hoard-desktop/ui/src/lib/api/index.ts`)

## [1.3.0] — 2026-05-09

Three small features that together make the desktop app feel less like a
toolbox and more like an appliance: the server self-heals when a client
knows about a game it doesn't, the app nags when a newer client or server
is available, and a one-click "magic" button does the whole detect →
track → start-agent dance for users who don't want to think about it.

### Added

- **Server self-heal of unknown games.** When the desktop client tries
  to track a game whose slug the server's catalog doesn't know yet (e.g.
  the server is on an older Ludusavi snapshot), the client now sends
  along the `display_name` and optional `steam_app_id` it already has.
  The server inserts a stub games row (`imported_from = 'client-supplied'`,
  `ON CONFLICT(slug) DO NOTHING`) and proceeds with the save. Old clients
  without these fields still get the original 422, so the change is
  backwards-compatible. (`crates/hoard-server/src/routes/saves.rs`,
  `crates/hoard-agent/src/api.rs`,
  `crates/hoard-desktop/src/commands/library.rs`,
  `crates/hoard-desktop/ui/src/routes/Library.svelte`)
- **Update checker for client and server.** A new `check_for_updates`
  Tauri command probes the GitHub releases API for the latest hoard tag
  and the configured server's `/v1/health` for its running version. Both
  probes run in parallel and tolerate `v` prefixes, prerelease suffixes,
  and double-digit components. The sidebar shows a small amber banner
  above the magic button when either side has an update available; the
  banner deep-links to `/settings`. Failures are silent — a network blip
  just leaves the banner hidden. (`crates/hoard-desktop/src/commands/updates.rs`,
  `crates/hoard-desktop/ui/src/lib/stores/updates.ts`,
  `crates/hoard-desktop/ui/src/App.svelte`)
- **Magic auto-setup button.** A new amber Sparkles button at the bottom
  of the sidebar runs `scan_library` → tracks every detection with
  `confidence === "high"` and at least one found path → boots the agent.
  Per-game errors are reported via toasts but don't abort the rest. The
  button shows phase-aware labels (`detecting`, `tracking 3/12`,
  `starting agent`) and is intentionally limited to high-confidence hits
  to avoid filling the server with false positives.
  (`crates/hoard-desktop/ui/src/lib/stores/magic.ts`,
  `crates/hoard-desktop/ui/src/App.svelte`)
- **Single-source version label in the sidebar.** Vite now injects
  `package.json`'s version into the bundle via `import.meta.env.VITE_HOARD_VERSION`,
  so the sidebar `v1.3.0` line stays in sync with the workspace version
  without a hand-maintained constant. (`crates/hoard-desktop/ui/vite.config.ts`,
  `crates/hoard-desktop/ui/src/vite-env.d.ts`,
  `crates/hoard-desktop/ui/src/App.svelte`)
- **i18n keys for the new surfaces.** Ten new keys (`magic.*` and
  `updates.*`) added to all eight locales (en, es, fr, de, it, pt, ja,
  zh).

## [1.2.2] — 2026-05-09

Hotfix #2 for v1.2.0: the v1.2.1 build no longer panicked, but instead
opened to a blank window (just the body background). Root cause:
`svelte-i18n`'s `init()` only *queues* the locale-dictionary load, so
the very first render reached `$_(...)` while no messages were loaded
yet, the formatter threw "Cannot format a message without first setting
the initial locale", and Svelte unwound the entire mount silently.
Fixed by awaiting `waitLocale()` before calling `mount()`.

### Fixed

- **App opened to a blank window** on every platform after v1.2.1. The
  body background colour was visible because `<body>` ships with a
  Tailwind class, but `#app` stayed empty. Mounting now waits for the
  active locale's dictionary to load. (`crates/hoard-desktop/ui/src/main.ts`,
  `crates/hoard-desktop/ui/src/lib/i18n/index.ts`)

## [1.2.1] — 2026-05-09

Hotfix for v1.2.0: the app crashed on launch with
`there is no reactor running, must be called from the context of a Tokio
1.x runtime`. The auto-update of the Ludusavi catalog was being spawned
with `tokio::spawn` from `setup()`, which runs before Tauri enters its
event loop and therefore has no ambient Tokio runtime. Switched to
`tauri::async_runtime::spawn` which is always available.

### Fixed

- **App crashed instantly on startup** on every platform (Linux/Windows/
  macOS). On Windows the process exited before the window appeared, with
  no console output. (`crates/hoard-desktop/src/commands/catalog.rs`)

## [1.2.0] — 2026-05-08

Desktop UX overhaul: friendlier path handling, restore-anywhere, and full
internationalisation.

### Added

- **Multi-language UI.** The desktop app now ships with translations for
  English, Spanish, French, German, Portuguese, Italian, Japanese, and
  Simplified Chinese. The language is auto-detected from the OS and can be
  changed at any time from **Settings → Language**.
- **Restore to any folder.** When restoring a snapshot for a save that
  isn't tracked on the current machine yet (e.g. you pulled it from
  another device), the app now opens a folder picker and remembers the
  choice — no more "Re-track from the Library" dead end.
- **Native folder pickers.** "Edit folder" on the History page and "Track
  this game" in the Library both grow a *Browse…* button that opens the
  OS folder dialog instead of forcing you to hand-type the path.

### Changed

- **Auto-create missing folders.** Specifying a save folder that doesn't
  exist yet (typed path, picker, or restore destination) now creates it
  for you instead of failing with *"doesn't exist on this machine — pick
  a different folder"*. Useful when restoring saves before installing
  the game.
- **Snapshot labels include a timestamp.** History rows now read
  `save_v3 · 2026-05-08 14:30` so the version line is self-describing
  and copy-pastable into bug reports.
- **Release pipeline.** Dropped the retired `macos-13` (Intel) runner
  from the desktop matrix and switched the publish gate to
  `success() || failure()`, so a stuck-in-queue runner can no longer
  block the rest of the platforms from publishing.

### Fixed

- Restore from the desktop UI now works end-to-end without falling back
  to the CLI — previously the download path resolved against
  `CliState` only, and any save without a local mapping erred out.
- `set_save_local_path` no longer rejects paths that haven't been
  created yet — it `mkdir -p`s them.

## [1.0.0] — 2026-05-08

First stable release. The desktop app, server, and CLI are now considered
stable; the HTTP API and on-disk schema will only change in
backwards-compatible ways within the 1.x line.

This release rolls up the v0.3 phase work (manifest catalog,
process-name detection, storage quota UI, packaging hardening) into a
finalised, signed-off product. From this point forward, official
Windows / Linux / macOS installers are published on every tag —
**users do not have to compile from source**.

### Added

- **Pre-built installers** for every platform on every tagged release
  (see the [Releases page](https://github.com/rleeon/hoard/releases/latest)):
  - **Windows**: NSIS `.exe` setup + `.msi` (MSI installer). Per-user
    install — no admin privileges required.
  - **Linux**: `.deb`, `.rpm`, and AppImage.
  - **macOS**: `.dmg` for both Intel and Apple Silicon.
  - Server tarball: `hoard-1.0.0-linux-x86_64.tar.gz` with the
    headless `hoard-server`, `hoard-admin`, and `hoard` CLI binaries.
  - SHA256 checksums alongside every artifact.
- **Game-detection upgrades** (rolled up from v0.3 phases 1–4b):
  - New `hoard-manifest` crate parses the
    [Ludusavi manifest](https://github.com/mtkennerly/ludusavi-manifest)
    YAML so the catalog covers thousands of titles instead of the
    seeded 10.
  - New `hoard-detect` crate combines filesystem heuristics, Steam
    library parsing (`libraryfolders.vdf` + `appmanifest_*.acf`), and
    process-name matching to identify which save folders belong to
    which game.
  - New `hoard-watcher` crate exposes the live filesystem +
    process watchers as a reusable library so both the desktop agent
    and any future headless daemon share the exact same change-
    detection logic.
  - Lazy `notify` watcher registration: the agent only opens an
    inotify/FSEvents handle when the user actually starts tracking a
    save, dramatically lowering the FD footprint on machines with
    hundreds of detected games.
- **Storage quota UI** (v0.3 phase 4a–5):
  - `whoami` now returns `storage_used_bytes` and
    `storage_quota_bytes`; the desktop app surfaces this as a
    quota bar on the Dashboard.
  - Per-game disk-usage breakdown on the Library page.
- **NSIS per-user install** (v0.3 phase 6): the Windows installer now
  defaults to `currentUser` install mode and a single-language
  English UI, removing the elevation prompt and the language picker
  on first run.

### Changed

- Workspace version bumped to `1.0.0` across every crate.
- README and `docs/install-client.md` updated to point users at the
  pre-built installers as the recommended install path; building
  from source is now an "advanced" option.
- Release CI made portable across runners: macOS bundles now hash
  with `shasum -a 256` (GNU `sha256sum` is not available on macOS
  runners), Linux still uses `sha256sum`. Outputs are byte-identical.
- CI installs `libdbus-1-dev` on the slim server-release runner so
  the CLI's `keyring` dependency builds even outside the
  desktop-runner's GTK stack.

### Fixed

- Tauri icon decoding: regenerated `icon.ico` and the seeded PNGs as
  8-bit RGBA (PNG color_type=6) so Tauri's image pipeline accepts
  them on every platform.
- Release-desktop workflow: Tauri-action's `beforeBuildCommand`
  resolution now finds a `package.json` at the repo root via a thin
  shim, fixing first-tag builds on a fresh checkout.
- `whoami` SQLx offline cache refreshed for the new quota query so
  CI no longer fails with `SQLX_OFFLINE` set.

### Stability commitment

From 1.0.0 onward:

- The HTTP API will only change in backwards-compatible ways within
  the 1.x series. Breaking changes go in 2.0 with a migration note.
- The on-disk snapshot layout (server-side `data/` and `trash/`
  trees) is stable. Old snapshots remain restorable across upgrades.
- The CLI flag surface is stable. New flags may appear; existing
  flags will not be removed without a deprecation cycle.

### Known limitations

- No code-signing on Windows or macOS yet — first-run shows the OS
  "unverified developer" warning. Documented workaround in
  [`docs/install-client.md`](docs/install-client.md#install).
- No auto-updater — install new versions over the top from the
  Releases page. See ADR 0007. Auto-update is on the 1.x roadmap
  once we have signing certificates.

## [0.2.0] — 2026-05-04

The desktop app release. v0.2 ships a Tauri + Svelte client for Linux,
Windows, and macOS that auto-detects installed games, watches their
save folders, uploads versioned snapshots in the background, and lets
you restore previous versions from a friendly UI. The server protocol
is unchanged from 0.1.x.

### Added

- **Desktop app** (`hoard-desktop`, Tauri 2 + Svelte 5):
  - **Onboarding wizard**: server URL probe (`/health`), token paste,
    automatic library scan on completion. Tokens stored in the OS
    secret store (Secret Service / DPAPI / Keychain).
  - **Library / detection**: filesystem heuristics + Steam library
    parsing identify candidate save folders for the seeded catalog.
    Tracking a save persists locally and creates the server-side
    `(game_slug, label)` namespace.
  - **Live agent**: filesystem watcher (`notify` + debouncer)
    triggers a backup on settled changes; process watcher
    (`sysinfo`) flushes immediately when the game stops. Both are
    debounced to avoid hammering the server.
  - **Dashboard**: per-save status pills (idle / scheduled /
    uploading / saved / failed-retrying / paused), a "Back up now"
    override, and a quick link to per-save history.
  - **History page**: snapshot list with file inventory and total
    size. **Restore** flow includes an optional pre-backup safety
    snapshot (default ON) and shows determinate progress for both
    phases. Soft-deleted snapshots are recoverable from the same
    page during the retention window.
  - **Manual controls**: pause/resume tracking per save, edit the
    local save folder path, force-backup, untrack.
  - **Logs viewer**: tail of the rolling daily file appender
    (`agent.log.YYYY-MM-DD`) with level filter and copy-all.
  - **Tray icon** (Linux/Windows/macOS): live state, "Backup all
    now", "Pause all", "Open dashboard", "Quit".
  - **Notifications**: per-event desktop notifications, individually
    toggle-able (success on, failure on, by default).
  - **Settings**: close-to-tray, autostart at login, start
    minimised, success/failure notifications, anonymous telemetry
    (off by default — see `docs/privacy.md`).
- **Packaging**: bundle targets for `.deb`, `.rpm`, `.AppImage`,
  `.nsis` setup `.exe`, `.msi`, and `.dmg` (Intel + Apple Silicon).
  New `release-desktop.yml` workflow runs on every `v*.*.*` tag.
- **Docs**: `docs/install-client.md` (per-platform install,
  uninstall, troubleshooting) and `docs/privacy.md` (every network
  call the desktop app makes, what it stores locally, and the
  opt-in telemetry contract).

### Changed

- Tracing setup now layers a daily-rotating file appender alongside
  stdout — required for the in-app Logs viewer. Affects only the
  desktop binary; the server logs identically to 0.1.x.

### Known limitations

- No code-signing on Windows or macOS yet — first-run shows the OS
  "unverified developer" warning. Documented workaround in
  `docs/install-client.md`.
- No auto-updater; install new versions over the top from the
  GitHub Releases page. See ADR 0007 for the rationale and the
  rollout plan in v0.3.
- Catalog is still the same 10 seeded games as 0.1.0. Multi-instance
  detection (e.g. two copies of Stardew Valley with different mods)
  is supported via labels but the UI doesn't surface a "rename
  label" action yet.

## [0.1.0] — 2026-05-03

First public release. Functionally complete end-to-end backup + restore
flow with versioned snapshots, soft delete, and per-user quotas. **API and
on-disk schema may still change in 0.x; expect to wipe and recreate at
least once before 1.0.**

### Added

- **Server** (`hoard-server`): Axum HTTP server backed by SQLite (WAL,
  `synchronous=NORMAL`, `foreign_keys=ON`) with embedded migrations.
- **Auth**: opaque bearer tokens (`hoard_v1_<64 hex>`), SHA256-hashed in
  the DB. `last_used_at` updated in the background. Argon2id passwords.
- **Games catalog**: 10 seeded games with kebab-case slugs, search by
  substring of slug or display name.
- **Saves**: per-user namespaces scoped to `(game_slug, label)` with
  per-save snapshot count, latest version, and total size.
- **Snapshots**: streaming multipart upload with per-file SHA256, atomic
  commit (`fs::rename` from `tmp/` into `data/` inside a SQLite
  transaction). Streaming `tar.zst` download built on the fly.
  Path traversal hardened. Soft delete moves directories to `trash/`;
  `restore` moves them back. Periodic cleanup task purges old `tmp/` and
  expired trash.
- **Quotas**: per-user `storage_quota_bytes` (default 100 GiB),
  enforced at upload time.
- **Audit log**: every snapshot create/delete/restore writes a row.
- **Admin CLI** (`hoard-admin`): `db {status,migrate,vacuum}`,
  `user {create,list,delete}` (Argon2id, optional TTY prompt),
  `token {create,list,revoke}`, `game {add,list,remove}`.
- **Client CLI** (`hoard`): `config`, `login/logout/whoami`, `status`,
  `games {search,show}`, `save {create,list,show,delete}`,
  `snapshots {list,delete,undelete}`, `backup` (with progress bar +
  `--remember`), `restore` (streaming zstd decode + tar extract + per-file
  SHA256 verification).
- **Packaging**: hardened systemd unit, idempotent `install.sh` /
  `uninstall.sh [--purge]`, multi-stage Dockerfile, docker-compose with
  named volume + `/v1/health` healthcheck.
- **CI**: `cargo fmt --check`, `cargo clippy -- -D warnings`,
  `cargo build --workspace`, `cargo test --workspace`.
- **Release workflow**: tag-driven release builds Linux x86_64 binaries
  and attaches them + checksums to a GitHub Release.

### Known limitations

- No Windows binaries yet (cross-compilation target wired but not
  CI-tested).
- No web UI.
- No multi-tenant / public registration flow — bring your own admin and
  hand out tokens.
- No rate limiting; put a reverse proxy in front for that.
- Single SQLite database; no replication. Back up the file.

[Unreleased]: https://github.com/rleeon/hoard/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/rleeon/hoard/releases/tag/v1.0.0
[0.2.0]: https://github.com/rleeon/hoard/releases/tag/v0.2.0
[0.1.0]: https://github.com/rleeon/hoard/releases/tag/v0.1.0
