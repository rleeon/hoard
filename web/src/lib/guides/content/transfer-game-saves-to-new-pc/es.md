---
title: "Cómo pasar tus partidas guardadas a un PC nuevo"
description: "¿PC nuevo o reinstalas Windows? Llévate todas tus partidas: qué cubre Steam Cloud, cómo hacerlo a mano, cómo hacerlo solo y las trampas."
order: 13
updated: 2026-10-09
---

Los juegos con Steam Cloud vuelven solos cuando inicias sesión en el PC nuevo. Todo lo demás es una carpeta que tienes que llevarte tú: juegos sin partidas en la nube, emuladores, cualquier cosa fuera de Steam. Puedes copiar esas carpetas a mano, o dejar que Hoard las respalde en el PC viejo y ponga cada una en su sitio en el nuevo. Aquí van las dos formas, y las trampas que le cuestan las partidas a la gente.

## Antes de borrar nada

- **Haz una lista de lo que juegas**, incluidos los juegos que llevas meses sin tocar. Son los que se olvidan.
- **Mira qué juegos tienen partidas en la nube.** En Steam lo dice la página de la tienda, y **Propiedades → General** muestra si está activado. Epic y GOG también lo indican juego a juego.
- **Copia el resto, y a ser posible todo.** Las partidas en la nube guardan una sola copia, la última. Si esa copia está rota, está rota en todas partes.

## A mano

1. **Busca la carpeta de cada juego.** La mayoría están en `Documents\My Games`, `Saved Games` o las carpetas de `AppData` (`Roaming`, `Local`, `LocalLow`). La lista completa está en [dónde guardan las partidas los juegos de PC](/guides/where-are-pc-game-saves-stored).
2. **Cópialas a un disco externo**, manteniendo la estructura de carpetas. Llévate también la carpeta `userdata` de Steam entera: pesa poco y cubre los juegos que guardan a través de Steam sin tener Steam Cloud activado.
3. **En el PC nuevo, instala primero el juego.** Si el juego necesita crear sus carpetas, ábrelo una vez y ciérralo en el menú principal. No empieces partida.
4. **Copia las partidas a su sitio** y abre el juego. Comprueba que tu progreso está ahí antes de borrar nada del disco viejo.

Funciona. El inconveniente es que es una copia de una sola vez: tienes que acordarte de todas las carpetas, y si sigues jugando en el PC viejo, los dos se separan desde ese día.

## Las trampas

- **Partidas ligadas a una cuenta.** Algunos juegos meten el ID de tu cuenta en el nombre de la carpeta o dentro de la partida: Elden Ring guarda bajo tu SteamID, y los juegos de Ubisoft bajo tu ID de Ubisoft. Misma cuenta en los dos PC: sin problema. Otra cuenta: el juego ve la ranura vacía.
- **OneDrive movió Documentos.** Si un PC hace copia de `Documentos` con OneDrive y el otro no, la "misma" carpeta está en dos sitios distintos. Haz clic derecho en `Documentos` y abre **Propiedades → Ubicación** para ver dónde está de verdad. Más en [OneDrive y las partidas guardadas](/guides/onedrive-game-saves).
- **Versiones del juego.** Una partida de una versión más nueva del juego puede no cargar en una más antigua. Actualiza el juego en el PC nuevo antes de copiar.
- **Mods.** Una partida con mods (sobre todo en juegos de Bethesda) puede negarse a cargar sin los mismos mods. Reinstálalos primero.
- **De Windows a Steam Deck o Linux.** La partida va dentro del prefijo de Proton del juego, que solo existe después de abrir el juego una vez. Mira [sincronizar partidas entre Steam Deck y PC](/guides/sync-saves-steam-deck-pc).

## De forma automática

Hoard convierte la mudanza en lo mismo que hace cada día: respaldar en una máquina y restaurar en otra.

1. **En el PC viejo**, instala Hoard e inicia sesión. Abre la **Biblioteca**: Hoard lista las partidas que ha encontrado de tus juegos, con la misma base de datos comunitaria que Ludusavi. Añade lo que falte señalando su carpeta.
2. **Comprueba que cada juego tiene una versión** en su **Historial**. Esa es tu red de seguridad antes de formatear el disco viejo.
3. **En el PC nuevo**, instala Hoard, inicia sesión con la misma cuenta e instala tus juegos. Hoard los empareja con sus copias por juego y restaura la última versión en la carpeta que espera esta máquina, aunque la ruta cambie (otro disco, otro nombre de usuario, un prefijo de Proton en un Deck).
4. **Antes de empezar una partida nueva**, deja que Hoard termine de devolver tus partidas. La app muestra el estado de cada juego.

Dos detalles lo hacen más seguro que una copia. Los ficheros de ajustes como `graphics.ini` se respaldan pero no se escriben encima de los del PC nuevo, así que tu hardware nuevo arranca con ajustes que le sientan bien (puedes traerlos al restaurar si las dos máquinas son parecidas). Y nada es definitivo: todas las versiones siguen en el historial, así que una restauración equivocada se deshace restaurando la anterior.

Si el PC viejo sigue en uso, simplemente sigue sincronizándose con el nuevo. Si se va para siempre, quítalo de tus dispositivos. El plan gratuito incluye tres.

## Sin nuestros servidores

Puedes hacer todo esto contra tu propio servidor: levanta `hoard-server` en un PC o un NAS, apunta las dos máquinas a él y las partidas no salen de casa. Sin cuenta con nosotros, sin telemetría hacia nosotros. Mira [cómo autoalojar Hoard](/guides/self-host-hoard).

<!-- faq -->

## Preguntas frecuentes

### ¿Las partidas de Steam se pasan solas?

Solo las de juegos con Steam Cloud. Inicia sesión en el PC nuevo, instala el juego y la partida se descarga. Los juegos sin Steam Cloud necesitan que copies su carpeta, o una herramienta que lo haga por ti.

### ¿Puedo copiar mi carpeta de usuario entera?

Funciona con la mayoría de partidas, pero también arrastra gigas de cachés, ajustes pensados para el hardware viejo y datos de aplicaciones que pueden dar guerra en una instalación nueva. Copiar solo las carpetas de guardado es más limpio.

### ¿Funcionarán mis partidas si mi usuario de Windows tiene otro nombre?

Sí, casi siempre. Las partidas se guardan dentro de tu carpeta de usuario, así que el nombre de la ruta da igual. Hoard lo resuelve solo.

### ¿Puedo pasar partidas de Windows a una Steam Deck?

Sí. Abre el juego una vez en el Deck para que exista su prefijo de Proton y coloca la partida dentro, o deja que lo haga Hoard. Mira [la guía de Steam Deck](/guides/sync-saves-steam-deck-pc).

### ¿Tengo que conservar el PC viejo hasta tener listo el nuevo?

Si copias a mano, guarda el disco externo hasta haber comprobado todos los juegos. Con Hoard, las partidas ya están en el servidor, así que el PC viejo puede irse en cuanto cada juego muestre una versión en su historial.
