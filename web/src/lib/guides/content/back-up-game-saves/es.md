---
title: "Cómo hacer copias de seguridad de tus partidas automáticamente"
description: "Configura copias de seguridad automáticas y versionadas en la nube de tus partidas de PC con Hoard, para que un fallo, una reinstalación o un mod problemático nunca borren tu progreso."
order: 1
updated: 2026-09-01
---

Perder una partida guardada significa perder horas de progreso. Hoard hace copias de seguridad de tus partidas de PC automáticamente y guarda un historial completo de versiones, para que siempre puedas volver atrás.

## Qué guarda Hoard

Hoard detecta las carpetas de guardado de los juegos a los que juegas y las copia a tu propia nube: Hoard Cloud o un servidor que alojes tú mismo. Cada copia está versionada, así que las versiones antiguas nunca se sobrescriben.

Para saber dónde guarda cada juego sus partidas, Hoard usa la misma base de datos comunitaria de ubicaciones que utiliza Ludusavi, así que la detección funciona desde el primer momento con miles de títulos. La diferencia está en lo que pasa después: en vez de dejar la copia en tu disco, Hoard la versiona en la nube automáticamente.

## Configura las copias automáticas

1. **Descarga e instala Hoard** para Windows, macOS o Linux desde la página de descargas.
2. Inicia sesión o apunta la app a tu servidor autoalojado.
3. Abre la **Biblioteca**. Hoard busca los juegos instalados y lista las partidas que encuentra.
4. Añade los juegos que quieras proteger. Hoard localiza cada carpeta de guardado automáticamente; puedes añadir una ruta a mano si un juego no se detecta.
5. Deja activado el **modo automático**. Hoard vigila las carpetas de guardado y hace la copia cuando dejas de jugar.

A partir de ahí cada sesión queda guardada sin que hagas nada.

## Dónde guardan realmente sus partidas los juegos de PC

No hay un único sitio, y ése es justo el motivo de que exista una herramienta así. En la práctica, una partida acaba en alguno de estos lugares:

- **Dentro de Steam**, en `userdata/<UserID>/<AppID>/remote/`, la carpeta que sincroniza el propio Steam Cloud.
- **`Documentos\My Games\…`**, lo más parecido a una convención que tiene Windows.
- **`%APPDATA%`, `%LOCALAPPDATA%` o `LocalLow`**, donde escriben la mayoría de juegos de Unity y Unreal.
- **`%USERPROFILE%\Saved Games`**, que usa un grupo más pequeño pero tozudo de títulos.
- **La propia carpeta de instalación del juego**, donde todavía guardan sorprendentes cantidades de títulos antiguos.
- **En Linux**, `~/.local/share` o `~/.config` para los juegos nativos, y dentro del prefijo de Proton — `steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…` — para los de Windows.
- **En macOS**, `~/Library/Application Support`.

De dónde venga el juego importa poco: los de GOG, Epic e itch caen en el mismo puñado de sitios, porque lo deciden el motor y el desarrollador, no la tienda.

## Qué se copia y qué no

Una carpeta de partidas rara vez contiene sólo partidas, así que Hoard reparte lo que encuentra en tres montones:

- **Los datos de partida** se copian y se restauran. Eso es tu progreso.
- **Los ficheros que son de una máquina concreta** — configuración, registros y similares — se suben para que formen parte de la copia, pero nunca se escriben encima de la copia de otro PC. Tus ajustes gráficos siguen siendo tuyos.
- **La basura** — cachés, volcados de fallos, temporales — se ignora, para que una copia no se hinche con cosas que nunca querrías de vuelta.

## Cuándo se hace la copia

Hoard vigila la carpeta y la captura **cuando dejas de jugar**, no mientras el juego tiene los ficheros abiertos. Si la partida se escribió hace unos segundos, espera a que la cosa se calme: un fichero que se está escribiendo no es un fichero que merezca capturarse a medias.

Cada captura es una versión. Las instantáneas se guardan por hash de contenido, así que un fichero que no cambia se almacena una sola vez: diez versiones de una partida de 2 GB ocupan unos 2 GB, no 20.

## Copias sin pasar por nuestros servidores

Si prefieres no usar la nube de nadie, levanta `hoard-server` tú mismo y apunta la aplicación ahí. Tus partidas van de tu PC a tu disco: sin cuenta con nosotros, sin telemetría hacia nosotros y sin nada que pase por nuestros servidores. Mira [cómo autoalojar Hoard](/guides/self-host-hoard).

## Consejo: revisa tu historial

Abre la pestaña **Historial** de un juego para ver cada copia con su fecha y tamaño. Desde ahí puedes restaurar cualquier versión anterior con un clic. Tus partidas viajan cifradas, se almacenan en la UE y puedes exportarlas o borrarlas cuando quieras.

¿Ya usas una herramienta de copia local como Ludusavi? Puedes seguir usándola, pero si quieres que esas copias acaben en la nube y se sincronicen entre equipos sin montar Rclone a mano, eso es justo lo que Hoard automatiza. Mira [Ludusavi frente a Hoard](/guides/ludusavi-alternative) para una comparativa justa.

<!-- faq -->

## Preguntas frecuentes

### ¿Hoard hace copias mientras juego?

No. Espera a que salgas y a que la carpeta de partidas se quede quieta, así que una copia nunca es un fichero a medio escribir.

### ¿Cuánto espacio necesitan mis partidas?

Menos del que imaginas. Las versiones se deduplican por hash de contenido, así que sólo ocupa espacio nuevo lo que cambió de verdad entre sesiones: la mayoría de colecciones caben de sobra en un par de gigas.

### ¿Y si uno de mis juegos no se detecta?

Apunta Hoard a la carpeta a mano y la rastreará como cualquier otra. La detección cubre miles de títulos, pero un juego que guarde en un sitio raro, o que hayas instalado a mano, a veces necesita la pista.

### ¿Copia también mis mods?

Hoard rastrea la carpeta de partidas, así que los mods que vivan en otro sitio no entran en la copia. Es deliberado: los mods son grandes, se vuelven a descargar, y una carpeta de mods sincronizándose entre máquinas da más problemas de los que resuelve.

### ¿Cambia algo si me autoalojo?

Nada. La misma detección, las mismas versiones, la misma captura automática. Lo único tuyo es el almacenamiento.
