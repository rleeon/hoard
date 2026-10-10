---
title: "Dónde están las partidas de Marvel's Spider-Man 2 (PC y Steam Deck)"
description: "Dónde guarda Marvel's Spider-Man 2 sus partidas en PC, la carpeta del número largo, la trampa de OneDrive, la ruta en Steam Deck, copia y sincronización."
order: 22
updated: 2026-10-09
---

En PC, Marvel's Spider-Man 2 guarda sus partidas en `Documentos\Marvel's Spider-Man 2\`, dentro de una subcarpeta con un número largo. En Steam, ese número es tu ID de Steam. Debajo tienes qué significa eso en la práctica, la trampa de OneDrive, la ruta en Steam Deck y cómo tener las partidas siempre copiadas y sincronizadas entre tu PC y tu Steam Deck.

## Dónde guarda Marvel's Spider-Man 2 las partidas

- **Windows:** `%USERPROFILE%\Documents\Marvel's Spider-Man 2\<número largo>`
- **Steam Deck y Linux** (Proton): `~/.local/share/Steam/steamapps/compatdata/2651280/pfx/drive_c/users/steamuser/Documents/Marvel's Spider-Man 2/<número largo>`

La versión de PC es sólo para Windows, así que en una Steam Deck corre con Proton y las partidas están dentro del prefijo de Proton que Steam mantiene para el juego; `2651280` es su ID en Steam. Si está instalado en la microSD, la carpeta `compatdata` está en la tarjeta.

Nixxes, el estudio del port de PC, describe la carpeta de partidas como «una subcarpeta con un número largo o una combinación de letras y números» dentro de `Documentos\Marvel's Spider-Man 2\`.

## La carpeta del número largo

La subcarpeta lleva el nombre de tu cuenta: en Steam es tu **ID de Steam de 64 bits**; la versión de Epic usa en su lugar una mezcla de letras y números. En cualquier caso es distinta para cada cuenta. Dos consecuencias:

- Si dos personas juegan en el mismo PC con cuentas de Steam distintas, cada una tiene su propia carpeta de partidas.
- Si copias partidas a otro PC a mano, ponlas en la carpeta de la cuenta de Steam de **esa** máquina. En una carpeta con otro ID, el juego no las ve.

La carpeta padre `Marvel's Spider-Man 2` también guarda el registro del juego y los volcados de errores (`.log`, `.mdmp`). No son partidas y no hace falta copiarlos.

## La trampa de OneDrive

Muchos PC con Windows redirigen `Documentos` a OneDrive. Si es tu caso, la ruta real es `%USERPROFILE%\OneDrive\Documents\Marvel's Spider-Man 2\`, y OneDrive sincroniza la carpeta por su cuenta mientras juegas. Eso trae dos problemas: OneDrive puede subir una partida a medio escribir, y «Liberar espacio» puede convertir la partida en un marcador que sólo está en línea. Si dependes de OneDrive aquí, marca la carpeta como **Mantener siempre en este dispositivo**.

## ¿Spider-Man 2 tiene partidas en la nube?

Sí, Steam Cloud, que mantiene al día las últimas partidas entre máquinas con la misma cuenta de Steam. No guarda versiones anteriores: si una partida se rompe, lo que se sincroniza es la rota.

## Cópiala a mano

1. Cierra el juego del todo.
2. Copia la carpeta `Marvel's Spider-Man 2` de `Documentos` a un sitio seguro.
3. Para restaurar, cierra el juego y vuelve a copiar la carpeta del número largo al mismo sitio, con la misma cuenta de Steam.

## Copia y sincronización automáticas con Hoard

[Hoard](/download) copia la carpeta de partidas cada vez que dejas de jugar y guarda todas las versiones. También la sincroniza entre tus PC y una Steam Deck, así que el juego sigue donde lo dejaste en cualquiera de los dos.

1. Instala Hoard e inicia sesión, o apúntalo a [tu propio servidor](/guides/self-host-hoard).
2. Abre la **Biblioteca** y comprueba que la carpeta que aparece para Spider-Man 2 es la de `Documentos` (o `OneDrive\Documents`). Si apunta a otro sitio, cámbiala a esa carpeta.
3. Juega. Al salir, la primera versión aparece en el historial.

Si más adelante una partida se estropea, [restaurar una versión anterior](/guides/restore-a-game-save) la devuelve.

<!-- faq -->

## Preguntas frecuentes

### ¿Qué es el número largo de la carpeta de partidas?

En Steam, tu ID de Steam de 64 bits; en Epic, el ID de tu cuenta. Cada cuenta tiene su propia carpeta, y el juego sólo lee la de la cuenta con la que has iniciado sesión.

### ¿Dónde están las partidas de Spider-Man 2 en Steam Deck?

Dentro del prefijo de Proton: `~/.local/share/Steam/steamapps/compatdata/2651280/pfx/drive_c/users/steamuser/Documents/Marvel's Spider-Man 2/`, en la carpeta del número largo.

### No encuentro la carpeta en Documentos. ¿Dónde está?

Mira en `OneDrive\Documents\Marvel's Spider-Man 2`. En la mayoría de instalaciones nuevas de Windows, Documentos vive dentro de OneDrive.

### ¿Puedo copiar mis partidas al PC de un amigo?

Los ficheros se copian, pero van en la carpeta con el ID de Steam de la cuenta de ese PC. Que el juego acepte partidas hechas con otra cuenta depende del juego, así que guarda una copia del original antes de probar.
