---
title: "Partidas en la nube para PCSX2: sincroniza tus tarjetas de memoria entre PC y Steam Deck"
description: "PCSX2 no tiene partidas en la nube. Sincroniza tus tarjetas de PS2 entre PC y Steam Deck automáticamente, con historial: rutas, tarjetas de carpeta y trampas."
order: 16
updated: 2026-10-09
---

PCSX2 no sincroniza las partidas por su cuenta: tu progreso de PS2 vive en ficheros de tarjeta de memoria en una sola máquina, y la otra nunca se entera. Hoard las sincroniza de forma automática. Cuando cierras PCSX2, respalda tus tarjetas de memoria, las baja en tus otros PC y en tu Steam Deck, y guarda todas las versiones para que una mala partida nunca te cueste la aventura.

## Dónde guarda PCSX2 tus partidas

PCSX2 guarda como una PS2 de verdad: en tarjetas de memoria. Por defecto hay dos, `Mcd001.ps2` y `Mcd002.ps2`, de 8 MB cada una, en una carpeta `memcards`. Una sola tarjeta guarda las partidas de todos los juegos que hayas jugado con ella.

- **Windows:** `Documents\PCSX2\memcards`. En modo portátil, la carpeta está junto al programa.
- **Linux:** `~/.config/PCSX2/memcards`.
- **Steam Deck** (el Flatpak de Discover): `~/.var/app/net.pcsx2.PCSX2/config/PCSX2/memcards`. Con EmuDeck, el enlace de `~/Emulation/saves/pcsx2` apunta a una de estas.
- **Mac:** `~/Library/Application Support/PCSX2/memcards`.

En PCSX2, **Settings → Memory Cards** muestra la carpeta que está usando de verdad y qué tarjeta hay en cada ranura. Hoard encuentra solo las carpetas de Windows, Linux y Steam Deck; en un Mac o en una instalación portátil, señala la carpeta `memcards` una vez.

## Tarjetas de fichero y tarjetas de carpeta

PCSX2 puede crear dos tipos de tarjeta de memoria, y al sincronizar la elección importa más de lo que parece.

- **Una tarjeta de fichero** (`.ps2`) es un solo fichero de 8 MB con las partidas de todos los juegos dentro. Guardas en cualquier juego y cambia el fichero entero, así que cada versión nueva son los 8 MB completos.
- **Una tarjeta de carpeta** es una carpeta en vez de un fichero, con cada partida en su propia subcarpeta. Guardas en un juego y solo cambian los ficheros de ese juego, así que las versiones pesan poco y el historial enseña qué partida se movió.

Puedes crear cualquiera de las dos desde **Settings → Memory Cards**. Elijas la que elijas, usa **el mismo tipo, los mismos nombres y las mismas ranuras** en todas las máquinas. Una tarjeta de fichero en el sobremesa y una de carpeta en el Deck son dos tarjetas distintas, y cada máquina creerá que la partida de la otra no existe.

## Cómo funciona la sincronización en el día a día

Juegas en el sobremesa y cierras PCSX2. Hoard espera a que PCSX2 haya salido y a que las tarjetas dejen de cambiar, y sube la versión nueva. Más tarde coges la Steam Deck. En cuanto tiene conexión, Hoard baja las tarjetas más nuevas, y cuando abres PCSX2 tu partida está ahí. Ciérralo en el Deck y pasa lo mismo en sentido contrario.

Ninguna de las dos máquinas tiene que estar encendida a la vez. Las tarjetas esperan en el servidor hasta que la otra máquina las pide.

## Trampas que conviene conocer

- **Cierra PCSX2, no solo el juego.** Hoard respalda cuando el emulador ha salido, así que nunca copia una tarjeta a mitad de escritura. En un Deck, suspender no cuenta como cerrar.
- **Mismo disco, misma región.** Las versiones PAL y NTSC de un juego tienen números de serie distintos (SLES y SLUS, por ejemplo), así que no ven las partidas de la otra. Usa la misma imagen de disco en todas partes.
- **Los estados son otra cosa.** Los estados (ficheros `.p2s` en `sstates`) son fotos del emulador y a menudo no cargan en otra versión de PCSX2. Hoard sincroniza las tarjetas de memoria; si quieres que también viajen los estados, añade la carpeta `sstates` como elemento propio y ten PCSX2 en la misma versión en todas las máquinas.
- **Una versión es la tarjeta entera.** Restaurar una versión anterior devuelve la tarjeta entera tal como estaba, con todos sus juegos. Hoard te enseña qué va a cambiar antes de confirmar, y tu tarjeta actual se guarda primero, así que una restauración siempre se puede deshacer.

## Configurarlo

1. Instala Hoard en cada máquina e inicia sesión con la misma cuenta.
2. En la **Biblioteca**, añade PCSX2 desde la lista de emuladores.
3. Comprueba que todas las máquinas usan el mismo tipo de tarjeta, los mismos nombres y las mismas ranuras.
4. Juega, cierra PCSX2 y sigue en la otra máquina.

¿Prefieres que se quede en casa? Levanta `hoard-server` en tu PC o tu NAS y apunta a él todas las máquinas. Sin cuenta con nosotros, sin telemetría hacia nosotros, sin nada pasando por nuestros servidores. Mira [cómo autoalojar Hoard](/guides/self-host-hoard). Para el resto de emuladores, mira [partidas de emuladores](/guides/back-up-emulator-saves).

<!-- faq -->

## Preguntas frecuentes

### ¿PCSX2 tiene partidas en la nube?

No. PCSX2 escribe las tarjetas de memoria en una carpeta local y la sincronización te la deja a ti. Hoard es una forma de hacerlo de forma automática, con historial de versiones encima.

### ¿Puedo sincronizar las partidas de PCSX2 entre un PC y una Steam Deck?

Sí. Instala Hoard en los dos con la misma cuenta. Hoard sabe dónde guarda PCSX2 sus tarjetas en Windows y en el Flatpak de la Steam Deck, y las empareja entre máquinas.

### ¿Uso tarjeta de fichero o de carpeta?

Para sincronizar, encaja mejor la de carpeta: solo se suben las partidas que han cambiado, y el historial enseña qué juego se movió. Las dos funcionan, siempre que todas las máquinas usen la misma.

### ¿Hoard sincroniza los estados de PCSX2?

No por defecto, porque los estados se rompen entre versiones de PCSX2. Añade la carpeta `sstates` a mano si los quieres, y ten PCSX2 en la misma versión en todas partes.

### ¿Restaurar una versión antigua devuelve atrás todos los juegos de la tarjeta?

Sí. Una versión es la tarjeta entera. Hoard enseña antes qué va a cambiar, y guarda también como versión la tarjeta que reemplazas.

### ¿Funciona con emuladores de PS2 en Android?

Hoy no. Hoard funciona en Windows, macOS, Linux y Steam Deck.
