---
title: "Partidas en la nube de Epic y GOG: qué cubren y cómo sincronizar el resto"
description: "La nube de Epic y GOG solo sirve para algunos juegos, solo dentro de su launcher y sin historial. Qué cubre y cómo sincronizar el resto de forma automática."
order: 19.5
updated: 2026-10-09
---

Tanto Epic como GOG tienen partidas en la nube, con las mismas pegas que Steam: el desarrollador tiene que admitirlas juego a juego, solo funcionan a través del launcher de la propia tienda y guardan la última copia en lugar de un historial. Aquí tienes qué cubre cada una, dónde están los huecos y cómo tener todos tus juegos sincronizados entre tus PC y una Steam Deck, los compraras donde los compraras.

## Epic Games Store

El launcher de Epic tiene un interruptor de partidas en la nube en sus ajustes, y los juegos que las admiten se sincronizan a través de él cuando juegas en otro PC. El soporte es juego a juego: el desarrollador tiene que implementarlo, y muchos juegos de la tienda nunca lo hicieron.

En Linux y en la Steam Deck no hay launcher oficial de Epic. Heroic puede sincronizar la nube de Epic en los juegos que la admiten, pero hay que activarlo juego a juego.

## GOG

GOG Galaxy sincroniza las partidas en la nube de los juegos que incluyen «Cloud saves» entre sus características en la página de la tienda. Hay dos pegas propias de GOG:

- **Solo a través de Galaxy.** Los instaladores offline de GOG, la parte sin DRM que es su gran atractivo, no tienen nube. Juega con el instalador y tus partidas se quedan en ese PC.
- **Juego a juego y plataforma a plataforma.** Un juego solo se sincroniza entre las plataformas para las que el desarrollador lo preparó.

Igual que con Epic, Heroic puede sincronizar la nube de GOG en Linux y en la Steam Deck si lo activas juego a juego.

## Ubisoft, EA y el resto

Ubisoft Connect y la app de EA tienen partidas en la nube para muchos de sus propios juegos, y cada una funciona solo dentro de su launcher. Amazon Games y las tiendas más pequeñas varían juego a juego.

## Lo que no hace ninguna

- **Historial.** Todos los launchers guardan la partida actual. Si una partida se rompe y se sincroniza, la buena desaparece en todas partes.
- **Entre tiendas.** El mismo juego comprado en Steam para un PC y en GOG para otro tiene dos nubes separadas que nunca se hablan.
- **Todo lo que está fuera del launcher.** Emuladores, instaladores sin DRM, juegos que instalaste a mano.
- **Juegos sin soporte.** Si el desarrollador no lo implementó, el launcher no puede hacer nada.

## Sincronizar el resto

Hoard trabaja por juego, no por tienda. Encuentra la carpeta de guardado de cada juego con una base de datos comunitaria que cubre miles de títulos, venga de donde venga el juego, la respalda automáticamente cuando dejas de jugar y la sincroniza con tus otros PC y tu Steam Deck, guardando todas las versiones.

Eso cubre los huecos de arriba:

- **Cualquier launcher, o ninguno.** Epic, GOG, Galaxy o el instalador offline, Heroic en Linux, un juego que descomprimiste en una carpeta.
- **Entre tiendas.** La mayoría de juegos guarda en el mismo sitio sea cual sea la tienda que los vendió, normalmente en `AppData` o `Documents`, así que una instalación de GOG en un PC y una de Steam en otro pueden compartir partida. Algunos añaden una carpeta con el ID de la cuenta o usan otro nombre según la tienda; compruébalo antes de fiarte.
- **Un historial.** Cada sesión es una versión a la que puedes volver.

Donde la nube del propio launcher ya sincroniza un juego, déjala seguir. Hoard añade el historial, y se encarga de sincronizar todo lo demás.

Si prefieres no usar la nube de nadie, levanta `hoard-server` en tu PC o tu NAS. Sin cuenta con nosotros, sin telemetría hacia nosotros, sin nada pasando por nuestros servidores. Mira [cómo autoalojar Hoard](/guides/self-host-hoard).

<!-- faq -->

## Preguntas frecuentes

### ¿Epic tiene partidas en la nube?

Sí, en los juegos cuyo desarrollador las implementó, a través del launcher de Epic. Muchos juegos de la tienda no las admiten, y el launcher no guarda historial de partidas anteriores.

### ¿GOG tiene partidas en la nube?

Sí, a través de GOG Galaxy, en los juegos que lo indican en su página de la tienda. Los instaladores offline no sincronizan nada.

### ¿Epic o GOG guardan versiones antiguas de mis partidas?

No. Los dos guardan solo la última copia. Para volver a una partida anterior necesitas una copia que guarde versiones.

### ¿Puedo pasar una partida de la versión de GOG a la de Steam?

A menudo sí: la mayoría de juegos guarda en la misma carpeta sea cual sea la tienda. Algunos añaden una carpeta con el ID de la cuenta o usan otro nombre de carpeta, así que comprueba antes las rutas.

### ¿Los instaladores offline de GOG sincronizan las partidas?

No a través de GOG, porque su nube solo funciona en Galaxy. Hoard los sincroniza como cualquier otro juego, porque sigue la carpeta de guardado y no el launcher.
