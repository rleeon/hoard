---
title: "¿Partida corrupta? Cómo recuperarla"
description: "Una partida que no carga no siempre está perdida. Dónde buscar una copia buena (backups del juego, Steam Cloud, Windows, OneDrive) y cómo evitar perderla."
order: 12
updated: 2026-10-09
---

Una partida que no carga casi nunca está perdida del todo. Lo normal es que exista una copia buena en algún sitio: un backup que hizo el propio juego, la copia de Steam Cloud, una versión anterior que guardó Windows u OneDrive, o la partida de otro PC. Pero el orden importa, porque un mal paso puede machacar la copia buena con la rota. Empieza por aquí.

## Primero: para y copia la carpeta

1. **Cierra el juego** y no empieces una partida nueva en esa ranura. Cada guardado a partir de ahora puede echar fuera una copia antigua.
2. **Copia la carpeta de guardado entera** al escritorio o a un USB. Así todo lo que pruebes después se puede deshacer. Si no sabes dónde está la carpeta, mira [dónde guardan las partidas los juegos de PC](/guides/where-are-pc-game-saves-stored).
3. **Pausa todo lo que sincronice esa carpeta.** Steam Cloud (juego a juego, en **Propiedades → General**), OneDrive, Syncthing. Si no, el fichero roto puede viajar justo al único sitio que todavía tiene una copia buena.

## Comprueba que de verdad está corrupta

Hay cosas que parecen corrupción y no lo son:

- **El juego se ha actualizado** y las partidas antiguas no cargan, o necesitan un parche. Mira las noticias o el foro del juego.
- **Faltan mods.** Los juegos de Bethesda, sobre todo, avisan de plugins que faltan y pueden rechazar una partida que los usaba. Reinstala los mods primero.
- **Estás en otra cuenta.** Algunos juegos archivan las partidas bajo el ID de tu cuenta de Steam o de Ubisoft, así que otra cuenta ve la ranura vacía.
- **El fichero solo está en la nube.** Con OneDrive, una partida con el icono de la nube se ha sacado del disco para liberar espacio. Haz clic derecho y elige **Mantener siempre en este dispositivo**.

Una partida de **0 KB**, o mucho más pequeña que las de al lado, sí está rota: la escritura se cortó a medias.

## Dónde puede haber una copia buena

Ve por orden. Las primeras son más rápidas y tienen más probabilidades de funcionar.

### 1. Los backups del propio juego

Muchos juegos guardan una copia de repuesto sin avisar. Busca en la carpeta de guardado ficheros que terminen en `.bak`, `_old` o `.backup`, y ranuras de autoguardado extra. Algunos casos conocidos:

- **Elden Ring** escribe `ER0000.sl2.bak` junto a la partida.
- **Stardew Valley** guarda una copia `_old` de cada granja, que es el día anterior del juego.
- **Terraria** guarda ficheros `.bak` de jugadores y mundos.
- **Minecraft Java** guarda `level.dat_old` dentro de cada mundo.

Para usar una, aparta el fichero roto (ya has copiado la carpeta) y renombra el backup con el nombre original.

### 2. Steam Cloud

Steam Cloud guarda la copia **más reciente**, no un historial. Solo ayuda si la partida se rompió después de la última subida, por ejemplo porque el juego se colgó y el fichero malo nunca llegó a sincronizarse. Puedes ver y descargar lo que Steam guarda de cada juego en [store.steampowered.com/account/remotestorage](https://store.steampowered.com/account/remotestorage) y volver a colocar el fichero a mano.

### 3. Versiones anteriores de Windows

Haz clic derecho en la carpeta de guardado y abre **Propiedades → Versiones anteriores**. Si el Historial de archivos o la Protección del sistema estaban activados en esa unidad, aquí aparecen copias antiguas de la carpeta, y puedes abrirlas para sacar los ficheros que necesites. Si la lista está vacía, no estaba activado ninguno de los dos.

### 4. El historial de versiones de OneDrive

Si OneDrive hace copia de tu carpeta `Documentos`, muchas partidas están ahí sin que lo sepas. En onedrive.com, haz clic derecho en el fichero de la partida y elige **Historial de versiones** para descargar una versión anterior. OneDrive las conserva durante un tiempo limitado, y los ficheros borrados también pasan un tiempo en su papelera.

### 5. Tus otras máquinas

¿Has jugado hace poco en un portátil o en una Steam Deck? Su copia puede ser anterior al problema. Pásala antes de que esa máquina sincronice la rota.

### 6. Si el fichero se borró, no se rompió

Mira primero la Papelera de reciclaje. Después, una herramienta de recuperación de ficheros puede encontrarlo, siempre que dejes de escribir en esa unidad. Cada instalación y cada descarga reducen las probabilidades.

## Cuando no aparece nada

Para algunos juegos populares, la comunidad tiene editores de partidas o herramientas de reparación que pueden rehacer un fichero dañado: busca el nombre del juego con "save repair". Si no, la respuesta honesta es que la única copia que vale es la que se hizo antes del problema.

## Por qué se rompen las partidas

- **Un cuelgue o un corte de luz a mitad de escritura.** El juego estaba guardando cuando murió.
- **El disco lleno.** El juego no pudo terminar de escribir y dejó un fichero truncado.
- **Una herramienta de sincronización la copió a medias**, o dos PC editaron la misma partida y ganó una de las copias.
- **Un mod** escribió algo que el juego no sabe volver a leer.
- **Un disco que falla**, que suele notarse también en otros ficheros.

## No vuelvas a quedarte sin partida

Todas las recuperaciones de arriba dependen de la suerte: que el juego hiciera un backup, o que Steam todavía no hubiera sincronizado. Una copia con versiones quita la suerte de la ecuación. Es lo que hace Hoard: respalda cada partida automáticamente cuando dejas de jugar, en cuanto la carpeta se queda quieta, así que una copia nunca es un fichero a medio escribir. Se guardan todas las versiones. Cuando algo se rompe, abres el **Historial** del juego y restauras la última buena con un clic; tu partida actual se guarda primero, así que hasta eso tiene vuelta atrás. Y como Hoard también mantiene tus partidas sincronizadas, la copia de tu portátil o tu Steam Deck nunca es una vieja y olvidada: todas tus máquinas trabajan con el mismo historial.

Un truco para ver cuándo se rompió: una caída brusca de tamaño entre dos versiones suele indicar una partida truncada. Más en [cómo restaurar una partida guardada anterior](/guides/restore-a-game-save).

Si prefieres que las copias se queden en casa, levanta `hoard-server` en tu PC o en tu NAS. Sin cuenta con nosotros, sin telemetría hacia nosotros, sin nada pasando por nuestros servidores. Mira [cómo autoalojar Hoard](/guides/self-host-hoard).

<!-- faq -->

## Preguntas frecuentes

### ¿Se puede reparar una partida corrupta?

Rara vez tal cual. Algunos juegos tienen herramientas de reparación de la comunidad, pero casi siempre recuperar significa encontrar una copia anterior: el backup del juego, Steam Cloud, Windows u OneDrive, u otro PC.

### ¿Steam Cloud guarda versiones antiguas de mis partidas?

No. Solo guarda el fichero actual. Si ya se ha subido una partida rota, Steam Cloud también tiene la rota.

### ¿Verificar los archivos del juego arregla una partida corrupta?

No. La verificación compara los ficheros del juego con los de Steam, no tus partidas. Ayuda si el juego en sí está dañado, pero no te devuelve el progreso.

### ¿Por qué mi partida pesa 0 KB?

El juego empezó a escribirla y no terminó: un cuelgue, un corte de luz o el disco lleno. Busca al lado un fichero `.bak` o `_old`, o una versión anterior en otro sitio.

### ¿Cómo evito que vuelva a pasar?

Con copias con versiones hechas cuando el juego no está abierto. Hoard lo hace automáticamente después de cada sesión y guarda cada versión, así que puedes volver a cualquiera.
