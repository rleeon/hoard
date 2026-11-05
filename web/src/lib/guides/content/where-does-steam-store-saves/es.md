---
title: "¿Dónde guarda Steam las partidas? Todas las ubicaciones en PC y Steam Deck"
description: "Steam tiene una carpeta userdata, pero la mayoría de juegos guarda en AppData, Documentos o su propia carpeta. Dónde mirar en Windows, Steam Deck y Mac."
order: 30
updated: 2026-11-05
---

Steam tiene una carpeta propia para partidas, `userdata\<account-id>\<AppID>\remote` dentro de la carpeta de Steam, pero la mayoría de juegos no la usa nunca. En la base de datos comunitaria de partidas, que conoce la ruta de Windows de 10.937 juegos de Steam, solo **1 de cada 10** guarda ahí. El resto escribe en AppData, en Documentos o en su propia carpeta de instalación, y Steam Cloud, cuando el juego lo tiene, recoge los ficheros estén donde estén.

## La carpeta de Steam: userdata

- **Windows:** `C:\Program Files (x86)\Steam\userdata\<account-id>\<AppID>\remote`
- **Steam Deck y Linux:** `~/.local/share/Steam/userdata/<account-id>/<AppID>/remote`
- **macOS:** `~/Library/Application Support/Steam/userdata/<account-id>/<AppID>/remote`

`<account-id>` es el número que Steam enseña como tu **código de amigo** (Amigos, y luego Añadir un amigo). Hay una carpeta por cada cuenta que ha iniciado sesión en esa máquina. `<AppID>` es el número de la dirección del juego en la tienda: `store.steampowered.com/app/413150` es Stardew Valley.

La carpeta de encima de `remote`, la del `<AppID>` a secas, guarda también `remotecache.vdf`. Es la contabilidad propia de Steam, no tu partida, y es normal que sea distinta en tu PC y en tu Deck.

## Dónde guardan de verdad la mayoría de juegos

Contado en esa misma base de datos el 5 de noviembre de 2026, el porcentaje de juegos de Steam que usa cada sitio (algunos usan más de uno):

- **`AppData\LocalLow`**, 23 %, sobre todo juegos hechos con Unity: `%USERPROFILE%\AppData\LocalLow\<desarrollador>\<juego>`
- **La carpeta de instalación del juego**, 22 %: `C:\Program Files (x86)\Steam\steamapps\common\<juego>`
- **`AppData\Local`**, 21 %: `%LOCALAPPDATA%\<desarrollador o juego>`
- **`Documentos`**, 15 %, a menudo dentro de `My Games`: `%USERPROFILE%\Documents\My Games\<juego>`
- **`AppData\Roaming`**, 12 %: `%APPDATA%\<juego>`
- **La carpeta `userdata` de Steam**, 10 %: mira arriba
- **`Juegos guardados`**, 3 %: `%USERPROFILE%\Saved Games\<desarrollador>\<juego>`

## En la Steam Deck: el prefijo de Proton

Los juegos de Windows corren en la Deck a través de Proton, y cada uno tiene su propio Windows en miniatura dentro de `~/.local/share/Steam/steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/`. AppData, Documentos y Juegos guardados viven ahí dentro, así que `%APPDATA%\Juego` en un PC pasa a ser `compatdata/<AppID>/pfx/drive_c/users/steamuser/AppData/Roaming/Juego` en la Deck. El prefijo aparece la primera vez que se abre el juego.

Los juegos con versión nativa de Linux se saltan el prefijo y suelen guardar en `~/.local/share` o `~/.config`. La carpeta de instalación es `~/.local/share/Steam/steamapps/common/<juego>`, o el mismo árbol `steamapps` en la tarjeta microSD si el juego está instalado ahí.

## Encuentra rápido las partidas de un juego

1. Busca el juego en la [lista de ubicaciones de partidas](/saves), o en su página de PCGamingWiki, en «Save game data location».
2. Si usa la carpeta de Steam, abre `userdata\<account-id>\<AppID>\remote` con el AppID de su página en la tienda.
3. Si no, guarda en el juego y ordena `%LOCALAPPDATA%`, `%USERPROFILE%\AppData\LocalLow`, `%APPDATA%` y `Documentos` por fecha de modificación. La carpeta que acaba de cambiar es la buena.

## Qué guarda Steam Cloud

Steam Cloud guarda la última partida en tu cuenta de Steam, en los juegos cuyo desarrollador lo activó. La [página de almacenamiento remoto](https://store.steampowered.com/account/remotestorage) de Steam lista todos los juegos con ficheros en tu Steam Cloud y te deja descargarlos.

Guarda el último estado, no un historial, así que una partida corrupta se sincroniza tan fielmente como una buena, y solo cubre juegos de Steam: nada de GOG, Epic, emuladores ni accesos directos que no son de Steam. Hay más en la [guía de alternativa a Steam Cloud](/guides/steam-cloud-alternative).

## Mantenlo copiado y sincronizado con Hoard

[Hoard](/download) es gratis y de código abierto. Encuentra las partidas de cada juego donde el juego las haya puesto, userdata, AppData, Documentos, la carpeta de instalación o un prefijo de Proton en la Deck, las copia cada vez que el juego guarda y otra vez al salir, guarda un historial al que puedes volver y las sincroniza entre tus PC y una Steam Deck. En la carpeta de Steam rastrea `<AppID>/remote/` y deja en paz `remotecache.vdf`, así que tu PC y tu Deck nunca parecen estar en desacuerdo.

1. Instala Hoard e inicia sesión en Hoard One, recomendado para todos, o apúntalo a [tu propio servidor](/guides/self-host-hoard).
2. Abre la **Biblioteca**. Tus juegos de Steam se detectan desde tu biblioteca de Steam y la base de datos comunitaria de partidas.
3. Juega. Al salir, la primera versión aparece en el historial.

## Hoard One, o tu propio servidor

Hoard es gratis de las dos maneras, y la aplicación es la misma en ambas:

- **Hoard One**, recomendado para todos. Nada que montar ni que mantener encendido: un servidor gestionado en la UE, gratis con 2 GB y tres dispositivos, sin tarjeta. [Pro](/pricing) solo añade espacio, 100 GB y dispositivos ilimitados, para cuando tus partidas no quepan.
- **Tu propio servidor.** Montas `hoard-server` en un PC, un NAS o una Raspberry Pi y todas tus máquinas sincronizan a través de él. Sin cuenta con nosotros, sin telemetría hacia nosotros y sin cupo nuestro: nada pasa por nuestros servidores, porque no hay nada nuestro en el camino. Mira [cómo montar tu propio servidor de Hoard](/guides/self-host-hoard).

El mismo programa, la misma detección, el mismo historial. Hoard One guarda todas las versiones para siempre; en tu propio servidor decides tú cuánto historial guardar.

<!-- faq -->

## Preguntas frecuentes

### ¿Dónde está la carpeta userdata de Steam?

Dentro de la carpeta de Steam: `C:\Program Files (x86)\Steam\userdata` en Windows si Steam está en su sitio por defecto, `~/.local/share/Steam/userdata` en una Steam Deck o en Linux, y `~/Library/Application Support/Steam/userdata` en un Mac.

### ¿Desinstalar un juego borra sus partidas?

Las partidas de la carpeta de instalación se van con ella. Las de AppData, Documentos o userdata suelen quedarse. En la Deck, desinstalar también puede borrar el prefijo de Proton del juego, y con él todas las partidas que hay dentro, así que haz una copia antes.

### ¿Puedo copiar a mano las partidas de Steam a otro PC?

Sí. Cierra el juego, copia la carpeta de partidas al mismo sitio en el otro PC y abre el juego allí. Si usa Steam Cloud, Steam puede preguntarte qué copia quedarte al abrirlo; elige la de fecha más reciente.

### ¿Hoard es gratis?

Sí. La aplicación, la línea de comandos y el servidor son gratis y de código abierto (AGPL-3.0), y montar tu propio servidor no cuesta más que la máquina. Hoard One, el servidor gestionado que recomendamos a todos, también es gratis con 2 GB y tres dispositivos; Pro añade espacio por 1,99 € al mes.

### ¿El self-hosting envía algo a Hoard?

No. En tu propio servidor no hay cuenta con nosotros ni telemetría hacia nosotros: tus partidas, tus usuarios y tus logs se quedan en tu máquina y nunca tocan las nuestras.
