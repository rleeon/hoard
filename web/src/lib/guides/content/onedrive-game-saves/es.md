---
title: "OneDrive y las partidas guardadas: qué se rompe y cómo arreglarlo"
description: "OneDrive movió Documentos y ahora las partidas fallan, desaparecen o salen duplicadas. Por qué pasa, cómo arreglarlo y una forma mejor de sincronizar."
order: 15
updated: 2026-10-09
---

En muchos PC con Windows, OneDrive hace copia de la carpeta `Documentos`, a menudo activada durante la instalación sin que nadie se dé cuenta. Los juegos que guardan en `Documentos`, que es casi todo `My Games`, la siguen dentro de OneDrive. Y ahí empiezan los problemas: partidas que no se guardan, partidas que necesitan descargarse antes de cargar, y copias con el nombre de tu PC pegado que el juego nunca lee. Aquí tienes por qué pasa y cómo arreglarlo.

## Cómo acabaron tus partidas en OneDrive

La copia de carpetas de OneDrive mueve `Documentos`, `Escritorio` e `Imágenes` a `C:\Users\<tú>\OneDrive\...`. Los juegos le preguntan a Windows dónde está `Documentos`, así que la siguen sin decir nada. Para comprobarlo, haz clic derecho en `Documentos` y abre **Propiedades → Ubicación**: si la ruta lleva `OneDrive`, tus partidas están dentro.

`AppData` y `Saved Games` no entran en esa copia, así que los juegos que guardan ahí no se ven afectados.

## Qué sale mal

- **Escrituras que chocan.** OneDrive sube los ficheros en cuanto cambian. Un juego que escribe su partida en ese mismo momento puede encontrarse el fichero en uso.
- **Partidas que solo están en la nube.** OneDrive puede liberar espacio dejando ficheros solo en la nube (el icono de la nube). El juego necesita entonces descargar la partida antes de cargarla, y sin conexión no hay nada que cargar.
- **Copias en conflicto.** Si usas OneDrive en dos PC con la misma cuenta, los dos sincronizan el mismo `My Games`. Juega en los dos antes de que uno se ponga al día y OneDrive conserva las dos versiones renombrando una con el nombre del PC. El juego ignora ese fichero.
- **Espacio.** El plan gratuito son 5 GB, y algunos juegos guardan además mods, cachés o grabaciones en `Documentos`.
- **No sabe qué es una sesión de juego.** OneDrive sincroniza fichero a fichero, en mitad de la partida, y versiona cada fichero por separado en lugar de la partida entera.

## Cómo arreglarlo

### Rápido: mantén las partidas en el equipo

Haz clic derecho en `Documentos\My Games` (o en la carpeta del juego) y elige **Mantener siempre en este dispositivo**. Eso acaba con el problema de los ficheros solo en la nube. No evita que OneDrive sincronice mientras juegas.

### Limpio: deja de hacer copia de Documentos

En OneDrive, abre **Configuración → Sincronización y copia de seguridad → Administrar copia de seguridad** y desactiva `Documentos`. Windows vuelve a apuntar `Documentos` a la carpeta local, pero los ficheros que ya se copiaron se quedan en la carpeta de OneDrive. Antes de desactivarlo, márcalos como **Mantener siempre en este dispositivo** para que estén de verdad en el disco. Después, con los juegos cerrados, mueve las carpetas de los juegos de vuelta al `Documentos` local, o los juegos empezarán de cero.

## Un reparto mejor

OneDrive es bueno con los documentos. Las partidas necesitan otra cosa: una copia hecha cuando el juego se ha cerrado, versiones de la partida entera y no de ficheros sueltos, y sincronización con tus otros PC y con una Steam Deck.

Eso es lo que hace Hoard. Encuentra tus partidas tanto si `Documentos` está en OneDrive como si no, porque le pregunta a Windows dónde está de verdad la carpeta. Las respalda automáticamente después de cada sesión, guarda todas las versiones y las sincroniza con tus otras máquinas.

Una regla: que la sincronización entre PC sea cosa de una sola herramienta. Si OneDrive hace copia de `Documentos` en varios PC de juego con la misma cuenta, ya está sincronizando esas partidas entre ellos. Desactiva la copia de `Documentos` en esos PC y deja las partidas a Hoard.

¿Prefieres nada de nube? Levanta `hoard-server` en tu PC o tu NAS: sin cuenta con nosotros, sin telemetría hacia nosotros, sin nada pasando por nuestros servidores. Mira [cómo autoalojar Hoard](/guides/self-host-hoard).

<!-- faq -->

## Preguntas frecuentes

### ¿Dejo que OneDrive haga copia de mis partidas?

En un solo PC, como copia, es mejor que nada. Como forma de sincronizar partidas entre PC da conflictos, porque no sabe cuándo hay un juego abierto.

### Desactivé la copia y mis partidas han desaparecido. ¿Dónde están?

Siguen en la carpeta de OneDrive: `C:\Users\<tú>\OneDrive\Documents\My Games`. Cierra los juegos y muévelas de vuelta al `Documentos` local.

### ¿Qué son los ficheros de partida con el nombre de mi PC?

Copias en conflicto de OneDrive. Dos PC cambiaron el mismo fichero antes de sincronizarse, y OneDrive guardó los dos. Averigua cuál es el más nuevo, ponle el nombre original y aparta el otro.

### ¿OneDrive puede devolverme una partida anterior?

A veces. En onedrive.com, haz clic derecho en el fichero y elige **Historial de versiones**. Funciona fichero a fichero y solo durante un tiempo limitado. Mira [cómo recuperar una partida corrupta](/guides/recover-corrupted-game-save).

### ¿Hoard funciona si mi carpeta Documentos está en OneDrive?

Sí. Hoard lee dónde dice Windows que está `Documentos`, así que encuentra las partidas en cualquiera de los dos sitios.
