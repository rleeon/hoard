---
title: "Dónde están las partidas de Cyberpunk 2077 (PC y Steam Deck)"
description: "Dónde guarda Cyberpunk 2077 sus partidas en Windows, Steam Deck y Mac, qué hay en cada carpeta y cómo copiarlas o llevarlas de un PC a otro."
order: 20
updated: 2026-10-02
---

En Windows, Cyberpunk 2077 guarda sus partidas en `%USERPROFILE%\Saved Games\CD Projekt Red\Cyberpunk 2077`, una carpeta por partida. Ésa es la respuesta corta. El resto de la página cubre las rutas de Steam Deck y Mac, qué hay de verdad en la carpeta y cómo tenerla siempre copiada.

## Dónde guarda Cyberpunk 2077 las partidas

- **Windows** (Steam, GOG o Epic): `%USERPROFILE%\Saved Games\CD Projekt Red\Cyberpunk 2077`
- **Steam Deck y Linux** (Proton): `~/.local/share/Steam/steamapps/compatdata/1091500/pfx/drive_c/users/steamuser/Saved Games/CD Projekt Red/Cyberpunk 2077`
- **Mac:** `~/Library/Application Support/CD Projekt Red/Cyberpunk 2077/Saves`

Las rutas de Windows y Mac son las que da CD Projekt Red en sus propias páginas de soporte. En una Steam Deck el juego corre dentro de un prefijo de Proton, un pequeño árbol de carpetas de Windows que Steam mantiene para cada juego; `1091500` es el ID de Cyberpunk en Steam. Si el juego está instalado en la microSD, busca `steamapps/compatdata/1091500` en la tarjeta.

## Qué hay en la carpeta

Cyberpunk no escribe un único fichero de partida. Escribe **una carpeta por partida**: `AutoSave-0`, `AutoSave-1` y siguientes, `ManualSave-0`, `ManualSave-1`, y `QuickSave-0`. Cada una guarda la partida en sí (`sav.dat`) y la captura y los metadatos que enseña el menú de carga.

De ahí salen dos cosas:

- **Copia la carpeta padre, no una partida suelta.** Copiar sólo el `ManualSave` más reciente deja fuera los autoguardados, que muchas veces tienen el progreso más nuevo.
- **Los autoguardados rotan.** El juego reutiliza unas pocas carpetas `AutoSave` y sobrescribe la más antigua. Un autoguardado de hace tres horas normalmente ya no existe, y por eso compensa tener un historial fuera del juego.

Los ajustes no están aquí. Los gráficos y controles viven en `UserSettings.json`, dentro de `%LOCALAPPDATA%\CD Projekt Red\Cyberpunk 2077`, junto a cachés y registros. Ésa es la carpeta que mucha gente (y alguna herramienta) copia por error: no guarda nada cuya pérdida te cueste progreso.

## ¿Cyberpunk 2077 tiene partidas en la nube?

Sí. La versión de Steam usa Steam Cloud y la de GOG, la nube de GOG Galaxy. Las dos mantienen al día el último estado de tus partidas entre máquinas de la misma tienda.

Lo que no hace ninguna:

- **Guardar versiones anteriores.** Si una partida se corrompe, o un mod la rompe, la nube también se queda con la copia rota.
- **Cruzar tiendas.** Steam Cloud y la nube de GOG no se hablan, aunque las partidas de PC de Steam, GOG y Epic cargan sin problema en cualquiera si copias la carpeta.

## Cópiala a mano

1. Cierra el juego del todo.
2. Copia la carpeta `Cyberpunk 2077` entera desde la ruta de arriba a un USB, otro disco o una carpeta en la nube.
3. Para restaurar, cierra el juego y vuelve a copiar la carpeta, sustituyendo lo que haya.

Funciona, pero sólo tan a menudo como te acuerdes, y sólo tienes la copia de la última vez.

## Copia y sincronización automáticas con Hoard

[Hoard](/download) copia la carpeta de partidas cada vez que dejas de jugar y guarda todas las versiones, así que una partida corrupta o un autoguardado que ya rotó están a un clic. También sincroniza la carpeta entre tus PC y una Steam Deck.

1. Instala Hoard e inicia sesión, o apúntalo a [tu propio servidor](/guides/self-host-hoard).
2. Abre la **Biblioteca**. Cyberpunk se detecta a partir de tu biblioteca de Steam y de la base de datos comunitaria de partidas.
3. Comprueba que la carpeta que aparece es la de `Saved Games\CD Projekt Red\Cyberpunk 2077`. Si aparece la de `AppData\Local`, cámbiala: ésa sólo tiene ajustes.
4. Juega. Al salir, la primera versión aparece en el historial.

Hoard rastrea la carpeta entera, así que cada `AutoSave`, `ManualSave` y `QuickSave` entra en la misma versión. Con una Deck y un sobremesa, la versión más nueva te espera en el que cojas después; mira [cómo funciona la sincronización entre PC](/guides/sync-game-saves-across-pcs).

<!-- faq -->

## Preguntas frecuentes

### ¿Dónde están las partidas de Cyberpunk 2077 en Steam Deck?

Dentro del prefijo de Proton del juego: `~/.local/share/Steam/steamapps/compatdata/1091500/pfx/drive_c/users/steamuser/Saved Games/CD Projekt Red/Cyberpunk 2077`. Si el juego está en la microSD, la carpeta `compatdata` está en la tarjeta.

### ¿Puedo pasar mis partidas de Cyberpunk 2077 de GOG a Steam?

Sí. Las partidas de PC son las mismas en Steam, GOG y Epic. Copia las carpetas de partida a la misma ruta en la otra instalación con el juego cerrado y aparecerán en el menú de carga.

### ¿Por qué hay tantas carpetas AutoSave?

El juego mantiene unas cuantas ranuras de autoguardado y sobrescribe la más antigua cada vez. Son partidas normales; sólo que se sustituyen solas.

### ¿Por qué ha desaparecido mi autoguardado antiguo?

Porque la ranura donde estaba se reutilizó. El juego sólo guarda unas pocas. Una herramienta de copias que guarde versiones es la única forma de recuperarlo cuando ya ha rotado.

### ¿Hoard sincroniza también mis ajustes?

No. Los ajustes viven en otra carpeta que no forma parte de la partida, así que cada máquina conserva los suyos, que normalmente es lo que quieres: una Deck y un sobremesa necesitan ajustes gráficos distintos. Más sobre esto en [sincronizar partidas entre PC](/guides/sync-game-saves-across-pcs).
