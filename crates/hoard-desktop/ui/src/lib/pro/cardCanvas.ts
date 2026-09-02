/**
 * Dibujo de la tarjeta compartible de Hoard-Wrapped.
 *
 * La tarjeta se pinta en un `<canvas>` y ese mismo canvas es a la vez lo que
 * se ve en pantalla y lo que se guarda en la galería: un solo layout, cero
 * divergencia entre vista previa y foto. La alternativa,maquetar en HTML y
 * volver a dibujarlo aquí para exportar, son dos diseños que se van
 * separando en cuanto alguien toca uno.
 *
 * Todo se dibuja en un espacio lógico de 1200×675 (16:9, la proporción que
 * previsualizan las redes) y se escala con `ctx.scale`, así el mismo código
 * sirve para la vista (1×) y para el PNG que se guarda (2×).
 *
 * Ojo con las imágenes: el `toDataURL` del export falla si el canvas queda
 * "tainted", y eso pasa en cuanto dibujas una imagen remota sin CORS. Por eso
 * la foto local (bytes que nos da Rust, blob propio) y las carátulas (idem)
 * son seguras, mientras que el avatar de la cuenta Cloud es una URL de Google
 * que puede o no responder con CORS. `renderToPng` cuenta con ello: si el
 * export peta, repite el dibujo sin el avatar remoto.
 */

/** Un cubo de actividad: un día, un mes… lo que toque según el rango. */
export type Cube = {
  /** Segundos jugados en ese tramo. */
  secs: number;
  /** Etiqueta bajo el cubo (L, M, 1, ENE…). Puede quedar oculta si no cabe. */
  label: string;
  /** Marca el tramo actual (hoy / este mes) con un anillo. */
  now?: boolean;
};

export type CardStat = { label: string; value: string };

export type CardData = {
  name: string;
  initials: string;
  /** Foto local ya cargada, o el avatar de la cuenta. `null` → iniciales. */
  avatar: HTMLImageElement | null;
  quote: string;
  /** "Últimos 7 días", "Último mes"… ya traducido. */
  rangeLabel: string;
  cubes: Cube[];
  stats: CardStat[];
  topGame: { label: string; cover: HTMLImageElement | null } | null;
  /** Rótulo sobre el juego más jugado ("Más jugado"). */
  topGameLabel: string;
  /** Rótulo del bloque de cubos ("Actividad"). */
  cubesLabel: string;
  /** Reclamo bajo la marca ("Copias automáticas de tus partidas"). */
  tagline: string;
};

export const CARD_W = 1200;
export const CARD_H = 675;

const FONT = '"Geist Sans", ui-sans-serif, system-ui, sans-serif';

/** Escala de intensidad, la misma familia esmeralda que el calendario. */
const LEVELS = ["#1b2320", "#064e3b", "#047857", "#10b981", "#34d399"];

function level(secs: number, max: number): number {
  if (secs <= 0 || max <= 0) return 0;
  const ratio = secs / max;
  if (ratio <= 0.25) return 1;
  if (ratio <= 0.5) return 2;
  if (ratio <= 0.75) return 3;
  return 4;
}

/** Rectángulo redondeado a mano: `ctx.roundRect` no está en todos los
 *  WebKit que nos toca soportar (SteamOS va por detrás). */
function roundRect(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  r: number,
): void {
  const rad = Math.min(r, w / 2, h / 2);
  ctx.beginPath();
  ctx.moveTo(x + rad, y);
  ctx.arcTo(x + w, y, x + w, y + h, rad);
  ctx.arcTo(x + w, y + h, x, y + h, rad);
  ctx.arcTo(x, y + h, x, y, rad);
  ctx.arcTo(x, y, x + w, y, rad);
  ctx.closePath();
}

/** Parte un texto en como mucho `maxLines` líneas que quepan en `maxWidth`.
 *  La última se recorta con puntos suspensivos si aún se pasa. */
function wrap(
  ctx: CanvasRenderingContext2D,
  text: string,
  maxWidth: number,
  maxLines: number,
): string[] {
  const words = text.split(/\s+/).filter(Boolean);
  if (words.length === 0) return [];
  const lines: string[] = [];
  let line = words[0];
  for (let i = 1; i < words.length; i++) {
    const candidate = `${line} ${words[i]}`;
    if (ctx.measureText(candidate).width <= maxWidth) {
      line = candidate;
    } else {
      lines.push(line);
      line = words[i];
      if (lines.length === maxLines) break;
    }
  }
  if (lines.length < maxLines) lines.push(line);
  // El japonés y el chino no separan por espacios: si una sola "palabra" no
  // cabe, hay que cortarla por caracteres.
  const out: string[] = [];
  for (const l of lines.slice(0, maxLines)) {
    if (ctx.measureText(l).width <= maxWidth) {
      out.push(l);
      continue;
    }
    let cur = "";
    for (const ch of l) {
      if (ctx.measureText(cur + ch).width > maxWidth) {
        if (out.length + 1 >= maxLines) break;
        out.push(cur);
        cur = ch;
      } else {
        cur += ch;
      }
    }
    out.push(cur);
  }
  const clipped = out.slice(0, maxLines);
  const last = clipped.length - 1;
  if (last >= 0 && ctx.measureText(clipped[last]).width > maxWidth) {
    let s = clipped[last];
    while (s.length > 1 && ctx.measureText(`${s}…`).width > maxWidth) {
      s = s.slice(0, -1);
    }
    clipped[last] = `${s}…`;
  }
  return clipped;
}

/** Recorta un texto de una línea a lo ancho disponible. */
function ellipsize(ctx: CanvasRenderingContext2D, text: string, maxWidth: number): string {
  if (ctx.measureText(text).width <= maxWidth) return text;
  let s = text;
  while (s.length > 1 && ctx.measureText(`${s}…`).width > maxWidth) s = s.slice(0, -1);
  return `${s}…`;
}

/** La marca: el mismo tile oscuro con la "H" en degradado del icono de la app. */
function drawLogo(ctx: CanvasRenderingContext2D, x: number, y: number, size: number): void {
  const s = size / 48;
  ctx.save();
  ctx.translate(x, y);
  roundRect(ctx, 0, 0, size, size, 12 * s);
  ctx.fillStyle = "#0a0a0a";
  ctx.fill();
  ctx.strokeStyle = "rgba(16,185,129,0.28)";
  ctx.lineWidth = Math.max(1, s);
  ctx.stroke();

  const grad = ctx.createLinearGradient(14 * s, 10 * s, 34 * s, 38 * s);
  grad.addColorStop(0, "#5eead4");
  grad.addColorStop(1, "#059669");
  ctx.fillStyle = grad;
  for (const [rx, ry, rw, rh] of [
    [13, 11, 6.5, 26],
    [28.5, 11, 6.5, 26],
    [13, 21, 22, 6],
  ]) {
    roundRect(ctx, rx * s, ry * s, rw * s, rh * s, 1.5 * s);
    ctx.fill();
  }
  ctx.restore();
}

function drawAvatar(
  ctx: CanvasRenderingContext2D,
  data: CardData,
  cx: number,
  cy: number,
  r: number,
): void {
  ctx.save();
  ctx.beginPath();
  ctx.arc(cx, cy, r, 0, Math.PI * 2);
  ctx.closePath();
  ctx.clip();
  if (data.avatar) {
    // `cover`: llenar el círculo recortando el sobrante del lado largo.
    const img = data.avatar;
    const scale = Math.max((r * 2) / img.width, (r * 2) / img.height);
    const w = img.width * scale;
    const h = img.height * scale;
    ctx.drawImage(img, cx - w / 2, cy - h / 2, w, h);
  } else {
    ctx.fillStyle = "rgba(16,185,129,0.16)";
    ctx.fillRect(cx - r, cy - r, r * 2, r * 2);
    ctx.fillStyle = "#6ee7b7";
    ctx.font = `700 ${Math.round(r * 0.82)}px ${FONT}`;
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    ctx.fillText(data.initials, cx, cy + 2);
  }
  ctx.restore();
  ctx.beginPath();
  ctx.arc(cx, cy, r, 0, Math.PI * 2);
  ctx.strokeStyle = "rgba(52,211,153,0.45)";
  ctx.lineWidth = 3;
  ctx.stroke();
}

/**
 * Pinta la tarjeta entera en el contexto dado, en coordenadas lógicas
 * 1200×675. El llamante decide la escala.
 */
export function drawCard(ctx: CanvasRenderingContext2D, data: CardData): void {
  ctx.save();
  ctx.textBaseline = "alphabetic";

  // --- fondo ------------------------------------------------------------
  ctx.fillStyle = "#050807";
  ctx.fillRect(0, 0, CARD_W, CARD_H);
  const glow = ctx.createRadialGradient(180, 60, 0, 180, 60, 720);
  glow.addColorStop(0, "rgba(16,185,129,0.20)");
  glow.addColorStop(1, "rgba(16,185,129,0)");
  ctx.fillStyle = glow;
  ctx.fillRect(0, 0, CARD_W, CARD_H);
  const glow2 = ctx.createRadialGradient(1140, 660, 0, 1140, 660, 560);
  glow2.addColorStop(0, "rgba(45,212,191,0.12)");
  glow2.addColorStop(1, "rgba(45,212,191,0)");
  ctx.fillStyle = glow2;
  ctx.fillRect(0, 0, CARD_W, CARD_H);

  // marco interior
  roundRect(ctx, 14, 14, CARD_W - 28, CARD_H - 28, 26);
  ctx.strokeStyle = "rgba(255,255,255,0.07)";
  ctx.lineWidth = 2;
  ctx.stroke();

  // --- cabecera: marca a la izquierda, dominio a la derecha (el "SEO") ---
  drawLogo(ctx, 56, 48, 44);
  ctx.textAlign = "left";
  ctx.fillStyle = "#fafafa";
  ctx.font = `700 26px ${FONT}`;
  ctx.fillText("Hoard", 114, 70);
  const brandW = ctx.measureText("Hoard").width;
  ctx.fillStyle = "#34d399";
  ctx.font = `600 26px ${FONT}`;
  ctx.fillText("Wrapped", 114 + brandW + 10, 70);

  ctx.textAlign = "right";
  ctx.fillStyle = "#a1a1aa";
  ctx.font = `500 20px ${FONT}`;
  ctx.fillText("hoard.services", CARD_W - 56, 70);

  // --- identidad --------------------------------------------------------
  drawAvatar(ctx, data, 112, 196, 52);

  // El ancho del nombre depende de si la caja del "más jugado" ocupa la
  // derecha: sin este recorte, un nombre largo se metía debajo de la caja.
  const nameW = (data.topGame ? CARD_W - 56 - 300 - 24 : CARD_W - 56) - 196;
  ctx.textAlign = "left";
  ctx.fillStyle = "#fafafa";
  ctx.font = `700 46px ${FONT}`;
  ctx.fillText(ellipsize(ctx, data.name, nameW), 196, 196);

  ctx.fillStyle = "#71717a";
  ctx.font = `500 20px ${FONT}`;
  ctx.fillText(ellipsize(ctx, data.rangeLabel, nameW), 198, 228);

  // juego más jugado, arriba a la derecha con su carátula
  if (data.topGame) {
    const boxW = 300;
    const x = CARD_W - 56 - boxW;
    // Justo encima de la caja de la frase, sin llegar a tocarla.
    const y = 142;
    roundRect(ctx, x, y, boxW, 94, 18);
    ctx.fillStyle = "rgba(255,255,255,0.04)";
    ctx.fill();
    ctx.strokeStyle = "rgba(255,255,255,0.07)";
    ctx.lineWidth = 1.5;
    ctx.stroke();

    const cover = data.topGame.cover;
    const cw = 52;
    const ch = 74;
    ctx.save();
    roundRect(ctx, x + 14, y + 11, cw, ch, 8);
    ctx.clip();
    if (cover) {
      const scale = Math.max(cw / cover.width, ch / cover.height);
      const w = cover.width * scale;
      const h = cover.height * scale;
      ctx.drawImage(cover, x + 14 + (cw - w) / 2, y + 11 + (ch - h) / 2, w, h);
    } else {
      ctx.fillStyle = "rgba(16,185,129,0.16)";
      ctx.fillRect(x + 14, y + 11, cw, ch);
      ctx.fillStyle = "#6ee7b7";
      ctx.font = `700 26px ${FONT}`;
      ctx.textAlign = "center";
      ctx.fillText(
        (data.topGame.label[0] ?? "?").toUpperCase(),
        x + 14 + cw / 2,
        y + 11 + ch / 2 + 10,
      );
      ctx.textAlign = "left";
    }
    ctx.restore();

    ctx.fillStyle = "#71717a";
    ctx.font = `600 12px ${FONT}`;
    ctx.fillText(
      ellipsize(ctx, data.topGameLabel.toUpperCase(), boxW - 96),
      x + 80,
      y + 34,
    );
    ctx.fillStyle = "#fafafa";
    ctx.font = `600 20px ${FONT}`;
    const lines = wrap(ctx, data.topGame.label, boxW - 96, 2);
    lines.forEach((l, i) => ctx.fillText(l, x + 80, y + 62 + i * 24));
  }

  // --- frase ------------------------------------------------------------
  const quoteTop = 292;
  roundRect(ctx, 56, quoteTop - 44, CARD_W - 112, 100, 20);
  const qgrad = ctx.createLinearGradient(56, 0, CARD_W - 56, 0);
  qgrad.addColorStop(0, "rgba(16,185,129,0.12)");
  qgrad.addColorStop(1, "rgba(16,185,129,0)");
  ctx.fillStyle = qgrad;
  ctx.fill();
  ctx.strokeStyle = "rgba(52,211,153,0.18)";
  ctx.lineWidth = 1.5;
  ctx.stroke();

  ctx.fillStyle = "rgba(52,211,153,0.55)";
  ctx.font = `700 56px ${FONT}`;
  ctx.fillText("“", 78, quoteTop + 12);

  ctx.fillStyle = "#e4e4e7";
  ctx.font = `italic 600 30px ${FONT}`;
  const qLines = wrap(ctx, data.quote, CARD_W - 112 - 80, 2);
  const qStart = qLines.length === 1 ? quoteTop + 17 : quoteTop - 10;
  qLines.forEach((l, i) => ctx.fillText(l, 120, qStart + i * 38));

  // --- datos curiosos ---------------------------------------------------
  const statsY = 384;
  const count = Math.max(1, data.stats.length);
  const gap = 18;
  const statW = (CARD_W - 112 - gap * (count - 1)) / count;
  data.stats.forEach((s, i) => {
    const x = 56 + i * (statW + gap);
    roundRect(ctx, x, statsY, statW, 92, 18);
    ctx.fillStyle = "rgba(255,255,255,0.035)";
    ctx.fill();
    ctx.strokeStyle = "rgba(255,255,255,0.06)";
    ctx.lineWidth = 1.5;
    ctx.stroke();

    ctx.textAlign = "center";
    ctx.fillStyle = "#34d399";
    ctx.font = `700 34px ${FONT}`;
    ctx.fillText(ellipsize(ctx, s.value, statW - 24), x + statW / 2, statsY + 46);
    ctx.fillStyle = "#71717a";
    ctx.font = `600 13px ${FONT}`;
    ctx.fillText(ellipsize(ctx, s.label.toUpperCase(), statW - 20), x + statW / 2, statsY + 72);
    ctx.textAlign = "left";
  });

  // --- cubitos ----------------------------------------------------------
  // La fila ocupa siempre el ancho útil, sea cual sea el rango: siete cubos
  // de semana salen grandes y treinta de mes salen pequeños, pero el bloque
  // empieza y acaba donde el resto de la tarjeta. El alto se acota para no
  // comerse el pie, así que con pocos cubos dejan de ser cuadrados y pasan a
  // ser losetas apaisadas, grandes, que es lo que se pidió.
  const BAND_TOP = 500;
  const BAND_H = 92;
  ctx.fillStyle = "#71717a";
  ctx.font = `600 13px ${FONT}`;
  ctx.fillText(data.cubesLabel.toUpperCase(), 56, BAND_TOP - 8);

  const cubes = data.cubes;
  if (cubes.length > 0) {
    const maxSecs = cubes.reduce((m, c) => Math.max(m, c.secs), 0);
    const avail = CARD_W - 112;
    const gapC = cubes.length <= 12 ? 14 : 6;
    const w = (avail - gapC * (cubes.length - 1)) / cubes.length;
    const h = Math.min(w, BAND_H);
    const top = BAND_TOP + (BAND_H - h) / 2;
    // Con muchos cubos los números se amontonan: etiquetamos uno de cada
    // cinco (y el último siempre, que es "hoy").
    const step = cubes.length > 12 ? 5 : 1;

    cubes.forEach((c, i) => {
      const x = 56 + i * (w + gapC);
      roundRect(ctx, x, top, w, h, Math.max(4, Math.min(w, h) * 0.2));
      ctx.fillStyle = LEVELS[level(c.secs, maxSecs)];
      ctx.fill();
      ctx.strokeStyle = c.now ? "rgba(52,211,153,0.9)" : "rgba(255,255,255,0.06)";
      ctx.lineWidth = c.now ? 2.5 : 1;
      ctx.stroke();

      const show = i === cubes.length - 1 || (cubes.length - 1 - i) % step === 0;
      if (w >= 16 && show) {
        ctx.textAlign = "center";
        ctx.fillStyle = c.now ? "#a1a1aa" : "#52525b";
        ctx.font = `600 ${Math.min(15, Math.max(10, w * 0.3))}px ${FONT}`;
        ctx.fillText(c.label, x + w / 2, top + h + 20);
        ctx.textAlign = "left";
      }
    });
  }

  // --- pie: la marca otra vez, que la imagen viaja sola ------------------
  ctx.fillStyle = "#3f3f46";
  ctx.font = `500 16px ${FONT}`;
  ctx.fillText(data.tagline, 56, CARD_H - 28);
  ctx.textAlign = "right";
  ctx.fillStyle = "#34d399";
  ctx.font = `600 17px ${FONT}`;
  ctx.fillText("hoard.services", CARD_W - 56, CARD_H - 28);
  ctx.textAlign = "left";

  ctx.restore();
}

/** Pinta la tarjeta en un canvas ya dimensionado (CSS aparte). */
export function paint(canvas: HTMLCanvasElement, data: CardData, scale: number): void {
  canvas.width = Math.round(CARD_W * scale);
  canvas.height = Math.round(CARD_H * scale);
  const ctx = canvas.getContext("2d");
  if (!ctx) return;
  ctx.setTransform(scale, 0, 0, scale, 0, 0);
  drawCard(ctx, data);
}

/**
 * Renderiza la tarjeta y devuelve el PNG en base64 (sin la cabecera del
 * data-URL, que es lo que espera `wrapple_save_card`).
 *
 * Si el canvas quedó contaminado por una imagen remota sin CORS,el avatar de
 * la cuenta Cloud, `toDataURL` lanza `SecurityError`; entonces repetimos el
 * dibujo con las iniciales en lugar de la foto, que es mejor que no poder
 * guardar nada.
 */
export function renderToPng(data: CardData, scale = 2): string {
  const canvas = document.createElement("canvas");
  paint(canvas, data, scale);
  try {
    return canvas.toDataURL("image/png").split(",")[1] ?? "";
  } catch {
    paint(canvas, { ...data, avatar: null }, scale);
    return canvas.toDataURL("image/png").split(",")[1] ?? "";
  }
}

/** Carga una imagen para el canvas. `null` si no se puede (y nunca lanza). */
export function loadImage(src: string, crossOrigin = false): Promise<HTMLImageElement | null> {
  return new Promise((resolve) => {
    const img = new Image();
    if (crossOrigin) img.crossOrigin = "anonymous";
    img.onload = () => resolve(img);
    img.onerror = () => resolve(null);
    img.src = src;
  });
}

/** Espera a que las fuentes propias estén listas: si dibujamos antes, el
 *  canvas cae a la fuente del sistema y la tarjeta sale con otra tipografía. */
export async function waitForFonts(): Promise<void> {
  if (!("fonts" in document)) return;
  try {
    await Promise.all([
      document.fonts.load(`700 46px "Geist Sans"`),
      document.fonts.load(`italic 600 30px "Geist Sans"`),
      document.fonts.load(`600 13px "Geist Sans"`),
    ]);
    await document.fonts.ready;
  } catch {
    /* da igual: se dibuja con lo que haya */
  }
}
