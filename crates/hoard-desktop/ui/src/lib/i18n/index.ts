/**
 * Internationalization bootstrap.
 *
 * Registers every locale lazily so messages aren't bundled into the initial
 * chunk, and seeds the active locale from the persisted user preference (with
 * a fallback to the navigator's language). Components import `$_`, `locale`,
 * etc. from `svelte-i18n` directly.
 */
import {
  init,
  locale,
  register,
  waitLocale,
  getLocaleFromNavigator,
} from "svelte-i18n";
import { get } from "svelte/store";

import { getPrefs, savePrefs } from "../api";

/** Locales the UI ships translations for. Adding a new one means dropping a
 *  JSON in `locales/` and pushing an entry here. */
export const supportedLocales: { code: string; label: string }[] = [
  { code: "en", label: "English" },
  { code: "es", label: "Español" },
  { code: "fr", label: "Français" },
  { code: "de", label: "Deutsch" },
  { code: "pt", label: "Português" },
  { code: "it", label: "Italiano" },
  { code: "ja", label: "日本語" },
  { code: "zh", label: "简体中文" },
];

// Lazy registration, Vite splits each JSON into its own chunk, so users only
// pay the cost of the locale they actually pick.
register("en", () => import("./locales/en.json"));
register("es", () => import("./locales/es.json"));
register("fr", () => import("./locales/fr.json"));
register("de", () => import("./locales/de.json"));
register("pt", () => import("./locales/pt.json"));
register("it", () => import("./locales/it.json"));
register("ja", () => import("./locales/ja.json"));
register("zh", () => import("./locales/zh.json"));

/** Pick the best available locale from a navigator string like "fr-FR". We
 *  match on the primary subtag so "pt-BR" still finds our `pt` bundle. */
function pickSupported(candidate: string | null | undefined): string | null {
  if (!candidate) return null;
  const lower = candidate.toLowerCase();
  const exact = supportedLocales.find((l) => l.code === lower);
  if (exact) return exact.code;
  const primary = lower.split(/[-_]/)[0];
  const match = supportedLocales.find((l) => l.code === primary);
  return match ? match.code : null;
}

// Seed `init` synchronously with a best-effort initial locale (navigator),
// then asynchronously override with the persisted preference once disk I/O
// resolves. This keeps the first paint localized correctly for users whose
// browser language matches their pref, which is the common case.
init({
  fallbackLocale: "en",
  initialLocale: pickSupported(getLocaleFromNavigator()) ?? "en",
});

/** Cuánto puede retrasar el idioma persistido al primer render.
 *
 *  Leer `prefs.json` es un `invoke` + una lectura de disco: normalmente un par
 *  de milisegundos, pero es I/O y nada garantiza que lo sea (disco dormido,
 *  perfil en red, un antivirus mirando el fichero). Por debajo de este plazo
 *  esperamos y el arranque sale ya en el idioma correcto; por encima montamos
 *  con el del sistema y corregimos cuando conteste. */
const PREFS_LOCALE_BUDGET_MS = 250;

/** Idioma guardado en prefs, o `null` si no hay o no se pudo leer. Arranca al
 *  importar el módulo para que corra en paralelo con el resto del bootstrap. */
const persistedLocale: Promise<string | null> = getPrefs()
  .then((prefs) => pickSupported(prefs.language ?? null))
  // Quedarnos con lo que eligió `init` es aceptable, la página de Ajustes
  // puede reparar un prefs.json corrupto.
  .catch(() => null);

/** Resuelve a `null` si `promise` tarda más de `ms`, sin cancelarla. */
function withBudget<T>(promise: Promise<T>, ms: number): Promise<T | null> {
  return Promise.race([
    promise,
    new Promise<null>((resolve) => setTimeout(() => resolve(null), ms)),
  ]);
}

/** Aplica el idioma persistido si difiere del activo. */
async function applyPersisted(code: string | null): Promise<void> {
  if (code && code !== get(locale)) {
    await locale.set(code);
  }
}

/** Promise the bootstrap (`main.ts`) awaits before calling `mount()`.
 *
 *  svelte-i18n's `$_` throws "Cannot format a message without first setting
 *  the initial locale" if a component renders before the locale loader has
 *  resolved. `init()` only *queues* the load, we must explicitly wait for
 *  it.
 *
 *  Lo que este promise **no** hace es esperar al disco sin límite. Antes
 *  encadenaba `getPrefs()` y luego `waitLocale()`, así que el `mount()`,y con
 *  él el primer pixel de la app, quedaba detrás de una ida y vuelta por IPC.
 *  Ahora la lectura de prefs corre en paralelo y sólo la esperamos
 *  `PREFS_LOCALE_BUDGET_MS`; si llega tarde, montamos con el idioma del sistema
 *  y [`i18nSettled`] la aplica después (svelte-i18n es reactivo: los textos se
 *  cambian solos, sin remontar nada). */
export const i18nReady: Promise<void> = (async () => {
  await applyPersisted(await withBudget(persistedLocale, PREFS_LOCALE_BUDGET_MS));
  // Block until the active locale's dictionary is loaded. Without this the
  // very first render sees `$locale = null` and `$_` throws, which silently
  // unwinds Svelte's mount() and leaves the user with a blank window.
  await waitLocale();
})();

/** El idioma definitivo, ya con la preferencia del disco aplicada llegase
 *  cuando llegase. Nadie tiene que esperarlo para pintar; existe para que el
 *  arranque lento acabe igualmente en el idioma elegido (y para que un test
 *  pueda esperar a que todo se asiente). */
export const i18nSettled: Promise<void> = (async () => {
  try {
    await i18nReady;
    await applyPersisted(await persistedLocale);
  } catch {
    // Un diccionario que no carga deja los textos en el idioma anterior, que
    // es mejor que una promesa rechazada sin dueño en la consola.
  }
})();

/** Lectura en vuelo, para que una ráfaga de eventos colapse en una sola. */
let syncing: Promise<void> | null = null;

/**
 * Relee el idioma guardado y lo aplica si se ha quedado atrás.
 *
 * Hace falta porque [`setLocale`] cambia el idioma **de la ventana que lo
 * llama** y persiste a prefs, pero no avisa a nadie: cada ventana tiene su
 * propio contexto JS y, por tanto, su propio store de svelte-i18n. La ventana
 * principal es la única que pasa por Ajustes, así que una segunda ventana que
 * sobreviva al cambio se queda con el idioma con el que montó, y el HUD del
 * juego sobrevive siempre, porque cerrarlo lo **esconde**, no lo destruye.
 *
 * Llamarlo al volver a enseñarse es la mitad barata del arreglo: una lectura de
 * prefs por apertura, sin eventos nuevos que emitir ni oír, y sin poder
 * disparar nada. Si prefs no se puede leer, o no hay idioma elegido, se queda
 * el que hubiera: no hay nada mejor que adivinar.
 */
export function syncPersistedLocale(): Promise<void> {
  if (syncing) return syncing;
  syncing = (async () => {
    try {
      const prefs = await getPrefs();
      await applyPersisted(pickSupported(prefs.language ?? null));
    } catch {
      /* best-effort */
    } finally {
      syncing = null;
    }
  })();
  return syncing;
}

/** Update the active locale and persist it to prefs so the next launch
 *  remembers the choice. */
export async function setLocale(code: string): Promise<void> {
  await locale.set(code);
  try {
    const current = await getPrefs();
    await savePrefs({ ...current, language: code });
  } catch {
    // Persisting is best-effort; the in-memory switch already happened.
  }
}
