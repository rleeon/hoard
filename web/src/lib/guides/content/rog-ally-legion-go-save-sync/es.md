---
title: "Sincronizar partidas entre ROG Ally, Legion Go, MSI Claw y tu PC"
description: "Las portátiles con Windows como ROG Ally, Legion Go y MSI Claw son PC. Sincroniza sus partidas con tu sobremesa de forma automática, con historial de versiones."
order: 19
updated: 2026-10-09
---

La ROG Ally, la Legion Go y la MSI Claw funcionan con Windows, así que para un juego son un PC más. Y ese es justo el problema: tu sobremesa y tu portátil guardan cada uno sus propias partidas. Steam Cloud cubre parte de tu biblioteca y las partidas en la nube de Xbox cubren Game Pass, pero todo lo demás se queda en la máquina donde jugaste. Hoard mantiene tus partidas sincronizadas entre la portátil y el sobremesa de forma automática: dejas de jugar en una y el juego te espera en la otra, con todas las versiones anteriores guardadas.

## Lo que ya te sigue

- **Los juegos de Steam con Steam Cloud** se sincronizan solos.
- **Los juegos de Game Pass y de la app de Xbox** usan la nube de Xbox, siempre que juegues la versión de Xbox en las dos máquinas.
- **Epic, GOG, Ubisoft y EA** tienen partidas en la nube para algunos de sus juegos, dentro de sus propios launchers. Mira [las partidas en la nube de Epic y GOG](/guides/epic-gog-cloud-saves).

Lo que queda fuera: juegos cuyo desarrollador nunca activó la nube, emuladores, juegos que instalaste a mano y cualquier juego en el que tu sobremesa y tu portátil no usan el mismo launcher.

## Configurarlo

1. **En la portátil**, cambia al escritorio de Windows, abre [la página de descarga](/download) e instala Hoard para Windows.
2. **Inicia sesión** con la cuenta que usas en el sobremesa, o apunta la app a tu propio servidor.
3. Abre la **Biblioteca** y revisa lo que ha encontrado Hoard. Añade lo que falte señalando su carpeta, por ejemplo un emulador.
4. **En el sobremesa**, instala Hoard con la misma cuenta. Los mismos juegos se emparejan solos.

El motor de sincronización es un servicio en segundo plano que arranca con Windows, así que sigue funcionando mientras estás en Armoury Crate, Legion Space, MSI Center M o el modo Big Picture de Steam. No hace falta abrir la ventana de Hoard para jugar.

## Trampas de las portátiles

### Suspender no es cerrar

En una portátil es muy fácil pulsar el botón de encendido y guardarla con el juego abierto. Hoard solo respalda cuando el juego se ha cerrado, porque un juego en marcha puede estar a medio escribir la partida, y nunca cambia la partida de un juego abierto. Si suspendes la portátil y luego juegas en el sobremesa, el progreso de la portátil todavía no se ha subido. **Cierra el juego antes de cambiar de máquina.**

### Juegos en la microSD

En una portátil es normal instalar los juegos en la tarjeta, y casi nunca afecta a las partidas: la mayoría de juegos guarda en tu carpeta de usuario del disco interno, estén instalados donde estén. La excepción son los juegos que guardan junto a su carpeta de instalación; si alguno no aparece, añade su carpeta a mano.

### Pantalla y ajustes

Tu portátil funciona a menos resolución y con una GPU más pequeña que tu sobremesa. Hoard respalda los ficheros de ajustes como `graphics.ini` junto con la partida, pero no los escribe encima de los de la otra máquina, así que cada una conserva los ajustes que le van bien. Si aun así quieres copiarlos, hay una opción para ello al restaurar.

### Mismo juego, distinta tienda

Un juego comprado en Steam para el sobremesa y jugado con Game Pass en la portátil son dos instalaciones distintas, y la versión de Xbox guarda sus partidas en un formato que solo entiende la app de Xbox. Para compartir una partida, juega la versión de la misma tienda en las dos.

### ¿Con SteamOS o Bazzite en vez de Windows?

Entonces tu portátil es una máquina Linux y las partidas viven dentro de prefijos de Proton, igual que en una Steam Deck. Mira [sincronizar partidas entre Steam Deck y PC](/guides/sync-saves-steam-deck-pc).

## Sin nuestros servidores

Si prefieres que las partidas se queden en casa, levanta `hoard-server` en tu PC o en un NAS y apunta a él las dos máquinas. Sin cuenta con nosotros, sin telemetría hacia nosotros, sin nada pasando por nuestros servidores. Mira [cómo autoalojar Hoard](/guides/self-host-hoard).

<!-- faq -->

## Preguntas frecuentes

### ¿La ROG Ally tiene partidas en la nube?

Tiene lo que tenga cada launcher: Steam Cloud, la nube de Xbox, y las de Epic, GOG, Ubisoft o EA en los juegos que la admiten. No hay una sincronización de partidas para todo el sistema. Hoard añade una para los juegos que esos dejan fuera.

### ¿Hoard funciona con Armoury Crate o Legion Space?

Sí. El motor de sincronización de Hoard es un servicio en segundo plano de Windows, independiente del launcher con el que abras los juegos.

### ¿La portátil cuenta como dispositivo?

Sí. El plan gratuito incluye tres dispositivos, así que caben un sobremesa, un portátil y una consola portátil. Pro y los servidores autoalojados no tienen límite de dispositivos.

### ¿Y las partidas de Game Pass?

Déjaselas a la nube de Xbox, que las sincroniza entre las instalaciones de la app de Xbox de las dos máquinas. Hoard cubre los juegos que no tienen nube propia.

### ¿Puedo sincronizar la portátil también con una Steam Deck?

Sí. Hoard funciona en las dos y empareja cada juego entre Windows y los prefijos de Proton del Deck.
