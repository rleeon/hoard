/**
 * Mandos de depuración del HUD. **TEMPORAL.**
 *
 * Existen para afinar el aspecto en vivo,opacidad del fondo, del texto,
 * tamaño de letra, y decidir los valores definitivos mirando el HUD sobre un
 * juego de verdad, que es la única forma de acertar con una superficie
 * translúcida. Cuando los números estén elegidos, esto se borra entero: el
 * store, `OverlayDebug.svelte` y su render en `Overlay.svelte`.
 *
 * Por eso vive en su propio directorio y no toca `stores/`: es un bloque que se
 * quita de una pieza, no algo que haya que ir desenredando.
 *
 * Se persiste en `localStorage` para que sobreviva a cerrar y reabrir el HUD
 * mientras se prueba; el HUD es su propia ventana y su propio contexto JS.
 */

/**
 * El interruptor. **Apagado**: los mandos no se pintan y el HUD usa los valores
 * de abajo tal cual.
 *
 * Está aquí, y en un `const` con nombre, para que volver a sacarlos sea cambiar
 * un `false` por un `true` y no reconstruir de memoria qué import y qué línea de
 * marcado hacían falta. El andamio se queda en el árbol a propósito: afinar una
 * superficie translúcida sobre un juego real hay que rehacerlo cada vez que
 * cambian los colores, y borrarlo obliga a reescribirlo entero.
 */
export const OVERLAY_DEBUG_PANEL = false;

const KEY = "hoard-overlay-debug";

export type OverlayDebug = {
  /** Opacidad del fondo del HUD (0 = solo se ve el juego). */
  bgOpacity: number;
  /** Opacidad de TODO el contenido (texto, logo, botón). */
  textOpacity: number;
  /** Tamaño base de la letra, en px. */
  fontSize: number;
  /** Ocultar el propio panel de depuración sin cerrar el HUD. */
  panelOpen: boolean;
};

const DEFAULTS: OverlayDebug = {
  bgOpacity: 0.72,
  textOpacity: 1,
  fontSize: 14,
  panelOpen: true,
};

function read(): OverlayDebug {
  try {
    const raw = localStorage.getItem(KEY);
    if (raw) return { ...DEFAULTS, ...(JSON.parse(raw) as Partial<OverlayDebug>) };
  } catch {
    /* almacenamiento caído o JSON corrupto, a los valores por defecto */
  }
  return { ...DEFAULTS };
}

/** Estado reactivo con runas: lo lee el HUD y lo escriben los deslizadores. */
export const overlayDebug = $state<OverlayDebug>(read());

/** Guarda el estado actual. Lo llaman los controles tras cada cambio. */
export function saveOverlayDebug(): void {
  try {
    localStorage.setItem(KEY, JSON.stringify(overlayDebug));
  } catch {
    /* best-effort: perder la preferencia de depuración no importa */
  }
}

/** Vuelve a los valores de partida. */
export function resetOverlayDebug(): void {
  Object.assign(overlayDebug, DEFAULTS);
  saveOverlayDebug();
}
