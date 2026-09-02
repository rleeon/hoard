/**
 * Drawing the Hoard-Wrapped shareable card.
 *
 * The card is painted on a `<canvas>` and that same canvas is both what shows on
 * screen and what gets saved to the gallery: one layout, zero divergence between the
 * preview and the picture. The alternative (laying it out in HTML and drawing it
 * again here to export) is two designs that drift apart the moment somebody touches
 * one.
 *
 * Everything is drawn in a logical 1200×675 space (16:9, the ratio the social
 * networks preview) and scaled with `ctx.scale`, so the same code serves the view
 * (1x) and the PNG that gets saved (2x).
 *
 * Mind the images: the export's `toDataURL` fails when the canvas is tainted, and
 * that happens the moment you draw a remote image without CORS. That is why the
 * local picture (bytes Rust gives us, our own blob) and the covers (likewise) are
 * safe, while the Cloud account's avatar is a Google URL that may or may not answer
 * with CORS. `renderToPng` accounts for it: if the export blows up, it draws again
 * without the remote avatar.
 */

/** One activity tile: a day, a month, whatever the range calls for. */
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
  /** "Last 7 days", "Last month" and so on, already translated. */
  rangeLabel: string;
  cubes: Cube[];
  stats: CardStat[];
  topGame: { label: string; cover: HTMLImageElement | null } | null;
  /** The label above the most-played game. */
  topGameLabel: string;
  /** The tile block's label. */
  cubesLabel: string;
  /** The strapline under the brand. */
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

/** A rounded rectangle by hand: `ctx.roundRect` is not in every WebKit we have to
 *  support (SteamOS lags behind). */
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

/** Splits a text into at most `maxLines` lines that fit `maxWidth`. The last one is
 *  trimmed with an ellipsis when it still overflows. */
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
  // Japanese and Chinese do not separate on spaces: when a single "word" does not
  // fit, it has to be cut by characters.
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

/** Trims a single-line text to the available width. */
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
    // `cover`: fill the circle, cropping the excess off the long side.
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
 * Paints the whole card into the given context, in logical 1200×675 coordinates.
 * The caller decides the scale.
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

  // The name's width depends on whether the most-played box takes the right-hand
  // side: without this trim, a long name slid under the box.
  const nameW = (data.topGame ? CARD_W - 56 - 300 - 24 : CARD_W - 56) - 196;
  ctx.textAlign = "left";
  ctx.fillStyle = "#fafafa";
  ctx.font = `700 46px ${FONT}`;
  ctx.fillText(ellipsize(ctx, data.name, nameW), 196, 196);

  ctx.fillStyle = "#71717a";
  ctx.font = `500 20px ${FONT}`;
  ctx.fillText(ellipsize(ctx, data.rangeLabel, nameW), 198, 228);

  // the most-played game, top right with its cover
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

  // ---- the tiles
  // The row always takes the usable width, whatever the range: a week's seven tiles
  // come out large and a month's thirty come out small, but the block starts and
  // ends where the rest of the card does. The height is bounded so it does not eat
  // the footer, so with few tiles they stop being squares and become wide landscape
  // tiles, large, which is what was asked for.
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
    // With many tiles the numbers pile up, so one in five is labelled (and always
    // the last, which is today).
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
 * Renders the card and returns the PNG in base64 (without the data URL's header,
 * which is what `wrapple_save_card` expects).
 *
 * When the canvas was tainted by a remote image with no CORS (the Cloud account's
 * avatar), `toDataURL` throws `SecurityError`; the drawing is then repeated with the
 * initials instead of the picture, which beats not being able to save anything.
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

/** Waits for our own fonts to be ready: drawing earlier drops the canvas back to
 *  the system font and the card comes out in a different typeface. */
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
