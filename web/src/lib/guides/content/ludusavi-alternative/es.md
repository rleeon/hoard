---
title: "Alternativa a Ludusavi: sincronización automática de partidas en la nube"
description: "Comparativa justa entre Ludusavi y Hoard. Ludusavi es una gran herramienta open source de copia local; Hoard añade sincronización gestionada en la nube e historial versionado entre todos tus PC, usando los mismos datos de ubicación de partidas."
order: 5
updated: 2026-09-01
---

Si buscas una forma de hacer copia y sincronizar tus partidas guardadas, seguramente has encontrado **Ludusavi**, y es excelente. Esta guía es una comparativa honesta para que elijas la herramienta adecuada, y explica dónde encaja Hoard si quieres sincronización automática en la nube entre equipos.

## Qué hace bien Ludusavi

Ludusavi es una herramienta gratuita y open source (creada por mtkennerly) para hacer copias y restaurar partidas de PC en Windows, macOS y Linux. Tiene una interfaz limpia y una CLI, detecta automáticamente las partidas de miles de juegos, guarda copias locales versionadas y puede subir esas copias a una nube tuya configurando **Rclone** (Google Drive, Dropbox y muchas más). Si quieres control total y un montaje a tu medida, Ludusavi es una opción fantástica, y es completamente gratis.

Hoard no viene a reemplazar eso. De hecho, **Hoard usa la misma base de datos comunitaria de ubicación de partidas en la que se apoya Ludusavi** para localizar dónde guarda cada juego sus saves, así que la calidad de detección está a la par.

## En qué se diferencia Hoard

El punto donde la mayoría se atasca con cualquier herramienta local es **sincronizar entre dispositivos**. Con Ludusavi lo haces tú: programas una copia, configuras un remoto de Rclone y luego restauras en el otro PC antes de jugar. Funciona, pero es manual.

Hoard convierte eso en **sincronización gestionada en la nube**:

- **Inicia sesión y listo.** Sin remotos de Rclone, sin scripts. Hoard sube tu partida cuando terminas de jugar y descarga la última antes de empezar, en todos los PC de tu cuenta.
- **Historial versionado en la nube.** Se conserva cada copia, así que puedes volver a cualquier partida anterior, incluso tras un fallo de disco o una instalación limpia.
- **Tiene en cuenta los conflictos.** Hoard compara fechas y guarda una copia local de lo que reemplaza, así que una sincronización nunca destruye progreso en silencio.
- **Sigue siendo open source y autoalojable.** Como Ludusavi, no hay bloqueo: usa Hoard Cloud o aloja el servidor tú mismo.

## Cara a cara

| | Ludusavi | Hoard |
|---|---|---|
| Copias locales | Sí | Sí |
| Detección de partidas | Manifiesto comunitario | El mismo manifiesto, más bibliotecas de Steam, procesos en ejecución y un barrido del disco |
| Almacenamiento en la nube | El tuyo, vía Rclone | Incluido, o tu propio servidor |
| Sincronización entre PC | Manual: copia aquí, restaura allí | Automática, al dejar de jugar y antes de empezar |
| Historial de versiones | Copias locales que podas tú | Todas las versiones en la nube, deduplicadas por hash de contenido |
| Emuladores | Sí | Sí |
| Interfaces | App de escritorio y CLI | App de escritorio, CLI y overlay dentro del juego |
| Precio | Gratis | Plan gratis de 2 GB y 3 dispositivos, Pro por encima, sin cupo si te autoalojas |
| Licencia | MIT | AGPL-3.0 |

## Cuándo Ludusavi es la mejor opción

Ésta es la parte que casi ninguna comparativa incluye. Ludusavi es mejor herramienta cuando:

- **Sólo juegas en un PC.** La sincronización en la nube resuelve un problema que no tienes. Con una copia local basta, y Ludusavi hace copias locales muy bien.
- **Ya tienes un remoto de Rclone que funciona.** Si tu almacenamiento está montado y va fino, la ventaja principal de Hoard es un paso de configuración que tú ya has pagado.
- **Quieres usarlo desde el modo Juego de una Steam Deck.** Ludusavi tiene un plugin de Decky, así que puedes lanzar una copia sin salir de la interfaz de consola.
- **Quieres una licencia permisiva.** Ludusavi es MIT y Hoard es AGPL-3.0. Si piensas construir algo encima y no publicar el resultado, esa diferencia importa.
- **No quieres nada corriendo de fondo.** Autoalojar Hoard implica mantener un servidor en pie, aunque sea en el mismo PC. Ludusavi es una aplicación que abres cuando te hace falta.

## Pasar de Ludusavi a Hoard

No hay importador, y es a propósito. Los pasos:

1. **Deja tus copias de Ludusavi exactamente donde están.** No se migra ni se borra nada. Consérvalas como red de seguridad las primeras semanas.
2. **Instala Hoard e inicia sesión**, o apúntalo a tu propio servidor.
3. **Déjalo escanear.** Lee el mismo manifiesto, así que la lista de juegos detectados debería resultarte familiar.
4. **No apuntes Hoard a la carpeta de copias de Ludusavi.** Rastrea la carpeta en la que escribe el juego. Una carpeta de copias es un duplicado que cambia por horario y no cuando juegas, y sincronizar la copia de una copia es como acabas restaurando el progreso de ayer. Hoard intenta detectarlo solo — `hoard doctor` avisa de una carpeta rastreada que parece un espejo de copias — pero es más fácil no rastrearla nunca.
5. **Juega una vez.** Al salir, la primera versión aparece en el historial.
6. **Repite en el segundo PC.** Inicias sesión y las versiones ya están ahí.

## Dos detalles que conviene saber

**Las partidas de Steam viven una carpeta más adentro de lo que parece.** En los juegos de Steam, Hoard rastrea `<AppID>/remote/` dentro de `userdata`, no la carpeta de encima. La carpeta padre guarda además `remotecache.vdf` y ficheros de logros y de tiempo jugado, y ésos son legítimamente distintos en cada máquina. Si sincronizas la padre, cada arranque parece un conflicto aunque no se haya movido ninguna partida. Es el motivo más común de que un montaje casero entre Steam Deck y sobremesa acabe peleándose consigo mismo.

**Las versiones salen baratas.** Las instantáneas se guardan por hash de contenido, así que un fichero que no cambia se almacena una sola vez. Diez versiones de una partida de 2 GB ocupan unos 2 GB, no 20, y eso es lo que hace práctico conservar el historial entero en vez de ir podándolo.

## Qué significa realmente autoalojarse

Es el punto donde casi todas las comparativas se equivocan con Hoard, así que conviene ser exacto. Hay dos formas de usarlo, y son genuinamente distintas:

- **Hoard Cloud** es la opción gestionada: inicias sesión y tus partidas se guardan en nuestros servidores, en la UE.
- **Autoalojarse es tuyo por completo.** Levantas `hoard-server` en tu PC o en tu NAS, y tus partidas van de tu máquina a tu disco. **No hay cuenta con nosotros, ni telemetría hacia nosotros, ni cupo, ni relé**: no pasa nada por nuestros servidores, porque no hay nada nuestro en el camino. No podemos ver una partida, ni el nombre de un juego, ni un correo, por la sencilla razón de que nada de eso nos llega. Si Hoard Cloud desapareciera mañana, un montaje autoalojado seguiría funcionando igual.

El mismo programa, la misma detección, el mismo historial de versiones. Lo único que cambia es de quién es el almacenamiento.

## ¿Cuál elegir?

- Elige **Ludusavi** si quieres una herramienta de copia gratuita y local y no te importa montar tu propia nube con Rclone.
- Elige **Hoard** si quieres que la copia *y* la sincronización entre PC funcionen solas, con historial versionado en la nube, sin renunciar a poder autoalojarte.

Mucha gente empieza con Ludusavi para copias locales y pasa a Hoard cuando juega a los mismos juegos en más de un equipo. Si es tu caso, mira [cómo sincronizar partidas entre PC](/guides/sync-game-saves-across-pcs) o simplemente [descarga Hoard](/download) e inicia sesión. Y si quieres el panorama completo, hay una [comparativa de todas las herramientas de sincronización](/guides/game-save-sync-comparison).

<!-- faq -->

## Preguntas frecuentes

### ¿Puedo usar Ludusavi y Hoard a la vez?

Sí. Leen las mismas ubicaciones de partidas y ninguno de los dos bloquea los ficheros. Mucha gente conserva Ludusavi para copias de archivo locales y deja que Hoard se encargue de la sincronización entre equipos. La única regla es no apuntar una herramienta a la carpeta de copias de la otra.

### ¿Hoard importa mis copias de Ludusavi?

No, y es deliberado. Una carpeta de copias es un duplicado que cambia según su propio horario, así que rastrearla sincronizaría un espejo desfasado en lugar de tu partida real. Hoard rastrea la carpeta en la que escribe el juego y arranca su propio historial desde tu siguiente sesión. Guarda el archivo de Ludusavi como red de seguridad.

### ¿Hoard es gratis?

Hoard Cloud tiene un plan gratuito con 2 GB de almacenamiento y 3 dispositivos, que cubre la mayoría de colecciones de partidas; Pro sube ambos. Autoalojar el servidor es gratis y no tiene cupo ninguno. Todo es open source bajo AGPL-3.0.

### ¿Funciona en Steam Deck?

Sí, en Steam Deck y en cualquier escritorio Linux, además de Windows y macOS. La Deck es justo el caso que necesita el detalle de `remote/` de más arriba, porque una Deck y un sobremesa escriben ficheros de logros y de tiempo jugado distintos junto a la misma partida.

### ¿Necesito Rclone o una cuenta de nube propia?

No. Ésa es la diferencia práctica principal: con Hoard Cloud el almacenamiento ya está listo al iniciar sesión. Si prefieres ser dueño del almacenamiento, levanta el servidor tú mismo contra un bucket compatible con S3 o una carpeta normal de tu máquina.

### ¿Autoalojarse envía algo a Hoard?

No. En modo autoalojado no hay cuenta con nosotros ni telemetría hacia nosotros: tus partidas, tus usuarios y tus registros viven en tu propio servidor y nunca tocan el nuestro. Ése es el sentido del modo, y por eso el servidor es el mismo binario open source que usamos nosotros y no una versión recortada.
