/**
 * 3D tilt action, the element leans toward the cursor on hover and a soft
 * directional glow follows the pointer. Applied to cards, panels, and covers
 * so the UI feels physical rather than flat.
 *
 * The transform + glow are driven by CSS variables this action sets
 * (--tilt-rx/--tilt-ry and --tilt-glow-x/--tilt-glow-y); the `.tilt` class in
 * app.css consumes them, so the action itself stays transform-free (cheap to
 * attach/teardown). The CSS's global reduced-motion rule flattens transitions
 * automatically when the OS asks for it, we don't skip the listeners here
 * because the glow (opacity-only) is still useful even without the 3D tilt.
 *
 * Anti-flicker: the CSS transform skews the bounding box so `mouseleave`
 * fires when the cursor is still visually on the card. We listen for
 * `pointermove` on the document and only reset when the pointer truly
 * exits the element's original (un-transformed) rect.
 *
 * Un listener compartido, pero varios nodos inclinándose a la vez
 * ---------------------------------------------------------------
 * Estas inclinaciones **se anidan**: una tarjeta de partida lleva `use:tilt` y
 * dentro la carátula lleva el suyo, así que apuntando a la portada se mueven
 * las dos, la tarjeta entera y la foto encima. Eso es el efecto.
 *
 * La versión original lo conseguía enganchando `pointermove` y `pointerup` al
 * documento *dentro de la acción*: cada elemento llevaba su propio par y su
 * propio estado. Funcionaba, pero en la biblioteca son dos listeners por juego
 * y todos comprueban límites en cada píxel que se mueve el ratón, descartando
 * el evento todos menos uno.
 *
 * Reducirlo a "un único nodo activo global" NO vale, y es un error fácil de
 * cometer: al entrar en la carátula se suelta la tarjeta y, como el puntero
 * nunca llega a *salir* de ella, su `pointerenter` no vuelve a dispararse y se
 * queda muerta el resto del recorrido. En el Panel, donde la carátula ocupa
 * casi toda la tarjeta, el efecto desaparece de hecho.
 *
 * Así que el estado es un **conjunto**: todos los nodos en los que ha entrado
 * el puntero y de los que aún no ha salido. Como sólo pueden estar los que se
 * apilan bajo el cursor, el conjunto tiene el tamaño del anidamiento (dos, en
 * la práctica) y no el del número de tarjetas, que era el punto. Los
 * listeners de documento siguen siendo uno, compartido por todas las
 * instancias, instalado con la primera y retirado con la última.
 */

import { tiltScale } from "../stores/motion";

/**
 * Grados máximos de inclinación en cada eje, a plena intensidad.
 *
 * El recorte no vive aquí sino en el nivel de Ajustes (`stores/motion.ts`),
 * que multiplica esta base: `sutil`,el defecto, la deja en 4°, la mitad del
 * efecto histórico. Así el usuario puede recuperar los 8° sin recompilar.
 *
 * Ojo con leer este número como "lo que se mueve la tarjeta": las
 * inclinaciones anidadas se suman en pantalla, así que la carátula dentro de
 * una tarjeta llega al doble. Escalar la base las reduce a las dos en la misma
 * proporción, que es lo que se busca.
 */
const BASE_MAX = 8;

type Armed = {
  /** Rect capturado ANTES de que el transform lo deforme. */
  rect: DOMRect;
  /** Grados máximos de ESTE nodo (una carátula puede pedir otros que la
   *  tarjeta que la contiene). */
  max: number;
};

/** Nodos bajo el cursor ahora mismo. Es una cadena de anidamiento, no una
 *  lista de todo lo que hay en pantalla. */
const armed = new Map<HTMLElement, Armed>();
/** Instancias vivas, al llegar a 0 se retiran los listeners compartidos. */
let liveCount = 0;
let raf = 0;
/** Último evento pendiente de pintar. Un solo rAF reparte a todo el conjunto,
 *  así que apuntar a una tarjeta con carátula sigue costando un fotograma. */
let pending: PointerEvent | null = null;

function schedule(e: PointerEvent): void {
  pending = e;
  if (raf) return;
  raf = requestAnimationFrame(() => {
    raf = 0;
    const ev = pending;
    pending = null;
    if (!ev) return;
    for (const [node, { rect, max }] of armed) {
      const px = (ev.clientX - rect.left) / rect.width; // 0..1
      const py = (ev.clientY - rect.top) / rect.height;
      node.style.setProperty("--tilt-ry", `${(px - 0.5) * 2 * max}deg`);
      node.style.setProperty("--tilt-rx", `${(0.5 - py) * 2 * max}deg`);
      node.style.setProperty("--tilt-glow-x", `${(px * 100).toFixed(1)}%`);
      node.style.setProperty("--tilt-glow-y", `${(py * 100).toFixed(1)}%`);
    }
  });
}

function release(node: HTMLElement): void {
  if (!armed.delete(node)) return;
  node.style.setProperty("--tilt-ry", "0deg");
  node.style.setProperty("--tilt-rx", "0deg");
}

function onDocMove(e: PointerEvent): void {
  if (armed.size === 0) return;
  // Check against the ORIGINAL rects, ignore the transformed bboxes.
  for (const [node, { rect }] of armed) {
    const inBounds =
      e.clientX >= rect.left &&
      e.clientX <= rect.right &&
      e.clientY >= rect.top &&
      e.clientY <= rect.bottom;
    if (!inBounds) release(node);
  }
  if (armed.size > 0) schedule(e);
}

function onDocUp(): void {
  // After a click, re-measure so stale bounds from before a layout shift
  // don't linger.
  for (const [node, state] of armed) {
    state.rect = node.getBoundingClientRect();
  }
}

export function tilt(node: HTMLElement, opts: { max?: number } = {}) {
  const base = opts.max ?? BASE_MAX;

  function onEnter(e: PointerEvent) {
    // El nivel se consulta al entrar, no al montar: cambiarlo en Ajustes surte
    // efecto en el siguiente elemento que apuntes, sin re-montar la pantalla.
    // Aquí NO se mira `prefers-reduced-motion`, eso lo decide el valor inicial
    // del nivel (ver `stores/motion.ts`), para que una elección explícita del
    // usuario siempre gane.
    const max = base * tiltScale();
    armed.set(node, { rect: node.getBoundingClientRect(), max });
    schedule(e);
  }

  node.addEventListener("pointerenter", onEnter);
  if (liveCount === 0) {
    document.addEventListener("pointermove", onDocMove);
    document.addEventListener("pointerup", onDocUp);
  }
  liveCount += 1;

  return {
    destroy() {
      node.removeEventListener("pointerenter", onEnter);
      armed.delete(node);
      liveCount -= 1;
      if (liveCount === 0) {
        document.removeEventListener("pointermove", onDocMove);
        document.removeEventListener("pointerup", onDocUp);
        cancelAnimationFrame(raf);
        raf = 0;
        pending = null;
      }
    },
  };
}
