---
title: "Cómo autoalojar Hoard con Docker (self-hosted)"
description: "Monta tu propio servidor de Hoard con Docker Compose en minutos. Código abierto, gratis y en tu hardware: una nube totalmente self-hosted para tus partidas guardadas, sin cuenta ni límite de espacio."
order: 0
featured: true
updated: 2026-09-03
---

Hoard es de código abierto y se puede autoalojar. En lugar de usar Hoard Cloud, puedes ejecutar el mismo `hoard-server` en tu propia máquina y apuntar todos tus dispositivos a él: sin cuenta y sin más límite de espacio que el disco que le des. Esta guía deja un servidor funcionando con Docker en pocos minutos.

## Por qué autoalojar Hoard

- **Control total.** Tus partidas viven en hardware que tú controlas, no en la nube de otro.
- **Sin cuota.** El espacio solo lo limita tu propio disco.
- **La misma app, las mismas funciones.** El historial versionado y la sincronización en segundo plano funcionan igual que con Hoard Cloud; solo cambia el backend.
- **Código abierto.** Puedes leer, auditar y modificar el servidor.

Esta es la diferencia clave frente a herramientas como [Ludusavi](/guides/ludusavi-alternative): Ludusavi es excelente para copias locales y para usar tu propia nube vía Rclone, pero la sincronización la montas tú. Hoard te da un servidor de sincronización gestionado que arrancas una vez y al que se conectan todos los dispositivos.

## Qué significa autoalojarse para tus datos

Conviene decirlo sin rodeos, porque es lo que casi todas las comparativas se equivocan sobre Hoard.

**Hoard Cloud** es la opción gestionada: inicias sesión y tus partidas están en nuestros servidores, en la UE.

**Un Hoard autoalojado es tuyo por completo.** Tus dispositivos hablan con tu servidor y con nada más. **No hay cuenta con nosotros, ni telemetría hacia nosotros, ni cupo, ni relé**: no pasa nada por nuestros servidores, porque no hay nada nuestro en el camino. No podemos ver una partida, ni el nombre de un juego, ni un correo, por la sencilla razón de que nada de eso nos llega. Si Hoard Cloud cerrara mañana, tu montaje seguiría funcionando igual.

Y para ser exactos en una cosa: tu servidor sí tiene sus propios accesos — el usuario que crearás más abajo y un token por dispositivo. Son tuyos, en tu máquina, en tu base de datos. Lo que no existe es una cuenta con nosotros.

## Qué necesitas

- Una máquina que esté siempre encendida (un servidor casero, un NAS que ejecute Docker o un VPS pequeño).
- Docker y Docker Compose instalados.
- Opcionalmente un dominio y un proxy inverso para HTTPS (recomendado para cualquier cosa fuera de tu red local).

## Instalación con Docker Compose

Clona el repositorio, crea una configuración a partir del ejemplo y arranca el stack:

```sh
git clone https://github.com/rleeon/hoard.git && cd hoard
mkdir -p deploy/docker/config
cp deploy/config.toml.example deploy/docker/config/config.toml
$EDITOR deploy/docker/config/config.toml      # Use nano or vim or something lol

cd deploy/docker
docker compose up -d
docker compose logs -f                         # wait for "listening"
```

Espera a que los logs muestren que el servidor está escuchando. Los datos se guardan en un volumen de Docker (`hoard-data`); haz copia de seguridad como con cualquier otro volumen. El contenedor escucha internamente en el puerto `12421`; usa otro puerto del host con `HOARD_PORT=9000 docker compose up -d`.

## Crea tu usuario y un token de dispositivo

El servidor no tiene pantalla de registro: los usuarios se crean por línea de comandos:

```sh
docker compose exec server hoard-admin --config /etc/hoard/config.toml \
    user create alice --admin --password 'CHANGE_ME'
docker compose exec server hoard-admin --config /etc/hoard/config.toml \
    token create alice --device 'desktop'
```

El token se muestra una sola vez y **no se puede recuperar después**, así que cópialo ahora.

## Conecta la aplicación de escritorio

Instala la [app de escritorio de Hoard](/download) en cada equipo. En el asistente inicial elige **Self-Host**, y pega la URL de tu servidor y el token que acabas de crear. A partir de ahí se comporta igual que Hoard Cloud: detecta tus juegos, copia las partidas automáticamente y mantiene el historial versionado. Consulta [sincronizar partidas entre varios PC](/guides/sync-game-saves-across-pcs) para el día a día.

## Mantén tu servidor al día

Cómo se actualiza depende de cómo lo instalaste, y equivocarse de comando no da error: simplemente no hace nada. Merece la pena saber cuál es el tuyo.

**Docker Compose.** Baja la imagen nueva y recrea el contenedor. Las dos mitades, en orden:

```sh
docker compose pull
docker compose up -d
```

Si te quedas en la primera, el contenedor viejo sigue corriendo intacto: `/v1/health` sigue informando de la versión antigua y la actualización parece haber fallado en silencio. `git pull` no actualiza ninguna de las dos: lo que corre es la imagen publicada, no tu copia del repositorio. Fija una versión (`ghcr.io/rleeon/hoard:1.1`) en lugar de `:latest` si prefieres elegir tú cuándo llega una nueva.

**Unraid.** Pestaña *Docker* → Hoard → *Apply update* cuando aparezca. No hay nada que teclear.

**Bare metal (systemd).** `sudo hoard-server upgrade` y después `sudo systemctl restart hoard-server`. Cambia el binario de forma atómica y a propósito no reinicia el servicio por su cuenta, para no cortar una sincronización en marcha.

`hoard-server upgrade` es sólo para la instalación bare metal. Dentro de un contenedor se niega a propósito —el cambio de binario no sobreviviría al siguiente `docker compose up -d`— e imprime los dos comandos de arriba; ejecuta `docker compose exec server hoard-server upgrade` si quieres verlo decirlo. Las migraciones de la base de datos las aplica el servidor al arrancar, así que nunca hay un paso aparte para ellas.

## Llevarlo a producción

Para cualquier cosa expuesta fuera de tu red local, termina el TLS en un proxy inverso (Caddy, nginx o Traefik). ¿Prefieres bare metal? El repositorio también incluye un script de instalación con `systemd` y un comando `hoard-server upgrade` que cambia el binario de forma atómica sin cortar una sincronización en curso.

## ¿Self-hosted o Hoard Cloud?

Autoalojar es ideal si ya tienes un servidor y quieres control total sin límites. Si prefieres no mantener infraestructura, [Hoard Cloud](/pricing) te da la misma sincronización gestionada por nosotros, con un plan gratuito para empezar. En cualquier caso, la app y tus partidas siguen siendo portables: puedes cambiar más adelante.

<!-- faq -->

## Preguntas frecuentes

### ¿Un Hoard autoalojado llama a casa?

No. La aplicación de escritorio habla con la dirección de servidor que tú le des. Tus partidas, tus usuarios y tus registros se quedan en tu máquina, y nada de eso nos llega.

### ¿El servidor autoalojado es el mismo código que Hoard Cloud?

Sí, el mismo binario `hoard-server`, bajo AGPL-3.0. No hay una edición comunitaria recortada ni funciones reservadas para la versión alojada.

### ¿Dónde se guardan realmente las partidas?

Por defecto, en el volumen de Docker que le des al contenedor, en tu propio disco. Si ya tienes almacenamiento de objetos, el servidor también habla S3, así que MinIO, Garage o Backblaze B2 sirven como respaldo. En cualquier caso, tus dispositivos sólo hablan con tu servidor.

### ¿Puedo montarlo en un NAS?

Sí, en cualquier NAS que corra Docker. El repositorio incluye una plantilla de Unraid, y la imagen baja al `PUID`/`PGID` que le indiques, así que las carpetas montadas acaban siendo del usuario correcto y no de root.

### ¿Necesito dominio y HTTPS?

En tu propia red local, no. En cuanto el servidor sea accesible desde fuera, pon un proxy inverso delante y termina ahí el TLS: Caddy, nginx o Traefik valen igual.

### ¿Y si mi servidor está caído cuando termino de jugar?

La instantánea se toma en local, así que no se pierde nada. Se sube sola en cuanto el servidor vuelve a responder.

### ¿Puedo empezar en Hoard Cloud y mudarme después?

Sí, y en los dos sentidos. Puedes exportarlo todo desde la página de tu cuenta, y la aplicación se puede apuntar a otro servidor sin reinstalar nada.
