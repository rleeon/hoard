---
title: "Cómo restaurar una partida guardada anterior"
description: "¿Tomaste una mala decisión, se corrompió un archivo o quieres empezar de cero? Vuelve a cualquier versión anterior de tu partida con el historial en la nube de Hoard, incluidas copias hechas con herramientas como Ludusavi."
order: 3
updated: 2026-09-01
---

Una mala decisión en el juego, un archivo corrupto o un mod que lo rompe todo: a veces solo necesitas volver atrás. Como Hoard guarda un historial completo de versiones de cada partida, restaurar una anterior lleva segundos.

## Restaurar una versión anterior

1. Abre **Hoard** y ve al juego en tu **Biblioteca**.
2. Abre su pestaña **Historial**. Verás cada copia con su fecha y tamaño.
3. Elige la versión que quieras y pulsa **Restaurar**.
4. Hoard vuelve a escribir esa instantánea en la carpeta de guardado del juego. Tu partida actual se respalda primero, así que la restauración es reversible.

## Restaurar en un PC nuevo o reinstalado

1. Instala Hoard e inicia sesión con tu cuenta.
2. Añade el juego a tu Biblioteca: Hoard encuentra la copia en la nube correspondiente.
3. Restaura la última versión, o cualquiera anterior, y sigue jugando.

Como Hoard localiza las carpetas de guardado con la misma base de datos comunitaria que Ludusavi, sabe dónde colocar una partida restaurada incluso en una instalación limpia, sin que busques rutas a mano.

## Cuando una partida se corrompe o un mod la rompe

Un juego que se cierra al cargar, un mod que reescribió lo que no debía, un autoguardado que cayó a mitad de escritura: la solución es la misma. Abre el **Historial** del juego, elige la última versión anterior al problema y restáurala. Las fechas y los tamaños suelen bastar para ver dónde se torció: una caída brusca de tamaño es buena señal de que una partida quedó truncada.

Si no tienes claro cuál es la buena, restaura la candidata más probable y compruébalo dentro del juego. Volver a intentarlo no cuesta nada, porque la versión que acabas de reemplazar también se guardó.

## Qué hace realmente una restauración

Tres cosas que conviene saber, porque son las que hacen que restaurar sea seguro:

1. **Tu partida actual se captura primero.** La restauración es reversible: lo que reemplazaste pasa a ser una versión más del historial.
2. **Sólo se descarga lo que falta.** Los ficheros que ya están en disco con el contenido correcto se aprovechan tal cual, así que restaurar una partida grande después de un cambio pequeño mueve unos megas y no la carpeta entera.
3. **Los ficheros propios de esta máquina no se tocan.** La configuración y los registros que viven junto a la partida se copian, pero no se escriben encima de los tuyos: tus controles y tus ajustes gráficos sobreviven a una restauración que venga de otro PC.

## Restaurar sin pasar por nuestros servidores

Si levantas tu propio `hoard-server`, las restauraciones funcionan exactamente igual, sólo que las versiones vienen de tu máquina y no de la nuestra. No hay cuenta con nosotros, ni telemetría hacia nosotros, ni nada que pase por nuestros servidores. Mira [cómo autoalojar Hoard](/guides/self-host-hoard).

## Consejo

Las restauraciones nunca son destructivas: la partida que reemplazas se guarda antes como una nueva versión, así que siempre puedes deshacer una restauración volviendo a la entrada anterior. Si hasta ahora solo guardabas copias en local (por ejemplo con Ludusavi), pasar a Hoard añade un historial versionado y fuera del equipo desde el que puedes restaurar incluso tras un fallo de disco.

<!-- faq -->

## Preguntas frecuentes

### ¿Restaurar sobrescribe mi progreso actual?

Sólo después de que tu partida actual se haya capturado como una versión nueva. Si restauras la equivocada, restaura la entrada anterior y vuelves al punto de partida.

### ¿Hasta dónde llega el historial?

Hasta donde permita el tope de versiones de tu plan, y una versión que fijes no se poda nunca para hacer sitio. En un servidor autoalojado el único límite es tu disco.

### ¿Puedo restaurar en un PC donde el juego todavía no está instalado?

Instala primero el juego para que exista su carpeta de partidas, y luego restaura. Hoard sabe dónde espera cada juego sus saves, así que escribe la instantánea en el sitio correcto sin que tengas que buscar la ruta.

### ¿Funciona restaurar entre Windows y una Steam Deck?

Sí. El mismo juego guarda en sitios distintos en cada uno — en la Deck, dentro del prefijo de Proton — y Hoard escribe la versión restaurada donde esa máquina la espera.

### ¿Cambia algo restaurar en un servidor autoalojado?

No. Misma aplicación, mismo historial, misma restauración de un clic. Lo único tuyo es el almacenamiento.
