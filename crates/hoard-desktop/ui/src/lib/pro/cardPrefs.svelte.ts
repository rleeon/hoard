/**
 * Ajustes de la tarjeta compartible, **solo de este equipo**.
 *
 * Nombre, frase y rango viven en un `store` local (`wrapple_card.json`, junto
 * al resto de preferencias de la app) y la foto es un PNG bajo el app-data
 * dir. Nada de esto sale del equipo: no viaja al servidor, no entra en la
 * sincronización y no aparece en el export de la cuenta. Cambiar de máquina
 * es empezar de cero aquí, y es lo que se pidió.
 *
 * La foto se recorta y escala en el webview antes de guardarse (cuadrada,
 * 512 px), así lo que llega a disco es un PNG acotado en vez de la foto de
 * 12 MP que salió del móvil.
 */
import { invoke } from "@tauri-apps/api/core";
import { LazyStore } from "@tauri-apps/plugin-store";

export type CardRange = "week" | "month" | "year";

type Saved = {
  /** Vacío = usar el nombre de la sesión. */
  name: string;
  /** Vacío = frase automática (la del dado). */
  quote: string;
  range: CardRange;
  /** Semilla del dado: subirla es "otra frase". */
  seed: number;
};

const STORE = new LazyStore("wrapple_card.json");
const STORE_KEY = "card";

const DEFAULTS: Saved = { name: "", quote: "", range: "year", seed: 1 };

/** Lado del avatar guardado. 512 basta para la tarjeta a 2× (112 px lógicos). */
const AVATAR_SIDE = 512;

let saved = $state<Saved>({ ...DEFAULTS });
/** Object URL de la foto local ya cargada, o `null` si no hay. */
let photoUrl = $state<string | null>(null);
let hydrated = $state(false);

export function cardPrefs(): Readonly<Saved> {
  return saved;
}

export function cardPhotoUrl(): string | null {
  return photoUrl;
}

export function cardHydrated(): boolean {
  return hydrated;
}

let persistTimer: ReturnType<typeof setTimeout> | null = null;

function schedulePersist(): void {
  if (persistTimer) clearTimeout(persistTimer);
  persistTimer = setTimeout(() => {
    persistTimer = null;
    void STORE.set(STORE_KEY, saved)
      .then(() => STORE.save())
      .catch(() => {
        /* que no se pueda guardar la preferencia no rompe la tarjeta */
      });
  }, 250);
}

export function setCardName(name: string): void {
  saved.name = name.slice(0, 40);
  schedulePersist();
}

export function setCardQuote(quote: string): void {
  saved.quote = quote.slice(0, 140);
  schedulePersist();
}

export function setCardRange(range: CardRange): void {
  saved.range = range;
  schedulePersist();
}

/** Vuelve a tirar el dado de la frase (y descarta la frase escrita a mano). */
export function rerollQuote(): void {
  saved.quote = "";
  saved.seed = (saved.seed + 1) % 1_000_000;
  schedulePersist();
}

export async function hydrateCardPrefs(): Promise<void> {
  try {
    const s = await STORE.get<Partial<Saved>>(STORE_KEY);
    if (s) saved = { ...DEFAULTS, ...s };
  } catch {
    /* primera vez, o store ilegible: nos quedamos con los defaults */
  }
  await loadPhoto();
  hydrated = true;
}

async function loadPhoto(): Promise<void> {
  try {
    const buf = await invoke<ArrayBuffer>("wrapple_avatar_bytes");
    if (photoUrl) URL.revokeObjectURL(photoUrl);
    photoUrl = URL.createObjectURL(new Blob([buf], { type: "image/png" }));
  } catch {
    if (photoUrl) URL.revokeObjectURL(photoUrl);
    photoUrl = null;
  }
}

/**
 * Recorta la imagen a un cuadrado centrado de `AVATAR_SIDE` px y devuelve su
 * PNG en base64. Recortamos al cuadrado (no deformamos) porque la tarjeta la
 * pinta dentro de un círculo.
 */
function squarePng(img: HTMLImageElement): string {
  const canvas = document.createElement("canvas");
  canvas.width = AVATAR_SIDE;
  canvas.height = AVATAR_SIDE;
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("sin canvas 2d");
  const side = Math.min(img.width, img.height);
  ctx.drawImage(
    img,
    (img.width - side) / 2,
    (img.height - side) / 2,
    side,
    side,
    0,
    0,
    AVATAR_SIDE,
    AVATAR_SIDE,
  );
  return canvas.toDataURL("image/png").split(",")[1] ?? "";
}

/** Guarda como foto de la tarjeta la imagen del path indicado. */
export async function setCardPhotoFromPath(path: string): Promise<void> {
  const buf = await invoke<ArrayBuffer>("wrapple_read_image", { sourcePath: path });
  const url = URL.createObjectURL(new Blob([buf]));
  try {
    const img = await new Promise<HTMLImageElement>((resolve, reject) => {
      const el = new Image();
      el.onload = () => resolve(el);
      el.onerror = () => reject(new Error("imagen ilegible"));
      el.src = url;
    });
    await invoke("wrapple_set_avatar", { pngBase64: squarePng(img) });
  } finally {
    URL.revokeObjectURL(url);
  }
  await loadPhoto();
}

export async function clearCardPhoto(): Promise<void> {
  await invoke("wrapple_clear_avatar");
  await loadPhoto();
}

/** Guarda el PNG de la tarjeta en la galería. Devuelve la ruta final. */
export function saveCardToGallery(pngBase64: string, label: string | null): Promise<string> {
  return invoke<string>("wrapple_save_card", { pngBase64, label });
}
