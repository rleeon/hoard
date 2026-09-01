---
title: "Cómo sincronizar partidas guardadas entre varios PC"
description: "Juega al mismo juego en tu sobremesa y tu portátil sin perder progreso. Sincroniza tus partidas entre PC automáticamente con Hoard: sincronización en la nube gestionada, sin montar Ludusavi y Rclone a mano."
order: 2
updated: 2026-09-01
---

Si juegas en más de un ordenador —un sobremesa en casa y un portátil de viaje— Hoard mantiene tus partidas sincronizadas para que siempre retomes donde lo dejaste.

## Cómo funciona la sincronización

Hoard sube cada partida a tu nube y descarga la última versión en tus otros equipos. Cuando terminas de jugar en un PC, la partida más reciente te espera en el siguiente.

## Configura la sincronización

1. Instala **Hoard** en cada PC en el que juegues (Windows, macOS o Linux).
2. Inicia sesión con la **misma cuenta** en cada equipo, o conéctalos al mismo servidor autoalojado.
3. Añade los mismos juegos a tu **Biblioteca** en cada PC. Hoard los empareja por juego, así que una partida guardada en uno aparece en los demás.
4. Mantén el **modo automático** activado. Hoard sube cuando terminas de jugar y descarga la última versión antes de empezar.

## ¿Vienes de Ludusavi?

Ludusavi es una gran herramienta open source para hacer copias y restaurar partidas en local, y puede subir esas copias a una nube que configures tú mismo con Rclone. Pero sincronizar entre dispositivos es algo que montas a mano: programas la copia, configuras el remoto y luego restauras en el otro PC antes de jugar.

Hoard convierte eso en sincronización gestionada. Usa los mismos datos comunitarios de ubicación de partidas que Ludusavi para encontrar tus saves, y luego sube tras cada sesión y descarga la última versión antes de la siguiente, en todos los PC de tu cuenta y con historial versionado en la nube. Sin remotos de Rclone, sin scripts. Y, como Ludusavi, Hoard es open source y se puede autoalojar. Mira la [comparativa completa con Ludusavi](/guides/ludusavi-alternative).

## Evitar conflictos

Hoard tiene en cuenta los conflictos: compara las fechas de modificación y guarda una copia local de cualquier partida que reemplaza, así que una sincronización nunca destruye progreso en silencio. Si un juego sigue abierto o la partida se tocó hace pocos minutos, Hoard espera.

## Steam Deck y sobremesa

El montaje de dos máquinas más habitual es también el que más se rompe cuando se monta a mano, y casi siempre por el mismo motivo.

En Windows, la partida de un juego puede estar en `Documentos\My Games\…` o dentro del `userdata` de Steam. En una Steam Deck, ese mismo juego de Windows corre bajo Proton, así que su partida vive dentro de un prefijo de compatibilidad: `steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…`. Dos rutas muy distintas, un solo juego, un solo progreso. Hoard lee los prefijos de Proton además de las ubicaciones nativas y empareja lo que encuentra por juego, así que la partida de la Deck y la del sobremesa pasan a ser dos versiones de un mismo historial en vez de dos carpetas sin relación.

El detalle que decide si esto funciona: en los juegos de Steam, Hoard rastrea `<AppID>/remote/` dentro de `userdata`, **no** la carpeta de encima. La carpeta padre guarda además `remotecache.vdf` y ficheros de logros y de tiempo jugado propios de cada máquina, que deben ser distintos entre tu Deck y tu sobremesa. Si sincronizas la padre, cada arranque parece un conflicto aunque no se haya movido ninguna partida. Ese único error es lo que hace que la mayoría de los montajes caseros entre Deck y PC parezcan estropeados.

## Juegos que Steam Cloud no cubre

Si todos los juegos a los que juegas soportaran Steam Cloud, no necesitarías nada de esto. En la práctica:

- **Juegos de cualquier sitio que no sea Steam.** GOG, Epic, itch, Battle.net, la app de Xbox y todo lo que hayas instalado a mano.
- **Juegos de Steam en los que el desarrollador nunca lo activó**, o lo activó sólo para una plataforma.
- **Emuladores.** RetroArch, Dolphin, PCSX2, RPCS3 y compañía guardan donde les parece, y Steam no sabe nada de eso.
- **Juegos que escriben fuera de la carpeta que vigila Steam**, que son más de los que imaginas.

A Hoard le da igual quién publicara el juego o de dónde venga: rastrea la carpeta que cambia cuando juegas.

## Cuando dos PC tocan la misma partida

Juegas en el portátil sin dejar que el sobremesa termine de sincronizar y tienes el problema clásico: dos partidas, las dos más nuevas que la última versión común.

Hoard nunca sobrescribe a ciegas. Compara fechas de modificación, guarda una copia local de lo que reemplaza, y espera mientras haya un juego abierto o la partida se haya tocado en los últimos minutos: un fichero que se está escribiendo no es un fichero que quieras subir a medias. Todas las versiones anteriores siguen en el historial de la nube, así que equivocarte de versión cuesta dos clics y no un fin de semana.

El límite honesto: **Hoard no fusiona dos partidas divergentes.** Ninguna herramienta puede — un fichero de partida es opaco, y no existe una forma correcta de mezclar dos tardes distintas de juego. Lo que te da a cambio es todas las versiones, en todas las máquinas, y la posibilidad de elegir.

## Sincronizar sin pasar por nuestros servidores

Conviene decirlo explícitamente, porque es la parte que casi todas las comparativas se equivocan. Hay dos formas de usar esto:

- **Hoard Cloud** es la opción gestionada: inicias sesión y tus partidas se guardan en nuestros servidores, en la UE.
- **Autoalojarse es tuyo por completo.** Levantas `hoard-server` en tu PC o en tu NAS y tus máquinas sincronizan a través de él. **No hay cuenta con nosotros, ni telemetría hacia nosotros, ni cupo, ni relé**: no pasa nada por nuestros servidores, porque no hay nada nuestro en el camino. Mira [cómo autoalojar Hoard](/guides/self-host-hoard).

El mismo programa, la misma detección, el mismo historial de versiones. Lo único que cambia es de quién es el almacenamiento.

## Consejo

Deja que cada equipo termine de sincronizar antes de abrir un juego: el panel muestra el estado en vivo, así sabes que la última partida ya está en su sitio.

<!-- faq -->

## Preguntas frecuentes

### ¿Cuántos PC puedo sincronizar?

Tres en el plan gratuito, ilimitados en Pro, e ilimitados si te autoalojas: tu servidor, tus reglas.

### ¿Tienen que estar las dos máquinas encendidas a la vez?

No. Tu partida sube al servidor cuando terminas de jugar y baja cuando la otra máquina la pide, así que el segundo PC puede estar apagado una semana y aun así recibir la última versión al encenderse.

### ¿Y si juego sin conexión?

Sin problema. La instantánea se toma en local al dejar de jugar, y se sube sola en cuanto la máquina vuelve a tener conexión.

### ¿Sincroniza también mods y ajustes?

Las partidas, sí. Los ficheros que son de una máquina concreta — configuración, registros y similares — se suben para que estén en la copia, pero no se escriben encima de la copia de otro PC, porque un ajuste gráfico que le va bien a tu sobremesa rara vez es el que quiere tu portátil.

### ¿Autoalojarse envía algo a Hoard?

No. En modo autoalojado no hay cuenta con nosotros ni telemetría hacia nosotros: tus partidas, tus usuarios y tus registros viven en tu propio servidor y nunca tocan el nuestro.
