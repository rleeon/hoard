/**
 * Intensidad del relieve (la inclinación 3D de `use:tilt` y su resplandor).
 *
 * Es un porcentaje continuo, no tres botones: 0 apaga el efecto, 100 son los 8°
 * históricos y el defecto,50, es la mitad. Una barra deja elegir el punto
 * exacto en vez de obligar a escoger entre tres saltos.
 *
 * Es cosa puramente de la interfaz, así que vive en `localStorage` y nunca toca
 * el `Prefs` de Rust, mismo criterio que el tema y el tono del acento.
 *
 * **No se consulta `prefers-reduced-motion`.** Se hizo durante un rato, para
 * elegir el valor inicial, y en la práctica dejaba la aplicación sin relieve de
 * fábrica: en WebKitGTK esa media query sale de `gtk-enable-animations`, un
 * ajuste que mucha gente tiene apagado sin pretender "cero efectos al pasar el
 * ratón". La respuesta de accesibilidad es esta barra, que se baja a 0 en un
 * gesto, no un defecto adivinado a partir de una señal que en este entorno no
 * significa lo que parece.
 */
import { writable } from "svelte/store";

const STORAGE_KEY = "hoard-motion";

/** Porcentaje por defecto: la mitad del efecto histórico. */
const DEFAULT_PCT = 50;

/** Valores de la versión anterior, cuando esto eran tres niveles con nombre.
 *  Se traducen al leer para que nadie pierda su elección al actualizar. */
const LEGACY: Record<string, number> = { off: 0, subtle: 50, full: 100 };

function clamp(n: number): number {
  return Math.max(0, Math.min(100, Math.round(n)));
}

function readInitial(): number {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw == null) return DEFAULT_PCT;
    if (raw in LEGACY) return LEGACY[raw];
    const n = Number(raw);
    if (Number.isFinite(n)) return clamp(n);
  } catch {
    /* almacenamiento deshabilitado, al defecto */
  }
  return DEFAULT_PCT;
}

/** Espejo síncrono del store. `tiltScale()` se llama desde una acción del DOM,
 *  no desde un componente, así que no puede suscribirse: lee esta variable, que
 *  la suscripción de abajo mantiene al día. */
let current = readInitial();

export const motionIntensity = writable<number>(current);

/** Factor 0–1 que multiplica los grados base. Lo lee `lib/actions/tilt.ts` en
 *  cada `pointerenter`, así que mover la barra se nota en el siguiente elemento
 *  que apuntes, sin re-montar nada. */
export function tiltScale(): number {
  return current / 100;
}

motionIntensity.subscribe((v) => {
  current = clamp(v);
  paint(current);
});

/**
 * El resplandor que sigue al cursor no lo pone la acción sino `app.css`, y con
 * un valor continuo ya no se puede resolver con reglas por nivel: se escriben
 * las variables directamente sobre `<html>`.
 *
 * `data-motion-on` marca "el usuario quiere movimiento", y lo usa `app.css`
 * para devolverle a `.tilt` su suavizado bajo movimiento reducido del sistema:
 * aplanárselo a quien ha subido la barra a mano no le da menos movimiento,los
 * grados son los que pidió, sólo se lo da a saltos persiguiendo al ratón.
 */
function paint(pct: number): void {
  if (typeof document === "undefined") return;
  const root = document.documentElement;
  const f = pct / 100;
  root.style.setProperty("--tilt-glow-strength", String(0.18 * f));
  root.style.setProperty("--glow-strength", String(f));
  if (pct > 0) root.dataset.motionOn = "";
  else delete root.dataset.motionOn;
}

/** Fija la intensidad y la recuerda para el próximo arranque. */
export function setMotionIntensity(pct: number): void {
  const v = clamp(pct);
  try {
    localStorage.setItem(STORAGE_KEY, String(v));
  } catch {
    /* best-effort */
  }
  motionIntensity.set(v); // dispara `paint`
}

/** Pinta la intensidad guardada al arrancar (lo llama `main.ts`, junto al tema). */
export function initMotion(): void {
  paint(readInitial());
}
