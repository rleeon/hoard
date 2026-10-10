---
title: "Dónde están las partidas de Palworld (PC y Steam Deck)"
description: "Dónde guarda Palworld sus mundos en PC y Steam Deck, qué es cada fichero, cómo funcionan los mundos cooperativos y cómo copiar y sincronizar tus partidas."
order: 24
updated: 2026-10-09
---

En PC (Steam), Palworld guarda sus partidas en `%LOCALAPPDATA%\Pal\Saved\SaveGames\<tu ID de Steam>`, con una carpeta por mundo dentro. Es la ruta que da Pocketpair en su FAQ oficial. Debajo tienes la ruta de Steam Deck, qué hace cada fichero, cómo cambia el cooperativo y cómo tener tus mundos siempre copiados y sincronizados entre tu PC y tu Steam Deck.

## Dónde guarda Palworld las partidas

- **Windows, versión de Steam:** `%LOCALAPPDATA%\Pal\Saved\SaveGames\<tu ID de Steam>\<ID del mundo>`
- **Steam Deck y Linux** (Proton): `~/.local/share/Steam/steamapps/compatdata/1623730/pfx/drive_c/users/steamuser/AppData/Local/Pal/Saved/SaveGames/`

La carpeta del ID de Steam es un número largo ligado a tu cuenta de Steam. Dentro, cada mundo que has creado tiene su propia carpeta con un nombre hexadecimal largo. En una Steam Deck el juego corre con Proton, así que las partidas están dentro del prefijo de Proton que Steam mantiene para él; `1623730` es el ID del juego en Steam.

La versión de la app de Xbox / Game Pass guarda sus partidas en otro sitio, empaquetado, y no son los mismos ficheros que copiarías entre instalaciones de Steam.

## Qué hay en la carpeta de un mundo

- **`Level.sav`** es el mundo en sí: tu base, el mapa, los Pals colocados en él.
- **`LevelMeta.sav`** guarda el nombre y el resumen del mundo para el menú.
- **`Players\`** guarda un `.sav` por cada jugador que ha estado en ese mundo.
- **`LocalData.sav`** y **`WorldOption.sav`** guardan datos locales y los ajustes del mundo.
- **`backup\`** son las copias automáticas del mundo que hace el propio juego.

Los ajustes están en otro sitio: `Pal\Saved\Config\Windows\GameUserSettings.ini` guarda gráficos y controles, y `Pal\Saved\Logs` los registros. Ninguno forma parte de tu progreso.

Al hacer la copia, llévate **la carpeta del mundo entera**, no sólo `Level.sav`. El mundo y los ficheros de jugador van juntos, y restaurar uno sin el otro es como acaban los personajes desacompasados con el mundo en el que están.

## Cooperativo y servidores dedicados

En cooperativo, **el mundo vive en el PC del anfitrión**. Tu personaje en ese mundo es un fichero en la carpeta `Players` del anfitrión, no en tu máquina. Si el anfitrión pierde su partida, el progreso de todos en ese mundo se va con ella. En un servidor dedicado, el mundo vive en el servidor.

Así que, en un mundo compartido, la copia que importa es la de la carpeta del anfitrión.

## ¿Palworld tiene partidas en la nube?

La versión de Steam usa Steam Cloud, que mantiene al día el último estado de tus mundos entre máquinas con la misma cuenta. No guarda versiones anteriores, y la carpeta `backup\` del juego está en el mismo disco que la partida, así que un disco muerto se lleva las dos.

## Cópiala a mano

1. Cierra el juego del todo.
2. Copia tu carpeta del ID de Steam desde `SaveGames` (contiene todos tus mundos) a un sitio seguro.
3. Para restaurar, cierra el juego y vuelve a copiarla al mismo sitio.

## Copia y sincronización automáticas con Hoard

[Hoard](/download) copia tus mundos cada vez que dejas de jugar y guarda todas las versiones fuera de la máquina, así que un mundo corrupto o un disco perdido no es el final de una base en la que llevas semanas. También sincroniza los mundos entre tus PC y una Steam Deck.

1. Instala Hoard e inicia sesión, o apúntalo a [tu propio servidor](/guides/self-host-hoard).
2. Abre la **Biblioteca**. Palworld se detecta a partir de tu biblioteca de Steam y de la base de datos comunitaria de partidas.
3. Juega. Al salir, la primera versión aparece en el historial.

Si eres anfitrión en cooperativo, ésta es la máquina que importa: copia la del anfitrión y el mundo compartido queda cubierto. Para devolver un mundo atrás, [restaura una versión anterior](/guides/restore-a-game-save).

<!-- faq -->

## Preguntas frecuentes

### ¿Dónde están las partidas de Palworld en Steam Deck?

Dentro del prefijo de Proton: `~/.local/share/Steam/steamapps/compatdata/1623730/pfx/drive_c/users/steamuser/AppData/Local/Pal/Saved/SaveGames/`, en la carpeta con tu ID de Steam.

### ¿Dónde se guarda mi personaje en el mundo de un amigo?

En el PC del anfitrión, en la carpeta `Players` de ese mundo. Tu PC no guarda copia de los mundos que aloja otra persona.

### ¿Qué fichero es mi mundo?

`Level.sav`, pero copia la carpeta del mundo entera: los ficheros de jugador y el mundo tienen que ir juntos.

### ¿Palworld hace copia de mi mundo por su cuenta?

Guarda copias automáticas en la carpeta `backup` del mundo. Están en el mismo disco que la partida, así que te protegen de una partida mala, no de perder el disco.
