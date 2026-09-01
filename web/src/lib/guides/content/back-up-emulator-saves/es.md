---
title: "Cómo hacer copia y sincronizar partidas de emuladores (RetroArch, Dolphin, PCSX2)"
description: "Haz copia y sincroniza los archivos de guardado y los estados guardados de tus emuladores entre PC —RetroArch, Dolphin, PCSX2, DuckStation y más— automáticamente con Hoard."
order: 6
updated: 2026-09-01
---

Las partidas de emulador se pierden con facilidad: los archivos de guardado y los estados guardados viven en carpetas dispersas, y una reinstalación o un PC nuevo pueden borrar años de progreso. Hoard hace la copia automáticamente y los mantiene sincronizados entre equipos.

## Emuladores con los que funciona Hoard

Hoard gestiona los archivos de guardado estándar de emulador (`.srm`, `.sav`, memory cards) y los estados guardados de los emuladores populares, entre ellos:

- **RetroArch** — guardados y estados por núcleo
- **Dolphin** (GameCube / Wii) — memory cards y archivos GCI
- **PCSX2** (PS2) — memory cards
- **DuckStation** (PS1), **PPSSPP** (PSP), **mGBA** y más

Como Hoard localiza las carpetas de guardado con la misma base de datos comunitaria que utiliza Ludusavi, muchas rutas de emulador se detectan automáticamente. Para cualquier ruta personalizada, puedes apuntar Hoard a una carpeta a mano.

## Configura la copia de partidas de emulador

1. **Instala Hoard** para Windows, macOS o Linux e inicia sesión.
2. Abre la **Biblioteca** y añade tu emulador, o añade manualmente su carpeta de guardados/estados si has cambiado la ubicación por defecto.
3. Mantén el **modo automático** activado. Hoard hace la copia tras cada sesión y guarda un historial versionado.
4. Instala Hoard en tus otros PC con la misma cuenta para sincronizar esas partidas en todas partes; mira [cómo sincronizar partidas entre PC](/guides/sync-game-saves-across-pcs).

## ¿Ludusavi para emuladores?

Ludusavi también puede hacer copia de partidas de emulador en local, y es una gran opción gratuita para eso. Si además quieres que esas partidas de emulador se sincronicen automáticamente entre equipos y mantengan un historial de versiones en la nube sin configurar Rclone, ahí es donde ayuda Hoard; lee la [comparativa completa entre Ludusavi y Hoard](/guides/ludusavi-alternative).

## Dónde guarda sus partidas cada emulador

Conviene saberlo, porque una instalación portable lo coloca todo en otro sitio:

- **RetroArch** — `saves/` y `states/` dentro de la carpeta de configuración: `%APPDATA%\RetroArch` en Windows, `~/.config/retroarch` en Linux.
- **Dolphin** — las memory cards en `GC/` y las partidas de Wii en la NAND emulada, dentro de `Documentos\Dolphin Emulator` o `~/.local/share/dolphin-emu`.
- **PCSX2** — `memcards/`, bajo `Documentos\PCSX2` o `~/.config/PCSX2`.
- **DuckStation** — `memcards/` y `savestates/` en su propia carpeta de datos.
- **PPSSPP** — `PSP/SAVEDATA` para las partidas y `PSP/PPSSPP_STATE` para los estados.
- **RPCS3** — `dev_hdd0/home/00000001/savedata`.
- **Cemu** — `mlc01/usr/save`.
- **mGBA y la mayoría de núcleos sueltos** — un `.sav` junto a la ROM, salvo que les hayas dicho otra cosa.

Una **instalación portable** — lo normal en consolas de mano y en llaves USB — guarda todo eso junto al ejecutable. Si ése es tu caso, apunta Hoard a esa carpeta y la rastreará como cualquier otra partida.

## Partida guardada y estado guardado no son lo mismo

Vale la pena separarlos, porque se comportan distinto cuando viajan:

- Una **partida guardada** (`.srm`, una memory card, una carpeta `SAVEDATA`) es el guardado propio del juego, escrito por la consola emulada. Se mueve entre máquinas y entre versiones del emulador sin protestar.
- Un **estado guardado** es un volcado de la memoria del emulador. Está atado a esa compilación, y a menudo al núcleo exacto, así que un estado escrito por una versión puede negarse a cargar en otra.

Hoard copia los dos. Sólo que no te sorprenda que un estado de una máquina actualizada no abra en una que se quedó atrás: mantén los emuladores en versiones iguales y apóyate en las partidas guardadas para lo que te importe.

## Un emulador, muchos juegos

Un emulador es un solo proceso que aloja decenas de títulos, y eso es lo que vuelve incómodas las partidas de emulador para una herramienta que piensa en términos de «el juego que está corriendo». Hoard mantiene los títulos separados en lugar de tratar el emulador entero como un único bulto, así que cada juego tiene su propio historial y no un montón común que cambia cada vez que abres cualquier cosa.

## Partidas de emulador sin pasar por nuestros servidores

Todo esto funciona igual contra tu propio servidor: levanta `hoard-server`, apunta la aplicación ahí, y tus partidas van de tu máquina a tu disco. Sin cuenta con nosotros, sin telemetría hacia nosotros, nada por nuestros servidores. Mira [cómo autoalojar Hoard](/guides/self-host-hoard).

## Consejo

Los estados guardados dependen de una versión concreta del emulador. Mantén tus emuladores actualizados de forma coherente entre PC para que un estado sincronizado cargue bien en todas partes.

<!-- faq -->

## Preguntas frecuentes

### ¿Hoard copia también mis ROMs?

No. Rastrea carpetas de partidas, no ficheros de juego. Las ROMs son grandes, no cambian y ya las tienes: no hay nada que versionar.

### Mi emulador es portable. ¿Funciona igual?

Sí. Añade a mano la carpeta que está junto al ejecutable y Hoard la rastreará como cualquier otra ubicación de partidas. Es el montaje habitual en consolas de mano.

### ¿Puedo sincronizar estados guardados entre dos PC?

Puedes, y Hoard lo hará. Que un estado cargue depende de que los emuladores estén en la misma versión en las dos máquinas, y eso es una limitación del emulador, no de la sincronización. Las partidas guardadas no tienen ese problema.

### ¿Funcionará con un emulador que no está en la lista?

Casi seguro que sí. La detección cubre los habituales de forma automática, y cualquier otro lo añades apuntando Hoard a su carpeta de partidas.

### ¿Cambia algo con emuladores si me autoalojo?

No. La misma detección, las mismas versiones, la misma sincronización. Lo único tuyo es el almacenamiento.
