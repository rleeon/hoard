---
title: "¿Dónde se guardan las partidas en PC? Todas las ubicaciones"
description: "Dónde guardan las partidas los juegos de PC en Windows, Steam Deck, Linux y Mac, qué launchers añaden carpetas propias y cómo encontrar las de cualquier juego."
order: 11
updated: 2026-10-09
---

No hay una sola carpeta. En Windows, casi todos los juegos guardan en uno de seis sitios: `Documents`, `Saved Games`, una de las tres carpetas de `AppData`, el `userdata` de Steam o su propia carpeta de instalación. Lo decide el motor y el desarrollador, no la tienda donde lo compraste. Aquí tienes todas las ubicaciones habituales, los launchers que añaden una capa propia y una forma rápida de encontrar las partidas de cualquier juego, incluso de uno que nadie ha documentado.

## Windows: los seis sitios de siempre

| Carpeta | Ruta típica | Quién la usa |
|---|---|---|
| Documentos | `%USERPROFILE%\Documents\My Games\<Juego>` | Juegos de Bethesda, Rockstar (`Documents\Rockstar Games`), muchos lanzamientos grandes antiguos |
| Partidas guardadas | `%USERPROFILE%\Saved Games\<Editora>\<Juego>` | Cyberpunk 2077 y una minoría testaruda |
| AppData\Roaming | `%APPDATA%\<Juego>` | Elden Ring, Stardew Valley, Minecraft (`.minecraft`), muchos indies |
| AppData\Local | `%LOCALAPPDATA%\<Juego>\Saved\SaveGames` | Juegos con Unreal Engine (Palworld usa `Pal\Saved\SaveGames`) |
| AppData\LocalLow | `%USERPROFILE%\AppData\LocalLow\<Empresa>\<Juego>` | Juegos con Unity (Hollow Knight y muchos más) |
| userdata de Steam | `C:\Program Files (x86)\Steam\userdata\<UserID>\<AppID>\remote` | Juegos que usan el almacenamiento de partidas de Steam |

Y un séptimo que se niega a desaparecer: **la carpeta de instalación del juego**, donde muchos juegos antiguos y algunos indies siguen escribiendo.

Dos apuntes prácticos. `AppData` está oculta, así que escribe `%APPDATA%` o `%LOCALAPPDATA%` en la barra de direcciones del Explorador en lugar de ir clicando. Y si OneDrive hace copia de tu carpeta `Documentos`, su ruta real es `C:\Users\<tú>\OneDrive\Documents`, cosa que sorprende a mucha gente. Mira [OneDrive y las partidas guardadas](/guides/onedrive-game-saves).

## Launchers que añaden su propia capa

La mayoría de launchers no deciden dónde van las partidas; lo decide el juego. Hay excepciones:

- **Steam** tiene una zona de partidas por juego en `userdata`. `<UserID>` es un número ligado a tu cuenta de Steam (hay una carpeta por cada cuenta que haya iniciado sesión en ese PC) y `<AppID>` es el número de la URL del juego en la tienda.
- **Ubisoft Connect** guarda en `C:\Program Files (x86)\Ubisoft\Ubisoft Game Launcher\savegames\<ID de usuario>\<ID del juego>`, con números en vez de nombres en los dos niveles.
- **La app de Xbox y PC Game Pass** usan `%LOCALAPPDATA%\Packages\<paquete>\SystemAppData\wgs`, con ficheros de nombre aleatorio que solo entiende la app de Xbox. Deja esas a las partidas en la nube de Xbox; copiarlas a mano rara vez funciona.
- **Epic, GOG y la app de EA** suelen dejarlo en manos del juego, así que sus títulos acaban en los sitios de arriba. Sus partidas en la nube, cuando las hay, copian desde ahí.

## El registro, rara vez

Algunos juegos, sobre todo títulos pequeños hechos con Unity, guardan el progreso en el registro de Windows, en `HKEY_CURRENT_USER\Software\<Empresa>\<Juego>`, y no en un fichero. No hay nada que copiar en una carpeta, y las herramientas de copia basadas en carpetas, Hoard incluido, no lo ven. Si lo necesitas, exporta esa clave con `regedit`.

## Steam Deck y Linux

- **Juegos de Windows con Proton:** guardan dentro de un prefijo por juego, `~/.local/share/Steam/steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/`, seguido de la ruta de Windows de la tabla (`Documents`, `AppData/Roaming`, etc.). Los juegos instalados en una microSD tienen el mismo árbol en el `steamapps/compatdata` de la propia tarjeta.
- **Juegos nativos de Linux:** usan `~/.local/share/<juego>` o `~/.config/<juego>`. Los de Unity van a `~/.config/unity3d/<Empresa>/<Juego>`.
- **El userdata de Steam** está en `~/.local/share/Steam/userdata/<UserID>/<AppID>/remote`.
- **Heroic, Lutris y Bottles** mantienen un prefijo de Wine por juego. Dentro, el árbol de Windows cuelga de `drive_c/users/<tu usuario>/`, no de `steamuser`.

El Deck tiene su propia guía: [sincronizar partidas entre Steam Deck y PC](/guides/sync-saves-steam-deck-pc).

## Mac

- **La mayoría de juegos:** `~/Library/Application Support/<Juego>`. Los de Unity usan `~/Library/Application Support/<Empresa>/<Juego>`.
- **Juegos de la Mac App Store:** van aislados en `~/Library/Containers/<id del paquete>/Data/Library/Application Support/`.

`~/Library` también está oculta. En el Finder, abre el menú **Ir** con la tecla Opción pulsada y aparece.

## Cómo encontrar las partidas de cualquier juego

Cuando un juego no sale en ninguna lista, tres trucos lo encuentran en un par de minutos:

1. **Búscalo en PCGamingWiki.** Casi todas las fichas de juego tienen una sección "Save game data location". Es la misma fuente de la que salen las bases de datos de partidas que usan Hoard y Ludusavi.
2. **Mira qué cambia.** Guarda en el juego, ciérralo y busca en tu carpeta de usuario los ficheros modificados en los últimos minutos. En Windows, busca `datemodified:today` dentro de `C:\Users\<tú>` y ordena por fecha. En Linux o en un Deck: `find ~ -type f -mmin -5 -not -path '*/.cache/*'`.
3. **Pregúntale a Steam.** En un juego con Steam Cloud, [store.steampowered.com/account/remotestorage](https://store.steampowered.com/account/remotestorage) lista los ficheros que Steam guarda de cada juego, con nombre y tamaño. Sabiendo cómo se llama el fichero, encontrar la carpeta es fácil.

## O deja que alguien las encuentre por ti

Hoard lee esa misma base de datos comunitaria, que cubre miles de juegos, y comprueba cada ruta candidata en tu equipo: prefijos de Proton, Heroic y Lutris, el `Documentos` de OneDrive, emuladores, instalaciones portátiles. Lo que encuentra se respalda automáticamente cada vez que dejas de jugar, con todas las versiones guardadas, y se mantiene sincronizado entre tus PC y tu Steam Deck. Lo que se le escape lo añades señalando la carpeta una vez. Mira [cómo hacer copia de tus partidas automáticamente](/guides/back-up-game-saves).

También hay páginas con las rutas exactas de algunos juegos populares: [Cyberpunk 2077](/guides/cyberpunk-2077-save-location), [Baldur's Gate 3](/guides/baldurs-gate-3-save-location), [Palworld](/guides/palworld-save-location), [Crimson Desert](/guides/crimson-desert-save-location) y [Marvel's Spider-Man 2](/guides/spider-man-2-save-location).

<!-- faq -->

## Preguntas frecuentes

### ¿Dónde guarda Steam las partidas?

Depende del juego. Algunos usan la zona propia de Steam, `Steam\userdata\<UserID>\<AppID>\remote`; la mayoría escribe en `Documents`, `AppData` o `Saved Games` como cualquier otro juego, y Steam Cloud las copia desde ahí.

### ¿Por qué no encuentro la carpeta AppData?

Porque está oculta. Escribe `%APPDATA%` (Roaming) o `%LOCALAPPDATA%` (Local) en la barra de direcciones del Explorador o en Ejecutar (Win + R). `LocalLow` está al lado de `Local`.

### ¿Las versiones de Steam, GOG y Epic guardan en el mismo sitio?

Normalmente sí, porque lo decide el juego y no la tienda. Hay excepciones: algunos juegos añaden una carpeta con el ID de tu cuenta, y alguna versión de tienda usa otro nombre de carpeta. Compruébalo antes de copiar partidas de una versión a otra.

### ¿Dónde están las partidas de la app de Xbox y Game Pass?

En `%LOCALAPPDATA%\Packages\<paquete>\SystemAppData\wgs`, en ficheros con nombres aleatorios que solo entiende la app de Xbox. Las sincroniza la nube de Xbox; copiarlas a mano rara vez funciona.

### Mi carpeta Documentos está dentro de OneDrive. ¿Es un problema?

Puede serlo. Los juegos siguen a la carpeta dentro de OneDrive, y OneDrive sincroniza las partidas mientras los juegos las escriben. Mira [OneDrive y las partidas guardadas](/guides/onedrive-game-saves).
