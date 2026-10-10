---
title: "Sincronizar partidas de RetroArch entre PC y Steam Deck"
description: "Sincroniza partidas y estados de RetroArch entre PC, Steam Deck y portátil: dónde están los .srm, Cloud Sync integrado frente a sync automático, y las trampas."
order: 14
updated: 2026-10-09
---

RetroArch guarda las partidas del juego como ficheros `.srm` en una carpeta `saves` y los estados en una carpeta `states`. Para sincronizarlos entre dispositivos puedes usar el Cloud Sync que trae RetroArch, con un servidor WebDAV que pongas tú, o una herramienta que vigile las dos carpetas. Hoard hace lo segundo de forma automática: respalda las dos carpetas cuando cierras RetroArch, las baja en tus otras máquinas, guarda todas las versiones y entiende las instalaciones de EmuDeck.

## Dónde guarda RetroArch las partidas

- **Windows:** `%APPDATA%\RetroArch\saves` y `\states`, o `saves` y `states` junto a `retroarch.exe` si lo instalaste en su propia carpeta.
- **Linux:** `~/.config/retroarch/saves`. El Flatpak usa `~/.var/app/org.libretro.RetroArch/config/retroarch/saves`.
- **Steam Deck con EmuDeck:** `~/Emulation/saves/retroarch/`, donde `saves` y `states` son enlaces a las carpetas reales. Hoard lee `retroarch.cfg` para saber adónde apuntan de verdad.
- **RetroDECK:** `~/retrodeck/saves` y `~/retrodeck/states` por defecto.
- **En cualquier otro sitio:** **Settings → Directory** muestra las carpetas que está usando RetroArch.

## Cuándo escribe RetroArch la partida de verdad

Esto pilla a mucha gente. RetroArch mantiene la partida del juego en memoria y solo escribe el `.srm` cuando cierras el juego o sales de RetroArch, salvo que tengas puesto **Settings → Saving → SaveRAM Autosave Interval**. Hasta entonces no hay nada en el disco, así que un cuelgue o una batería agotada se llevan todo desde la última escritura, y ninguna herramienta de sincronización puede mover una partida que no se ha escrito.

Pon un intervalo de autoguardado de unos segundos. Y antes de cambiar de dispositivo, **cierra RetroArch**, no solo el juego: Hoard respalda cuando RetroArch se ha cerrado, así que nunca copia una partida a medio escribir. En un Deck, suspender no cuenta como cerrar.

## Configura igual todos los dispositivos

- **Opciones de ordenación.** **Settings → Saving** puede repartir partidas y estados en subcarpetas por nombre de núcleo o por carpeta de contenido. Si un dispositivo ordena y el otro no, el fichero sincronizado cae en una carpeta donde RetroArch no mira. Usa los mismos ajustes en todos.
- **Nombres de las ROM.** El `.srm` se llama como la ROM: `Super Metroid (USA).sfc` guarda en `Super Metroid (USA).srm`. Una ROM con otro nombre en el otro dispositivo no lo encuentra.
- **El mismo núcleo.** Dos núcleos de la misma consola no siempre leen las partidas del otro. Elige uno por sistema y úsalo en todas partes.
- **Versiones del núcleo, para los estados.** Un estado es una foto de la memoria del núcleo y a menudo no carga en otra versión. Las partidas normales no tienen ese problema.

Otra trampa con los estados: un estado incluye la memoria del juego, partida incluida. Si cargas un estado antiguo, la siguiente escritura del `.srm` devuelve esa partida antigua. Si usas **Auto Load State**, sincroniza también los estados, para que el que viaje sea el más nuevo.

## ¿Cloud Sync de RetroArch o Hoard?

Siendo justos con los dos:

- **El Cloud Sync de RetroArch** viene integrado y sincroniza partidas y estados con un servidor WebDAV que montes o alquiles. Funciona también en Android e iOS, cosa que Hoard hoy no hace. Si ya tienes Nextcloud, que guarda versiones de los ficheros por su cuenta, encaja bien, y es la mejor opción si el móvil forma parte de tu montaje.
- **Hoard** no necesita servidor WebDAV. Respalda y sincroniza solo, guarda un historial de versiones al que volver, y cubre también tus emuladores independientes y tus juegos de PC. Trata la carpeta `saves` entera como un solo elemento, así que volver atrás restaura todos los juegos que hay dentro tal como estaban en ese momento. Antes de confirmar te enseña qué va a cambiar, y tus ficheros actuales se guardan primero.

Elige uno por carpeta. Dos herramientas escribiendo las mismas partidas es la receta de los conflictos.

## Configurarlo con Hoard

1. Instala Hoard en cada dispositivo e inicia sesión con la misma cuenta.
2. En la **Biblioteca**, añade RetroArch. Las partidas y los estados aparecen como dos elementos.
3. Iguala los ajustes de arriba en todos los dispositivos.
4. Juega, cierra RetroArch y sigue en el otro dispositivo.

¿Prefieres que se quede en casa? Levanta `hoard-server` en tu PC o tu NAS: sin cuenta con nosotros, sin telemetría hacia nosotros, sin nada pasando por nuestros servidores. Mira [cómo autoalojar Hoard](/guides/self-host-hoard).

<!-- faq -->

## Preguntas frecuentes

### ¿RetroArch tiene partidas en la nube?

Sí, un Cloud Sync integrado que necesita un servidor WebDAV. Hoard es la alternativa si no quieres montar uno, quieres un historial de versiones o también usas emuladores independientes.

### ¿Por qué no se ha sincronizado mi partida de RetroArch?

Casi siempre es una de tres cosas: RetroArch aún no había escrito el `.srm` (seguía abierto y sin intervalo de autoguardado), los dos dispositivos reparten las partidas en subcarpetas distintas, o las ROM tienen nombres diferentes.

### ¿Puedo sincronizar también los estados?

Sí. Hoard sigue `states` como un elemento propio. Que un estado cargue en el otro dispositivo depende de que los dos usen la misma versión del núcleo.

### ¿Funciona con EmuDeck y RetroDECK?

Sí. Hoard lee la configuración de RetroArch para seguir los enlaces de EmuDeck hasta las carpetas reales. En RetroDECK, añade `~/retrodeck/saves` y `~/retrodeck/states` si no aparecen solas.

### ¿Hoard sincroniza RetroArch en Android?

Hoy no. Hoard funciona en Windows, macOS, Linux y Steam Deck.
