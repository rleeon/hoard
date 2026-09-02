---
title: "Syncthing para partidas guardadas: qué funciona y qué se rompe"
description: "Syncthing es un sincronizador de ficheros excelente, pero las partidas guardadas rompen tres de sus supuestos. Qué falla, cómo lo apaña la gente, y cuándo conviene una herramienta que sepa lo que es un save."
order: 9
updated: 2026-09-01
---

Syncthing es la respuesta a la que mucha gente llega primero, y con razón: es gratis, open source, punto a punto, y funciona. Pero las partidas guardadas rompen tres de los supuestos sobre los que se construye un sincronizador de ficheros genérico, y los fallos son silenciosos. Esta guía va de qué se rompe de verdad, y de cuándo merece la pena usar algo que sepa lo que es una partida.

## Por qué la gente acaba ahí

Es software genuinamente bueno. Sin cuenta, sin suscripción, tus ficheros no se quedan en el disco de ninguna empresa, y sincroniza cualquier cosa: documentos, fotos, una carpeta de partidas. Si ya lo tienes montado para otras cosas, apuntarlo a una carpeta de saves te cuesta treinta segundos. Ése es un argumento real, y en algunos montajes es el correcto.

## Las tres cosas que se rompen

**Sincroniza con el juego abierto.** Syncthing reacciona a que un fichero cambie, porque eso es lo correcto para un documento. Un juego escribe su partida en mitad de la sesión, a veces en varias pasadas, y un fichero pillado a medio escribir es un fichero que se propaga incompleto. La otra máquina se queda con una partida que el juego puede negarse a cargar.

**Los conflictos se convierten en ficheros, no en decisiones.** Cuando las dos máquinas cambian la misma partida, Syncthing hace lo seguro y conserva las dos, renombrando una a `algo.sync-conflict-20260901-143022-ABCDEFG.sav`. No se pierde nada, pero el juego no sabe qué es ese fichero y tú acabas comparando fechas en un explorador para decidir qué tarde de juego te quedas. Repítelo unas cuantas veces y la carpeta se llena de ficheros de conflicto que nadie se atreve a borrar.

**El versionado es por fichero, no por sesión.** Syncthing puede guardar copias viejas en `.stversions`, y eso es mejor que nada. Pero una partida suele ser varios ficheros que sólo tienen sentido juntos, y restaurar significa buscar a mano la fecha correcta de cada uno. No existe un «deja este juego como estaba el martes».

Y una cuarta, específica de Steam: si lo apuntas a `userdata/<UserID>/<AppID>/` en vez de a la carpeta `remote/` de dentro, también estás sincronizando `remotecache.vdf` y ficheros de logros y tiempo jugado que **deben** ser distintos entre máquinas. Entonces cada arranque parece un conflicto aunque no se haya movido ninguna partida. Es el motivo más común de que un montaje casero entre Steam Deck y sobremesa parezca estropeado.

## Lo que acabas construyendo

Nada de lo anterior es irresoluble. La gente lo apaña con patrones de exclusión por juego, una política de versionado, y la costumbre de cerrar el juego y esperar antes de tocar el otro PC. Funciona, y es un mantenimiento que te llevas de por vida: un juego nuevo son rutas nuevas, y el día que se te olvide esperar es el día que te enteras.

## Qué hace en su lugar una herramienta que entiende de partidas

Hoard captura **cuando dejas de jugar**, una vez que la carpeta se queda quieta, así que una instantánea nunca es un fichero a medio escribir. Cada captura es una versión de la partida entera, no de ficheros sueltos, así que restaurar es un clic y lo devuelve todo junto. Sabe qué carpeta es de qué juego — leyendo el mismo manifiesto comunitario de ubicaciones que comparte el ecosistema open source, con más de 20.000 títulos — así que no hay rutas que mantener, y rastrea `<AppID>/remote/` y no la carpeta de encima.

## Cuándo Syncthing es la mejor respuesta

Siendo justos:

- **Ya lo tienes corriendo**, y añadir una carpeta te sale gratis.
- **Quieres punto a punto sin servidor ninguno**, ni siquiera el tuyo.
- **Sincronizas mucho más que partidas** y prefieres una sola herramienta para todo.
- **Nunca vuelves atrás.** Si la última partida es todo lo que has necesitado, un historial de versiones es maquinaria que no vas a usar.

## Usar los dos

Conviven sin pelearse, y es un montaje razonable: que el sincronizador genérico se ocupe de tus documentos y de lo que sea, y que de las carpetas de partidas se ocupe una herramienta que las entienda. La única regla es no apuntar los dos a la misma carpeta: dos programas escribiendo los mismos ficheros es la forma de fabricar justo los conflictos que querías evitar.

## Sin nuestros servidores tampoco

Si parte del atractivo es que nada toque el disco de una empresa, Hoard se puede usar igual: `hoard-server` en tu propio PC o NAS, y tus partidas van de tu máquina a tu disco. **No hay cuenta con nosotros, ni telemetría hacia nosotros, ni relé**: no pasa nada por nuestros servidores, porque no hay nada nuestro en el camino. Mira [cómo autoalojar Hoard](/guides/self-host-hoard).

El mismo binario, la misma detección, el mismo historial. Lo único que cambia es de quién es el almacenamiento. También hay una [comparativa de todas las herramientas de sincronización](/guides/game-save-sync-comparison).

<!-- faq -->

## Preguntas frecuentes

### ¿Syncthing sirve para sincronizar partidas?

Sí, y en casos sencillos lo hace bien. El problema empieza con juegos que escriben mientras juegas, partidas hechas de varios ficheros, y cualquier montaje donde las dos máquinas se editen entre sincronizaciones.

### ¿Qué son los ficheros .sync-conflict de mi carpeta de partidas?

Es el sincronizador conservando las dos versiones tras un conflicto en vez de elegir una. No se pierde nada, pero el juego no puede leerlos, y decidir cuál te quedas es trabajo manual cada vez.

### ¿Por qué mi partida de Steam da conflicto en cada arranque?

Casi siempre porque la carpeta sincronizada es la que está por encima de `remote/`. Contiene `remotecache.vdf` y ficheros de logros y tiempo jugado que son legítimamente distintos en cada máquina, así que los dos extremos nunca coinciden.

### ¿Tengo que cerrar el juego antes de sincronizar?

Con un sincronizador genérico, sí: ésa es la costumbre que evita las partidas a medio escribir. Una herramienta que entiende de saves espera sola a que la carpeta se quede quieta.

### ¿Puedo seguir usando los dos a la vez?

Sí. Sólo que no apuntes los dos a la misma carpeta, o se pelearán por los mismos ficheros.
