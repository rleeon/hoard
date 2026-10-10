---
title: "Dónde están las partidas de Crimson Desert (PC y Steam Deck)"
description: "Dónde guarda Crimson Desert sus partidas en Windows, Steam Deck y Mac, qué carpeta las contiene de verdad y cómo copiarlas y sincronizarlas entre tus PC."
order: 21
updated: 2026-10-09
---

En Windows, Crimson Desert guarda sus partidas en `%LOCALAPPDATA%\Pearl Abyss\CD\save`. Es la carpeta que indica Pearl Abyss en su propio FAQ. Debajo tienes las rutas de Steam Deck y Mac, qué hay dentro y cómo tenerla siempre copiada y sincronizada entre tu PC y tu Steam Deck.

## Dónde guarda Crimson Desert las partidas

- **Windows:** `%LOCALAPPDATA%\Pearl Abyss\CD\save` (es decir, `C:\Users\<tú>\AppData\Local\Pearl Abyss\CD\save`)
- **Steam Deck y Linux** (Proton): `~/.local/share/Steam/steamapps/compatdata/3321460/pfx/drive_c/users/steamuser/AppData/Local/Pearl Abyss/CD/save`
- **Mac, versión de Steam:** `~/Library/Application Support/Pearl Abyss/CD/save`
- **Mac, versión de la App Store:** `~/Library/Containers/com.pearlabyss.CrimsonDesert/Data/Library/Application Support/Pearl Abyss/CD/save`

No hay versión nativa para Linux, así que en una Steam Deck el juego corre con Proton y sus partidas están dentro del prefijo de Proton que Steam mantiene para él; `3321460` es el ID del juego en Steam. Si está en la microSD, la carpeta `compatdata` está en la tarjeta.

`AppData` es una carpeta oculta en Windows. Lo más rápido es pegar `%LOCALAPPDATA%\Pearl Abyss\CD\save` en la barra de direcciones del Explorador de archivos.

## Qué hay en la carpeta

Dentro de `save` hay dos subcarpetas. Según Pearl Abyss, **la que tiene un nombre numérico guarda las partidas que creas en el juego**. Al hacer la copia, llévate la carpeta `save` entera en lugar de elegir ficheros: ocupa poco y no te dejas nada que el juego necesite.

Las versiones de Steam y de la App Store en Mac usan rutas distintas. Si cambias de una a otra, copia las partidas a mano una vez.

## ¿Crimson Desert tiene partidas en la nube?

Sí, la versión de Steam tiene Steam Cloud, que mantiene al día las últimas partidas entre máquinas con la misma cuenta de Steam.

Lo que no hace:

- **Guardar versiones anteriores.** Steam Cloud guarda el estado actual. Si una partida se corrompe, lo que se sincroniza es la corrupta.
- **Cubrir otras tiendas.** Una copia de la App Store de Mac y una de Steam no comparten nube.

## Cópiala a mano

1. Cierra el juego del todo.
2. Copia la carpeta `save` entera desde la ruta de arriba a un sitio seguro: otro disco, un USB, una carpeta en la nube.
3. Para restaurar, cierra el juego y vuelve a copiarla, sustituyendo lo que haya.

Está bien como copia puntual antes de una actualización grande o una reinstalación. Como rutina depende de que te acuerdes, y sólo tienes la copia de la última vez.

## Copia y sincronización automáticas con Hoard

[Hoard](/download) copia la carpeta de partidas cada vez que dejas de jugar y guarda todas las versiones, así que puedes volver atrás desde una partida rota o una decisión de la que te arrepientes. También mantiene la carpeta sincronizada entre tus PC y una Steam Deck.

1. Instala Hoard e inicia sesión, o apúntalo a [tu propio servidor](/guides/self-host-hoard).
2. Abre la **Biblioteca**. Crimson Desert se detecta a partir de tu biblioteca de Steam y de la base de datos comunitaria de partidas, en la ruta de arriba.
3. Juega. Al salir, la primera versión aparece en el historial.

A partir de ahí cada sesión añade una versión, y la más nueva está en la máquina en la que te sientes después. Si una partida se estropea, [restaurar una anterior](/guides/restore-a-game-save) son un par de clics.

<!-- faq -->

## Preguntas frecuentes

### ¿Dónde están las partidas de Crimson Desert en Steam Deck?

Dentro del prefijo de Proton del juego: `~/.local/share/Steam/steamapps/compatdata/3321460/pfx/drive_c/users/steamuser/AppData/Local/Pearl Abyss/CD/save`. Si el juego está en la microSD, la carpeta `compatdata` está en la tarjeta.

### ¿Qué subcarpeta tiene mis partidas?

La del nombre numérico, dentro de `save`. Aun así, copia la carpeta `save` entera para no dejarte nada.

### No encuentro la carpeta AppData. ¿Dónde está?

Está oculta por defecto. Pega `%LOCALAPPDATA%\Pearl Abyss\CD\save` en la barra de direcciones del Explorador de archivos y pulsa Intro, o activa los elementos ocultos en el menú Vista.

### ¿Puedo jugar en el sobremesa y en la Steam Deck con la misma partida?

Sí. Steam Cloud lo hace con la última partida en la misma cuenta de Steam. Hoard también, y guarda una versión por sesión, así que puedes volver atrás si algo se rompe.
