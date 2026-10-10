---
title: "Sincronizar partidas entre Windows y Linux en un PC con arranque dual"
description: "Mismo PC, dos sistemas, dos carpetas de partidas. Sincroniza tus partidas entre Windows y Linux de forma automática, por qué falla compartir NTFS y las trampas."
order: 18
updated: 2026-10-09
---

En un PC con arranque dual, el mismo juego tiene dos partidas separadas: una en tu carpeta de usuario de Windows y otra dentro de un prefijo de Proton en Linux. Steam Cloud las une en los juegos que lo admiten; todo lo demás se desincroniza la primera vez que cambias de sistema. Hoard las mantiene sincronizadas de forma automática. Instálalo en los dos sistemas con la misma cuenta, y la partida de cada juego te sigue arranques el que arranques.

## Por qué un juego tiene dos partidas

El disco es el mismo, pero las carpetas de guardado no:

- **En Windows**, un juego escribe en `Documents`, `Saved Games` o una de las carpetas `AppData` bajo `C:\Users\<tú>`.
- **En Linux**, ese mismo juego de Windows corre con Proton y escribe dentro de su propio prefijo: `~/.local/share/Steam/steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/`, seguido de esa misma ruta de Windows.

Dos copias de una partida, en dos sistemas que nunca están encendidos a la vez. Juegues donde juegues el último, el otro sistema no se entera. Hoard busca en los dos sitios y los empareja por juego, así que la partida de Windows y la de Linux pasan a ser dos versiones de un mismo historial.

## Lo que ya cubre Steam Cloud

En los juegos de Steam con Steam Cloud, Steam sincroniza solo la partida entre tu instalación de Windows y la de Proton. Ahí lo que aporta Hoard es el historial: Steam guarda solo la partida actual, así que una rota sustituye a la buena en los dos sistemas. En los juegos sin Steam Cloud, y en todo lo que está fuera de Steam, Hoard también se encarga de sincronizar.

## ¿Y si comparto una carpeta en el disco de Windows?

Es la primera idea de casi todo el mundo: apuntar Linux a las partidas de la partición de Windows y listo. Suele fallar de tres maneras:

- **Inicio rápido e hibernación.** Cuando Windows se apaga con el inicio rápido activado, deja su partición medio hibernada, y Linux la monta en solo lectura o se niega a montarla. Tu juego no puede escribir la partida.
- **NTFS con Proton.** Usar prefijos de Proton o bibliotecas de Steam desde un disco NTFS es una fuente conocida de problemas de permisos y de nombres de fichero. Los juegos en Linux están más a gusto en un sistema de ficheros de Linux.
- **Los enlaces se sustituyen.** Enlazar la carpeta de partidas de un sistema dentro del otro funciona hasta que un juego, una actualización o una reinstalación cambia el enlace por una carpeta de verdad, sin avisar.

Dejar que cada sistema guarde las partidas donde las espera el juego, y sincronizarlas entre ellos, evita las tres.

## Configurarlo

1. **En Windows**, instala Hoard e inicia sesión.
2. **En Linux**, instala Hoard desde [la página de descarga](/download) e inicia sesión con la misma cuenta.
3. **Abre una vez cada juego de Proton en Linux**, para que exista su prefijo. Antes de eso no hay carpeta donde poner la partida.
4. Revisa la **Biblioteca** en los dos sistemas: deberían aparecer los mismos juegos en cada uno, y Hoard los empareja por juego.

## La trampa que solo tiene el arranque dual

Con dos PC separados, la partida espera en el servidor hasta que la otra máquina la pide. En un PC con arranque dual, la "otra máquina" es el mismo ordenador después de reiniciar, y eso cambia una costumbre.

Hoard sube una partida cuando el juego se ha cerrado y la carpeta se ha quedado quieta. **Si cierras el juego y reinicias en ese mismo momento, puede que la subida aún no se haya hecho**, y el otro sistema arranca sin tu último progreso. Se pondrá al día la próxima vez que vuelvas a arrancar el primero, pero para entonces puede que hayas jugado sobre la partida vieja.

Así que: cierra el juego, dale un momento a Hoard, comprueba en la app que la partida está al día, y después reinicia.

## Versiones nativas de Linux

Algunos juegos tienen versión nativa de Linux además de la de Windows. Las dos no siempre usan el mismo formato de partida, y algunas las guardan en sitios completamente distintos. Si quieres la misma partida en los dos sistemas, lo más seguro es ejecutar también en Linux la versión de Windows con Proton: en Steam, **Propiedades → Compatibilidad** y fuerza una versión de Proton. Así los dos sistemas ejecutan el mismo juego y escriben los mismos ficheros.

## Ajustes y dispositivos

Los ajustes gráficos pueden ser distintos entre los dos sistemas, así que Hoard respalda los ficheros de ajustes como `graphics.ini` pero no los escribe encima de los del otro sistema. Si aun así quieres copiarlos, hay una opción para ello al restaurar.

Cada sistema operativo cuenta como un dispositivo propio, así que un PC con arranque dual usa dos de los tres del plan gratuito. Pro y los servidores autoalojados no tienen límite de dispositivos.

¿Prefieres que las partidas se queden en casa? Levanta `hoard-server` en un NAS o en otra máquina y apunta a él los dos sistemas. Sin cuenta con nosotros, sin telemetría hacia nosotros, sin nada pasando por nuestros servidores. Mira [cómo autoalojar Hoard](/guides/self-host-hoard).

<!-- faq -->

## Preguntas frecuentes

### ¿Steam Cloud sincroniza entre Windows y Linux?

Sí, en los juegos que lo admiten: Steam guarda una copia en la nube por cuenta, juegues en el sistema que juegues. No guarda historial, y no cubre juegos sin Steam Cloud ni nada fuera de Steam.

### ¿Puedo dejar mis partidas en la partición NTFS compartida?

No es recomendable. El inicio rápido puede dejar la partición en solo lectura en Linux, y Proton se lleva mal con NTFS. Es más fiable que cada sistema guarde sus partidas en su sitio y sincronizarlas.

### ¿Por qué no aparecía mi partida después de reiniciar?

Lo más probable es que la subida no hubiera terminado cuando reiniciaste. Vuelve a arrancar el primer sistema, deja que Hoard suba la partida y comprueba la app antes de cambiar otra vez.

### ¿Un PC con arranque dual cuenta como un solo dispositivo?

No, como dos: cada sistema operativo se registra como un dispositivo propio. En el plan gratuito son dos de tres; Pro y los servidores autoalojados no tienen límite.

### ¿Y si un juego tiene versión nativa de Linux?

Sus partidas pueden no coincidir con las de la versión de Windows. Para compartir una partida, ejecuta también en Linux la versión de Windows con Proton.
