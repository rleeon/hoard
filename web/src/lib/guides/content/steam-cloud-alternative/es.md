---
title: "Alternativa a Steam Cloud: copia las partidas que Steam no guarda"
description: "Steam Cloud sólo cubre juegos de Steam cuyo desarrollador lo activó, y no guarda historial de versiones. Hoard copia todos los juegos a los que juegas, vengan de donde vengan, con un historial al que puedes volver, en la nube o en tu propio servidor."
order: 7
updated: 2026-09-01
---

Steam Cloud hace muy bien el trabajo concreto que hace, y la mayoría de la gente descubre sus límites justo el día que pierde algo. Esta guía explica dónde están esos límites y qué hacer con los juegos que se quedan fuera.

## Qué cubre realmente Steam Cloud

Steam Cloud sincroniza la carpeta de un juego cuando **el desarrollador lo configuró**, ya sea declarando qué ficheros sincronizar o llamando a la API de Steam desde dentro del juego. Ése es todo el modelo, y de ahí salen tres consecuencias:

- Sólo funciona con juegos comprados y lanzados desde Steam.
- Que funcione o no lo decide el desarrollador, juego por juego, y a veces por plataforma.
- Cada juego tiene su propio cupo de almacenamiento, fijado por ese desarrollador.

Cuando funciona es invisible y excelente: cierras el juego en un PC, lo abres en otro y tu progreso está ahí.

## Dónde te deja expuesto

- **Todo lo que no sea un juego de Steam.** GOG, Epic, itch, Battle.net, la app de Xbox, emuladores, cualquier cosa instalada a mano. Steam ni sabe que existen.
- **Juegos de Steam donde nunca se activó.** Bastantes títulos, sobre todo antiguos o pequeños, sencillamente no lo tienen. La ficha de la tienda lo dice, pero nadie lo mira antes de empezar una partida de 60 horas.
- **No hay marcha atrás.** Éste es el grande. Steam guarda el estado actual de tu partida, no su historial. Si el fichero se corrompe, si un mod se come tu mundo o si machacas una partida buena con una mala, la copia de la nube ya es la mala. Puedes ver los ficheros que Steam guarda de un juego, pero no hay una versión anterior a la que volver.
- **El diálogo de conflicto.** Cuando Steam cree que la partida local y la remota no cuadran, te pide que elijas con poco más que dos fechas delante. Si eliges mal, la otra copia desaparece.

## Qué añade Hoard

Hoard vigila la carpeta en la que escribe cada juego y captura una **versión nueva cada vez que terminas de jugar**:

- **Le da igual de dónde venga el juego.** Steam, GOG, Epic, itch, emuladores o una carpeta que le señales a mano.
- **Se conservan todas las versiones**, así que recuperarte de una partida corrupta o de una mala decisión son dos clics y no una partida perdida.
- **Sincroniza entre tus máquinas** igual, incluidas una Steam Deck y un sobremesa.
- **Nada se destruye en silencio.** La partida que se reemplaza se captura antes, así que hasta una restauración equivocada es reversible.

Las instantáneas se guardan por hash de contenido, así que diez versiones de una partida de 2 GB ocupan unos 2 GB y no 20, que es lo que hace práctico conservar el historial entero.

## Usar los dos a la vez

No se pelean, y no tienes que elegir. En un juego de Steam con soporte de nube, deja que Steam siga sincronizando lo que ya sincroniza; lo que aporta Hoard ahí es el historial, que es justo lo que Steam no guarda. Para todo lo demás, Hoard se encarga también de la sincronización.

Un detalle que importa si tienes Steam Deck además de sobremesa: Hoard rastrea `<AppID>/remote/` dentro de `userdata`, no la carpeta de encima, porque la padre guarda `remotecache.vdf` y ficheros de logros y tiempo jugado propios de cada máquina. Ésa es la distinción que suele fallar en una sincronización casera, y por eso esos montajes parecen entrar en conflicto en cada arranque.

## Cuándo basta con Steam Cloud

Conviene decirlo claro: si todos los juegos a los que juegas son de Steam y con soporte de nube, juegas en un solo PC y nunca has necesitado deshacer una partida, Steam Cloud ya hace el trabajo y no necesitas nada más. Lo que justifica añadir Hoard es el historial de versiones, los juegos de fuera de Steam y las máquinas a las que Steam Cloud no llega.

## Sin la nube de nadie

Si lo que te atrae es no depender de ninguna plataforma, Hoard se puede usar entero sobre tu propio hardware: `hoard-server` en un PC o en un NAS, y tus partidas van de tu máquina a tu disco. **No hay cuenta con nosotros, ni telemetría hacia nosotros, ni relé**: no pasa nada por nuestros servidores, porque no hay nada nuestro en el camino. Mira [cómo autoalojar Hoard](/guides/self-host-hoard).

El mismo programa, la misma detección, el mismo historial de versiones. Lo único que cambia es de quién es el almacenamiento.

<!-- faq -->

## Preguntas frecuentes

### ¿Hoard sustituye a Steam Cloud?

No tiene por qué. Steam Cloud mantiene sincronizada tu partida actual en los juegos que lo soportan; Hoard añade el historial de versiones y cubre los juegos que no. Usar los dos es lo normal.

### ¿Steam Cloud puede volver a una partida anterior?

No. Steam guarda el estado actual de los ficheros, no su historial. Una vez que una partida mala se ha sincronizado, eso es lo que hay en la nube. Para volver atrás hace falta una herramienta con versiones.

### ¿Por qué no se sincronizan todos mis juegos de Steam?

Porque quien lo activa es el desarrollador, juego por juego y a veces por plataforma. La ficha del juego en la tienda incluye Steam Cloud entre sus características cuando está soportado, y muchos títulos sencillamente no lo están.

### ¿Hoard funciona con juegos que no son de Steam?

Sí, y es buena parte del sentido que tiene. Localiza las partidas con una base de datos comunitaria que cubre más de 20.000 títulos, de cualquier tienda, y para lo raro puedes señalarle la carpeta a mano.

### ¿Usar los dos provoca conflictos?

No. Hoard captura una versión cuando dejas de jugar y la carpeta se queda quieta, y nunca sobrescribe sin capturar antes lo que reemplaza.

### ¿Puedo mantener mis partidas fuera de las dos nubes?

Sí. Autoaloja el servidor y tus partidas no salen nunca de hardware tuyo, sin cuenta y sin telemetría hacia ningún sitio.
