---
title: "Comparativa de sincronización de partidas: Hoard frente a Ludusavi, Syncthing, OpenSave y las demás"
description: "Hoard, Ludusavi, Syncthing, OpenSave, GameSave Manager, SaveState, EmuSync y más, comparados: en qué destaca cada uno y una tabla lado a lado."
order: 4
updated: 2026-11-05
---

Steam Cloud solo cubre los juegos que compraste en Steam, y solo cuando el desarrollador se molestó en activarlo. Emuladores, GOG, Epic, itch.io, juegos que no son de Steam, cualquier cosa con mods: nada de eso entra. Si juegas en más de un equipo, un sobremesa y una Steam Deck por ejemplo, acabas copiando carpetas a mano y confiando en haber cogido la más reciente.

Hay varias herramientas que resuelven esto y no todas hacen lo mismo. Unas hacen copias locales, otras replican carpetas entre dispositivos, otras suben a una nube. Esta página las repasa y dice en qué es buena de verdad cada una. Hoard es mi proyecto, así que va al final, con lo que no hace, y con una tabla que puedes leer sin fiarte de una sola línea del texto.

## Ludusavi

La más conocida, y con razón. Ludusavi (de mtkennerly) es una herramienta de copia gratuita y open source, con interfaz y con CLI, construida sobre el manifiesto comunitario de ubicaciones de partidas que cubre decenas de miles de juegos: el mismo manifiesto que usan casi todas las de esta lista, Hoard incluido. Guarda copias locales versionadas y puede subirlas a una nube tuya configurando Rclone.

**Mejor si:** quieres copias locales, control total y ningún servidor en ninguna parte. Es la opción más segura de la lista y no cuesta nada.

**Dónde se queda:** la sincronización entre equipos es algo que montas tú. Programas una copia, configuras un remoto de Rclone y te acuerdas de restaurar en el otro PC *antes* de jugar. Funciona, pero nada te impide olvidarte del último paso.

## Syncthing

No es una herramienta de juegos: es un espejo de carpetas peer-to-peer de propósito general, y muy bueno. Le señalas una carpeta de partidas y aparece en tus otros dispositivos.

**Mejor si:** ya lo tienes montado y quieres los ficheros en dos sitios sin nube por medio.

**Dónde se queda:** replica, no fotografía. Una partida corrupta llega a todos los dispositivos en segundos, exactamente igual de rápido que una buena. Su versionado es por fichero, sin noción de qué es una sesión de juego, así que "volver a como estaba el martes por la noche" es algo que reconstruyes a mano. Dos máquinas que jugaron sin conexión te dan ficheros de conflicto, no una fusión.

## OpenSave

Sincronización peer-to-peer hecha específicamente para partidas, en Go, con licencia MIT, para Windows, Linux, Steam Deck y, desde la versión 2.4, macOS. Sin cuenta y sin servidor: los dispositivos se emparejan entre ellos y sincronizan por la red local o a través de un código de sala en un relay. Fotografía cada cambio, tiene ramas para partidas paralelas, resuelve conflictos por linaje de sincronización en vez de por reloj, y transfiere solo los bloques que cambiaron. Opcionalmente puede replicar a Drive, Dropbox, OneDrive o WebDAV.

**Mejor si:** te niegas a tener una cuenta y tus dispositivos coinciden encendidos lo bastante a menudo.

**Dónde se queda:** peer-to-peer significa que la partida vive solo en tus dispositivos. Si muere la Deck que tenía la única copia reciente y nunca configuraste la réplica, se acabó. Los dos dispositivos tienen que estar en marcha para que haya sincronización.

## OpenCloudSaves

Una interfaz multiplataforma que sincroniza tus carpetas de partidas contra una nube que ya pagas — OneDrive, Google Drive, Dropbox, Nextcloud — usando Rclone por debajo.

**Mejor si:** quieres tus partidas en una cuenta de almacenamiento que ya tienes, con una interfaz en vez de ficheros de configuración de Rclone.

**Dónde se queda:** el proyecto se archivó en enero de 2025 y ya no tiene mantenimiento, así que no recibirá arreglos para juegos nuevos ni para actualizaciones del sistema. Además, no hay deduplicación por contenido. Diez copias de una partida de 2 GB son 20 GB de tu cuota de Drive, y las nubes de disco sincronizan ficheros, no sesiones de juego, así que lo que recuperas es como estuviera la carpeta en ese momento.

## Game Backup Monitor

Primero Windows, y el original de todo este género. GBM vigila el proceso del juego y, cuando sales, comprime la partida con 7-Zip y guarda un historial numerado.

**Mejor si:** estás en un solo PC con Windows y quieres un archivo comprimido local sin pensar en nada.

**Dónde se queda:** es una herramienta de copia, no de sincronización. Llevar el archivo a una segunda máquina es cosa tuya, y Steam Deck / SteamOS no es su terreno.

## GameSave Manager

Una herramienta veterana y gratuita para Windows, de código cerrado y con su propia base de datos de juegos. Hace copias y restaura, y su función más conocida, **Sync & Link**, mueve la carpeta de partidas a una carpeta en la nube como Dropbox u OneDrive y deja un enlace en su lugar, para que el cliente de la nube la mantenga sincronizada.

**Lo mejor si:** estás en Windows, ya vives en Dropbox u OneDrive y quieres que la partida simplemente esté ahí.

**Dónde se queda:** sólo Windows, y Sync & Link significa que un cliente de nube genérico sincroniza la carpeta viva mientras el juego escribe en ella, el mismo patrón que hace [arriesgado Syncthing para las partidas](/guides/syncthing-game-saves). El historial de versiones es el que guarde tu nube.

## Game Save Manager (GSM)

Pese al nombre, no es el mismo programa que el de arriba. GSM, de dyang886, es una aplicación para Windows con licencia GPL-3.0 que encuentra los juegos instalados en Steam, Epic, Battle.net y otros launchers con la base de datos de PCGamingWiki, guarda por cada juego una serie de copias locales que rota con el límite que tú fijes, y se lleva todo el historial a un PC nuevo en un archivo `.gsmr` que exportas. Hay incluso un tercer homónimo: el game-save-manager de mcthesw, popular en comunidades chinas, que copia de forma programada las carpetas que añades a mano, con WebDAV para tener una copia fuera de la máquina.

**Mejor si:** juegas en un solo PC con Windows y quieres copias locales con versiones casi sin configurar nada, y además una mudanza fácil a otra máquina.

**Dónde se queda:** no sincroniza entre dispositivos: el segundo PC recibe tus partidas cuando exportas un archivo y lo importas allí. Solo Windows.

## SaveState

SaveState, de Matteo842, es un gestor de copias con licencia GPL-3.0 para Windows y Linux, con una app aparte para Android. Sueltas encima el acceso directo de un launcher y encuentra la carpeta de partidas, y su soporte de emuladores es muy completo: RetroArch, Dolphin, las tarjetas de memoria de PCSX2, xemu y más de veinte emuladores más. Las copias son archivos `.zip` comprimidos que rotan, y una sincronización opcional los lleva a Google Drive, WebDAV, FTP, una carpeta de red o Git.

**Mejor si:** quieres una herramienta de copias amable, con mucha cobertura de emuladores, y no te importa restaurar a mano en la otra máquina.

**Dónde se queda:** está pensada alrededor de archivos de copia, no de la partida viva. La sincronización mueve esos archivos entre máquinas, pero ponerlos en la carpeta del juego en el otro lado sigue siendo una restauración que lanzas tú. No hay versión para macOS.

## EmuSync

EmuSync es una herramienta de sincronización con licencia GPL-3.0 para partidas de PC y de emuladores, pensada primero para un PC con Windows y una Steam Deck. Asocias la ubicación de las partidas de cada juego, en cada dispositivo, a un único punto de sincronización en tu propio Google Drive, Dropbox u OneDrive, o en una carpeta compartida, y sincroniza sola o cuando se lo pides. Un plugin de Decky enseña el estado en el modo Juego.

**Mejor si:** juegas entre un PC con Windows y una Deck y quieres tus partidas en una nube que ya tienes.

**Dónde se queda:** la nube guarda el estado actual, y sus copias y restauraciones son locales de cada dispositivo, no un historial compartido entre ellos. No está probada en macOS, y la última versión es de febrero de 2026.

## Aletheia

Una de las más recientes, AGPL, y va justo a la parte que las demás cubren a medias: los lanzadores. Heroic, itch.io, Lutris, Steam, GOG Galaxy y Xbox, en Windows, Linux y macOS.

**Mejor si:** tu biblioteca está repartida entre lanzadores que otras herramientas detectan mal, sobre todo Xbox/Game Pass y Heroic.

**Dónde se queda:** es un proyecto joven con un alcance deliberadamente estrecho. Copiar y restaurar es todo el conjunto de funciones; no hay una nube versionada detrás.

## SaveSync

La comercial, se vende en Steam como pago único y está centrada en Windows. Su truco es que no apunta a ti-en-dos-PC, sino al cooperativo: las partidas van a entradas privadas y no listadas del Steam Workshop para que un amigo pueda bajarse tu mundo de Valheim o de Factorio, y además hay sincronización por red local.

**Mejor si:** el problema que resuelves es "mi amigo hospeda y necesito su partida", no "que mis partidas me sigan".

**Dónde se queda:** código cerrado, Windows, atado a Steam como transporte, y una lista de juegos cooperativos soportados en vez de todo lo que tengas.

## Tachyon

El nombre más nuevo de la lista, en beta gratuita para Windows. Tachyon detecta las partidas de más de 5.000 juegos de PC, las sincroniza a través de su propia nube y guarda un historial de versiones con varias «líneas temporales».

**Lo mejor si:** estás en Windows, quieres algo que funcione nada más instalarlo y no te importa que sea una beta.

**Dónde se queda:** de momento sólo Windows (Linux y macOS aparecen como «próximamente»), así que aún no hay Steam Deck; sin código fuente publicado; sin forma de usar tu propio servidor. El precio tras la beta no se ha anunciado.

## Plugins para Steam Deck (Decky)

En la propia Deck, varios plugins de Decky Loader cubren parte del trabajo desde el modo Juego. Decky Cloud Save sincroniza carpetas con OneDrive, Google Drive o Dropbox mediante rclone al cerrar un juego; su autor ya no lo mantiene y remite a dos sucesores, Decky Ludusavi (una interfaz de Ludusavi para el modo Juego) y SDH-GameSync (un fork que baja las partidas al abrir cada juego y las sube al cerrarlo). Steamback guarda en local las diez instantáneas más recientes de los juegos que usan Steam Cloud, tomadas al salir.

**Mejor si:** juegas sobre todo en la Deck y quieres algo que se maneje sin salir del modo Juego.

**Dónde se queda:** solo funcionan en la Deck, y la parte de llevar las partidas a otro dispositivo depende de otra cosa: rclone y tu nube, Ludusavi o el propio Steam Cloud. Las instantáneas de Steamback se quedan en la Deck y solo cubren juegos con Steam Cloud.

## Un apunte sobre EmuDeck

EmuDeck sale en estas conversaciones y no es un competidor en el sentido normal: es un instalador y configurador de emuladores para Steam Deck, y la sincronización que ofrece es una comodidad añadida a ese trabajo (Rclone contra una nube de disco, solo para partidas de emulador). Se solapa con las herramientas de arriba sin ser lo mismo: EmuDeck te deja los emuladores montados, y las de aquí cuidan las partidas de toda la biblioteca. Hay gente que usa EmuDeck junto a una de estas, y es un montaje sensato, no redundante.

## Hoard

Hoard copia cada partida en cuanto el juego termina de escribirla y la mantiene sincronizada en todas las máquinas en las que juegas. El motor corre como servicio en segundo plano, `hoardd`, sin ventana, así que sigue funcionando en el modo Juego de SteamOS. Espera a que la carpeta lleve unos segundos quieta, así que nunca sube un fichero a medio escribir; hace una copia más al salir; y nunca cambia la partida de un juego abierto: lo que baja de otra máquina espera a que cierres el juego.

- **La otra máquina puede estar apagada.** La partida más nueva espera en el servidor, así que la Deck que coges la semana que viene la recibe sin que el PC esté encendido. Las herramientas punto a punto necesitan los dos dispositivos despiertos a la vez.
- **Todas las versiones, guardadas.** Cada partida es una versión a la que puedes volver desde la app o con `hoard restore`, también después de un fallo de disco o de una instalación limpia. Hoard One guarda todas las versiones para siempre; en tu propio servidor decides tú cuánto historial guardar.
- **Solo viaja lo que cambia.** Las subidas mandan solo los ficheros que han cambiado, comprimidos con zstd, y las versiones comparten todo lo que no cambia: diez versiones de una partida de 2 GB en la que cambia poco ocupan unos 2 GB, no 20.
- **SHA-256 a la subida y a la bajada.** La corrupción se detecta antes de que pueda sobrescribir una partida buena, y Hoard guarda una copia de todo lo que sustituye.
- **Gratis y de código abierto, con tu propio servidor si lo quieres.** La aplicación, la CLI y el servidor son AGPL-3.0. Hoard One, el servidor gestionado que recomendamos a todo el mundo, es gratis con 2 GB y tres dispositivos, y no hay nada que montar. O ejecuta `hoard-server` con Docker en un PC, un NAS o una Raspberry Pi, sobre disco local o cualquier almacenamiento compatible con S3: sin cuenta con nosotros, sin telemetría hacia nosotros y sin cupo nuestro.
- **Windows, Linux, macOS y SteamOS**, con la misma aplicación de escritorio en los cuatro y una CLI para servidores y equipos sin pantalla.
- **Una detección que cubre toda la biblioteca:** Steam, Epic, GOG, prefijos de Proton y Wine, miles de juegos gracias a la base de datos comunitaria de partidas, y presets para 19 emuladores, de PCSX2, Dolphin y RPCS3 a Ryujinx, Cemu, DuckStation y RetroArch.

## El detalle que decide la sincronización Steam Deck ↔ PC

Conviene saberlo elijas la herramienta que elijas. La partida en la nube de un juego de Steam vive en `<AppID>/remote/`, y la carpeta de *encima* guarda `remotecache.vdf`, el estado de logros, estadísticas y contadores de horas jugadas, cosas que legítimamente son distintas entre tu Deck y tu sobremesa.

Sincroniza la carpeta padre y tendrás un conflicto permanente entre dos máquinas que nunca discreparon sobre una sola partida. Hoard rastrea `remote/`, no la carpeta padre. A cualquier herramienta a la que le señales una carpeta a mano se le puede decir lo mismo, y es lo primero que hay que mirar cuando un montaje de sincronización marca conflictos sin motivo aparente.

## Lo que Hoard no hace

- **Sincronizar sin servidor.** Hoard siempre sincroniza a través de uno: Hoard One si prefieres no montar nada, o el tuyo. Es justo lo que permite que el otro dispositivo esté apagado, y lo que deja una copia que sobrevive a las dos máquinas. Si no quieres un servidor en ningún caso, OpenSave, Syncthing o Ludusavi están hechos para eso.
- **Compartir mundos de cooperativo.** Pasarle un mundo a un amigo es para lo que está hecho SaveSync; Hoard cuida tus propias partidas.
- **Fusionar dos partidas que han divergido.** Ninguna herramienta puede unir dos partidas en una. Hoard guarda las dos como versiones, en todas tus máquinas, y eliges tú.

## La distinción entre Hoard One y autoalojarse

Las comparativas sobre Hoard casi siempre funden las dos en una, y el resultado sale mal, así que conviene decirlo claro:

- **Hoard One** es la opción gestionada, la que recomendamos a todos: inicias sesión y tus partidas se guardan en nuestros servidores, en la UE.
- **Un Hoard autoalojado es tuyo por completo.** Levantas `hoard-server` en tu PC o en tu NAS y tus partidas van de tu máquina a tu disco. **No hay cuenta con nosotros, ni telemetría hacia nosotros, ni cupo, ni relé**: no pasa nada por nuestros servidores, porque no hay nada nuestro en el camino. No podemos ver una partida, ni el nombre de un juego, ni un correo, porque nada de eso nos llega. Si Hoard One cerrara mañana, un montaje autoalojado seguiría igual.

El mismo código, la misma detección. Lo que cambia es de quién es el almacenamiento, y un ajuste por defecto: un servidor autoalojado va podando las versiones antiguas con el tiempo, salvo que lo desactives. Y siendo exactos en un detalle: tu servidor sí tiene sus propios accesos — un usuario y un token por dispositivo — pero viven en tu base de datos, no en la nuestra.

## La tabla

| Herramienta | Sincronización automática | Funciona con el otro dispositivo apagado | Dónde viven las partidas | Historial | Plataformas | Licencia |
|---|---|---|---|---|---|---|
| **Hoard** | Sí, cada vez que el juego guarda | Sí | Hoard One o tu propio servidor (disco o compatible con S3) | Todas las versiones, deduplicadas | Win · Linux · macOS · Deck | AGPL-3.0, gratis |
| **Ludusavi** | Manual, o con Rclone que montas tú | Solo con un remoto de Rclone | Local, más tu remoto de Rclone | Copias locales versionadas | Win · Linux · macOS | MIT, gratis |
| **Syncthing** | Sí, espejo continuo | No, los dos tienen que estar encendidos | Solo en tus dispositivos | Versionado por fichero | Todas | MPL-2.0, gratis |
| **OpenSave** | Sí, punto a punto | No, salvo que configures un espejo en la nube | Tus dispositivos, espejo en la nube opcional | Instantáneas y ramas | Win · Linux · macOS · Deck | MIT, gratis |
| **OpenCloudSaves** | A través de tu nube | Sí, a través de la nube | OneDrive / Drive / Dropbox / Nextcloud | Lo que guarde la nube | Win · Linux · macOS | Código abierto, archivado |
| **Game Backup Monitor** | No | No sincroniza | Archivos 7-Zip locales | Copias numeradas | Windows · Linux | GPL-3.0, gratis |
| **GameSave Manager** | A través de tu nube (Sync & Link) | Sí, a través de la nube | Local, más Dropbox / OneDrive | Lo que guarde la nube | Windows | Gratis, código cerrado |
| **Game Save Manager (GSM)** | No, exportar e importar | No sincroniza | Copias locales | Copias locales rotativas | Windows | GPL-3.0, gratis |
| **SaveState** | Opcional, de los archivos de copia | Sí, a través de tu almacenamiento | Zips locales, más Drive / WebDAV / FTP / SMB / Git | Archivos rotativos | Windows · Linux | GPL-3.0, gratis |
| **EmuSync** | Sí, a través de tu nube | Sí, a través de la nube | Drive / Dropbox / OneDrive o una carpeta compartida | Copias locales | Windows · Linux · Deck | GPL-3.0, gratis |
| **Aletheia** | Copia y restauración por launcher | No sincroniza | Tu almacenamiento | Copias | Win · Linux · macOS | AGPL-3.0 |
| **SaveSync** | Sí, y con amigos | Sí, a través de Steam | Entradas privadas de Steam Workshop | Según la app | Windows | De pago, código cerrado |
| **Tachyon** | Sí, a través de su propia nube | Sí | La nube de Tachyon | Versiones y líneas temporales | Windows (beta) | Beta gratuita, sin código publicado |

## Entonces cuál

Si quieres que las copias y la sincronización ocurran solas, entre PCs y una Steam Deck, con todas las versiones guardadas, la otra máquina libre de estar apagada y la opción de que todo sea tuyo, eso es Hoard. [Descárgalo](/download), entra en Hoard One, o lee antes [cómo montarlo en tu servidor con Docker](/guides/self-host-hoard). Es gratis de las dos maneras.

Para trabajos más acotados: un solo PC con Windows y archivos locales, Game Backup Monitor o GSM. Copias locales con control total y sin servidor en ningún sitio, Ludusavi. Sin cuenta en ningún caso y con dispositivos que suelen estar encendidos a la vez, OpenSave o Syncthing. Partidas que tienen que vivir en una carpeta de Drive que ya pagas, Ludusavi con un remoto de Rclone, o EmuSync; OpenCloudSaves lo hacía, pero está archivado. Compartir un mundo de cooperativo con amigos, SaveSync.

Si venías de OpenCloudSaves y lo que querías era tener las partidas en un almacenamiento que controlas, más que en Drive como tal, un Hoard autoalojado lo hace y añade el historial: escribe en disco local o en cualquier bucket compatible con S3, como Backblaze B2 o MinIO. Si es Ludusavi lo que estás sopesando, hay una [comparativa más larga con Ludusavi](/guides/ludusavi-alternative).

## Comparativas una a una

Cada una entra más a fondo que el bloque de arriba, incluido dónde gana la otra herramienta:

- [Hoard frente a Ludusavi](/guides/ludusavi-alternative)
- [Hoard como alternativa a Steam Cloud](/guides/steam-cloud-alternative)
- [Sincronización punto a punto frente a un servidor tuyo](/guides/opensave-alternative)
- [Syncthing para partidas: qué se rompe](/guides/syncthing-game-saves)
- [Hoard frente a GameSave Manager y los otros Game Save Manager](/guides/game-save-manager-alternative)
- [Sustituir OpenCloudSaves ahora que está archivado](/guides/opencloudsaves-alternative)
- [Una alternativa gratis a SaveSync](/guides/savesync-alternative)

<!-- faq -->

## Preguntas frecuentes

### ¿Cuál de estas herramientas guarda historial de versiones?

Hoard conserva cada partida como una versión a la que puedes volver, en todas tus máquinas. Ludusavi, GSM y SaveState guardan copias locales versionadas o rotativas, y OpenSave guarda instantáneas en tus dispositivos. La mayoría del resto sincroniza o copia el estado actual, lo que significa que una partida corrupta se propaga fielmente a tu otra máquina.

### ¿Cuál funciona sin servidor ni cuenta?

Ludusavi con copias locales, y cualquier herramienta punto a punto. Hoard también entra si te autoalojas: sin cuenta con nosotros y sin nada que pase por nuestros servidores.

### ¿Cuál sincroniza con mi otro dispositivo apagado?

Cualquiera que guarde la partida en algún sitio que no sean tus dispositivos: Hoard, a través de Hoard One o de tu propio servidor, y las herramientas que escriben en una nube, como EmuSync o Ludusavi con Rclone. La sincronización punto a punto (OpenSave sin su espejo, Syncthing) necesita los dos dispositivos encendidos a la vez.

### ¿Cuál cubre juegos que no están en Steam?

Todas las herramientas de gestión de partidas de aquí, porque localizan los saves con la misma base de datos comunitaria y no a través de una tienda. La que no lo hace es Steam Cloud: sólo cubre juegos de Steam cuyo desarrollador lo activó.

### ¿Tengo que quedarme con una sola?

No, y mucha gente no lo hace. Una herramienta de copia local y una de sincronización resuelven mitades distintas del problema. La única regla es no apuntar nunca una a la carpeta de copias de la otra, o acabas sincronizando un espejo desfasado en vez de tu partida real.

### ¿Cuál es el detalle que rompe la mayoría de montajes caseros?

Sincronizar la carpeta que está por encima de `<AppID>/remote/` en el `userdata` de Steam. La padre guarda `remotecache.vdf` y ficheros de logros y tiempo jugado que deben ser distintos en cada máquina, así que cada arranque parece un conflicto aunque no se haya movido ninguna partida.
