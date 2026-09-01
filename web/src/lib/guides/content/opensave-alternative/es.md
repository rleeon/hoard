---
title: "Alternativa a OpenSave: entre equipos o con un servidor tuyo"
description: "OpenSave sincroniza partidas directamente entre tus PC, sin nada en medio. Hoard sincroniza a través de un servidor —el nuestro o uno tuyo— y guarda historial de versiones. Una mirada honesta a cuándo gana cada diseño."
order: 8
updated: 2026-09-01
---

Las dos herramientas resuelven el mismo problema y discrepan en la arquitectura, que es lo único que merece compararse. Esta página pone los dos diseños uno al lado del otro, incluidos los casos en los que el otro es mejor respuesta.

## La diferencia de verdad: entre equipos o con servidor

**OpenSave** es punto a punto. Tus máquinas hablan entre ellas directamente y no hay nada en medio. No hay cuenta ni almacenamiento que pagar, y opcionalmente puede espejar una copia en una nube que ya tengas.

**Hoard** sincroniza a través de un servidor. Ese servidor es Hoard Cloud, gestionado por nosotros, o `hoard-server` corriendo en tu propio PC o NAS. Tu partida sube cuando dejas de jugar y baja cuando otra máquina la pide.

Todo lo demás sale de esa única decisión.

## Qué te da tener un servidor

- **La otra máquina no tiene que estar encendida.** Terminas en el sobremesa, el portátil sigue cerrado una semana, y la última partida está esperando cuando lo abres. Lo punto a punto necesita los dos extremos despiertos a la vez, que es perfecto en un escritorio e incómodo con una consola de mano que coges dos veces al mes.
- **Un historial de versiones, no sólo el último estado.** Cada sesión es una versión a la que puedes volver. Es la parte que importa el día que un mod se come tu mundo o una partida se escribe a medias: una sincronización directa copia fielmente el fichero roto al otro PC.
- **Una copia que sobrevive al hardware.** Que tus dos PC mueran en el mismo piso no es un escenario exótico. Una partida que sólo existió en esas dos máquinas se muere con ellas.
- **Nada que preparar en la red.** Ningún NAT que atravesar, ningún puerto que abrir, ninguna condición de estar los dos en la misma LAN.

## Qué te da lo punto a punto

Siendo justos con el otro lado:

- **Ningún almacenamiento que pagar, nunca.** No hay cupo que agotar porque no hay depósito. El plan gratuito de Hoard son 2 GB, y por encima pagas o te autoalojas.
- **Nada en medio por diseño.** Si el objetivo es que un fichero no toque nunca el disco de un tercero, la transferencia directa es la respuesta más corta posible.
- **Nada que mantener.** Ningún servidor en pie, ni siquiera el tuyo.

Si juegas en dos sobremesas que están los dos encendidos, nunca quieres volver atrás y prefieres no pensar en almacenamiento, ese diseño encaja limpio y Hoard es más maquinaria de la que necesitas.

## La cuestión de la privacidad, con precisión

Aquí es donde las comparativas de Hoard suelen equivocarse, así que conviene ser exacto. Hay dos formas de usar Hoard, y son genuinamente distintas:

- **Hoard Cloud** es la opción gestionada: inicias sesión y tus partidas se guardan en nuestros servidores, en la UE.
- **Autoalojarse es tuyo por completo.** Levantas `hoard-server` en tu PC o en tu NAS y tus partidas van de tu máquina a tu disco. **No hay cuenta con nosotros, ni telemetría hacia nosotros, ni cupo, ni relé**: no pasa nada por nuestros servidores, porque no hay nada nuestro en el camino. No podemos ver una partida, ni el nombre de un juego, ni un correo, porque nada de eso nos llega. Si Hoard Cloud cerrara mañana, un montaje autoalojado seguiría igual.

O sea que «servidor» no significa «el ordenador de otro» salvo que tú lo elijas. Un Hoard autoalojado mantiene tus partidas en hardware tuyo, exactamente igual que una transferencia directa, y encima te da el historial y el caso de la máquina apagada.

## Detección y cobertura

Las dos herramientas encuentran partidas de un catálogo grande de forma automática. Hoard lee el mismo manifiesto comunitario de ubicaciones que comparte el ecosistema open source, con más de 20.000 títulos, y le suma el barrido de bibliotecas de Steam, los procesos en ejecución y un escaneo del disco. En los juegos de Steam rastrea `<AppID>/remote/` dentro de `userdata` y no la carpeta de encima, porque la padre guarda `remotecache.vdf` y ficheros de logros y tiempo jugado propios de cada máquina: si sincronizas eso, cada arranque parece un conflicto. Lo raro se lo señalas a mano.

## ¿Cuál deberías usar?

- **Punto a punto** si tus máquinas están encendidas a la vez, no quieres almacenamiento en la ecuación y la última partida es todo lo que has necesitado nunca.
- **Hoard** si quieres un historial al que volver, una máquina que pueda estar apagada una semana y una copia que sobreviva a los dos PC, con la opción de usar nuestra nube o tu propio servidor.

Hay una [comparativa de todas las herramientas de sincronización](/guides/game-save-sync-comparison) si quieres el panorama completo, y una [comparativa con Ludusavi](/guides/ludusavi-alternative) para la parte de copias locales.

<!-- faq -->

## Preguntas frecuentes

### ¿Hoard necesita cuenta?

Para Hoard Cloud sí, porque es a lo que está atada la sincronización. Autoalojado no hay ninguna cuenta con nosotros: tu servidor tiene sus propios usuarios y un token por dispositivo, y no salen de tu máquina.

### ¿Puede funcionar Hoard sin ninguna nube?

Sí. Levanta `hoard-server` en un PC o en un NAS y tus partidas van de tu máquina a tu disco, sin que nada pase por nuestros servidores.

### ¿Tienen que estar los dos PC encendidos a la vez?

No, y ésa es la ventaja práctica de sincronizar a través de un servidor. Tu partida sube cuando dejas de jugar y baja cuando la otra máquina la pida.

### ¿Una transferencia directa guarda historial de versiones?

No de por sí: copiar un fichero a otra máquina te deja el estado actual en las dos. Hoard captura cada sesión como una versión, y eso es lo que hace posible volver atrás desde una partida corrupta.

### ¿Hoard también es open source?

Sí, AGPL-3.0, servidor incluido. El servidor autoalojado es el mismo binario que usamos nosotros, no una edición recortada.
