---
title: "Alternativa a GameSave Manager: copia y sincronización automáticas, también en Linux"
description: "¿Buscas una alternativa a GameSave Manager? Hoard copia cada partida solo, la sincroniza entre tus PC y una Steam Deck, y funciona en Linux y macOS."
order: 27
updated: 2026-11-05
---

Hay tres programas distintos que se llaman Game Save Manager, y los tres copian partidas en un solo PC con Windows. Si quieres copias que se hagan solas y te sigan a tus otros PC, a una Steam Deck o a Linux, [Hoard](/download) es gratis, de código abierto y está hecho justo para eso.

## Qué Game Save Manager tienes

- **GameSave Manager**, el veterano de código cerrado para Windows. Tiene su propia base de datos de juegos, copia a un único archivo cuando se lo pides o de forma programada, y su **Sync & Link** mueve una carpeta de partidas a Dropbox, OneDrive o Google Drive y deja un enlace en su sitio.
- **Game Save Manager (GSM)**, de dyang886, GPL-3.0, para Windows. Encuentra los juegos instalados con la base de datos de PCGamingWiki, guarda por cada juego una serie de copias locales que rota y se lleva el historial a un PC nuevo en un archivo `.gsmr`.
- **game-save-manager**, de mcthesw, AGPL-3.0, para Windows y popular en comunidades chinas. Copia de forma programada las carpetas que añades, con WebDAV para tener una copia fuera de la máquina.

## Qué hacen bien

- **Sencillos y locales.** Eliges juegos, pulsas copiar y tienes un archivo. Sin cuenta y sin nada que configurar.
- **Buena detección en Windows.** Sus bases de datos grandes encuentran la mayoría de juegos instalados sin que tengas que buscar rutas.
- **Una mudanza limpia a un PC nuevo.** Un solo archivo exportado se lo lleva todo.

Si solo juegas en un PC con Windows y no te importa pulsar un botón, cualquiera de ellos cumple.

## Dónde llega más lejos Hoard

| | Herramientas Game Save Manager | Hoard |
|---|---|---|
| Cuándo copia | Cuando pulsas el botón, o de forma programada | Cada vez que el juego guarda, y otra vez al salir |
| Sincronización entre PC | Exportar e importar, o una carpeta en la nube con Sync & Link | Automática: la partida más nueva te espera en la siguiente máquina |
| El otro PC puede estar apagado | Solo a través de una carpeta en la nube | Sí, la partida espera en el servidor |
| Historial | Copias locales, o lo que guarde la nube | Todas las versiones, deduplicadas, en todas tus máquinas |
| Partidas a medio escribir | Un cliente de nube puede sincronizar una carpeta enlazada mientras el juego escribe | Nunca sube un fichero a medio escribir ni cambia la partida de un juego abierto |
| Plataformas | Windows | Windows, Linux, macOS y SteamOS |
| Licencia | Código cerrado, GPL-3.0 o AGPL-3.0 | AGPL-3.0, gratis |

La fila de Linux es la que busca la gente. Ninguna de las herramientas Game Save Manager funciona en Linux ni en una Steam Deck. Hoard sí, de forma nativa: la misma aplicación en los cuatro sistemas, con los prefijos de Proton y Wine detectados, así que la partida de un juego en tu PC con Windows y la del mismo juego en la Deck quedan como dos versiones de una misma historia.

## Cómo cambiarte

1. Instala [Hoard](/download) e inicia sesión en Hoard One, recomendado para todos, o apúntalo a [tu propio servidor](/guides/self-host-hoard).
2. Si usabas **Sync & Link**, devuelve antes la carpeta de partidas a su sitio original, para que un cliente de nube y Hoard no sincronicen los mismos ficheros.
3. Abre la **Biblioteca**. Hoard encuentra tus juegos con la base de datos comunitaria de partidas y tus launchers; añade a mano cualquier carpeta que se le escape.
4. Juega. La primera versión aparece en el historial en cuanto el juego guarda. Instala Hoard en tus otras máquinas con la misma cuenta y la [sincronización entre PC](/guides/sync-game-saves-across-pcs) ocurre sola.

Tus archivos antiguos de Game Save Manager pueden quedarse donde están como copia en frío.

## Hoard One, o tu propio servidor

Hoard es gratis de las dos maneras, y la aplicación es la misma en ambas:

- **Hoard One**, recomendado para todos. Nada que montar ni que mantener encendido: un servidor gestionado en la UE, gratis con 2 GB y tres dispositivos, sin tarjeta. [Pro](/pricing) solo añade espacio, 100 GB y dispositivos ilimitados, para cuando tus partidas no quepan.
- **Tu propio servidor.** Montas `hoard-server` en un PC, un NAS o una Raspberry Pi y todas tus máquinas sincronizan a través de él. Sin cuenta con nosotros, sin telemetría hacia nosotros y sin cupo nuestro: nada pasa por nuestros servidores, porque no hay nada nuestro en el camino. Mira [cómo montar tu propio servidor de Hoard](/guides/self-host-hoard).

El mismo programa, la misma detección, el mismo historial. Hoard One guarda todas las versiones para siempre; en tu propio servidor decides tú cuánto historial guardar.

<!-- faq -->

## Preguntas frecuentes

### ¿Hay un GameSave Manager para Linux?

No: GameSave Manager es solo para Windows, y también los dos Game Save Manager de código abierto. Hoard funciona de forma nativa en Linux y SteamOS, encuentra las partidas dentro de los prefijos de Proton y Wine, y las sincroniza con tus PC con Windows.

### ¿Ludusavi o Game Save Manager?

Los dos son herramientas de copia local. Ludusavi es de código abierto, funciona en Windows, Linux y macOS y se apoya en el manifiesto comunitario de partidas; los Game Save Manager son solo para Windows. Si quieres copia y sincronización sin montar Rclone tú, Hoard usa los mismos datos comunitarios y sincroniza solo. Hay una [comparativa completa con Ludusavi](/guides/ludusavi-alternative).

### ¿Puedo conservar mis copias antiguas?

Sí. Hoard no las toca. Guarda los archivos como copia en frío; a partir de la primera partida, el historial de Hoard toma el relevo.

### ¿Hoard es gratis?

Sí. La aplicación, la línea de comandos y el servidor son gratis y de código abierto (AGPL-3.0), y montar tu propio servidor no cuesta más que la máquina. Hoard One, el servidor gestionado que recomendamos a todos, también es gratis con 2 GB y tres dispositivos; Pro añade espacio por 1,99 € al mes.

### ¿El self-hosting envía algo a Hoard?

No. En tu propio servidor no hay cuenta con nosotros ni telemetría hacia nosotros: tus partidas, tus usuarios y tus logs se quedan en tu máquina y nunca tocan las nuestras.
