# 0017 — Actualización remota del server self-hosted disparada desde la app

- **Status**: Accepted
- **Date**: 2026-05-28
- **Context**: 1.8.x (self-hosted ops)
- **Supersedes**: amplía el flujo de upgrade del server descrito en ADR
  [0008](0008-in-app-updater.md) (in-app updater) — no lo reemplaza para
  el cliente desktop.

## Contexto

Hoy el desktop sabe detectar que el server self-hosted va por detrás
(`probe_server` contra `/v1/health`) y, en Linux con el server en la
misma máquina, dispara `pkexec hoard-server upgrade`. En cualquier otro
caso (server en otra caja, desktop en Windows/macOS) la UI solo sabe
**copiar el comando** `sudo hoard-server upgrade` al portapapeles, que es
busywork: obliga al operador a entrar al servidor físico.

El objetivo es: desde la app, en cualquier SO, pulsar un botón y que el
server Linux self-hosted se actualice y reinicie él solo, esté donde esté.

### Restricción de partida: el blindaje de la unidad systemd

`deploy/systemd/hoard-server.service` corre como usuario `hoard` con
`ProtectSystem=strict` (toda `/usr/local/bin` es read-only para el
proceso), `NoNewPrivileges=true`, `MemoryDenyWriteExecute=true`,
filtro de syscalls `@system-service` y `ReadWritePaths=/var/lib/hoard`.
Es decir, **el proceso de red no puede sobrescribir su binario ni
reiniciar su unidad, y eso es deseable**: dar a un servicio expuesto a
internet permiso de escritura sobre su propio ejecutable sería un
primitivo de persistencia para un atacante que logre RCE. No se relaja.

## Decisiones

### D1 — El proceso de red nunca ejecuta la parte privilegiada

El handler HTTP solo **escribe un marcador** en su directorio escribible
(`{data_dir}/.upgrade-requested`). No descarga, no hace swap, no reinicia.
Toda acción privilegiada vive en un oneshot root separado, fuera de la
sandbox. Separación de privilegios: la superficie de red queda sin
capacidad de escalar.

### D2 — `POST /v1/admin/upgrade`, solo self-hosted, gate admin

- Montado en `run_self_hosted` (`main.rs`), **nunca** en `cloud/run.rs`.
  Por construcción no existe en la instancia gestionada de Fly.io.
- Pasa por el middleware `require_auth` existente y exige
  `AuthUser.is_admin == true`; si no, `403`.
- Cuerpo vacío. El handler **ignora deliberadamente** cualquier versión
  o URL que pudiera venir en la petición (ver D5). Escribe el marcador y
  responde `202 Accepted` con `{ "status": "scheduled" }`.
- Gateable además por config `server.allow_remote_upgrade` (default
  `true` en self-hosted). Un operador paranoico puede ponerlo a `false`.

### D3 — Oneshot root + path unit vigilando el marcador

Dos units nuevas, instaladas por `install.sh` solo en el deploy systemd:

- `hoard-upgrade.path`: `PathExists=/var/lib/hoard/.upgrade-requested`,
  dispara `hoard-upgrade.service`.
- `hoard-upgrade.service`: `Type=oneshot`, `User=root`. Ejecuta un script
  que: (1) borra el marcador primero —para que un fallo no haga loop—,
  (2) corre `hoard-server upgrade` (root sí puede escribir
  `/usr/local/bin`), (3) si el swap fue OK, `systemctl restart
  hoard-server`. El oneshot está fuera de la sandbox del servicio web.

El binario de `hoard-server upgrade` ya existe (`upgrade.rs`); se le
añade verificación de firma (D5).

### D4 — El restart NO depende de `Restart=` del servicio web

El reinicio lo hace el oneshot con `systemctl restart hoard-server`, no
un `exit()` del proceso confiando en `Restart=always`. Así no tocamos la
política `Restart=on-failure` de la unidad principal y el reinicio es
explícito y observable en los logs de `hoard-upgrade.service`.

### D5 — Firma de releases (minisign) verificada antes del swap

Razón: el oneshot corre como root un binario bajado de GitHub. Si la
cuenta de GitHub se compromete y suben un asset malicioso, sin firma se
ejecutaría como root en todas las cajas self-hosted.

- Se genera un par minisign **una sola vez**. La clave privada vive solo
  en los secrets de GitHub Actions (`MINISIGN_SECRET_KEY`,
  `MINISIGN_KEY_PASSWORD`); nunca en el repo ni en una caja de usuario.
- El workflow de release firma cada `hoard-*-linux-x86_64.tar.gz` y sube
  el `.minisig` junto al asset.
- La **clave pública va embebida en el binario** (`const MINISIGN_PUBKEY`)
  y se versiona en el repo (es pública por definición).
- `hoard-server upgrade` descarga asset + `.minisig`, verifica la firma
  contra la pubkey embebida y **aborta el swap si no valida**. Crate:
  `minisign-verify` (verificación pura, sin libsodium).
- Defensa en profundidad con D2: aunque alguien burle la auth del
  endpoint, el oneshot solo instala la última release **firmada** del
  repo canónico; no puede inyectar código del atacante. Lo peor que se
  consigue es forzar un cambio de versión válida + reinicio (DoS leve).

### D6 — Desktop: un command que hace POST, y polling de confirmación

- Nuevo command Tauri `trigger_server_upgrade()` que hace
  `POST {server_url}/v1/admin/upgrade` con el token vía el `ApiClient`
  existente. Reemplaza la rama `pkexec` local para el caso self-hosted;
  funciona desde Windows/macOS/Linux y con server local o remoto.
- Tras el `202`, el desktop hace polling a `/v1/health` (con tolerancia a
  fallos durante el reinicio) hasta que `version` cambie o expire un
  timeout (~90 s), y refresca el panel.
- El botón se gatea por `is_admin` (de `whoami`), no por la clasificación
  de URL `is_local_server`. Se elimina `canInAppUpgrade` basado en URL.

## Consecuencias

- Nuevo endpoint privilegiado-por-efecto, mitigado por: solo self-hosted,
  solo admin, marcador en dir sandboxed, oneshot que solo instala
  releases firmadas del repo canónico.
- `install.sh`/`uninstall.sh` instalan/retiran dos units nuevas.
- Operadores que actualicen a mano (`sudo hoard-server upgrade`) siguen
  igual; el marcador es un camino adicional, no el único.
- Deploy Docker queda fuera de alcance: en contenedor la actualización es
  `pull` de imagen nueva, no swap de binario. El endpoint puede existir
  pero el oneshot no aplica; se documenta como no soportado por ahora.
- La firma protege a los self-hosters de un compromiso del upstream. La
  clave privada en GitHub Secrets es ahora un activo crítico del proyecto.
