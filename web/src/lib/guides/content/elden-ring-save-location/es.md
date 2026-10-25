---
title: "Dónde están las partidas de Elden Ring (PC y Steam Deck)"
description: "Dónde guarda Elden Ring ER0000.sl2 en PC y Steam Deck, qué contiene, las partidas de Seamless Co-op y Nightreign, y cómo copiar todos tus personajes."
order: 26
updated: 2026-10-18
---

En PC, Elden Ring guarda las partidas en `%APPDATA%\EldenRing\<tu ID de Steam>`, en un único fichero, `ER0000.sl2`, que contiene todos los personajes que has creado, hasta diez. Debajo tienes la ruta de Steam Deck, qué más hay en la carpeta, dónde guardan Seamless Co-op y Nightreign sus partidas, y cómo tenerlo todo copiado y sincronizado entre tu PC y tu Steam Deck.

## Dónde guarda Elden Ring las partidas

- **Windows (Steam):** `%APPDATA%\EldenRing\<tu ID de Steam>\ER0000.sl2`
- **Steam Deck y Linux** (Proton): `~/.local/share/Steam/steamapps/compatdata/1245620/pfx/drive_c/users/steamuser/AppData/Roaming/EldenRing/<tu ID de Steam>/`

`%APPDATA%` es `C:\Users\<tu usuario>\AppData\Roaming`: pega `%APPDATA%\EldenRing` en la barra de direcciones del Explorador de archivos para ir directo. La carpeta del ID de Steam es un número largo ligado a tu cuenta de Steam. En una Steam Deck el juego corre con Proton, así que la partida está dentro del prefijo de Proton que Steam mantiene para él; `1245620` es el ID del juego en Steam.

## Qué hay en la carpeta de partidas

- **`ER0000.sl2`** es tu progreso: todos los personajes, su equipo y por dónde van en el mundo, Shadow of the Erdtree incluido.
- **`ER0000.sl2.bak`** es una copia de seguridad que el juego guarda al lado.
- **`ER0000.co2`**, si lo ves, es del mod Seamless Co-op (más abajo).

Los ajustes gráficos no están en esta carpeta: viven un nivel más arriba, en `%APPDATA%\EldenRing\GraphicsConfig.xml`.

De cómo funciona el fichero salen dos cosas:

- **Los personajes no se copian ni se restauran de uno en uno.** Restaurar un `ER0000.sl2` antiguo devuelve todos los personajes a ese momento, no solo el que querías arreglar.
- **La partida va ligada a tu cuenta de Steam.** Tu ID de Steam va escrito dentro del fichero, y el juego no carga una partida hecha con otra cuenta. Pasar tu propia partida entre tus PC y una Steam Deck funciona sin problema.

## Seamless Co-op y Nightreign

### Seamless Co-op

El mod Seamless Co-op no toca `ER0000.sl2`: guarda su propia partida, `ER0000.co2`, en la misma carpeta, así que tus personajes con mod y sin mod son ficheros distintos. La comunidad del mod desaconseja copiar un `.co2` encima de la partida normal para jugar en línea.

### Elden Ring Nightreign

Nightreign es otro juego, con su propia carpeta: `%APPDATA%\Nightreign\<tu ID de Steam>\NR0000.sl2` en Windows, y la misma ruta dentro de su propio prefijo de Proton en una Steam Deck (`compatdata/2622380`). Nunca toca tus personajes de Elden Ring.

## ¿Elden Ring tiene partidas en la nube?

Sí, con Steam Cloud, que mantiene `ER0000.sl2` al día entre las máquinas de una misma cuenta. Solo guarda el último estado, sin versiones anteriores a las que volver, y el `.bak` del juego está en el mismo disco que la partida, así que un disco muerto se lleva los dos.

## Cópiala a mano

1. Cierra el juego del todo.
2. Copia tu carpeta del ID de Steam desde `%APPDATA%\EldenRing` a un sitio seguro. Contiene `ER0000.sl2`, su `.bak` y la partida de Seamless Co-op, si la tienes.
3. Para restaurar, cierra el juego y vuelve a copiar la carpeta al mismo sitio.

## Mantenlo copiado y sincronizado con Hoard

[Hoard](/download) es gratis y de código abierto. Copia las partidas de Elden Ring cada vez que el juego guarda y otra vez al salir, guarda un historial al que puedes volver y sincroniza la carpeta entre tus PC y una Steam Deck. Solo viajan los ficheros que han cambiado, comprimidos, y cada fichero se comprueba con su hash SHA-256 al volver.

1. Instala Hoard e inicia sesión en Hoard One, recomendado para todos, o apúntalo a [tu propio servidor](/guides/self-host-hoard).
2. Abre la **Biblioteca**. Elden Ring se detecta desde tu biblioteca de Steam y la base de datos comunitaria de partidas.
3. Juega. Al salir, la primera versión aparece en el historial.

Hoard se lleva la carpeta del ID de Steam entera, así que la partida de Seamless Co-op también va en cada versión. Cada versión son todos tus personajes en ese momento: para volver atrás, [restaura una versión anterior](/guides/restore-a-game-save), y las versiones más nuevas siguen en el historial por si cambias de idea.

## Hoard One, o tu propio servidor

Hoard es gratis de las dos maneras, y la aplicación es la misma en ambas:

- **Hoard One**, recomendado para todos. Nada que montar ni que mantener encendido: un servidor gestionado en la UE, gratis con 2 GB y tres dispositivos, sin tarjeta. [Pro](/pricing) solo añade espacio, 100 GB y dispositivos ilimitados, para cuando tus partidas no quepan.
- **Tu propio servidor.** Montas `hoard-server` en un PC, un NAS o una Raspberry Pi y todas tus máquinas sincronizan a través de él. Sin cuenta con nosotros, sin telemetría hacia nosotros y sin cupo nuestro: nada pasa por nuestros servidores, porque no hay nada nuestro en el camino. Mira [cómo montar tu propio servidor de Hoard](/guides/self-host-hoard).

El mismo programa, la misma detección, el mismo historial. Hoard One guarda todas las versiones para siempre; en tu propio servidor decides tú cuánto historial guardar.

<!-- faq -->

## Preguntas frecuentes

### ¿Puedo usar mi partida de Elden Ring en otra cuenta de Steam?

Tal cual, no. Tu ID de Steam va escrito dentro de `ER0000.sl2`, y el juego no carga una partida hecha con otra cuenta. Tu propia partida, en tu propia cuenta, pasa sin problema entre tus PC y una Steam Deck.

### ¿Restaurar una partida antigua devuelve atrás a todos mis personajes?

Sí. Todos tus personajes comparten `ER0000.sl2`, así que cualquier restauración los devuelve a todos a ese momento. Con Hoard las versiones más nuevas siguen en el historial, así que volver atrás nunca es un viaje sin retorno.

### ¿Hoard copia las partidas de Seamless Co-op?

Sí. El mod guarda `ER0000.co2` en la misma carpeta del ID de Steam que la partida normal, y Hoard copia la carpeta entera, así que las dos van en cada versión.

### ¿Hoard es gratis?

Sí. La aplicación, la línea de comandos y el servidor son gratis y de código abierto (AGPL-3.0), y montar tu propio servidor no cuesta más que la máquina. Hoard One, el servidor gestionado que recomendamos a todos, también es gratis con 2 GB y tres dispositivos; Pro añade espacio por 1,99 € al mes.

### ¿El self-hosting envía algo a Hoard?

No. En tu propio servidor no hay cuenta con nosotros ni telemetría hacia nosotros: tus partidas, tus usuarios y tus logs se quedan en tu máquina y nunca tocan las nuestras.
