---
title: "Comparativa de sincronización de partidas: Hoard frente a Ludusavi, Syncthing, OpenSave y las demás"
description: "Comparativa honesta de las herramientas que hacen copia y sincronizan partidas de PC — Ludusavi, Syncthing, OpenSave, OpenCloudSaves, Game Backup Monitor, Aletheia, SaveSync y Hoard — con tabla y un apartado sobre dónde pierde Hoard."
order: 4
updated: 2026-09-01
---

Steam Cloud solo cubre los juegos que compraste en Steam, y solo cuando el desarrollador se molestó en activarlo. Emuladores, GOG, Epic, itch.io, juegos que no son de Steam, cualquier cosa con mods: nada de eso entra. Si juegas en más de un equipo, un sobremesa y una Steam Deck por ejemplo, acabas copiando carpetas a mano y confiando en haber cogido la más reciente.

Hay varias herramientas que resuelven esto y no todas hacen lo mismo. Unas hacen copias locales, otras replican carpetas entre dispositivos, otras suben a una nube. Esta página las repasa y dice en qué es buena de verdad cada una. Hoard es mi proyecto, así que la parte honesta va al final: un apartado sobre dónde pierde Hoard, y una tabla que puedes leer sin fiarte de una sola línea del texto.

## Ludusavi

La más conocida, y con razón. Ludusavi (de mtkennerly) es una herramienta de copia gratuita y open source, con interfaz y con CLI, construida sobre el manifiesto comunitario de ubicaciones de partidas que cubre decenas de miles de juegos: el mismo manifiesto que usan casi todas las de esta lista, Hoard incluido. Guarda copias locales versionadas y puede subirlas a una nube tuya configurando Rclone.

**Mejor si:** quieres copias locales, control total y ningún servidor en ninguna parte. Es la opción más segura de la lista y no cuesta nada.

**Dónde se queda:** la sincronización entre equipos es algo que montas tú. Programas una copia, configuras un remoto de Rclone y te acuerdas de restaurar en el otro PC *antes* de jugar. Funciona, pero nada te impide olvidarte del último paso.

## Syncthing

No es una herramienta de juegos: es un espejo de carpetas peer-to-peer de propósito general, y muy bueno. Le señalas una carpeta de partidas y aparece en tus otros dispositivos.

**Mejor si:** ya lo tienes montado y quieres los ficheros en dos sitios sin nube por medio.

**Dónde se queda:** replica, no fotografía. Una partida corrupta llega a todos los dispositivos en segundos, exactamente igual de rápido que una buena. Su versionado es por fichero, sin noción de qué es una sesión de juego, así que "volver a como estaba el martes por la noche" es algo que reconstruyes a mano. Dos máquinas que jugaron sin conexión te dan ficheros de conflicto, no una fusión.

## OpenSave

Sincronización peer-to-peer hecha específicamente para partidas, en Go, con licencia MIT, para Windows, Linux y Steam Deck. Sin cuenta y sin servidor: los dispositivos se emparejan entre ellos y sincronizan por la red local o a través de un código de sala en un relay. Fotografía cada cambio, tiene ramas para partidas paralelas, resuelve conflictos por linaje de sincronización en vez de por reloj, y transfiere solo los bloques que cambiaron. Opcionalmente puede replicar a Drive, Dropbox, OneDrive o WebDAV.

**Mejor si:** te niegas a tener una cuenta y tus dispositivos coinciden encendidos lo bastante a menudo.

**Dónde se queda:** peer-to-peer significa que la partida vive solo en tus dispositivos. Si muere la Deck que tenía la única copia reciente y nunca configuraste la réplica, se acabó. Los dos dispositivos tienen que estar en marcha para que haya sincronización, y no hay versión para macOS.

## OpenCloudSaves

Una interfaz multiplataforma que sincroniza tus carpetas de partidas contra una nube que ya pagas — OneDrive, Google Drive, Dropbox, Nextcloud — usando Rclone por debajo.

**Mejor si:** quieres tus partidas en una cuenta de almacenamiento que ya tienes, con una interfaz en vez de ficheros de configuración de Rclone.

**Dónde se queda:** no hay deduplicación por contenido. Diez copias de una partida de 2 GB son 20 GB de tu cuota de Drive, y las nubes de disco sincronizan ficheros, no sesiones de juego, así que lo que recuperas es como estuviera la carpeta en ese momento.

## Game Backup Monitor

Primero Windows, y el original de todo este género. GBM vigila el proceso del juego y, cuando sales, comprime la partida con 7-Zip y guarda un historial numerado.

**Mejor si:** estás en un solo PC con Windows y quieres un archivo comprimido local sin pensar en nada.

**Dónde se queda:** es una herramienta de copia, no de sincronización. Llevar el archivo a una segunda máquina es cosa tuya, y Steam Deck / SteamOS no es su terreno.

## Aletheia

La más nueva del grupo, AGPL, y va justo a la parte que las demás cubren a medias: los lanzadores. Heroic, itch.io, Lutris, Steam, GOG Galaxy y Xbox, en Windows, Linux y macOS.

**Mejor si:** tu biblioteca está repartida entre lanzadores que otras herramientas detectan mal, sobre todo Xbox/Game Pass y Heroic.

**Dónde se queda:** es un proyecto joven con un alcance deliberadamente estrecho. Copiar y restaurar es todo el conjunto de funciones; no hay una nube versionada detrás.

## SaveSync

La comercial, se vende en Steam como pago único y está centrada en Windows. Su truco es que no apunta a ti-en-dos-PC, sino al cooperativo: las partidas van a entradas privadas y no listadas del Steam Workshop para que un amigo pueda bajarse tu mundo de Valheim o de Factorio, y además hay sincronización por red local.

**Mejor si:** el problema que resuelves es "mi amigo hospeda y necesito su partida", no "que mis partidas me sigan".

**Dónde se queda:** código cerrado, Windows, atado a Steam como transporte, y una lista de juegos cooperativos soportados en vez de todo lo que tengas.

## Un apunte sobre EmuDeck

EmuDeck sale en estas conversaciones y no es un competidor en el sentido normal: es un instalador y configurador de emuladores para Steam Deck, y la sincronización que ofrece es una comodidad añadida a ese trabajo (Rclone contra una nube de disco, solo para partidas de emulador). Se solapa con las herramientas de arriba sin ser lo mismo: EmuDeck te deja los emuladores montados, y las de aquí cuidan las partidas de toda la biblioteca. Hay gente que usa EmuDeck junto a una de estas, y es un montaje sensato, no redundante.

## Hoard

Hoard toma la sesión de juego como unidad. El motor corre como servicio en segundo plano — `hoardd`, sin ventana, así que funciona en el modo juego de SteamOS —, se entera de que has dejado de jugar y hace la instantánea entonces, en vez de reaccionar a cada escritura de fichero en mitad de la partida.

- **Historial versionado por sesión.** Cada sesión es una versión a la que puedes volver, incluso después de un fallo de disco o una instalación limpia.
- **Deduplicación por hash de contenido.** Diez versiones de una partida de 2 GB ocupan unos 2 GB, no 20 GB. Las transferencias van comprimidas con zstd.
- **SHA-256 al subir y al bajar.** La corrupción se detecta antes de que pueda sobrescribir una partida buena. Nada se sobrescribe en silencio: ese es todo el diseño.
- **Nube o autoalojado, el mismo binario.** Hoard Cloud tiene plan gratuito (2 GB, 3 dispositivos, historial completo). O levantas `hoard-server` tú mismo con Docker Compose contra cualquier almacenamiento compatible con S3 — MinIO, Garage, Backblaze B2 — sin cuenta y sin cuota. AGPL-3.0.
- **Windows, Linux y macOS**, más una CLI sin interfaz para una Steam Deck o un servidor.
- **Emuladores en beta:** PCSX2, RPCS3, Dolphin, Cemu, Ryujinx, RetroArch, DuckStation, PPSSPP y otros como preajustes.

## El detalle que decide la sincronización Steam Deck ↔ PC

Conviene saberlo elijas la herramienta que elijas. La partida en la nube de un juego de Steam vive en `<AppID>/remote/`, y la carpeta de *encima* guarda `remotecache.vdf`, el estado de logros, estadísticas y contadores de horas jugadas, cosas que legítimamente son distintas entre tu Deck y tu sobremesa.

Sincroniza la carpeta padre y tendrás un conflicto permanente entre dos máquinas que nunca discreparon sobre una sola partida. Hoard rastrea `remote/`, no la carpeta padre. A cualquier herramienta a la que le señales una carpeta a mano se le puede decir lo mismo, y es lo primero que hay que mirar cuando un montaje de sincronización marca conflictos sin motivo aparente.

## Dónde pierde Hoard

- **Quiere un servidor.** Cuenta en la nube o máquina tuya, en cualquier caso es infraestructura, y OpenSave o Ludusavi no necesitan ninguna.
- **El soporte de emuladores está en beta.** Las instalaciones portables y las manías de cada emulador todavía lo pillan, y hoy Aletheia y OpenSave cubren mejor algunos casos raros de lanzadores y emuladores.
- **macOS apenas está probado en hardware real.** Compila y funciona, pero nadie ha vivido ahí durante meses.
- **Es joven.** Ludusavi y Game Backup Monitor llevan años de informes de fallos a la espalda. Hoard no, y eso importa en algo que custodia una partida de 200 horas.
- **No hace cooperativo.** Si quieres pasarle un mundo a un amigo, SaveSync está hecho para eso y Hoard no.

## La distinción entre Hoard Cloud y autoalojarse

Las comparativas sobre Hoard casi siempre funden las dos en una, y el resultado sale mal, así que conviene decirlo claro:

- **Hoard Cloud** es la opción gestionada: inicias sesión y tus partidas se guardan en nuestros servidores, en la UE.
- **Un Hoard autoalojado es tuyo por completo.** Levantas `hoard-server` en tu PC o en tu NAS y tus partidas van de tu máquina a tu disco. **No hay cuenta con nosotros, ni telemetría hacia nosotros, ni cupo, ni relé**: no pasa nada por nuestros servidores, porque no hay nada nuestro en el camino. No podemos ver una partida, ni el nombre de un juego, ni un correo, porque nada de eso nos llega. Si Hoard Cloud cerrara mañana, un montaje autoalojado seguiría igual.

El mismo binario, la misma detección, el mismo historial de versiones. Lo único que cambia es de quién es el almacenamiento. Y siendo exactos en un detalle: tu servidor sí tiene sus propios accesos — un usuario y un token por dispositivo — pero viven en tu base de datos, no en la nuestra.

## La tabla

| Herramienta | Sincronización automática entre dispositivos | Dónde viven las partidas | Historial | Plataformas | Licencia |
|---|---|---|---|---|---|
| **Hoard** | Sí, por sesión de juego | Hoard Cloud o tu propio servidor (compatible con S3) | Versionado por sesión, deduplicado | Win · Linux · macOS · Deck | AGPL-3.0, plan gratuito |
| **Ludusavi** | Manual, o Rclone que montas tú | Local, más tu remoto de Rclone | Copias locales versionadas | Win · Linux · macOS | Gratis, open source |
| **Syncthing** | Sí, espejo continuo | Solo tus dispositivos | Versionado por fichero | Todo | Gratis, open source |
| **OpenSave** | Sí, peer-to-peer | Tus dispositivos, réplica opcional en nube | Instantáneas y ramas | Win · Linux · Deck | MIT |
| **OpenCloudSaves** | Sí, vía tu nube de disco | OneDrive / Drive / Dropbox / Nextcloud | Lo que guarde la nube | Win · Linux · macOS | Gratis, open source |
| **Game Backup Monitor** | No | Archivos 7-Zip locales | Copias numeradas | Windows | Gratis, open source |
| **Aletheia** | Copia y restauración por lanzador | Tu almacenamiento | Copias | Win · Linux · macOS | AGPL-3.0 |
| **SaveSync** | Sí, y con amigos | Entradas privadas del Steam Workshop | Según la app | Windows | De pago, código cerrado |

## Entonces cuál

Si quieres una sola máquina respaldada y nada más, coge Ludusavi o Game Backup Monitor. Si no quieres una cuenta bajo ningún concepto y tus dispositivos suelen estar encendidos a la vez, OpenSave. Si tus partidas deben acabar en una carpeta de Drive que ya pagas, OpenCloudSaves. Si compartes un mundo cooperativo con amigos, SaveSync.

Si lo que quieres es que la copia *y* la sincronización entre PC y una Steam Deck pasen solas, con una versión por sesión a la que volver y la opción de autoalojarlo todo, para eso está Hoard. [Descárgalo](/download), o léete antes [cómo autoalojarlo con Docker](/guides/self-host-hoard). También hay una [comparativa larga con Ludusavi](/guides/ludusavi-alternative) si es esa la que estás sopesando.

## Comparativas una a una

Cada una entra más a fondo que el bloque de arriba, incluido dónde gana la otra herramienta:

- [Hoard frente a Ludusavi](/guides/ludusavi-alternative)
- [Hoard como alternativa a Steam Cloud](/guides/steam-cloud-alternative)
- [Sincronización punto a punto frente a un servidor tuyo](/guides/opensave-alternative)
- [Syncthing para partidas: qué se rompe](/guides/syncthing-game-saves)

<!-- faq -->

## Preguntas frecuentes

### ¿Cuál de estas herramientas guarda historial de versiones?

Hoard conserva cada sesión como una versión a la que puedes volver. Ludusavi guarda copias locales versionadas. La mayoría del resto sincroniza o copia el estado actual, lo que significa que una partida corrupta se propaga fielmente a tu otra máquina.

### ¿Cuál funciona sin servidor ni cuenta?

Ludusavi con copias locales, y cualquier herramienta punto a punto. Hoard también entra si te autoalojas: sin cuenta con nosotros y sin nada que pase por nuestros servidores.

### ¿Cuál cubre juegos que no están en Steam?

Todas las herramientas de gestión de partidas de aquí, porque localizan los saves con la misma base de datos comunitaria y no a través de una tienda. La que no lo hace es Steam Cloud: sólo cubre juegos de Steam cuyo desarrollador lo activó.

### ¿Tengo que quedarme con una sola?

No, y mucha gente no lo hace. Una herramienta de copia local y una de sincronización resuelven mitades distintas del problema. La única regla es no apuntar nunca una a la carpeta de copias de la otra, o acabas sincronizando un espejo desfasado en vez de tu partida real.

### ¿Cuál es el detalle que rompe la mayoría de montajes caseros?

Sincronizar la carpeta que está por encima de `<AppID>/remote/` en el `userdata` de Steam. La padre guarda `remotecache.vdf` y ficheros de logros y tiempo jugado que deben ser distintos en cada máquina, así que cada arranque parece un conflicto aunque no se haya movido ninguna partida.
