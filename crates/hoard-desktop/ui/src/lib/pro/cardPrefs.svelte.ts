/**
 * The shareable card's settings, **this machine's only**.
 *
 * The name, the phrase and the rank live in a local store (`wrapple_card.json`,
 * alongside the app's other preferences) and the picture is a PNG under the
 * app-data dir. None of it leaves the machine: it does not travel to the server,
 * does not enter the sync and does not appear in the account export. Changing
 * machine means starting from scratch here, which is what was asked for.
 *
 * The picture is cropped and scaled in the webview before being stored (square,
 * 512 px), so what reaches disk is a bounded PNG rather than the 12 MP photo that
 * came off the phone.
 */
import { invoke } from "@tauri-apps/api/core";
import { LazyStore } from "@tauri-apps/plugin-store";

export type CardRange = "week" | "month" | "year";

type Saved = {
  /** Empty means use the session's name. */
  name: string;
  /** Empty means the automatic phrase (the dice's). */
  quote: string;
  range: CardRange;
  /** Semilla del dado: subirla es "otra frase". */
  seed: number;
};

const STORE = new LazyStore("wrapple_card.json");
const STORE_KEY = "card";

const DEFAULTS: Saved = { name: "", quote: "", range: "year", seed: 1 };

/** The stored avatar's side. 512 is enough for the card at 2x (112 logical px). */
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
 * Crops the image to a centred square of `AVATAR_SIDE` px and returns its PNG in
 * base64. It crops to the square rather than distorting because the card paints it
 * inside a circle.
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

/** Saves the card's PNG into the gallery. Returns the final path. */
export function saveCardToGallery(pngBase64: string, label: string | null): Promise<string> {
  return invoke<string>("wrapple_save_card", { pngBase64, label });
}
