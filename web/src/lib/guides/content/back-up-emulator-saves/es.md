---
title: "Cómo hacer copia y sincronizar partidas de emuladores (RetroArch, Dolphin, PCSX2)"
description: "Copia y sincroniza partidas de emuladores entre PC y Steam Deck: RetroArch, Dolphin, PCSX2, DuckStation y más, con historial y dónde guarda cada uno."
order: 6
updated: 2026-10-09
---

Las partidas de emulador se pierden con facilidad: los archivos de guardado y los estados guardados viven en carpetas dispersas, y una reinstalación o un PC nuevo pueden borrar años de progreso. Hoard hace la copia automáticamente y los mantiene sincronizados entre equipos, Steam Deck incluida.

## Emuladores con los que funciona Hoard

Hoard gestiona los archivos de guardado estándar de emulador (`.srm`, `.sav`, memory cards, carpetas de guardado por juego) y los estados guardados. Sabe de serie dónde guardan estos:

- **Sony:** PCSX2 (PS2), DuckStation (PS1), PPSSPP (PSP), RPCS3 (PS3), shadPS4 (PS4), Vita3K (PS Vita)
- **Nintendo:** Dolphin (GameCube / Wii), Cemu (Wii U), Ryujinx, yuzu, Eden, Suyu, Citron y Sudachi (Switch), Citra / Azahar (3DS), melonDS (DS), mGBA (GBA), Project64 (N64)
- **Otros:** RetroArch (multisistema), xemu (Xbox), Flycast (Dreamcast)

Como Hoard localiza las carpetas de guardado con la misma base de datos comunitaria que utiliza Ludusavi, muchas rutas se detectan automáticamente. Para cualquier ruta personalizada, puedes apuntar Hoard a una carpeta a mano.

## Configura la copia de partidas de emulador

1. **Instala Hoard** para Windows, macOS o Linux e inicia sesión.
2. Abre la **Biblioteca** y añade tu emulador, o añade manualmente su carpeta de guardados/estados si has cambiado la ubicación por defecto.
3. Mantén el **modo automático** activado. Hoard hace la copia tras cada sesión y guarda un historial versionado.
4. Instala Hoard en tus otros PC con la misma cuenta para sincronizar esas partidas en todas partes; mira [cómo sincronizar partidas entre PC](/guides/sync-game-saves-across-pcs).

## ¿Ludusavi para emuladores?

Ludusavi también puede hacer copia de partidas de emulador en local, y es una gran opción gratuita para eso. Si además quieres que esas partidas de emulador se sincronicen automáticamente entre equipos y mantengan un historial de versiones en la nube sin configurar Rclone, ahí es donde ayuda Hoard; lee la [comparativa completa entre Ludusavi y Hoard](/guides/ludusavi-alternative).

## Partidas en la nube para cada emulador

Ninguno de los emuladores sueltos de abajo sincroniza partidas entre máquinas por su cuenta: las partidas son ficheros normales en tu disco. Eso es buena noticia, porque cualquier herramienta que vigile la carpeta correcta puede llevarlas. Aquí tienes dónde guarda cada uno. «Steam Deck» se refiere a la versión Flatpak que se instala desde la tienda Discover.

### Partidas de PCSX2 en la nube (PS2)

PCSX2 escribe las memory cards (ficheros `.ps2`) en `memcards/`:

- Windows: `Documentos\PCSX2\memcards`
- Linux: `~/.config/PCSX2/memcards`
- Steam Deck: `~/.var/app/net.pcsx2.PCSX2/config/PCSX2/memcards`

Una memory card guarda las partidas de todos los juegos que hayas jugado en ella, así que viaja como una sola pieza: restaurar una versión anterior devuelve atrás la tarjeta entera, no un juego suelto.

La guía completa, con tarjetas de fichero y de carpeta: [partidas de PCSX2 en la nube](/guides/pcsx2-cloud-saves).

### Partidas de Dolphin en la nube (GameCube y Wii)

Las partidas de GameCube viven en `GC/` (imágenes de memory card o una carpeta por tarjeta) y las de Wii en la NAND emulada, en `Wii/`:

- Windows: `Documentos\Dolphin Emulator\GC` y `\Wii`
- Linux: `~/.local/share/dolphin-emu/GC` y `/Wii`
- Steam Deck: `~/.var/app/org.DolphinEmu.dolphin-emu/data/dolphin-emu/`

La guía completa, con carpetas GCI y la memoria de la Wii: [partidas de Dolphin en la nube](/guides/dolphin-cloud-saves).

### Partidas de DuckStation en la nube (PS1)

DuckStation guarda las memory cards en `memcards/` y, por defecto, crea una tarjeta distinta para cada juego, algo que encaja muy bien con la sincronización:

- Windows: `Documentos\DuckStation\memcards` (las versiones recientes usan `%LOCALAPPDATA%\DuckStation\memcards`)
- Linux: `~/.local/share/duckstation/memcards`
- Steam Deck: `~/.var/app/org.duckstation.DuckStation/`, dentro de `data/` o `config/`

### Sincronizar partidas de RetroArch

RetroArch separa `saves/` (las partidas del juego) de `states/` (los estados guardados). Hoard rastrea la carpeta de partidas; añade `states/` como entrada propia si juegas con estados:

- Windows: `%APPDATA%\RetroArch`, o junto a `retroarch.exe` en una instalación portable
- Linux: `~/.config/retroarch`
- Steam Deck: `~/.var/app/org.libretro.RetroArch/config/retroarch`, o `~/Emulation/saves/retroarch` si lo instalaste con EmuDeck

RetroArch tiene además un Cloud Sync integrado que habla con un servidor WebDAV que pones tú. Es una opción razonable si sólo usas RetroArch y ya tienes WebDAV. Hoard no necesita WebDAV, guarda un historial de versiones al que volver y cubre también los emuladores sueltos.

### PPSSPP (PSP)

Las partidas van a `PSP/SAVEDATA` y los estados a `PSP/PPSSPP_STATE`:

- Windows: `Documentos\PPSSPP\PSP\SAVEDATA`, o `memstick\PSP\SAVEDATA` junto al ejecutable en una instalación portable
- Linux: `~/.config/ppsspp/PSP/SAVEDATA`
- Steam Deck: `~/.var/app/org.ppsspp.PPSSPP/config/ppsspp/PSP/SAVEDATA`

### RPCS3 (PS3)

Las partidas viven en `dev_hdd0/home/00000001/savedata`, dentro de la carpeta de RPCS3 en Windows y bajo `~/.config/rpcs3/` en Linux y Steam Deck.

### Emuladores de Switch: Ryujinx, yuzu, Eden, Suyu, Citron, Sudachi

Ryujinx guarda las partidas en `bis/user/save` (bajo `%APPDATA%\Ryujinx` o `~/.config/Ryujinx`). La familia de yuzu usa `nand/user/save` dentro de su propia carpeta en `%APPDATA%` o `~/.local/share`.

Aquí hay una trampa. El árbol de tipo yuzu va `save/<cuenta>/<perfil>/<id-del-juego>/`, y el identificador de perfil se genera la primera vez que arranca el emulador, así que es distinto en cada instalación. Si sincronizas la carpeta `save/` entera entre dos máquinas, cada una acaba con el perfil de la otra al lado del suyo, y ningún juego ve el progreso del otro. Hoard baja hasta la carpeta propia de cada juego, así que el mismo título casa entre máquinas se llame como se llame el perfil.

### Citra y Azahar (3DS)

Las partidas están muy abajo, en `sdmc/Nintendo 3DS/<id0>/<id1>/title/…`, y `id0`/`id1` salen de las claves de la consola emulada, así que también cambian en cada instalación. Hoard lo resuelve igual que el árbol de Switch: una entrada por juego, emparejada entre máquinas.

### El resto

- **Cemu (Wii U):** `mlc01/usr/save`, bajo `%APPDATA%\Cemu` o `~/.local/share/Cemu`.
- **shadPS4 (PS4):** `savedata`, bajo `%APPDATA%\shadPS4` o `~/.local/share/shadPS4`.
- **Vita3K (PS Vita):** `ux0/user/00/savedata` dentro de su carpeta de datos.
- **mGBA, melonDS y la mayoría de emuladores de la época de cartuchos:** un `.sav` junto a la ROM, salvo que les hayas dicho otra cosa. Añade a mano las partidas de la carpeta de ROMs.

## Partidas de emulador en una Steam Deck

En una Steam Deck los emuladores suelen venir de Flatpak, así que sus carpetas están bajo `~/.var/app/<id>/` en vez de en los habituales `~/.config` o `~/.local/share`. EmuDeck lo reúne todo en `~/Emulation/saves/`, una carpeta por emulador. En cualquier caso, añades la carpeta una vez y Hoard la vigila.

Lo que importa en una portátil: el motor de Hoard corre como un servicio en segundo plano, así que hace la copia al salir de un juego en el modo Juego sin ninguna ventana abierta. Coges la Deck después de una sesión en el sobremesa y la partida ya está ahí.

## Partida guardada y estado guardado no son lo mismo

Vale la pena separarlos, porque se comportan distinto cuando viajan:

- Una **partida guardada** (`.srm`, una memory card, una carpeta `SAVEDATA`) es el guardado propio del juego, escrito por la consola emulada. Se mueve entre máquinas y entre versiones del emulador sin protestar.
- Un **estado guardado** es un volcado de la memoria del emulador. Está atado a esa compilación, y a menudo al núcleo exacto, así que un estado escrito por una versión puede negarse a cargar en otra.

Hoard copia los dos. Sólo que no te sorprenda que un estado de una máquina actualizada no abra en una que se quedó atrás: mantén los emuladores en versiones iguales y apóyate en las partidas guardadas para lo que te importe.

## Un emulador, muchos juegos

Un emulador es un solo proceso que aloja decenas de títulos, y eso es lo que vuelve incómodas las partidas de emulador para una herramienta que piensa en términos de «el juego que está corriendo». Hoard mantiene los títulos separados en lugar de tratar el emulador entero como un único bulto, así que cada juego tiene su propio historial y no un montón común que cambia cada vez que abres cualquier cosa. Si una partida se estropea, puedes [volver a una versión anterior](/guides/restore-a-game-save).

## Partidas de emulador sin pasar por nuestros servidores

Todo esto funciona igual contra tu propio servidor: levanta `hoard-server`, apunta la aplicación ahí, y tus partidas van de tu máquina a tu disco. Sin cuenta con nosotros, sin telemetría hacia nosotros, nada por nuestros servidores. Mira [cómo autoalojar Hoard](/guides/self-host-hoard).

## Consejo

Los estados guardados dependen de una versión concreta del emulador. Mantén tus emuladores actualizados de forma coherente entre PC para que un estado sincronizado cargue bien en todas partes.

<!-- faq -->

## Preguntas frecuentes

### ¿Hoard copia también mis ROMs?

No. Rastrea carpetas de partidas, no ficheros de juego. Las ROMs son grandes, no cambian y ya las tienes: no hay nada que versionar.

### ¿PCSX2, Dolphin o DuckStation tienen partidas en la nube integradas?

No. Escriben las partidas en carpetas locales y te dejan la sincronización a ti. Apunta una herramienta de sincronización a las carpetas de arriba y las partidas te seguirán entre máquinas.

### ¿RetroArch tiene sincronización en la nube?

Sí, un Cloud Sync integrado que necesita un servidor WebDAV que montes o alquiles tú. Hoard es la alternativa si prefieres no configurar WebDAV, quieres un historial de versiones al que volver o también juegas con emuladores sueltos.

### ¿Funciona en una Steam Deck en el modo Juego?

Sí. El motor corre como un servicio en segundo plano, así que las partidas se copian al salir de un juego, sin ninguna ventana abierta. Las carpetas de Flatpak y de EmuDeck funcionan igual que cualquier otra.

### Mi emulador es portable. ¿Funciona igual?

Sí. Añade a mano la carpeta que está junto al ejecutable y Hoard la rastreará como cualquier otra ubicación de partidas. Es el montaje habitual en consolas de mano.

### ¿Puedo sincronizar estados guardados entre dos PC?

Puedes, y Hoard lo hará. Que un estado cargue depende de que los emuladores estén en la misma versión en las dos máquinas, y eso es una limitación del emulador, no de la sincronización. Las partidas guardadas no tienen ese problema.

### ¿Funcionará con un emulador que no está en la lista?

Casi seguro que sí. La detección cubre los habituales de forma automática, y cualquier otro lo añades apuntando Hoard a su carpeta de partidas.

### ¿Cambia algo con emuladores si me autoalojo?

No. La misma detección, las mismas versiones, la misma sincronización. Lo único tuyo es el almacenamiento.
