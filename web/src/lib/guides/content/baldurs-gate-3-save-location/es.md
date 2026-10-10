---
title: "Dónde están las partidas de Baldur's Gate 3 (PC y Steam Deck)"
description: "Dónde guarda Baldur's Gate 3 sus partidas en Windows, Steam Deck y Mac, qué es partida y qué son mods o ajustes, el modo Honor, copia y sincronización."
order: 23
updated: 2026-10-09
---

En Windows, Baldur's Gate 3 guarda sus partidas en `%LOCALAPPDATA%\Larian Studios\Baldur's Gate 3\PlayerProfiles\Public\Savegames\Story`, una carpeta por partida. Es la ruta que da Larian en su propio FAQ de soporte. Debajo tienes las rutas de Steam Deck y Mac, lo que hay junto a las partidas, el modo Honor y cómo tenerlo todo copiado y sincronizado entre tu PC y tu Steam Deck.

## Dónde guarda Baldur's Gate 3 las partidas

- **Windows:** `%LOCALAPPDATA%\Larian Studios\Baldur's Gate 3\PlayerProfiles\Public\Savegames\Story`
- **Steam Deck y Linux** (Proton): `~/.local/share/Steam/steamapps/compatdata/1086940/pfx/drive_c/users/steamuser/AppData/Local/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story`
- **Mac:** `~/Documents/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story`

Larian retiró la versión nativa para Linux, así que en una Steam Deck el juego corre con Proton y las partidas están dentro del prefijo de Proton que Steam mantiene para él; `1086940` es el ID del juego en Steam. Si está instalado en la microSD, la carpeta `compatdata` está en la tarjeta.

## Qué es partida y qué no

Cada partida es **una carpeta** dentro de `Story`, con un fichero `.lsv` y una miniatura. Lo que hay alrededor es otra cosa:

- **`Mods`** (dentro de `Baldur's Gate 3`) guarda los ficheros de los mods.
- **`modsettings.lsx`** (dentro de `PlayerProfiles\Public`) es la lista de mods activos y su orden de carga.
- **Los ajustes**, como gráficos y controles, son ficheros de configuración junto al perfil, no parte de una partida.

El que pilla a la gente son los mods. Una partida hecha con mods espera que esos mismos mods estén activos al cargarla. Si llevas una partida con mods a otro PC, llévate también la lista de mods, o el juego avisará de que faltan y puede que la partida no cargue como esperas.

## El modo Honor

El modo Honor mantiene una única partida que el juego sobrescribe mientras juegas, y si tu grupo cae, la partida de Honor se acaba (puedes seguir en modo Personalizado, sin el Honor). Copiar esa partida es decisión tuya: una copia de antes de un combate difícil es, técnicamente, una vuelta atrás, y hay jugadores que la quieren justo para un cuelgue o un fallo del juego, mientras otros lo ven como hacer trampa al modo. Una herramienta de copias guarda las versiones igual; si alguna vez restauras una, es cosa tuya y de los dados.

## ¿Baldur's Gate 3 tiene partidas en la nube?

Sí. En Steam usa Steam Cloud, que mantiene al día las últimas partidas entre máquinas con la misma cuenta. Sólo guarda el estado actual: si una partida se corrompe o la actualización de un mod la rompe, ésa es la versión que se sincroniza.

## Cópiala a mano

1. Cierra el juego del todo.
2. Copia la carpeta `PlayerProfiles` entera desde la ruta de arriba (contiene `Savegames` y `modsettings.lsx`).
3. Para restaurar, cierra el juego y vuelve a copiarla.

## Copia y sincronización automáticas con Hoard

[Hoard](/download) copia la carpeta de partidas cada vez que dejas de jugar y guarda todas las versiones, así que una partida rota por la actualización de un mod o por un parche está a una restauración. También mantiene la carpeta sincronizada entre tus PC y una Steam Deck.

1. Instala Hoard e inicia sesión, o apúntalo a [tu propio servidor](/guides/self-host-hoard).
2. Abre la **Biblioteca**. Baldur's Gate 3 se detecta a partir de tu biblioteca de Steam y de la base de datos comunitaria de partidas.
3. Juega. Al salir, la primera versión aparece en el historial.

Cada sesión añade una versión con todas las partidas dentro. Para volver atrás, abre el historial y [restaura una versión anterior](/guides/restore-a-game-save); lo que tienes ahora en el PC se copia antes, así que probar una vieja nunca es un viaje sin vuelta.

<!-- faq -->

## Preguntas frecuentes

### ¿Dónde están las partidas de Baldur's Gate 3 en Steam Deck?

Dentro del prefijo de Proton del juego: `~/.local/share/Steam/steamapps/compatdata/1086940/pfx/drive_c/users/steamuser/AppData/Local/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story`.

### ¿Puedo pasar una partida con mods a otro PC?

Sí, siempre que el otro PC tenga los mismos mods instalados y activos en el mismo orden. Copia `modsettings.lsx` junto con la partida e instala los mismos ficheros de mods.

### ¿Puedo hacer copia de una partida del modo Honor?

La partida es una carpeta normal, así que sí, cualquier herramienta de copias puede copiarla. Si restaurarla encaja con el espíritu del modo, lo decides tú.

### ¿Por qué mi partida dice que faltan mods?

Se hizo con mods que ahora no están activos. Vuelve a activar los mismos mods, en el mismo orden, y cargará con normalidad.
