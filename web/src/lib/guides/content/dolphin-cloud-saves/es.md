---
title: "Partidas en la nube para Dolphin: sincroniza GameCube y Wii entre PC y Steam Deck"
description: "Dolphin no tiene partidas en la nube. Sincroniza GameCube y Wii entre PC y Steam Deck automáticamente, con historial: rutas, tarjetas y trampas."
order: 17
updated: 2026-10-09
---

Dolphin no sincroniza las partidas entre máquinas: tus tarjetas de memoria de GameCube y tu Wii emulada viven en una carpeta de un solo PC. Hoard las sincroniza de forma automática. Cuando cierras Dolphin, respalda tus partidas de GameCube y Wii, las baja en tus otros PC y en tu Steam Deck, y guarda todas las versiones para que siempre puedas volver atrás.

## Dónde guarda Dolphin tus partidas

Todo vive en la carpeta de usuario de Dolphin. La forma más rápida de encontrarla es **File → Open User Folder** dentro de Dolphin. Dentro hay:

- `GC`, con las tarjetas de memoria de GameCube.
- `Wii`, la memoria interna de la Wii emulada, partidas incluidas.
- `StateSaves`, con los estados.

Dónde está esa carpeta:

- **Windows:** `Documents\Dolphin Emulator`. Las instalaciones más recientes pueden usar `%APPDATA%\Dolphin Emulator`, y una instalación portátil tiene una carpeta `User` junto a `Dolphin.exe`.
- **Linux:** `~/.local/share/dolphin-emu`.
- **Steam Deck** (el Flatpak de Discover): `~/.var/app/org.DolphinEmu.dolphin-emu/data/dolphin-emu`.
- **Mac:** `~/Library/Application Support/Dolphin`.

Hoard encuentra solo las carpetas de `Documents`, Linux y Steam Deck. Para `%APPDATA%`, una instalación portátil o un Mac, señala las carpetas `GC` y `Wii` una vez.

## GameCube: tarjetas de fichero o carpetas GCI

En **Options → Configuration → GameCube**, cada ranura de tarjeta de memoria puede ser una de dos cosas:

- **Un fichero de tarjeta**, una imagen en bruto como `MemoryCardA.USA.raw` con las partidas de todos los juegos dentro. Cualquier guardado reescribe el fichero entero.
- **Una carpeta GCI**, donde cada partida es su propio fichero `.gci`, en una carpeta como `GC/USA/Card A`. Solo es nueva la partida que ha cambiado, así que las versiones pesan poco y se leen fácil.

Para sincronizar, encajan mejor las carpetas GCI. En cualquier caso, **usa el mismo ajuste en todas las máquinas**: un fichero de tarjeta en un PC y una carpeta GCI en el otro hace que cada uno vea la tarjeta vacía. Si necesitas pasar partidas de un tipo a otro, **Tools → Memory Card Manager** de Dolphin importa y exporta ficheros `.gci`.

Las tarjetas también van **por región** (USA, EUR, JAP). Una copia PAL y una NTSC del mismo juego no ven las partidas de la otra, así que usa la misma imagen de disco en todas partes.

## Wii: la memoria de la consola emulada

Las partidas de Wii viven dentro de la carpeta `Wii`, en `Wii/title/00010000/<ID del juego>/data` para los juegos en disco. Esa carpeta es toda la memoria de la consola emulada: partidas, Miis, ajustes del sistema y cualquier canal que hayas instalado. Hoard la respalda como un solo elemento, así que restaurar una versión devuelve la memoria de la consola tal como estaba en ese momento. Antes de confirmar, Hoard te enseña qué va a cambiar, y tus ficheros actuales se guardan primero.

Si solo quieres pasar una partida de Wii a mano, Dolphin puede exportarla: clic derecho sobre el juego en la lista y **Export Wii Save**.

## Cómo funciona la sincronización en el día a día

Juegas en el sobremesa y cierras Dolphin. Hoard espera a que Dolphin haya salido y a que las carpetas se queden quietas, y sube la versión nueva. Más tarde coges la Steam Deck; en cuanto tiene conexión, Hoard baja las partidas más nuevas. Ciérralo en el Deck y pasa lo mismo en sentido contrario. Ninguna de las dos máquinas tiene que estar encendida a la vez que la otra.

## Trampas que conviene conocer

- **Cierra Dolphin, no solo el juego.** Hoard respalda cuando el emulador ha salido. En un Deck, suspender no cuenta como cerrar.
- **Los estados son frágiles.** Los estados de Dolphin suelen romperse entre versiones de Dolphin. Hoard sincroniza las partidas de verdad; si también quieres `StateSaves`, añádela como elemento propio y ten Dolphin en la misma versión en todas partes.
- **Rutas personalizadas.** Si cambiaste la raíz de la NAND de Wii o la ruta de las carpetas GCI en **Options → Configuration → Paths**, señala esas carpetas en Hoard.

## Configurarlo

1. Instala Hoard en cada máquina e inicia sesión con la misma cuenta.
2. En la **Biblioteca**, añade Dolphin desde la lista de emuladores.
3. Usa el mismo ajuste de tarjeta y la misma región en todas las máquinas.
4. Juega, cierra Dolphin y sigue en la otra máquina.

¿Prefieres que se quede en casa? Levanta `hoard-server` en tu PC o tu NAS y apunta a él todas las máquinas. Sin cuenta con nosotros, sin telemetría hacia nosotros, sin nada pasando por nuestros servidores. Mira [cómo autoalojar Hoard](/guides/self-host-hoard). Para el resto de emuladores, mira [partidas de emuladores](/guides/back-up-emulator-saves).

<!-- faq -->

## Preguntas frecuentes

### ¿Dolphin tiene partidas en la nube?

No. Dolphin guarda las partidas en una carpeta local y la sincronización te la deja a ti. Hoard es una forma de sincronizarlas de forma automática, con historial de versiones encima.

### ¿Puedo sincronizar las partidas de Dolphin entre un PC y una Steam Deck?

Sí. Instala Hoard en los dos con la misma cuenta. Hoard sabe dónde guarda Dolphin sus partidas en Windows, en Linux y en el Flatpak de la Steam Deck, y las empareja entre máquinas.

### ¿Uso fichero de tarjeta o carpeta GCI?

Para sincronizar, carpeta GCI: cada partida es su propio fichero, así que las versiones pesan poco y enseñan qué juego cambió. Elijas lo que elijas, usa lo mismo en todas las máquinas.

### ¿Sincroniza también las partidas de Wii?

Sí. La carpeta `Wii` guarda la memoria de la consola emulada, partidas incluidas, y Hoard la respalda y sincroniza igual que las tarjetas de GameCube.

### ¿Hoard sincroniza los estados de Dolphin?

No por defecto, porque los estados se rompen entre versiones de Dolphin. Añade la carpeta `StateSaves` a mano si los quieres.

### ¿Funciona con Dolphin en Android?

Hoy no. Hoard funciona en Windows, macOS, Linux y Steam Deck.
