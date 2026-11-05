---
title: "Alternativa a OpenCloudSaves: está archivado, esto es lo que puedes usar"
description: "OpenCloudSaves está archivado desde enero de 2025. Hoard es un sustituto mantenido y de código abierto que sincroniza solo y guarda todas las versiones."
order: 28
updated: 2026-11-05
---

OpenCloudSaves se archivó el 20 de enero de 2025. El código y la última versión siguen ahí, pero el proyecto ya no acepta cambios, así que nada lo arreglará para juegos nuevos, launchers nuevos o actualizaciones del sistema. Si sincronizaba tus partidas a través de una nube, [Hoard](/download) es un sustituto gratis y de código abierto, mantenido activamente, que encuentra tus partidas solo y las sincroniza cada vez que el juego escribe una.

## Qué hacía bien OpenCloudSaves

- **Tu propia nube.** Las partidas iban al OneDrive, Google Drive, Dropbox o Nextcloud que ya tenías, mediante Rclone, con una interfaz de verdad encima en lugar de ficheros de configuración.
- **Prudente de serie.** Hacía un ensayo antes de cada sincronización, así que veías qué iba a cambiar antes de que cambiara.
- **Filtros y juegos propios.** Podías sincronizar solo ciertos ficheros, dejar fuera los ajustes gráficos y definir juegos tú mismo, incluso entre tiendas.
- **Todos los escritorios.** Windows, macOS y Linux, Steam Deck incluida.

## Qué necesita un sustituto

- **Alguien que lo mantenga.** Las ubicaciones de partidas se mueven y los launchers cambian; una herramienta de sincronización que nadie actualiza acaba sincronizando la carpeta equivocada.
- **Un historial, no solo la última copia.** Una nube sincroniza ficheros, así que una partida corrupta llega a todas tus máquinas tan fielmente como una buena.
- **Nada de ficheros a medio escribir.** Una sincronización genérica puede pillar una partida mientras el juego aún la escribe; la [guía de OneDrive](/guides/onedrive-game-saves) explica qué rompe eso.

## Hoard como sustituto

| | OpenCloudSaves | Hoard |
|---|---|---|
| Mantenimiento | Archivado desde enero de 2025 | Sí, código abierto (AGPL-3.0) |
| Encontrar partidas | Definiciones de partidas, más carpetas que añades | Automático: base de datos comunitaria, Steam, Epic, GOG, prefijos de Proton y Wine, 19 emuladores |
| Cuándo sincroniza | Cuando se lanza una sincronización, tras un ensayo | Cada vez que el juego guarda, y otra vez al salir |
| Dónde viven las partidas | Tu nube | Hoard One, o tu propio servidor sobre disco local o cualquier almacenamiento compatible con S3 |
| Historial | Lo que guarde la nube | Todas las versiones, deduplicadas |
| Diez versiones de una partida de 2 GB | Unos 20 GB de cupo de la nube | Unos 2 GB, porque las versiones comparten lo que no cambia |
| Plataformas | Windows, macOS, Linux | Windows, Linux, macOS y SteamOS |

Lo único que Hoard no hace es escribir en tu Google Drive o tu OneDrive. Sincroniza a través de un servidor, y eso es lo que le da el historial y permite que la otra máquina esté apagada. Si lo que querías era un almacenamiento que controlas más que esa nube en concreto, un Hoard autoalojado escribe en un NAS, en disco local o en cualquier bucket compatible con S3, como Backblaze B2. Si tiene que ser esa nube, Ludusavi con un remoto de Rclone es la opción mantenida más parecida.

## Cómo cambiarte

1. Antes de tocar nada, copia tus carpetas de partidas a un sitio seguro.
2. Deja de sincronizar esas carpetas con OpenCloudSaves, para que dos herramientas nunca escriban en la misma partida.
3. Instala Hoard e inicia sesión en Hoard One, recomendado para todos, o apúntalo a [tu propio servidor](/guides/self-host-hoard).
4. Abre la **Biblioteca**. Hoard encuentra tus juegos; añade a mano cualquier carpeta propia. La primera versión aparece en el historial en cuanto el juego guarda.
5. Instala Hoard en tus otras máquinas con la misma cuenta.

## Hoard One, o tu propio servidor

Hoard es gratis de las dos maneras, y la aplicación es la misma en ambas:

- **Hoard One**, recomendado para todos. Nada que montar ni que mantener encendido: un servidor gestionado en la UE, gratis con 2 GB y tres dispositivos, sin tarjeta. [Pro](/pricing) solo añade espacio, 100 GB y dispositivos ilimitados, para cuando tus partidas no quepan.
- **Tu propio servidor.** Montas `hoard-server` en un PC, un NAS o una Raspberry Pi y todas tus máquinas sincronizan a través de él. Sin cuenta con nosotros, sin telemetría hacia nosotros y sin cupo nuestro: nada pasa por nuestros servidores, porque no hay nada nuestro en el camino. Mira [cómo montar tu propio servidor de Hoard](/guides/self-host-hoard).

El mismo programa, la misma detección, el mismo historial. Hoard One guarda todas las versiones para siempre; en tu propio servidor decides tú cuánto historial guardar.

<!-- faq -->

## Preguntas frecuentes

### ¿Es seguro seguir usando OpenCloudSaves?

La última versión sigue disponible, pero el proyecto está archivado: no habrá arreglos para juegos nuevos ni cambios del sistema. Además salió como beta, y su autor pedía hacer una copia de las partidas antes de usarlo. Si te quedas en él, guarda una copia a mano.

### ¿Hoard puede guardar las partidas en Google Drive u OneDrive?

No. Hoard sincroniza a través de un servidor: Hoard One, o el tuyo sobre disco local o almacenamiento compatible con S3. Tu carpeta `Documentos` puede seguir en OneDrive; Hoard le pregunta a Windows dónde está de verdad y encuentra ahí las partidas.

### ¿Hoard es gratis?

Sí. La aplicación, la línea de comandos y el servidor son gratis y de código abierto (AGPL-3.0), y montar tu propio servidor no cuesta más que la máquina. Hoard One, el servidor gestionado que recomendamos a todos, también es gratis con 2 GB y tres dispositivos; Pro añade espacio por 1,99 € al mes.

### ¿El self-hosting envía algo a Hoard?

No. En tu propio servidor no hay cuenta con nosotros ni telemetría hacia nosotros: tus partidas, tus usuarios y tus logs se quedan en tu máquina y nunca tocan las nuestras.
