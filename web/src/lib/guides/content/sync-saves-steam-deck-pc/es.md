---
title: "Cómo sincronizar partidas entre Steam Deck y PC"
description: "Sincroniza tus partidas entre Steam Deck y PC de forma automática, también juegos fuera de Steam, emuladores y juegos sin Steam Cloud. Pasos, rutas y trampas."
order: 10
updated: 2026-10-09
---

En los juegos de Steam con Steam Cloud, tu Deck y tu PC ya comparten las partidas. Todo lo demás necesita ayuda: juegos en los que el desarrollador nunca activó Steam Cloud, juegos de Epic y GOG que lanzas con Heroic, emuladores y cualquier cosa que hayas añadido como juego ajeno a Steam. Hoard los cubre todos de forma automática. Cuando cierras un juego en una máquina, respalda la partida, y la otra máquina la descarga, con todas las versiones anteriores guardadas por si algo sale mal.

## Lo que Steam Cloud ya hace en el Deck

Si un juego tiene Steam Cloud, Steam sube la partida al cerrarlo y la baja al abrirlo en otra máquina. En la página de la tienda puedes ver si lo tiene, y desactivarlo juego a juego en **Propiedades → General**.

Los huecos son los de siempre:

- **Juegos que no lo tienen.** Lo decide el desarrollador, juego a juego, y muchos juegos de PC nunca lo activaron.
- **Todo lo que está fuera de Steam.** Heroic, Lutris, emuladores, un juego que instalaste a mano.
- **No hay marcha atrás.** Steam guarda la partida actual, no un historial. Si se sincroniza una partida rota, la buena desaparece en las dos máquinas.

Lo tienes más desarrollado en la guía de [alternativa a Steam Cloud](/guides/steam-cloud-alternative).

## Dónde guarda el Deck tus partidas

El Deck ejecuta los juegos de Windows con Proton, así que el mismo juego guarda en un sitio distinto que en tu PC:

- **Juegos de Windows (Proton):** `~/.local/share/Steam/steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/`, seguido de la ruta habitual de Windows: `Documents`, `AppData/Roaming`, `AppData/Local`, `AppData/LocalLow` o `Saved Games`. El AppID es el número que aparece en la URL del juego en la tienda.
- **Juegos en la microSD:** la tarjeta tiene su propio `steamapps/compatdata/<AppID>`, con el mismo árbol dentro.
- **Juegos nativos de Linux:** normalmente `~/.local/share/<juego>` o `~/.config/<juego>`. Los juegos hechos con Unity usan `~/.config/unity3d/<Empresa>/<Juego>`.
- **Heroic, Lutris y Bottles:** cada uno mantiene un prefijo de Wine por juego, con el árbol de Windows en `drive_c/users/<tu usuario>/` en lugar de `steamuser`.
- **Emuladores:** EmuDeck los reúne en `~/Emulation/saves/`. Mira [partidas de emuladores](/guides/back-up-emulator-saves) y [RetroArch](/guides/retroarch-save-sync).

En tu PC, ese mismo juego escribe en `C:\Users\<tú>\...`. Dos rutas distintas para una sola partida: por eso copiar carpetas a mano sale mal. Hoard busca en todos estos sitios y empareja lo que encuentra por juego, así que la partida del Deck y la del PC pasan a ser dos versiones de un mismo historial.

## Configurarlo

1. En el Deck, cambia al modo escritorio: **botón Steam → Apagar → Cambiar al escritorio**.
2. Abre un navegador, entra en [la página de descarga](/download) y baja **Hoard Setup** para Linux. En el gestor de archivos, abre las propiedades del fichero, permite que se ejecute como programa y ábrelo.
3. Inicia sesión con la misma cuenta que usas en tu PC, o apunta la app a tu propio servidor.
4. Abre la **Biblioteca** y revisa lo que ha encontrado Hoard. Añade lo que falte señalando su carpeta: un prefijo de Heroic, un emulador, un juego que instalaste tú.
5. Instala Hoard en tu PC con la misma cuenta. Los mismos juegos se emparejan solos.
6. Vuelve al modo juego. No hace falta volver al escritorio.

Hoard Setup deja la app en tu carpeta personal y el motor de sincronización en un servicio en segundo plano que arranca con el Deck. No escribe nada en el sistema de solo lectura de SteamOS, así que las actualizaciones del sistema no lo tocan.

## Cómo es un día normal

Juegas en el PC por la noche y cierras el juego. Hoard espera a que se haya cerrado y a que la partida deje de cambiar, y entonces la sube. A la mañana siguiente coges el Deck. En cuanto tiene conexión, Hoard ve la versión más nueva y la escribe en el prefijo de Proton. Abres el juego y sigues. Cuando lo cierras en el Deck, pasa lo mismo en sentido contrario.

Ninguna de las dos máquinas tiene que estar encendida a la vez que la otra. La partida espera en el servidor hasta que la otra la pide.

## Las trampas que conviene conocer

### Suspender no es cerrar

El Deck pone muy fácil pulsar el botón de encendido y dejar el juego abierto. Hoard solo respalda una partida cuando el juego se ha cerrado, porque un juego en marcha puede estar a medio escribirla. Y nunca cambia la partida de un juego que está abierto. Así que si suspendes el Deck y luego juegas en el PC, el progreso del Deck todavía no se ha subido, y la partida nueva del PC espera hasta que cierres el juego en el Deck.

La costumbre que lo evita todo: **cierra el juego antes de cambiar de máquina.** Si Proton deja un proceso muerto al salir, cosa que pasa a menudo, Hoard se da cuenta de que el juego ya no está y sigue.

### Dale unos segundos al despertar

Cuando el Deck sale de la suspensión, el wifi tarda un momento en volver, y solo entonces puede Hoard buscar una partida más nueva. Si abres un juego en esos primeros segundos, la descarga espera a que lo cierres. Dale un momento con conexión antes de ponerte a jugar.

### La microSD

Si un juego está en la tarjeta y la tarjeta no está puesta, Hoard no descarga una partida a una carpeta que no existe. Espera a que vuelva la tarjeta.

### Los ajustes se quedan en cada máquina

El Deck funciona a 1280×800 con una GPU de portátil. Tu sobremesa, seguramente no. Hoard respalda los ficheros de ajustes como `graphics.ini` junto con la partida, pero no los escribe encima de los de la otra máquina, así que el Deck conserva los suyos. Si aun así quieres copiarlos, hay una opción para ello al restaurar. Más en [sincronizar partidas entre varios PC](/guides/sync-game-saves-across-pcs).

### La carpeta `remote` de Steam

En los juegos de Steam, la partida está en `userdata/<UserID>/<AppID>/remote/`. La carpeta de encima también guarda `remotecache.vdf` y ficheros de tiempo de juego y logros que tienen que ser distintos en el Deck y en el PC. Si sincronizas a mano la carpeta padre, cada arranque parece un conflicto. Hoard solo sigue `remote/`.

## Steam Cloud y Hoard a la vez

No se estorban. En un juego con Steam Cloud, deja que Steam siga sincronizándolo. Lo que añade Hoard ahí es el historial de versiones, para que una partida rota en una máquina no se lleve tu progreso. En todos los demás juegos, Hoard también se encarga de sincronizar.

## Sin nuestros servidores

Si prefieres que las partidas no salgan de casa, levanta `hoard-server` en tu PC o en un NAS y apunta a él tanto el Deck como el PC. Sin cuenta con nosotros, sin telemetría hacia nosotros, sin nada pasando por nuestros servidores. Mira [cómo autoalojar Hoard](/guides/self-host-hoard).

<!-- faq -->

## Preguntas frecuentes

### ¿Hoard funciona en el modo juego?

Sí. El motor de sincronización es un servicio en segundo plano que arranca con el Deck, así que respalda y restaura sin ninguna ventana abierta. Solo necesitas el modo escritorio para instalarlo y para añadir carpetas a mano.

### ¿Una actualización de SteamOS lo borra?

No. Todo lo que instala Hoard vive en tu carpeta personal, y las actualizaciones de SteamOS no la tocan.

### ¿Sincroniza juegos de Heroic, Lutris o EmuDeck?

Sí. Hoard busca dentro de los prefijos de Heroic, Lutris y Bottles y en las carpetas de EmuDeck. Si un juego no aparece, señala su carpeta de guardado una vez y queda seguido como cualquier otro.

### ¿Y si he jugado en los dos sin sincronizar?

Hoard nunca sobrescribe a ciegas. Compara versiones, guarda una copia de lo que reemplaza y todas las versiones anteriores siguen en el historial. No puede fusionar dos sesiones de juego distintas en una sola partida (nada puede), pero puedes elegir con cuál quedarte.

### ¿El Deck cuenta como dispositivo?

Sí. El plan gratuito incluye tres dispositivos, así que caben un PC, un portátil y un Deck. Pro y los servidores autoalojados no tienen límite de dispositivos.

### ¿Puedo usar la versión de línea de comandos en el Deck?

Sí. El comando `hoard` ejecuta el mismo motor sin ventana, y hay quien lo prefiere en una portátil. Mira [la página de la CLI](/cli).
