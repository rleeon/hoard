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
  /** `null` when the user took the phrase off the card. */
  quote: string | null;
  /** "Last 7 days", "Last month" and so on, already translated. */
  rangeLabel: string;
  cubes: Cube[];
  stats: CardStat[];
  topGame: { label: string; cover: HTMLImageElement | null } | null;
  /** The label above the most-played game. */
  topGameLabel: string;
  /** The tile block's label. */
  cubesLabel: string;
  palette: CardPalette;
};

export const CARD_W = 1200;
export const CARD_H = 675;

const FONT = '"Geist Sans", ui-sans-serif, system-ui, sans-serif';

// ---- colour
// The card follows an accent like the rest of the app. The canvas gets plain
// sRGB rather than `oklch()` strings: older WebKitGTK builds ignore a
// fillStyle they cannot parse and silently keep the previous colour.

export type CardPalette = {
  bg: string;
  /** Boxes on the card: a step above the background, same hue. */
  surface: string;
  /** Empty activity tile. */
  empty: string;
  a300: string;
  a400: string;
  /** What `--color-accent` would be with this hue, for controls that show it
   *  (the slider's thumb). Same maths as `applyAccentHue`. */
  accent: string;
  /** Translucent accent for the initials and cover placeholders. */
  tint: string;
  nowStroke: string;
  /** Activity tiles, from none to the busiest. */
  levels: string[];
};

type Oklch = [number, number, number];

function toLinear(c: number): number {
  return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
}

function fromLinear(c: number): number {
  const v = c <= 0.0031308 ? 12.92 * c : 1.055 * c ** (1 / 2.4) - 0.055;
  return Math.min(1, Math.max(0, v));
}

function hexToOklch(hex: string): Oklch {
  const n = parseInt(hex.slice(1), 16);
  const r = toLinear(((n >> 16) & 255) / 255);
  const g = toLinear(((n >> 8) & 255) / 255);
  const b = toLinear((n & 255) / 255);
  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
  const L = 0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s;
  const A = 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s;
  const B = 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s;
  return [L, Math.hypot(A, B), ((Math.atan2(B, A) * 180) / Math.PI + 360) % 360];
}

function oklchToRgb([L, C, H]: Oklch): [number, number, number] {
  const h = (H * Math.PI) / 180;
  const A = C * Math.cos(h);
  const B = C * Math.sin(h);
  const l = (L + 0.3963377774 * A + 0.2158037573 * B) ** 3;
  const m = (L - 0.1055613458 * A - 0.0638541728 * B) ** 3;
  const s = (L - 0.0894841775 * A - 1.291485548 * B) ** 3;
  return [
    fromLinear(4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s),
    fromLinear(-1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s),
    fromLinear(-0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s),
  ].map((v) => Math.round(v * 255)) as [number, number, number];
}

const rgb = ([r, g, b]: [number, number, number], a = 1) =>
  a === 1 ? `rgb(${r},${g},${b})` : `rgba(${r},${g},${b},${a})`;

// The stock look: Tailwind's emerald, the ramp the app shows while Settings is
// on its first gem. The neutrals are the card's own near-blacks.
const STOCK = {
  bg: "#050807",
  surface: "#0f1211",
  empty: "#1b2320",
  a300: "#6ee7b7",
  a400: "#34d399",
  a500: "#10b981",
  a700: "#047857",
  a900: "#064e3b",
};

function hexRgb(hex: string): [number, number, number] {
  const n = parseInt(hex.slice(1), 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

/**
 * The colours for a hue, with the same lightness and chroma per step the app
 * repaints its emerald ramp with (`ACCENT_STOPS`). `null` is the stock emerald.
 */
export function cardPalette(
  hue: number | null,
  stops: [string, number, number][],
): CardPalette {
  let c: Record<keyof typeof STOCK, [number, number, number]>;
  if (hue == null) {
    c = Object.fromEntries(
      Object.entries(STOCK).map(([k, v]) => [k, hexRgb(v)]),
    ) as typeof c;
  } else {
    const stop = (name: string): Oklch => {
      const [, l, ch] = stops.find(([k]) => k === `--color-emerald-${name}`)!;
      return [l, ch, hue];
    };
    // The neutrals keep their lightness and chroma and only lean to the hue,
    // the way the app's greys follow `--tint-hue`.
    const lean = (hex: string): [number, number, number] => {
      const [l, ch] = hexToOklch(hex);
      return oklchToRgb([l, ch, hue]);
    };
    c = {
      bg: lean(STOCK.bg),
      surface: lean(STOCK.surface),
      empty: lean(STOCK.empty),
      a300: oklchToRgb(stop("300")),
      a400: oklchToRgb(stop("400")),
      a500: oklchToRgb(stop("500")),
      a700: oklchToRgb(stop("700")),
      a900: oklchToRgb(stop("900")),
    };
  }
  return {
    bg: rgb(c.bg),
    surface: rgb(c.surface),
    empty: rgb(c.empty),
    a300: rgb(c.a300),
    a400: rgb(c.a400),
    accent: rgb(oklchToRgb([0.62, 0.15, hue ?? 158])),
    tint: rgb(c.a500, 0.16),
    nowStroke: rgb(c.a400, 0.9),
    levels: [rgb(c.empty), rgb(c.a900), rgb(c.a700), rgb(c.a500), rgb(c.a400)],
  };
}

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
    // Black inside, the accent only in the ring and the letters, as everywhere
    // else in the app a picture is missing.
    ctx.fillStyle = "#000";
    ctx.fillRect(cx - r, cy - r, r * 2, r * 2);
    ctx.fillStyle = data.palette.a300;
    ctx.font = `700 ${Math.round(r * 0.82)}px ${FONT}`;
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    ctx.fillText(data.initials, cx, cy + 2);
  }
  ctx.restore();
  ctx.beginPath();
  ctx.arc(cx, cy, r, 0, Math.PI * 2);
  ctx.strokeStyle = data.palette.a400;
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

  const pal = data.palette;

  ctx.fillStyle = pal.bg;
  ctx.fillRect(0, 0, CARD_W, CARD_H);

  // marco interior
  roundRect(ctx, 14, 14, CARD_W - 28, CARD_H - 28, 26);
  ctx.strokeStyle = "rgba(255,255,255,0.07)";
  ctx.lineWidth = 2;
  ctx.stroke();

  // ---- brand: just the words, with Hoard's H in the accent
  ctx.textAlign = "left";
  ctx.font = `700 26px ${FONT}`;
  ctx.fillStyle = pal.a400;
  ctx.fillText("H", 56, 72);
  ctx.fillStyle = "#fafafa";
  ctx.fillText("oard", 56 + ctx.measureText("H").width, 72);
  const brandW = ctx.measureText("Hoard").width;
  ctx.fillStyle = pal.a400;
  ctx.font = `600 26px ${FONT}`;
  ctx.fillText("Wrapped", 56 + brandW + 10, 72);

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
    ctx.fillStyle = pal.surface;
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
      ctx.fillStyle = pal.tint;
      ctx.fillRect(x + 14, y + 11, cw, ch);
      ctx.fillStyle = pal.a300;
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

  // ---- phrase
  // Without it the block below would leave a 116 px hole under the identity,
  // so the rest moves up by half of it and the other half goes to the bottom.
  const lift = data.quote == null ? 58 : 0;
  if (data.quote != null) {
    const quoteTop = 292;
    roundRect(ctx, 56, quoteTop - 44, CARD_W - 112, 100, 20);
    ctx.fillStyle = pal.surface;
    ctx.fill();
    ctx.strokeStyle = "rgba(255,255,255,0.07)";
    ctx.lineWidth = 1.5;
    ctx.stroke();

    ctx.fillStyle = "#e4e4e7";
    ctx.font = `italic 600 30px ${FONT}`;
    const qLines = wrap(ctx, data.quote, CARD_W - 112 - 48, 2);
    const qStart = qLines.length === 1 ? quoteTop + 17 : quoteTop - 10;
    qLines.forEach((l, i) => ctx.fillText(l, 80, qStart + i * 38));
  }

  // --- datos curiosos ---------------------------------------------------
  const statsY = 384 - lift;
  const count = Math.max(1, data.stats.length);
  const gap = 18;
  const statW = (CARD_W - 112 - gap * (count - 1)) / count;
  data.stats.forEach((s, i) => {
    const x = 56 + i * (statW + gap);
    roundRect(ctx, x, statsY, statW, 92, 18);
    ctx.fillStyle = pal.surface;
    ctx.fill();
    ctx.strokeStyle = "rgba(255,255,255,0.06)";
    ctx.lineWidth = 1.5;
    ctx.stroke();

    ctx.textAlign = "center";
    ctx.fillStyle = pal.a400;
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
  const BAND_TOP = 500 - lift;
  const BAND_H = 92;
  ctx.fillStyle = "#71717a";
  ctx.font = `600 13px ${FONT}`;
  ctx.fillText(data.cubesLabel.toUpperCase(), 56, BAND_TOP - 8);

  const cubes = data.cubes;
  if (cubes.length > 0) {
    const maxSecs = cubes.reduce((m, c) => Math.max(m, c.secs), 0);
    const avail = CARD_W - 112;
    // A month has 30 tiles; a year 12 or 13, which still fit with room.
    const dense = cubes.length > 13;
    const gapC = dense ? 6 : 14;
    const w = (avail - gapC * (cubes.length - 1)) / cubes.length;
    const h = Math.min(w, BAND_H);
    const top = BAND_TOP + (BAND_H - h) / 2;
    // With many tiles the numbers pile up, so one in five is labelled (and always
    // the last, which is today).
    const step = dense ? 5 : 1;

    cubes.forEach((c, i) => {
      const x = 56 + i * (w + gapC);
      roundRect(ctx, x, top, w, h, Math.max(4, Math.min(w, h) * 0.2));
      ctx.fillStyle = pal.levels[level(c.secs, maxSecs)];
      ctx.fill();
      ctx.strokeStyle = c.now ? pal.nowStroke : "rgba(255,255,255,0.06)";
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

  // ---- footer: the address, since the picture travels on its own
  ctx.textAlign = "right";
  ctx.fillStyle = pal.a400;
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
